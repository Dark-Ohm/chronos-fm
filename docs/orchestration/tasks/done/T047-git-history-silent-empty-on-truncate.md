# T047 — Git History silently empty on truncate

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-10)
>
> Fix: history/commit_detail 256KB cap + trailing-record leniency + no
> `unwrap_or_default` on history Err. Tests 46/46; grim shows 50 real commits.
> Report: `report-log/T047-git-history-truncation-fix-report.md`.

**Source:** T046 sub-view grim discovery. **Related:** T038.
