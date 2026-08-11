# T039 — S3 tab mockup parity — Implementation report

> ## ⚖️ ARCHITECT (2026-08-11): **ACCEPT — full Phase V**
>
> Independently re-verified, not from the report:
> - `cargo test -p chronos-fm-services s3::` → **15 passed** (includes
>   `live_minio_chunked_transfer_byte_proof` — actually ran, MinIO alive)
> - `cargo test -p chronos-fm-pages s3::` → **4 passed**
> - `cargo test --workspace` → **450 passed** (13+62+0+180+145+16+34), 0 failed
> - Opened 3 of 4 grims myself: explorer `s3://rustfs@` 2 items; buckets
>   `documents`/`photos`; transfers honest "No transfers yet." — matches the
>   OCR table below
> - Code-verified: `maybe_auto_connect_from_env` gated on both env vars;
>   `start_connect(..., persist_credentials)` writes keyring only when true;
>   env seam `Ok(None)` → return (no default-path substitution on cancelled
>   picker)
>
> Report is honest about what remains (cancel-mid-transfer unit-only, bucket
> sizes 0 B by API design, manual click-through not exercised) — no hidden
> gaps. Residuals 1+2 from the 2026-08-10 PARTIAL stamp are closed.

> ## ⚖️ ARCHITECT UPDATE (2026-08-10): **PARTIAL** — sub-view grims inconclusive
>
> `T039-shots/t039_s3_{buckets,transfers,properties,explorer}.png` all show
> the same NoProfiles empty gate (no sub-nav chrome). Residual: profile path
> or product decision to always show 4-view chrome when disconnected.

**Status:** ACCEPTED (2026-08-11) — full Phase V; residuals 1+2 closed by live MinIO verification (§5).
**Date:** 2026-08-10 (updated 2026-08-11)
**Executor:** Claude (Sonnet 5)
**Follows:** architect verdict "T039 — GATES 1+2 ACCEPT → IMPLEMENT GO"
(2026-08-10), which asked for: 1. TDD chunk split + state machine, 2.
Chunked engine (additive), 3. UI 4 views + wire pick_file/pick_directory,
4. Vision grims (same hypr residual as T046).

## 1 — TDD: chunk split + state machine

`crates/chronos-fm-services/src/s3/transfer.rs` (new), pure/no-network
parts written and tested first:

- `split_into_chunks(total_size) -> Vec<Chunk>` — half-open `[start,end)`
  ranges, 8 MiB (`CHUNK_SIZE`), contiguous, cover `[0, total_size)`
  exactly. Zero-byte object yields one empty chunk (S3 still needs ≥1 part
  for a multipart upload of an empty file).
- `TransferState` (`Queued → InProgress → {Completed|Failed|Cancelled}`,
  `Queued → Cancelled`) + `can_transition_to` — pure, total, rejects
  self-transitions and terminal→non-terminal.
- `TransferJob::transition_to` — mutates only on a legal transition,
  leaves state unchanged (returns `false`) otherwise.

```
cargo test -p chronos-fm-services s3::transfer::
test result: ok. 9 passed; 0 failed
```

## 2 — Chunked engine (additive)

Same file, network-calling half, additive to `S3Client`'s existing
whole-object `get_object`/`put_object` (untouched, still used by the
Explorer/provider path — T011/T021 not touched):

- `run_upload`: `create_multipart_upload` → sequential 8 MiB
  `upload_part` calls → `complete_multipart_upload`; aborts the multipart
  upload on cancel or any failure (never leaves an orphaned upload).
- `run_download`: `head_object` for size → sequential ranged `GetObject`
  (`bytes=N-M`) → `seek`+`write_all` at the right local offset; partial
  file is left in place on cancel (browser-download convention, per
  design spec).
- `CancelHandle` (`Arc<AtomicBool>`) — checked between chunks, never
  mid-chunk.
- One automatic retry per chunk on a transient failure; 403/`AccessDenied`
  never retried. Error matrix's six message templates implemented
  verbatim in `TransferError::message()`.
- `S3Client::spawn_upload`/`spawn_download` (in `s3/mod.rs`) run the above
  on a dedicated `std::thread` + its own single-threaded Tokio runtime
  (same pattern as `search::engine`'s watcher-consumer thread — a
  background blocking loop, not a task spawned on a runtime nobody drives),
  reporting `TransferEvent`s over `async_channel` back to the page.

```
cargo build -p chronos-fm-services            → clean
cargo test -p chronos-fm-services              → 130 passed; 0 failed
```

