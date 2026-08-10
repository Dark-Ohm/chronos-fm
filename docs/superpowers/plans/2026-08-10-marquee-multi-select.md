# Marquee Multi-Select (T050) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add standard empty-space marquee selection to list and grid layouts while preserving the existing single `ExplorerPane::selection` model and modifier-click behavior.

**Architecture:** A focused `explorer/marquee.rs` module owns pure rectangle and selection math plus drag/measurement records. List and grid renderers report real window-coordinate bounds with `on_prepaint`; the shared listing surface owns pointer routing and paints the clipped overlay. A geometry token freezes the press-epoch hitboxes and cancels safely when scrolling or layout changes make them stale.

**Tech Stack:** Rust, GPUI/gpui-component `ElementExt::on_prepaint`, `VisualTestContext` mouse simulation, existing Chronos-FM explorer state and theme.

## Global Constraints

- Use only `ExplorerPane::selection` and `selected_paths()`; do not add a second selection model.
- All marquee/item/viewport geometry is stored in GPUI window coordinates.
- Linux Ctrl is additive; preserve the existing `modifiers.platform || modifiers.control` dual binding.
- Plain empty click clears selection; Ctrl empty click preserves it.
- Marquee starts only from empty listing space, never header/row/tile/input/control chrome.
- Plain marquee replaces selection; Ctrl marquee unions with the press snapshot.
- Drag threshold is 4 px in either axis; edge/corner contact counts as intersection.
- Plain completion uses min/max hit indices for anchor/active; additive preserves an existing anchor.
- Only currently measured list/grid entries participate; no off-screen approximation or auto-scroll.
- A geometry-token change cancels drag, removes the overlay, keeps the last live selection, and excludes stale bounds.
- DnD, Shift+Arrow, spring-loaded folders, and unrelated cleanup remain out of scope.
- Do not self-ACCEPT; provide Claim -> Evidence for Architect review.

---

### Task 1: Pure Marquee Geometry and Selection Math

**Files:**
- Create: `crates/chronos-fm-pages/src/explorer/marquee.rs`
- Modify: `crates/chronos-fm-pages/src/explorer.rs:1-30`

**Interfaces:**
- Consumes: GPUI `Point<Pixels>` and `Bounds<Pixels>`.
- Produces: `MarqueeDrag`, `MeasuredItem`, `GeometryToken`, `normalized_rect`, `intersects_closed`, `past_threshold`, and `selection_for_hits` for later state/render tasks.

- [ ] **Step 1: Write failing pure tests in `marquee.rs`**

Cover normalization in all four directions, closed-edge intersection, the 4 px axis threshold, replace/union semantics, and stable min/max completion:

```rust
#[test]
fn touching_edges_intersect() {
    let marquee = Bounds::new(point(px(0.), px(0.)), size(px(10.), px(10.)));
    let item = Bounds::new(point(px(10.), px(10.)), size(px(5.), px(5.)));
    assert!(intersects_closed(marquee, item));
}

#[test]
fn ctrl_hits_union_with_press_selection() {
    let base = BTreeSet::from([1, 4]);
    assert_eq!(selection_for_hits(&base, [2, 4], true), BTreeSet::from([1, 2, 4]));
}
```

- [ ] **Step 2: Run the new test target and observe the red state**

Run: `cargo test -p chronos-fm-pages --lib marquee`

Expected: compile failure because the module/types/helpers do not exist yet.

- [ ] **Step 3: Implement the minimal pure module**

Use concrete state with a press-epoch hitbox snapshot:

```rust
pub(crate) const DRAG_THRESHOLD: Pixels = px(4.0);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeometryToken {
    pub view_mode: ViewMode,
    pub entries_revision: u64,
    pub viewport: Bounds<Pixels>,
    pub scroll_offset: Point<Pixels>,
    pub item_sizes_revision: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct MeasuredItem {
    pub index: usize,
    pub bounds: Bounds<Pixels>,
}

#[derive(Clone, Debug)]
pub(crate) struct MarqueeDrag {
    pub start: Point<Pixels>,
    pub current: Point<Pixels>,
    pub token: GeometryToken,
    pub hitboxes: Vec<MeasuredItem>,
    pub base_selection: BTreeSet<usize>,
    pub prior_anchor: Option<usize>,
    pub prior_active: Option<usize>,
    pub additive: bool,
}
```

