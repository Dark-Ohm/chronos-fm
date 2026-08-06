use std::collections::HashMap;
use std::path::PathBuf;

/// A value read from a `udisks2` D-Bus property. Kept independent of
/// `zvariant` so this parser has zero D-Bus dependency and stays trivially
/// unit-testable; the real backend (Task 2) converts `zvariant::OwnedValue`
/// into this at the I/O boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceValue {
    Bool(bool),
    U64(u64),
    Str(String),
    ObjectPath(String),
    /// `ay` — byte arrays udisks2 uses for NUL-terminated C strings
    /// (`Block.Device`) and for mount point lists (one path, NUL-joined
    /// — see the `unmounted` variant handling below for the multi-mount
    /// case, which v1 collapses to "first mount point").
    Bytes(Vec<u8>),
}

/// Raw shape of `org.freedesktop.DBus.ObjectManager.GetManagedObjects`:
/// object path -> interface name -> property name -> value.
pub type ManagedObjects = HashMap<String, HashMap<String, HashMap<String, DeviceValue>>>;

/// A single storage volume, filtered and flattened for UI display.
#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    /// The `udisks2` object path of the `Block`/`Filesystem` object — the
    /// stable key used for mount/unmount/eject calls (Task 2).
    pub object_path: String,
    pub label: String,
    pub device_node: String,
    pub mount_point: Option<PathBuf>,
    pub is_removable: bool,
    pub size_bytes: u64,
    /// Object path of the owning `org.freedesktop.UDisks2.Drive` object
    /// (from the `Block.Drive` property). `Drive.Eject` lives on the Drive
    /// object, not the Block object the panel holds, so this is what `eject`
    /// must target (T008). `None` when the block device has no Drive
    /// reference (non-removable, no owning drive).
    pub drive_object_path: Option<String>,
}

fn as_bool(v: Option<&DeviceValue>) -> bool {
    matches!(v, Some(DeviceValue::Bool(true)))
}

fn as_u64(v: Option<&DeviceValue>) -> u64 {
    match v {
        Some(DeviceValue::U64(n)) => *n,
        _ => 0,
    }
}

fn bytes_to_string(v: Option<&DeviceValue>) -> String {
    match v {
        Some(DeviceValue::Bytes(b)) => String::from_utf8_lossy(b)
            .trim_end_matches('\0')
            .to_string(),
        Some(DeviceValue::Str(s)) => s.trim_end_matches('\0').to_string(),
        _ => String::new(),
    }
}

fn as_object_path(v: Option<&DeviceValue>) -> Option<String> {
    match v {
        Some(DeviceValue::ObjectPath(p)) => Some(p.clone()),
        _ => None,
    }
}

