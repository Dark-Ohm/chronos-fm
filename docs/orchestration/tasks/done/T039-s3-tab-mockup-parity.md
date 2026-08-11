# T039 — Phase V/F: S3 tab mockup parity

> ## ⚖️ ARCHITECT (2026-08-10): **PARTIAL-ACCEPT**
>
> ### Code — ACCEPT
> Chunked transfer engine + 4-view UI + ashpd pickers (unit tests green).
>
> ### Vision so far
> - Empty NoProfiles: `T046-shots/t046_s3.png` + `T039-shots/t039_s3_*.png`
> - **Gap:** with zero profiles the page short-circuits to empty gate — **sub-nav
>   (Explorer/Buckets/Transfers/Properties) is not visible** in grims. So
>   `--page=s3:transfers` does not prove Transfers chrome without a profile.
>
> ### Residuals (block full V ACCEPT)
> 1. Prove 4-view shell with a real profile (or show sub-nav chrome even when
>    disconnected — product decision).
> 2. Optional: RustFS live transfer byte proof (Phase F-ish).
>
> Report: `report/T039-s3-tab-mockup-parity-report.md`
> Shots: `report-log/T039-shots/`

## Strategy

**Epic:** T042. Mockup: `docs/design/mockups/Chronos-S3-Tab.dc.html`.

Phase V = 4-view chrome + honest empty; Phase F = live transfers / RustFS.
