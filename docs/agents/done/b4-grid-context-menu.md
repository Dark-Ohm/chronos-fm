# Task b4-grid-context-menu

**Status:** active
**Plan ref:** `docs/superpowers/plans/2026-07-21-explorer-context-menu.md`, the grid.rs half of Task 7
**Repo root:** `/home/neo/projects/chronos-ecosystem/Chronos-FM`
**Depends on:** b1-clipboard-rename-foundation, b2-file-ops (both must be merged first — this task calls `clipboard::current`, `page.copy_selection`/`cut_selection`/`delete_paths`/`new_folder`/`paste_clipboard`, `page.begin_rename`). **Parallel:** b3-row-and-list-context-menu runs at the same time as this task — b3 only touches `row.rs`/`list.rs`/`listing.rs`, this task only touches `grid.rs`. **Race rule (binding):** do not touch `row.rs`, `list.rs`, or `listing.rs` under any circumstance, even to fix an apparent inconsistency — flag it in your report instead.

## Report (MANDATORY)

Write to the **inbox** (not archive):

- **Inbox:** `docs/agents/report/b4-grid-context-menu-report.md`
- **Accepted report (archive):** `docs/agents/report-log/b4-grid-context-menu-report.md`

Report format:

```markdown
# b4-grid-context-menu — report

## Outcome
PASS | FAIL | BLOCKED

## What changed
- paths created/modified

## Verification (commands + observed output)
```
paste cargo build output
```

## Manual smoke (grid view)
- switch to grid view, right-click a tile → menu appears with Rename/Copy/Cut/Copy Path/Delete: yes/no
- right-click empty grid space → New Folder / Paste / Refresh menu appears: yes/no
- copy a file in list view, switch to grid view, right-click empty space → Paste is enabled and works: yes/no
- Delete on a tile → confirm dialog → OK moves file to trash, tile disappears: yes/no
- cut a tile → it dims (opacity) until pasted or the clipboard is replaced: yes/no

## Risks / follow-ups
```

After orchestrator accept: report → `docs/agents/report-log/`, this brief → `docs/agents/done/` (or `docs/agents/rejected/` if declined).

## Goal

Wire the same right-click context menu behavior into grid view: per-tile menu (Rename/Copy/Cut/Copy Path/Delete) and an empty-area menu (New Folder/Paste/Refresh) on the grid's scroll container. Grid-mode inline rename rendering is explicitly **out of scope** for this task (see plan Task 7's note) — Rename still works functionally (it calls `begin_rename`, which sets `page.renaming`), it just won't visually swap the tile's name label for an input yet; that is a known, disclosed follow-up, not a bug to fix here.

Read the plan file's Task 7 (grid.rs section) in full — it contains the exact `gpui-component` API calls and before/after code for every edit, matching what b3 does for `row.rs`/`list.rs` one-for-one (same menu items, same `Entity::update` pattern for reaching pane state from an `on_click` handler that only gets `&mut App`).

## Deliverables (touch only this)

| Path | Role |
|---|---|
| `crates/chronos-fm-pages/src/explorer/view/listing/grid.rs` | context menu (empty area + per-tile) + cut dimming |

Do not touch `row.rs`, `list.rs`, `listing.rs` (b3 owns them), `clipboard.rs`, `rename.rs`, `file_ops.rs`, `state.rs` (b1/b2 own them, already merged — read-only for you).

## Verification (required)

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
cargo build -p chronos-fm-pages
cargo build -p chronos-fm
```

Both must build clean. `chronos-fm-pages`'s own test suite (`cargo test -p chronos-fm-pages`) does not need to change for this task — `grid.rs` has no dedicated unit tests in the plan (GPUI rendering code, not pure logic) — but run it anyway and confirm it's still green (nothing here should break it). Then run the manual smoke checklist above with `cargo run -p chronos-fm` — there is no GUI screenshot harness for this crate, so this step is a manual claim; report it honestly (which checks you actually ran vs. which you couldn't).

## Constraints

- Any element wrapped in `.context_menu(...)` must have an explicit `.id(...)` first, same rule as b3.
- `PopupMenuItem::on_click` handlers only get `&mut App` — reach the pane via `let pane = cx.entity();` captured outside the menu closure, then `pane.update(cx, |pane, cx| { ... })` inside the handler, exactly as the plan shows for `row.rs`.
- Every new `pub` item needs a doc comment.

## Out of scope

List view (b3), grid-mode inline rename *rendering* (functional rename via `begin_rename` still applies, only the visual input-swap is deferred), clipboard/rename/file_ops logic itself (b1/b2, already done).
