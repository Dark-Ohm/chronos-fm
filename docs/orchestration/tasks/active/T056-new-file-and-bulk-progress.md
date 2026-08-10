# T056 — New empty file + bulk op progress/cancel

**Epic:** T048. **Priority:** P2.  
**Spec:** explorer-essentials §1.4 (progress)

## Problem

Only **New Folder** exists. Bulk copy/move of many files has no progress UI
or cancel (footer bar in spec).

## Must

1. Context menu / key residual: **New File** → unique name + optional inline
   rename (same as new folder pattern).
2. For multi-file copy/move/trash above a threshold (e.g. >1s or >N files):
   footer progress + **Cancel** cooperative flag.
3. Completion toast/status “N files moved” brief.

## Done when

Unit + live grim; architect ACCEPT.

## Related

T048 · T053 · `file_ops`
