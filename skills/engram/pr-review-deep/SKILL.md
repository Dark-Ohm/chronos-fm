---

name: engram-pr-review-deep
description: >
  Deep technical review protocol for Engram pull requests.
  Trigger: Reviewing any external or internal contribution before merge.
license: Apache-2.0
metadata:
  author: gentleman-programming
  version: "1.0"
---

> Part of the `chronos/skills` vault. For the project skill map and session orientation, see `start-here`; full Engram index in `catalog.md`.

## When to Use

Use this skill when:
- Evaluating PRs from contributors
- Reviewing risky refactors
- Deciding merge vs request-changes

---

## Review Protocol

1. Read full diff, not only summary.
2. Run relevant tests locally.
3. Validate API/contracts and migration safety.
4. Check docs against implementation.
5. Flag commit hygiene violations.

---

## Merge Gate

Merge only when:
- checks are green
- risk is understood
- blockers are resolved
- scope is coherent

Otherwise request changes with actionable items.
