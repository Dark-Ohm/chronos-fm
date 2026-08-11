# T051 - In-app drag and drop report (Tasks 4-5)


## Architect review (2026-08-11)

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED**
>
> Independent review: Approved (no Critical). Architect closed the sole
> Important (docs) by adding first-class Claim→Evidence for unselected
> normalization and background completion, and renaming two misleading tests.
>
> ### Explicit sign-off on executor caveats
> 1. **Pending second-drop test** calling `can_accept_listing_cwd_drop` /
>    `begin_file_drop` after a real first gesture is **accepted** for v1:
>    those are the exact production methods the second `on_drop` would invoke;
>    chaining two full GPUI drag gestures is harness-fragile and not required
>    for the concurrency contract.
> 2. **No dedicated live grim of cursor `DragCopy`** is **accepted**: live
>    Ctrl-copy FS evidence + post-state grim (`T051-dnd-copy-after.png`) and
>    unit/e2e cursor assertions at drop-time modifier change are sufficient;
>    optional residual only.
>
> ### Minor notes (non-blocking, addressed or waived)
> - Test names `*_via_breadcrumb` / `*_own_descendant` renamed to match actual
>   targets (`cwd_surface` / `onto_self`).
> - `cargo fmt -p chronos-fm-services` not re-proven in Task 5; no Task 4–5
>   services code change beyond Task 1 — waive for this close.



**Status:** IMPLEMENTED - awaiting Architect review. Executor does not self-ACCEPT.

Scope of this report: Task 4 (split-pane end-to-end routing and failure
hardening) and Task 5 (verification, release evidence, executor report) of
`docs/superpowers/plans/2026-08-10-dnd-in-app.md`. Tasks 1-3 were implemented
and reviewed PASS on this branch before this agent started
(`docs/orchestration/tasks/report-log/T051-task3-review.md`); they were not
touched, re-implemented, or re-tested beyond the shared regression matrix
below.

## Outcome

Task 4 added nine production-tree tests against a real, Root-wrapped
`ExplorerPage` split into two panes rooted at separate tempdirs, driving
actual List row hitboxes (mouse down/move/up, modifier changes) rather than
calling pane methods directly. All nine passed against the existing Task 1-3
implementation with **no production code changes** — evidence did not
surface a defect, so no hardening fix was required. Task 4 committed only
`crates/chronos-fm-pages/src/explorer/dnd.rs` (test-only diff).

Task 5 ran the full plan-mandated verification matrix (all green), built the
release binary, and captured live evidence under Hyprland with the actual
release binary, class `chronos-fm`, a real two-pane local fixture, and
`ydotool`-driven mouse/keyboard input: a held multi-item drag with a
highlighted destination, a completed cross-pane multi-file+directory Move,
and a completed Ctrl-at-drop Copy that produced a `unique_name` duplicate
while leaving the source and the pre-existing destination file untouched.

## Claims and evidence

Claim: A held drag from a real List row carries the full current selection
(here 4 items: 3 files + 1 directory) and shows a compact preview with a
count badge; the other pane's empty listing space is a valid Move target and
receives a highlighted-border affordance.

Evidence: `crates/chronos-fm-pages/src/explorer/dnd.rs:1598`
(`e2e_cross_pane_move_reloads_both_panes_and_selects_destination`) drives a
real mouse-down/move/up sequence across a two-pane `ExplorerPage`, asserts
`active_drag_cursor` is `ClosedHand` over the other pane's empty cwd, and
asserts the `file-drag-preview-count` debug element exists. `cargo test -p
chronos-fm-pages --lib dnd` exited 0, 31 passed, 0 failed (22 baseline +
9 new). Live: `docs/orchestration/tasks/report-log/T051-dnd-held.png` shows a
`gamma.txt` + `4 items` preview and an accent-bordered destination pane,
captured by `grim -g "$(hyprctl clients -j | jq ...)"` immediately after
`ydotool click 0x40` (down) + three `ydotool mousemove -a` steps, while the
mouse button was still held (release had not yet been sent).

Truth base: Chronos-FM | runtime.

Claim: Releasing without Ctrl performs a Move: sources disappear from the
origin pane, appear at the destination, both panes reload, and the
destination pane selects exactly the successful destination paths (spec §6).

