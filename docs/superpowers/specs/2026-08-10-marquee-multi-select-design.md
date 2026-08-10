# Marquee Multi-Select (T050) - Design Document

**Date:** 2026-08-10  
**Ticket:** `docs/orchestration/tasks/active/T050-marquee-multi-select.md`  
**Project:** Chronos-FM (`crates/chronos-fm-pages/src/explorer/`)  
**Scope:** Mouse modifier selection and marquee selection in list and grid layouts.

**Design status:** Architect **APPROVE — IMPLEMENT GO** (2026-08-10). Write plan, then implement.
separate Architect stamp after this file is committed.


> ## ✅ ARCHITECT VERDICT (2026-08-10): **APPROVE — IMPLEMENT GO**
>
> Spec `a228319` covers T050 Must and all six non-negotiables from design
> review:
>
> | Constraint | Spec § |
> |------------|--------|
> | Window coords + viewport clip | §3 |
> | Ctrl additive (Linux); click dual-bind | §1, §4 |
> | Click/Ctrl/Shift preserved; marquee ≠ break Shift-anchor | §1, §6 |
> | min/max `filtered_entries` index for anchor/active | §4 end |
> | Epoch bump → cancel marquee, keep last live selection | §3, §8 |
> | Sole selection model for T051 (`selection` / `selected_paths`) | §1, §9 |
>
> Approach **1** (measured `on_prepaint` bounds) accepted; pure-index geometry
> and full listing Element rejected for T050.
>
> Edge-touch closed rects, 4 px threshold, empty-space start only, list
> virtualized-visible / grid laid-out tiles — **APPROVE**.
>
> **Next:** implementation plan → execute → report (no self-ACCEPT). Live
> grims list **and** grid over ≥3 entries required for ACCEPT.


## 1. Goal

Complete standard file-manager mouse selection without introducing a second
selection model. Existing batch operations and the future T051 drag-and-drop
flow continue to consume `ExplorerPane::selection` and `selected_paths()`.

T050 adds a measured-bounds marquee to both listing layouts and preserves the
existing click semantics:

- Plain click selects one entry and clears the previous selection.
- Ctrl+click toggles the clicked entry on Linux. The existing
  `platform || control` rule continues to provide Cmd on platforms that use it.
- Shift+click selects the inclusive `filtered_entries` range from the existing
  anchor. Repeated Shift+click operations do not move that anchor.
- Plain empty-space click clears selection, anchor, and active row.
- Ctrl+empty-space click preserves selection, anchor, and active row.

Grid visual order is `filtered_entries` index order. This is the same order
used by list rows and `select_range_to`.

## 2. Architecture

Add `explorer/marquee.rs` as the owner of marquee geometry and drag state. It
contains pure helpers for rectangle normalization, intersection, threshold
testing, and replace/additive selection calculation. Listing renderers provide
measurements and pointer events; they do not duplicate selection math.

`ExplorerPane` remains the sole selection owner. It gains:

- The current marquee drag, if any.
- The most recent measured listing viewport and item hitboxes.
- A layout token used to reject stale measurements.

List rows and grid tiles record their actual `Bounds<Pixels>` through
`on_prepaint`. The listing viewport records its bounds through the same API.
The list records only virtual rows produced for the current visible range.
Grid records every tile laid out in its scroll viewport. No index-to-position
geometry is inferred.

## 3. Coordinate Space and Layout Token

Mouse event positions and `on_prepaint` bounds use GPUI window coordinates.
The marquee rectangle, listing viewport, and item hitboxes are therefore stored
and intersected in that one coordinate space. The marquee is clipped to the
recorded listing viewport before hit-testing and painting.

The layout token represents geometry-relevant state:

- View mode.
- Filtered entry order/version.
- Listing viewport bounds.
- List/grid scroll offset.
- Measured item-size generation.

Measurements are tagged with the token under which they were produced. A drag
copies the current token and currently measured hitboxes at press time. Only
that press-epoch snapshot participates in v1 hit-testing.

Scrolling or another geometry change during a drag changes the layout token.
The next event cancels the marquee, removes its rectangle, and keeps the last
live selection already shown. It never reuses stale hitboxes. T050 does not
auto-scroll while marquee dragging. A test must cover token invalidation.

