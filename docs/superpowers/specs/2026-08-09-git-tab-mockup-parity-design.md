# T038 Git Tab Mockup Parity — Design

**Date:** 2026-08-09  
**Status:** Approved direction; implementation pending written-spec review  
**Visual source of truth:** `docs/design/mockups/Chronos-Git-Tab.dc.html`  
**Code source of truth:** `crates/chronos-fm-pages/src/git.rs` and
`crates/chronos-fm-services/src/git/mod.rs`

## Goal

Bring the Chronos-FM Git tab to the Git mockup's content layout and density
while preserving the live T010 operations, and add the thin system-Git reads
and remote actions required for first-class History and Remotes views.

The Explorer/page shell remains T037's responsibility. No changes are planned
to the `Source` GPUI fork.

## Current facts and scope boundary

The existing Git page already provides status groups (staged, modified,
untracked), stage/unstage, commit, local branch list/create/checkout, unified
diff, push/pull, stash push/pop/drop, follow mode, pin mode, refresh, and busy
guards. The Git service already wraps system Git for push/pull/stash and has
unit-test helpers based on temporary repositories.

The service does not currently expose commit history/detail or configured
remote listing/management. Those are the only backend additions in this
feature. The implementation must not replace the existing `gix` repository
status path or alter the existing operation semantics.

## Design

### 1. Git service data and operations

Extend `crates/chronos-fm-services/src/git/mod.rs` with documented public data
objects and thin system-Git operations:

- `CommitEntry`: short/full hash as needed by the UI, subject, author, relative
  date, parent hashes, and tags where available.
- `CommitFileStat`: path plus additions/deletions; binary values are represented
  honestly when Git reports `-`.
- `CommitDetail`: selected commit metadata and file stats.
- `RemoteEntry`: remote name and fetch/push URLs, normalized so duplicate
  `git remote -v` rows become one remote object.
- `history(repo, limit)`: bounded `git log` query with a machine-readable field
  separator; parser handles empty output and malformed records as errors rather
  than inventing commits.
- `commit_detail(repo, hash)`: bounded `git show --numstat` query for the
  selected commit.
- `remotes(repo)`: parse `git remote -v` into normalized remote entries.
- `fetch(repo, remote)`: run system Git fetch for the selected remote.
- `delete_remote(repo, name)`: remove a configured remote through system Git.
- Amend support in the existing commit operation, preserving the current
  non-amend call path.

All system command output is passed through the existing error/truncation
conventions. Remote names and commit hashes must be validated/escaped by
passing them as separate `Command` arguments, never by shell interpolation.

Pure parsing helpers receive unit tests first. Tests cover empty output,
multiple parents, subjects containing separators, binary numstat rows,
fetch-only remotes, and fetch+push remotes.

### 2. Git page state and navigation

Extend `GitPage` with:

- `GitView::{Changes, History, Branches, Stashes, Remotes}`;
- bounded history entries and selected commit detail;
- normalized remotes;
- amend state and remote-name/URL input state where required;
- per-operation busy state compatible with the existing double-click guards.

Refresh collects the existing status, branches, stashes, bounded history, and
remotes in the background. A failure is surfaced with its concrete reason;
missing optional data must not silently masquerade as an empty repository.

### 3. Visual structure

The Git content area follows the mockup's hierarchy using the existing product
theme and local SVG assets:

- compact Git toolbar with repository path/branch context and Pull, Push,
  Refresh, and Pin actions;
- compact sub-navigation for Changes, History, Branches, Stashes, and Remotes,
  with honest badges derived from loaded data;
- Changes view with staged/unstaged/untracked groups, dense file rows, stage or
  unstage affordances, commit/amend box, and a separate unified-diff pane;
- History view with bounded commit list, selected commit detail, and file
  addition/deletion statistics;
- Branches view with local branches and any remote branch data already
  available from the service, plus existing create/checkout behavior;
- Stashes view with existing push/pop/drop and the already available Apply
  operation;
- Remotes view with remote cards, URLs, fetch/push/delete controls, and an Add
  Remote form wired to the service;
- honest loading, empty, and concrete-error states following the ratified T023
  empty-state rules.

Icons must come only from `crates/chronos-fm-ui/assets/icons/`. Colors must use
`chronos_fm_ui::theme`; the mockup's dark tokens guide hierarchy but do not
create a second palette.

### 4. Interaction and safety rules

- Existing stage, unstage, commit, branch, diff, push, pull, stash, follow,
  pin, refresh, and busy-guard behavior remains functional.
- New asynchronous operations use the same background-executor and foreground
  entity-update pattern already used by `GitPage`.
- Remote and commit identifiers are passed as argument values, not shell-built
  command strings.
- Errors remain visible until the next successful operation or refresh; no
  silent `unwrap`/`expect` paths are added.
- No fake history, remote, ahead/behind, author, timestamp, or file counts are
  rendered when the service cannot provide the value.

## Verification

1. Parser/service unit tests, including the new history/detail/remote cases.
2. Existing Git service tests and the full relevant crate test target.
3. `cargo fmt --check` and `cargo check -p chronos-fm-pages`.
4. Release build: `cargo build --release -p chronos-fm`.
5. Inspect the final diff to ensure only T038 files plus this spec/plan/report
   are included; preserve unrelated working-tree changes.
6. Live runtime evidence from the release binary and `grim`: Git Changes,
   History, Branches, Stashes, and Remotes states where available, compared
   side-by-side with the canonical mockup. If the environment cannot safely
   launch or exercise a state, report it as `UNVERIFIED` rather than infer it.

## Explicit residuals

Fancy merge-graph rendering, checkout/cherry-pick of historical commits, and
remote branch ahead/behind calculations are not included unless the existing
service data provides them without inventing values. Any such residual must be
listed in the final T038 report with path/line or command evidence.
