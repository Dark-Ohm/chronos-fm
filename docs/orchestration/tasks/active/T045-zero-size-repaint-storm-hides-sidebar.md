# T045 — Zero-size repaint storm disrupts sidebar rendering (was T037#5)

> ## ⚖️ ARCHITECT (2026-08-10): **GO — investigate + fix** (not ACCEPT)
>
> Filing + evidence quality OK: full tree vs sidebar-only contrast, storm
> sustained (30k+), listing stable under storm, H1–H4 matrix honest.
> **T037 blocked on this.** Prefer H4 log capture first (cheapest), then
> Source site of `can't render at a zero size`. No self-ACCEPT.
> Report: `report/T045-zero-size-repaint-storm-hides-sidebar-report.md`.


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
| H1 | `h_resizable`'s continuous re-layout (`page.resizable` state notifying every frame, e.g. from `v_virtual_list`'s scroll-handle bookkeeping) triggers a `cx.notify()` loop that never settles, and the sidebar's fixed-width div loses a size race against the resizable panels during some frames | Log a per-frame counter in `view.rs`'s outer `render` (call count over 5s) with only sidebar in the tree vs. with the full tree restored — compare render frequency |
| H2 | `v_virtual_list`'s own internal state (`scroll_handle`, `content_size`) recomputes unstably each frame because `page.item_sizes` (or another input) changes identity every render (e.g. a `Rc::new` rebuilt from scratch instead of memoized), so its content invalidates every frame and the request-layout tree keeps changing shape | Check whether `Rc<Vec<Size<Pixels>>>` passed to `v_virtual_list` is stable across frames (same `Rc` pointer) when nothing actually changed, or rebuilt every render in `list.rs` |
| H3 | The "can't render at a zero size" errors originate from the *listing/preview* subtree (not the sidebar) but the resulting reflow starves the sidebar's `flex_row` cross-axis allocation on the frames it fires | Find the error's log source location in `Source/gpui` (grep the exact log message) and correlate its call site with which element is zero-sized |
| H4 | Unrelated to T044's restored subtree at all — pre-existed even with only the sidebar present, just below a detection threshold this session didn't check for (no log capture was done on the sidebar-only grims) | Re-run the sidebar-only tree (temporarily) with `RUST_LOG=info` captured and check whether the same "zero size" storm was already present before T044's fix |

## Progress (2026-08-10, same session, per architect GO)

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
