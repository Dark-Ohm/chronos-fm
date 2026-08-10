//! Chunked S3 transfer engine (T039, design spec
//! `docs/superpowers/specs/2026-08-10-s3-tab-mockup-parity-design.md`).
//!
//! Additive to [`super::S3Client`]'s existing whole-object `get_object`/
//! `put_object` — those stay untouched for the provider/browsing path
//! (T011/T021). This module is only used by the Transfers view's explicit
//! Upload/Download actions.
//!
//! Chunk-boundary math and the job state machine's transition table are
//! pure and unit-tested below. The network-calling engine (multipart
//! upload / ranged download, further down this file) has not yet had an
//! integration pass against a live endpoint — see the design spec's
//! "Verification" §2 for what that still needs to cover (this session
//! only ran the pure unit tests, not RustFS).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::CompletedMultipartUpload;
use aws_sdk_s3::types::CompletedPart;
use aws_sdk_s3::Client;

/// Part size for both multipart upload and ranged download windows (design
/// spec §1) — matches S3's own minimum part size (5 MiB) with headroom,
/// chosen so the last part is never below the minimum even for files a few
/// bytes over a chunk boundary.
pub const CHUNK_SIZE: u64 = 8 * 1024 * 1024;

/// One `[start, end)` byte range within the object/file, half-open so
/// adjacent chunks share no byte and `end - start` is always the chunk's
/// exact length (including a shorter final chunk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub index: usize,
    pub start: u64,
    /// Exclusive.
    pub end: u64,
}

impl Chunk {
    pub fn len(&self) -> u64 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Split `total_size` bytes into `CHUNK_SIZE`-bounded chunks, half-open,
/// contiguous, covering exactly `[0, total_size)`. A zero-byte object
/// yields a single empty chunk (index 0, start == end == 0) rather than an
/// empty `Vec` — S3 still requires at least one part for a multipart
/// upload of an empty file, and callers can special-case `is_empty()` if
/// they need to skip the network call entirely.
pub fn split_into_chunks(total_size: u64) -> Vec<Chunk> {
    if total_size == 0 {
        return vec![Chunk {
            index: 0,
            start: 0,
            end: 0,
        }];
    }
    let mut chunks = Vec::with_capacity((total_size / CHUNK_SIZE + 1) as usize);
    let mut start = 0u64;
    let mut index = 0usize;
    while start < total_size {
        let end = (start + CHUNK_SIZE).min(total_size);
        chunks.push(Chunk { index, start, end });
        start = end;
        index += 1;
    }
    chunks
}

/// Direction of a transfer job (design spec §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
}

/// A transfer job's lifecycle state (design spec §2). No `Paused` variant
/// — see the design doc for why (no pause control in the mockup, and
/// resumable multipart uploads need part-ETag persistence across restarts,
/// which is out of scope here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferState {
    Queued,
    InProgress,
    Completed,
    Failed { reason: String },
    Cancelled,
}

impl TransferState {
    /// Whether this is one of the three terminal states — a UI or test can
    /// use this instead of hand-matching all three every time.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TransferState::Completed | TransferState::Failed { .. } | TransferState::Cancelled
        )
    }

    /// Whether `self -> next` is a legal transition per the design spec's
    /// table. Pure and total (never panics on any pair) so it can gate
    /// every state mutation in the engine/page without trusting call
    /// sites to only ever construct valid sequences by convention.
    pub fn can_transition_to(&self, next: &TransferState) -> bool {
        use TransferState::*;
        match (self, next) {
            (Queued, InProgress) => true,
            (Queued, Cancelled) => true,
            (InProgress, Completed) => true,
            (InProgress, Failed { .. }) => true,
            (InProgress, Cancelled) => true,
            _ => false,
        }
    }
}

/// One in-flight or finished transfer (design spec §2's `TransferJob`,
/// promoted here so both the engine and the page state share one
/// definition instead of two structurally-identical structs drifting
/// apart).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferJob {
    pub id: u64,
    pub direction: TransferDirection,
    pub local_path: PathBuf,
    pub bucket: String,
    pub key: String,
    pub state: TransferState,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

