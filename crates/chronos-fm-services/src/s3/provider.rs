//! `FileSystemProvider` implementation for `S3Client` (T011, Task 4).

use chronos_fm_core::errors::{Error, Result};
use chronos_fm_models::file_entry::FileEntryDto;

use crate::fs::listing::ListResult;
use crate::fs::provider::FileSystemProvider;
use crate::s3::{self, S3Client};

impl FileSystemProvider for S3Client {
    fn root_label(&self) -> String {
        format!("S3 — {}", self.profile_name())
    }

    fn list_dir(&self, path: &str, _limit: usize, _cursor: Option<&str>) -> Result<ListResult> {
        if !s3::is_s3_path(path) {
            return Err(Error::Other(format!(
                "S3FileSystemProvider: not an S3 path: {path}"
            )));
        }

        // Profile root (s3://profile@): list buckets.
        if path.ends_with('@') {
            let entries = self.list_buckets()?;
            return Ok(ListResult {
                entries,
                next_cursor: None,
            });
        }

        // Bucket or prefix level.
        let (profile, bucket, key) = s3::parse_s3_path(path).ok_or_else(|| {
            Error::Other(format!("S3FileSystemProvider: malformed S3 path: {path}"))
        })?;

        // Verify the profile matches.
        if profile != self.profile_name() {
            return Err(Error::Other(format!(
                "profile mismatch: path uses {profile:?}, client is {:?}",
                self.profile_name()
            )));
        }

        self.list_objects(&bucket, &key)
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let (_, bucket, key) = s3::parse_s3_path(path).ok_or_else(|| {
            Error::Other(format!("S3FileSystemProvider: malformed S3 path: {path}"))
        })?;
        self.get_object(&bucket, &key)
    }

    fn metadata(&self, path: &str) -> Result<FileEntryDto> {
        let (_, bucket, key) = s3::parse_s3_path(path).ok_or_else(|| {
            Error::Other(format!("S3FileSystemProvider: malformed S3 path: {path}"))
        })?;
        // Directories are synthetic — no head_object for them.
        if key.is_empty() || key.ends_with('/') {
            let name = key
                .trim_end_matches('/')
                .rsplit_once('/')
                .map(|(_, n)| n.to_string())
                .unwrap_or_else(|| bucket.clone());
            return Ok(FileEntryDto {
                name,
                path: path.to_string(),
                kind: "dir".to_string(),
                size: 0,
                modified: 0,
            });
        }
        self.head_object(&bucket, &key)
    }

    fn create_dir(&self, _parent: &str, _name: &str) -> Result<()> {
        // S3 has no real directories — they are implicit from prefixes.
        // Creating a zero-byte marker object is optional; the plan says
        // no-op for v1.
        Ok(())
    }

    fn delete(&self, path: &str, is_dir: bool) -> Result<()> {
        let (_, bucket, key) = s3::parse_s3_path(path).ok_or_else(|| {
            Error::Other(format!("S3FileSystemProvider: malformed S3 path: {path}"))
        })?;

        if is_dir {
            // Recursive: list all keys under the prefix and delete them.
            let prefix = if key.ends_with('/') {
                key.clone()
            } else {
                format!("{key}/")
            };
            let objects = self.list_objects(&bucket, &prefix)?;
            for entry in &objects.entries {
                if entry.kind != "file" {
                    continue;
                }
                let (_, _, obj_key) = s3::parse_s3_path(&entry.path).ok_or_else(|| {
                    Error::Other(format!("S3FileSystemProvider: malformed path in list: {}", entry.path))
                })?;
                self.delete_object(&bucket, &obj_key)?;
            }
        } else {
            self.delete_object(&bucket, &key)?;
        }
        Ok(())
    }

    fn rename(&self, from: &str, to: &str) -> Result<()> {
        let (_, from_bucket, from_key) = s3::parse_s3_path(from).ok_or_else(|| {
            Error::Other(format!("S3FileSystemProvider: malformed from path: {from}"))
        })?;
        let (_, to_bucket, to_key) = s3::parse_s3_path(to).ok_or_else(|| {
            Error::Other(format!("S3FileSystemProvider: malformed to path: {to}"))
        })?;

        if from_bucket != to_bucket {
            return Err(Error::Other(
                "cross-bucket rename not supported".to_string(),
            ));
        }

        // For files: get + put + delete.
        let content = self.get_object(&from_bucket, &from_key)?;
        self.put_object(&to_bucket, &to_key, &content)?;
        self.delete_object(&from_bucket, &from_key)?;
        Ok(())
    }

    fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let (_, bucket, key) = s3::parse_s3_path(path).ok_or_else(|| {
            Error::Other(format!("S3FileSystemProvider: malformed S3 path: {path}"))
        })?;
        self.put_object(&bucket, &key, content)
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn scheme(&self) -> &str {
        "s3"
    }
}
