# In-App Drag and Drop (T051) - Design Document

**Date:** 2026-08-10
**Ticket:** `docs/orchestration/tasks/active/T051-dnd-in-app.md`
**Project:** Chronos-FM
**Scope:** Local filesystem drag and drop between Explorer list/grid entries,
folder targets, and the two visible panes.

**Design status:** PROPOSED - awaiting Architect review. Do not implement until
the committed spec receives an Architect `IMPLEMENT GO` stamp.

## 1. Goal and User Contract

T051 adds typed, in-process file drag and drop without creating another
selection or clipboard model.

- A drag that starts on a selected row or tile carries the complete current
  `ExplorerPane::selected_paths()` selection.
- A drag that starts on an unselected row or tile first makes that item the sole
  selection, then carries that one path.
- A plain drop moves by default.
- Ctrl at drop time copies on Linux. The existing `control || platform` convention
  also provides the Cmd mirror on platforms that use it.
- A directory row or tile accepts a drop into that directory.
- Empty listing space and the current-directory breadcrumb accept a drop into
  that pane's `cwd`. Item bounds must exclude rows and tiles from the empty-space
  target so an invalid item target cannot fall through to the surrounding cwd.
- The same rules apply within one pane and across the two visible panes.

Only local filesystem panes participate. Provider-backed paths such as S3 are
not converted to local `PathBuf`s and are invalid T051 targets and sources.

## 2. Approaches

### Approach 1 - Typed payload, pane-owned targets, shared transfer helper

**Adopt.** Define an internal `FileDrag` payload containing the source pane
entity and an immutable path snapshot. List rows and grid tiles create it;
folder items, listing empty space, and the breadcrumb consume it. Both Paste and
Drop call one filesystem batch-transfer helper.

This matches GPUI's existing typed `on_drag`, `can_drop`, `drag_over`, and
`on_drop` APIs, keeps pane-local rendering in the pane, and lets the payload
reload its source after a cross-pane move.

### Approach 2 - ExplorerPage-owned global drag coordinator

**Reject for v1.** Hoisting every item measurement and target decision into
`ExplorerPage` would make the generic `PaneGroup` and page container aware of
file-specific drag state. Cross-pane drops do not require that coupling because
the typed payload can carry the source pane entity.

### Approach 3 - Reuse the clipboard as drag state

**Reject.** Writing Cut or Copy clipboard state at drag start would unexpectedly
replace the user's clipboard, blur cancellation semantics, and make a cancelled
drag observable. Paste and Drop share the transfer operation, not the transient
interaction state.

No Source patch is planned. The local gpui-ce fork already exposes the required
typed drag/drop, drag-over styling, drop predicate, modifier, cursor, and test
event APIs. Source remains the authority if implementation probes reveal a gap.

## 3. Ownership and Data Flow

Add `explorer/dnd.rs` for DnD-specific state and pure decisions. It owns:

- `FileDrag`: source `Entity<ExplorerPane>`, source entity id, immutable local
  paths, and the initiating path/name used by the preview.
- `DropTarget`: a concrete local destination directory and target kind
  (folder item, listing cwd, or breadcrumb cwd).
- `DropMode`: Move or Copy.
- Pure validation and mode-selection helpers.
- The compact drag preview view.

`ExplorerPane::selection` remains the only selection model. The drag payload is
a press-time path snapshot, not a second live selection. T052 may later adapt
the same path payload for external protocol data, but T051 contains no external
URI or clipboard export.

The transfer flow is:

1. The source item builds a payload from the current selection, or only itself
   when it is not selected.
2. When GPUI crosses its drag threshold, the preview constructor makes an
   unselected initiating item the sole pane selection and renders the preview.
3. Targets use `can_drop` for the complete payload and concrete destination.
4. At mouse-up, the target reads `window.modifiers()` and resolves Move versus
   Copy. Modifier state at drag start is not authoritative.
5. The filesystem batch runs on the background executor.
6. Completion updates the destination pane and, for cross-pane Move, the source
   pane. Entity disappearance is a benign cancellation of the UI update, not of
   an already completed filesystem operation.

The generic `PaneGroup` stays file-agnostic. `ExplorerPage` does not own a
parallel drag registry or transfer queue.

## 4. Shared Transfer Path and Conflict Policy

Extract one batch-transfer helper in `chronos-fm-services::fs::ops` and route
both `ExplorerPane::paste_clipboard` and T051 drops through it. There must be no
separate Paste-versus-Drop name-resolution loop.

The helper accepts source paths, a destination directory, and Copy/Move mode.
For every source it:

1. Reads the source's single final component.
2. Calls the existing `ops::unique_name(destination, name)` immediately before
   the operation.
3. Calls `ops::copy_path` or `ops::move_path` with that resolved destination.
4. Records success with source/destination paths and whether the destination
   name changed, or records a source-scoped error.

`move_path` retains its existing cross-volume `rename` then copy-plus-delete
fallback. Cross-volume does not alter the conflict rule.

T051 never overwrites an occupied destination name. `file.txt` becomes
`file (2).txt`, matching Paste. A collision status such as
`Renamed to file (2).txt` is optional and is not an ACCEPT gate. T053 will later
replace the fixed policy with Rename/Overwrite/Skip/Apply-to-all UI for both
Paste and Drop. Until then, automatic `unique_name` is the product policy.

The batch result preserves all successes and failures. Runtime partial success
must be surfaced as an error status with counts and failing names; it must not
look like total success. There is no rollback transaction in T051.

## 5. Target Validation

Validation is pure where possible and is shared by `can_drop` and `on_drop` so
visual acceptance cannot disagree with execution.

A target is valid only when:

- The payload contains at least one local path.
- The destination exists and is a local directory.
- No source is the destination itself.
- No directory source would be copied or moved into itself or one of its
  descendants. Existing paths are canonicalized for this containment check.
- A Move does not target the source's current parent directory. This is a no-op,
  not an automatic rename.

Copying into the same parent is valid and produces a unique-name duplicate.
If any payload member fails a predictable validation rule, the entire target is
invalid before filesystem work begins. This prevents a mixed selection from
partially executing merely because one member was self-referential.

Directory item targets are above the cwd surface in event routing. The cwd
surface additionally rejects pointer positions inside any measured row/tile
bound, whether the item is a file, an invalid directory, or a valid directory.
Thus a rejected item drop cannot bubble into an unintended parent-directory
drop.

## 6. Selection and Reload Policy

The destination pane reloads after any successful transfer. Destination paths
that are visible in its current `cwd` become the new destination selection in
`filtered_entries` order.

- Cross-pane Move: reload source and destination; clear stale source selection;
  select successful destinations in the destination pane.
- Cross-pane Copy: retain source selection; reload and select successful
  destinations in the destination pane.
- Same-pane drop into a child directory: reload the current source listing;
  moved sources disappear and selection clears. The child contents are not
  selected because that directory is not the pane's current listing.
- Same-pane Copy into the current cwd: reload once and select the newly created
  unique-name copies.

If source and target are the same pane entity, update and reload it once; never
re-enter `Entity::update` on the entity whose listener is already running.
Other tabs are not force-reloaded in v1. The two participating visible panes are
the affected panes required by T051.

On total failure, preserve the existing listings and show an error status on the
drop target. On partial failure, reload the affected panes, select successful
destinations where visible, and show an error summary. Completion sends
`cx.notify()` for every pane whose visible state changed.

## 7. Interaction and Rendering

Rows and tiles are drag sources only when they represent local entries and no
inline rename is active for that item. Search fields, rename inputs, headers,
resize handles, menus, dialogs, breadcrumbs, and listing chrome never become
file drag sources accidentally.

The preview is compact and stable:

- One item: file/folder icon and truncated name.
- Multiple items: first name plus a visible count badge (`+N` or `N items`).
- Maximum dimensions prevent a long name from resizing the preview.

Valid folder/cwd targets receive a restrained accent border and light accent
fill through typed drag-over styling. Cursor policy is:

- Move: closed-hand/move cursor.
- Ctrl/Cmd Copy: GPUI `DragCopy`.
- Invalid or non-target: `OperationNotAllowed`.

Cursor and target highlight update from the current modifier/hover state. A
drop target never navigates into the directory, opens it, or becomes
spring-loaded in T051.

Starting a file drag cancels any active T050 marquee. A file drag cannot start
from empty listing space, so marquee and file DnD remain disjoint interactions.

## 8. Concurrency and Failure Handling

Filesystem transfer work runs off the GPUI UI thread. T051 does not add the
T056 progress UI, cancellation, or operation queue; while one drop is pending,
that destination pane rejects another T051 drop and may show a muted working
status.

Errors from invalid/missing paths, permissions, destination lookup, copy, move,
or cross-volume cleanup are converted to pane status messages. No panic and no
silent overwrite are allowed. The typed drag ends on mouse-up whether the drop
is accepted or rejected.

If the target pane closes before background completion, the filesystem result
is still logged. If the source pane closes, destination completion still
reloads and reports normally. Weak entity update failures are ignored only
after the operation result has been recorded.

## 9. Testing and Evidence

Pure/service tests cover:

- Plain means Move; Ctrl on Linux and platform/Cmd mirror mean Copy.
- Empty payload, non-directory destination, source equals target, directory
  into descendant, and same-parent Move rejection.
- Same-parent Copy acceptance.
- Shared batch transfer uses `unique_name` for both Copy and Move.
- The batch helper continues to delegate Move to `move_path`; same-volume move
  behavior stays covered, and an EXDEV fallback test is added when the test
  harness can inject or provide distinct filesystems without host assumptions.
- Batch results distinguish full success, total failure, and partial failure.

Root-wrapped GPUI tests use the production list/grid and split-pane render tree:

- Dragging a selected item carries all selected paths.
- Dragging an unselected item makes it the sole selection.
- List and Grid both originate file drags.
- Same-pane folder item and other-pane folder item accept valid drops.
- Other-pane empty listing space and breadcrumb target its `cwd`.
- File rows, self/descendant folders, provider panes, controls, and item-covered
  cwd space reject the drop without fall-through.
- Ctrl changed during a drag is evaluated at drop time.
- Preview count, valid highlight, invalid cursor, successful reload, selection
  policy, and error status are observable.

Verification requires focused tests, `cargo test -p chronos-fm-pages --lib`,
`cargo test --workspace`, and `cargo build --release -p chronos-fm`.

Runtime evidence uses the release `class=chronos-fm` window with a real local
two-pane fixture. The report records exact automation commands and filesystem
observations, and includes:

- A held-drag frame showing the preview and a highlighted destination target.
- Before/after evidence showing a multi-file move between panes.
- A Ctrl-copy observation proving the source remains and the unique destination
  appears. A separate grim is optional when the filesystem command evidence and
  visible destination state are unambiguous.

The executor reports Claim -> Evidence -> Truth base and does not self-ACCEPT.

## 10. Non-Goals and Handoffs

- External drag-in or drag-out and MIME/URI transport (T052).
- Overwrite, Skip, Apply-to-all, or conflict dialogs (T053).
- Undo/redo recording (T054).
- Progress UI, cancellation, or a general operation queue (T056).
- Spring-loaded folders, hover navigation, or automatic directory opening.
- S3/Git/plugin semantic drops.
- Marquee auto-scroll or changes to T050 selection geometry.
- A user-configurable DnD keymap.
