# T050 — Multi-select: modifiers + marquee (rubber-band) mouse

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-10)
>
> Merged on main `f6c8f99`. Measured-bounds marquee list+grid; 22 marquee tests
> green; live grims list/grid. Report:
> `report/T050-marquee-multi-select-report.md`. Shots: `report-log/T050-marquee-*.png`.
>
> Residual: off-screen/auto-scroll (non-goal); T051 DnD next.


**Epic:** T048. **Priority:** P0.  
**Code:** listing list + grid (`explorer/view/listing*`), selection state on `ExplorerPane`

## Problem

Selection is index-set based and multi-select is used by batch rename / copy,
but **mouse rubber-band (marquee) selection** is missing or incomplete — user
cannot drag a rectangle over several files (grid or list) to select them.
Without this, “select several then drag” (T051) and batch ops feel broken.

## Must

### Click modifiers
1. **Click** — select one (clear others).
2. **Ctrl+Click** — toggle item in selection.
3. **Shift+Click** — range from anchor to clicked index (list order / grid
   visual order — document choice in report).
4. Preserve anchor for repeated Shift ranges (standard FM behaviour).

### Marquee (rubber-band)
5. Press empty space in listing (not on row chrome that steals drag) → drag
   rectangle; all items whose hitboxes intersect rect join selection.
6. Works in **list** and **grid** layouts.
7. Visual: dashed/accent rectangle while dragging; selection highlights live.
8. Ctrl+marquee = additive; plain marquee = replace selection (match Thunar
   unless report justifies otherwise).
9. Virtualized lists: only **currently laid-out / measured** items required
   for v1; document if off-screen virtual rows are skipped.

## Done when

1. Unit tests for selection math (range, toggle, marquee rect ∩ item bounds)
   where pure helpers exist.
2. Live grim: marquee over ≥3 files in list **and** grid (if grid visible).
3. Report + architect ACCEPT.

## Out of scope

- DnD of the selection (T051) — but selection API must expose paths for DnD.
- Keyboard-only range (Shift+arrows) — nice residual, not gate.

## Related

T048 · T006 batch rename · T051 DnD