impl TransferJob {
    /// Attempt the transition; returns `false` (and leaves `self.state`
    /// unchanged) for an illegal transition instead of silently applying
    /// it or panicking — a caller that ignores the return value gets a
    /// job stuck in its old (still-valid) state rather than a corrupted
    /// one.
    #[must_use]
    pub fn transition_to(&mut self, next: TransferState) -> bool {
        if !self.state.can_transition_to(&next) {
            return false;
        }
        self.state = next;
        true
    }
}

/// Progress event sent from the background transfer task to the page's
/// foreground poll (design spec §1 — same channel-then-foreground-poll
/// shape as `git::watcher::GitWatcher`, not a new pattern).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferProgress {
    pub job_id: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// Cooperative cancellation flag shared between the page (which calls
/// [`CancelHandle::cancel`] from a Cancel button) and the background
/// transfer loop (which checks [`CancelHandle::is_cancelled`] between
/// chunks — never mid-chunk, per the design spec).
#[derive(Debug, Clone, Default)]
pub struct CancelHandle(Arc<AtomicBool>);

impl CancelHandle {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// What the background transfer loop reports back to the page's
/// foreground poll, over the same channel-then-poll shape as
/// `git::watcher::GitWatcher` (design spec §1).
#[derive(Debug, Clone)]
pub enum TransferEvent {
    Progress(TransferProgress),
    Completed { job_id: u64 },
    Failed { job_id: u64, reason: String },
    Cancelled { job_id: u64 },
}

/// One error-matrix reason string (design spec §3) — kept as a small enum
/// internally so the retry decision (`is_transient`) and the final
/// message text live next to each other instead of drifting apart.
enum TransferError {
    LocalFileGone(PathBuf),
    Transient { part: usize, total: usize, source: String },
    AccessDenied { bucket: String, key: String },
    NotFound { bucket: String, key: String },
    DiskFull { path: PathBuf, source: String },
    Other(String),
}

impl TransferError {
    fn message(&self) -> String {
        match self {
            TransferError::LocalFileGone(path) => {
                format!("local file no longer exists: {}", path.display())
            }
            TransferError::Transient { part, total, source } => {
                format!("network timeout on part {part}/{total} after 1 retry: {source}")
            }
            TransferError::AccessDenied { bucket, key } => {
                format!("access denied: {bucket}/{key} — check credentials or bucket policy")
            }
            TransferError::NotFound { bucket, key } => {
                format!("object no longer exists: {bucket}/{key}")
            }
            TransferError::DiskFull { path, source } => {
                format!("disk full writing {}: {source}", path.display())
            }
            TransferError::Other(msg) => msg.clone(),
        }
    }
}

/// Best-effort transient-vs-permanent classification from an SDK error's
/// `Display` text. Not a substitute for matching on typed service errors
/// (`ProvideErrorMetadata::code()`) — flagged here as a residual: a typed
/// classification per operation (`GetObjectError`, `UploadPartError`, …)
/// would be more precise but multiplies call sites for a first pass. 4xx
/// codes never retry; anything else gets the one automatic retry the
/// design spec calls for.
fn is_permanent_denied(err_text: &str) -> bool {
    err_text.contains("AccessDenied") || err_text.contains("403")
}

fn is_not_found(err_text: &str) -> bool {
    err_text.contains("NoSuchKey") || err_text.contains("NotFound") || err_text.contains("404")
}

/// Run a chunked multipart upload of `local_path` to `bucket`/`key`,
/// reporting progress and the terminal event on `events`. Sequential
/// parts (design spec: simpler cancellation/progress in v1). Aborts the
/// multipart upload on cancel or failure so no orphaned upload is left on
/// the S3 side (S3 bills for those).
pub async fn run_upload(
    client: &Client,
    job_id: u64,
    local_path: &Path,
    bucket: &str,
    key: &str,
    cancel: &CancelHandle,
    events: &async_channel::Sender<TransferEvent>,
) {
    let metadata = match std::fs::metadata(local_path) {
        Ok(m) => m,
        Err(_) => {
            let _ = events
                .send(TransferEvent::Failed {
                    job_id,
                    reason: TransferError::LocalFileGone(local_path.to_path_buf()).message(),
                })
                .await;
            return;
        }
    };
    let total_size = metadata.len();
    let chunks = split_into_chunks(total_size);

    let create = match client
        .create_multipart_upload()
        .bucket(bucket)
        .key(key)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            let _ = events
                .send(TransferEvent::Failed {
                    job_id,
                    reason: TransferError::Other(format!("create_multipart_upload: {e}")).message(),
                })
                .await;
            return;
        }
    };
    let Some(upload_id) = create.upload_id().map(str::to_string) else {
        let _ = events
            .send(TransferEvent::Failed {
                job_id,
                reason: TransferError::Other("create_multipart_upload returned no upload id".into())
                    .message(),
            })
            .await;
        return;
    };

    let mut completed_parts = Vec::with_capacity(chunks.len());
    let mut bytes_done = 0u64;
    let total_parts = chunks.len();

    for chunk in &chunks {
        if cancel.is_cancelled() {
            abort_upload(client, bucket, key, &upload_id).await;
            let _ = events.send(TransferEvent::Cancelled { job_id }).await;
            return;
        }

        let mut buf = vec![0u8; chunk.len() as usize];
        if let Err(e) = read_exact_at(local_path, chunk.start, &mut buf) {
            let reason = if e.kind() == std::io::ErrorKind::NotFound {
                TransferError::LocalFileGone(local_path.to_path_buf()).message()
            } else {
                TransferError::Other(format!("reading {}: {e}", local_path.display())).message()
            };
            abort_upload(client, bucket, key, &upload_id).await;
            let _ = events.send(TransferEvent::Failed { job_id, reason }).await;
            return;
        }

        let part_number = (chunk.index + 1) as i32;
        let attempt = upload_one_part(client, bucket, key, &upload_id, part_number, buf.clone()).await;
        let result = match attempt {
            Ok(etag) => Ok(etag),
            Err(e) if is_permanent_denied(&e) => Err(TransferError::AccessDenied {
                bucket: bucket.to_string(),
                key: key.to_string(),
            }),
            Err(first_err) => {
                // One automatic retry for a transient failure only.
                match upload_one_part(client, bucket, key, &upload_id, part_number, buf).await {
                    Ok(etag) => Ok(etag),
                    Err(e) => Err(TransferError::Transient {
                        part: part_number as usize,
                        total: total_parts,
                        source: if e.is_empty() { first_err } else { e },
                    }),
                }
            }
        };

        match result {
            Ok(etag) => {
                completed_parts.push(
                    CompletedPart::builder()
                        .part_number(part_number)
                        .e_tag(etag)
                        .build(),
                );
                bytes_done += chunk.len();
                let _ = events
                    .send(TransferEvent::Progress(TransferProgress {
                        job_id,
                        bytes_done,
                        bytes_total: total_size,
                    }))
                    .await;
            }
            Err(err) => {
                abort_upload(client, bucket, key, &upload_id).await;
                let _ = events
                    .send(TransferEvent::Failed {
                        job_id,
                        reason: err.message(),
                    })
                    .await;
                return;
            }
        }
    }

    let complete = client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(&upload_id)
        .multipart_upload(
            CompletedMultipartUpload::builder()
                .set_parts(Some(completed_parts))
                .build(),
        )
        .send()
        .await;

    match complete {
        Ok(_) => {
            let _ = events.send(TransferEvent::Completed { job_id }).await;
        }
        Err(e) => {
            let _ = events
                .send(TransferEvent::Failed {
                    job_id,
                    reason: TransferError::Other(format!("complete_multipart_upload: {e}")).message(),
                })
                .await;
        }
    }
}