Evidence: the same test (`dnd.rs:1598`) asserts `!a.exists()` /
`!b.exists()` / `!c.exists()` and `right.join(name).exists()` for all three
files after `settle_drop`, `left_pane` selection is empty, and
`right_pane.selected_paths()` equals the three destination paths sorted.
Same-pane and cross-pane pane-level equivalents (`cross_pane_move_...`,
`same_pane_folder_move_...`) remain green in the 31/31 `dnd` result. Live:
`docs/orchestration/tasks/report-log/T051-dnd-move-after.png` shows the
source pane reduced to 1 item (`dup.txt`) and the destination pane showing 5
items (`archive`, `alpha.txt`, `beta.txt`, `dup.txt`, `gamma.txt`) with the
four moved entries highlighted as the new selection. Filesystem: `find` and
`stat` before/after (recorded in this session) show
`fixture/source/{alpha.txt,beta.txt,gamma.txt,archive/}` gone and present
under `fixture/destination/` with unchanged byte sizes (11, 10, 11, 15
bytes respectively); `fixture/destination/dup.txt` (pre-existing, 39 bytes)
was untouched. The release binary's own log recorded the completion:
`chronos_fm_pages::explorer::dnd: file drop filesystem work completed
success_count=4 failure_count=0 mode=Move`.

Truth base: Chronos-FM | runtime | filesystem observation.

Claim: Ctrl is resolved from `window.modifiers()` at drop time, not at press
time: a drag started with no modifier and pressed to Ctrl only while
hovering the target still performs a Copy at release.

Evidence: `crates/chronos-fm-pages/src/explorer/dnd.rs:1671`
(`e2e_ctrl_evaluated_at_drop_time_copies_across_panes`) starts a drag with
`Modifiers::default()`, asserts the cursor reads `ClosedHand` (Move) while
unmodified, then moves the mouse again with `control: true` over the same
target and asserts the cursor switches to `DragCopy` before drop; the drop
itself confirms Copy semantics (source retained, destination has the
duplicate, source pane keeps its selection per spec §6 cross-pane Copy). The
production `drop_mode` function this depends on lives at `dnd.rs:200` and is
read from `window.modifiers()` at the `on_drop` callback in
`crates/chronos-fm-pages/src/explorer/view/listing.rs:173` /
`view/header.rs:166` / `view/listing/row.rs:464` /
`view/listing/grid.rs:273` - the same call site `can_drop` uses, so
highlight and execution cannot disagree. Live: the drag on `dup.txt` began
plain (mouse-down + move with default modifiers), Ctrl was pressed only
after the pointer was already over the destination pane
(`docs/orchestration/tasks/report-log/T051-dnd-held.png` shows the preceding
plain-Move held state for the multi-item drag; the Ctrl-copy sequence is a
second, separate drag - see below), and `T051-dnd-copy-after.png` shows the
resulting Copy.

Truth base: Chronos-FM | Source (`gpui`'s `window.modifiers()` /
`cx.active_drag_cursor_style()`) | runtime.

Claim: Ctrl-at-drop Copy never overwrites an occupied destination name; it
always produces a `unique_name` duplicate, and the source remains.

Evidence: pure `unique_name` behavior is proven at
`crates/chronos-fm-services/src/fs/ops.rs:140` and covered by
`fs::ops::tests::transfer_paths_copy_keeps_both_with_unique_name`
(`cargo test -p chronos-fm-services --lib transfer_paths` exited 0, 6
passed). At the DnD layer,
`same_pane_cwd_copy_selects_unique_name` (`dnd.rs:1375`) and the new
`e2e_same_parent_ctrl_copy_via_cwd_surface_creates_unique_duplicate`
(`dnd.rs:1826`) exercise the same path through `begin_file_drop` ->
`transfer_paths`. Live: source `fixture/source/dup.txt` (content `dup
source version`, 19 bytes) was dragged onto the destination pane, which
already held its own pre-existing `dup.txt` (content `dup destination
version (pre-existing)`, 39 bytes); after the Ctrl-release, `find` shows
`fixture/destination/dup (2).txt` (19 bytes, content `dup source version` -
verified byte-for-byte via `cat`) alongside the *unmodified*
`fixture/destination/dup.txt` (still 39 bytes, still the pre-existing
content), and `fixture/source/dup.txt` still exists (19 bytes). The release
log recorded: `chronos_fm_pages::explorer::dnd: file drop filesystem work
completed success_count=1 failure_count=0 mode=Copy`. Screenshot
`docs/orchestration/tasks/report-log/T051-dnd-copy-after.png` shows both
`dup.txt` and the highlighted new `dup (2).txt` in the destination pane, and
the source pane still listing its single `dup.txt`.

