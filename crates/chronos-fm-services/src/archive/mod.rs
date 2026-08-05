//! Archive browsing — virtual filesystem for zip, tar, tar.gz, tar.zst.
//!
//! Archives are opened lazily, cached in a small LRU pool (3 entries), and
//! exposed through the same `FileEntryDto` interface as real directories so
//! the Explorer layer needs no branching.

mod zip_archive;
mod tar_archive;

use anyhow::Result;
use chronos_fm_models::file_entry::FileEntryDto;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub use zip_archive::ZipFs;
pub use tar_archive::TarArchive;

/// Recognised archive formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
    TarZst,
}

impl ArchiveFormat {
    /// Detect format from a file extension. Returns `None` for unsupported
    /// extensions (the caller falls back to real filesystem).
    pub fn from_path(path: &Path) -> Option<Self> {
        let s = path.to_string_lossy();
        if s.ends_with(".tar.zst") || s.ends_with(".tar.zstd") {
            Some(Self::TarZst)
        } else if s.ends_with(".tar.gz") || s.ends_with(".tgz") {
            Some(Self::TarGz)
        } else if s.ends_with(".tar") {
            Some(Self::Tar)
        } else if s.ends_with(".zip") {
            Some(Self::Zip)
        } else {
            None
        }
    }
}

/// Parses a virtual archive path into `(archive_path, inner_path)`.
///
/// The virtual separator is `::` — everything before it is the real on-disk
/// archive file, everything after is the internal path relative to the archive
/// root. Returns `None` for regular filesystem paths.
pub fn split_archive_path(path: &str) -> Option<(PathBuf, String)> {
    if let Some(pos) = path.find("::") {
        let archive = PathBuf::from(&path[..pos]);
        let inner = path[pos + 2..].to_string();
        Some((archive, inner))
    } else {
        None
    }
}

/// Builds a virtual archive path from the archive file path and an inner path.
pub fn make_archive_path(archive_path: &Path, inner_path: &str) -> String {
    format!("{}::{}", archive_path.display(), inner_path.trim_start_matches('/'))
}

/// Errors specific to archive operations.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("corrupt archive: {0}")]
    Corrupt(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("file not found in archive: {0}")]
    NotFound(String),
    #[error("archive is read-only")]
    ReadOnly,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Common trait for all archive backends.
pub(crate) trait ArchiveFs: Send + Sync {
    /// List entries under `inner_dir` (empty or `"/"` for root).
    fn list(&self, inner_dir: &str) -> Result<Vec<FileEntryDto>, ArchiveError>;

    /// Read a file's contents from the archive.
    fn read_file(&self, path_in_archive: &str) -> Result<Vec<u8>, ArchiveError>;

    /// Whether this archive supports write operations.
    fn is_read_only(&self) -> bool {
        false
    }

    /// Write (replace or create) a file inside the archive.
    fn write_file(&mut self, _path: &str, _data: &[u8]) -> Result<(), ArchiveError> {
        Err(ArchiveError::ReadOnly)
    }

    /// Remove a file from the archive.
    fn remove_file(&mut self, _path: &str) -> Result<(), ArchiveError> {
        Err(ArchiveError::ReadOnly)
    }

    /// Flush changes back to the archive file on disk.
    fn commit(&mut self) -> Result<(), ArchiveError> {
        Ok(())
    }
}

// ---- LRU cache ----

type CacheKey = PathBuf;
type CacheEntry = Box<dyn ArchiveFs>;

static ARCHIVE_CACHE: std::sync::LazyLock<Mutex<LruCache<CacheKey, CacheEntry>>> =
    std::sync::LazyLock::new(|| {
        Mutex::new(LruCache::new(
            NonZeroUsize::new(3).expect("3 > 0"),
        ))
    });

/// Opens an archive (from cache if already open, otherwise reads from disk).
/// The returned reference is NOT held across await points — callers should
/// re-acquire via `with_archive` for each operation.
fn open_archive(path: &Path) -> Result<(), ArchiveError> {
    let mut cache = ARCHIVE_CACHE.lock().map_err(|e| ArchiveError::Other(anyhow::anyhow!("lock poison: {e}")))?;
    if cache.contains(path) {
        return Ok(());
    }

    let format = ArchiveFormat::from_path(path)
        .ok_or_else(|| ArchiveError::UnsupportedFormat(path.display().to_string()))?;

    let archive: Box<dyn ArchiveFs> = match format {
        ArchiveFormat::Zip => Box::new(ZipFs::open(path)?),
        ArchiveFormat::Tar | ArchiveFormat::TarGz | ArchiveFormat::TarZst => {
            Box::new(TarArchive::open(path, format)?)
        }
    };

    cache.put(path.to_path_buf(), archive);
    Ok(())
}

/// Runs `f` with a reference to the cached archive. Opens the archive if
/// it's not already in cache.
pub(crate) fn with_archive<T>(
    path: &Path,
    f: impl FnOnce(&dyn ArchiveFs) -> Result<T, ArchiveError>,
) -> Result<T, ArchiveError> {
    open_archive(path)?;
    let cache = ARCHIVE_CACHE.lock().map_err(|e| ArchiveError::Other(anyhow::anyhow!("lock poison: {e}")))?;
    let entry = cache.peek(path).ok_or_else(|| ArchiveError::Other(anyhow::anyhow!("archive evicted from cache")))?;
    f(entry.as_ref())
}

/// Runs `f` with a mutable reference to the cached archive.
pub(crate) fn with_archive_mut<T>(
    path: &Path,
    f: impl FnOnce(&mut dyn ArchiveFs) -> Result<T, ArchiveError>,
) -> Result<T, ArchiveError> {
    open_archive(path)?;
    let mut cache = ARCHIVE_CACHE.lock().map_err(|e| ArchiveError::Other(anyhow::anyhow!("lock poison: {e}")))?;
    let entry = cache.get_mut(path).ok_or_else(|| ArchiveError::Other(anyhow::anyhow!("archive evicted from cache")))?;
    f(entry.as_mut())
}

/// Public API: list a directory inside an archive.
///
/// `archive_path` is the real on-disk path of the archive file.
/// `inner_dir` is the directory inside the archive (empty or `"/"` for root).
/// Returns `Vec<FileEntryDto>` with virtual paths (`archive.zip::/file.txt`).
pub fn list_dir(archive_path: &Path, inner_dir: &str) -> Result<Vec<FileEntryDto>, ArchiveError> {
    with_archive(archive_path, |archive| {
        let inner = if inner_dir == "/" { "" } else { inner_dir.trim_start_matches('/') };
        archive.list(inner)
    })
}

/// Public API: read a file from inside an archive.
pub fn read_file(archive_path: &Path, path_in_archive: &str) -> Result<Vec<u8>, ArchiveError> {
    with_archive(archive_path, |archive| {
        archive.read_file(path_in_archive)
    })
}

/// Public API: write a file into an archive.
pub fn write_file(archive_path: &Path, path_in_archive: &str, data: &[u8]) -> Result<(), ArchiveError> {
    with_archive_mut(archive_path, |archive| archive.write_file(path_in_archive, data))
}

/// Public API: flush archive changes to disk.
pub fn commit(archive_path: &Path) -> Result<(), ArchiveError> {
    with_archive_mut(archive_path, |archive| archive.commit())
}
