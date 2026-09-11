# T061 — Google Drive tab

**Epic:** T059. **Priority:** P2.
**Depends:** T060 (OAuth foundation proven against Google) — blocked
until T060 ACCEPT.
**Code:** `crates/chronos-fm-services/src/gdrive/` (provider + transfer),
`crates/chronos-fm-pages/src/gdrive.rs` (tab UI, T039 shape).

## Problem

First real cloud-provider tab, exercising T060's OAuth foundation and the
`FileSystemProvider` trait against Drive API v3.

## Must

1. `FileSystemProvider` impl for Drive: `list_dir`/`read_file`/
   `write_file`/`delete`/`rename`/`metadata`. Drive's file-ID model (not a
   path hierarchy like S3 keys) needs a documented mapping to the
   path-shaped interface — note the impedance mismatch and how it's
   resolved, don't paper over it.
2. Tab UI, T039 pattern: sub-nav (Explorer / Transfers / Properties — no
   Buckets equivalent, Drive is single-root per account), reuse
   `dialogs::pick_file`/`pick_directory` for upload/download same as S3.
3. Chunked/resumable upload for large files (Drive API supports resumable
   uploads — use it, don't naively buffer whole files in memory).
4. Honest empty/error states (T016/T039 discipline): auth not yet
   connected, API error, empty folder — each visually distinct, none
   silently indistinguishable from "working but empty".
5. Multi-account: out of scope for v1 unless trivial from T060's design —
   state explicitly if deferred.

## Design gate

Before large implementation: short note — which parts of T060's OAuth
flow are reused as-is, what Drive-specific token scopes are needed
(`drive.file` vs `drive` — prefer the narrowest scope that works, name
why if broader is required).

## Done when

1. Live proof: connect a real Google account, list real files/folders,
   upload and download a real file, byte-identical roundtrip (T039's
   `live_minio_chunked_transfer_byte_proof` is the evidence bar to match).
2. `class=chronos-fm` grims of the tab with real Drive data, OCR- or
   visually-verified content (T039's grim discipline).
3. Unit tests for the path/file-ID mapping logic (pure, no network) +
   workspace green.
4. Report + architect stamp. No self-ACCEPT.

## Related

T059 (epic) · T060 (auth dependency) · T039 (closest sibling —
`s3.rs`/`s3/provider.rs`/`s3/transfer.rs` are the reference shape)
