//! GPUI global holding the live removable-media device list (T001/T002
//! sibling infra; spec: docs/superpowers/specs/2026-08-06-removable-media-mount.md).

use chronos_fm_services::devices::Device;
use gpui::{App, Global};

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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{BorrowAppContext, TestAppContext};

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
                store.devices.push(chronos_fm_services::devices::Device {
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
}
