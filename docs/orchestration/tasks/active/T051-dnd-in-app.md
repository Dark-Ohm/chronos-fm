# T051 — Drag-and-drop in-app (listing → folder, cross-pane)

**Epic:** T048. **Priority:** P0.  
**Depends (soft):** T050 multi-select so multi-file drag is real.  
**Code:** explorer listing + pane drop targets; `docs/explorer-essentials.md` §2

## Problem

Settings honestly lists “Drag & drop — not wired yet”. Only **tab reorder**
uses gpui `on_drag`/`on_drop`. Users cannot drag files onto a folder row or
into the other split pane.

## Must (v1)

1. Drag **current selection** (1+ paths) from listing.
2. Drop on **directory row** in same pane → **move** by default.
3. Drop with modifier (Ctrl = copy on Linux — document; match Thunar if easy).
4. Drop into **other pane’s cwd** (empty area or breadcrumb/cwd target) →
   move/copy into that pane’s directory.
5. Drop on directory in other pane → into that directory.
6. Invalid targets: no-op / no-drop cursor; no silent partial success without
   status error.
7. After success: reload affected panes; selection policy documented
   (clear or select dropped).
8. Drag preview: count badge or name stub (can be minimal).

## Non-goals (this ticket)

- External app drop/drag-out → **T052**
- Conflict dialog polish → **T053** (v1 may use existing unique_name /
  fail with status — must not corrupt; document)

## Done when

1. Tests for pure path resolution / move-vs-copy decision if extracted.
2. Live grim: before/after tree or listing showing moved files; two-pane
   drop if split available.
3. Report + architect ACCEPT.

## Related

T048 · T050 · T052 · `file_ops` paste path for reuse
