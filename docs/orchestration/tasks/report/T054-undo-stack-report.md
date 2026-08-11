# T054 — Undo stack for file ops: report

**Status:** IMPLEMENTED — awaiting Architect review. Executor does not self-ACCEPT.

Spec: `docs/explorer-essentials.md` §1.3. Ticket:
`docs/orchestration/tasks/active/T054-undo-stack.md`.

## Outcome

Window-scoped `Ctrl+Z` undo / `Ctrl+Shift+Z` redo for rename, batch rename,
copy, move, trash→restore, and new folder. One stack per window (shared
across panes and tabs, §1.3), session-only, capped at 50 entries (silent
eviction). Permanent delete and file-content edits stay non-undoable;
T053 `Overwrite` conflict resolutions are excluded from undo (architect
decision #1: overwrite destroys the destination's prior contents).

`ExplorerPane` emits `PaneEvent::Undoable(UndoEntry)` after each successful
mutation; `ExplorerPage` owns the pure `UndoStack` and executes
`reverse()`/`forward()` on the filesystem, reporting success back so a
failed undo/redo is dropped from history rather than re-offered. Undo/redo
reload every live tab and report through the active pane's footer status
("Undid copy" / "Redid move to trash").

A live-run discovery beyond the ticket's scope: after an inline rename
commits, keyboard focus was left on the (dropped) rename input, so the
subsequent `Ctrl+Z` never dispatched to the pane. `commit_rename` /
`cancel_rename` now re-focus the pane's `focus_handle` (the same pattern the
search bar lacks today — noted, not fixed here).

## Claims and evidence

Claim: The stack is window-scoped, pure, capped at 50, and correct across
undo/redo/eviction/fresh-mutation-clears-redo.

Evidence: `crates/chronos-fm-pages/src/explorer/undo.rs` —
`UndoStack { undo, redo }` with `push` (clears redo, evicts oldest past
`UNDO_CAPACITY`), `undo`/`redo` pops, and `record_undone`/`record_redone`.
Pure tests: `stack_undo_pops_most_recent_and_redo_restores_it`,
`stack_new_mutation_clears_redo_branch`,
`stack_evicts_oldest_over_capacity`, `stack_record_undone_and_redone_round_trip`,
`stack_empty_undo_and_redo_are_noops`. `cargo test -p chronos-fm-pages --lib
explorer::undo` — 6 passed, 0 failed.

Truth base: Chronos-FM.

Claim: Every undoable op reverses and re-applies correctly on a real
filesystem, including batch-rename permutation reversal and trash restore.

Evidence: `UndoEntry::reverse`/`forward` in `undo.rs` (rename, batch rename
reversed in reverse application order, copy deletes destinations, move
moves back, folder delete/recreate, trash via `restore_trash_items` /
`trash_path`). Filesystem round-trip tests: `rename_reverse_and_forward_round_trip`,
`batch_rename_reverse_restores_original_names`,
`copy_reverse_deletes_destination_and_forward_recopies`,
`move_reverse_moves_back_and_forward_removes`,
`create_folder_reverse_deletes_and_forward_recreates`. All in the same 6/6
`explorer::undo` result.

Truth base: Chronos-FM.

Claim: The trash path is real OS-trash, not a copy — services expose
`trash_path_undoable` + `restore_trash_items` and the `TrashItem`
round-trip works in this environment.

Evidence: `crates/chronos-fm-services/src/fs/ops.rs` — `trash_path_undoable`,
`restore_trash_items`, `pub use trash::TrashItem`, and the `TransferSuccess`
gains `overwrote`. Tests: ops trash round-trip test plus the `overwrote`
assertion in the mixed-plan transfer test. `cargo test -p
chronos-fm-services` — 146 passed, 0 failed.

Truth base: Chronos-FM.

Claim: Paste and Drop record one compound entry per gesture, excluding T053
overwrites; no duplicated recording logic.

