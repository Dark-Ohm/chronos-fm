# Chronos-FM — agent cheatsheet (canon shortcuts)

**Read first:** `HANDOFF.md` (top checkpoint) · `AGENTS.md`.

## Roles
- **Architect:** decide, stamp ACCEPT, ticket hygiene — not default coder.
- **Executor:** implement + evidence — **never self-ACCEPT**.
- **Client:** mockups / priority.

## Canon paths
| Need | Path |
|------|------|
| Session resume | `HANDOFF.md` |
| Rules | `AGENTS.md` |
| Decisions | `docs/DECISIONS.log` |
| Queue | `docs/orchestration/tasks/active/` |
| Done / reports / grims | `…/done/` · `…/report/` · `…/report-log/` |
| Mockups (UI truth) | `docs/design/mockups/*.html` |
| gpui truth | `../Source/` only |
| Specs / plans | `docs/superpowers/specs/` · `plans/` |

## Two epics
| Epic | About | Next |
|------|--------|------|
| **T042** | Pixel-copy tabs Phase V | T039 residual |
| **T048** | Explorer essentials | **T052** external DnD |

## Shipped essentials (do not re-litigate)
| Ticket | Feature | Tests filter |
|--------|---------|--------------|
| T049 | File-op keys | `--lib keybindings` |
| T050 | Marquee select | `--lib marquee` |
| T051 | In-app DnD | `--lib dnd` |

**DnD collision v1:** auto `unique_name` (like paste). Overwrite UI = T053.

## Workflow loop
1. Design → Architect APPROVE  
2. Plan → Architect GO (subagent-driven if multi-task)  
3. Implement in worktree if large; tests + release grims  
4. Report Claim→Evidence; **no self-ACCEPT**  
5. Architect ACCEPT → merge main → optional push **only if user asks**

## Build / run (Linux)
```bash
cargo build --release -p chronos-fm
./target/release/chronos-fm --page=explorer
# subviews: --page=git:history  settings:appearance  s3:transfers
script/dev/t046_page_smoke.sh 'git:history' /tmp/grim.png
```
Hyprland: window **class=`chronos-fm`** (`app_id` in UI window options).

## Hygiene
- Stage **by file**, never whole `active/`.
- No secrets in config/report/grim (S3 keys → keyring).
- No fake progress/history for screenshots.
- Source edits: what / why / зачем only when better for Chronos-FM + fork.

## Resume one-liner for new agent
> Read HANDOFF #8. main @ 09ea4cc. T049–T051 ACCEPT. Next product P1: **T052**.
> Queue in `docs/orchestration/tasks/active/`. Push only if user asks.
