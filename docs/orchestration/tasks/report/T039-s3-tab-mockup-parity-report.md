# T039 — S3 tab mockup parity — Implementation report

> ## ⚖️ ARCHITECT UPDATE (2026-08-10): **PARTIAL-ACCEPT** (unchanged gates)
>
> T046 closed with empty-state grim only (`t046_s3.png` NoProfiles).
> Still open:
> 1. Sub-view grims: `s3:explorer|buckets|transfers|properties` (honest empty OK)
> 2. RustFS integration (real transfer bytes) — Phase F / residual
>
> Code ship stands; not full-close.

**Status:** IMPLEMENTED, NOT ACCEPTED — not claimed done. Per session policy
the executor does not accept its own work; every claim below is either a
command + its output or explicitly marked UNVERIFIED.
**Date:** 2026-08-10
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

## What is verified vs. not

| Claim | Evidence |
|---|---|
| Chunk-split math is correct at boundaries (exact multiple, remainder, zero, sub-chunk) | 5 unit tests, green |
| State machine allows exactly the documented transitions, rejects the rest | 2 unit tests, green |
| Engine compiles against the real `aws-sdk-s3` API surface | `cargo build -p chronos-fm-services` clean |
| Engine's actual network behavior (upload/download/retry/abort) | **NOT VERIFIED** — no RustFS run this session |
| 4-view UI compiles, existing T021 regression test still passes | `cargo build`/`cargo test -p chronos-fm-pages` clean |
| `format_bytes` display helper | 3 unit tests, green |
| No workspace regression | `cargo test --workspace --no-fail-fast`: 0 failed |
| Release build stable, app launches, no panics | live grim, `GRIM_OK`, log has zero error/panic lines |
| The 4 views visually match the mockup | **UNVERIFIED** — same hypr click-focus blocker as T046 |

## Recommendation

Two real residuals, not blocking on each other:

1. **RustFS integration pass** (design spec verification item 2) — actually
   upload and download a file against the local RustFS stand, byte-for-
   byte check, and exercise the cancel-mid-transfer path for real. This is
   the only way to know the engine works, not just compiles.
2. **Visual verification** — needs either working interactive input in
   this sandbox (T046's open question) or a config with a pre-selected S3
   profile + some way to force the initial view to Buckets/Transfers/
   Properties for a scripted grim sequence without relying on clicks.

Both are honest gaps, not implementation debt hidden as "done."
