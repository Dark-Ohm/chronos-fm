# T038 — Git tab mockup parity — Implementation report

> ## ✅ ARCHITECT FINAL (2026-08-10): **ACCEPT / CLOSED** (Phase V)
>
> Vision pack complete on real repo (clean session): Changes / History /
> Branches / Stashes / Remotes — `report-log/T038-shots/t038_git_*.png`.
> Ticket → `done/T038-git-tab-mockup-parity.md`. Phase F mockup extras separate.

**Status:** IMPLEMENTED, NOT ACCEPTED — awaiting review. Do not treat this
report as a self-grant of ACCEPT; per session policy the executor does not
accept its own work.
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5)
**Design spec:** `docs/superpowers/specs/2026-08-09-git-tab-mockup-parity-design.md` (architect-approved 2026-08-09)

## Summary

Implemented the design spec's service-layer extension and page-layer
sub-navigation for the Git tab: `GitView::{Changes, History, Branches,
Stashes, Remotes}`, bounded commit history + on-demand commit detail,
normalized remotes with fetch/add/delete, and Commit/Amend segmented
control on the existing commit box. TDD throughout — every new service
function has unit tests written and run before/alongside the
implementation, matching the spec's "pure parsing helpers receive unit
tests first" requirement.

**What is verified:** service-layer correctness (25 new unit tests, all
green, real git-repo fixtures — no mocked git), full-workspace test suite
green, release build clean, diff scoped to exactly the two files this
ticket touches.

**What is NOT verified:** the Git tab's live visual appearance and
interactive behavior. See "Live verification — UNVERIFIED" below; this is
an honest gap, not an inferred pass.

## Service layer (`crates/chronos-fm-services/src/git/mod.rs`)

New public API, matching the design spec exactly:

| Item | What |
|---|---|
| `CommitEntry`, `CommitFileStat`, `CommitDetail`, `RemoteEntry` | Data types (design §1) |
| `history(repo, limit)` | Bounded `git log`, `\x1f`/`\x1e`-delimited pretty format (unlikely to collide with subject/author/ref-name content — verified with a subject containing literal `,`/`:`) |
| `commit_detail(repo, hash)` | `git show --numstat` + same pretty-format header; binary files parse additions/deletions to `None`, never `0` |
| `remotes(repo)` | `git remote -v`, deduped fetch/push rows into one `RemoteEntry` per name |
| `fetch(repo, remote)`, `delete_remote(repo, name)`, `add_remote(repo, name, url)` | Thin system-git wrappers; identifiers passed as separate `Command` arguments, never shell-interpolated |
| `commit_amend(repo, message)` | New commit replacing `HEAD`, same parents as the original — **not** a flag on `commit()`, a separate function (design: "preserving the current non-amend call path") |

### `commit_amend`'s real bug (found via TDD, not assumed)

The first implementation used `repo.commit_as(sig, sig, "HEAD", ...)` — the
same call shape as `commit()`'s non-amend path — and it failed every test
with `"Reference ... was not supposed to exist ... but actual content
was <original HEAD>"`. Traced into gix 0.86's `commit_as_inner`
(`repository/object.rs:411-424`): its ref-update safety check requires the
ref's **current** value to equal the **new commit's first parent**. That's
correct for an ordinary commit (parent == old HEAD) but always wrong for
an amend, whose parents are the *original* commit's own parents, not the
original commit itself. Fixed by writing the commit object directly
(`repo.write_object`) and moving `HEAD` via `repo.edit_reference` with an
explicit `PreviousValue::MustExistAndMatch(original_head_id)` — a real CAS
against the actual thing being replaced, not a parent-inferred one.

### Test coverage (25 new tests, `cargo test -p chronos-fm-services git::`)

Per design spec §"Verification" #1's explicit list — empty output,
multiple parents, subjects with separator-like characters, binary numstat
rows, fetch-only remotes, fetch+push remotes — plus amend-specific cases
(message replace, message-preserving, folds in newly-staged changes,
correct parent count). Pure-parser tests (`parse_history`,
`parse_commit_fields`) run with zero I/O; integration tests use real
temp-directory git repos via the existing `git()`/`git_output()`/
`repo_with_base_commit()` fixtures, matching the file's established
pattern — no new test infrastructure invented.

```
cargo test -p chronos-fm-services git::
test result: ok. 43 passed; 0 failed; 0 ignored
```

## Page layer (`crates/chronos-fm-pages/src/git.rs`)

- `GitView` enum + `select_view` action; `render_sub_nav` — a compact
  horizontal tab strip with honest count badges (changed-files total,
  history/branches/stashes/remotes counts) derived from already-loaded
  state, never invented.
- `refresh`'s background read now also loads `history(repo, 50)` and
  `remotes(repo)` alongside the existing status/branches/stashes read,
  best-effort (`unwrap_or_default`, matching the existing branches/stashes
  pattern — an empty result is not an error).
- `select_commit` loads `commit_detail` on demand when a History row is
  clicked; a stale in-flight read is discarded if the user selects a
  different commit before it lands (same guard pattern as `refresh`'s
  `refresh_generation`, applied per-selection here since History doesn't
  need a monotonic counter — only "is this still the selected hash").
