# Removable Media — Devices Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a live "Devices" sidebar section that lists removable/internal
storage volumes via `udisks2` over D-Bus, lets the user mount/navigate/
unmount/eject them, and updates itself on hotplug (no manual refresh).

**Architecture:** A pure, unit-testable parser turns raw D-Bus
`GetManagedObjects` output into `Vec<Device>`. A thin `zbus`-backed I/O
layer (behind a `DeviceBackend` trait so mount/unmount logic stays
testable against a fake) owns the actual system-bus calls. A GPUI
`Global` (`DeviceStore`) holds the current device list; a background
task started at app init keeps it live via `InterfacesAdded`/
`InterfacesRemoved` signals. The sidebar reads the global and renders a
new section using the T002 `elevated_card`/`section_header` patterns.

**Tech Stack:** `zbus` (system bus client, no `tokio` dependency), GPUI
`Global`/`cx.background_spawn`/`cx.spawn` (existing project convention —
see `crates/chronos-fm-pages/src/explorer/search.rs::trigger_search` for
the reference idiom), `chronos-fm-services` (new `devices` module).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-06-removable-media-mount.md`.
- No `tokio` in the app core (P2 milestone, already enforced project-wide
  — see `crates/chronos-fm-services/src/search/engine.rs` comment). All
  async work goes through GPUI's own executor (`cx.background_spawn`/
  `cx.spawn`), never `tokio::spawn`.
- `unsafe_code = deny`, `clippy::unwrap_used`/`expect_used = warn` outside
  `#[cfg(test)]` (workspace lints, `crates/chronos-fm-services/Cargo.toml`
  already has `[lints] workspace = true`).
- Every new `pub` item needs a doc comment (`missing_docs = "warn"`).
- v1 filter: `Drive.Removable == true` **or** (`Filesystem` interface
  present **and** `Block.HintSystem == false`). Exclude LUKS-encrypted,
  unformatted, and system-hinted volumes from v1 — do not attempt to
  handle them, just don't list them.
- No AI trailers in commits (repo convention, check `git log` if unsure).
- Filesystem/D-Bus mutation only through the new `devices` module — never
  shell out to `mount`/`umount`/`eject` directly.

---

### Task 1: `Device` model + pure ManagedObjects parser

**Files:**
- Create: `crates/chronos-fm-services/src/devices/mod.rs`
- Create: `crates/chronos-fm-services/src/devices/parse.rs`
- Test: inline `#[cfg(test)]` in `parse.rs`
- Modify: `crates/chronos-fm-services/src/chronos_fm_services.rs` (add `pub mod devices;`)
- Modify: `crates/chronos-fm-services/Cargo.toml` (add `zbus`, run `cargo add zbus -p chronos-fm-services` so the version resolves to whatever is current on crates.io — do not hand-pin a version number)

**Interfaces:**
- Produces: `pub struct Device { pub object_path: String, pub label: String, pub device_node: String, pub mount_point: Option<PathBuf>, pub is_removable: bool, pub size_bytes: u64 }` (all fields `pub`, `#[derive(Debug, Clone, PartialEq)]`)
- Produces: `pub fn parse_managed_objects(raw: &ManagedObjects) -> Vec<Device>` where `pub type ManagedObjects = std::collections::HashMap<String, std::collections::HashMap<String, std::collections::HashMap<String, DeviceValue>>>` and `pub enum DeviceValue { Bool(bool), U64(u64), Str(String), ObjectPath(String), Bytes(Vec<u8>) }` — a small local value enum so this file has **zero** dependency on `zbus`/`zvariant` types, keeping the parser trivially unit-testable. Task 2's real backend converts `zvariant::OwnedValue` into `DeviceValue` at the I/O boundary.

- [ ] **Step 1: Write the failing test**

```rust
// crates/chronos-fm-services/src/devices/parse.rs
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-fm-services devices::parse:: 2>&1`
Expected: FAIL — `parse_managed_objects`, `Device`, `DeviceValue`, `ManagedObjects` not defined (module doesn't exist yet).

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/chronos-fm-services/src/devices/parse.rs
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
        });
    }

    devices
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-fm-services devices::parse:: 2>&1`
Expected: PASS — 3 tests (`parses_removable_usb_partition_with_mount_point`, `excludes_system_hinted_internal_partition`, `unmounted_device_has_none_mount_point`).

- [ ] **Step 5: Wire the module up**

```rust
// crates/chronos-fm-services/src/devices/mod.rs
//! Removable/internal storage device listing and mount control via
//! `udisks2` (spec: docs/superpowers/specs/2026-08-06-removable-media-mount.md).

