# T045 — Zero-size repaint storm disrupts sidebar rendering (was T037#5)

> ## ⚖️ ARCHITECT (2026-08-10, pass 4): **PROGRESS — not ACCEPT; path 2 GO**
>
> Pass 3 stamp was **stale** relative to `5305aba`. Frame-trace (path 1)
> findings **accepted**:
>
> - **H5a** still FALSIFIED (static RAII).
> - **“Double paint (one correct + one zero)” RETRACTED** for instrumented
>   `Svg::paint`: `icons/house.svg` — **494/494 calls** over ~22s / frames
>   1–494 are `Bounds::default()`; **zero** non-zero paints through this path.
> - Whatever shows a correct house icon on screen is **not** this code path
>   (or compositing keeps a prior GPU texture without a successful element
>   paint this session — renderer-level).
> - Duplicate pane / split **ruled out** (`panes=1`).
> - Instrumentation reverted; Source + FM clean; tests green per `5305aba`.
>
> ### Decision
> - **Stop** more Chronos-FM release + `eprintln!` cycles on the element tree
>   for this bug — diminishing returns.
> - **Next executor work: path 2** — minimal isolated repro in
>   `Source/gpui/examples/` (sidebar SVG + `v_virtual_list` + optional
>   resizable; **no** Chronos-FM crates). Goal: reproduce zero-size SVG storm
>   and/or missing sidebar layout without FM app code.
> - **H6 (secondary, after or alongside path 2):** `gpui_wgpu` dirty-rect /
>   atlas compositing — only if example does not reproduce or proves paint
>   path is red herring for “sidebar missing”.
> - Still **no blind Source layout patch**. T037 remains blocked on T045.
>
> Commit: `5305aba`. Report must stay aligned with pass 4 (not pass 3 only).

**Priority:** P1 — blocks T037 §7 full visual ACCEPT (Places sidebar must be
visible alongside a real listing, not just in isolation).
**Source:** first flagged as residual "T037#5" in
`report/T037-visual-shell-mockup-parity-report.md` (log noise only, no
visible-content impact claimed at the time). Escalated here with new
evidence: it visibly disrupts the sidebar once T044's fix restored the full
three-panel (`sidebar` + `h_resizable(listing, preview)`) tree.

## Symptom (facts)

- Release binary, `class=chronos-fm`-verified live grims (3 independent
  runs, `t037_verify2.png`/`t044_final.png`/etc, 15–25s settle each,
  2026-08-10): the Places sidebar is **absent** every time (nav rail
  present, no Places column) once the listing/preview panels are also
  present in the tree.
- Earlier the same session, with **only** the sidebar in the render tree
  (before T044's fix restored listing/preview), the sidebar rendered
  correctly and reproducibly (`after-t014a-textmemo-fix.png`,
  `t037_verify3.png`) — so this is not the T014-A text-memo bug (already
  fixed) and not a sidebar-code defect in isolation.
- `RUST_LOG=info` logs show `ERROR: can't render at a zero size` starting
  ~8s after launch (right around first real content paint) and continuing
  **without stopping** — 30,000+ occurrences by a 20–25s settle, in all 3
  runs. Not a transient startup race; a sustained loop.
- File listing itself renders correctly and is stable (T044) even while
  this storm is active — the storm's damage is localized to the sidebar (or
  to whichever element loses the race for space on a given frame), not the
  whole window.

## Hypotheses (must disprove with evidence — do not assume one)