Truth base: Chronos-FM | runtime | filesystem observation.

Claim: A directory dragged onto itself (or the item's own descendant target)
is rejected, and a rejected item target never falls through to the pane's
outer cwd drop (self/descendant and same-parent no-op cases both).

Evidence: pure rejection is proven at `dnd.rs:216`
(`validate_drop`) and its unit tests
(`validation_rejects_self_descendant_and_same_parent_move`,
`validation_rejects_entire_mixed_payload`). The no-fall-through contract is
implemented by `can_accept_listing_cwd_drop` at `dnd.rs:279`, which requires
the pointer to be inside the listing viewport **and** outside every
`measured_items` bound before delegating to `can_accept_file_drop` - so an
invalid item target under the pointer can never be reinterpreted as a valid
cwd drop. Task 4 adds two production-tree proofs of this exact contract in
the split-pane tree:
`e2e_folder_dragged_onto_self_does_not_fall_through_to_cwd`
(`dnd.rs:1738`) drops a directory on itself and asserts
`CursorStyle::OperationNotAllowed`, no active drag survives mouse-up, and no
`container (2)` appears at the pane cwd; and
`e2e_same_parent_move_via_cwd_surface_performs_no_filesystem_operation`
(`dnd.rs:1784`) drops a selected file onto its own pane's cwd surface
(same-parent Move, a no-op per spec §5) and asserts no `stay (2).txt` is
created. `dnd_routing_items_rename_header_and_provider_reject`
(`dnd.rs:1144`, pre-existing from Task 3) covers the equivalent file-row and
provider-pane cases. All pass in the 31/31 `dnd` and 6/6 `dnd_routing`
results.

Truth base: Chronos-FM.

Claim: A destination pane with one drop already pending refuses a second
drop rather than starting a second background job or corrupting the first.

Evidence: `pending_is_synchronous_and_rejects_a_second_drop` (`dnd.rs:1232`,
pre-existing) proves `begin_file_drop` sets `drop_pending` synchronously
before spawning and rejects re-entry. Task 4 adds
`e2e_pending_target_refuses_a_second_production_drop` (`dnd.rs:1990`), which
performs a real production drop via `dispatch_mouse_up` (the exact
`MouseUpEvent` production `on_drop` handlers consume), confirms
`drop_pending` is still true, then calls the same
`can_accept_listing_cwd_drop`/`begin_file_drop` methods the production
`can_drop`/`on_drop` closures in `view/listing.rs` invoke with a second
payload, and asserts both refuse. After settling, only the first drop's
payload reached the filesystem (`right.join("second.txt")` does not exist,
`second.txt` remains at the source).

Note on this test's shape: an earlier version of this test tried to chain a
*second full GPUI mouse-drag gesture* immediately after the first one ended
via `dispatch_mouse_up` (used because the plain `simulate_mouse_up` alias
does not appear to route the real `on_drop` callback the way
`dispatch_event`/`to_platform_input()` does - `dnd_routing_*` production
tests rely on the latter for actual filesystem effects). That chained second
gesture did not reliably re-enter an "active drag" state in the test
harness. Rather than depend on undocumented harness behavior for chaining
two full drags, the test was rewritten to call the exact shared
`can_accept_listing_cwd_drop`/`begin_file_drop` methods the second
`on_drop` would call - the same code path, without the harness-dependent
gesture chaining. This is a deliberate scope judgment, flagged here rather
than hidden.

Truth base: Chronos-FM.

Claim: A partial failure (one source vanishes mid-transfer) still reloads
and selects the successful destination and reports a visible, counted error
status - proven through the real cross-pane production path, not only the
direct pane-level call.

Evidence: pane-level proof is `partial_failure_reloads_success_and_reports_counts`
(`dnd.rs:1436`, pre-existing). Task 4 adds
`e2e_cross_pane_partial_failure_reports_visible_error` (`dnd.rs:1879`),
which drags two real selected files (one is deleted from disk immediately
after the production mouse-up, racing the in-flight background transfer,
the same adversarial timing as the pane-level test) and asserts the
surviving file exists at the destination, is selected, and
`status_for_footer()` returns an error containing `"1 succeeded, 1 failed"`
and the vanished file's name.

Truth base: Chronos-FM.

Claim: If the source pane's entity is released (e.g. closed) while a
cross-pane transfer is still pending, the destination still completes and
reloads normally (spec §8).

Evidence: pane-level proof is
`released_source_while_pending_still_completes_target` (`dnd.rs:1475`,
pre-existing, using an orphaned unrooted entity). Task 4 adds
`e2e_closing_source_pane_before_completion_still_reloads_destination`
(`dnd.rs:1928`), which performs a real production drop inside a two-pane
`ExplorerPage`, then calls the production `ExplorerPage::close_pane(0, ...)`
action (the same method the pane's close-tab/close-pane UI action calls) to
release the source pane's only remaining strong reference, confirms via a
captured `WeakEntity` that the source pane entity was actually dropped
(`source_weak.upgrade().is_none()`), and then asserts the destination still
completes: the file moved, the destination pane is not pending, and its
selection is correct.

Truth base: Chronos-FM.

Claim: Mouse-up on an invalid target always ends the GPUI drag - no stuck
preview, no stuck cursor, no filesystem side effect - even when the pointer
never reaches a valid target.

Evidence: `e2e_mouse_up_on_invalid_target_ends_drag_with_no_stuck_preview_or_cursor`
(`dnd.rs:2065`) drags a file onto a covered (file) row, confirms
`OperationNotAllowed`, releases, and asserts `!active_drag`, no
`file-drag-preview` debug element remains painted, `drop_pending` is false,
and no `first (2).txt` was created.

Truth base: Chronos-FM.

Claim: Starting a file drag from a real List row cancels an active T050
marquee, and this remains true in the production split-pane tree, not only
the pane-level module tests.

Evidence: `dnd_routing_unselected_list_row_normalizes_and_cancels_marquee`
(`dnd.rs:1023`, pre-existing Task 3 test) begins a real marquee via
`begin_marquee`, then starts a real file drag and asserts `pane.marquee` is
`None` immediately after drag activation, driven by `FileDrag::activate`
(`dnd.rs:73`), which calls `pane.cancel_marquee()`. This is part of the
31/31 `dnd` result and the 23/23 `marquee` result was unaffected
(`cargo test -p chronos-fm-pages --lib marquee` exited 0, 23 passed).

Truth base: Chronos-FM.

Claim: Only local filesystem panes participate; a provider-backed pane
rejects a local drag both by cursor feedback and by refusing the transfer.

Evidence: `dnd_routing_items_rename_header_and_provider_reject` (`dnd.rs:1144`,
pre-existing) sets `pane.provider = Some(...)` mid-test and asserts
`OperationNotAllowed` plus no filesystem mutation, and separately that a
provider-origin item cannot start a drag at all. `can_accept_file_drop`
(`dnd.rs:260`) and `begin_file_drop` (`dnd.rs:298`) both check
`self.provider.is_some()` and the drag source's provider before any
validation or filesystem work runs.

Truth base: Chronos-FM.

Claim: Paste and Drop share one transfer helper and one `unique_name` path -
no duplicated conflict-resolution logic exists between them.

Evidence: `crates/chronos-fm-services/src/fs/ops.rs:164`
(`transfer_paths`) is the single call site used by both
`crates/chronos-fm-pages/src/explorer/file_ops.rs:43`
(`paste_clipboard`, Task 1) and `dnd.rs:329`
(`begin_file_drop`'s `cx.background_spawn`). Neither call site reimplements
name-collision resolution; `unique_name` (`ops.rs:140`) is the only
collision path in the codebase for either flow.

Truth base: Chronos-FM.

Claim: Cross-volume Move still delegates to `move_path`'s existing
copy-plus-delete fallback unchanged; DnD did not introduce a second Move
implementation.

Evidence: `transfer_paths` (`ops.rs:164`) calls `move_path` (`ops.rs:249`)
for `TransferMode::Move`, unchanged from Task 1. `move_path`'s own
same-volume-rename and cross-volume-EXDEV-fallback tests
(`move_path_within_volume_renames_and_removes_source` and its EXDEV-fallback
sibling) are part of the 6/6 `transfer_paths` result and 143/143
`chronos-fm-services --lib` result below. Task 4 did not modify `ops.rs`.

Truth base: Chronos-FM.

Claim: No T052 (external DnD), T053 (conflict UI), T054 (undo), or T056
(progress/cancel UI) surface exists in the branch diff.

Evidence: `git diff 8c1c633..a45970c --stat -- crates/` (the Task 1 baseline
through this agent's Task 4 commit) touches only
`crates/chronos-fm-pages/src/explorer.rs`,
`crates/chronos-fm-pages/src/explorer/dnd.rs`,
`crates/chronos-fm-pages/src/explorer/file_ops.rs`,
`crates/chronos-fm-pages/src/explorer/navigation.rs` (a 4-line `#[cfg(test)]`
reload counter),
`crates/chronos-fm-pages/src/explorer/state.rs`,
`crates/chronos-fm-pages/src/explorer/view/header.rs`,
`crates/chronos-fm-pages/src/explorer/view/listing.rs`,
`crates/chronos-fm-pages/src/explorer/view/listing/grid.rs`, and
`crates/chronos-fm-pages/src/explorer/view/listing/row.rs` - no OS drag-and-drop
integration, no conflict-resolution dialog, no undo/history recording, and
no progress/cancel queue code exists in any of them. `drop_pending` remains
a single bool per pane with a status-line error message on failure, not a
queue.

Truth base: Chronos-FM.

## Verification commands and results

All commands run from
`/home/neo/projects/chronos-ecosystem/Chronos-FM/.worktrees/t051-dnd` at
commit `a45970c` (branch `feat/t051-dnd`).

```
cargo test -p chronos-fm-services --lib transfer_paths
  -> exit 0, 6 passed, 0 failed

cargo test -p chronos-fm-pages --lib dnd
  -> exit 0, 31 passed, 0 failed   (22 baseline Task 1-3 + 9 new Task 4)

cargo test -p chronos-fm-pages --lib dnd_routing
  -> exit 0, 6 passed, 0 failed

cargo test -p chronos-fm-pages --lib marquee
  -> exit 0, 23 passed, 0 failed

cargo test -p chronos-fm-pages --lib keybindings
  -> exit 0, 16 passed, 0 failed

cargo test -p chronos-fm-pages --lib explorer::tests
  -> exit 0, 42 passed, 0 failed

cargo test -p chronos-fm-pages --lib
  -> exit 0, 179 passed, 0 failed, 0 ignored

cargo test --workspace
  -> exit 0, 447 passed, 0 failed, 0 ignored across all crates
     (13 chronos-fm-core + 62 chronos-fm-services + 0 chronos-fm-models
      + 179 chronos-fm-pages + 143 chronos-fm-store + 16 chronos-fm
      + 34 chronos-fm-ui; 6 doc-test suites, 0 doc-tests each)

cargo build --release -p chronos-fm
  -> exit 0, `target/release/chronos-fm` produced, 3m34s build time
```

`cargo fmt -p chronos-fm-pages` was run once to format the touched file; it
reformatted the entire crate (pre-existing formatting debt unrelated to
T051). Only `crates/chronos-fm-pages/src/explorer/dnd.rs`'s formatting
changes were kept; every other file's incidental reformat was reverted with
`git checkout --` before committing, per the plan's "restore only formatter
changes created by this task outside the approved T051 file set" step.
`git diff --check` reported no whitespace errors on the kept diff.

No production code changes were required by Task 4 or Task 5 - all new
tests passed against the existing Task 1-3 implementation on first green
(after two test-harness fixes described below), so there is no separate
"hardening commit."

Two test-authoring corrections worth naming honestly (not defects in
production code, defects in my first test draft):

1. The first draft of the split-pane fixture clicked List rows at their
   full measured-bounds center (`item_point`), which is correct for a
   full-width single pane but not for a narrow half-window split pane: the
   List view's column widths are independent of the pane's rendered width,
   so a row's measured bounds can be wider than its pane's visible/clipped
   area, and a click at the row's horizontal center landed outside the
   clip region and hit nothing. Switched to the existing `item_name_point`
   helper (offset from the row's left edge, already used by Task 3 for
   this reason) for all split-pane row targets.
2. The breadcrumb's `debug_selector` (`"current-directory-breadcrumb"`) is
   not unique across two panes in `cx.debug_bounds` (a `HashMap` keyed by
   the static string), so querying it in a two-pane test silently returns
   whichever pane painted last, not the pane under test. The two
   same-parent tests were changed to target the pane's own empty-listing
   cwd surface instead of its breadcrumb, which exercises the identical
   `DropTargetKind::ListingCwd` validation path without the selector
   collision.

## Live release evidence

Compositor: Hyprland, single monitor `HDMI-A-1` at `2560,0 1920x1200`.
Binary: the exact binary built above,
`target/release/chronos-fm`, launched with an isolated `HOME` /
`XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `XDG_CACHE_HOME` (a fresh scratch
directory, no pre-existing chronos-fm config/session), `--theme dark
--accent blue --page explorer`, `cwd` set to the fixture root.

Window: confirmed via `hyprctl clients -j | jq 'select(.class=="chronos-fm")'`
(class `chronos-fm`, address `0x564168c85df0`, PID `1500086`), floated and
resized to `1400x900` at `2600,100` for a workable split layout, geometry
re-verified immediately before every `grim -g` capture per this repo's
live-smoke discipline (stale geometry is not trusted). Vulkan adapter
confirmed from the runtime log: `Selected GPU adapter: "NVIDIA GeForce RTX
3070" (Vulkan)`.

Fixture: real tempdir-equivalent scratch directory, `fixture/source/`
containing `alpha.txt` (11B), `beta.txt` (10B), `gamma.txt` (11B), `dup.txt`
(19B, content `dup source version`), and nested `archive/readme.txt` (15B);
`fixture/destination/` pre-seeded with its own `dup.txt` (39B, content `dup
destination version (pre-existing)`) to force a `unique_name` collision.

Input: `ydotool key`/`click`/`mousemove -a` (absolute-mode; this
compositor doubles the requested coordinate and offsets by the monitor
origin - calibrated with `hyprctl cursorpos` before use, per this repo's
recorded live-smoke gotcha). Splitting used the production keybinding
(`ctrl-\`, keycodes `29:1 43:1 43:0 29:0`); navigation and selection used
real double-clicks and Ctrl-clicks on real List rows; the drag itself used
`ydotool click 0x40` (down), several `mousemove -a` steps past the drag
threshold and onto the target, then either `ydotool click 0x80` (plain
Move) or `ydotool key 29:1` (Ctrl down) followed by `click 0x80` then
`key 29:0` (Ctrl-Copy, Ctrl evaluated at release as required).

- `docs/orchestration/tasks/report-log/T051-dnd-before.png`: both panes
  visible (`source` left, `destination` right); left pane has `archive`,
  `alpha.txt`, `beta.txt`, `gamma.txt` selected (footer: "4 selected"),
  `dup.txt` correctly not selected; right pane shows its single
  pre-existing `dup.txt`.
- `docs/orchestration/tasks/report-log/T051-dnd-held.png`: mouse button
  still down, drag preview shows `gamma.txt` + a `4 items` badge, the
  destination pane's listing has a visible accent-bordered highlight.
- `docs/orchestration/tasks/report-log/T051-dnd-move-after.png`: released
  without Ctrl; source pane now shows only `dup.txt` (1 item); destination
  pane now shows 5 items (`archive`, `alpha.txt`, `beta.txt`, `dup.txt`,
  `gamma.txt`), with the four moved entries highlighted as the new
  selection and the pre-existing `dup.txt` correctly not selected.
  Filesystem: `stat` confirmed the four moved entries exist under
  `destination/` at their original byte sizes and no longer exist under
  `source/`; `destination/dup.txt` was untouched. Runtime log:
  `file drop filesystem work completed success_count=4 failure_count=0
  mode=Move`.
- `docs/orchestration/tasks/report-log/T051-dnd-copy-after.png`: a second,
  separate drag of `dup.txt` from `source`, begun with no modifier
  (confirming the earlier-established Move default), Ctrl pressed only
  while hovering the destination, released with Ctrl held. Destination pane
  now shows 6 items including a highlighted new `dup (2).txt`; the original
  `dup.txt` is present and unhighlighted; source pane still shows its
  `dup.txt` (1 item, unchanged selection - cross-pane Copy retains source
  selection per spec §6). Filesystem: `destination/dup (2).txt` is 19 bytes
  with content `dup source version` (byte-identical to `source/dup.txt`,
  confirmed via `cat`); `destination/dup.txt` remained 39 bytes with its
  original pre-existing content, unmodified; `source/dup.txt` still exists.
  Runtime log: `file drop filesystem work completed success_count=1
  failure_count=0 mode=Copy`.

All four PNGs were visually inspected (not just filesystem-checked) before
being committed: each shows the real `chronos-fm` title bar and only the
`chronos-fm` window (no other app's content bled through - an earlier,
uncommitted capture with a stray overlapping window was discarded and
retaken after focusing the window), no mid-transition/loading state, no
clipped pane, and file/folder names are legible.

## Non-negotiables checklist (from the design)

- Collision handling is `unique_name`-only, proven live (`dup (2).txt`,
  no overwrite of the pre-existing `dup.txt`) and by the shared
  `transfer_paths` code path both Paste and Drop use. Yes.
- Ctrl is evaluated at drop time, proven by
  `e2e_ctrl_evaluated_at_drop_time_copies_across_panes` and live (drag
  started plain, Ctrl pressed only mid-hover). Yes.
- Local filesystem panes only; provider panes reject, proven by
  `dnd_routing_items_rename_header_and_provider_reject`. Yes.
- No fall-through: proven by
  `e2e_folder_dragged_onto_self_does_not_fall_through_to_cwd` and
  the pre-existing item-covered-cwd case. Yes.
- Paste and Drop share one transfer helper: proven by both call sites using
  `transfer_paths`/`unique_name` exclusively. Yes.
- Starting a file drag cancels T050 marquee state:
  `dnd_routing_unselected_list_row_normalizes_and_cancels_marquee`. Yes.
- T052/T053/T054/T056 are out of scope and absent from the diff (see
  above). Yes.
- No `git push`, no merge to main, no self-ACCEPT, ticket left in `active/`
  (not moved to `done/`). Yes - this report does not stamp ACCEPT.
- Staged by file, never whole directories, in every commit on this branch
  including this task's. Yes.

## Commits made by this agent

- `a45970c` - `test(explorer): cover cross-pane file drops` (Task 4:
  `crates/chronos-fm-pages/src/explorer/dnd.rs` only)
- This report + evidence commit (Task 5, committed immediately after this
  file is written)


Claim: Dragging an unselected list/grid item makes that item the sole selection
before the payload is finalized, and cancels any active T050 marquee (spec §1 /
§7 event order).

Evidence: `FileDrag::activate` (`dnd.rs:73`) sole-selects the initiating path
when it was not selected and calls `cancel_marquee`. Production tree proof:
`dnd_routing_unselected_list_row_normalizes_and_cancels_marquee` (`dnd.rs:1023`)
begins a marquee, starts a real file drag from an unselected row, and asserts
`pane.marquee` is `None` with the initiating row selected. Covered in the
31/31 `dnd` suite (22 baseline + 9 Task 4). This claim is a first-class
normalization requirement of the design, not only an incidental marquee note.

Truth base: Chronos-FM.

Claim: Filesystem transfer runs off the UI thread; completion reloads panes and
updates selection after the background job finishes (spec §3 step 5–6, §8).

Evidence: `begin_file_drop` (`dnd.rs:298`) sets `drop_pending` synchronously,
then `cx.background_spawn(async move { transfer_paths(...) })` and
`cx.spawn` → `complete_file_drop` for UI updates. Task 4 production proofs
that observe completion *after* `settle_drop` / background park include
`e2e_cross_pane_move_reloads_both_panes_and_selects_destination`,
`e2e_ctrl_evaluated_at_drop_time_copies_across_panes`,
`e2e_cross_pane_partial_failure_reports_visible_error`, and
`e2e_closing_source_pane_before_completion_still_reloads_destination` (destination
still completes when the source entity is released mid-flight). Live release log
line `file drop filesystem work completed success_count=… mode=Move|Copy`
matches background completion, not a UI-thread inline transfer.

Truth base: Chronos-FM | runtime log.

## Residual / handed off

- T052 external DnD, T053 conflict UI (Rename/Overwrite/Skip/Apply-to-all),
  T054 undo, T056 progress/cancel UI remain out of scope, as designed.
- The pending-target-refuses-a-second-drop coverage at the split-pane
  production level exercises the shared `can_accept_listing_cwd_drop`/
  `begin_file_drop` methods directly rather than a second chained GPUI
  drag gesture (see "Verification commands and results" above for why);
  flagged as a judgment call for the reviewer, not hidden as a gap.
- Live evidence used a Copy-mode destination row highlight implied by the
  accent border visible in `T051-dnd-held.png`; a distinct `DragCopy`
  cursor frame was not separately screenshotted mid-hover during the live
  session (the automated `e2e_ctrl_evaluated_at_drop_time_copies_across_panes`
  test does assert the `DragCopy` cursor programmatically). If the
  Architect wants a live grim of the Ctrl-hover cursor specifically, that
  is a small additional capture, not a code gap.
