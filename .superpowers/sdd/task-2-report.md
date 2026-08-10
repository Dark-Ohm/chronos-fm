# T050 Task 2 - ExplorerPane Measurement and Drag Lifecycle

Status: implementation complete; no self-ACCEPT.

## Scope

- Added `ExplorerPane` state for the active marquee, measured item bounds,
  viewport/token state, exclusion bounds, and layout revisions.
- Added measurement and lifecycle APIs: `record_listing_viewport`,
  `record_item_bounds`, `record_marquee_exclusion`, `begin_marquee`,
  `update_marquee`, `marquee_rect`, `finish_marquee`, and `cancel_marquee`.
- Added lifecycle tests for plain replacement, Ctrl union with anchor
  preservation, below-threshold plain/Ctrl empty-space clicks, and stale-token
  cancellation.
- Kept pointer routing and drawing out of scope for Task 3.

## TDD Evidence

Claim: lifecycle behavior was specified by tests before production code.

Evidence: `cargo test -p chronos-fm-pages --lib marquee::tests::pane_` first
failed with the expected missing `ExplorerPane` lifecycle methods/fields
(`begin_marquee`, `update_marquee`, `finish_marquee`, `marquee_rect`,
`record_listing_viewport`, `record_item_bounds`, and `marquee`). After the
implementation, the same command exited 0 with `4 passed, 0 failed`.

Truth base: Chronos-FM.

## Verification

Claim: pure marquee geometry plus all Task 2 lifecycle tests pass.

Evidence: `cargo test -p chronos-fm-pages --lib marquee` exited 0 with
`13 passed, 0 failed`.

Truth base: Chronos-FM.

Claim: existing selection behavior remains intact.

Evidence:

- `cargo test -p chronos-fm-pages --lib selection_single_toggle_range_and_paths`
  exited 0 (`1 passed, 0 failed`).
- `cargo test -p chronos-fm-pages --lib selection_all_clear_and_arrow_navigation`
  exited 0 (`1 passed, 0 failed`).
- `cargo test -p chronos-fm-pages --lib apply_filter_resets_stale_selection`
  exited 0 (`1 passed, 0 failed`).
- `git diff --check` exited 0.

Truth base: Chronos-FM.

## Behavioral Notes

- Plain empty-space press clears selection immediately. Ctrl/Cmd preserves the
  snapshot. The rectangle remains hidden until movement reaches the 4 px
  helper threshold.
- Drag hit tests use the press-time hitbox snapshot and the clipped viewport
  rectangle. Token mismatch cancels the drag but retains the last live
  selection.
- Entry-order and item-size revisions invalidate stored measurements. View
  changes and column-resize starts cancel interaction state. Filter rebuilds
  cancel before clearing selection.
- `record_marquee_exclusion` is ready for Task 3 to register list headers and
  controls. Task 3 must call `cx.notify()` from pointer handlers after an
  accepted update; measurement callbacks intentionally do not notify to avoid
  repaint loops.

## Residual

No renderer or live-window evidence is claimed here. Task 3 must wire
`on_prepaint` measurements, pointer routing, scroll cancellation, and overlay
painting before visual verification.

## Rejection Fix Evidence

Claim: a changed geometry token immediately cancels an active drag, so a later
mouse-up cannot apply stale completion state.

Evidence: `state.rs:438-445` calls `cancel_marquee` before replacing the token
and clearing measurement state. Regression
`pane_viewport_token_change_cancels_before_mouse_up` (`marquee.rs:277-298`)
asserts that the rectangle is removed immediately and that a subsequent
`finish_marquee` cannot set anchor/active state.

Truth base: Chronos-FM.

Claim: the 4 px threshold latches once crossed.

Evidence: `MarqueeDrag::dragging` (`marquee.rs:45-57`) is set monotonically in
`update_marquee` (`state.rs:525-565`) and gates both rectangle visibility and
completion (`state.rs:568-590`). Regression
`pane_drag_threshold_latches_after_returning_below_four_pixels`
(`marquee.rs:301-320`) moves out, returns below four pixels, and releases;
selection is recomputed empty with no anchor/active mismatch.

Truth base: Chronos-FM.

Claim: async search replacement follows a revisioned, cancellation-safe
filtered-entry path even when row sizes are unchanged.

