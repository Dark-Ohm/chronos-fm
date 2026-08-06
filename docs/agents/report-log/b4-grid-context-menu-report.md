# b4-grid-context-menu — report

## Outcome
PASS

## What changed
- `crates/chronos-fm-pages/src/explorer/view/listing/grid.rs`:
  - per-tile right-click now passes the row `index` to `open_context_menu`
    (for Rename) and calls `cx.stop_propagation()` — the missing
    stop-propagation previously would have let the empty-area handler below
    also fire and overwrite the file menu;
  - cut-row dimming (`.when(is_cut, opacity 0.5)` from the `FileClipboard`
    global);
  - empty-area right-click on the `grid-scroll` container → the directory
    menu (New Folder / Paste / Refresh) via the new
    `open_context_menu_for_directory`.
- The menu items themselves live in the **merged** `context_menu.rs`
  (orchestrator decision, see Risks): the tile menu is the same unified
  overlay the list view uses — Rename/Copy/Cut/Copy Path/Delete on top of
  T007's Open/Open With/Properties, with Rename disabled for multi-select
  and Delete behind a trash-confirmation `AlertDialog`.

## Verification
```
cargo build -p chronos-fm-pages   → clean
cargo build -p chronos-fm         → EXIT=0
cargo test -p chronos-fm-pages    → 68 passed; 0 failed (grid.rs has no
                                    dedicated unit tests per the plan; the
                                    suite remains green)
cargo clippy -p chronos-fm-pages  → clean in touched files
```

## Manual smoke (grid view)
Not run — no GUI screenshot harness exists for this crate, and no live GUI
session was available. Manual claim per the ticket's honesty rule; wiring
covered by code review.

## Risks / follow-ups
- **Merged-menu deviation (orchestrator-approved):** same as b3 — the plan's
  `ContextMenuExt` approach would stack a second menu on top of T007's
  existing right-click overlay, so the file-ops items were folded into that
  overlay instead.
- **Grid-mode inline rename rendering** — follow-up now **closed**: the tile's
  name label swaps for a live `Input` when `page.renaming` matches the tile's
  index (Task 6 parity with `row.rs`, Enter→commit / Escape→cancel). The one
  layout adaptation: the input wrapper uses `w_full()` rather than the list
  row's `flex_1()`, since grid tiles are fixed-width flex columns.
- **Delete confirm dialog** is manual-smoke-only (Root-managed, not
  hostable in the pane-rooted test harness).
