//! Abstract filesystem backend trait, so the Explorer can list, read, and
//! mutate entries through a uniform interface whether the backing store is a
//! local disk or S3-compatible object storage (T011,
//! `docs/superpowers/specs/2026-08-06-s3-tab-live.md` §2.1).
//!
//! The trait is synchronous — methods return `Result<T>`, not `Future` — so
//! the existing synchronous Explorer pipeline (`reload`, `apply_filter`,
//! `change_dir`) can dispatch through it without async contagion. Providers
//! that need network I/O (S3) use internal blocking (e.g. `pollster::block_on`)
//! and are called from the background executor.

use chronos_fm_core::errors::Result;
use chronos_fm_models::file_entry::FileEntryDto;

use super::listing::{ListParams, ListResult};

/// Abstract filesystem backend — local disk or S3-compatible object storage.
pub trait FileSystemProvider: Send + Sync {
    /// Human-readable label for the root (e.g. "Local Files" or "S3").
    fn root_label(&self) -> String;

    /// List entries at `path`. `path` is provider-specific:
    /// - Local: absolute filesystem path
    /// - S3: "s3://profile@bucket" or "s3://profile@bucket/prefix/"
    fn list_dir(&self, path: &str, limit: usize, cursor: Option<&str>) -> Result<ListResult>;

    /// Read the full contents of a file at `path`.
    fn read_file(&self, path: &str) -> Result<Vec<u8>>;

    /// Check existence + metadata of a single entry.
    fn metadata(&self, path: &str) -> Result<FileEntryDto>;

    /// Create a directory at `parent` named `name`. For S3 this may be a
    /// no-op or a zero-byte marker object.
    fn create_dir(&self, parent: &str, name: &str) -> Result<()>;

    /// Delete an entry. When `is_dir` is true, the provider recursively
    /// removes all children.
    fn delete(&self, path: &str, is_dir: bool) -> Result<()>;

    /// Rename/move an entry from `from` to `to`. For S3 "directories" this
    /// is a copy+delete of every prefixed key.
    fn rename(&self, from: &str, to: &str) -> Result<()>;

    /// Write `content` to `path`, creating or overwriting the entry.
    fn write_file(&self, path: &str, content: &[u8]) -> Result<()>;

    /// Whether this provider supports mutating operations. When `true` the
    /// UI hides Delete/Rename/New Folder/Upload actions.
    fn is_read_only(&self) -> bool;

    /// Scheme prefix for display and path disambiguation, e.g. "file" or "s3".
    fn scheme(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Local filesystem adapter — delegates to the existing `listing` and `ops`
// modules.
// ---------------------------------------------------------------------------

/// A [`FileSystemProvider`] backed by the local filesystem.
pub struct LocalFileSystemProvider;

impl FileSystemProvider for LocalFileSystemProvider {
    fn root_label(&self) -> String {
        "Local Files".to_string()
    }

    fn list_dir(&self, path: &str, limit: usize, cursor: Option<&str>) -> Result<ListResult> {
        super::listing::list_dir_sync(ListParams {
            path,
            limit,
            cursor,
        })
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        // Avoid clippy's disallowed `std::fs::read` — use `File::open` + `Read`
        // (same pattern as `zip_archive.rs` / `spotlight.rs` in this crate).
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn metadata(&self, path: &str) -> Result<FileEntryDto> {
        let md = std::fs::symlink_metadata(path)?;
        let kind = if md.is_dir() {
            "dir"
        } else if md.is_symlink() {
            "symlink"
        } else {
            "file"
        };
        let modified = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        Ok(FileEntryDto {
            name,
            path: path.to_string(),
            kind: kind.to_string(),
            size: md.len(),
            modified,
        })
    }

    fn create_dir(&self, parent: &str, name: &str) -> Result<()> {
        super::ops::create_dir(std::path::Path::new(parent), name)?;
        Ok(())
    }

    fn delete(&self, path: &str, _is_dir: bool) -> Result<()> {
        super::ops::delete_permanent(std::path::Path::new(path))
    }

    fn rename(&self, from: &str, to: &str) -> Result<()> {
        super::ops::move_path(std::path::Path::new(from), std::path::Path::new(to))?;
        Ok(())
    }

    fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        use std::io::Write;
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(path)?;
        file.write_all(content)?;
        Ok(())
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn scheme(&self) -> &str {
        "file"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_methods)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn provider() -> LocalFileSystemProvider {
        LocalFileSystemProvider
    }

    #[test]
    fn local_provider_lists_directory() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();

        let res = provider()
            .list_dir(&dir.path().to_string_lossy(), 100, None)
            .unwrap();
        let names: Vec<_> = res.entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"sub"));
    }

    #[test]
    fn local_provider_reads_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.bin");
        std::fs::write(&path, b"payload").unwrap();

        let content = provider().read_file(&path.to_string_lossy()).unwrap();
        assert_eq!(content, b"payload");
    }

    #[test]
    fn local_provider_creates_and_deletes_dir() {
        let dir = tempdir().unwrap();
        let parent = dir.path().to_string_lossy().to_string();
        provider().create_dir(&parent, "new-dir").unwrap();
        assert!(dir.path().join("new-dir").is_dir());
        provider()
            .delete(&format!("{parent}/new-dir"), true)
            .unwrap();
        assert!(!dir.path().join("new-dir").exists());
    }

    #[test]
    fn local_provider_writes_and_renames_file() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");

        provider()
            .write_file(&a.to_string_lossy(), b"content")
            .unwrap();
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "content");

        provider()
            .rename(&a.to_string_lossy(), &b.to_string_lossy())
            .unwrap();
        assert!(!a.exists());
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "content");
    }

    #[test]
    fn local_provider_is_not_read_only() {
        assert!(!provider().is_read_only());
    }

    #[test]
    fn local_provider_scheme_is_file() {
        assert_eq!(provider().scheme(), "file");
    }
}
