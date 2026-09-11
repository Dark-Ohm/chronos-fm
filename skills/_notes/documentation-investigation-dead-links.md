# documentation-investigation — Dead Links

> Task 4 of skills-graph hardening. See [[checkpoint-skill]] for the full index.

## Problem

`documentation-investigation/SKILL.md` points to two skills that do not exist
in this repo:

- `technical-documentation-investigation` — not present locally (likely a
  renamed/global skill).
- `planning` — ambiguous; local equivalents are [[writing-plans]] and
  [[executing-plans]].

These are dead links. Per the hardening rule: **fix, don't delete.** Replace
with real local targets.

## Evidence

- Grep over `chronos/skills/**/SKILL.md`: no `name: technical-documentation-investigation`.
- `planning` resolves to [[writing-plans]] / [[executing-plans]] in this repo.

## Recommended fix

In `documentation-investigation` "Integration with Other Skills" section,
replace the two dead names with [[writing-plans]] and [[executing-plans]]
(and drop the `technical-documentation-investigation` reference, or point it
to [[philip-main]] if the intent was "audit docs").

## Status

- [ ] Edit `documentation-investigation/SKILL.md` to fix the two links
- [ ] Verify no remaining dead `related_skills` / cross-refs in the vault

## Related
- [[checkpoint-skill]] — Task 1
- [[engram-island]] — Task 2
- [[architecture-crates-ui-discrepancy]] — Task 3
- [[philip-main]] — likely the right target for "technical documentation"
