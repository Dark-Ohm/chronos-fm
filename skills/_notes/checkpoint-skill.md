# Checkpoint — Skill Index (chronos/skills)

> Index note. Companion to [[start-here]] (session router). Updated 2026-07-17
> after waves audio/OSD/volume/MPRIS + field rules.

## Purpose

`start-here` routes a session. This file is a **static map** of skills in
`skills/` so a leaf skill can be placed in the graph without re-reading the
router.

## Project-specific (ChronOS)

| Skill | Role |
|---|---|
| [[start-here]] | Session entry: docs order, siblings, skill table |
| [[chronos-shell]] | Layout/wiring: crates, services, bar widgets, field rules |
| [[gpui-layer-shell]] | Layer-shell popup height / `window.resize` / clip bugs |
| [[gpui]] | Generic GPUI API (fork APIs, not ChronOS product logic) |

## Process

| Skill | Role |
|---|---|
| brainstorming | Design before implementation |
| writing-plans / executing-plans | Spec → plan → execute |
| subagent-driven-development | Independent tasks same session |
| dispatching-parallel-agents | 2+ independent tasks |
| test-driven-development | Tests before code |
| systematic-debugging | Root cause before fix |
| using-git-worktrees | Isolation (**ChronOS: sibling of repo**, not `/tmp`) |
| requesting-code-review / receiving-code-review | Review gate |
| verification-before-completion | Evidence before “done” (+ release UX) |
| finishing-a-development-branch | Merge/PR/cleanup |
| writing-skills / editing-skills | Author skills |
| using-superpowers | Invoke skills before winging |

## Docs / memory / other

| Skill | Role |
|---|---|
| philip-main | Docs from code evidence |
| documentation-investigation | Doc extraction across a tree |
| mcp-memory-workflow | lean-ctx / engram session protocol |
| dogfood | Web QA — rarely ChronOS |
| fable-discipline/* | fable-method / loop / judge / domain |
| rust-skills-master | Generic Rust rules |
| engram/* | **Sibling Engram product** skills — not ChronOS shell architecture |

## Outside `skills/` but live on this machine

- `hindsight-self-hosted` — real Hindsight stack (bank `chronos-ecosystem`);
  do **not** use stale hindsight-local/cloud (`uvx hindsight-embed`).
- Repo canon: `HANDOFF.md`, `ARCHITECTURE.md`, `DECISIONS.log`, `MEMORY.md`,
  `AGENTS.md`.

## Authority

On conflict: repo docs > skills > dialogue memory > Hindsight RAG.
