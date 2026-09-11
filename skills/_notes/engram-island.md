# Engram Island — 21 Orphaned Skills

> Task 2 of skills-graph hardening. See [[checkpoint-skill]] for the full index.

## Problem

All 21 skills under `engram/` declare `related_skills: []` and reference no
peer skill. They are a disconnected island inside `chronos/skills/`. An agent
entering any `engram/*` skill has no path to [[start-here]] or any other node.

This contradicts the design goal: *an agent entering any skill must be able to
find the path to the others.*

## Inventory (21)

architecture-guardrails, backlog-triage, branch-pr, business-rules,
commit-hygiene, cultural-norms, dashboard-htmx, docs-alignment,
gentleman-bubbletea, issue-creation, memory-protocol, plugin-thin,
pr-review-deep, project-structure, sdd-flow, server-api, testing-coverage,
tui-quality, ui-elements, visual-language — plus the `engram` parent.

## Open question

Are these intentionally a separate project's skill set (Engram) nested here?
If yes, they may belong in their own vault/tree, not under `chronos/skills/`.
If they should participate in the chronos graph, each needs a reverse pointer
to [[start-here]] (or [[checkpoint-skill]]).

## Status

- [ ] Decide: own tree vs. wired into chronos graph
- [ ] If wired: add reverse pointer to each (minimal, like [[chronos-shell]] did)

## Related
- [[checkpoint-skill]] — Task 1
- [[architecture-crates-ui-discrepancy]] — Task 3
- [[documentation-investigation-dead-links]] — Task 4
