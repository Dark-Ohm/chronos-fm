# AGENTS — Chronos-FM

Project-specific agent rules. Read **`HANDOFF.md` first** on a new session
(latest checkpoint is at the top). Generic coding defaults may still live in
`.rules` / tool config; **this file wins** on Chronos-FM product process.

## What this repo is

Rust **file manager** (gpui / ChronOS design language) under
`/home/neo/projects/chronos-ecosystem/Chronos-FM`.  
Shared UI fork: **`/home/neo/projects/chronos-ecosystem/Source`** via path-deps
`../Source/*` in workspace `Cargo.toml`.

## Roles

| Role | Does |
|------|------|
| **Client** | Product vision (mockups), priorities |
| **Architect** | Decide, accept/reject reports, ticket hygiene — not default implementer |
| **Executor** | Implement, release binary, grim, evidence — **no self-ACCEPT** |

## Canonical sources of truth

| Concern | Authority |
|---------|-----------|
| Product UI vision | `docs/design/mockups/*.html` (shell: `chronos-file-manager.dc.html`) |
| gpui / component API | **`Source/` only** — not Zed, not RAG/datasets, not “memory” |
| App behavior | `crates/*` + live config/runtime |
| Session memory | `HANDOFF.md` (top checkpoint) + `docs/DECISIONS.log` |
| Work queue | `docs/orchestration/tasks/active/` |
| Agent cheatsheet | `docs/orchestration/CHEATSHEET.md` |

If mockup HTML and an older written spec disagree → **HTML wins** until
spec is re-extracted.

## Programs (locked)

### T042 — Phase V pixel-copy (tabs)
Match mockup layout/IA/density; real data only; missing API → empty/disabled.
Children: T037 shell, T038 Git, T039 S3, T040 Settings, T041 Extensions.
**T039** still PARTIAL (profile / 4-view residual).

### T048 — Explorer essentials (Thunar parity)
Daily muscle memory. **T049 keys · T050 marquee · T051 in-app DnD = ACCEPT.**
Next: **T052** external DnD → T053 conflict → T054 undo → T055/T056 polish.

### Policies shipped with T051
- Move default; **Ctrl at drop time** = Copy.
- Name conflict: **`unique_name`** (no overwrite) until T053 dialog.
- Paste and Drop share `chronos_fm_services::fs::ops::transfer_paths`.

## Evidence (ACCEPT)

```
Claim: …
Evidence: path:lines | command+exit | grim path | config key (no secrets)
Truth base: Source | Chronos-FM | mockup | runtime | config
```

- Visual ACCEPT: **release binary + grim** of `class=chronos-fm` **and** vision.
- Unit green alone is not enough for window/UX.
- Do not file browser/other app screenshots as Chronos-FM evidence.

## Source edits

Allowed when they make Chronos-FM (and the shared fork) **better**
(bugfix, missing API, layout/theme hook, crash, perf).  
Each Source change: **what / why / for what** in commit + report.  
No silent fork churn, no Zed copy-paste as authority.

## Orchestration hygiene

- Tickets: `docs/orchestration/tasks/{active,done,report,report-log,rejected}/`
- Architect stamps reports; executors do not self-close ACCEPT.
- Stage **by file**, never whole `active/` directories.
- Prefer worktrees for multi-task features; merge FF after ACCEPT.
- `git push` only when the user asks.

## Build / run (Linux Hyprland)

```bash
cargo build --release -p chronos-fm
./target/release/chronos-fm   # class chronos-fm
# pages smoke: script/dev/t046_page_smoke.sh git:history /tmp/x.png
```

Prefer verifying GPU path (Vulkan) and window mapping before claiming UI done.

## Do not

- Invent APIs from Zed docs.
- Fake S3 transfer progress or git history for screenshots.
- Put AWS secrets in `config.toml` or reports (keyring only).
- Drop mockup sections because “live is thinner”.
- Push origin or force-push without explicit user request.

## Quick links

- `HANDOFF.md` — session checkpoint (read first)
- `docs/orchestration/CHEATSHEET.md` — one-page resume
- `docs/DECISIONS.log` — ratified decisions
- `docs/design/mockups/README.md` — mockup index
- `docs/architecture.md` — crate map
- `docs/orchestration/tasks/active/` — current work
