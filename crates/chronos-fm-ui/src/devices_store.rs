//! GPUI global holding the live removable-media device list (T001/T002
//! sibling infra; spec: docs/superpowers/specs/2026-08-06-removable-media-mount.md).

use chronos_fm_services::devices::{Device, DeviceBackend};
use gpui::{App, AppContext, AsyncApp, BorrowAppContext, Global};
use std::path::PathBuf;
use std::sync::Arc;

/// Live snapshot of removable/internal storage devices, refreshed by the
/// background hotplug subscription (Task 5) and by mount/unmount calls
/// (Task 4).
#[derive(Default)]
pub struct DeviceStore {
    pub devices: Vec<Device>,
    /// Most recent mount/unmount/eject error, surfaced by the sidebar
    /// (Task 6) as an inline status line. Cleared on the next successful
    /// operation.
    pub last_error: Option<String>,
}

impl Global for DeviceStore {}

/// Registers an empty `DeviceStore` as the app global. Call once at
/// startup (`crates/chronos-fm/src/app.rs`, alongside
/// `clipboard::init(app)` — see Task 5 for the hotplug subscription that
/// populates it after this).
pub fn init(cx: &mut App) {
    cx.set_global(DeviceStore::default());
}

/// Reads the current device list/error state.
pub fn current(cx: &App) -> &DeviceStore {
    cx.global::<DeviceStore>()
}

impl DeviceStore {
    /// Mounts the device at `object_path` on GPUI's background executor,
    /// then updates `DeviceStore` and invokes `on_mounted` with the
    /// resolved mount path (or records the error in `last_error`).
    pub fn mount_and_navigate(
        cx: &mut App,
        backend: Arc<dyn DeviceBackend>,
        object_path: String,
        on_mounted: impl FnOnce(&mut App, PathBuf) + 'static,
    ) {
        cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_spawn({
                    let object_path = object_path.clone();
                    async move { backend.mount(&object_path).await }
                })
                .await;

            cx.update(|cx| match result {
                Ok(path) => {
                    cx.update_global::<DeviceStore, _>(|store, _cx| {
                        store.last_error = None;
                        if let Some(d) =
                            store.devices.iter_mut().find(|d| d.object_path == object_path)
                        {
                            d.mount_point = Some(path.clone());
                        }
                    });
                    on_mounted(cx, path);
                }
                Err(error) => {
                    cx.update_global::<DeviceStore, _>(|store, _cx| {
                        store.last_error = Some(format!("Mount failed: {error}"));
                    });
                }
            });
        })
        .detach();
    }

    /// Unmounts the device at `object_path` in the background; on success
    /// clears its `mount_point`, on failure records `last_error`.
    pub fn unmount(cx: &mut App, backend: Arc<dyn DeviceBackend>, object_path: String) {
        cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_spawn({
                    let object_path = object_path.clone();
                    async move { backend.unmount(&object_path).await }
                })
                .await;

            cx.update(|cx| {
                cx.update_global::<DeviceStore, _>(|store, _cx| match result {
                    Ok(()) => {
                        store.last_error = None;
                        if let Some(d) =
                            store.devices.iter_mut().find(|d| d.object_path == object_path)
                        {
                            d.mount_point = None;
                        }
                    }
                    Err(error) => {
                        store.last_error = Some(format!("Unmount failed: {error}"));
                    }
                });
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{BorrowAppContext, TestAppContext};
    use std::sync::Mutex as StdMutex;

    #[gpui::test]
    async fn init_registers_empty_store(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        cx.update(|cx| {
            let store = current(cx);
            assert!(store.devices.is_empty());
            assert!(store.last_error.is_none());
        });
    }

    #[gpui::test]
    async fn update_global_replaces_device_list(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        cx.update(|cx| {
            cx.update_global::<DeviceStore, _>(|store, _cx| {
                store.devices.push(Device {
                    object_path: "/o/1".into(),
                    label: "USB".into(),
                    device_node: "/dev/sdb1".into(),
                    mount_point: None,
                    is_removable: true,
                    size_bytes: 42,
                });
            });
        });
        cx.update(|cx| {
            assert_eq!(current(cx).devices.len(), 1);
            assert_eq!(current(cx).devices[0].label, "USB");
        });
    }

    struct MountOkBackend {
        mounted_at: PathBuf,
    }

    #[async_trait::async_trait]
    impl DeviceBackend for MountOkBackend {
        async fn list(&self) -> anyhow::Result<Vec<Device>> {
            Ok(vec![])
        }
        async fn mount(&self, _object_path: &str) -> anyhow::Result<PathBuf> {
            Ok(self.mounted_at.clone())
        }
        async fn unmount(&self, _object_path: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn eject(&self, _object_path: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[gpui::test]
    async fn mount_and_navigate_calls_callback_with_mount_path(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        let backend: Arc<dyn DeviceBackend> = Arc::new(MountOkBackend {
            mounted_at: PathBuf::from("/run/media/neo/USB"),
        });
        let navigated_to = Arc::new(StdMutex::new(None));
        let navigated_to_clone = navigated_to.clone();

        cx.update(|cx| {
            DeviceStore::mount_and_navigate(cx, backend, "/o/1".to_string(), move |_cx, path| {
                *navigated_to_clone.lock().unwrap() = Some(path);
            });
        });

        cx.run_until_parked();
        assert_eq!(
            *navigated_to.lock().unwrap(),
            Some(PathBuf::from("/run/media/neo/USB"))
        );
    }
}
