# T039 — S3 tab mockup parity — Progress report (gates 1 + 2)

> ## ⚖️ ARCHITECT VERDICT (2026-08-10): **GATES 1+2 ACCEPT — IMPLEMENT GO**
>
> ### Gate 1 (ashpd) — **ACCEPT**
> Spot-checked `dialogs.rs`: `pick_file` / `pick_directory` via portal
> FileChooser; cancel → `None` honest; `uri_to_path` file:// only +
> percent-decode; rejects non-file schemes. Direct ashpd pin matches Source
> style; Cargo.lock ashpd/zbus counts = 1 (no dup graph). Architect re-ran
> `cargo test -p chronos-fm-services dialogs::` → **3/3**. Unwired to UI is
> correct for this gate.
>
> ### Gate 2 (design spec) — **DESIGN APPROVED**
> Spec `docs/superpowers/specs/2026-08-10-s3-tab-mockup-parity-design.md`
> is sound for Phase F transfer subsystem + Phase V shell:
> - sequential 8 MiB chunked upload/download — OK v1
> - state machine without Paused — OK (mockup)
> - cancel between chunks; leave partial download — OK
> - retry 1× transient only, never 4xx — OK
> - error matrix (6 cases) — OK
> - `S3View` 4 views mirroring T038 pattern — OK
> - whole-object provider path preserved (T011/T021) — **hard wall**
> - secrets discipline in spec — OK
>
> Optional later revisits (not blocking GO): parallel parts, pause/resume.
>
> ### Gate 3 — ongoing discipline (not a one-shot)
> No secrets in artifacts/grim/report. Continue through impl.
>
> ### Not ACCEPT for full T039
> Implementation not started — correct stop after T038 vision lesson.
> Next executor: implement per approved design (TDD chunk split + state
> machine first), then vision grims (hypr focus residual same as T046).
>
> Ticket stays **active**. Ship gate code + spec as commit.


**Status:** GATES RESOLVED, IMPLEMENTATION NOT STARTED — not accepted,
not claimed done. Per session policy the executor does not accept its own
work.
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5)

## Summary

The architect's 2026-08-09 verdict on T039 required three gates before
heavy implementation: (1) compile-probe `ashpd` FileChooser, (2) a written
design spec (like T038's) with job state machine + error matrix, (3)
secrets never in report/grim. This session resolved (1) with real code +
tests, wrote (2) as
`docs/superpowers/specs/2026-08-10-s3-tab-mockup-parity-design.md`, and
(3) is a discipline rule threaded through that document rather than a
one-time check — nothing here touches credentials.

**Full T039 implementation (chunked transfer engine, job state machine,
4-view UI shell) is genuinely out of scope for this pass** — it's
comparable in size to T038, which was its own full session. Rather than
rush a large, unverified implementation right after T038's visual-proof
gap (T046), this pass stopped at "gates resolved, spec ready for review"
so the actual build can start from an approved design instead of guessing
at scope mid-implementation.

## Gate 1 — ashpd compile probe: done, with code

`crates/chronos-fm-services/src/dialogs.rs` (new file, ~70 lines):
`pick_file`/`pick_directory` wrapping `ashpd::desktop::file_chooser::
SelectedFiles::open_file()`, `uri_to_path` for the `file://`-URI → local
path conversion.

- Added `ashpd = { version = "0.13", default-features = false, features =
  ["async-io", "file_chooser"] }` + `percent-encoding = "2"` directly to
  `chronos-fm-services/Cargo.toml`, matching Source's own workspace pin.
- **Confirmed no duplicate dependency graph:** `grep -c '^name = "zbus"'
  Cargo.lock` and `'^name = "ashpd"'` both return `1` — this resolves to
  the *same* already-locked instance pulled in transitively via
  `gpui_linux`/`oo7`, not a second copy (the exact risk the original T039
  report flagged as an open sub-decision).
- `ashpd::Uri` turned out to be a thin string wrapper (not `url::Url` as
  I first assumed) — no built-in path-conversion helper, so `uri_to_path`
  hand-parses the `file://` prefix and percent-decodes via
  `percent-encoding` (already transitively present, confirmed before
  adding it directly). 3 unit tests: accepts `file://`, rejects a non-file
  scheme (`mtp://`, honestly — no silent mangling into a bogus local
  path), percent-decode round-trip (`%20` → space).

```
cargo test -p chronos-fm-services dialogs::
test result: ok. 3 passed; 0 failed
```

**Not wired up:** nothing calls `pick_file`/`pick_directory` yet — that's
Transfers-view implementation, not this gate.

## Gate 2 — design spec: written, not yet reviewed/approved

`docs/superpowers/specs/2026-08-10-s3-tab-mockup-parity-design.md`. Key
decisions, each with a stated reason (not just an assertion):

- **Chunked engine:** sequential (not parallel) 8 MiB multipart parts for
  upload, ranged `GetObject` windows for download — same chunk size both
  directions for symmetric progress. Sequential-not-parallel chosen for
  simpler cancellation/progress semantics in v1, explicitly flagged as
  revisitable if throughput proves insufficient.
- **Job state machine:** `Queued → InProgress → {Completed | Failed |
  Cancelled}`, no `Paused` state — the mockup doesn't show a pause
  control, and pause/resume needs part-ETag persistence across restarts,
  which is real scope the mockup doesn't ask for.
- **Cancellation:** cooperative, checked between chunks (not mid-chunk —
  an S3 part is atomic). Cancel-during-download leaves the partial local
  file in place (matches ordinary browser-download behavior) rather than
  deleting it.
- **Retry:** one automatic retry per chunk on a transient error only
  (never on 4xx — retrying an auth/permissions failure just delays a real
  error, doesn't fix it).
- **Error matrix:** six concrete failure shapes with their exact
  user-facing message templates (local file vanished, network timeout
  post-retry, 403, 404, disk full, multipart-abort-itself-fails-during-
  cancel — the last one intentionally logged, not double-surfaced, since
  the job is already Cancelled from the user's perspective).
- **Views:** `S3View::{Explorer, Buckets, Transfers, Properties}` — same
  shape as T038's `GitView`, reusing a pattern instead of inventing a new
  one.

## What is verified vs. not

| Claim | Evidence |
|---|---|
| ashpd compiles against the real dependency graph, no duplication | `cargo build -p chronos-fm-services` clean; `grep -c` on Cargo.lock |
| `dialogs.rs` conversion logic is correct | 3 unit tests, green |
| No regression in the rest of the workspace | `cargo test --workspace --no-fail-fast`: 0 failed |
| Release build still clean | `cargo build --release -p chronos-fm`: clean |
| Design spec is architecturally sound | **NOT verified by anyone but me** — this needs the same review T038's spec got before implementation started, not a rubber stamp |
| Chunked engine / job state machine / 4-view UI | **NOT IMPLEMENTED** — this report doesn't claim otherwise anywhere |

## Recommendation

Review the design spec (same process as T038's 2026-08-09 spec review)
before I or anyone starts the chunked-engine implementation — several of
its calls (sequential-not-parallel parts, no-pause-in-v1, single-job-per-
profile-in-flight) are real scope decisions a reviewer might want to
push back on before code gets written around them.
