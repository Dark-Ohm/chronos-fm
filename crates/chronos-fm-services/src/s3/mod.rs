//! S3-compatible object storage client (T011,
//! `docs/superpowers/specs/2026-08-06-s3-tab-live.md` §2.3).
//!
//! Wraps `aws-sdk-s3` with a synchronous API surface so the
//! `FileSystemProvider` trait can remain sync. Each `S3Client` owns a
//! single-threaded Tokio runtime on which all network calls are dispatched.

pub mod provider;
/// Chunked upload/download engine + job state machine (T039, additive to
/// the whole-object methods below).
pub mod transfer;

use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::primitives::ByteStream;
use chronos_fm_core::errors::{Error, Result};
use chronos_fm_models::file_entry::FileEntryDto;
use crate::fs::listing::ListResult;
use std::sync::Arc;

/// Profile for one S3-compatible endpoint (mirrors `config::S3Profile`).
#[derive(Debug, Clone)]
pub struct S3Profile {
    pub endpoint: String,
    pub region: String,
    pub force_path_style: bool,
}

/// Wraps an `aws_sdk_s3::Client` with synchronous helper methods that
/// dispatch onto an owned single-threaded Tokio runtime.
///
/// The runtime is shared (`Arc`) with the chunked-transfer threads
/// (`spawn_upload`/`spawn_download`): aws-sdk's hyper connection pool is
/// tied to the runtime that drives it, so cloning the inner `Client` onto a
/// *different* runtime hangs once the pool holds live connections from the
/// original one (found live: T039 residual 2 — the transfer engine had
/// never actually run against a server before). Driving every request on
/// the one runtime keeps the pool coherent.
pub struct S3Client {
    inner: aws_sdk_s3::Client,
    profile_name: String,
    runtime: Arc<tokio::runtime::Runtime>,
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
            runtime: Arc::new(runtime),
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

    // ---- Chunked transfer engine (T039, additive) ----

    /// Spawn a chunked multipart upload on a dedicated background thread
    /// (same channel-then-foreground-poll shape as `git::watcher`, design
    /// spec §1). Returns immediately; the whole-object `put_object` above
    /// is untouched and still used for the provider/browsing path.
    pub fn spawn_upload(
        &self,
        job_id: u64,
        local_path: std::path::PathBuf,
        bucket: String,
        key: String,
    ) -> (
        transfer::CancelHandle,
        async_channel::Receiver<transfer::TransferEvent>,
    ) {
        let client = self.inner.clone();
        // Share the client's own runtime: the hyper connection pool is bound
        // to it (see `S3Client` docs) — a fresh runtime here hangs.
        let runtime = self.runtime.clone();
        let cancel = transfer::CancelHandle::new();
        let cancel_for_thread = cancel.clone();
        let (tx, rx) = async_channel::bounded(64);
        std::thread::spawn(move || {
            runtime.block_on(transfer::run_upload(
                &client,
                job_id,
                &local_path,
                &bucket,
                &key,
                &cancel_for_thread,
                &tx,
            ));
        });
        (cancel, rx)
    }