`normalized_rect` uses component-wise min/max. `intersects_closed` compares
inclusive left/right/top/bottom edges. `past_threshold` is true when either
absolute axis delta is at least `DRAG_THRESHOLD`. `selection_for_hits` returns
hits alone for plain drag and `base_selection ∪ hits` for additive drag.

- [ ] **Step 4: Run focused tests**

Run: `cargo test -p chronos-fm-pages --lib marquee`

Expected: all pure marquee tests pass.

- [ ] **Step 5: Commit the pure unit**

```bash
git add crates/chronos-fm-pages/src/explorer.rs crates/chronos-fm-pages/src/explorer/marquee.rs
git commit -m "feat(explorer): add marquee selection geometry"
```

### Task 2: ExplorerPane Measurement and Drag Lifecycle

**Files:**
- Modify: `crates/chronos-fm-pages/src/explorer/state.rs:21-103,194-267,332-389`
- Modify: `crates/chronos-fm-pages/src/explorer/list_setup.rs` at reload/filter/item-size revision points.
- Test: `crates/chronos-fm-pages/src/explorer/marquee.rs`

**Interfaces:**
- Consumes: Task 1 geometry types/helpers.
- Produces: `record_listing_viewport`, `record_item_bounds`, `begin_marquee`, `update_marquee`, `finish_marquee`, `cancel_marquee`, and `marquee_rect` on `ExplorerPane`.

- [ ] **Step 1: Add failing lifecycle tests**

Create a test pane with measured indices 0..3 and assert:

```rust
pane.begin_marquee(point(px(0.), px(0.)), Modifiers::default());
pane.update_marquee(point(px(40.), px(40.)));
assert_eq!(pane.selection, BTreeSet::from([0, 1, 2]));
pane.finish_marquee();
assert_eq!(pane.selection_anchor, Some(0));
assert_eq!(pane.active_index, Some(2));
```

Add separate tests for Ctrl union/anchor preservation, sub-threshold click,
empty plain/Ctrl click, and token mismatch cancellation retaining the last
live selection.

- [ ] **Step 2: Run lifecycle tests and verify failure**

Run: `cargo test -p chronos-fm-pages --lib marquee::tests::pane_`

Expected: compile failures for missing `ExplorerPane` lifecycle methods/fields.

- [ ] **Step 3: Add measurement/lifecycle state**

Add to `ExplorerPane`:

```rust
pub(crate) marquee: Option<MarqueeDrag>,
pub(crate) measured_items: BTreeMap<usize, Bounds<Pixels>>,
pub(crate) listing_viewport: Option<Bounds<Pixels>>,
pub(crate) geometry_token: Option<GeometryToken>,
pub(crate) entries_revision: u64,
pub(crate) item_sizes_revision: u64,
```

Initialize them in `ExplorerPane::new`. Increment revisions only when filtered
entry order or measured item sizes change, not on ordinary marquee repaint.
`record_listing_viewport` updates the token from view mode, viewport, current
scroll offset, and revisions. A token change clears the previous measurement
map before the new prepaint callbacks populate it. `record_item_bounds` stores
only bounds produced for the current token.

`begin_marquee` first rejects points outside the viewport or inside a measured
item/header/control exclusion. It snapshots token/hitboxes/base selection.
Plain begins by calling `clear_selection`; additive keeps state.

`update_marquee` cancels when the live token differs. Otherwise it clips the
normalized rectangle, computes intersecting indices, writes live selection,
and notifies. `finish_marquee` applies the min/max anchor/active rules from the
spec. `cancel_marquee` drops only drag state, retaining the last selection.

- [ ] **Step 4: Cancel from state-invalidating operations**

Call `cancel_marquee` when reload/filter/order, view mode, dialog opening, or
focus-loss paths invalidate interactive listing state. Do not alter file-op or
`selected_paths()` behavior.

- [ ] **Step 5: Run pure and existing selection tests**

Run:

```bash
cargo test -p chronos-fm-pages --lib marquee
cargo test -p chronos-fm-pages --lib selection_single_toggle_range_and_paths
cargo test -p chronos-fm-pages --lib selection_all_clear_and_arrow_navigation
cargo test -p chronos-fm-pages --lib apply_filter_resets_stale_selection
```

