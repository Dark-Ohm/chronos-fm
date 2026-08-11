# T053 — Paste/drop conflict dialog

> ## ⚖️ ARCHITECT (2026-08-11): **ACCEPT**
>
> Workspace 464 passed / 0 failed; live grim of dialog + Rename queue
> verified; re-entrancy panic fix confirmed by live run without panics.

**Epic:** T048. **Priority:** P1.  
**Depends:** paste (T049) and/or drop (T051) paths.  
**Spec:** `docs/explorer-essentials.md` §1.2

## Problem

Settings: “Paste overwrite — not wired yet”. Spec requires
Rename / Overwrite / Skip / Apply to all when destination name exists.
Without this, bulk paste/drop is unsafe or silent-unique-rename only.

## Must

1. On name conflict during paste or in-app drop: modal with
   **Skip · Rename · Overwrite · Cancel**; **Apply to all** checkbox.
2. Default focus non-destructive (**Rename** per spec).
3. Apply-to-all applies remaining conflicts of same session op.
4. Progress residual may stay T056; v1 can be sequential with one dialog
   at a time.
5. Tests for decision application pure logic.

## Done when

Unit + live grim of dialog; architect ACCEPT.

## Status

**ACCEPT (2026-08-11)** — moved to `done/`

Report: `docs/orchestration/tasks/report-log/T053-paste-conflict-dialog-report.md`  
Design note: `docs/superpowers/specs/2026-08-11-paste-drop-conflict-design-note.md`  
Grims: `docs/orchestration/tasks/report-log/T053-shots/`  
Workspace tests: 464 passed / 0 failed; live queue verified (no panics).

## Related

T048 · T049 · T051 · T052 · explorer-essentials §1.2
