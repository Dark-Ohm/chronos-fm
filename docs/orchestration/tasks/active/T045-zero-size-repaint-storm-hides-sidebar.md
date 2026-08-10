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
