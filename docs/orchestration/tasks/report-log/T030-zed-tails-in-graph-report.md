# T030 — Zed tails in build graph: close report

> ## ✅ ARCHITECT VERDICT: **CLOSED — DEFERRED** (2026-08-09)
>
> Ticket closed without full hermeticization. Not a false “done”: the five
> `zed-industries/zed` edges remain a known dependency. Re-open only when
> offline/hermetic fork builds are a hard requirement.

## Facts at close

- Investigation scope from the ticket still accurate: `gpui` pulls
  `http_client` → `util` and `util_macros` → `perf` → `collections` from
  zed git (see `Source/gpui/Cargo.toml` workspace pins).
- **Partial related work already landed under T032**, not T030:
  `reqwest_client` + `http_client_tls` path-vendored; comment in
  `Source/Cargo.toml` still states **http_client stays on git (T030)**.
- `gpui_zed_util` remains a workspace member with unclear consumers vs
  zed’s `util` — Step 1 not executed as a dedicated T030 deliverable.
- No T030 implementation commits; no unit of work claimed complete.

## Why close now

1. Product path (Git/S3/perf/empty-state) is the priority; fork hermetic
   debt is real but not blocking daily Chronos-FM builds when network
   and the pinned zed rev are available.
2. Finishing T030 properly is multi-step (measure `http_client` use on
   Linux, vendor or feature-gate, move `gpui-component` zed pin, decide
   `gpui_zed_util`) — deserves a fresh ticket with an explicit go when
   hermetic builds are scheduled.
3. Keeping an open T030 with zero progress creates false backlog signal.

## Residuals (new ticket when scheduled)

- Measure Linux call sites of `http_client` in `Source/gpui`
- Resolve `gpui_zed_util` (wire or remove)
- Vendor/drop `util_macros` → `perf` → `collections` if macro surface is small
- Align `Source/gpui-component` zed rev pin with any graph change
- Success criterion: `cargo tree -e normal -p chronos-fm` has **zero**
  `zed-industries/zed` package sources for those five crates

## Files

- Ticket → `done/T030-zed-tails-in-graph.md`
- This report → `report-log/`