/// Flattens a raw `GetManagedObjects` snapshot into the v1 device list:
/// keeps only removable drives or non-system-hinted filesystems (spec
/// §2 "Фильтр v1"), drops everything else (LUKS containers, unformatted
/// partitions, system/boot partitions).
pub fn parse_managed_objects(raw: &ManagedObjects) -> Vec<Device> {
    // Removable flag lives on the *Drive* object; block devices only
    // reference their drive by object path. Build a lookup first.
    let removable_drives: std::collections::HashSet<&str> = raw
        .iter()
        .filter(|(_, ifaces)| {
            ifaces
                .get("org.freedesktop.UDisks2.Drive")
                .is_some_and(|props| as_bool(props.get("Removable")))
        })
        .map(|(path, _)| path.as_str())
        .collect();

    let mut devices = Vec::new();

    for (object_path, ifaces) in raw {
        let Some(block) = ifaces.get("org.freedesktop.UDisks2.Block") else {
            continue; // Drive-only objects (no partition) aren't listed directly.
        };
        let Some(fs) = ifaces.get("org.freedesktop.UDisks2.Filesystem") else {
            continue; // v1 only lists mountable filesystems.
        };

        let hint_system = as_bool(block.get("HintSystem"));
        let drive_path = as_object_path(block.get("Drive"));
        let is_removable = drive_path
            .as_deref()
            .is_some_and(|p| removable_drives.contains(p));

        if hint_system && !is_removable {
            continue; // Filtered out per spec: system-hinted, non-removable.
        }

        let device_node = bytes_to_string(block.get("Device"));
        let label = {
            let l = bytes_to_string(block.get("IdLabel"));
            if l.is_empty() {
                device_node.clone()
            } else {
                l
            }
        };
        let mount_point = {
            let raw_mp = bytes_to_string(fs.get("MountPoints"));
            if raw_mp.is_empty() {
                None
            } else {
                Some(PathBuf::from(raw_mp))
            }
        };

        devices.push(Device {
            object_path: object_path.clone(),
            label,
            device_node,
            mount_point,
            is_removable,
            size_bytes: as_u64(block.get("Size")),
            drive_object_path: drive_path,
        });
    }

    devices
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn props(pairs: &[(&str, DeviceValue)]) -> HashMap<String, DeviceValue> {
        pairs.iter().cloned().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn parses_removable_usb_partition_with_mount_point() {
        let mut raw: ManagedObjects = HashMap::new();

        // Drive object: removable, no filesystem interface itself.
        raw.insert(
            "/org/freedesktop/UDisks2/drives/USB_Stick".to_string(),
            {
                let mut ifaces = HashMap::new();
                ifaces.insert(
                    "org.freedesktop.UDisks2.Drive".to_string(),
                    props(&[("Removable", DeviceValue::Bool(true))]),
                );
                ifaces
            },
        );

        // Block+Filesystem object: the actual partition, points back at the drive.
        raw.insert(
            "/org/freedesktop/UDisks2/block_devices/sdb1".to_string(),
            {
                let mut ifaces = HashMap::new();
                ifaces.insert(
                    "org.freedesktop.UDisks2.Block".to_string(),
                    props(&[
                        ("Device", DeviceValue::Bytes(b"/dev/sdb1\0".to_vec())),
                        ("IdLabel", DeviceValue::Str("MY_USB".to_string())),
                        ("Size", DeviceValue::U64(8_000_000_000)),
                        ("HintSystem", DeviceValue::Bool(false)),
                        (
                            "Drive",
                            DeviceValue::ObjectPath(
                                "/org/freedesktop/UDisks2/drives/USB_Stick".to_string(),
                            ),
                        ),
                    ]),
                );
                ifaces.insert(
                    "org.freedesktop.UDisks2.Filesystem".to_string(),
                    props(&[(
                        "MountPoints",
                        DeviceValue::Str("/run/media/neo/MY_USB\0".to_string()),
                    )]),
                );
                ifaces
            },
        );

        let devices = parse_managed_objects(&raw);

        assert_eq!(devices.len(), 1);
        let d = &devices[0];
        assert_eq!(d.label, "MY_USB");
        assert_eq!(d.device_node, "/dev/sdb1");
        assert_eq!(d.size_bytes, 8_000_000_000);
        assert!(d.is_removable);
        assert_eq!(d.mount_point.as_deref(), Some(std::path::Path::new("/run/media/neo/MY_USB")));
    }

    #[test]
    fn excludes_system_hinted_internal_partition() {
        let mut raw: ManagedObjects = HashMap::new();
        raw.insert(
            "/org/freedesktop/UDisks2/block_devices/sda1".to_string(),
            {
                let mut ifaces = HashMap::new();
                ifaces.insert(
                    "org.freedesktop.UDisks2.Block".to_string(),
                    props(&[
                        ("Device", DeviceValue::Bytes(b"/dev/sda1\0".to_vec())),
                        ("IdLabel", DeviceValue::Str("boot".to_string())),
                        ("Size", DeviceValue::U64(500_000_000)),
                        ("HintSystem", DeviceValue::Bool(true)),
                    ]),
                );
                ifaces.insert(
                    "org.freedesktop.UDisks2.Filesystem".to_string(),
                    props(&[("MountPoints", DeviceValue::Str(String::new()))]),
                );
                ifaces
            },
        );

        assert!(parse_managed_objects(&raw).is_empty());
    }

    #[test]
    fn drive_object_path_populates_from_block_drive_property() {
        let mut raw: ManagedObjects = HashMap::new();
        raw.insert(
            "/org/freedesktop/UDisks2/block_devices/sdb1".to_string(),
            {
                let mut ifaces = HashMap::new();
                ifaces.insert(
                    "org.freedesktop.UDisks2.Block".to_string(),
                    props(&[
                        ("Device", DeviceValue::Bytes(b"/dev/sdb1\0".to_vec())),
                        ("IdLabel", DeviceValue::Str("MY_USB".to_string())),
                        ("Size", DeviceValue::U64(8_000_000_000)),
                        ("HintSystem", DeviceValue::Bool(false)),
                        (
                            "Drive",
                            DeviceValue::ObjectPath(
                                "/org/freedesktop/UDisks2/drives/USB_Stick".to_string(),
                            ),
                        ),
                    ]),
                );
                ifaces.insert(
                    "org.freedesktop.UDisks2.Filesystem".to_string(),
                    props(&[("MountPoints", DeviceValue::Str(String::new()))]),
                );
                ifaces
            },
        );
        // No drive object needed for this test — we only assert the resolved
        // drive path carried on the Device.

        let devices = parse_managed_objects(&raw);
        assert_eq!(devices.len(), 1);
        assert_eq!(
            devices[0].drive_object_path.as_deref(),
            Some("/org/freedesktop/UDisks2/drives/USB_Stick")
        );
    }

    #[test]
    fn drive_object_path_is_none_when_block_has_no_drive() {
        let mut raw: ManagedObjects = HashMap::new();
        raw.insert(
            "/org/freedesktop/UDisks2/block_devices/sdc1".to_string(),
            {
                let mut ifaces = HashMap::new();
                ifaces.insert(
                    "org.freedesktop.UDisks2.Block".to_string(),
                    props(&[
                        ("Device", DeviceValue::Bytes(b"/dev/sdc1\0".to_vec())),
                        ("IdLabel", DeviceValue::Str("BACKUP".to_string())),
                        ("Size", DeviceValue::U64(1_000_000_000)),
                        ("HintSystem", DeviceValue::Bool(false)),
                    ]),
                );
                ifaces.insert(
                    "org.freedesktop.UDisks2.Filesystem".to_string(),
                    props(&[("MountPoints", DeviceValue::Str(String::new()))]),
                );
                ifaces
            },
        );

        let devices = parse_managed_objects(&raw);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].drive_object_path, None);
    }

    #[test]
    fn unmounted_device_has_none_mount_point() {
        let mut raw: ManagedObjects = HashMap::new();
        raw.insert(
            "/org/freedesktop/UDisks2/block_devices/sdc1".to_string(),
            {
                let mut ifaces = HashMap::new();
                ifaces.insert(
                    "org.freedesktop.UDisks2.Block".to_string(),
                    props(&[
                        ("Device", DeviceValue::Bytes(b"/dev/sdc1\0".to_vec())),
                        ("IdLabel", DeviceValue::Str("BACKUP".to_string())),
                        ("Size", DeviceValue::U64(1_000_000_000)),
                        ("HintSystem", DeviceValue::Bool(false)),
                        (
                            "Drive",
                            DeviceValue::ObjectPath(
                                "/org/freedesktop/UDisks2/drives/Backup_Drive".to_string(),
                            ),
                        ),
                    ]),
                );
                ifaces.insert(
                    "org.freedesktop.UDisks2.Filesystem".to_string(),
                    props(&[("MountPoints", DeviceValue::Str(String::new()))]),
                );
                ifaces
            },
        );
        raw.insert(
            "/org/freedesktop/UDisks2/drives/Backup_Drive".to_string(),
            {
                let mut ifaces = HashMap::new();
                ifaces.insert(
                    "org.freedesktop.UDisks2.Drive".to_string(),
                    props(&[("Removable", DeviceValue::Bool(true))]),
                );
                ifaces
            },
        );

        let devices = parse_managed_objects(&raw);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].mount_point, None);
    }
}
