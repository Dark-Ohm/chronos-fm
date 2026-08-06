use super::{ArchiveError, ArchiveFormat, ArchiveFs, make_archive_path};
use chronos_fm_models::file_entry::FileEntryDto;
use std::io::{Read, Write};
use std::path::Path;

/// Tar archive backend (read + write).
///
/// For compressed formats (`.tar.gz`, `.tar.zst`), decompression is applied
/// transparently on read and compression on write via `flate2` / `zstd` crates.
pub struct TarArchive {
    format: ArchiveFormat,
    path: std::path::PathBuf,
    /// Cached file listing.
    entries: Vec<FileEntryDto>,
    /// Full raw bytes of the archive file (for rebuild on write).
    /// Stored so we can modify entries and re-serialize.
    raw_bytes: Vec<u8>,
}

impl TarArchive {
    pub fn open(path: &Path, format: ArchiveFormat) -> Result<Self, ArchiveError> {
        let raw_bytes = std::fs::read(path)?;
        let entries = Self::parse_entries(path, &raw_bytes, format)?;
        Ok(Self {
            format,
            path: path.to_path_buf(),
            entries,
            raw_bytes,
        })
    }

    fn parse_entries(
        archive_path: &Path,
        data: &[u8],
        format: ArchiveFormat,
    ) -> Result<Vec<FileEntryDto>, ArchiveError> {
        let reader: Box<dyn Read> = match format {
            ArchiveFormat::TarGz => Box::new(flate2::read::GzDecoder::new(data)),
            ArchiveFormat::TarZst => Box::new(zstd::Decoder::new(data).map_err(|e| ArchiveError::Corrupt(e.to_string()))?),
            ArchiveFormat::Tar => Box::new(data),
            _ => return Err(ArchiveError::UnsupportedFormat(format!("{format:?}"))),
        };

        let mut archive = tar::Archive::new(reader);
        let mut entries = Vec::new();

        for entry_res in archive.entries().map_err(|e| ArchiveError::Corrupt(e.to_string()))? {
            let entry = entry_res.map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
            let header = entry.header();
            let name = header
                .path()
                .map_err(|e| ArchiveError::Corrupt(e.to_string()))?
                .to_string_lossy()
                .to_string();

            let kind = match header.entry_type() {
                tar::EntryType::Directory => "dir",
                tar::EntryType::Regular => "file",
                tar::EntryType::Symlink => "symlink",
                _ => continue,
            }
            .to_string();

            let modified = header.mtime().map(|t| t as u64).unwrap_or(0);
            let size = header.size().map(|s| s as u64).unwrap_or(0);

            let arc_path = make_archive_path(archive_path, &name);
            let file_name = Path::new(&name)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(name.clone());

            entries.push(FileEntryDto {
                name: file_name,
                path: arc_path,
                kind,
                size,
                modified,
            });
        }

        entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(entries)
    }

    /// Rebuild the tar archive from the current entry list and raw file data.
    /// Writes to a temp file, then atomically renames.
    fn rebuild(&mut self) -> Result<(), ArchiveError> {
        let tmp_path = self.path.with_extension("tar.tmp");
        let out_file = std::fs::File::create(&tmp_path)?;

        let out_buf = std::io::BufWriter::new(out_file);

        let writer: Box<dyn Write> = match self.format {
            ArchiveFormat::TarGz => Box::new(flate2::write::GzEncoder::new(
                out_buf,
                flate2::Compression::default(),
            )),
            ArchiveFormat::TarZst => Box::new(
                zstd::Encoder::new(out_buf, 3)
                    .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?
                    .auto_finish(),
            ),
            ArchiveFormat::Tar => Box::new(out_buf),
            _ => return Err(ArchiveError::UnsupportedFormat(format!("{:?}", self.format))),
        };

        let mut builder = tar::Builder::new(writer);

        // Clone raw_bytes so the borrow doesn't conflict with the assignment below.
        let raw_copy = self.raw_bytes.clone();
        let reader: Box<dyn Read> = match self.format {
            ArchiveFormat::TarGz => Box::new(flate2::read::GzDecoder::new(raw_copy.as_slice())),
            ArchiveFormat::TarZst => Box::new(
                zstd::Decoder::new(raw_copy.as_slice())
                    .map_err(|e| ArchiveError::Corrupt(e.to_string()))?,
            ),
            ArchiveFormat::Tar => Box::new(raw_copy.as_slice()),
            _ => unreachable!(),
        };

        let mut old_archive = tar::Archive::new(reader);
        for entry_res in old_archive
            .entries()
            .map_err(|e| ArchiveError::Corrupt(e.to_string()))?
        {
            let entry = entry_res.map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
            let header = entry.header().clone();
            let mut body = Vec::new();
            let mut entry = entry;
            entry
                .read_to_end(&mut body)
                .map_err(|e| ArchiveError::Io(e))?;

            builder
                .append(&header, body.as_slice())
                .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
        }

        builder
            .into_inner()
            .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;

        std::fs::rename(&tmp_path, &self.path)?;
        self.raw_bytes = std::fs::read(&self.path)?;

        Ok(())
    }
}

impl ArchiveFs for TarArchive {
    fn list(&self, _inner_dir: &str) -> Result<Vec<FileEntryDto>, ArchiveError> {
        Ok(self.entries.clone())
    }

