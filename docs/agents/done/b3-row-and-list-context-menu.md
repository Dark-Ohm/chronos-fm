# Task b3-row-and-list-context-menu

**Status:** active
**Plan ref:** `docs/superpowers/plans/2026-07-21-explorer-context-menu.md`, Tasks 5, 6, and the list.rs half of Task 7
**Repo root:** `/home/neo/projects/chronos-ecosystem/Chronos-FM`
**Depends on:** b1-clipboard-rename-foundation, b2-file-ops (both must be merged first — this task calls `clipboard::current`, `page.copy_selection`/`cut_selection`/`delete_paths`/`new_folder`/`paste_clipboard`, `page.begin_rename`/`commit_rename`/`cancel_rename`, `page.renaming`). **Parallel:** b4-grid-context-menu runs at the same time as this task — b4 only touches `grid.rs`, this task only touches `row.rs`/`list.rs`/`listing.rs`. **Race rule (binding):** do not touch `grid.rs` under any circumstance, even to fix an apparent inconsistency — flag it in your report instead.

## Report (MANDATORY)

Write to the **inbox** (not archive):

- **Inbox:** `docs/agents/report/b3-row-and-list-context-menu-report.md`
- **Accepted report (archive):** `docs/agents/report-log/b3-row-and-list-context-menu-report.md`

Report format:

```markdown
# b3-row-and-list-context-menu — report

## Outcome
PASS | FAIL | BLOCKED

## What changed
- paths created/modified

## Verification (commands + observed output)
```
paste cargo build / cargo test output
```

## Manual smoke (list view)
- right-click a row → menu appears with Rename/Copy/Cut/Copy Path/Delete: yes/no
- right-click an unselected row while another is selected → selection jumps to it: yes/no
- multi-select then right-click one of them → Rename shows disabled, selection stays multi: yes/no
- Delete → confirm dialog → OK moves file to trash, row disappears: yes/no
- inline rename: Enter commits, Escape cancels (verified on disk): yes/no
- right-click empty space below the rows → New Folder / Paste / Refresh menu appears: yes/no

## Risks / follow-ups
```

After orchestrator accept: report → `docs/agents/report-log/`, this brief → `docs/agents/done/` (or `docs/agents/rejected/` if declined).

## Goal

Wire the actual right-click context menu into list view: per-row menu (Open behavior stays as-is; Rename/Copy/Cut/Copy Path/Delete), right-click selection normalization, cut-row dimming, inline rename rendering (swap the filename for an editable `Input` when `page.renaming` matches the row), and the empty-area menu (New Folder/Paste/Refresh) on the listing's background.

Read the plan file's Tasks 5, 6, and 7 in full — they contain the exact, already-verified `gpui-component` API calls (`ContextMenuExt`, `PopupMenuItem`, `AlertDialog` via `window.open_alert_dialog`, `DialogButtonProps`, `ButtonVariant::Danger`) and the exact before/after code for every edit, including the full final version of `render_table_with_header` in Task 7's step 1 (do not improvise your own version of that function — copy the plan's, it already accounts for a subtlety in reusing the `entity` handle instead of calling `cx.entity()` twice).

## Deliverables (touch only these)

| Path | Role |
|---|---|
| `crates/chronos-fm-pages/src/explorer/view/listing/row.rs` | context menu + right-click selection + cut dimming + inline rename render |
| `crates/chronos-fm-pages/src/explorer/view/listing/list.rs` | thread `window` through to `row::render`; empty-area context menu on `render_table_with_header`'s container |
| `crates/chronos-fm-pages/src/explorer/view/listing.rs` | pass `window` into `list::render` |

Do not touch `grid.rs` (b4 owns it), `clipboard.rs`, `rename.rs`, `file_ops.rs`, `state.rs` (b1/b2 own them, already merged — read-only for you).

## Verification (required)

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
cargo build -p chronos-fm-pages
cargo build -p chronos-fm
cargo test -p chronos-fm-pages
```

All must PASS/build clean. Then run the manual smoke checklist above with `cargo run -p chronos-fm` — there is no GUI screenshot harness for this crate, so this step is a manual claim; report it honestly (which checks you actually ran vs. which you couldn't).

## Constraints

- Any element wrapped in `.context_menu(...)` must have an explicit `.id(...)` first (plan's Global Constraints explain why — a pointer-address fallback id is not guaranteed stable across renders).
- `PopupMenuItem::on_click` handlers only get `&mut App`, not `&mut Context<ExplorerPane>` — reach the pane via `let pane = cx.entity();` captured outside the menu closure, then `pane.update(cx, |pane, cx| { ... })` inside the handler, exactly as the plan shows.
- Every new `pub` item needs a doc comment.

## Out of scope

Grid view (b4), clipboard/rename/file_ops logic itself (b1/b2, already done).
