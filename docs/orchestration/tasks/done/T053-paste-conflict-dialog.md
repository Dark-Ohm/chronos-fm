# T053 — Paste/drop conflict dialog

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

## Related

T048 · T049 · T051 · explorer-essentials §1.2
