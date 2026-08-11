# T054 — Executor brief: Undo stack for file ops

**Ticket:** `active/T054-undo-stack.md` · **Epic:** T048
**Priority:** P1 — spec: `docs/explorer-essentials.md` §1.3, §8
**Revision 2** — architect-reviewed; v1 had incomplete call sites and an
unlocked pane→page architecture. Fixed below, read this version.

## Module path (spec names it, use it)

`docs/explorer-essentials.md` §8: "undo stack は `chronos-fm-pages::
explorer::undo` で管理" — new file `crates/chronos-fm-pages/src/explorer/
undo.rs`. Pure stack struct/logic lives there, unit-tested with no GPUI
context (same style as T053's `conflict.rs::ConflictQueue`).

## Where window-level state lives (checked, not assumed)

`RootView` (`root.rs:39-47`) holds exactly **one** `Entity<ExplorerPage>`
per window — `ExplorerPage` (`page.rs`) is the window-scoped level, and
already hosts `PANES_CONTEXT` keybindings (`page.rs:75` `actions!(...)`,
dual-bind `cmd-`/`ctrl-` pattern at `page.rs:101-130`, e.g.
`KeyBinding::new("cmd-t", NewTab, Some(PANES_CONTEXT))` /
`KeyBinding::new("ctrl-t", ...)`). **Locked decision:** the undo stack
field and `Undo`/`Redo` actions live on `ExplorerPage`, dual-bound
`cmd-z`/`ctrl-z` and `cmd-shift-z`/`ctrl-shift-z` under `PANES_CONTEXT` —
copy the existing pattern, don't invent a new one.

## Locked architecture: how a pane's mutation reaches the page's stack

Mutations happen on `ExplorerPane`; the stack lives on `ExplorerPage` one
level up. `PaneEvent` (`types.rs:74-77`) currently has exactly one variant
(`Navigated(String)`) — there is no existing channel for "an undoable
mutation happened". **Do not invent your own** (no new global, no ad-hoc
`WeakEntity` back-reference) — extend `PaneEvent` with a variant carrying
an undo-entry payload (e.g. `PaneEvent::Undoable(UndoEntry)`, entry type
defined in `undo.rs`) and push onto the page's stack from the page's
existing pane-event subscription path (`ExplorerPage` already reacts to
`PaneEvent` for tab/pane bookkeeping — extend that match, don't add a
second observer).

## Call sites — the full set (v1 of this brief only listed file_ops.rs;
## rename and both drop paths were missing — fixed here)

| Op | Where | Reverses via |
|---|---|---|
| Rename (single) | `rename.rs:62` `commit_rename` → `ops::rename_in_place` (`:76`) | `rename_in_place(new_path, old_name)` |
| Rename (batch) | `batch_rename.rs:144` `apply` → `ops::rename_in_place` (`:153`), looped per file | **one compound undo entry for the whole batch gesture**, not N entries — see policy below |
| Paste copy/move | `file_ops.rs:72` `paste_clipboard`, after conflict resolution | `ops::TransferSuccess` (see below) |
| In-app drop | `dnd.rs` `begin_file_drop` → `transfer_paths_resolved` | same `TransferSuccess` path |
| External drop | `dnd.rs` `begin_external_drop` → `transfer_paths_resolved` | same `TransferSuccess` path |
| New folder | `file_ops.rs:109` `new_folder` | delete the created dir — see new-folder+rename policy below |
| Trash | `file_ops.rs:135/164` `confirm_delete_selection`/`delete_paths` → `ops::trash_path` (`ops.rs:340`) | `trash::os_limited::restore_all` — see trash wall below |

**`copy_selection`/`cut_selection` (`file_ops.rs:21,31`) are NOT push
sites** — they only write the in-memory clipboard, no filesystem
mutation happens yet. The undo entry is created when the *paste* commits
(`paste_clipboard`, after conflict resolution succeeds), not when the
user presses Ctrl+C/Ctrl+X. Pushing on copy/cut produces stack garbage
and no-op undos — don't.

## Reverse info comes free from `TransferSuccess` — use it, don't reconstruct paths

`ops.rs:35-43`: every successful copy/move already returns
`TransferSuccess { source: PathBuf, destination: PathBuf, renamed: bool,
move_kind: Option<MoveKind> }` inside `TransferReport::successes`
(`ops.rs:59`). This is exactly what an undo entry needs — `source` +
`destination` for the reverse copy/move, `move_kind` (`Rename` vs.
`CrossVolume`, `ops.rs:16-22`) to pick the right reverse operation. Build
the undo entry from this struct at the paste/drop call sites; don't
re-derive source/destination from UI state after the fact.

## The wall: trash restore needs an identity, not just a path

`trash` crate — `Cargo.toml:36` pins `"5"`, **`Cargo.lock` resolves
5.2.6** (say it this way in the report, not "pinned 5.2.6 in Cargo.toml").
Checked the installed source directly:

- `ops::trash_path` (`ops.rs:340-342`) currently calls
  `trash::delete(path)` and discards everything about where the item
  went.
- Restore is `trash::os_limited::restore_all` — `os_limited` (Linux/
  Windows per the crate's own module doc; fine for this app's Linux
  target) — and it takes `TrashItem` values obtained from
  `trash::os_limited::list()`, **not** returned by `delete()`.
- **Consequence:** to make trash undoable, capture the matching
  `TrashItem` (e.g. via `list()` right after `delete()`, matched by
  original path) at delete time — a "record the path, call restore
  later" design will not compile against this crate's actual surface.
  Confirm the exact call shape against
  `~/.cargo/registry/src/*/trash-5.2.6/src/lib.rs` before writing the
  Trash undo-entry variant.

## Permanent delete — not in scope, don't build a dual path for it

Checked: the explorer's delete keybinding/menu path
(`confirm_delete_selection`/`delete_paths`) is **trash-only** today.
`ops::delete_permanent` exists in the services crate but nothing in the
Explorer UI calls it — Shift+Delete-style permanent delete is listed in
`docs/explorer-essentials.md` §6/settings as "not wired yet", not a real
call site. **Don't spend time distinguishing trash-vs-permanent inside
`file_ops.rs`** — there is only one delete path today, and it's
undoable. Just don't wire undo *to* `delete_permanent` if you touch it
for any other reason.

## Architect-stamped decisions (don't leave these to guesswork)

1. **Overwrite (T053 conflict resolution) is NOT undoable in T054 v1.**
   It destroys the destination's prior content; making it undoable needs
   a pre-overwrite backup, which is new scope beyond this ticket. Report
   must state this explicitly — Overwrite is excluded from the undo
   stack, Rename/Skip/no-conflict copy-or-move are included.
2. **Batch rename = one undo entry per user gesture**, not N. Undoing a
   batch-rename-of-12-files is one Ctrl+Z, not twelve.
3. **New folder + immediate rename — stamped: two entries.**
   `new_folder` creates a directory then typically opens rename-in-place
   for it. Push the *create* as one undo entry (delete-on-undo of
   whatever the current path is); if the user renames before undoing,
   push a second, ordinary rename undo entry on top through the normal
   rename path (no special-casing needed). Two `Ctrl+Z`'s fully unwind
   create+rename, one `Ctrl+Z` just undoes the rename and leaves the
   (now originally-named) folder in place. This reuses the rename path
   as-is instead of detecting "was this an atomic gesture" — simpler,
   no new state to track.

## Must (from the ticket)

1. Window-level stack (`ExplorerPage`, per locked architecture above).
2. `Ctrl+Z` undo / `Ctrl+Shift+Z` redo, dual-bound with `Cmd`.
3. Undoable: rename (single + batch), copy, move, trash→restore, new
   folder — full call-site table above, not paste-only.
4. Not undoable: permanent delete (not a real call site today — see
   above), content edit, Overwrite conflict resolution (architect-stamped
   above).
5. Session-only — no disk persistence, no `state.redb` wiring.
6. Cap stack size at **50** (this ticket's number — `docs/
   explorer-essentials.md` §1.3 doesn't specify one, the ticket does;
   don't cite it as a spec requirement). At the cap, drop the oldest
   entry silently — no user-visible warning needed.

## Tests (mandatory)

- Pure stack logic in `undo.rs` (push/undo/redo/cap-eviction), no GPUI
  context — same style as `conflict.rs`'s `ConflictQueue` tests.
- At least one live grim: perform an op (e.g. rename), `Ctrl+Z`, confirm
  the original name is back on screen, `class=chronos-fm`.
- `cargo test --workspace` green, new test count visible in the report.

## Done when

Tests for the stack + live one-undo-path proof; report with Claim →
Evidence blocks covering every row of the call-site table (which ops
were wired, which weren't and why); no self-ACCEPT, ticket stays in
`active/` until architect stamp.

## Related

T048 (epic) · T049 (keybinding pattern to copy) · T053 (conflict
resolutions — Overwrite explicitly excluded, see stamped decision above)
· `docs/explorer-essentials.md` §1.3, §8 · trash crate source (path
above) for the restore API shape
