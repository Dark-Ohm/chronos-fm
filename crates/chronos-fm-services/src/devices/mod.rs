//! Removable/internal storage device listing and mount control via
//! `udisks2` (spec: docs/superpowers/specs/2026-08-06-removable-media-mount.md).

mod parse;
pub use parse::{parse_managed_objects, Device, DeviceValue, ManagedObjects};

mod backend;
pub use backend::{DeviceBackend, UDisks2Backend};
