# File Operations Keybindings (T049) - Design Document

**Date:** 2026-08-10  
**Ticket:** `docs/orchestration/tasks/active/T049-file-ops-keybindings.md`  
**Project:** Chronos-FM (`crates/chronos-fm-pages/src/explorer/`)  
**Scope:** Built-in file-operation shortcuts for a focused explorer pane.

## 1. Goal

Add the standard file-manager shortcuts from T049 without introducing a user
keymap registry. Each shortcut must operate on the focused pane and use the
same production operation path as the corresponding context-menu command.

The current uncommitted T049 diff is failed, incomplete work. It may be
salvaged only where its behavior, focus semantics, and tests survive review.

## 2. Input Routing

Keep shortcut dispatch on the root element of each `ExplorerPane`, alongside
the pane's existing selection and navigation keyboard handling. The focused
pane owns file operations; `ExplorerPage` continues to own split and tab
shortcuts.

On Linux, Ctrl bindings are required. The existing platform-modifier handling
also provides Cmd mirrors. The supported commands are:

- Ctrl/Cmd+C: copy the current selection.
- Ctrl/Cmd+X: cut the current selection.
- Ctrl/Cmd+V: paste into the pane's current directory.
- F2: inline rename for one selected entry; Batch Rename for multiple entries.
- Delete: open the existing trash confirmation flow.
- Ctrl/Cmd+A: select every filtered entry.
- Enter: activate the current entry through the existing open/navigation path.
- Ctrl/Cmd+Shift+N: create a folder in the current directory.

The pane must not steal editing keys from a focused search, inline-rename, or
dialog input. Editing controls remain the nearest consumer of copy, cut,
paste, select-all, Delete, and Enter as applicable.

## 3. Shared Operation Paths

Keyboard and context-menu entry points call shared `ExplorerPane` methods:

- Copy, cut, and paste use the existing clipboard methods.
- Rename uses one selection-aware method so F2 and the context menu cannot
  diverge between inline and batch behavior.
- Delete uses one confirmation helper and never calls `delete_paths` directly.
- Enter uses the existing entry activation method.
- New Folder uses the existing unique-name and inline-rename flow.

No file operation is reimplemented in the key handler. Empty selections and
missing active rows are harmless no-ops. Existing operation failures continue
to surface through the pane status/error mechanisms.

## 4. Testing

Use GPUI keystroke dispatch through a `gpui_component::Root`-wrapped explorer
window, matching the production dispatch tree. Tests must cover:

- Copy, cut, and paste effects.
- Single-selection and multi-selection F2 behavior.
- Delete opening confirmation without deleting before confirmation.
- Select all over filtered entries.
- Enter navigating into a selected directory.
- New Folder creation.
- Focused text input isolation from file-operation shortcuts.

Verification proceeds from focused keybinding tests to the full pages crate,
workspace tests, formatting for touched files, and a release build. Runtime
keyboard evidence is preferred for the report. The executor reports evidence
but does not self-ACCEPT the ticket.

## 5. Non-Goals

- User-editable keybindings or Settings UI integration.
- Undo, drag-and-drop, or conflict-resolution changes.
- Refactoring split/tab shortcuts or introducing a new global command system.
- Unrelated cleanup in the existing dirty worktree.