- Changes view: existing Staged/Modified/Untracked sections + diff pane
  unchanged; the commit bar gained a Commit/Amend segmented control
  (`commit_mode_option`, explicit target-state set, not a naive toggle —
  clicking the already-active option is a no-op, not a flip). `commit()`
  branches on `self.amend` to call `commit_amend` instead of `commit`, and
  allows an empty message only in Amend mode (keeps the original message),
  matching design §"Interaction and safety rules".
- History view: bounded commit list + selected-commit detail card
  (subject, hash, per-file +/− stats, "binary" label instead of a fake
  `+0`/`−0` for binary files).
- Branches/Stashes views: **unchanged**, routed through the same
  `render_branches`/`render_stash_section` functions as before — reused,
  not rewritten, since they already had live create/checkout and
  push/pop/drop wired from T010.
- Remotes view: cards with fetch/push badges, Fetch/Delete actions, and an
  Add Remote form (two `Input`s + button) wired to `add_remote_action`.

## Explicit deviation from the mockup (flagging per design §"Explicit residuals")

The mockup's sub-navigation is a **left sidebar column** (repo head +
ahead/behind + nav list + footer with `origin`/`HEAD`/`gix` info). This
implementation uses a **compact horizontal tab strip** instead — same IA
(five views, honest badges), different geometry. Chosen for scope: the
mockup's sidebar also carries repo-identity chrome (`origin ·
N↑/N↓`, `HEAD <hash>`, `gix 0.68 · chronos-shell` version string) that
either doesn't exist as live data yet (ahead/behind counts) or would
duplicate the existing header bar's repo-path/branch display. Not
reconciling those without inventing ahead/behind numbers felt more honest
than a partial-fidelity sidebar. Flagging explicitly rather than silently
shipping a different layout than the approved mockup.

Also **not implemented** (all covered by design §"Explicit residuals" —
no service data exists for these, would require inventing values):
merge-graph rendering (mockup's `historyView` SVG lane graph), checkout/
cherry-pick of historical commits, remote branch ahead/behind, and the
mockup's separate `local`/`remote` branch two-column split (Branches view
still uses the pre-existing single local-branch list — no remote-branch
listing exists in the service; adding it was not in this ticket's approved
backend scope).

## Verification

| Check | Result |
|---|---|
| `cargo test -p chronos-fm-services git::` | 43/43 passed (25 new) |
| `cargo test --workspace --no-fail-fast` | 0 failed, all crates |
| `cargo build --release -p chronos-fm` | clean |
| `cargo check -p chronos-fm-pages` | clean (only pre-existing missing-docs warnings) |
| `cargo fmt --check` | **NOT run workspace-wide** — see below |
| Diff scope (`git status`) | exactly `crates/chronos-fm-pages/src/git.rs` + `crates/chronos-fm-services/src/git/mod.rs` |

### `cargo fmt` — residual, not silently skipped

`cargo fmt -- <files>` does not scope to the given files the way I
expected — it reformatted the *whole workspace* (51 files touched,
confirmed via `git status`), which violates the design spec's explicit
"only T038 files... preserve unrelated working-tree changes" rule. I
reverted every file except my two targets. My two files are therefore
**not guaranteed byte-identical to `rustfmt`'s output** — I hand-formatted
new code to match the surrounding style, but a real `cargo fmt --check`
on just these two files was not obtained without the collateral-damage
risk. Whoever has a safe way to scope rustfmt to two files (or is willing
to accept the workspace-wide reformat as a separate, disclosed commit)
should close this gap — did not want to guess and possibly ship an
un-disclosed unrelated-file diff.

### Live verification — UNVERIFIED (honest gap, not inferred)

Per design spec §"Verification" #6: "If the environment cannot safely
launch or exercise a state, report it as `UNVERIFIED` rather than infer
it."

- Confirmed the release binary launches cleanly and the **Explorer** tab
  (T037/T045 work) still renders correctly — no regression from adding
  fields to a sibling page.
- Could **not** interactively navigate to the Git tab to capture
  Changes/History/Branches/Stashes/Remotes grims. Root cause isolated,
  not just "didn't work": `hyprctl dispatch focuswindow` fails in this
  session's Lua-Hyprland config (`')' expected near 'class'`) — a known
  dead API in this environment (matches an existing ChronOS field note:
  "hyprctl dispatch мёртв в Lua-Hyprland 0.56.1"). Without a working focus
  dispatch, synthetic `ydotool` clicks land on whatever window already has
  focus (confirmed via `hyprctl activewindow`: a browser window, not
  chronos-fm) rather than the target. Tried against an easy, large,
  unambiguous target first (the Places sidebar's "Documents" row) before
  concluding this was an input-routing problem, not a coordinate-math
  problem specific to the Git nav icon.
- **No screenshot evidence exists for any Git sub-view.** The visual
  claims in this report (segmented control layout, badge styling, card
  layout) are description of the code, not confirmed rendering. Do not
  read "implemented" as "looks right" — that check is still open.

## Recommendation

Someone with working interactive input in this environment (or a
different sandbox) needs to click through all five views on a real repo
and grim each one against the mockup before this can be ACCEPTed. I did
not attempt to lower this bar or claim the visual check some other way.
