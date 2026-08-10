# T039 S3 Tab Mockup Parity — Design

**Date:** 2026-08-10
**Status:** **Architect APPROVED** (2026-08-10) — gate 1+2 closed; implement GO.
gate 3 (no secrets) remains ongoing discipline.
**Visual source of truth:** `docs/design/mockups/Chronos-S3-Tab.dc.html`
**Code source of truth:** `crates/chronos-fm-pages/src/s3.rs` and
`crates/chronos-fm-services/src/s3/mod.rs`

## Goal

Bring the S3 tab to the mockup's four-view shell (Explorer / Buckets /
Transfers / Properties) with a **real** chunked transfer subsystem behind
Transfers — measurable progress, cooperative cancellation, retry — not a
cosmetic queue. Preserve the existing whole-object provider path (T011/
T021) untouched; the chunked engine is additive, not a replacement.

## Gate 1 — ashpd compile probe: RESOLVED

`crates/chronos-fm-services/src/dialogs.rs` (this session). `ashpd = {
version = "0.13", default-features = false, features = ["async-io",
"file_chooser"] }` added directly to `chronos-fm-services/Cargo.toml`,
matching Source's own workspace pin
(`Source/Cargo.toml:161-163`) so it resolves to the **same already-locked**
instance pulled in transitively via `gpui_linux`/`oo7` — confirmed via
`grep -c '^name = "ashpd"' Cargo.lock` / `zbus` both `1`, not `2`.
`pick_file`/`pick_directory` wrap `ashpd::desktop::file_chooser::
SelectedFiles::open_file()`; `uri_to_path` hand-parses the `file://`
scheme + percent-decodes (`ashpd::Uri` is a thin string wrapper, not
`url::Url` — no path-conversion helper exists on it). 3 unit tests
(accepts `file://`, rejects non-file schemes like `mtp://`, percent-decode
round-trip), `cargo test -p chronos-fm-services dialogs::` green. This
resolves the open sub-decision in the original T039 report ("no
established native dialog dependency... recommended: add a direct `ashpd`
dependency... must be verified with a compile probe").

**Not yet done:** wiring `pick_file`/`pick_directory` into the Transfers
view's Upload/Download local-path selection (§4 below) — this document is
the design, not the implementation.

## Gate 2 — this document

## Current facts and scope boundary

`S3Client` (`s3/mod.rs`) wraps `aws-sdk-s3` synchronously via an owned
single-threaded Tokio runtime; every method (`list_buckets`,
`list_objects`, `get_object`, `put_object`, `delete_object`,
`head_object`) does a whole-object `block_on` call — no streaming, no
progress, no cancellation. `S3Page` (`s3.rs`) has a five-state connect
flow (`NoProfiles`/`NeedCredentials`/`Connecting`/`Browsing`/`Error`) and,
once connected, embeds an `ExplorerPane` for browsing — no multi-view
shell, no bucket list view, no transfer queue, no properties panel exist
today (confirmed in the original T039 report's inventory, still accurate).

`aws-sdk-s3 1.140.0` (locked) has `create_multipart_upload`, `upload_part`,
`complete_multipart_upload`, `abort_multipart_upload`, and ranged `GetObject`
(`range` header) — all available for a real chunked engine without a new
SDK dependency.

## Design

### 1. Chunked transfer engine (`chronos-fm-services/src/s3/transfer.rs`, new)

Two directions, both chunked, both cooperatively cancellable:

- **Upload (multipart):** `create_multipart_upload` → split the local file
  into fixed-size parts (8 MiB, matching S3's minimum part size other than
  the last part) → `upload_part` per chunk, sequentially (no parallel
  parts in v1 — simpler cancellation/progress semantics, revisit if
  throughput is proven insufficient) → `complete_multipart_upload` on
  success or `abort_multipart_upload` on cancel/failure (never leave an
  orphaned incomplete multipart upload — S3 bills for those).
- **Download (ranged GET):** repeated `get_object` calls with a `Range:
  bytes=N-M` header, 8 MiB windows, written to the local file via
  `std::fs::File` at the corresponding offset (`seek` + `write_all`) —
  avoids buffering the whole object in memory, matches the upload side's
  chunk size for symmetric progress reporting.
- **Progress:** each chunk completion sends `TransferProgress { job_id,
  bytes_done, bytes_total }` on an `async_channel` (already a workspace
  dependency, used elsewhere in this crate) from the background Tokio task
  to the page's `cx.spawn`-driven foreground poll — same
  channel-then-foreground-poll shape as `GitWatcher`'s `.git`-change
  signal (`git/watcher.rs`), not a new pattern.
- **Cancellation:** cooperative, checked **between** chunks (not
  mid-chunk — a part upload is atomic from S3's perspective, no reason to
  abort mid-part). A `CancellationToken`-shaped flag (`Arc<AtomicBool>`,
  no new dependency needed for something this simple) checked at the top
  of each chunk's loop iteration; on cancel, upload aborts the multipart
  upload, download leaves the partial local file in place (mirrors how a
  browser download resume/cancel typically behaves — deleting a
  partially-downloaded multi-GB file the user might want to resume later
  would be more surprising than leaving it).
- **Retry:** one automatic retry per chunk on a transient error (network
  timeout, 5xx) before marking the job Failed; no retry on 4xx (auth,
  permissions, bucket policy — retrying won't fix these and would hide a
  real error behind a delay).

### 2. Job state machine

```
Queued → InProgress → Completed
                    ↘ Failed { reason: String }
                    ↘ Cancelled
```

No `Paused` state in v1 — the mockup doesn't show a pause control (only
implied by "cancel" in the rows sketch), and pause/resume for a multipart
upload requires persisting part ETags across a restart, which is real
scope the mockup doesn't ask for. If a later ticket wants pause/resume,
it's additive to this state machine, not a redesign.

| Transition | Trigger |
|---|---|
| `Queued → InProgress` | Engine picks up the next queued job (single job in flight per profile in v1 — matches "not a cosmetic queue" without needing a concurrency-limit design yet; revisit if real usage shows this is too slow) |
| `InProgress → Completed` | Last chunk + `complete_multipart_upload` (or last ranged GET) succeeds |
| `InProgress → Failed` | A chunk's retry also fails, or `complete_multipart_upload`/final GET fails |
| `InProgress → Cancelled` | User clicks Cancel; engine finishes the in-flight chunk, then stops instead of starting the next one |
| `Queued → Cancelled` | User clicks Cancel before the job started (no in-flight chunk to finish) |

`TransferJob` (page state, not service state — the service is stateless
per-call, matching `S3Client`'s existing shape):

```rust
struct TransferJob {
    id: u64,
    direction: TransferDirection, // Upload | Download
    local_path: PathBuf,
    bucket: String,
    key: String,
    state: TransferState,
    bytes_done: u64,
    bytes_total: u64,
}
```

### 3. Error matrix (concrete reasons, never a bare "error")

| Failure | Surfaced as |
|---|---|
| Local file vanished mid-upload | `"local file no longer exists: {path}"` |
| Network timeout, chunk N of M | after retry also fails: `"network timeout on part {n}/{m} after 1 retry: {source}"` |
| 403 (permissions/policy) | `"access denied: {bucket}/{key} — check credentials or bucket policy"` (never retried) |
| 404 (download, object vanished) | `"object no longer exists: {bucket}/{key}"` |
| Disk full (download write) | `"disk full writing {path}: {os_error}"` |
| Multipart abort itself fails (cancel path) | logged, not surfaced as a second user-facing error — the job is already Cancelled from the user's perspective; an orphaned multipart upload on the S3 side is a background-cleanup concern, not a UI error |

### 4. Page state and navigation (`crates/chronos-fm-pages/src/s3.rs`)

Mirrors T038's `GitView` pattern:

```rust
enum S3View { Explorer, Buckets, Transfers, Properties }
```

- **Explorer:** existing embedded `ExplorerPane` — unchanged.
- **Buckets:** `list_buckets()` result as a real list (already available,
  just not surfaced as its own view today).
- **Transfers:** `Vec<TransferJob>` + progress bars driven by the channel
  above; Upload/Download buttons open `dialogs::pick_file`/
  `pick_directory` (gate 1) for the local side; **no fake progress** — a
  job with `bytes_total == 0` (unknown size, shouldn't happen for
  chunked-with-known-size transfers but defensively) shows an
  indeterminate state, not a fabricated percentage.
- **Properties:** object/bucket metadata from `head_object` when
  available; honest empty state (T023 pattern) when nothing is selected.

### 5. Interaction and safety rules

- T011/T021 whole-object provider path (`ExplorerPane` browsing) is
  **not** touched — chunked engine is additive, used only by the
  Transfers view's explicit Upload/Download actions.
- Secrets (S3 access key / secret key) never appear in a `TransferJob`,
  never get logged, never reach a report/grim artifact — same rule as the
  existing connect flow already follows (keyring only).
- Local paths from `dialogs::pick_file`/`pick_directory` are real,
  user-selected paths; no default/guessed path is ever silently
  substituted if the picker returns `None` (cancelled).

## Verification (once implemented — not yet done)

1. Unit tests for the chunk-splitting logic (part boundaries, last-part
   sizing) and the state machine's transition table — pure functions,
   no S3 network calls needed for these.
2. Integration test against RustFS (the existing live S3 test stand,
   `rustfs-for-s3-testing` memory) — real multipart upload + real ranged
   download, byte-for-byte verification against the seeded content,
   matching T021's precedent ("совпадение самих данных с засеянными", not
   an access-log claim RustFS can't back).
3. Cancellation test: start a large-enough transfer, cancel mid-flight,
   assert the multipart upload was aborted (no orphaned upload) and no
   fake "completed" state was reached.
4. `cargo build --release -p chronos-fm`, live grim of all four views
   against `Chronos-S3-Tab.dc.html`, vision review — same discipline as
   T038 (and T038's own visual-proof gap — see **T046** — means this
   ticket should not repeat that mistake: budget real live-verification
   time before claiming visual ACCEPT, not just code-complete).

## Explicit residuals

Parallel multipart parts (throughput), pause/resume across a session
restart, and a global (not per-profile) concurrency limit are not in this
design — flagged here rather than silently added or silently dropped from
scope.
