//! GPUI global holding the live removable-media device list (T001/T002
//! sibling infra; spec: docs/superpowers/specs/2026-08-06-removable-media-mount.md).

use chronos_fm_services::devices::{Device, DeviceBackend};
use gpui::{App, AppContext, AsyncApp, BorrowAppContext, Global};
use std::collections::HashSet;
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
    /// The live device backend (T008): set by the app's hotplug watcher once
    /// it connects, read by the sidebar at click time so mount/unmount/eject
    /// never open a new D-Bus connection per click. `None` while the backend
    /// is unavailable (headless/container) — the sidebar renders rows without
    /// click handlers in that case.
    pub backend: Option<Arc<dyn DeviceBackend>>,
    /// `object_path`s whose `mount()` call is currently in flight (T008 guard):
    /// a second `mount_and_navigate` for the same volume while the first is
    /// still running is dropped, so rapid double-clicks on an unmounted row
    /// can't issue duplicate mounts. The path is registered synchronously
    /// (before the background task spawns) and cleared on both success and
    /// error, so the volume becomes clickable again once the mount settles.
    /// (If the background `mount()` never settles — a hung udisks2 — the path
    /// stays in the set and the volume stays unclickable; accepted v1 edge
    /// case, no timeout.)
    pub mounting: HashSet<String>,
    /// `object_path`s whose `unmount()` call is currently in flight (T008
    /// guard, symmetric to `mounting`): a second unmount click while the first
    /// is still running would otherwise call `backend.unmount()` on an
    /// already-unmounted volume and surface a spurious "Unmount failed" error.
    pub unmounting: HashSet<String>,
    /// `object_path`s whose `eject()` call is currently in flight (T008 guard,
    /// symmetric to `mounting`/`unmounting`): rapid double-clicks can't issue
    /// duplicate `Drive.Eject` calls on the same volume.
    pub ejecting: HashSet<String>,
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
        // In-flight guard (T008): drop re-clicks for a volume whose `mount()`
        // is still running, and refuse to mount a volume that is being ejected
        // (mounting a drive mid-eject races the backend). Registered
        // synchronously — before `cx.spawn` — so a second click in the same
        // frame is already blocked. The `try_global` check is purely defensive
        // (the store is always registered by `init`); the insert below would
        // panic if it were ever missing, exactly as the completion path did
        // before the guard.
        if cx
            .try_global::<DeviceStore>()
            .is_some_and(|store| {
                store.mounting.contains(&object_path) || store.ejecting.contains(&object_path)
            })
        {
            return;
        }
        cx.update_global::<DeviceStore, _>(|store, _cx| {
            store.mounting.insert(object_path.clone());
        });

        cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_spawn({
                    let object_path = object_path.clone();
                    async move { backend.mount(&object_path).await }
                })
                .await;

            cx.update(|cx| {
                cx.update_global::<DeviceStore, _>(|store, _cx| {
                    // Release the guard on both outcomes so a later click works.
                    store.mounting.remove(&object_path);
                    match &result {
                        Ok(path) => {
                            store.last_error = None;
                            if let Some(d) =
                                store.devices.iter_mut().find(|d| d.object_path == object_path)
                            {
                                d.mount_point = Some(path.clone());
                            }
                        }
                        Err(error) => {
                            store.last_error = Some(format!("Mount failed: {error}"));
                        }
                    }
                });
                if let Ok(path) = &result {
                    on_mounted(cx, path.clone());
                }
            });
        })
        .detach();
    }

    /// Unmounts the device at `object_path` in the background; on success
    /// clears its `mount_point`, on failure records `last_error`.
    pub fn unmount(cx: &mut App, backend: Arc<dyn DeviceBackend>, object_path: String) {
        // In-flight guard (T008), symmetric to `mounting`: a second unmount
        // click while the first is still running would call `backend.unmount()`
        // on an already-unmounted volume and surface a spurious error. Also
        // skip while the volume is being ejected — eject unmounts internally,
        // so a concurrent unmount would race it.
        if cx
            .try_global::<DeviceStore>()
            .is_some_and(|store| {
                store.unmounting.contains(&object_path) || store.ejecting.contains(&object_path)
            })
        {
            return;
        }
        cx.update_global::<DeviceStore, _>(|store, _cx| {
            store.unmounting.insert(object_path.clone());
        });

        cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_spawn({
                    let object_path = object_path.clone();
                    async move { backend.unmount(&object_path).await }
                })
                .await;

            cx.update(|cx| {
                cx.update_global::<DeviceStore, _>(|store, _cx| {
                    // Release the guard on both outcomes.
                    store.unmounting.remove(&object_path);
                    match result {
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
                    }
                });
            });
        })
        .detach();
    }

    /// Ejects the drive owning the volume at `object_path` in the background;
    /// on success clears the volume's `mount_point` (udisks2 unmounts any
    /// mounted filesystem as part of ejecting), on failure records
    /// `last_error`. `drive_object_path` is the resolved `Block.Drive` path
    /// carried on [`Device::drive_object_path`] — `Drive.Eject` lives on the
    /// Drive object, not the Block object the panel holds.
    pub fn eject(
        cx: &mut App,
        backend: Arc<dyn DeviceBackend>,
        object_path: String,
        drive_object_path: Option<String>,
    ) {
        // In-flight guard (T008), symmetric to `mounting`/`unmounting`: drop a
        // second eject for the same volume, and refuse to eject a volume whose
        // mount is still in flight (`Drive.Eject` racing `Filesystem.Mount`).
        if cx
            .try_global::<DeviceStore>()
            .is_some_and(|store| {
                store.ejecting.contains(&object_path) || store.mounting.contains(&object_path)
            })
        {
            return;
        }
        cx.update_global::<DeviceStore, _>(|store, _cx| {
            store.ejecting.insert(object_path.clone());
        });

        cx.spawn(async move |cx: &mut AsyncApp| {
            let result = cx
                .background_spawn({
                    let object_path = object_path.clone();
                    let drive_object_path = drive_object_path.clone();
                    async move {
                        backend.eject(&object_path, drive_object_path.as_deref()).await
                    }
                })
                .await;

            cx.update(|cx| {
                cx.update_global::<DeviceStore, _>(|store, _cx| {
                    // Release the guard on both outcomes.
                    store.ejecting.remove(&object_path);
                    match result {
                        Ok(()) => {
                            store.last_error = None;
                            if let Some(d) =
                                store.devices.iter_mut().find(|d| d.object_path == object_path)
                            {
                                d.mount_point = None;
                            }
                        }
                        Err(error) => {
                            store.last_error = Some(format!("Eject failed: {error}"));
                        }
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
                    drive_object_path: None,
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
        async fn eject(&self, _object_path: &str, _drive_object_path: Option<&str>) -> anyhow::Result<()> {
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

    #[gpui::test]
    async fn unmount_clears_mount_point_on_success(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        cx.update(|cx| {
            cx.update_global::<DeviceStore, _>(|store, _cx| {
                store.devices.push(Device {
                    object_path: "/o/1".into(),
                    label: "USB".into(),
                    device_node: "/dev/sdb1".into(),
                    mount_point: Some(PathBuf::from("/run/media/neo/USB")),
                    is_removable: true,
                    size_bytes: 42,
                    drive_object_path: None,
                });
            });
        });
        let backend: Arc<dyn DeviceBackend> = Arc::new(MountOkBackend {
            mounted_at: PathBuf::from("/run/media/neo/USB"),
        });

        // The sidebar's unmount click target dispatches exactly this call.
        cx.update(|cx| DeviceStore::unmount(cx, backend, "/o/1".to_string()));
        cx.run_until_parked();
        cx.update(|cx| {
            let store = current(cx);
            assert!(store.last_error.is_none(), "success clears the error");
            assert_eq!(store.devices[0].mount_point, None, "mount point cleared");
        });
    }

    /// Counts `mount`/`unmount`/`eject` invocations so the in-flight guards can
    /// assert that duplicate clicks never reach the backend; `last_eject` also
    /// records the args an eject call was forwarded with.
    struct CountingBackend {
        mount_calls: StdMutex<usize>,
        unmount_calls: StdMutex<usize>,
        eject_calls: StdMutex<usize>,
        last_eject: StdMutex<Option<(String, Option<String>)>>,
    }

    #[async_trait::async_trait]
    impl DeviceBackend for CountingBackend {
        async fn list(&self) -> anyhow::Result<Vec<Device>> {
            Ok(vec![])
        }
        async fn mount(&self, _object_path: &str) -> anyhow::Result<PathBuf> {
            *self.mount_calls.lock().unwrap() += 1;
            Ok(PathBuf::from("/run/media/neo/USB"))
        }
        async fn unmount(&self, _object_path: &str) -> anyhow::Result<()> {
            *self.unmount_calls.lock().unwrap() += 1;
            Ok(())
        }
        async fn eject(&self, object_path: &str, drive_object_path: Option<&str>) -> anyhow::Result<()> {
            *self.eject_calls.lock().unwrap() += 1;
            *self.last_eject.lock().unwrap() = Some((
                object_path.to_string(),
                drive_object_path.map(str::to_string),
            ));
            Ok(())
        }
    }

    #[gpui::test]
    async fn mount_and_navigate_drops_duplicate_while_in_flight(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        let backend = Arc::new(CountingBackend {
            mount_calls: StdMutex::new(0),
            unmount_calls: StdMutex::new(0),
            eject_calls: StdMutex::new(0),
            last_eject: StdMutex::new(None),
        });
        // Keep a concrete handle for the counter assertions below.
        let counters = backend.clone();
        let backend: Arc<dyn DeviceBackend> = backend;
        let callbacks = Arc::new(StdMutex::new(0usize));

        // Two rapid clicks on the same unmounted volume, same frame: the first
        // call registers the path in `mounting` synchronously, so the second
        // (executor hasn't run yet — the mount is still in flight) must be
        // dropped and must not fire its navigate callback.
        cx.update(|cx| {
            for _ in 0..2 {
                let cb = callbacks.clone();
                DeviceStore::mount_and_navigate(
                    cx,
                    backend.clone(),
                    "/o/1".to_string(),
                    move |_cx, _path| *cb.lock().unwrap() += 1,
                );
            }
        });

        cx.run_until_parked();
        let counters = counters.clone();
        assert_eq!(
            *counters.mount_calls.lock().unwrap(),
            1,
            "duplicate mount never reaches the backend"
        );
        assert_eq!(
            *callbacks.lock().unwrap(),
            1,
            "only the accepted click's navigate callback fires"
        );

        // The guard releases once the operation settles: a later click mounts
        // again instead of being stuck permanently.
        cx.update(|cx| {
            let cb = callbacks.clone();
            DeviceStore::mount_and_navigate(
                cx,
                backend.clone(),
                "/o/1".to_string(),
                move |_cx, _path| *cb.lock().unwrap() += 1,
            );
        });
        cx.run_until_parked();
        assert_eq!(
            *counters.mount_calls.lock().unwrap(),
            2,
            "guard releases once the operation settles"
        );
        assert_eq!(*callbacks.lock().unwrap(), 2);
    }

    #[gpui::test]
    async fn unmount_drops_duplicate_while_in_flight(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        cx.update(|cx| {
            cx.update_global::<DeviceStore, _>(|store, _cx| {
                store.devices.push(Device {
                    object_path: "/o/1".into(),
                    label: "USB".into(),
                    device_node: "/dev/sdb1".into(),
                    mount_point: Some(PathBuf::from("/run/media/neo/USB")),
                    is_removable: true,
                    size_bytes: 42,
                    drive_object_path: None,
                });
            });
        });
        let backend = Arc::new(CountingBackend {
            mount_calls: StdMutex::new(0),
            unmount_calls: StdMutex::new(0),
            eject_calls: StdMutex::new(0),
            last_eject: StdMutex::new(None),
        });
        // Keep a concrete handle for the counter assertions below.
        let counters = backend.clone();
        let backend: Arc<dyn DeviceBackend> = backend;

        // Two rapid unmount clicks, same frame: only the first reaches the
        // backend; the second would otherwise error on an already-unmounted
        // volume and surface a spurious "Unmount failed" line.
        cx.update(|cx| {
            DeviceStore::unmount(cx, backend.clone(), "/o/1".to_string());
            DeviceStore::unmount(cx, backend.clone(), "/o/1".to_string());
        });
        cx.run_until_parked();

        assert_eq!(
            *counters.unmount_calls.lock().unwrap(),
            1,
            "duplicate unmount never reaches the backend"
        );
        cx.update(|cx| {
            let store = current(cx);
            assert!(store.last_error.is_none(), "no spurious error from the dropped click");
            assert_eq!(store.devices[0].mount_point, None);
        });

        // The guard releases once the operation settles: a later unmount click
        // reaches the backend again (no permanently dead unmount clicks).
        cx.update(|cx| DeviceStore::unmount(cx, backend.clone(), "/o/1".to_string()));
        cx.run_until_parked();
        assert_eq!(
            *counters.unmount_calls.lock().unwrap(),
            2,
            "guard releases once the operation settles"
        );
    }

    #[gpui::test]
    async fn eject_forwards_drive_path_and_clears_mount_point(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        cx.update(|cx| {
            cx.update_global::<DeviceStore, _>(|store, _cx| {
                store.devices.push(Device {
                    object_path: "/o/1".into(),
                    label: "USB".into(),
                    device_node: "/dev/sdb1".into(),
                    mount_point: Some(PathBuf::from("/run/media/neo/USB")),
                    is_removable: true,
                    size_bytes: 42,
                    drive_object_path: Some(
                        "/org/freedesktop/UDisks2/drives/USB_Stick".into(),
                    ),
                });
            });
        });
        let backend = Arc::new(CountingBackend {
            mount_calls: StdMutex::new(0),
            unmount_calls: StdMutex::new(0),
            eject_calls: StdMutex::new(0),
            last_eject: StdMutex::new(None),
        });
        // Keep a concrete handle for the counter assertions below.
        let counters = backend.clone();
        let backend: Arc<dyn DeviceBackend> = backend;

        // The sidebar's eject click target dispatches exactly this call.
        cx.update(|cx| {
            DeviceStore::eject(
                cx,
                backend.clone(),
                "/o/1".to_string(),
                Some("/org/freedesktop/UDisks2/drives/USB_Stick".to_string()),
            );
        });
        cx.run_until_parked();

        assert_eq!(*counters.eject_calls.lock().unwrap(), 1);
        assert_eq!(
            *counters.last_eject.lock().unwrap(),
            Some((
                "/o/1".to_string(),
                Some("/org/freedesktop/UDisks2/drives/USB_Stick".to_string()),
            )),
            "the resolved Drive path is forwarded so Drive.Eject targets the Drive object"
        );
        cx.update(|cx| {
            let store = current(cx);
            assert!(store.last_error.is_none(), "success clears the error");
            assert_eq!(store.devices[0].mount_point, None, "eject unmounts the volume");
        });
    }

    #[gpui::test]
    async fn eject_drops_duplicate_while_in_flight(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        let backend = Arc::new(CountingBackend {
            mount_calls: StdMutex::new(0),
            unmount_calls: StdMutex::new(0),
            eject_calls: StdMutex::new(0),
            last_eject: StdMutex::new(None),
        });
        // Keep a concrete handle for the counter assertions below.
        let counters = backend.clone();
        let backend: Arc<dyn DeviceBackend> = backend;
        let drive_path = Some("/org/freedesktop/UDisks2/drives/USB_Stick".to_string());

        // Two rapid eject clicks, same frame: only the first reaches the backend.
        cx.update(|cx| {
            DeviceStore::eject(cx, backend.clone(), "/o/1".to_string(), drive_path.clone());
            DeviceStore::eject(cx, backend.clone(), "/o/1".to_string(), drive_path.clone());
        });
        cx.run_until_parked();
        assert_eq!(
            *counters.eject_calls.lock().unwrap(),
            1,
            "duplicate eject never reaches the backend"
        );

        // The guard releases once the operation settles.
        cx.update(|cx| {
            DeviceStore::eject(cx, backend.clone(), "/o/1".to_string(), drive_path.clone());
        });
        cx.run_until_parked();
        assert_eq!(
            *counters.eject_calls.lock().unwrap(),
            2,
            "guard releases once the operation settles"
        );
    }

    #[gpui::test]
    async fn eject_is_dropped_while_mount_is_in_flight(cx: &mut TestAppContext) {
        cx.update(|cx| init(cx));
        let backend = Arc::new(CountingBackend {
            mount_calls: StdMutex::new(0),
            unmount_calls: StdMutex::new(0),
            eject_calls: StdMutex::new(0),
            last_eject: StdMutex::new(None),
        });
        // Keep a concrete handle for the counter assertions below.
        let counters = backend.clone();
        let backend: Arc<dyn DeviceBackend> = backend;
        let drive_path = Some("/org/freedesktop/UDisks2/drives/USB_Stick".to_string());

        // Mount first, then eject the same volume in the same frame: the mount
        // is still in flight (registered synchronously), so the eject must be
        // dropped — `Drive.Eject` racing `Filesystem.Mount` on the real backend
        // could eject a drive mid-mount.
        cx.update(|cx| {
            DeviceStore::mount_and_navigate(cx, backend.clone(), "/o/1".to_string(), |_cx, _path| {});
            DeviceStore::eject(cx, backend.clone(), "/o/1".to_string(), drive_path.clone());
        });
        cx.run_until_parked();

        assert_eq!(*counters.mount_calls.lock().unwrap(), 1);
        assert_eq!(
            *counters.eject_calls.lock().unwrap(),
            0,
            "eject dropped while the volume is still mounting"
        );
    }
}