    /// Spawn a chunked ranged-GET download on a dedicated background
    /// thread. Mirrors [`S3Client::spawn_upload`].
    pub fn spawn_download(
        &self,
        job_id: u64,
        bucket: String,
        key: String,
        local_path: std::path::PathBuf,
    ) -> (
        transfer::CancelHandle,
        async_channel::Receiver<transfer::TransferEvent>,
    ) {
        let client = self.inner.clone();
        // Share the client's own runtime — see `spawn_upload`.
        let runtime = self.runtime.clone();
        let cancel = transfer::CancelHandle::new();
        let cancel_for_thread = cancel.clone();
        let (tx, rx) = async_channel::bounded(64);
        std::thread::spawn(move || {
            runtime.block_on(transfer::run_download(
                &client,
                job_id,
                &bucket,
                &key,
                &local_path,
                &cancel_for_thread,
                &tx,
            ));
        });
        (cancel, rx)
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

    /// Deterministic pseudo-random bytes (LCG), so the byte proof is
    /// reproducible and not just `vec![0u8; n]`.
    fn lcg_bytes(len: usize) -> Vec<u8> {
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (state >> 33) as u8
            })
            .collect()
    }

    /// Wait for a terminal `TransferEvent` on `rx`, returning the collected
    /// events and whether the job completed (vs failed/cancelled).
    fn wait_terminal(
        rx: &async_channel::Receiver<transfer::TransferEvent>,
        deadline: std::time::Instant,
    ) -> (Vec<transfer::TransferEvent>, bool) {
        let mut events = Vec::new();
        while std::time::Instant::now() < deadline {
            match rx.try_recv() {
                Ok(ev) => {
                    let terminal = matches!(
                        ev,
                        transfer::TransferEvent::Completed { .. }
                            | transfer::TransferEvent::Failed { .. }
                            | transfer::TransferEvent::Cancelled { .. }
                    );
                    events.push(ev);
                    if terminal {
                        let completed = matches!(
                            events.last(),
                            Some(transfer::TransferEvent::Completed { .. })
                        );
                        return (events, completed);
                    }
                }
                Err(async_channel::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(async_channel::TryRecvError::Closed) => break,
            }
        }
        (events, false)
    }

    /// T039 residual 2 — live byte proof against a local S3-compatible
    /// endpoint (MinIO on `127.0.0.1:9000`, seeded via
    /// `script/dev/t039_seed_minio.py` which creates the `photos` bucket).
    /// The chunked engine had never actually moved a byte to or from S3
    /// before this test (T039 report §2, "Not covered by any test").
    ///
    /// Skipped (early return) when no endpoint is reachable or the `photos`
    /// bucket is missing, so CI without a live server stays green.
    #[test]
    fn live_minio_chunked_transfer_byte_proof() {
        let profile = S3Profile {
            endpoint: "http://127.0.0.1:9000".to_string(),
            region: "us-east-1".to_string(),
            force_path_style: true,
        };
        let client = match S3Client::from_profile("rustfs", &profile, "minioadmin", "minioadmin") {
            Ok(c) => c,
            Err(_) => return,
        };
        // Reachability + fixture check; anything but a working server skips.
        let buckets = match client.list_buckets() {
            Ok(b) => b,
            Err(_) => return,
        };
        if !buckets.iter().any(|b| b.name == "photos") {
            return;
        }

        let tmp = tempfile::tempdir().unwrap();
        let key = "t039-proof/upload.bin";
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);

        // 1) Whole-object roundtrip (the provider path, unchanged by T039).
        let small = lcg_bytes(4096);
        client.put_object("photos", "t039-proof/small.bin", &small).unwrap();
        assert_eq!(
            client.get_object("photos", "t039-proof/small.bin").unwrap(),
            small,
            "whole-object put/get roundtrip must be byte-identical"
        );

        // 2) Chunked multipart upload: 2 full CHUNK_SIZE parts + a short
        //    tail = 3 parts, proving the multipart path (not just a
        //    single-part upload).
        let body = lcg_bytes((2 * transfer::CHUNK_SIZE) as usize + 123);
        let local = tmp.path().join("upload.bin");
        std::fs::write(&local, &body).unwrap();
        let (_, rx) = client.spawn_upload(1, local.clone(), "photos".into(), key.into());
        let (events, completed) = wait_terminal(&rx, deadline);
        assert!(
            completed,
            "chunked upload must complete; events: {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, transfer::TransferEvent::Progress(_))),
            "upload must report at least one progress event across chunks"
        );
        let stored = client.get_object("photos", key).unwrap();
        assert_eq!(stored.len(), body.len(), "stored size must match source");
        assert_eq!(stored, body, "chunked upload must be byte-identical");

        // 3) Chunked ranged-GET download, byte-identical on disk.
        let dl = tmp.path().join("download.bin");
        let (_, rx) =
            client.spawn_download(2, "photos".into(), key.into(), dl.clone());
        let (events, completed) = wait_terminal(&rx, deadline);
        assert!(
            completed,
            "chunked download must complete; events: {events:?}"
        );
        assert_eq!(
            std::fs::read(&dl).unwrap(),
            body,
            "chunked download must be byte-identical to the uploaded file"
        );

        // Cleanup so the live stand stays tidy.
        let _ = client.delete_object("photos", key);
        let _ = client.delete_object("photos", "t039-proof/small.bin");
    }
}
