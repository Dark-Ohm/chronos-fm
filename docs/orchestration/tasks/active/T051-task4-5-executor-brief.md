# T051 Tasks 4–5 — Executor brief (new agent)

> ## ⚖️ ARCHITECT (2026-08-11): **GO — finish T051 on worktree**
>
> Tasks **1–3 done + Task 3 review PASS**. You own **Task 4 + Task 5** only.
> Do **not** re-implement Tasks 1–3. Do **not** self-ACCEPT. Do **not**
> `git push` unless the user explicitly asks.

---

## Where the code lives

| Item | Value |
|------|--------|
| **Worktree** | `/home/neo/projects/chronos-ecosystem/Chronos-FM/.worktrees/t051-dnd` |
| **Branch** | `feat/t051-dnd` |
| **HEAD (start here)** | `d6bfc54` — *docs: T051 Task 3 review PASS — GO Task 4* |
| **main** | still plan-only (`84832f0`); **do not implement on bare main** |
| **Plan** | `docs/superpowers/plans/2026-08-10-dnd-in-app.md` (Task 4 §, Task 5 §) |
| **Spec** | `docs/superpowers/specs/2026-08-10-dnd-in-app-design.md` (APPROVE) |
| **Task 3 review** | `docs/orchestration/tasks/report-log/T051-task3-review.md` |

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM/.worktrees/t051-dnd
git log --oneline main..HEAD   # should start at d6bfc54
```

### Already on the branch (do not redo)

| Commit | What |
|--------|------|
| `8c1c633` | Shared `transfer_paths` + unique_name batch |
| `7d48f71` | Partial paste counts |
| `b72d1f0` / `fbb91bd` | FileDrop lifecycle + hardening |
| `e0da38c` | List/grid drag sources + pane drop targets |
| `d6bfc54` | Task 3 review PASS (docs only) |

Key modules: `crates/chronos-fm-pages/src/explorer/dnd.rs`,  
`view/listing.rs` / `row.rs` / `grid.rs`,  
`crates/chronos-fm-services/src/fs/ops.rs` (`transfer_paths`).

Baseline green at Task 3: `cargo test -p chronos-fm-pages --lib dnd` → **22/22**.

---

## Your scope

### Task 4 — Split-pane E2E + failure hardening

Follow plan **Task 4** steps exactly:

1. Production split-pane fixture (two real local panes / dirs).
2. End-to-end **Move** and **Ctrl-at-drop Copy** across panes.
3. Adversarial: invalid target, fall-through blocked, pending reject, partial failure status.
4. Full DnD + pages matrix green.
5. Commit only T051-touched files (no drive-by).

### Task 5 — Verification + report

1. `cargo fmt` scoped; no unrelated noise.
2. Focused + workspace tests + `cargo build --release -p chronos-fm`.
3. Live release evidence: `class=chronos-fm`, real two-pane fixture:
   - held-drag preview + target highlight
   - multi-file **cross-pane move** before/after
   - **Ctrl-copy** source remains + unique dest (FS evidence required; grim optional if FS is clear)
4. Write  
   `docs/orchestration/tasks/report/T051-dnd-in-app-report.md`  
   Claim → Evidence → Truth base. **No self-ACCEPT.**
5. Shots under `docs/orchestration/tasks/report-log/T051-*` if grims taken.

---

## Non-negotiables (from design)

- **Collision:** auto `unique_name` only — never overwrite (T053 later).
- **Ctrl at drop time** for Copy; plain = Move.
- **Local FS only** — provider/S3 panes reject.
- **No fall-through:** measured item bounds block listing cwd drop.
- **Same transfer helper** for Paste and Drop.
- **Cancel marquee** when file drag activates.
- **No** external DnD (T052), conflict UI (T053), undo (T054), progress UI (T056).
- **No** push origin; **no** merge to main yourself (Architect after ACCEPT).
- Stage **by file**, never whole directories.

---

## Skills

- `subagent-driven-development` or execute Tasks 4 then 5 inline in one agent if preferred
- `test-driven-development`, `gpui`, `verification-before-completion`, `rust-skills`

---

## Done when (executor)

- [ ] Task 4 committed on `feat/t051-dnd`
- [ ] Task 5 report + evidence committed
- [ ] `cargo test -p chronos-fm-pages --lib dnd` (and plan matrix) green
- [ ] release build green
- [ ] ticket left **awaiting Architect ACCEPT** (do not move to `done/`)

## Out of scope for this agent

- Architect ACCEPT / merge to main / push
- T052–T056
- Unrelated worktree dirt on main (`.workbuddy`, `docs/fable`, etc.)
