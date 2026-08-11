# T058 — Synchronous recursive `$HOME` watcher blocked window startup report

## Architect review (2026-08-11)

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED**
>
> Independently re-verified (not relying on the report's screenshot):
> - `grep` — the only `SearchService::new` call in the app sits inside
>   `spawn_search_service`'s `background_spawn` (app.rs:161); the window is
>   built with `None`.
> - `cargo test -p chronos-fm-services search::` → 9 passed, 0 failed.
> - `cargo test -p chronos-fm-pages search_service_arrives_after_window_opens`
>   → 1 passed.
> - `T058-window.png` shows a real `$HOME` tree (119 items, `.ssh`,
>   `.gnupg`, `Projects`…), not an empty window.
> - Own live run: window `mapped: true` in `hyprctl clients` at **538 ms**
>   (was ~35 s); the engine line `Index already has 151805 documents`
>   arrives +5.7 s after the GPU-adapter line, with the window already open.
>
> Root cause eliminated substantively, not cosmetically: the synchronous
> `SearchService::new` is off the window-building thread; T016/T033 behavior
> preserved (`watcher.rs` untouched).

**Status:** ACCEPTED / CLOSED (2026-08-11). Executor closes the ticket
(`active/` → `done/`) and moves this report to `report-log/` after the
architect's ACCEPT stamp.

## Outcome

`SearchService::new` (index open + recursive inotify watcher over `$HOME`,
measured ~35 s on this machine's 303k-directory home) was moved off the
window-building thread. The window now opens immediately with
`search_service: None` and the finished service is injected into `RootView`
when background initialization completes. Live release run on this machine:
**window mapped ~0.5–0.8 s** after process start (was tens of seconds), the
search engine arrives asynchronously ~4.5–5.6 s later in the background,
and the search UI honestly reports "search starting" during that window
instead of presenting a working-looking search that silently finds nothing.

Root cause (as established in the ticket): `SearchService::new` ran
synchronously inside `app.open_window` before `RootView::new`; it creates a
`FileWatcher` whose `RecursiveMode::Recursive` watch on Linux synchronously
walks every directory of `$HOME` (303 565 dirs) before returning. The
initial-indexing job was already deferred to `cx.background_spawn`; the
watcher setup — the more expensive half — was not.

## Changes (working tree, uncommitted — pending acceptance)

| File | Change |
|------|--------|
| `crates/chronos-fm/src/app.rs` | `SearchService::new` moved into a background task (`App::spawn` + `AsyncApp::background_spawn`); window opens with `None`; finished service injected via `WeakEntity::update_in` → `RootView::set_search_service`; initial-indexing job taken in the same background task and run on its own `background_spawn`, gated on successful injection |
| `crates/chronos-fm-pages/src/root.rs` | `RootView::set_search_service` — stores the service, pushes it into the explorer page, (re)starts the indexing-progress poll on a None→Some transition only |
| `crates/chronos-fm-pages/src/explorer/page.rs` | Shared `SearchServiceState { service, init_reported }` slot read by the pane factory so tabs opened *after* injection inherit the service; `ExplorerPage::set_search_service` flips the slot + every live pane |
| `crates/chronos-fm-pages/src/explorer/state.rs` | `ExplorerPane::search_initializing` field — honest "starting" vs "failed" state |
| `crates/chronos-fm-pages/src/explorer/view/listing/search_bar.rs` | Availability banner: "⏳ Full-text search starting — indexing $HOME in the background" (muted) while init is pending; the pre-existing "⚠ … unavailable — index failed to load" (danger) only after init has reported back |
| `crates/chronos-fm-services/src/search/engine.rs` | `SearchEngine::new` delegates to new `new_with_home(home, excludes)` (mirrors `IndexManager::new_with_path` precedent) so tests build against a tempdir, never the real `$HOME` |
| `crates/chronos-fm-services/src/search.rs` | `SearchService::new_with_home` |

T016/T033 behavior is untouched: `watcher.rs` was not modified; the
recursive fast-path + per-directory fallback + self-index callback filter
still run, now on a background thread.

## Claims and evidence

Claim: The window no longer blocks on search-service construction — it opens
with `search_service: None` and the service is injected asynchronously when
background initialization finishes, without losing the initial-indexing job
or the progress loop.

Evidence: `app.rs` (`spawn_search_service`) creates the service inside
`cx.background_spawn` (via `App::spawn`/`AsyncApp`), takes the one-shot
`InitialIndexingJob` in the same task, injects via
`view_weak.update_in(&mut *cx, …)` → `RootView::set_search_service`, and
only then runs `job.run()` on its own `background_spawn` (gated on
injection success — otherwise the progress channel has no consumer).
`RootView::set_search_service` restarts `start_progress_loop` on None→Some
(guarded so a second call cannot double the per-frame poll). Live: window
mapped 511 ms / 777 ms across two release runs (below), and the log shows
the engine line (`Index already has 151805 documents, skipping initial
indexing`) arriving asynchronously at +4.5 s / +5.6 s after the GPU-adapter
line — i.e. watcher setup ran in the background, after the window was
already interactive.

Truth base: Chronos-FM | runtime.

Claim: Tabs opened after injection inherit the search service (the pane
factory must not snapshot `None` at page construction — otherwise a split or
new tab created during/after background init would silently lack search).

Evidence: `ExplorerPage::new` now stores the service in a shared
`Rc<RefCell<SearchServiceState>>`; the `PaneGroup` factory reads
`state.service` / `state.init_reported` for every pane it builds.
`search_service_arrives_after_window_opens` (`explorer/tests.rs`) builds the
page with `None`, injects a real service (tempdir home), asserts the
existing pane gets it, then `test_new_tab` and asserts the new tab has it
too. `cargo test -p chronos-fm-pages search_service_arrives_after_window_opens`
→ exit 0, 1 passed.

Truth base: Chronos-FM.

Claim: The search UI is honest during background init — it never looks like
a working search that silently finds nothing.

Evidence: the search bar renders an explicit banner whenever
`search_service` is `None`; the message and color now depend on
`search_initializing`: muted "⏳ Full-text search starting…" while the
background task has not reported, danger "⚠ … unavailable — index failed to
load" (the T016 text) once it has. `ExplorerPane::search_initializing`
starts false and is set true by the explorer factory only while
`service.is_none() && !init_reported` (auxiliary panes built via
`ExplorerPane::build(None, …)` — S3 listings, drop targets — keep the
pre-T058 "failed" wording, no semantic drift). `trigger_search`'s existing
degraded path (filename filtering + "Search index unavailable" status) is
unchanged and still covers the init window.

Truth base: Chronos-FM.

Claim: T058's mandatory structural test — `FileWatcher::new` is not invoked
by the window-construction path (verified structurally, not with a timer).

Evidence: the window-building path (`RootView::new` → `ExplorerPage::new` →
`ExplorerPane::build`) only ever receives an `Option<Arc<SearchService>>`
and never calls `SearchService::new`/`FileWatcher::new`; the only call site
of `SearchService::new` in the app is inside `spawn_search_service`'s
`background_spawn`. `search_service_arrives_after_window_opens` builds the
whole `ExplorerPage` with no service at all and asserts the panes start
honestly (`is_none` + `search_initializing`) — the window construction path
completes without touching the watcher. `engine_builds_against_explicit_home`
(`search/engine.rs`) additionally proves engine construction (incl. the
recursive watcher) completes quickly and cleanly against a small tempdir
home. No timing asserts anywhere.

Truth base: Chronos-FM.

Claim: T016 (surviving unreadable subdirs) and T033 (self-index feedback
filter) behavior is preserved and green after the move to background.

Evidence: `watcher.rs` untouched; `cargo test -p chronos-fm-services search::`
→ exit 0, 9 passed (4 exclusions + 1 new engine + 2 indexer + 2 watcher,
incl. `watcher_survives_unreadable_subdir` and
`watcher_callback_filters_self_index_paths`).

Truth base: Chronos-FM.

## Verification commands and results

All commands run from `/home/neo/projects/chronos-ecosystem/Chronos-FM`
(main branch, working tree).

```
cargo test -p chronos-fm-services search::
  -> exit 0, 9 passed, 0 failed   (incl. T016/T033 watcher tests + new engine smoke test)

cargo test -p chronos-fm-pages search_service_arrives_after_window_opens
  -> exit 0, 1 passed, 0 failed   (finished in 0.03–0.05 s — tempdir home, no timing)

cargo test --workspace
  -> exit 0, 0 failed across all crates
     (13 chronos-fm-core + 62 chronos-fm-services + 180 chronos-fm-pages
      + 144 chronos-fm-store + 16 chronos-fm + 34 chronos-fm-ui, 0 doc-tests)

cargo build --release -p chronos-fm
  -> exit 0, target/release/chronos-fm produced

cargo clippy -p chronos-fm-services -p chronos-fm-pages -p chronos-fm
  -> no new warnings in changed files (pre-existing s3.rs/theme warnings only)
```

## Live release evidence

Compositor: Hyprland (same machine, same `$HOME` with 300k+ directories).
Binary: `target/release/chronos-fm` built above, launched with
`RUST_LOG=info`, log to `/tmp/t058-run.log`.

Run 1 (before review fixes): window in `hyprctl clients` with
`mapped: 1`, `visible: 1`, class `chronos-fm` at **509 ms** after spawn.
Log: `Selected GPU adapter … (Vulkan)` at `07:27:11.719`, engine
`Index already has 151805 documents, skipping initial indexing` at
`07:27:16.568` (+4.8 s, asynchronous). No multi-second silence on the
window path.

Run 2 (final binary after review fixes): **WINDOW mapped after 777 ms**
(`mapped: 1`), GPU at `07:34:31.019`, engine line at `07:34:36.613`
(+5.6 s). Grim capture of the window region (geometry
`1141,42 1118x1388`, class `chronos-fm`): `docs/orchestration/tasks/report-log/T058-window.png`
(105 KB, 6572 distinct colors, dark theme — real rendered window, not
blank).

Acceptance criteria from the ticket, checked:

- Time from process start to window on `hyprctl clients`: **~0.5–0.8 s**,
  not tens of seconds. Yes.
- No multi-second silence between `Selected GPU adapter` and first sign of
  window readiness caused by watcher-setup; watcher/tantivy lines appear
  later, asynchronously. Yes (GPU → window mapped in <1 s; engine line at
  +4.5–5.6 s).
- `cargo test --workspace` green; new + old watcher tests visible in the
  counter. Yes (0 failed; 9 search tests incl. both watcher tests).

## Non-negotiables checklist (from the ticket)

- Watcher setup off the window-building thread: `SearchService::new` runs
  in `cx.background_spawn` (mirroring `InitialIndexingJob::run`). Yes.
- `search_service` in `RootView` appears asynchronously: window opens with
  `None`, injected via `set_search_service` (channel/callback into the
  Context, `cx.notify()`). Yes.
- T016 fallback (per-directory watch with `excludes`) preserved, executed
  in the background. Yes — `watcher.rs` unchanged.
- No regression from the move to background: UI shows "search starting"
  until the watcher is up; never a working-looking silent search. Yes —
  banner + degraded search path.
- Mandatory tests: structural test that `FileWatcher::new` is not called in
  the window constructor (pages test, no timers), T016/T033 green, workspace
  green. Yes.
- No `git push`; executor does not self-ACCEPT; ticket stays in `active/`.
  Yes.

## Residual / handed off

- The "search starting" banner is verified at the state level (unit test
  asserts `search_initializing` transitions); the visual banner render is
  the direct `service.is_none()` / `search_initializing` mapping in
  `search_bar.rs`. If the Architect wants a live grim with the search bar
  open during the init window (needs input to toggle search), that is a
  small additional capture, not a code gap.
- Changes are staged in the working tree on `main` (not committed); commits
  and the ticket move to `done/` await Architect acceptance, per
  orchestration hygiene.