## 4. Pointer Flow

Marquee starts only from a left-button press inside empty listing space. Header
chrome, a list row, a grid tile, rename/search inputs, context menus, column
resize handles, and other controls must not start it. The surface handler uses
the measured item/header bounds to reject non-empty targets rather than relying
on fragile event-order assumptions.

At press:

1. Capture start/current position, modifiers, layout token, measured hitboxes,
   prior selection, prior anchor, and prior active row.
2. For a plain press, clear selection immediately. For Ctrl, preserve it.
3. Do not show the rectangle until movement reaches 4 px in either axis.

During movement after the threshold:

- Normalize the rectangle for every drag direction and clip it to the viewport.
- Intersect it with the press-epoch hitboxes.
- Plain marquee replaces selection with the hit indices.
- Ctrl+marquee unions hit indices with the selection snapshot taken at press.
- Update highlights and the dashed rectangle live.

Intersection uses closed rectangle edges: touching an item edge or corner
counts as a hit. Zero-area movement below the threshold never selects items.

At release:

- Movement below threshold is an empty-space click: plain remains cleared and
  Ctrl remains unchanged.
- For plain marquee, anchor becomes the minimum hit index and active becomes the
  maximum hit index. If there are no hits, both become `None`.
- For additive marquee, preserve the prior anchor when it exists. If it does
  not exist, use the minimum hit index. Active becomes the maximum new hit when
  there is one; otherwise preserve the prior active row.
- Remove the drag state and rectangle.

Minimum/maximum refer to `filtered_entries` index order, which is also visual
reading order for the wrapped grid.

## 5. Rendering

Render the marquee as an absolute overlay inside the listing viewport, above
rows/tiles but below dialogs and context menus. It uses:

- `border_dashed()` with the current accent color.
- A one-pixel border.
- A low-opacity accent fill.
- No rounded card treatment or animation.

The overlay must not accept pointer events or alter listing layout. Selection
highlighting continues to use the existing `is_selected` rendering paths.

## 6. Existing Click Selection

Both list rows and grid tiles must retain or complete the same modifier logic:

- Shift takes precedence over Ctrl/Cmd and calls `select_range_to`.
- Ctrl/Cmd toggles through `toggle_select`.
- Plain click calls `select_single`.

Marquee does not replace these methods. Plain marquee establishes a stable
min-index anchor, and additive marquee preserves an existing anchor, so a
subsequent Shift+click continues through `select_range_to` without special
cases.

## 7. Testing and Evidence

Pure tests in or beside `marquee.rs` cover:

- Rectangle normalization in all drag directions.
- Edge-touch and overlap intersection behavior.
- The 4 px threshold.
- Plain replace and Ctrl-additive selection.
- Minimum/maximum anchor and active-row rules.
- Additive preservation of an existing Shift anchor.
- Layout-token mismatch cancellation and stale-bound exclusion.

GPUI tests cover list and grid routing:

- Plain, Ctrl, and Shift clicks retain the required behavior.
- Empty press starts marquee; row/tile/header/control presses do not.
- Dragging across at least three measured entries updates selection live.
- Mouse-up and mouse-up-out finish the drag and remove the overlay.

The executor must run focused tests, the pages/workspace suites, and the release
build. Runtime evidence requires the release `class=chronos-fm` window and live
grims showing a marquee over at least three entries in list and grid views.
The report states that virtualized off-screen list rows are intentionally
excluded. The executor reports Claim -> Evidence and does not self-ACCEPT.

## 8. Failure Handling

Missing measurements are skipped rather than approximated. A stale token
cancels the active marquee. Pointer release outside the listing is handled so
drag state cannot remain stuck. Directory reload, filter/order change, view-mode
switch, dialog opening, or pane loss of focus also cancels marquee state without
performing file operations.

## 9. Non-Goals

- Drag-and-drop of selected paths (T051).
- External drag-and-drop (T052).
- Keyboard Shift+Arrow range extension.
- Marquee auto-scroll or off-screen virtual-row selection.
- Spring-loaded folders.
- A new selection collection separate from `ExplorerPane::selection`.
- Unrelated listing or shared Source refactors.
