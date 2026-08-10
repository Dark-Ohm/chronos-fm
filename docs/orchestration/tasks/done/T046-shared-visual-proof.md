# T046 — Shared visual proof (`--page=` / sub-views)

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-10)
>
> ### Delivered — **ACCEPT**
> 1. `--page=<name>` CLI + smoke (`script/dev/t046_page_smoke.sh`) — ACCEPT
> 2. `--page=<page>:<sub>` sub-view routing — ACCEPT
> 3. Default-view grims (settings/extensions/git/s3) + sub-view grims
>    (`settings:appearance`, `git:history`) under `class=chronos-fm`
>
> ### Defect found via flag → **T047** (ACCEPT closed)
> History silent empty on 4KB truncate — fixed (256KB + no silent empty).
>
> ### Residual **WAIVED** (not blocking)
> Hypr focus/click path for interaction-only proof. Superseded by `--page=`
> for visual ACCEPT of pages/sub-views. Optional backlog only if product
> needs automated click tests later.
>
> Reports:
> - `report/T046-page-flag-and-default-view-proof-report.md`
> - `report/T046-subview-flag-and-history-bug-report.md`
> Shots: `report-log/T046-shots/`

**Priority residual:** none (hypr optional backlog).