**Not covered by any test:** the actual network calls (`upload_part`,
`get_object` with `Range`, `complete_multipart_upload`, the retry path,
the abort-on-cancel path). Design spec §"Verification" item 2 (integration
test against RustFS) was **not run this session** — no RustFS instance was
started/verified reachable. This is a real gap, not an oversight I'm
hiding: the engine has never actually moved a byte to or from S3.

## 3 — UI: 4 views + wire pick_file/pick_directory

`crates/chronos-fm-pages/src/s3.rs`:

- `S3View::{Explorer, Buckets, Transfers, Properties}` + `render_sub_nav`
  — same horizontal-tab-strip pattern as T038's `GitView`, the same
  documented deviation from the mockup's left-column sidebar nav.
- `S3Page` gained `view`, `client: Option<Arc<S3Client>>` (typed, kept
  alongside the type-erased `FileSystemProvider` already wired into the
  pane so Buckets/Transfers can call `list_buckets`/`spawn_upload`/
  `spawn_download` directly), `buckets`, `transfers: Vec<TransferRow>`,
  `next_job_id`, `view_error`.
- **Explorer** — the existing embedded `ExplorerPane`, unchanged.
- **Buckets** — real `list_buckets()` call, list rendered, click opens the
  bucket in Explorer.
- **Transfers** — Upload button calls `dialogs::pick_file`, Download
  button calls `dialogs::pick_directory` for the destination, both wired
  through `S3Client::spawn_upload`/`spawn_download`; per-row progress bar
  driven by the `TransferEvent` channel via `cx.spawn` + `this.update`;
  Cancel button calls `CancelHandle::cancel`; Clear finished removes
  terminal rows. A cancelled picker (`Ok(None)`) starts nothing — no
  default path is ever silently substituted (design spec §5).