Evidence: `transfer_entry(report, mode)` in `undo.rs` builds a single
`Copy`/`Move` entry from `TransferReport.successes` filtered on
`!overwrote`, returning `None` for an all-overwrite/empty gesture
(`transfer_entry_excludes_overwrites_and_returns_none_on_empty`). Callers:
`file_ops.rs` (paste_clipboard), `dnd.rs` (both drop completion paths).
Rename/batch-rename/new-folder/delete call sites emit their entries in
`rename.rs`, `batch_rename.rs`, `file_ops.rs`.

Truth base: Chronos-FM.

Claim: Page-level e2e — undo/redo round-trips through the real pane event
subscription, window-scoped across panes.

Evidence: `crates/chronos-fm-pages/src/explorer/tests.rs`
`page_undo_redo_rename_round_trips` (rename→undo→redo on disk),
`page_undo_paste_copy_removes_the_copy_and_redo_recopies` (copy→undo removes
destination→redo re-copies),
`page_undo_new_folder_deletes_it`,
`page_undo_is_window_scoped_across_panes` (mutation in pane 1, undo from the
page stack). `cargo test -p chronos-fm-pages` — 208 passed, 0 failed.
`cargo test --workspace` — **483 passed, 0 failed** (per-crate totals vary
slightly between `--workspace` feature-unified builds and isolated `-p`
runs — e.g. services counts 4 feature-gated tests more in the workspace
context; the architect re-ran the workspace suite independently and
confirmed 483).

Truth base: Chronos-FM.

Claim: Clippy is clean on the T054 file set (two warnings found on my code
were fixed — `UndoEntry`/`PaneEvent` visibility and dead `can_undo`/`can_redo`,
now `#[cfg(test)]`).

Evidence: `cargo clippy -p chronos-fm-pages -p chronos-fm-services` — no
warnings in undo.rs / ops.rs / page.rs / types.rs / rename.rs /
batch_rename.rs / file_ops.rs / dnd.rs / view.rs (remaining crate warnings
are pre-existing elsewhere).

Truth base: Chronos-FM.

## Live release evidence

Compositor: Hyprland 0.56.2, DP-1 2560x1440. Binary:
`target/release/chronos-fm` (built from this tree), isolated `HOME`
`/tmp/t054_home` (alpha.txt / delta.txt / gamma.txt, no real user data),
`--theme dark --accent blue`, class `chronos-fm`, moved to ws 12.

Input: `ydotool` evdev keycodes (Down=108, F2=60, Delete=**111**, Enter=28,
Ctrl=29, Z=44). Two live-run gotchas recorded honestly: (1) my first Delete
keypress used X11 code 119, which is `KEY_PAUSE` in evdev — the app's own
`on_key_down` trace logged `key=pause`, proving the harness error, not an
app defect; corrected to 111. (2) The live session's focus is contested by
the user's other apps (`input:follow_mouse=1`), so every step re-dispatched
focus to ws 12 and verified `activewindow` immediately before injecting keys.

1. **Rename + undo + redo**: Down → F2 → Ctrl+A → "beta" → Enter renamed
   `alpha.txt`→`beta.txt` (FS + grim). `Ctrl+Z` restored `alpha.txt` —
   `docs/orchestration/tasks/report-log/t054_undo_grim.png`. `Ctrl+Shift+Z`
   re-applied to `beta.txt` — `t054_redo_grim.png` (shows "B beta").
2. **Trash + undo + redo**: Down×3 → Delete opened
   "Delete Selected Items?" ("item(s) will be moved to Trash.", Cancel/Delete)
   — `t054_dlgok.png`. Enter confirmed: the selected `alpha.txt` moved to
   trash (FS: MISSING; `TrashItem` recorded with `original_parent:
   /tmp/t054_home`). `Ctrl+Z` restored it from the OS trash — FS: EXISTS,
   trash dir empty, footer shows "Undid move to trash"
   — `t054_undone.png`. `Ctrl+Shift+Z` re-trashed it — FS: MISSING, trash
   `files/` contains alpha.txt, footer shows "Redid move to trash" + 2 items
   — `t054_redo.png`.

