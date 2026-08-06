# b3-row-and-list-context-menu — report

## Outcome
PASS

## What changed
- `crates/chronos-fm-pages/src/explorer/view/listing/row.rs` — right-click
  moved to a dedicated `on_mouse_down(Right)` handler (selection
  normalization + `open_context_menu(path, ix, position)` +
  `stop_propagation`, so the empty-area handler below never stacks); cut-row
  dimming (`.when(is_cut, opacity 0.5)` reading the `FileClipboard` global);
  Task 6 inline rename rendering — the filename swaps for a real `Input`
  when `page.renaming` matches the row, with Enter→commit / Escape→cancel.
- `crates/chronos-fm-pages/src/explorer/view/listing/list.rs` — empty-area
  context menu: the table container (`listing-empty-area`) opens the
  directory menu (New Folder / Paste / Refresh) on right-click.
- `crates/chronos-fm-pages/src/explorer/context_menu.rs` — **merged**
  (orchestrator decision, see Risks): the per-file menu now includes the
  plan's Rename/Copy/Cut/Copy Path/Delete items on top of T007's Open/Open
  With/Properties, in one overlay. Rename is disabled when
  `selection.len() > 1` (multi-select awareness). Delete opens the
  trash-confirmation `AlertDialog` (`window.open_alert_dialog`, Danger
  variant) and trashes `selected_paths()`.
- `crates/chronos-fm-pages/src/explorer/navigation.rs` —
  `open_context_menu` gained the row `index` (for Rename) and snaps
  `single_selected`; new `open_context_menu_for_directory` for empty space.
- `crates/chronos-fm-pages/src/explorer/tests.rs` — `new_explorer`/
  `new_explorer_page`/`new_explorer_page_with_store` now register the
  `FileClipboard` global (row renderers consult it for cut dimming; the app
  registers it at startup).

## Verification
```
cargo build -p chronos-fm-pages   → clean
cargo build -p chronos-fm         → EXIT=0
cargo test -p chronos-fm-pages    → 68 passed; 0 failed
cargo clippy -p chronos-fm-pages  → clean in touched files
```

## Manual smoke (list view)
Not run — no GUI screenshot harness exists for this crate, and no live GUI
session was available in this environment. This is a manual claim per the
ticket's honesty rule; the wiring is covered by code review + the harness
constraints below.

## Risks / follow-ups
- **Merged-menu deviation (orchestrator-approved):** the plan specified
  gpui-component `ContextMenuExt` popups, but T007's custom "Open With"
  overlay already owned right-click in `row.rs`/`grid.rs` — stacking a second
  menu on the same event would render two menus. Per orchestrator decision,
  the b3/b4 items were folded into the existing overlay instead of adding a
  parallel mechanism.
- **Delete confirm dialog is manual-smoke-only:** `window.open_alert_dialog`
  routes through `gpui_component::Root`, which the pane-rooted test windows
  don't have. Production wraps its window root in `Root::new`, so the flow
  works in-app; it is not unit-testable in the current harness.
- **Test-harness trap (documented):** painting a live `Input` (inline rename
  or visible search bar) across an `update` boundary panics in `Root::read`
  because the test windows are not Root-wrapped. Tests that start a rename
  must clear it in the same closure; this is now documented on
  `new_explorer_for_tests` so future tests don't hit it silently.
