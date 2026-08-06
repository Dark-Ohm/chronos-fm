//! S3-compatible object storage client (T011,
//! `docs/superpowers/specs/2026-08-06-s3-tab-live.md` §2.3).
//!
//! Wraps `aws-sdk-s3` with a synchronous API surface so the
//! `FileSystemProvider` trait can remain sync. Each `S3Client` owns a
//! single-threaded Tokio runtime on which all network calls are dispatched.

use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::primitives::ByteStream;
use chronos_fm_core::errors::{Error, Result};
use chronos_fm_models::file_entry::FileEntryDto;
use crate::fs::listing::ListResult;

/// Profile for one S3-compatible endpoint (mirrors `config::S3Profile`).
#[derive(Debug, Clone)]
pub struct S3Profile {
    pub endpoint: String,
    pub region: String,
    pub force_path_style: bool,
}

/// Wraps an `aws_sdk_s3::Client` with synchronous helper methods that
/// dispatch onto an owned single-threaded Tokio runtime.
pub struct S3Client {
    inner: aws_sdk_s3::Client,
    profile_name: String,
    runtime: tokio::runtime::Runtime,
}

impl S3Client {
    /// Build an S3 client for `profile_name` using explicit credentials.
    /// The caller is responsible for retrieving access key + secret from
    /// the keyring (Task 6) or environment. Creates its own single-threaded
    /// Tokio runtime for S3 operations.
    pub fn from_profile(
        profile_name: &str,
        profile: &S3Profile,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Other(format!("tokio runtime: {e}")))?;

        let credentials =
            Credentials::new(access_key, secret_key, None, None, "chronos-fm-s3");
        let region = Region::new(profile.region.clone());

        let config = runtime.block_on(async {
            let shared = aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(credentials)
                .endpoint_url(&profile.endpoint)
                .load()
                .await;
            aws_sdk_s3::config::Builder::from(&shared)
                .force_path_style(profile.force_path_style)
                .build()
        });

        Ok(Self {
            inner: aws_sdk_s3::Client::from_conf(config),
            profile_name: profile_name.to_string(),
            runtime,
        })
    }

    /// The profile this client was built for.
    pub fn profile_name(&self) -> &str {
        &self.profile_name
    }

    // ---- Bucket listing ----

    /// List all buckets. Each bucket is represented as a directory entry.
    pub fn list_buckets(&self) -> Result<Vec<FileEntryDto>> {
        self.runtime.block_on(async {
            let resp = self
                .inner
                .list_buckets()
                .send()
                .await
                .map_err(|e| Error::Other(format!("list_buckets: {e}")))?;

            let mut entries = Vec::new();
            for bucket in resp.buckets() {
                if let Some(name) = bucket.name() {
                    let path = s3_path(&self.profile_name, name, "");
                    entries.push(FileEntryDto {
                        name: name.to_string(),
                        path,
                        kind: "dir".to_string(),
                        size: 0,
                        modified: bucket
                            .creation_date()
                            .and_then(|d| {
                                d.to_millis()
                                    .ok()
                                    .map(|ms| (ms / 1000) as u64)
                            })
                            .unwrap_or(0),
                    });
                }
            }
            Ok(entries)
        })
    }

    // ---- Object listing ----

    /// List objects at `bucket` + `prefix`, using `delimiter("/")` so
    /// `CommonPrefixes` become directory entries and `Contents` become
    /// file entries.
    pub fn list_objects(&self, bucket: &str, prefix: &str) -> Result<ListResult> {
        self.runtime.block_on(async {
            let mut paginator = self
                .inner
                .list_objects_v2()
                .bucket(bucket)
                .prefix(prefix)
                .delimiter("/")
                .into_paginator()
                .send();

            let mut entries = Vec::new();
            while let Some(page) = paginator.next().await {
                let output =
                    page.map_err(|e| Error::Other(format!("list_objects_v2: {e}")))?;

                // Common prefixes become directories.
                for cp in output.common_prefixes() {
                    if let Some(prefix_name) = cp.prefix() {
                        let rel = prefix_name
                            .strip_prefix(prefix)
                            .unwrap_or(prefix_name)
                            .trim_end_matches('/');
                        if rel.is_empty() {
                            continue;
                        }
                        let path =
                            s3_path(&self.profile_name, bucket, prefix_name);
                        entries.push(FileEntryDto {
                            name: rel.to_string(),
                            path,
                            kind: "dir".to_string(),
                            size: 0,
                            modified: 0,
                        });
                    }
                }
                // Contents become files.
                for obj in output.contents() {
                    if let Some(key) = obj.key() {
                        if key == prefix || (prefix.is_empty() && key.ends_with('/'))
                        {
                            continue;
                        }
                        let name = key
                            .strip_prefix(prefix)
                            .unwrap_or(key)
                            .to_string();
                        let path =
                            s3_path(&self.profile_name, bucket, key);
                        entries.push(FileEntryDto {
                            name,
                            path,
                            kind: "file".to_string(),
                            size: obj.size().unwrap_or(0) as u64,
                            modified: obj
                                .last_modified()
                                .and_then(|t| {
                                    t.to_millis()
                                        .ok()
                                        .map(|ms| (ms / 1000) as u64)
                                })
                                .unwrap_or(0),
                        });
                    }
                }
            }
            Ok(ListResult {
                entries,
                next_cursor: None,
            })
        })
    }

    // ---- Object operations ----

    /// Download an object.
    pub fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        self.runtime.block_on(async {
            let resp = self
                .inner
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| Error::Other(format!("get_object {bucket}/{key}: {e}")))?;
            let bytes = resp
                .body
                .collect()
                .await
                .map_err(|e| Error::Other(format!("read body {bucket}/{key}: {e}")))?;
            Ok(bytes.into_bytes().to_vec())
        })
    }

    /// Upload an object.
    pub fn put_object(&self, bucket: &str, key: &str, content: &[u8]) -> Result<()> {
        let body = ByteStream::from(content.to_vec());
        self.runtime.block_on(async {
            self.inner
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(body)
                .send()
                .await
                .map_err(|e| Error::Other(format!("put_object {bucket}/{key}: {e}")))?;
            Ok(())
        })
    }

    /// Delete an object.
    pub fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        self.runtime.block_on(async {
            self.inner
                .delete_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| Error::Other(format!("delete_object {bucket}/{key}: {e}")))?;
            Ok(())
        })
    }

    /// Get object metadata.
    pub fn head_object(&self, bucket: &str, key: &str) -> Result<FileEntryDto> {
        self.runtime.block_on(async {
            let resp = self
                .inner
                .head_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| Error::Other(format!("head_object {bucket}/{key}: {e}")))?;
            Ok(FileEntryDto {
                name: key
                    .rsplit_once('/')
                    .map(|(_, name)| name.to_string())
                    .unwrap_or_else(|| key.to_string()),
                path: s3_path(&self.profile_name, bucket, key),
                kind: "file".to_string(),
                size: resp.content_length().unwrap_or(0) as u64,
                modified: resp
                    .last_modified()
                    .and_then(|t| t.to_millis().ok().map(|ms| (ms / 1000) as u64))
                    .unwrap_or(0),
            })
        })
    }
}

