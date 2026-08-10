# T045 — Zero-size repaint storm disrupts sidebar rendering — Report

> ## ⚖️ ARCHITECT VERDICT (2026-08-10): **FILED / GO** — not resolved
>
> Evidence for filing accepted. Root cause open. Next: H4 then Source log site.
> Unblocks T037 when fixed + grim shows Places + listing together.


**Status:** OPEN — filed with initial evidence, root cause not yet found
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5), discovered while verifying T044's fix

## Summary

Not a new investigation from scratch — this is the pre-existing "T037#5"
residual ("Zero-size repaint storm (34k errors/25s) | Known | T037#5",
`report/T037-visual-shell-mockup-parity-report.md`), escalated to its own
ticket because it now has a **visible content impact**, not just log noise:
once T044's fix restored the listing/preview panels alongside the sidebar,
the Places sidebar disappeared from every verification grim.

## Evidence gathered this session

- 3 independent release-binary, `class=chronos-fm`-reverified grims
  (`report-log/T044-shots/*.png`, doubling as T045 evidence — sidebar
  absent in all 3, `report-log/T045-shots/sidebar-missing-under-storm.png`).
- `RUST_LOG=info` logs (`/tmp/t044_final.log` and predecessors) show
  `ERROR: can't render at a zero size` starting ~8s after launch (matches
  first real content paint) and continuing without stopping through the
  full settle window — 30,000+ occurrences by 20-25s. Confirmed via
  `grep -c "zero size"` and `grep -m1` for the first timestamp vs. process
  start timestamp: this is a sustained loop, not a one-off startup race.
- Contrast: earlier the same session, with **only** the sidebar in the
  render tree (before T044 restored listing/preview), the sidebar rendered
  correctly and reproducibly on 2 separate grims
  (`report-log/T037-shots/after-t014a-textmemo-fix.png`, `t037_verify3.png`
  in scratch). No log capture was taken on those runs, so H4 (storm
  pre-existed even sidebar-only) is not yet falsified — flagged as the
  cheapest hypothesis to check first.

## Hypotheses

See ticket `active/T045-zero-size-repaint-storm-hides-sidebar.md` for the
full H1–H4 matrix. Not disproven yet — no further investigation performed
this session; this report documents the filing evidence only.

## Why not fixed in the same session

Root-causing this requires either instrumenting `Source/gpui`'s zero-size
error site (find where "can't render at a zero size" is logged and what
element triggers it) or a render-frequency counter comparison between the
sidebar-only tree and the full tree — both are non-trivial, multi-step
investigations on their own, and conflating them with T044's already-solid
fix risked losing a clean, verified win under an open-ended investigation.
Split intentionally so T044 could close on its own evidence.

## Update (2026-08-10, per architect GO): H1 and H4 falsified, exact site found

**H4 falsified.** Temporarily reverted `view.rs` to the sidebar-only tree
(`git show 5b6fee3:...`), rebuilt, ran 20s with `RUST_LOG=info`: **zero**
"zero size" errors. The storm is genuinely absent without the restored
`h_resizable`/`v_virtual_list` subtree — not a pre-existing background
condition this session missed. Restored the real `view.rs` afterward
(`git checkout HEAD --`).

**Exact log site found:** `Source/gpui/src/svg_renderer.rs:202`
(`render_alpha_mask`'s `ensure!`), reached via `Window::paint_svg`
(`window.rs:4164`) from `Svg::paint`'s `.log_err()`
(`elements/svg.rs:130`) — every hit is swallowed, never a panic.

**Instrumented `Svg::paint`** (temporary, reverted —
`git -C Source checkout -- gpui/src/elements/svg.rs`) to log which icon
path hits zero bounds. 15s run, `CHRONOS_SVG_DEBUG=1`: **12,588
occurrences**, spanning every sidebar Places icon, every header/toolbar
icon, and listing/device icons — **~14 distinct paths, all at
`bounds=(origin (0,0), size (0,0))`**, continuously from the first log line
to the last (not a startup transient). These same icons render correctly
on screen (per T044's grims) — meaning each is painted **twice** per
frame: once correctly, once bogus.

**H1 falsified.** Suspected `ResizablePanelGroup::on_prepaint`
(`gpui-component/.../resizable/panel.rs:159`) — strict `!=` comparison on
`bounds.size` triggering `cx.notify()` on any float jitter, a classic
self-sustaining loop shape. Instrumented (temporary, reverted —
`git -C Source checkout -- gpui-component/crates/ui/src/resizable/
panel.rs`) to log every `size_changed` event. Result: only **3**
occurrences in 12s, all large real changes (window settling from `0px` to
a stable `710px`), then it stops. Not a loop, not the storm's source.

**New leading hypothesis for the next pass:** `v_virtual_list`'s per-item
`item.layout_as_root(...)` (`virtual_list.rs:716`, traced in T044) calls
`window.compute_layout(...)` (`element.rs:499`) — a **nested**, mid-paint
invocation of the same frame-global `TaffyLayoutEngine` (the one T014-A's
memo fix touched) that every other element's committed bounds also depend
on. Worth checking whether a nested `compute_layout` call for one subtree
disturbs `computed_layouts`/`frame_node_order`/`absolute_layout_bounds`
for sibling nodes (sidebar/toolbar icons) whose paint callback fires later
in the same outer frame, reading a stale/reset `(0,0)` bounds. Not tested
this session — the fix site is inside core layout-engine re-entrancy
handling, judged too risky to blind-fix without further isolation given
the fork's blast radius (shared by ChronOS too).

## Verdict

**Filed and substantially narrowed, not resolved.** Two of four original
hypotheses falsified with hard evidence (H1, H4); exact log site and
affected-icon set identified; a concrete, evidence-backed new hypothesis
(nested `compute_layout` re-entrancy in `v_virtual_list`) handed to the
next pass instead of the original vaguer H1–H3. All debug instrumentation
reverted; tree is clean (`git status` on Source shows no diff from this
investigation).