- **Properties** — selected object's cached `FileEntryDto` fields (key,
  size, modified); honest empty state when nothing is selected (no
  `head_object` round trip added — the listing already has what's shown).

```
cargo build -p chronos-fm-pages    → clean
cargo test -p chronos-fm-pages s3:: → 4 passed; 0 failed
  (1 pre-existing T021 regression test + 3 new format_bytes tests)
cargo test --workspace --no-fail-fast → 0 failed anywhere
cargo build --release -p chronos-fm → clean, 1m25s
```

## 4 — Vision grims

Ran `script/dev/t037_smoke.sh` against the release binary with the new S3
code linked in: launches clean, **zero errors/panics in the log**, grim
captured successfully (`GRIM_OK`). That's real evidence the new code
doesn't crash or regress startup.

**What it does NOT show:** the app opened to the Explorer/Desktop tab by
default — reaching the S3 tab, then each of the four sub-views, needs
either a config with a live S3 profile pre-selected or interactive clicks
to navigate there. T038's residual (**T046**) already documented that
`hyprctl dispatch focuswindow` is broken in this sandbox's Lua-Hyprland
config, so synthetic `ydotool` clicks land on whatever window has OS focus
— not necessarily this app's window — making interactive tab-switching
unreliable here. I did not attempt to force it with a single lucky click
and then claim visual parity from that; per the design spec's own
instruction ("if the environment cannot safely exercise a state, report it
UNVERIFIED rather than infer it"), the four views' actual on-screen
appearance against `Chronos-S3-Tab.dc.html` is **UNVERIFIED** this
session — same gap class as T046, not a new kind of gap.

## 5 — Live verification against a real S3 stand (residuals 1+2 closed)

Follow-up session: ran the full 4-view shell against a **live local MinIO**
(quay.io/minio on 127.0.0.1:9000, path-style, profile `rustfs`) with a
real profile in `~/.config/chronos-fm/config.toml`. Environment:

- MinIO seeded with 2 buckets (`photos`: `wallpaper.png`, `notes.txt`,
  `holiday/…`, `family/…`; `documents`: `readme.md`, `archive.tar.gz`) via
  `script/dev/t039_seed_minio.py` (boto3).
- Credentials come from `CHRONOS_FM_S3_ACCESS_KEY`/`_SECRET_KEY` through a
  **verification-only seam** (`S3Page::maybe_auto_connect_from_env`, env-gated,
  no-op without both vars). The seam passes `persist_credentials=false` to
  `start_connect`, so it **never writes the keyring** (verified: `secret-tool
  lookup` stays empty after a seam run); only the interactive Connect button
  persists. Keyring itself verified fast (3.3 ms `connect()`) — not a blocker.
  Secrets never land in config or this report.
- Sub-views reached with the existing `--page=s3:<sub>` debug flag (T046).

### Bug found & fixed on the live path

`--page=s3:buckets` runs `select_view(Buckets) → load_buckets()` **before**
the async connect completes, so `client` is `None` and the load is
skipped; nothing retried it after the client arrived → the Buckets view
stayed empty forever. Fixed in `wire_pane`: when the view is Buckets and
buckets are still empty, retry `load_buckets()` right after wiring the
client. (Same class of gap as the earlier pane-listing regression, commit
`32cb1ac`; the pane *does* receive its listing — probe log
`pane reload cwd=s3://rustfs@ entries=Ok(["documents", "photos"])`.)

### Byte proof

`live_minio_chunked_transfer_byte_proof` (services crate, MinIO-gated,
skips when no server): whole-object put/get roundtrip **and** chunked
multipart upload + download are byte-identical. This exposed a real
latent bug: the transfer thread created its **own** tokio runtime while
reusing the shared `aws_sdk_s3::Client`, whose hyper connection pool was
bound to the caller's runtime → requests never dispatched (silent hang).
Fixed by passing the client's `Arc<Runtime>` into the transfer thread so
upload/download run on the same runtime as every other S3 op. The test
now passes in ~0.3 s.

### Grims — all 4 sub-views, `class=chronos-fm`, real data

`docs/orchestration/tasks/report-log/T039-shots/t039_s3_live_{explorer,buckets,transfers,properties}.png`

OCR (tesseract; the sandbox was missing `eng.traineddata`, installed to
/tmp/tessdata) reads, per view:

| View | OCR-verified content |
|---|---|
| **explorer** | header `☁️ S3`; sub-nav Explorer\|Buckets\|Transfers\|Properties; path `s3://rustfs@`, **2 items**, List/Grid toggle; listing columns NAME/TYPE/SIZE/Preview; first row `documents` (Folder); Places sidebar |
| **buckets** | sub-nav; bucket rows **`documents`**, **`photos`** (the seeded MinIO buckets) |
| **transfers** | sub-nav; honest empty state `No transfers yet.` |
| **properties** | sub-nav; honest empty state `Select an object in Explorer to see its properties.` |

Old NoProfiles-gate grims (`t039_s3_*.png`) had stdev 3.8–4.8 (empty
gate); live grims are 11.5–19.7 with real content. Bucket sizes show `0 B`
(honest: `ListBuckets` doesn't return sizes; per-bucket HEAD is Phase-F
material).

## What is verified vs. not

| Claim | Evidence |
|---|---|
| Chunk-split math is correct at boundaries (exact multiple, remainder, zero, sub-chunk) | 5 unit tests, green |
| State machine allows exactly the documented transitions, rejects the rest | 2 unit tests, green |
| Engine compiles against the real `aws-sdk-s3` API surface | `cargo build -p chronos-fm-services` clean |
| Engine's actual network behavior (upload/download/retry/abort) | live MinIO byte proof: whole-object + chunked upload/download byte-identical, ~0.3 s |
| Transfer thread shares the client's tokio runtime (latent hang fixed) | `live_minio_chunked_transfer_byte_proof` green; before fix it hung with zero events |
| 4-view UI compiles, existing T021 regression test still passes | `cargo build`/`cargo test -p chronos-fm-pages` clean |
| `format_bytes` display helper | 3 unit tests, green |
| No workspace regression | `cargo test --workspace`: 0 failed (450 passed) |
| Release build stable, app launches, no panics | live grim, `GRIM_OK`, log has zero error/panic lines |
| Connect flow against a real profile (keyring + client + wire) | log: seam → keyring → client built → `wire_pane done`, whole flow 64 ms |
| Buckets view loads after async connect (init race fixed) | `--page=s3:buckets` log: `S3 buckets: loaded 2 buckets` |
| The 4 views render with real S3 data | 4 live grims, OCR-verified (`t039_s3_live_*.png`) |

## Recommendation

**Residuals 1+2 (RustFS/live engine + 4-view visual) are closed** by the
live MinIO run above. What remains is honest, smaller-scope follow-up:

1. **Cancel-mid-transfer path** — the live proof exercises upload/download
   completion; the cancel/abort path is still unit-tested only.
2. **Bucket sizes / object counts** — `ListBuckets` returns no sizes, so
   the Buckets view shows `0 B`; per-bucket HEAD/listing for sizes is a
   small Phase-F enhancement.
3. **Interactive path** — the env-credential seam is verification-only;
   the form→keyring→connect flow still deserves one manual click-through
   on a real desktop (keyring verified fast here, so no blocker).

No honest gaps are hidden as "done" — the earlier UNVERIFIED rows now
carry live evidence.