The temporary `tracing::info!` instrumentation used to diagnose the live
session (pane `on_key_down` / focus, undo push, undo action) was removed
after the run; the release binary in the tree is trace-free.

## Non-negotiables checklist

- Window-level stack (not per-pane): yes (§1.3, one `UndoStack` on
  `ExplorerPage`; e2e `page_undo_is_window_scoped_across_panes`).
- Ctrl+Z / Ctrl+Shift+Z with Cmd mirrors: yes, dual bindings in
  `bind_pane_keys` under `PANES_CONTEXT` (`cmd-z`/`ctrl-z`,
  `cmd-shift-z`/`ctrl-shift-z`).
- Undoable: rename, copy, move, trash→restore, new folder: yes.
- Not undoable: permanent delete, content edits, T053 overwrites: yes.
- Session-only, no persistence: yes.
- Stack cap documented (50, silent eviction): yes (`UNDO_CAPACITY`).
- Focus restored after inline rename so `Ctrl+Z` dispatches: yes.
- No Source/ edits (git status in `Source/` unchanged this session): yes.
- No `git push`, no self-ACCEPT, ticket left in `active/`: yes.

## Independent review — findings addressed

Code review surfaced four points; two were verified as non-issues and two
were real bugs, both fixed with tests:

1. **Transfer mode flows through correctly** (verified, no change):
   `file_ops.rs:paste_clipboard` derives `TransferMode` from the clipboard
   mode (Cut→Move, Copy→Copy) and passes the *same* value to both
   `transfer_paths_resolved` and `transfer_entry`; `dnd.rs` resolves `mode`
   at drop time and threads it through `complete_file_drop`/`complete_external_drop`
   into the entry. No path hardcodes the mode.
2. **Single emission through the T053 conflict path** (verified, no
   change): `transfer_with_conflict_dialog` runs the transfer + `emit`
   closure exactly once, after the last conflict decision; `transfer_entry`
   excludes `overwrote` successes, so a T053 Overwrite cannot leak onto the
   stack.
3. **Trash redo→undo stale `TrashItem`** (fixed): redo re-trashes the file
   and creates a *new* trash entry, so the recorded item's id is dead on a
   second undo. `restore_trash_items` now restores by identity when the
   trashinfo is live and otherwise falls back to the live entry whose
   `original_path` matches, passing a truly-stale item through so the error
   still surfaces. Test: `restore_trash_items_recovers_after_redo_retrash_created_a_new_entry`
   (ops.rs).
4. **`CreateFolder` undo could delete a populated folder** (fixed):
   `delete_permanent` recurses (`remove_dir_all`), so undoing a "New Folder"
   the user had since filled would destroy their files. `reverse` now uses
   the new `ops::delete_empty_dir` (empty-only; "not empty" error otherwise)
   — a failed undo drops the entry and reports "Undo failed: … not empty …"
   instead of deleting data. Tests: `delete_empty_dir_only_removes_empty_directories`
   (ops.rs) and `create_folder_reverse_refuses_a_populated_folder` (undo.rs).

## Residual / handed off

- The search bar's open/close does not restore pane focus (same focus gap
  the rename path had); pre-existing, out of scope.
- Paste remains synchronous (pre-existing).
- Permanent-delete UI call sites do not exist today, so no undo entry is
  recorded for them — matches the "not undoable" list.
- The live trash flow exercised a single-file trash+restore; multi-file
  trash undo is covered by unit/e2e (`Trash { items: Vec }` iterates every
  item in both directions).
- Undo of a trash op that was redo'd and then undo'd again now works through
  the original-path fallback (covered by the ops.rs redo-retrash test), but
  was not repeated in a live run — the live evidence covers delete→undo→redo.
