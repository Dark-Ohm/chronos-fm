# T048 — Epic: Explorer essentials (Thunar/Dolphin daily-driver parity)

**Priority:** P0 product (replaces “feels incomplete as a file manager”).  
**Role:** index only — work lives in children.  
**Orthogonal to:** T042 pixel-copy (tabs vision). Can run in parallel with T039 residual.

## Why

Context menus already cover copy/cut/paste/rename/new folder/trash/open-with
(b1–b4, T006, T007). What still blocks **daily muscle memory** vs Thunar/Dolphin:

- almost no **keyboard** for file ops
- no **rubber-band / marquee** multi-select with mouse
- no **file drag-and-drop** (in-app or external)
- no paste **conflict** policy, **undo**, terminal-here, etc.

## Strategy

1. Selection UX first (keyboard multi-select + marquee) so ops act on real sets.
2. DnD in-app (same pane → folder, cross-pane), then external.
3. Conflict + undo so bulk moves are safe.
4. Terminal / New File / bulk progress as polish.

Truth bases: `crates/chronos-fm-pages/src/explorer/*`,  
`docs/explorer-essentials.md`, live release + grim `class=chronos-fm`.  
Source edits only if gpui DnD/input APIs are missing — **what / why / зачем**.

## Children

| ID | Title | Pri | Depends |
|----|-------|-----|---------|
| **T049** | File-ops keybindings (Ctrl+C/X/V, F2, Del, Ctrl+A, …) | P0 | — |
| **T050** | Multi-select: Shift/Ctrl-click + **marquee (rubber-band) mouse** list+grid | P0 | — |
| **T051** | **DnD in-app** (listing → folder, cross-pane move/copy) | P0 | T050 preferred |
| **T052** | **DnD external** (into/out of Chronos-FM ↔ other apps) | P1 | T051 |
| **T053** | Paste/drop conflict dialog (Rename/Overwrite/Skip/Apply all) | P1 | T049 or T051 |
| **T054** | Undo stack (rename/copy/move/trash/new folder) | P1 | T049 |
| **T055** | Open terminal here | P2 | — |
| **T056** | New empty file + bulk op progress/cancel | P2 | T053 for progress |

## Order

1. T049 ∥ T050 (keyboard + marquee — independent)
2. T051 (needs multi-select quality from T050)
3. T052 after T051 stable
4. T053 when paste/drop paths need policy
5. T054 after ops are keyboard-reachable
6. T055 / T056 polish

## Non-negotiables

- Real FS only; no fake progress for screenshots
- Claim → Evidence → Truth base; vision for interactive ACCEPT
- Executors do not self-ACCEPT
- Stage tickets **by file**, never whole `active/` dir

## Done when (epic)

T049–T052 ACCEPT (or waived by client). T053–T056 ACCEPT or residual filed.
User can: multi-select with mouse rectangle + keys; drag within app and to/from
another app; copy/cut/paste/delete/rename from keyboard.
