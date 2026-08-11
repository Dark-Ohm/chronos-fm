# T054 — Undo stack for file ops

**Epic:** T048. **Priority:** P1.  
**Spec:** `docs/explorer-essentials.md` §1.3

## Problem

No Cmd/Ctrl+Z after rename/copy/move/trash/new folder. Window-scoped undo
is specified; not implemented.

## Must

1. Window-level stack (not per-pane).
2. Ctrl+Z undo / Ctrl+Shift+Z redo (Linux); mirror Cmd on dual-bind style.
3. Undoable: rename, copy, move, trash→restore, new folder.
4. Not undoable: permanent delete, content edit.
5. Session-only (no disk persistence).
6. Caps stack size (document, e.g. 50).

## Done when

Tests for stack + live one undo path; architect ACCEPT.

## Related

T048 · T049 · trash API