async fn upload_one_part(
    client: &Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
    part_number: i32,
    buf: Vec<u8>,
) -> Result<String, String> {
    let resp = client
        .upload_part()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .part_number(part_number)
        .body(ByteStream::from(buf))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.e_tag()
        .map(str::to_string)
        .ok_or_else(|| "upload_part returned no ETag".to_string())
}

/// Best-effort abort — per the design spec's error matrix, a failure here
/// is logged, not surfaced as a second user-facing error (the job is
/// already Failed/Cancelled from the user's perspective; an orphaned
/// multipart upload is a background-cleanup concern).
async fn abort_upload(client: &Client, bucket: &str, key: &str, upload_id: &str) {
    if let Err(e) = client
        .abort_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .send()
        .await
    {
        tracing::warn!("abort_multipart_upload {bucket}/{key} ({upload_id}) failed: {e}");
    }
}

fn read_exact_at(path: &Path, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(buf)
}

/// Run a chunked ranged-GET download of `bucket`/`key` to `local_path`.
/// On cancel, the partial local file is left in place (matches ordinary
/// browser-download behavior — see design spec §1).
pub async fn run_download(
    client: &Client,
    job_id: u64,
    bucket: &str,
    key: &str,
    local_path: &Path,
    cancel: &CancelHandle,
    events: &async_channel::Sender<TransferEvent>,
) {
    let head = match client.head_object().bucket(bucket).key(key).send().await {
        Ok(resp) => resp,
        Err(e) => {
            let text = e.to_string();
            let reason = if is_not_found(&text) {
                TransferError::NotFound {
                    bucket: bucket.to_string(),
                    key: key.to_string(),
                }
                .message()
            } else {
                TransferError::Other(format!("head_object: {e}")).message()
            };
            let _ = events.send(TransferEvent::Failed { job_id, reason }).await;
            return;
        }
    };
    let total_size = head.content_length().unwrap_or(0).max(0) as u64;
    let chunks = split_into_chunks(total_size);

    let mut file = match File::create(local_path) {
        Ok(f) => f,
        Err(e) => {
            let _ = events
                .send(TransferEvent::Failed {
                    job_id,
                    reason: TransferError::DiskFull {
                        path: local_path.to_path_buf(),
                        source: e.to_string(),
                    }
                    .message(),
                })
                .await;
            return;
        }
    };

    let mut bytes_done = 0u64;
    for chunk in &chunks {
        if cancel.is_cancelled() {
            let _ = events.send(TransferEvent::Cancelled { job_id }).await;
            return;
        }

        let range = format!("bytes={}-{}", chunk.start, chunk.end.saturating_sub(1));
        let attempt = fetch_range(client, bucket, key, &range).await;
        let bytes = match attempt {
            Ok(b) => b,
            Err(e) if is_not_found(&e) => {
                let _ = events
                    .send(TransferEvent::Failed {
                        job_id,
                        reason: TransferError::NotFound {
                            bucket: bucket.to_string(),
                            key: key.to_string(),
                        }
                        .message(),
                    })
                    .await;
                return;
            }
            Err(e) if is_permanent_denied(&e) => {
                let _ = events
                    .send(TransferEvent::Failed {
                        job_id,
                        reason: TransferError::AccessDenied {
                            bucket: bucket.to_string(),
                            key: key.to_string(),
                        }
                        .message(),
                    })
                    .await;
                return;
            }
            Err(first_err) => match fetch_range(client, bucket, key, &range).await {
                Ok(b) => b,
                Err(e) => {
                    let _ = events
                        .send(TransferEvent::Failed {
                            job_id,
                            reason: TransferError::Transient {
                                part: chunk.index + 1,
                                total: chunks.len(),
                                source: if e.is_empty() { first_err } else { e },
                            }
                            .message(),
                        })
                        .await;
                    return;
                }
            },
        };

        if let Err(e) = file.seek(SeekFrom::Start(chunk.start)).and_then(|_| file.write_all(&bytes)) {
            let _ = events
                .send(TransferEvent::Failed {
                    job_id,
                    reason: TransferError::DiskFull {
                        path: local_path.to_path_buf(),
                        source: e.to_string(),
                    }
                    .message(),
                })
                .await;
            return;
        }

        bytes_done += chunk.len();
        let _ = events
            .send(TransferEvent::Progress(TransferProgress {
                job_id,
                bytes_done,
                bytes_total: total_size,
            }))
            .await;
    }

    let _ = events.send(TransferEvent::Completed { job_id }).await;
}