| ID | Hypothesis | How to falsify |
|----|------------|----------------|
| H1 **FALSIFIED** | `h_resizable`'s continuous re-layout (`page.resizable` state notifying every frame, e.g. from `v_virtual_list`'s scroll-handle bookkeeping) triggers a `cx.notify()` loop that never settles, and the sidebar's fixed-width div loses a size race against the resizable panels during some frames | Log a per-frame counter in `view.rs`'s outer `render` (call count over 5s) with only sidebar in the tree vs. with the full tree restored — compare render frequency |
| H2 | `v_virtual_list`'s own internal state (`scroll_handle`, `content_size`) recomputes unstably each frame because `page.item_sizes` (or another input) changes identity every render (e.g. a `Rc::new` rebuilt from scratch instead of memoized), so its content invalidates every frame and the request-layout tree keeps changing shape | Check whether `Rc<Vec<Size<Pixels>>>` passed to `v_virtual_list` is stable across frames (same `Rc` pointer) when nothing actually changed, or rebuilt every render in `list.rs` |
| H3 | The "can't render at a zero size" errors originate from the *listing/preview* subtree (not the sidebar) but the resulting reflow starves the sidebar's `flex_row` cross-axis allocation on the frames it fires | Find the error's log source location in `Source/gpui` (grep the exact log message) and correlate its call site with which element is zero-sized |
| H4 **FALSIFIED** | Unrelated to T044's restored subtree at all — pre-existed even with only the sidebar present, just below a detection threshold this session didn't check for (no log capture was done on the sidebar-only grims) | Re-run the sidebar-only tree (temporarily) with `RUST_LOG=info` captured and check whether the same "zero size" storm was already present before T044's fix |

## Progress (2026-08-10, same session, per architect GO)

> **pass 4 note:** Early “double paint” wording below is **superseded** by the
> frame-trace: house.svg never gets a non-zero `Svg::paint` in that trace.