// ---- Path helpers ----

fn s3_path(profile: &str, bucket: &str, key: &str) -> String {
    format!("s3://{profile}@{bucket}/{key}")
}

/// Parse an S3 path into `(profile, bucket, key)`. Returns `None` when
/// the path does not start with `s3://` or is otherwise malformed.
pub fn parse_s3_path(path: &str) -> Option<(String, String, String)> {
    let rest = path.strip_prefix("s3://")?;
    let (profile, rest) = rest.split_once('@')?;
    let (bucket, key) = rest.split_once('/')?;
    Some((
        profile.to_string(),
        bucket.to_string(),
        key.to_string(),
    ))
}

/// Whether `path` is an S3 path.
pub fn is_s3_path(path: &str) -> bool {
    path.starts_with("s3://")
}

/// Build a bucket-root path: `s3://<profile>@<bucket>/`.
pub fn s3_bucket_path(profile: &str, bucket: &str) -> String {
    format!("s3://{profile}@{bucket}/")
}

/// Build the profile-root path (bucket listing): `s3://<profile>@`.
pub fn s3_profile_root(profile: &str) -> String {
    format!("s3://{profile}@")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_s3_path_extracts_components() {
        let (profile, bucket, key) =
            parse_s3_path("s3://personal@my-bucket/photos/beach.jpg").unwrap();
        assert_eq!(profile, "personal");
        assert_eq!(bucket, "my-bucket");
        assert_eq!(key, "photos/beach.jpg");
    }

    #[test]
    fn parse_s3_path_returns_none_for_non_s3() {
        assert!(parse_s3_path("/home/user/file.txt").is_none());
        assert!(parse_s3_path("").is_none());
    }

    #[test]
    fn is_s3_path_detects_prefix() {
        assert!(is_s3_path("s3://x@y/z"));
        assert!(!is_s3_path("/local/file"));
    }

    #[test]
    fn s3_bucket_path_formatted_correctly() {
        assert_eq!(
            s3_bucket_path("minio", "my-bucket"),
            "s3://minio@my-bucket/"
        );
    }

    #[test]
    fn s3_profile_root_formatted_correctly() {
        assert_eq!(s3_profile_root("personal"), "s3://personal@");
    }
}
