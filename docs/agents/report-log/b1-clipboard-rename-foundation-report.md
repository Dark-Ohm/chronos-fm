# b1-clipboard-rename-foundation — report

## Outcome
PASS

## What changed
- `crates/chronos-fm-pages/src/explorer/clipboard.rs` (NEW) — `FileClipboard` global
  (`paths` + `mode`), `ClipboardMode::{Copy, Cut}`, `init`/`set_copy`/`set_cut`/`clear`/`current`,
  2 tests. Per plan `docs/superpowers/plans/2026-07-21-explorer-context-menu.md` Task 1.
- `crates/chronos-fm-pages/src/explorer/rename.rs` (NEW) — `ExplorerPane::begin_rename` /
  `commit_rename` / `cancel_rename` (inline rename via `ops::rename_in_place`, error-after-`reload()`
  ordering), 2 tests. Plan Task 3 (+ Step-4-fixed test module).
- `crates/chronos-fm-pages/src/explorer/state.rs` — added
  `pub renaming: Option<(usize, Entity<InputState>)>` field + init; `apply_filter` now clears
  `renaming` alongside the selection (stale-index hygiene, review item).
- `crates/chronos-fm-pages/src/explorer.rs` — `pub mod clipboard;`, `mod rename;`,
  `pub(crate) mod tests;` (per plan Tasks 1/3 Step 4).
- `crates/chronos-fm-pages/src/explorer/tests.rs` — new `pub(crate) new_explorer_for_tests(cx, cwd)`
  harness (roots a pane at a tempdir, registers the clipboard global).
- `crates/chronos-fm/src/app.rs` — `chronos_fm_pages::explorer::clipboard::init(app);` after
  `gpui_component::init(app);` (plan Task 2).

## Verification (commands + observed output)

```bash
$ cargo test -p chronos-fm-pages clipboard:: rename::
test explorer::clipboard::tests::set_cut_replaces_a_prior_copy ... ok
test explorer::clipboard::tests::set_copy_then_clear_round_trips ... ok
test explorer::rename::tests::begin_rename_then_commit_renames_file_on_disk ... ok
test explorer::rename::tests::cancel_rename_clears_state_without_touching_disk ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 60 filtered out; finished in 0.07s

$ cargo test -p chronos-fm-pages
test result: ok. 62 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s

$ cargo build -p chronos-fm
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.90s
```

Full workspace: `cargo test --workspace` EXIT=0 (62 pages / 38 services / 30 ui, все зелёные).

## Risks / follow-ups
- **Expected `dead_code` warning**: `begin_rename`/`commit_rename`/`cancel_rename` are "never
  used" until b3/b4 wire the context menu (plan's own sequencing: Task 4 `new_folder` calls
  `begin_rename`, Tasks 6/7 render/consume it). Not silenced — will resolve when b3/b4 land.
- Review items applied: `begin_rename` guards against replacing an in-flight rename;
  `apply_filter` clears `renaming` (stale index after reload/filter/search);
  `new_explorer_for_tests` doc notes it resets the clipboard on each call.
- b2/b3/b4 remain active; b3/b4 consume this foundation's methods.
