use super::parse::{parse_managed_objects, Device, DeviceValue, ManagedObjects};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::Connection;

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

    /// Exposes the underlying connection so the hotplug watcher (Task 5)
    /// can build an `ObjectManagerProxy` for signal subscriptions.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    fn value_to_device_value(v: &OwnedValue) -> Option<DeviceValue> {
        let v: &Value = v;
        match v {
            Value::Bool(b) => Some(DeviceValue::Bool(*b)),
            Value::U64(n) => Some(DeviceValue::U64(*n)),
            Value::Str(s) => Some(DeviceValue::Str(s.to_string())),
            Value::ObjectPath(p) => Some(DeviceValue::ObjectPath(p.to_string())),
            Value::Array(arr) if arr.element_signature().to_string() == "y" => {
                let bytes: Vec<u8> = arr
                    .iter()
                    .filter_map(|el| u8::try_from(el.clone()).ok())
                    .collect();
                Some(DeviceValue::Bytes(bytes))
            }
            // `MountPoints` is `aay` (array of byte-arrays); v1 only needs
            // the first mount point, so unwrap one level here.
            Value::Array(arr) if arr.element_signature().to_string() == "ay" => {
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
        let proxy = zbus::Proxy::new(
            &self.connection,
            SERVICE,
            MANAGER_PATH,
            "org.freedesktop.DBus.ObjectManager",
        )
        .await
        .context("building ObjectManager proxy")?;

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
        assert_eq!(
            devices[0].mount_point,
            Some(PathBuf::from("/run/media/neo/USB"))
        );
    }
}