async fn fetch_range(client: &Client, bucket: &str, key: &str, range: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .range(range)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let bytes = resp.body.collect().await.map_err(|e| e.to_string())?;
    Ok(bytes.into_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_into_chunks_zero_size_yields_one_empty_chunk() {
        let chunks = split_into_chunks(0);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].is_empty());
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 0);
    }

    #[test]
    fn split_into_chunks_exact_multiple_of_chunk_size() {
        let total = CHUNK_SIZE * 3;
        let chunks = split_into_chunks(total);
        assert_eq!(chunks.len(), 3);
        for (i, c) in chunks.iter().enumerate() {
            assert_eq!(c.index, i);
            assert_eq!(c.len(), CHUNK_SIZE);
        }
        assert_eq!(chunks.last().unwrap().end, total);
    }

    #[test]
    fn split_into_chunks_last_chunk_is_shorter() {
        let total = CHUNK_SIZE * 2 + 100;
        let chunks = split_into_chunks(total);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), CHUNK_SIZE);
        assert_eq!(chunks[1].len(), CHUNK_SIZE);
        assert_eq!(chunks[2].len(), 100);
    }

    #[test]
    fn split_into_chunks_smaller_than_one_chunk() {
        let chunks = split_into_chunks(42);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 42);
    }

    #[test]
    fn split_into_chunks_are_contiguous_and_cover_the_whole_range() {
        // Property check across several sizes, not just the hand-picked
        // boundary cases above.
        for total in [1u64, 100, CHUNK_SIZE - 1, CHUNK_SIZE, CHUNK_SIZE + 1, CHUNK_SIZE * 5 + 7] {
            let chunks = split_into_chunks(total);
            assert_eq!(chunks[0].start, 0, "first chunk must start at 0 for total={total}");
            assert_eq!(
                chunks.last().unwrap().end,
                total,
                "last chunk must end exactly at total={total}"
            );
            for w in chunks.windows(2) {
                assert_eq!(
                    w[0].end, w[1].start,
                    "chunks must be contiguous with no gap/overlap for total={total}"
                );
            }
            let sum: u64 = chunks.iter().map(Chunk::len).sum();
            assert_eq!(sum, total, "chunk lengths must sum to total for total={total}");
        }
    }

    #[test]
    fn state_machine_allows_only_documented_transitions() {
        use TransferState::*;
        assert!(Queued.can_transition_to(&InProgress));
        assert!(Queued.can_transition_to(&Cancelled));
        assert!(InProgress.can_transition_to(&Completed));
        assert!(InProgress.can_transition_to(&Failed {
            reason: "x".into()
        }));
        assert!(InProgress.can_transition_to(&Cancelled));
    }

    #[test]
    fn state_machine_rejects_undocumented_transitions() {
        use TransferState::*;
        // No going back from a terminal state.
        assert!(!Completed.can_transition_to(&InProgress));
        assert!(!Cancelled.can_transition_to(&InProgress));
        assert!(!Failed { reason: "x".into() }.can_transition_to(&InProgress));
        // Can't skip Queued -> InProgress directly to Completed/Failed —
        // a job must actually run first.
        assert!(!Queued.can_transition_to(&Completed));
        assert!(!Queued.can_transition_to(&Failed {
            reason: "x".into()
        }));
        // No self-transitions.
        assert!(!Queued.can_transition_to(&Queued));
        assert!(!InProgress.can_transition_to(&InProgress));
    }

    #[test]
    fn transition_to_mutates_only_on_legal_transition() {
        let mut job = TransferJob {
            id: 1,
            direction: TransferDirection::Upload,
            local_path: PathBuf::from("/tmp/x"),
            bucket: "b".into(),
            key: "k".into(),
            state: TransferState::Queued,
            bytes_done: 0,
            bytes_total: 100,
        };
        assert!(job.transition_to(TransferState::InProgress));
        assert_eq!(job.state, TransferState::InProgress);

        // Illegal: InProgress -> Queued isn't in the table.
        assert!(!job.transition_to(TransferState::Queued));
        assert_eq!(
            job.state,
            TransferState::InProgress,
            "an illegal transition must leave the job's state unchanged"
        );

        assert!(job.transition_to(TransferState::Completed));
        assert_eq!(job.state, TransferState::Completed);
    }

    #[test]
    fn is_terminal_matches_the_three_end_states() {
        assert!(!TransferState::Queued.is_terminal());
        assert!(!TransferState::InProgress.is_terminal());
        assert!(TransferState::Completed.is_terminal());
        assert!(TransferState::Cancelled.is_terminal());
        assert!(TransferState::Failed { reason: "x".into() }.is_terminal());
    }
}
