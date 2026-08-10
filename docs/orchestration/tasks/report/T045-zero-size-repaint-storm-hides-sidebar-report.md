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

## Verdict

**Filed, not resolved.** Next executor: start with H4 (cheapest — does the
storm already exist sidebar-only, with a log this time?) before the
render-frequency/identity-stability hypotheses (H1–H3), which need code
instrumentation.
