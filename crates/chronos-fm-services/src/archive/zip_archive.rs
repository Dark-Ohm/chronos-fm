use super::{ArchiveError, ArchiveFs, make_archive_path};
use chronos_fm_models::file_entry::FileEntryDto;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

/// Zip archive backend (read + write).
pub struct ZipFs {
    /// In-memory mutable representation. `None` if no changes have been made
    /// and we're serving reads directly from the original file.
    dirty: bool,
    /// The original on-disk file path (for commit).
    path: std::path::PathBuf,
    /// Cached file listing for fast repeated `list()` calls.
    entries: Vec<FileEntryDto>,
}

impl ZipFs {
    pub fn open(path: &Path) -> Result<Self, ArchiveError> {
        let file = std::fs::File::open(path)?;
        let mut za = zip::ZipArchive::new(file).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
        let mut entries = Vec::new();

        for i in 0..za.len() {
            let entry = za.by_index(i).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
            let name = entry.name().to_string();

            // Skip directory entries (zip stores them as trailing-slash names)
            if name.ends_with('/') && entry.size() == 0 {
                continue;
            }

            let kind = if entry.is_dir() { "dir" } else { "file" }.to_string();
            // zip 7.x DateTime → unix_timestamp requires chaining through
            // deprecated to_time() + OffsetDateTime. Not worth it for v1.
            let modified = 0u64;

            let arc_path = make_archive_path(path, &name);

            entries.push(FileEntryDto {
                name: Path::new(&name)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or(name.clone()),
                path: arc_path,
                kind,
                size: entry.size(),
                modified,
            });
        }

        // Add synthetic directory entries for paths that contain subdirectories
        let dirs = synthetic_dirs(&entries, path);
        entries.extend(dirs);

        entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(Self {
            dirty: false,
            path: path.to_path_buf(),
            entries,
        })
    }
}

impl ArchiveFs for ZipFs {
    fn list(&self, inner_dir: &str) -> Result<Vec<FileEntryDto>, ArchiveError> {
        if inner_dir.is_empty() || inner_dir == "/" {
            Ok(self.entries.clone())
        } else {
            let prefix = format!("{}/", inner_dir.trim_end_matches('/'));
            Ok(self
                .entries
                .iter()
                .filter(|e| {
                    // Filter entries whose archive path contains the prefix
                    // but are not in deeper subdirectories
                    if let Some(rel) = e.path.strip_prefix(&format!(
                        "{}::/",
                        self.path.display()
                    )) {
                        rel.starts_with(&prefix) || rel == inner_dir
                    } else {
                        false
                    }
                })
                .cloned()
                .collect())
        }
    }

    fn read_file(&self, path_in_archive: &str) -> Result<Vec<u8>, ArchiveError> {
        let file = std::fs::File::open(&self.path)?;
        let mut za = zip::ZipArchive::new(file).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
        let entry = za
            .by_name(path_in_archive)
            .map_err(|_| ArchiveError::NotFound(path_in_archive.to_string()))?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        let mut reader = entry;
        reader
            .read_to_end(&mut buf)
            .map_err(|e| ArchiveError::Io(e))?;
        Ok(buf)
    }

    fn write_file(&mut self, path_in_archive: &str, data: &[u8]) -> Result<(), ArchiveError> {
        // Rebuild the zip with the updated file.
        let tmp_path = self.path.with_extension("zip.tmp");

        let in_file = std::fs::File::open(&self.path)?;
        let mut za = zip::ZipArchive::new(in_file).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;

        let out_file = std::fs::File::create(&tmp_path)?;
        let mut writer = zip::ZipWriter::new(out_file);

        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);

        let mut found = false;

        for i in 0..za.len() {
            let entry = za.by_index(i).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
            let name = entry.name().to_string();

            if name == path_in_archive {
                // Replace with new data
                writer
                    .start_file(&name, options)
                    .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
                writer
                    .write_all(data)
                    .map_err(|e| ArchiveError::Io(e))?;
                found = true;
            } else {
                // Copy existing entry
                writer
                    .raw_copy_file(entry)
                    .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
            }
        }

        if !found {
            // Add as new file
            writer
                .start_file(path_in_archive, options)
                .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
            writer
                .write_all(data)
                .map_err(|e| ArchiveError::Io(e))?;
        }

        writer
            .finish()
            .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;

        // Atomic rename: replace original with updated archive
        std::fs::rename(&tmp_path, &self.path)?;
        self.dirty = false;

        // Refresh entries
        *self = Self::open(&self.path)?;

        Ok(())
    }

    #[allow(dead_code)]
    fn remove_file(&mut self, path_in_archive: &str) -> Result<(), ArchiveError> {
        // Same pattern: rebuild without the removed file
        let tmp_path = self.path.with_extension("zip.tmp");

        let in_file = std::fs::File::open(&self.path)?;
        let mut za = zip::ZipArchive::new(in_file).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;

        let out_file = std::fs::File::create(&tmp_path)?;
        let mut writer = zip::ZipWriter::new(out_file);

        for i in 0..za.len() {
            let entry = za.by_index(i).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
            let name = entry.name().to_string();
            if name == path_in_archive {
                continue; // Skip — this file is removed
            }
            writer
                .raw_copy_file(entry)
                .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
        }

        writer
            .finish()
            .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;

        std::fs::rename(&tmp_path, &self.path)?;

        // Refresh entries
        *self = Self::open(&self.path)?;

        Ok(())
    }
}

/// Synthesize directory entries from file paths inside the archive.
/// E.g. if `reports/report.txt` exists, create a `reports/` dir entry.
fn synthetic_dirs(entries: &[FileEntryDto], archive_path: &Path) -> Vec<FileEntryDto> {
    let prefix = format!("{}::/", archive_path.display());
    let mut dirs = std::collections::HashSet::new();

    for e in entries {
        if let Some(rel) = e.path.strip_prefix(&prefix) {
            let p = Path::new(rel);
            let mut current = p.parent();
            while let Some(parent) = current {
                let parent_str = parent.to_string_lossy();
                if !parent_str.is_empty() && parent_str != "/" {
                    dirs.insert(parent_str.to_string());
                }
                current = parent.parent();
            }
        }
    }

    dirs
        .into_iter()
        .map(|d| {
            let arc_path = make_archive_path(archive_path, &d);
            let name = Path::new(&d)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(d.clone());
            FileEntryDto {
                name,
                path: arc_path,
                kind: "dir".to_string(),
                size: 0,
                modified: 0,
            }
        })
        .collect()
}