Expected: marquee tests pass; existing selection tests remain green.

- [ ] **Step 6: Commit lifecycle state**

```bash
git add crates/chronos-fm-pages/src/explorer/state.rs crates/chronos-fm-pages/src/explorer/list_setup.rs crates/chronos-fm-pages/src/explorer/marquee.rs
git commit -m "feat(explorer): manage marquee drag state"
```

### Task 3: Measured List/Grid Routing and Overlay

**Files:**
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing.rs:1-65`
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing/list.rs:12-104`
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing/row.rs` at the outer row element.
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing/grid.rs:12-180`

**Interfaces:**
- Consumes: Task 2 measurement and pointer lifecycle methods.
- Produces: actual list/grid item measurements, background drag routing, and marquee visual overlay.

- [ ] **Step 1: Write failing GPUI routing tests before renderer changes**

Add a Root-wrapped `VisualTestContext` harness that paints a fixed-size explorer
window with at least six files. Read the measured viewport/items from the pane,
choose an empty start point and an end point spanning at least three bounds,
then dispatch:

```rust
vcx.simulate_mouse_down(root.into(), start, MouseButton::Left, Modifiers::default());
vcx.simulate_mouse_move(root.into(), end, Some(MouseButton::Left), Modifiers::default());
assert!(pane.read(cx).selection.len() >= 3);
vcx.simulate_mouse_up(root.into(), end, MouseButton::Left, Modifiers::default());
assert!(pane.read(cx).marquee.is_none());
```

Repeat in `ViewMode::Grid`, with Ctrl modifiers for additive behavior, and add
negative cases using a measured row/tile/header point.

- [ ] **Step 2: Run routing tests and verify red state**

Run: `cargo test -p chronos-fm-pages --lib marquee_routing`

Expected: no measurements and no drag selection before renderer integration.

- [ ] **Step 3: Record viewport and item bounds with production APIs**

Import `gpui_component::ElementExt`. Attach `on_prepaint` to the shared listing
viewport and to the outer list-row/grid-tile elements. Capture `cx.entity()` and
update the pane without `cx.notify()` from measurement callbacks:

```rust
.on_prepaint(move |bounds, _, cx| {
    pane.update(cx, |pane, _| pane.record_item_bounds(ix, bounds, token.clone()));
})
```

List records only rows emitted by `v_virtual_list`'s `visible_range`. Grid
records only tiles whose prepaint callback ran. Header bounds are recorded as
an exclusion so its empty pixels cannot begin marquee.

- [ ] **Step 4: Add shared pointer routing**

On the listing viewport install Left `on_mouse_down`, `on_mouse_move`,
`on_mouse_up`, and `on_mouse_up_out`. Start only when `begin_marquee` accepts
the empty-space position. Move only while the left button is pressed. Up/out
always calls `finish_marquee`. Add a scroll-wheel/layout-token hook that calls
`cancel_marquee` before stale hitboxes can be reused.

- [ ] **Step 5: Paint the clipped overlay**

When `marquee_rect()` is `Some`, append an absolute, pointer-transparent child:

```rust
div()
    .absolute()
    .left(rect.origin.x - viewport.origin.x)
    .top(rect.origin.y - viewport.origin.y)
    .w(rect.size.width)
    .h(rect.size.height)
    .border_1()
    .border_dashed()
    .border_color(theme::accent(cx))
    .bg(theme::accent(cx).opacity(0.10))
```

Keep row/tile click handlers unchanged except for measurement instrumentation;
their existing Shift > Ctrl/Cmd > plain precedence remains authoritative.

- [ ] **Step 6: Run renderer and routing tests**

Run:

```bash
cargo test -p chronos-fm-pages --lib marquee
cargo test -p chronos-fm-pages --lib marquee_routing
cargo test -p chronos-fm-pages --lib explorer::tests
```

Expected: list and grid drags select at least three measured entries; negative
targets do not start marquee; existing explorer tests pass.

- [ ] **Step 7: Commit renderer integration**

```bash
git add crates/chronos-fm-pages/src/explorer/view/listing.rs crates/chronos-fm-pages/src/explorer/view/listing/list.rs crates/chronos-fm-pages/src/explorer/view/listing/row.rs crates/chronos-fm-pages/src/explorer/view/listing/grid.rs crates/chronos-fm-pages/src/explorer/marquee.rs
git commit -m "feat(explorer): add list and grid marquee selection"
```

