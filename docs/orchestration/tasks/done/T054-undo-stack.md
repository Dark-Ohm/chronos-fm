# T054 — Undo stack for file ops

> ## ⚖️ ARCHITECT (2026-08-11): **ACCEPT**
>
> Workspace 483 passed / 0 failed (re-ran independently); live grims of
> rename→undo and trash→undo verified with footer status; the two bugs the
> internal review caught (trash redo stale `TrashItem`, `delete_permanent`
> on populated folder) are real and properly fixed with tests.

**Epic:** T048. **Priority:** P1.  
**Spec:** `docs/explorer-essentials.md` §1.3

**Status:** ACCEPT — report in
`docs/orchestration/tasks/report/T054-undo-stack-report.md`.

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
