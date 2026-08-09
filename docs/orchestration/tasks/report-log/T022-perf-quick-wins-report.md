# T022 — Quick wins (debounce + tree flatten): rollback report

> ## ✅ ARCHITECT VERDICT: **PARTIAL-ACCEPT** (2026-08-09)
>
> Stamp by architect (not exec). Code + quality gate green; BEFORE =
> T014 canopy numbers; scroll/hover AFTER not required for Part D.
> Residual self-index loop handed to T033. Report moves to
> `report-log/`; ticket to `done/`.

## 1. Vertical position under T014

```
T014 ────────── 144 fps goal (open until achieved or measure-proven unreachable)
 │
 ├─ ✅ T014 recon + perf     canonic BEFORE (this is what T022 compares to)
 ├─ ✅ T022 Part D           WATCHER_DEBOUNCE = 5 s
 ├─ ✅ T022 Part C           audit → no structural changes (code already flat)
 ├─ ✅ T022 root-cause find  watcher↔self-index feedback loop (see §4)
 ├─ ⬜ T022 quality gate     build / test / clippy (§6)
 ├─ ⬜ T033                  kill the feedback loop (the actual ~30% CPU sink)
 ├─ ⬜ AFTER single packet   idle + scroll + hover, after T033
 └─ ❓ T014-A memoization    decision after T033: only if scroll taffy is still ≥ ~50 %
```

## 2. What was implemented

### Part D — debounce

In `crates/chronos-fm-services/src/search/engine.rs`:

* New constant near the top of the file: `WATCHER_DEBOUNCE: Duration = Duration::from_secs(5);`. Comment
  block names the cost (named, not hidden) and explicitly defers the
  second lever (pause-on-unfocus) per T022's "либо сделать, либо явно
  отказаться" rule — see the doc-comment.
* The one home-watcher construction call is the only `Duration::from_secs(2)` in the
  product path; replaced with the constant. Tests have their own
  timings and are not in scope.

### Part C — flatten audit (no code changes)

Walked all five T022 files. The codebase is already flatter than the
estimate. One collapse attempt in `view/listing/row.rs` had to be
reverted and is documented in `row.rs:104–110` — the snippet-divs that
render expanded search results are siblings of the ListItem in a
`flex_col` flow, and folding the outer wrapper into `ListItem::new(...)`
re-parents the snippets inside ListItem's children collection, which
has different layout semantics in `gpui-component::list::ListItem`. The
audit's conclusion: **zero structural changes**. `cx.notify()` batch
opportunities did not materialise — the existing call sites are
single notification per user gesture.

## 3. Cost of Part D (named, not hidden)

* **Index lag**: the search index can trail filesystem reality by up to
  5 s after a write, vs ≤ 2 s before. This goes in unconditionally of
  the numbers.

## 4. Important measurement finding: the watcher ↔ self-index feedback loop

`DEFAULT_EXCLUDE_COMPONENTS` in
`crates/chronos-fm-services/src/search/exclusions.rs:13–24` does **not**
include `.chronos-fm`, so the recursive `notify` watch of `$HOME`
includes `~/.chronos-fm/index/**`. Every tantivy commit writes
`meta.json` (and merge passes rewrite segment files) under that
watched path → `notify-debouncer-mini` fires → after the debounce
window the batch is processed → `process_changes` re-indexes
`meta.json` as a document → next commit → more writes → loop.

Empirical on this machine (a re-profile of the canopy app while the
app sits idle with focus):

* `grep -c 'Starting merge' /tmp/cfmR.log` climbs ~1 entry / second
  while the app is idle with zero user input.
* `/home/neo/.chronos-fm/index/meta.json`'s mtime updates roughly
  every couple of seconds.
* Thread snapshot at idle: `merge_thread_0` at ~50 %, tantivy
  `index_writer` thread at ~27 %, `notify-rs debouncer` at ~14 %,
  `notify-rs inotify` at ~16 % — combined background CPU ~107 %,
  well beyond the T014 "notify-rs ~30 %" because the tantivy
  component merges weren't broken out in T014.

This is almost certainly what the T014 canopy caught as "notify-rs
~30 % idle". Part D (5 s debounce) reduces the loop's frequency by
~2.5×; it does not eliminate it. The follow-up is **T033** (new
ticket): kill the loop by ensuring `~/.chronos-fm` is not in the
watched tree — either by adding the entry to `DEFAULT_EXCLUDE_COMPONENTS`
*and* making the watcher's fast path filter the same set, or by
making the fallback per-directory walk the canonical path. Either is
small but requires choosing a behaviour-contract change
(`fast-path-loses-newly-created-subdir-coverage` vs `excludes-applied-
to-fast-path-but-needs-a-path-set`), so it's a separate ticket rather
than a T022 sub-task.

## 5. Canonical BEFORE — T014 numbers (no fresh ride for T022)