**H4 FALSIFIED with evidence.** Temporarily reverted `view.rs` to the
sidebar-only tree (`git show 5b6fee3:...view.rs`), rebuilt, ran with
`RUST_LOG=info` captured over a 20s settle: **zero** "can't render at a
zero size" errors (`grep -c "zero size" /tmp/t037_smoke.log` → `0`). The
storm does not pre-exist — it is genuinely triggered by restoring
`h_resizable`/`v_virtual_list` (T044's fix). Restored the real `view.rs`
via `git checkout HEAD --` afterward.

**Log source found:** `Source/gpui/src/svg_renderer.rs:202`,
`render_alpha_mask`'s `anyhow::ensure!(!params.size.is_zero(), "can't
render at a zero size")`, reached from `Window::paint_svg`
(`window.rs:4164`) via `Svg::paint`'s `.log_err()` (`elements/svg.rs:130`)
— every hit is swallowed and logged, never a crash.

**Instrumented `Svg::paint` (temporary, reverted after — `git -C Source
checkout -- gpui/src/elements/svg.rs`) to print which icon path hits zero
bounds.** Result (15s run, `CHRONOS_SVG_DEBUG=1`): **12,588 occurrences**,
spanning ~14 distinct icon paths — every sidebar Places icon
(`house.svg`, `monitor.svg`, `file-text.svg`, `file-image.svg`,
`download.svg`) **and** every header/toolbar icon (`search.svg`,
`plus.svg`, `panel-bottom-open.svg`, `layout-dashboard.svg`,
`circle-user.svg`, `chevron-left/right.svg`, `arrow-up.svg`) **and**
listing/device icons (`folder.svg`, `hard-drive.svg`). All at
`bounds = Bounds { origin: (0,0), size: (0,0) }` — not just zero size,
zero origin too. First occurrence at process start (line 12 of the log,
effectively immediately), last occurrence at the very last log line —
**continuous for the entire run, not a startup transient.** These same
icons **do** render correctly on screen in the grims (T044's evidence
screenshots) — so each affected icon is being painted **twice** per frame:
once correctly (what appears on screen) and once bogus (what errors).

**H1 (my own hypothesis) FALSIFIED.** Suspected `ResizablePanelGroup`'s
`on_prepaint` (`Source/gpui-component/crates/ui/src/resizable/panel.rs:159`)
— it compares `state.bounds.size.along(axis) != bounds.size.along(axis)`
with strict `!=` and calls `state.adjust_to_container_size(cx)` →
unconditional `cx.notify()` on any change, which looked like a classic
float-jitter self-notify loop. Instrumented it (temporary, reverted —
`git -C Source checkout -- gpui-component/crates/ui/src/resizable/
panel.rs`) to log every `size_changed` event with old/new/diff. Result: only
**3** occurrences in 12s (`0px→1035px`, `1035px→717px`, `717px→710px`) —
the window settling from initial layout to a stable size, then it stops.
Not a loop; not the storm's source.

**New leading hypothesis (not yet tested):** the double-paint pattern (every
affected icon painting once correctly, once at exactly `(0,0)-(0,0)`,
continuously) points at `v_virtual_list`'s per-item
`item.layout_as_root(available_space, window, cx)` call inside its own
`prepaint` (`Source/gpui-component/crates/ui/src/virtual_list.rs:716`,
traced in T044's investigation). `layout_as_root` itself calls
`window.compute_layout(...)` (`Source/gpui/src/element.rs:499`) — a nested,
mid-paint invocation of the *same* frame-global layout engine
(`TaffyLayoutEngine`, the one T014-A's memo fix touched) that every other
element's `bounds` resolution also depends on. A nested `compute_layout`
call while the outer frame's own layout/paint sequencing is still in
progress is a plausible way for sibling elements (sidebar/toolbar icons,
not part of the virtual list) to have their committed bounds transiently
reset or read as a stale/degenerate `(0,0)` when *their* paint callback
fires — worth checking `computed_layouts`/`frame_node_order`
re-entrancy in `taffy.rs` for what happens when `compute_layout` is called
again for a *different* subtree before the outer one has finished pinning
every node's `absolute_layout_bounds`.

## H5 narrowing (2026-08-10, continued): offset-stack leak ruled out by static review

> **Architect:** H5a (stack leak) closed by static review — good stop.
> Next must produce either timing correlation or isolated gpui repro (H5b/H6).


Read `Window::layout_bounds` (`window.rs:4375`): its returned `bounds.origin`
is `stored_absolute_origin + self.pixel_snap_point(self.element_offset())`
— i.e. it depends on the **ambient** `element_offset_stack` at call time, not
a value fixed per-node. This looked like a strong lead: if
`v_virtual_list`'s nested `layout_as_root`/`prepaint_at` calls left that
stack unbalanced (pushed without a matching pop), every *later* sibling
paint in the same frame would read a corrupted ambient offset.

**Ruled out by code review** (no live test needed for this part): traced
every push/pop pair in the exact call path —
`AnyElement::prepaint_at` (`element.rs:643`) wraps its single `self.prepaint`
call in `window.with_absolute_element_offset(origin, |window| ...)`, which
itself (`window.rs:3358`) pushes, calls the closure, then unconditionally
pops — a single-expression RAII-shaped scope, no early return, no loop
inside it that could double-push. `v_virtual_list`'s own item loop
(`virtual_list.rs:692`) wraps the whole per-item loop in one
`window.with_content_mask(...)` call, same shape. **Every scope here is
correctly balanced by construction** — this specific leak mechanism is not
what's happening. (Still worth checking `element_offset_stack` depth
*empirically* — this review only rules out an *unbalanced push/pop*, not
every possible way the ambient offset could be wrong going in.)

**Also notable, not yet explained:** the debug output is not "slightly
off" — it is **exactly** `Bounds { origin: (0,0), size: (0,0) }` on all
12,588 occurrences, no variance. That's the shape of a `Bounds::default()`
value, not a corrupted-but-computed one. Worth checking whether some path
paints with a genuinely default/uninitialized `Bounds` rather than one
derived from `layout_bounds()` at all — e.g. a second, distinct paint
invocation for the same element that never went through the normal
layout→bounds resolution this frame.

**Stopping point for this session:** further progress needs either (a) a
live instrumented trace correlating the *exact* frame-relative timing of a
zero-bounds SVG paint against `v_virtual_list`'s nested-layout window, or
(b) a minimal isolated repro in a new `Source/gpui/examples/` file (sibling
SVG icon + a nested `layout_as_root` caller, no Chronos-FM code at all) —
both are real next steps, not done this pass. Handing off with the search
space narrowed rather than forcing another live cycle for its own sake.

## Live timed correlation (2026-08-10, continued): the mystery deepened, not solved

Per architect-approved path 1. Added a per-thread frame counter
(`Window::draw`'s entry point) and logged **every** `Svg::paint` invocation
(not just zero-bounds ones — both `self.path` and `self.external_path`
branches, plus a top-of-closure log that fires regardless of which branch
matches or whether `style.text.color` is `None`) with the frame number.

**Result, `icons/house.svg` specifically (sidebar "Home", unambiguous —
only used by `sidebar.rs::folder_icon_path`):** every logged invocation,
from **frame 1** (the very first `Window::draw()` call) through **frame
494** (~22s in, long after grim captures already show the icon correctly
on screen) is `bounds=(0,0)-(0,0)`. **Zero non-zero occurrences across the
entire process lifetime.** `self.path=Some("icons/house.svg")`,
`self.external_path=None`, `color_some=true` on every single one — so it's
not a silently-skipped paint (`style.text.color == None`) either; the
`if let` branch I originally instrumented is provably the *only* branch
that ever fires for this element, and it *always* fails.

**This rules out "double paint, one correct one bogus" as I described it
in the previous update** — there is no second, correct invocation of
`Svg::paint` for this element to be found anywhere in this trace. Whatever
puts the correct icon on screen (confirmed via T044's grim evidence, and
via this session's own `t037_smoke.sh` screenshots) is not something this
instrumentation can see, meaning either:
- it doesn't go through `Element::paint` on this `Svg` value's own
  `self.path` branch at all (a genuinely different rendering mechanism
  for the same visual result), or
- gpui's frame compositing does not fully repaint the whole surface on
  every `draw()` call (a partial/dirty-rect update, or the wgpu backend
  retaining a previous frame's sprite/output for a region that a later,
  failed paint attempt doesn't overwrite) — a renderer-level question this
  session's log-based instrumentation cannot answer.

**Also ruled out this pass:** a duplicate/hidden second `ExplorerPane`
(split view) rendering its own sidebar at zero width. Instrumented
`ExplorerPage::restore_session`'s entry (Chronos-FM's own code, cheap
build) to log the restored session snapshot's pane count:
`panes=1 pane0_tabs=1 pane1_tabs=None` — there is exactly one pane, no
split, no second sidebar instance to explain a "correct + bogus" pair that
way either.

**All debug instrumentation reverted** (`Source/gpui/src/window.rs`,
`elements/svg.rs`, `gpui.rs`, deleted `t045_debug.rs`;
Chronos-FM's `explorer/page.rs`) — `git status` clean on both repos.

**Honest assessment:** this has moved from "narrowed hypothesis" to "a
surprising, well-evidenced fact that doesn't fit the mental model of how
`Svg::paint`/`layout_bounds`/frame compositing are supposed to interact."
Resolving it needs either GPU/renderer-level tooling (frame capture,
RenderDoc-style inspection of what the wgpu backend actually submits per
draw call) or a much deeper read of `gpui_wgpu`'s scene-to-GPU pipeline —
beyond what CPU-side `eprintln!` instrumentation of the element tree can
distinguish. Recommending the next pass start with **path 2** (the
isolated `Source/gpui/examples/` repro the architect also approved) since
a minimal reproduction would let a renderer-level investigation iterate
much faster than this session's full `chronos-fm` release-binary cycle
(~2 min/iteration).

## Done when

1. Root cause Claim → Evidence (one H* confirmed, others falsified).
2. Fix: sidebar renders reliably alongside a populated listing on a fresh
   release grim (no per-frame flakiness across 3+ repeated captures).
3. "can't render at a zero size" errors stop (or are proven benign/expected
   and rate-limited, with evidence for why they're safe to ignore).
4. Regression test if the root cause is a stable-identity/memoization bug
   (unit-testable without a live window).
5. Fresh grim attached under `report-log/T045-shots/`.
6. Unblocks T037 §7 full ACCEPT together with T044.

## Walls

- Source truth; no Zed/datasets.
- Facts only.
- `class=chronos-fm`-reverified grim only (see T037/T044 "settled.png is not
  FM" trap) — re-check right before `grim`, not just before the settle
  sleep (`script/dev/t037_smoke.sh` was hardened for this in T044's
  session, reuse it).
- Vision/omnimodal on final grim.

## Related

T037 Phase V shell · T044 (dropped listing/preview subtree — fixed, sidebar
absence is what's left) · `report/T037-visual-shell-mockup-parity-report.md`
("T037#5") · T042 epic
