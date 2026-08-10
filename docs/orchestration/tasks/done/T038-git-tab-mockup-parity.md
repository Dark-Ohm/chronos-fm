# T038 — Phase V: Git tab smart pixel-copy / function

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (Phase V, 2026-08-10)
>
> ### Code — ACCEPT (prior)
> Service history/remotes/amend + page sub-nav; horizontal nav deviation OK.
>
> ### Vision — ACCEPT (populated repo path)
> Grims under `report-log/T038-shots/` (`class=chronos-fm`), session state
> cleared so explorer cwd = repo (not restored Pictures):
> - `t038_git_changes.png` — Modified/Untracked + Commit/Amend
> - `t038_git_history.png` — 50 commits (T047 fix live)
> - `t038_git_branches.png` — main + Create & switch
> - `t038_git_stashes.png` — honest empty + stash push
> - `t038_git_remotes.png` — origin fetch/push + add remote form
>
> Empty not-a-repo path earlier: `T046-shots/t046_git.png`.
>
> ### Phase F residual (not blocking V)
> Merge-graph density, remote-tracking branches, cherry-pick — mockup extras.
>
> ### Smoke note
> Populated Git proof needs clean session (`state.redb` aside) or path on a
> repo; restore_tabs can land on non-repo (e.g. Pictures).
>
> Report: `report/T038-git-tab-mockup-parity-report.md`


## Strategy: smart pixel-copy (Phase V) first

**Agreed (client + architect):** mockup = product vision. Live may lag.
Delivery: **Phase V = visual/IA copy**, then **Phase F = function fill**.

### Phase V (THIS ticket — Must)

1. Layout/IA/density **match mockup** (shell chrome, nav, panels, empty states).
2. Wire **already-real** data/actions only (no fake history/transfers/progress).
3. Views without backend: **render chrome + honest empty/disabled** ("needs …"), do not delete from IA.
4. Icons: `crates/chronos-fm-ui/assets/icons/` only — not mockup SVG paths.
5. Theme: ChronOS dark tokens (`chronos.theme.json` / `chronos_fm_ui::theme`).
6. Proof: release binary + grim + **vision** review vs mockup.
7. Do **not** mix mega-backend (multipart, git log engine, plugin host) into V PR.

### Phase F (residual / follow-up tickets)

Backend + full interactivity for mockup-only views. Separate tickets after V ACCEPT.

### Non-negotiables

| Rule | Detail |
|------|--------|
| Source truth | `/home/neo/projects/chronos-ecosystem/Source` + `crates/*` — not Zed, not datasets |
| Source edits | Only for the better; commit+report **what / why / зачем** |
| Facts only | Claim → Evidence → Truth base; UNVERIFIED ≠ ACCEPT |
| Executor | **Vision/omnimodal** for visual ACCEPT |
| No secrets | in report/grim/config (S3 keys = keyring only) |


## Phase V scope (this ticket)

| View | V behavior |
|------|------------|
| Shell | Toolbar (branch chip, pull/push, refresh) + left sub-nav |
| **Changes** | Full density: staged/unstaged/untracked + commit bar + diff — **live T010 ops** |
| **History** | Layout + empty/disabled or stub list if no API — **no fake commits** |
| **Branches** | Live branch list/create/checkout in mockup layout |
| **Stashes** | Live stash UI in mockup layout |
| **Remotes** | Layout + empty/disabled until Phase F |

## Phase F residual (not V)

`git log` / commit detail / remotes list-add-delete / amend / merge-graph — separate ticket after V.
Design note option C remains **Phase F roadmap**, not V gate.

## Done when (V)

Grim of Git tab vs mockup: chrome + Changes/Branches/Stashes match; History/Remotes present honestly; T010 ops still work; vision ACCEPT V.

## Related

T036 · T010 live base · design `docs/superpowers/specs/2026-08-09-git-tab-mockup-parity-design.md` (F reference)
