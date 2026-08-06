//! Application services: filesystem listing, full-text/regex search, and
//! optional syntax highlighting.

/// Synchronous filesystem listing services.
pub mod fs;
/// Full-text and regex file search services.
pub mod search;
/// Archive browsing — virtual folder for zip, tar, tar.gz, tar.zst.
pub mod archive;
/// MIME type detection and freedesktop.org application resolution.
pub mod mime;
/// Removable media device listing and mount control via `udisks2`.
pub mod devices;
/// Syntax highlighting backed by `syntect`, mapped to GPUI colors.
#[cfg(feature = "gui")]
pub mod syntax;