Evidence: `replace_filtered_entries` (`state.rs:728-744`) cancels, replaces,
bumps `entries_revision` on order change, and invalidates geometry.
`search.rs:70-90` uses it on both search success and failure; reload failure
uses it at `navigation.rs:73-82`. Regression
`pane_replacing_filtered_entries_cancels_and_revisions_measurements`
(`marquee.rs:323-347`) verifies the cancelled drag, cleared geometry, revision
bump, and unchanged item-size revision for equal-sized replacement rows.

Truth base: Chronos-FM.

Claim: opening listing overlays and pane focus loss cancel marquee state.

Evidence: properties/context-menu opening paths cancel at
`navigation.rs:277-335`; batch rename cancels before dialog creation at
`batch_rename.rs:236-243`; production pane construction retains GPUI's
pane-subtree `on_focus_out` subscription at `state.rs:198-213`. Regression
`pane_opening_listing_overlays_cancels_marquee` (`marquee.rs:349-381`) covers
properties plus directory and file context menus. The direct batch-dialog test
is not feasible in this pane-only harness because opening its focused `Input`
without the production `Root` panics in `gpui_component::Root::read`; the
production method is nevertheless covered by the explicit first statement
above.

Truth base: Chronos-FM / Source.

Claim: same-token control/header measurements cannot grow without bound.

Evidence: `marquee_exclusions` is now keyed by static renderer IDs and updated
with `BTreeMap::insert` (`state.rs:460-469`), replacing an element's previous
bounds per prepaint. Regression `pane_deduplicates_same_token_exclusions`
(`marquee.rs:383-400`) records the same key three times, asserts one entry, and
verifies that the latest bounds replace the earlier value.

Truth base: Chronos-FM.

## Takeover Verification And Self-Review

The takeover audit preserved the inherited implementation where it matched the
stamped design and tightened two proof gaps: the async replacement regression
now demonstrates an entry-revision bump while item-size revision is unchanged,
and exclusion deduplication verifies replacement rather than only map length.
Pane focus cancellation uses `Context::on_focus_out`, the Source API that covers
the pane handle and its descendants; `on_blur` only covers direct focus on the
handle.

Commands and results:

- `cargo test -p chronos-fm-pages --lib marquee` exited 0: `18 passed; 0 failed`.
- `cargo test -p chronos-fm-pages --lib selection_single_toggle_range_and_paths`
  exited 0: `1 passed; 0 failed`.
- `cargo test -p chronos-fm-pages --lib selection_all_clear_and_arrow_navigation`
  exited 0: `1 passed; 0 failed`.
- `cargo test -p chronos-fm-pages --lib apply_filter_resets_stale_selection`
  exited 0: `1 passed; 0 failed`.
- `git diff --check` exited 0.

Self-review:

- Geometry-token replacement cancels before mutating the token, so mouse-up
  cannot complete stale state; the last live selection is explicitly retained.
- The threshold is monotonic for the press epoch and selection is recomputed
  even after the pointer returns below 4 px, keeping completion state coherent.
- All direct async assignments to `filtered_entries` in search and reload error
  paths now use the revisioned replacement helper; ordinary unchanged-order
  filter rebuilds do not spuriously bump `entries_revision`.
- Properties, both context-menu variants, batch rename, and pane-subtree focus
  loss cancel only interaction state and do not alter file-operation selection
  APIs.
- A direct focus-event regression was attempted with a Root-wrapped test pane,
  but this GPUI test window did not dispatch `WindowFocusEvent` listeners for
  the synthetic descendant-to-sibling focus transition, including a control
  listener registered directly on `Window`. The retained evidence is the
  production `on_focus_out` subscription and Source's documented subtree
  semantics; no false-green test was kept.
- Renderer measurement callbacks, pointer routing, scroll cancellation, and
  overlay painting remain Task 3 by contract.

Claim: the rejection fixes and required selection regressions pass.

Evidence:

- `cargo test -p chronos-fm-pages --lib marquee` exited 0 (`18 passed, 0 failed`).
- `cargo test -p chronos-fm-pages --lib selection_single_toggle_range_and_paths`
  exited 0 (`1 passed, 0 failed`).
- `cargo test -p chronos-fm-pages --lib selection_all_clear_and_arrow_navigation`
  exited 0 (`1 passed, 0 failed`).
- `cargo test -p chronos-fm-pages --lib apply_filter_resets_stale_selection`
  exited 0 (`1 passed, 0 failed`).

Truth base: Chronos-FM.
