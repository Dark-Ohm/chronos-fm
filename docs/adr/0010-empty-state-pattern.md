# 0010 — Empty-state pattern for Chronos-FM pages

> Status: Accepted  
> Date: 2026-08-09  
> Related: ChronOS T252 (ratified 2026-08-05); Chronos-FM T023

## Context

Card chrome is unified (`elevated_card` / `section_header`, T002). What to
show when data is missing was never written down. ChronOS solved the same
problem in T252 with six absence classes and an honesty rule.

## Decision

1. **Adopt** the ChronOS T252 six-class empty-state language for all
   `chronos-fm-pages` surfaces (see full text in
   [`docs/DECISIONS.log`](../DECISIONS.log) § 2026-08-09 — T023).
2. **Do not** introduce a mandatory shared empty-state helper in T023;
   cases differ by design.
3. **Accept** current page behaviour audited in DECISIONS.log (including
   Git hiding empty file sections; S3 NoProfiles + Open Settings; Git
   Loading… / Not a repository).
4. Record long-form decisions in **`docs/DECISIONS.log`** going forward;
   keep **ADR stubs** for discoverability (this file is the first of that
   dual path).

## Consequences

- New pages must pick a class from the six and stay honest (§13).
- Violations are fixed in focused tickets, not a repo-wide rewrite.
- Optional later: soft-cap banners, search “no matches,” `muted_note`
  helper only if ≥3 identical sites appear.
