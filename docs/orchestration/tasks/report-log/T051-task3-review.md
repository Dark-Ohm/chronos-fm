# T051 Task 3 — Architect review (gate close)

**Date:** 2026-08-11  
**Worktree HEAD:** `e0da38c` (`feat/t051-dnd`)  
**Verdict:** **PASS — Task 3 review closed. GO Task 4.**

## Why the stuck `/root/t051_task3_review` is closed without a new agent

The automated reviewer never completed (usage limit / interrupt). Architect
performed a targeted static + test re-verification of the three open questions.

## Findings (no blockers)

### 1. Invalid item does not fall through to cwd

Production path:
- Listing surface `can_drop` / `drag_over` / `on_drop` all call
  `ExplorerPane::can_accept_listing_cwd_drop`
  (`view/listing.rs` ~125–177).
- That helper requires pointer inside listing viewport **and** outside every
  `measured_items` hitbox **and** `validate_drop` (`dnd.rs` ~279–295).

Test:
- `dnd_routing_items_rename_header_and_provider_reject` hovers a **file** under
  an otherwise valid Ctrl-copy-to-cwd drag, asserts
  `CursorStyle::OperationNotAllowed`, `!drop_pending`, and no `a (2).txt`.

### 2. Cursor / highlight are not false-positive-only tests

- Folder rows: cursor + `drag_over` style both gate on `can_accept_file_drop`
  (same as `can_drop` / `on_drop`).
- Cwd surface: cursor + highlight gate on `can_accept_listing_cwd_drop`
  (same as execution).
- Routing tests assert **cursor + drop_pending + filesystem** together for
  accept paths (empty space, breadcrumb, folder) and reject paths (file
  cover, provider, rename chrome).

### 3. Event ordering (marquee vs file drag)

- `FileDrag::activate` cancels marquee and normalizes unselected press to
  sole selection (`dnd.rs` ~73–93).
- Covered by `dnd_routing_unselected_list_row_normalizes_and_cancels_marquee`.
- Empty-space marquee and file `on_drag` remain disjoint (row/tile sources
  only when `file_drag_for_item` returns Some).

## Re-run evidence (architect)

```
cargo test -p chronos-fm-pages --lib dnd  → 22 passed, 0 failed
```

(plus prior green marquee/keybindings/pages matrix from executor Task 3.)

## Gate decision

| Task | Status |
|------|--------|
| 1 Shared transfer + Paste | done on branch |
| 2 Domain + async lifecycle | done on branch |
| **3 List/grid routing** | **done + review PASS** |
| 4 Split-pane E2E hardening | **next** |
| 5 Release grims + report | after 4 |

No code change required for this gate close.
