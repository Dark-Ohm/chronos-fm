# T046 — Shared visual proof residual (`--page=` / sub-views)

> ## ⚖️ ARCHITECT (2026-08-10): **PARTIAL-ACCEPT** (both slices)
>
> - `--page=<name>`: ACCEPT (default-view grims)
> - `--page=<page>:<sub>`: ACCEPT (routing + sub-view grims)
> - Residual for interaction-only proof: hypr click path (optional)
> - **Defect found via flag → T047** (history 4KB silent empty)
>
> Reports:
> - `report/T046-page-flag-and-default-view-proof-report.md`
> - `report/T046-subview-flag-and-history-bug-report.md`
> Shots: `report-log/T046-shots/`

**Priority residual:** P2 for hypr click; **T047 is P1** for History correctness.