    fn read_file(&self, path_in_archive: &str) -> Result<Vec<u8>, ArchiveError> {
        let reader: Box<dyn Read> = match self.format {
            ArchiveFormat::TarGz => Box::new(flate2::read::GzDecoder::new(self.raw_bytes.as_slice())),
            ArchiveFormat::TarZst => Box::new(
                zstd::Decoder::new(self.raw_bytes.as_slice())
                    .map_err(|e| ArchiveError::Corrupt(e.to_string()))?,
            ),
            ArchiveFormat::Tar => Box::new(self.raw_bytes.as_slice()),
            _ => unreachable!(),
        };

        let mut archive = tar::Archive::new(reader);
        for entry_res in archive.entries().map_err(|e| ArchiveError::Corrupt(e.to_string()))? {
            let mut entry = entry_res.map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
            let name = entry
                .header()
                .path()
                .map_err(|e| ArchiveError::Corrupt(e.to_string()))?
                .to_string_lossy()
                .to_string();

            if name == path_in_archive {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| ArchiveError::Io(e))?;
                return Ok(buf);
            }
        }

        Err(ArchiveError::NotFound(path_in_archive.to_string()))
    }
    fn write_file(&mut self, path_in_archive: &str, data: &[u8]) -> Result<(), ArchiveError> {
        // For tar, we need to rebuild with modified entries.
        // Collect all entries, replace the target, and rebuild.
        let tmp_path = self.path.with_extension("tar.tmp");
        let out_file = std::fs::File::create(&tmp_path)?;
        let out_buf = std::io::BufWriter::new(out_file);

        let writer: Box<dyn Write> = match self.format {
            ArchiveFormat::TarGz => Box::new(flate2::write::GzEncoder::new(
                out_buf,
                flate2::Compression::default(),
            )),
            ArchiveFormat::TarZst => Box::new(
                zstd::Encoder::new(out_buf, 3)
                    .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?
                    .auto_finish(),
            ),
            ArchiveFormat::Tar => Box::new(out_buf),
            _ => return Err(ArchiveError::UnsupportedFormat(format!("{:?}", self.format))),
        };

        let mut builder = tar::Builder::new(writer);
        let mut found = false;

        // Clone to avoid borrow conflict with the assignment below.
        let raw_copy = self.raw_bytes.clone();
        let reader: Box<dyn Read> = match self.format {
            ArchiveFormat::TarGz => Box::new(flate2::read::GzDecoder::new(raw_copy.as_slice())),
            ArchiveFormat::TarZst => Box::new(
                zstd::Decoder::new(raw_copy.as_slice())
                    .map_err(|e| ArchiveError::Corrupt(e.to_string()))?,
            ),
            ArchiveFormat::Tar => Box::new(raw_copy.as_slice()),
            _ => unreachable!(),
        };

        let mut old_archive = tar::Archive::new(reader);
        for entry_res in old_archive
            .entries()
            .map_err(|e| ArchiveError::Corrupt(e.to_string()))?
        {
            let mut entry = entry_res.map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
            let entry_name = entry
                .header()
                .path()
                .map_err(|e| ArchiveError::Corrupt(e.to_string()))?
                .to_string_lossy()
                .to_string();

            if entry_name == path_in_archive {
                // Replace with new data
                let mut header = entry.header().clone();
                header.set_size(data.len() as u64);
                builder
                    .append(&header, data)
                    .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
                found = true;
            } else {
                let header = entry.header().clone();
                let mut body = Vec::new();
                entry
                    .read_to_end(&mut body)
                    .map_err(|e| ArchiveError::Io(e))?;
                builder
                    .append(&header, body.as_slice())
                    .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
            }
        }

        if !found {
            // New file
            let mut header = tar::Header::new_gnu();
            header.set_path(path_in_archive)
                .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
            header.set_size(data.len() as u64);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(0o644);
            builder
                .append(&header, data)
                .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;
        }

        builder
            .into_inner()
            .map_err(|e| ArchiveError::Other(anyhow::anyhow!(e)))?;

        std::fs::rename(&tmp_path, &self.path)?;
        self.raw_bytes = std::fs::read(&self.path)?;
        self.entries = Self::parse_entries(&self.path, &self.raw_bytes, self.format)?;

        Ok(())
    }

    fn commit(&mut self) -> Result<(), ArchiveError> {
        // Changes are written immediately on write_file.
        // commit is a no-op for tar (rebuild happens on each write).
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_tar_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut buf);
            for (name, data) in files {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_entry_type(tar::EntryType::Regular);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, name, &**data).unwrap();
            }
            builder.finish().unwrap();
        }
        buf
    }

    fn write_file(path: &Path, data: &[u8]) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(data).unwrap();
    }

    #[test]
    fn tararchive_list_and_read() {
        let tmp = tempfile::tempdir().unwrap();
        let tpath = tmp.path().join("test.tar");
        write_file(
            &tpath,
            &make_tar_bytes(&[("hello.txt", b"hello tar"), ("dir/nested.txt", b"nested tar")]),
        );

        let ta = TarArchive::open(&tpath, ArchiveFormat::Tar).unwrap();
        let entries = ta.list("").unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.iter().any(|n| *n == "hello.txt"),
            "file present: {names:?}"
        );
        assert_eq!(ta.read_file("hello.txt").unwrap(), b"hello tar");
        assert_eq!(ta.read_file("dir/nested.txt").unwrap(), b"nested tar");
        assert!(matches!(
            ta.read_file("missing.txt"),
            Err(ArchiveError::NotFound(_))
        ));
    }

    #[test]
    fn tararchive_gz_list_and_read() {
        let tmp = tempfile::tempdir().unwrap();
        let tpath = tmp.path().join("test.tar.gz");
        let raw = make_tar_bytes(&[("a.txt", b"gzipped content")]);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&raw).unwrap();
        let compressed = gz.finish().unwrap();
        write_file(&tpath, &compressed);

        let ta = TarArchive::open(&tpath, ArchiveFormat::TarGz).unwrap();
        assert_eq!(ta.read_file("a.txt").unwrap(), b"gzipped content");
    }
}