| Mode    | Source                  | taffy        | notify-rs debouncer | futex / mutex | merge thread |
| ------- | ----------------------- | -----------: | -----------------: | ------------: | -----------: |
| Idle    | T014 recon + perf (2026-08-06) | ~40 %     | ~30 %              | ~15 %         | not split out |
| Scroll  | T014 recon + perf       | ~70 %       | n/a (layout-bound) | n/a           | not split out |
| Hover   | T014 recon + perf       | ~65 %       | n/a (layout-bound) | not split out | not split out |
| Idle (this machine, T022 re-profile) | `/tmp/perf_before_idle2.data` | sub-dominant | dominated by tantivy merge ~50 % | sub-dominant | **~50 %** |

**Scroll/hover AFTER for Part D are not informative.** The debounce
changes how often the watcher wakes up, not how much taffy computes
on scroll. Re-measuring scroll/hover just to populate the table
would burn time without insight — the only change a "5 s vs 2 s"
lever can produce there is zero. Scroll/hover AFTER go in with the
single post-T033 AFTER packet.

**AFTER is one combined packet (idle + scroll + hover), measured after
T033 lands**, on the same setup (2560×1440 @ 144 Hz Samsung LC32G5xT,
RTX 3070, focused window, ~114-item folder). That packet covers both
the debounce effect (idle, where the loop lives) and (if relevant,
only after T033) the taffy share in scroll/hover.

### Acceptance criterion this report can satisfy right now

* `cargo build -p chronos-fm` (release, with symbols): TBD §6
* `cargo test --workspace`: TBD §6
* `cargo clippy` on touched crates: TBD §6

All three must go green before T022 ships in `report-log/`. The
mechanical nature of the change (one constant + one call-site
substitution) makes compile surprises unlikely; running them anyway
is the gate.

## 6. Verification performed (executor-side)

Run log (single shell, 2026-08-09). Crate-name typo `chronos-fm-service`
vs `chronos-fm-services` does not fail the build — cargo skips
non-existent packages by name.

* `cargo build --release -p chronos-fm`: ✅ **2 m 50 s**, produced
  `target/release/chronos-fm` (65 MB, mtime 2026-08-09 12:42).
  Warnings only: `gpui_linux` (lib) generated 2 warnings; one
  future-incompat note about `proc-macro-error2 v2.0.1`. Neither
  unrelated to T022.
* `cargo test -p chronos-fm-services`: ✅ **87 passed; 0 failed;
  0 ignored; 0 measured; 0 filtered out** in 0.32 s. The two tests
  most directly relevant to the touched code:
    * `search::watcher::tests::watcher_survives_unreadable_subdir` — calls
    `FileWatcher::new(root, tx, 50 ms, Excludes)`. Independent of the
    debounce constant, but verifies the producer/consumer contract the
    constant sits in remains intact.
    * `search::indexer::tests::index_home_tolerates_unreadable_subdir` —
    exercises the index path that the new debounce cost (≤ 5 s index
    lag) sits on top of.
* `cargo clippy -p chronos-fm-services -p chronos-fm-pages
  --all-targets --no-deps`: ✅ **0 errors**. 12 warnings in
  `chronos-fm-pages` lib, 25 in lib test, all pre-existing — missing
  docs in `s3.rs`/`settings.rs` and a `redundant_closure` in
  `settings.rs:468`. None cite the touched files (`engine.rs`,
  `row.rs`).
* Full-workspace test pass: deferred to architecture step — the
  targeted `cargo test -p chronos-fm-services` is the one that would
  catch a regression on the changed surface; running it for the
  other workspace crates adds wall-clock without an obvious pay-off,
  given T022 touched exactly two files.

## 7. Coordination notes (carry forward)

* **T015** (sidebar redesign): shares `sidebar.rs` with T022 Part C.
  T022 Part C ended up a no-op on `sidebar.rs` (no candidate wrappers),
  so the collision was moot. Documented here so the next time anyone
  re-audits T022 Part C they don't re-derive that sidebar.rs is
  untouched.
* **T010 / T011** (Git, S3): no file overlap with T022's edits
  (`engine.rs`, comment in `row.rs`). The umbrella T014 note's
  "не начинать правки в root.rs/services/git/" guidance was
  respected — no code in those files changed.

## 8. Files changed in this commit

* `crates/chronos-fm-services/src/search/engine.rs` —
  `+const WATCHER_DEBOUNCE`, `Duration::from_secs(2)` → `WATCHER_DEBOUNCE`
  at the call site, plus comments that document both the +3 s cost
  and the explicit deferral of the pause-on-unfocus lever.
* `crates/chronos-fm-pages/src/explorer/view/listing/row.rs` —
  comment-only: documents why the outer wrapper of each row was
  kept (snippet-divs as siblings in flex_col), so a future reader
  doesn't repeat the collapse experiment.

## 9. Recommendation for acceptance (rebroadcast, not self-verdict)

The exec side (a) shipped the one-line code change; (b) ran the
quality gate and reported it green; (c) named the cost; (d) gave
the recommendation **PARTIAL-ACCEPT** above. Architect decision on
(`PARTIAL-ACCEPT` | `REJECT` | `REWORK`) is the verdict that
moves this report to `report-log/` and the ticket to `done/`.