### Task 4: Verification, Live Evidence, and Executor Report

**Files:**
- Create: `docs/orchestration/tasks/report/T050-marquee-multi-select-report.md`
- Create: `docs/orchestration/tasks/report-log/T050-marquee-list.png`
- Create: `docs/orchestration/tasks/report-log/T050-marquee-grid.png`
- Modify only if tests expose a T050 defect: files listed in Tasks 1-3.

**Interfaces:**
- Consumes: completed T050 behavior and test harness.
- Produces: reproducible Claim -> Evidence report for Architect review.

- [ ] **Step 1: Format and inspect scoped diff**

Run:

```bash
cargo fmt -p chronos-fm-pages
git diff --check -- crates/chronos-fm-pages/src/explorer.rs crates/chronos-fm-pages/src/explorer/marquee.rs crates/chronos-fm-pages/src/explorer/state.rs crates/chronos-fm-pages/src/explorer/list_setup.rs crates/chronos-fm-pages/src/explorer/view/listing.rs crates/chronos-fm-pages/src/explorer/view/listing/list.rs crates/chronos-fm-pages/src/explorer/view/listing/row.rs crates/chronos-fm-pages/src/explorer/view/listing/grid.rs
```

Restore formatter-only changes outside this exact T050 list; do not clean the
unrelated worktree.

- [ ] **Step 2: Run automated verification**

Run:

```bash
cargo test -p chronos-fm-pages --lib marquee
cargo test -p chronos-fm-pages --lib marquee_routing
cargo test -p chronos-fm-pages --lib
cargo test --workspace
cargo build --release -p chronos-fm
```

Expected: all T050/pages/workspace tests pass and release build exits 0. If an
unrelated pre-existing release failure remains, record its exact diagnostic and
do not silently expand T050 scope.

- [ ] **Step 3: Capture live list and grid evidence**

Launch `target/release/chronos-fm` in an isolated directory containing at least
six real files. Confirm `class=chronos-fm`. Use compositor-targeted pointer
automation to drag from empty space across at least three items in List, capture
`T050-marquee-list.png`, switch to Grid, repeat, and capture
`T050-marquee-grid.png`. Keep the pointer pressed when capturing so the dashed
rectangle and live highlights are both visible.

Record exact launch, compositor automation, screenshot, and filesystem commands
in the report. Do not infer keyboard/pointer causality from screenshots alone.

- [ ] **Step 4: Write Claim -> Evidence report**

Document:

- Existing Click/Ctrl/Shift semantics and Shift-anchor preservation.
- Plain/Ctrl marquee selection behavior.
- Window-coordinate measurements and press-epoch cancellation.
- Visible-only virtual list limitation.
- `selection`/`selected_paths()` continuity for T051.
- Test commands with pass/fail counts, release result, exact live commands, and
  both grim paths.

Set status to `IMPLEMENTED - awaiting Architect review`; never self-ACCEPT.

- [ ] **Step 5: Request independent code review**

Ask the reviewer to check spec compliance, stale-bound safety, pointer routing,
input/chrome isolation, selection anchor rules, test strength, and report claim
accuracy. Resolve findings and rerun affected verification.

- [ ] **Step 6: Commit only T050 artifacts**

```bash
git add crates/chronos-fm-pages/src/explorer.rs crates/chronos-fm-pages/src/explorer/marquee.rs crates/chronos-fm-pages/src/explorer/state.rs crates/chronos-fm-pages/src/explorer/list_setup.rs crates/chronos-fm-pages/src/explorer/view/listing.rs crates/chronos-fm-pages/src/explorer/view/listing/list.rs crates/chronos-fm-pages/src/explorer/view/listing/row.rs crates/chronos-fm-pages/src/explorer/view/listing/grid.rs docs/orchestration/tasks/report/T050-marquee-multi-select-report.md docs/orchestration/tasks/report-log/T050-marquee-list.png docs/orchestration/tasks/report-log/T050-marquee-grid.png
git commit -m "feat(explorer): add T050 marquee multi-select"
```

Do not move the ticket to done; Architect owns ACCEPT and task hygiene.