mod parse;
pub use parse::{Device, DeviceValue, ManagedObjects, parse_managed_objects};
```

Add to `crates/chronos-fm-services/src/chronos_fm_services.rs`:

```rust
pub mod devices;
```

Add `zbus` (run this, don't hand-pin a version):

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
cargo add zbus -p chronos-fm-services --no-default-features --features tokio=false 2>&1 || cargo add zbus -p chronos-fm-services
```

(zbus defaults to its own `async-io` executor when the `tokio` feature is
off — verify with `cargo tree -p chronos-fm-services -i zbus` that no
`tokio` crate is pulled in; if it is, add `--no-default-features` and
re-check the feature list `cargo add zbus --dry-run` prints.)

- [ ] **Step 6: Commit**

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
git add crates/chronos-fm-services/src/devices/ crates/chronos-fm-services/src/chronos_fm_services.rs crates/chronos-fm-services/Cargo.toml Cargo.lock
git commit -m "services: device model + pure udisks2 ManagedObjects parser"
```

---

### Task 2: `DeviceBackend` trait + real `zbus`/`udisks2` implementation

**Files:**
- Create: `crates/chronos-fm-services/src/devices/backend.rs`
- Modify: `crates/chronos-fm-services/src/devices/mod.rs` (export)

**Interfaces:**
- Consumes: `Device`, `DeviceValue`, `ManagedObjects`, `parse_managed_objects` (Task 1)
- Produces:
  ```rust
  #[async_trait::async_trait]
  pub trait DeviceBackend: Send + Sync {
      async fn list(&self) -> anyhow::Result<Vec<Device>>;
      async fn mount(&self, object_path: &str) -> anyhow::Result<PathBuf>;
      async fn unmount(&self, object_path: &str) -> anyhow::Result<()>;
      async fn eject(&self, object_path: &str) -> anyhow::Result<()>;
  }
  pub struct UDisks2Backend { connection: zbus::Connection }
  impl UDisks2Backend {
      pub async fn connect() -> anyhow::Result<Self>;
  }
  ```
  (`UDisks2Backend` implements `DeviceBackend`.) Later tasks depend on
  the trait, not the concrete type, so `DeviceStore` (Task 3) and its
  tests can use a fake.

This task needs `async-trait` (`cargo add async-trait -p chronos-fm-services`)
and `zvariant` (pulled in transitively by `zbus`, re-exported as
`zbus::zvariant`).

- [ ] **Step 1: Write the failing test (fake backend contract test)**

```rust
// crates/chronos-fm-services/src/devices/backend.rs — bottom of file
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// In-memory fake used by this file's own tests and by Task 3/4's
    /// `DeviceStore` tests — kept `pub(crate)` so both can import it.
    pub(crate) struct FakeBackend {
        pub devices: Mutex<Vec<Device>>,
        pub mount_result: Mutex<anyhow::Result<PathBuf>>,
    }

    #[async_trait::async_trait]
    impl DeviceBackend for FakeBackend {
        async fn list(&self) -> anyhow::Result<Vec<Device>> {
            Ok(self.devices.lock().unwrap().clone())
        }
        async fn mount(&self, object_path: &str) -> anyhow::Result<PathBuf> {
            let mut result = self.mount_result.lock().unwrap();
            let taken = std::mem::replace(&mut *result, Err(anyhow::anyhow!("consumed")));
            let path = taken?;
            if let Some(d) = self
                .devices
                .lock()
                .unwrap()
                .iter_mut()
                .find(|d| d.object_path == object_path)
            {
                d.mount_point = Some(path.clone());
            }
            Ok(path)
        }
        async fn unmount(&self, object_path: &str) -> anyhow::Result<()> {
            if let Some(d) = self
                .devices
                .lock()
                .unwrap()
                .iter_mut()
                .find(|d| d.object_path == object_path)
            {
                d.mount_point = None;
            }
            Ok(())
        }
        async fn eject(&self, _object_path: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fake_backend_mount_updates_mount_point() {
        let backend = FakeBackend {
            devices: Mutex::new(vec![Device {
                object_path: "/o/1".into(),
                label: "USB".into(),
                device_node: "/dev/sdb1".into(),
                mount_point: None,
                is_removable: true,
                size_bytes: 100,
            }]),
            mount_result: Mutex::new(Ok(PathBuf::from("/run/media/neo/USB"))),
        };
        let path = backend.mount("/o/1").await.unwrap();
        assert_eq!(path, PathBuf::from("/run/media/neo/USB"));
        let devices = backend.list().await.unwrap();
        assert_eq!(devices[0].mount_point, Some(PathBuf::from("/run/media/neo/USB")));
    }
}
```

Note: this one test uses `#[tokio::test]` purely as a lightweight async
test runner for the *fake* (no I/O, no D-Bus, no GPUI) — it does not add
a `tokio` runtime dependency to the production binary, only to
`[dev-dependencies]`. Run `cargo add tokio -p chronos-fm-services --dev --features rt,macros`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-fm-services devices::backend:: 2>&1`
Expected: FAIL — `DeviceBackend`, `FakeBackend` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/chronos-fm-services/src/devices/backend.rs
use super::parse::{Device, DeviceValue, ManagedObjects, parse_managed_objects};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use zbus::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

const SERVICE: &str = "org.freedesktop.UDisks2";
const MANAGER_PATH: &str = "/org/freedesktop/UDisks2";

/// Storage backend for the Devices panel. `UDisks2Backend` is the real
/// implementation; tests use an in-memory fake (see `tests::FakeBackend`)
/// so `DeviceStore` logic (Task 3/4) never needs a live D-Bus connection.
#[async_trait::async_trait]
pub trait DeviceBackend: Send + Sync {
    async fn list(&self) -> Result<Vec<Device>>;
    async fn mount(&self, object_path: &str) -> Result<PathBuf>;
    async fn unmount(&self, object_path: &str) -> Result<()>;
    async fn eject(&self, object_path: &str) -> Result<()>;
}

/// Real `udisks2` backend over the system D-Bus.
pub struct UDisks2Backend {
    connection: Connection,
}

impl UDisks2Backend {
    /// Connects to the system bus. Fails if `udisks2` isn't running or the
    /// system bus is unreachable (headless/container environments) —
    /// callers should treat this as "Devices panel unavailable", not a
    /// hard app-startup error.
    pub async fn connect() -> Result<Self> {
        let connection = Connection::system()
            .await
            .context("connecting to the D-Bus system bus")?;
        Ok(Self { connection })
    }

    async fn object_manager_proxy(&self) -> Result<zbus::Proxy<'_>> {
        zbus::Proxy::new(
            &self.connection,
            SERVICE,
            MANAGER_PATH,
            "org.freedesktop.DBus.ObjectManager",
        )
        .await
        .context("building ObjectManager proxy")
    }

    fn value_to_device_value(v: &OwnedValue) -> Option<DeviceValue> {
        let v: &Value = v;
        match v {
            Value::Bool(b) => Some(DeviceValue::Bool(*b)),
            Value::U64(n) => Some(DeviceValue::U64(*n)),
            Value::Str(s) => Some(DeviceValue::Str(s.to_string())),
            Value::ObjectPath(p) => Some(DeviceValue::ObjectPath(p.to_string())),
            Value::Array(arr) if arr.element_signature().as_str() == "y" => {
                let bytes: Vec<u8> = arr
                    .iter()
                    .filter_map(|el| u8::try_from(el.clone()).ok())
                    .collect();
                Some(DeviceValue::Bytes(bytes))
            }
            // `MountPoints` is `aay` (array of byte-arrays); v1 only needs
            // the first mount point, so unwrap one level here.
            Value::Array(arr) if arr.element_signature().as_str() == "ay" => {
                let first = arr.iter().next()?;
                let inner: zbus::zvariant::Array = first.clone().try_into().ok()?;
                let bytes: Vec<u8> = inner
                    .iter()
                    .filter_map(|el| u8::try_from(el.clone()).ok())
                    .collect();
                Some(DeviceValue::Bytes(bytes))
            }
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl DeviceBackend for UDisks2Backend {
    async fn list(&self) -> Result<Vec<Device>> {
        let proxy = self.object_manager_proxy().await?;
        let raw: HashMap<
            OwnedObjectPath,
            HashMap<String, HashMap<String, OwnedValue>>,
        > = proxy
            .call_method("GetManagedObjects", &())
            .await
            .context("calling GetManagedObjects")?
            .body()
            .deserialize()
            .context("decoding GetManagedObjects reply")?;

        let mut converted: ManagedObjects = HashMap::new();
        for (object_path, ifaces) in raw {
            let mut converted_ifaces = HashMap::new();
            for (iface, props) in ifaces {
                let mut converted_props = HashMap::new();
                for (key, value) in props {
                    if let Some(v) = Self::value_to_device_value(&value) {
                        converted_props.insert(key, v);
                    }
                }
                converted_ifaces.insert(iface, converted_props);
            }
            converted.insert(object_path.to_string(), converted_ifaces);
        }

        Ok(parse_managed_objects(&converted))
    }

    async fn mount(&self, object_path: &str) -> Result<PathBuf> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            SERVICE,
            object_path,
            "org.freedesktop.UDisks2.Filesystem",
        )
        .await
        .context("building Filesystem proxy")?;
        let options: HashMap<&str, Value> = HashMap::new();
        let reply = proxy
            .call_method("Mount", &(options,))
            .await
            .context("calling Filesystem.Mount")?;
        let mount_path: String = reply.body().deserialize().context("decoding Mount reply")?;
        Ok(PathBuf::from(mount_path))
    }

    async fn unmount(&self, object_path: &str) -> Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            SERVICE,
            object_path,
            "org.freedesktop.UDisks2.Filesystem",
        )
        .await
        .context("building Filesystem proxy")?;
        let options: HashMap<&str, Value> = HashMap::new();
        proxy
            .call_method("Unmount", &(options,))
            .await
            .context("calling Filesystem.Unmount")?;
        Ok(())
    }

    async fn eject(&self, object_path: &str) -> Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            SERVICE,
            object_path,
            "org.freedesktop.UDisks2.Drive",
        )
        .await
        .context("building Drive proxy")?;
        let options: HashMap<&str, Value> = HashMap::new();
        proxy
            .call_method("Eject", &(options,))
            .await
            .context("calling Drive.Eject")?;
        Ok(())
    }
}
```

**Note for the implementer:** `zbus`'s exact `Proxy`/`Message::body().deserialize()`
API shape drifts between major versions. Run `cargo doc -p zbus --open`
(or check docs.rs for the version `cargo add` resolved in Task 1) before
this step and adjust method names if they've moved — the interface
names/method names/signatures (`GetManagedObjects`, `Mount`, `Unmount`,
`Eject`, and the `a{oa{sa{sv}}}` shape) are the stable, versioned D-Bus
API and will not have changed; only the Rust binding surface might.

Add to `devices/mod.rs`:

```rust
mod backend;
pub use backend::{DeviceBackend, UDisks2Backend};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-fm-services devices:: 2>&1`
Expected: PASS — Task 1's 3 parser tests + this task's `fake_backend_mount_updates_mount_point`.

- [ ] **Step 5: Build check (real backend compiles; not exercised without a live bus)**

Run: `cargo build -p chronos-fm-services 2>&1`
Expected: exit 0. No test calls `UDisks2Backend::connect()` — that only
runs against a real system bus, verified live per the spec's
verification section, not in CI.

- [ ] **Step 6: Commit**

```bash
git add crates/chronos-fm-services/src/devices/backend.rs crates/chronos-fm-services/src/devices/mod.rs crates/chronos-fm-services/Cargo.toml Cargo.lock
git commit -m "services: DeviceBackend trait + real udisks2/zbus implementation"
```

---

### Task 3: `DeviceStore` GPUI global

**Files:**
- Create: `crates/chronos-fm-ui/src/devices_store.rs`
- Modify: `crates/chronos-fm-ui/src/chronos_fm_ui.rs` (add `pub mod devices_store;`)
- Modify: `crates/chronos-fm-ui/Cargo.toml` (add `chronos-fm-services = { path = "../chronos-fm-services", features = ["gui"] }` if not already a dependency — check first with `grep chronos-fm-services crates/chronos-fm-ui/Cargo.toml`)

**Interfaces:**
- Consumes: `chronos_fm_services::devices::{Device, DeviceBackend}` (Tasks 1–2)
- Produces:
  ```rust
  pub struct DeviceStore { pub devices: Vec<Device>, pub last_error: Option<String> }
  impl gpui::Global for DeviceStore {}
  pub fn init(cx: &mut gpui::App); // sets DeviceStore::default() as the global
  pub fn current(cx: &gpui::App) -> &DeviceStore; // cx.global::<DeviceStore>()
  ```
  `DeviceStore::default()` is `{ devices: vec![], last_error: None }`.
  Later tasks (4, 5, 6) read `chronos_fm_ui::devices_store::current(cx)`
  and write via `cx.update_global::<DeviceStore, _>(...)`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/chronos-fm-ui/src/devices_store.rs — bottom of file
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-fm-ui devices_store:: 2>&1`
Expected: FAIL — module/`init`/`current`/`DeviceStore` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/chronos-fm-ui/src/devices_store.rs
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-fm-ui devices_store:: 2>&1`
Expected: PASS — `init_registers_empty_store`, `update_global_replaces_device_list`.

- [ ] **Step 5: Commit**

```bash
git add crates/chronos-fm-ui/src/devices_store.rs crates/chronos-fm-ui/src/chronos_fm_ui.rs crates/chronos-fm-ui/Cargo.toml Cargo.lock
git commit -m "ui: DeviceStore global for the removable-media device list"
```

---

### Task 4: Mount/unmount/eject actions with error surfacing

**Files:**
- Modify: `crates/chronos-fm-ui/src/devices_store.rs`

**Interfaces:**
- Consumes: `DeviceStore`, `current` (Task 3); `DeviceBackend` (Task 2)
- Produces:
  ```rust
  impl DeviceStore {
      pub fn mount_and_navigate(
          cx: &mut App,
          backend: std::sync::Arc<dyn chronos_fm_services::devices::DeviceBackend>,
          object_path: String,
          on_mounted: impl FnOnce(&mut App, std::path::PathBuf) + 'static,
      );
      pub fn unmount(
          cx: &mut App,
          backend: std::sync::Arc<dyn chronos_fm_services::devices::DeviceBackend>,
          object_path: String,
      );
  }
  ```
  `on_mounted` lets the sidebar (Task 6) trigger navigation without this
  module depending on `chronos-fm-pages`.

- [ ] **Step 1: Write the failing test**

```rust
// crates/chronos-fm-ui/src/devices_store.rs — inside existing #[cfg(test)] mod tests
    use chronos_fm_services::devices::DeviceBackend;
    use std::sync::{Arc, Mutex as StdMutex};

    struct MountOkBackend {
        mounted_at: std::path::PathBuf,
    }

    #[async_trait::async_trait]
    impl DeviceBackend for MountOkBackend {
        async fn list(&self) -> anyhow::Result<Vec<chronos_fm_services::devices::Device>> {
            Ok(vec![])
        }
        async fn mount(&self, _object_path: &str) -> anyhow::Result<std::path::PathBuf> {
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
            mounted_at: std::path::PathBuf::from("/run/media/neo/USB"),
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
            Some(std::path::PathBuf::from("/run/media/neo/USB"))
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-fm-ui devices_store:: 2>&1`
Expected: FAIL — `DeviceStore::mount_and_navigate` not defined.

- [ ] **Step 3: Write minimal implementation**

Add to `crates/chronos-fm-ui/src/devices_store.rs` (above the `#[cfg(test)]` block):

```rust
use chronos_fm_services::devices::DeviceBackend;
use std::path::PathBuf;
use std::sync::Arc;

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
        cx.spawn(async move |cx: &mut gpui::AsyncApp| {
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
            })
            .ok();
        })
        .detach();
    }

    /// Unmounts the device at `object_path` in the background; on success
    /// clears its `mount_point`, on failure records `last_error`.
    pub fn unmount(cx: &mut App, backend: Arc<dyn DeviceBackend>, object_path: String) {
        cx.spawn(async move |cx: &mut gpui::AsyncApp| {
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
            })
            .ok();
        })
        .detach();
    }
}
```

Add `async-trait` to `crates/chronos-fm-ui/Cargo.toml` `[dev-dependencies]`
(`cargo add async-trait -p chronos-fm-ui --dev`) for the test's `MountOkBackend`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-fm-ui devices_store:: 2>&1`
Expected: PASS — all `devices_store` tests, including
`mount_and_navigate_calls_callback_with_mount_path`.

- [ ] **Step 5: Commit**

```bash
git add crates/chronos-fm-ui/src/devices_store.rs crates/chronos-fm-ui/Cargo.toml Cargo.lock
git commit -m "ui: mount/unmount actions on DeviceStore with error surfacing"
```

---

### Task 5: Hotplug subscription wired at app startup

**Files:**
- Modify: `crates/chronos-fm/src/app.rs`
- Modify: `crates/chronos-fm/Cargo.toml` (add `chronos-fm-services` `devices` re-export if not already visible; likely already a dependency via `chronos-fm-pages`)

**Interfaces:**
- Consumes: `chronos_fm_ui::devices_store::{init, DeviceStore}` (Task 3); `chronos_fm_services::devices::{UDisks2Backend, DeviceBackend}` (Task 2)
- Produces: nothing new consumed by later tasks — this is the wiring
  that makes Tasks 1–4 actually run in the live app. No unit test (it
  talks to a real system bus); verified live per the spec.

- [ ] **Step 1: Add the startup call**

Find where `chronos_fm_pages::explorer::clipboard::init(app)` is called
in `crates/chronos-fm/src/app.rs` (added by task b1, elsewhere in this
repo's queue) and add immediately after it:

```rust
chronos_fm_ui::devices_store::init(app);
spawn_device_hotplug_watcher(app);
```

Add the function (same file, module scope):

```rust
/// Connects to `udisks2` and keeps `DeviceStore` live: an initial listing
/// on success, then a subscription to `InterfacesAdded`/`InterfacesRemoved`
/// that re-lists on every signal (simpler and more robust than diffing
/// individual signal payloads, and the list is cheap — a handful of
/// devices). If `udisks2` is unreachable (no system bus, container,
/// headless CI), logs a warning and leaves `DeviceStore` empty — the
/// Devices sidebar section (Task 6) just renders nothing, not an error.
fn spawn_device_hotplug_watcher(cx: &mut gpui::App) {
    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        let backend = match cx
            .background_spawn(async { chronos_fm_services::devices::UDisks2Backend::connect().await })
            .await
        {
            Ok(backend) => std::sync::Arc::new(backend),
            Err(error) => {
                tracing::warn!("Devices panel unavailable: {error}");
                return;
            }
        };

        let backend_for_list: std::sync::Arc<dyn chronos_fm_services::devices::DeviceBackend> =
            backend.clone();

        // Initial snapshot.
        refresh_devices(cx, backend_for_list.clone()).await;

        // Live hotplug: udisks2's ObjectManager emits InterfacesAdded/
        // InterfacesRemoved on the same object (/org/freedesktop/UDisks2)
        // this module already talks to; re-list on every signal rather
        // than parsing the signal payload incrementally.
        let connection = backend.connection().clone();
        let proxy = match zbus::fdo::ObjectManagerProxy::new(
            &connection,
            "org.freedesktop.UDisks2",
            "/org/freedesktop/UDisks2",
        )
        .await
        {
            Ok(proxy) => proxy,
            Err(error) => {
                tracing::warn!("Devices hotplug subscription unavailable: {error}");
                return;
            }
        };

        let Ok(mut added) = proxy.receive_interfaces_added().await else {
            return;
        };
        let Ok(mut removed) = proxy.receive_interfaces_removed().await else {
            return;
        };

        loop {
            futures::select_biased! {
                _ = added.next() => refresh_devices(cx, backend_for_list.clone()).await,
                _ = removed.next() => refresh_devices(cx, backend_for_list.clone()).await,
                complete => break,
            }
        }
    })
    .detach();
}

async fn refresh_devices(
    cx: &mut gpui::AsyncApp,
    backend: std::sync::Arc<dyn chronos_fm_services::devices::DeviceBackend>,
) {
    let result = cx.background_spawn(async move { backend.list().await }).await;
    cx.update(|cx| {
        cx.update_global::<chronos_fm_ui::devices_store::DeviceStore, _>(|store, _cx| {
            if let Ok(devices) = result {
                store.devices = devices;
            }
        });
    })
    .ok();
}
```

**Note for the implementer:** `UDisks2Backend` needs a `pub fn
connection(&self) -> &zbus::Connection` accessor added in Task 2 (small
addition — add it now if you skipped it, it's a one-line getter) so this
task can build the `ObjectManagerProxy` for signal subscriptions.
`zbus::fdo::ObjectManagerProxy` and `futures::select_biased!`/`StreamExt`
are the standard zbus/futures signal-subscription idiom — verify exact
method names (`receive_interfaces_added`/`receive_interfaces_removed`)
against the `zbus` version resolved in Task 1 via `cargo doc -p zbus
--open`; add `futures = "0.3"` to `crates/chronos-fm/Cargo.toml` if not
already present (`grep futures crates/chronos-fm/Cargo.toml`).

- [ ] **Step 2: Build check**

Run: `cargo build -p chronos-fm 2>&1`
Expected: exit 0.

- [ ] **Step 3: Live smoke (not automatable — do this before committing)**

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
cargo build --release --bin chronos-fm 2>&1
RUST_LOG=info ./target/release/chronos-fm 2>&1 | grep -i device
```

Insert a real USB stick while the app runs. Expected log line similar to
`refresh_devices` running with no `Devices panel unavailable` warning.
(Full UI verification is Task 6 + the plan's final live-verification
step — this step only confirms the D-Bus plumbing doesn't error out.)

- [ ] **Step 4: Commit**

```bash
git add crates/chronos-fm/src/app.rs crates/chronos-fm/Cargo.toml crates/chronos-fm-services/src/devices/backend.rs Cargo.lock
git commit -m "app: subscribe to udisks2 hotplug signals at startup"
```

---

### Task 6: Sidebar "Devices" section

**Files:**
- Modify: `crates/chronos-fm-pages/src/explorer/view/sidebar.rs`

**Interfaces:**
- Consumes: `chronos_fm_ui::devices_store::{current, DeviceStore}` (Task 3), `DeviceStore::mount_and_navigate`/`::unmount` (Task 4), `chronos_fm_ui::patterns::{elevated_card, section_header}` (T002, already in this file for the "Folders" section)
- Produces: nothing consumed elsewhere — this is the leaf UI task.

- [ ] **Step 1: Write the failing test**

Follow the existing `pane.rs` "renders without panicking" pattern
(`crates/chronos-fm-ui/src/components/pane.rs::tests`, referenced by the
T001 acceptance report):

```rust
// crates/chronos-fm-pages/src/explorer/view/sidebar.rs — bottom of file, extend
// or create the #[cfg(test)] mod if one doesn't already exist there
#[cfg(test)]
mod devices_section_tests {
    use gpui::{TestAppContext, point, px, size};

    #[gpui::test]
    async fn devices_section_renders_with_one_device(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            chronos_fm_ui::devices_store::init(cx);
            cx.update_global::<chronos_fm_ui::devices_store::DeviceStore, _>(|store, _cx| {
                store.devices.push(chronos_fm_services::devices::Device {
                    object_path: "/o/1".into(),
                    label: "MY_USB".into(),
                    device_node: "/dev/sdb1".into(),
                    mount_point: None,
                    is_removable: true,
                    size_bytes: 8_000_000_000,
                });
            });
        });
        let cx = cx.add_empty_window();
        cx.draw(point(px(0.0), px(0.0)), size(px(300.0), px(800.0)), |_window, cx| {
            super::render_devices_section(cx)
        });
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-fm-pages sidebar::devices_section 2>&1`
Expected: FAIL — `render_devices_section` not defined.

- [ ] **Step 3: Write minimal implementation**

Add to `crates/chronos-fm-pages/src/explorer/view/sidebar.rs`, called
from the existing `render` function right after the "Folders" section
(the one wrapped in `elevated_card`/`section_header(cx, "Folders", ...)`
per T002's report):

```rust
use chronos_fm_ui::devices_store;
use gpui::{IntoElement, ParentElement, Styled, div, px};

/// Renders the "Devices" sidebar section: one row per removable/internal
/// volume from `DeviceStore` (Task 3), using the T002 elevated-card
/// pattern already applied to the "Folders" section above it. Empty when
/// `DeviceStore` has no devices (no removable media / udisks2
/// unavailable) — renders nothing rather than an empty card, matching
/// the "don't take up space for nothing" instinct until T252's
/// empty-state pattern lands upstream (ChronOS, not yet closed as of
/// this plan — see spec's backlog note).
pub(crate) fn render_devices_section(cx: &gpui::App) -> impl IntoElement {
    let store = devices_store::current(cx);
    if store.devices.is_empty() {
        return div().into_any_element();
    }

    let mut card = chronos_fm_ui::patterns::elevated_card(cx)
        .child(chronos_fm_ui::patterns::section_header(cx, "Devices", "removable media"));

    if let Some(error) = &store.last_error {
        card = card.child(
            div()
                .text_color(chronos_fm_ui::theme::theme::danger(cx))
                .text_xs()
                .child(error.clone()),
        );
    }

    for device in &store.devices {
        let object_path = device.object_path.clone();
        let is_mounted = device.mount_point.is_some();
        let label = device.label.clone();

        card = card.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .child(
                    div()
                        .text_color(chronos_fm_ui::theme::theme::fg(cx))
                        .child(label),
                )
                .child(
                    div()
                        .text_color(chronos_fm_ui::theme::theme::muted(cx))
                        .text_xs()
                        .child(if is_mounted { "eject" } else { "mount" }),
                ),
        );
    }

    card.into_any_element()
}
```

**Note for the implementer:** this step wires the row *rendering* only
(matches Task 6's test, which only exercises layout). Wiring the actual
`on_click` handlers to `DeviceStore::mount_and_navigate`/`::unmount`
(Task 4) needs a `cx: &mut Context<ExplorerPane>` (or wherever
`sidebar::render` is called from) and a live `Arc<dyn DeviceBackend>` to
pass through — thread it the same way the existing "Folders" click
handlers reach `ExplorerPane` methods in this file (read the existing
`sidebar_item` click wiring immediately above where you insert this
section before writing the click handlers, and match its signature).
Call `chronos_fm_ui::devices_store::DeviceStore::mount_and_navigate`/
`::unmount` from those handlers instead of writing new mount logic here.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-fm-pages sidebar::devices_section 2>&1`
Expected: PASS — `devices_section_renders_with_one_device`.

- [ ] **Step 5: Full workspace check**

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
cargo build --workspace 2>&1; echo EXIT=$?
cargo test --workspace 2>&1 | grep -E "^test result"
```

Expected: `EXIT=0`, all `test result: ok` lines, no `FAILED`.

- [ ] **Step 6: Commit**

```bash
git add crates/chronos-fm-pages/src/explorer/view/sidebar.rs
git commit -m "ui: Devices sidebar section (list, mount/unmount, live hotplug)"
```

---

## Final Live Verification (do this before declaring the plan complete)

Per the spec's Verification section — not automatable, do it on a real
machine with a real USB stick:

1. `cargo build --release --bin chronos-fm 2>&1; echo EXIT=$?` — expect 0.
2. Launch the release binary, confirm no `Devices panel unavailable`
   warning in `RUST_LOG=info` output (udisks2 reachable).
3. Insert a USB stick. Confirm the "Devices" section appears in the
   sidebar **without restarting the app** (hotplug signal worked).
4. Click the device → confirm navigation into its mount point; confirm
   with `lsblk` / `findmnt` that it's actually mounted at the path shown.
5. Click eject/unmount → confirm the row updates and `lsblk` confirms
   unmounted.
6. `grim` screenshots of the Devices section, both themes (dark/light —
   T001 palette), attach to the task report.
7. Pull the USB stick without clicking unmount first — confirm the app
   doesn't crash or hang (udisks2 will emit `InterfacesRemoved`; confirm
   the row disappears on the next hotplug signal).

## Self-Review Notes

- **Spec coverage:** device model+filter (Task 1), mount/unmount/eject
  (Task 2/4), live hotplug (Task 5), sidebar UI on T002 patterns (Task
  6), error surfacing (Task 4/6) — all of spec §1–§4 covered. Spec's
  "вне скоупа v1" items (LUKS, format, burn, network protocols,
  autorun) are deliberately not tasked here.
- **Type consistency checked:** `Device` (Task 1) fields used identically
  in Tasks 2–6; `DeviceBackend` trait (Task 2) signature matches every
  call site in Tasks 4–5; `DeviceStore` (Task 3) fields (`devices`,
  `last_error`) match Task 4/6 usage.
- **Known soft spot flagged explicitly, not hidden:** Task 5's exact
  `zbus::fdo::ObjectManagerProxy` method names and Task 2's
  `Value`/`OwnedValue` pattern-match arms are correct for the *D-Bus
  wire protocol* (stable, versioned, not guessed) but the *zbus Rust
  binding surface* can drift between major versions — both tasks say
  explicitly to check `cargo doc -p zbus` against the resolved version
  before trusting the exact method names verbatim. This is a real, named
  risk, not a placeholder.
