# T038 — Git tab: visual parity with design mockup

> ## ⚖️ ARCHITECT VERDICT: **DESIGN APPROVED — implement** (2026-08-09)
>
> Inventory facts OK (T010 ops live; no multi-view; no history/remotes API;
> mockup 5 views). **Option C (extend backend + full view shell)** approved
> as product-vision delivery — mockup is SoT to strive toward, not cut down.
> Residuals listed (merge-graph, etc.) OK.
>
> **Not** implementation ACCEPT. No code, no grim. Next: implementation
> plan (tests first) → code → release grim → vision. Spec
> `docs/superpowers/specs/2026-08-09-git-tab-mockup-parity-design.md` is
> **go** for executor (user may still comment; no block unless conflict).
>
> Ticket stays `active/`.


**Status:** DESIGN AGREED — implementation not started. This is an inventory +
design-approval report, not an implementation/acceptance report.

**Truth bases used:**
- Chronos-FM source: `crates/*`
- GPUI fork source: `/home/neo/projects/chronos-ecosystem/Source`
- Visual authority: `docs/design/mockups/Chronos-Git-Tab.dc.html`
- Ticket: `docs/orchestration/tasks/active/T038-git-tab-mockup-parity.md`
- Approved design: `docs/superpowers/specs/2026-08-09-git-tab-mockup-parity-design.md`

## Inventory (facts only)

### Current Git page (`crates/chronos-fm-pages/src/git.rs`)

- **Claim:** The page already implements the T010 operation set.
  **Evidence:** `git.rs` — `GitPage` struct (line 59), fields for status,
  branches, stashes, commit/branch/stash inputs, busy guard (lines 59–93);
  `push` (line 499), `pull` (line 524), `stash_push_action` (line 549),
  `stash_pop` (line 579), `stash_drop` (line 604), stage/unstage/commit/
  checkout/create-branch/show-diff handlers, and refresh loop.
  **Truth base:** Chronos-FM source.

- **Claim:** The render tree is a single vertical flow — header, branches,
  stash, file sections, commit bar, diff panel — with no multi-view shell.
  **Evidence:** `git.rs` render helpers: `render_header` (line 742 area),
  `render_branches`, `render_stash_section` (line 1001), `render_file_section`,
  `render_commit_bar`, `render_diff_panel` (line 922); `elevated_card` usage at
  lines 710, 776, 928, 1007.
  **Truth base:** Chronos-FM source.

- **Claim:** Unified diff is produced service-side.
  **Evidence:** `crates/chronos-fm-services/src/git/mod.rs:394`
  (`unified_diff`), `render_unified_diff` (line 489); page calls it at
  `git.rs:456`.
  **Truth base:** Chronos-FM source.

### Git service (`crates/chronos-fm-services/src/git/mod.rs`)

- **Claim:** History/log and remote listing/management are NOT provided.
  **Evidence:** public symbol inventory — `RepoStatus` (line 37), `StashEntry`
  (line 294), `list_branches` (line 303), `create_branch` (line 324),
  `checkout_branch` (line 364), `unified_diff` (line 394), `push` (line 569),
  `pull` (line 577), stash operations (lines 585–658). No commit-log or
  remote-list API exists.
  **Truth base:** Chronos-FM source.

### Mockup (`docs/design/mockups/Chronos-Git-Tab.dc.html`)

- **Claim:** The mockup specifies a five-view content shell with badges.
  **Evidence:** nav items at lines 588–593: `changes` (Changes, badge: changed
  count), `history`, `branches`, `stashes`, `remotes`; view subtitles lines
  628–633.
  **Truth base:** mockup HTML.

- **Claim:** The mockup specifies per-view toolbars and Actions view layout.
  **Evidence:** Changes toolbar (Stage all / Unstage all / Refresh, lines
  637–641), Commit/Amend segmented control (lines 700–702), diff pane with
  add/del coloring (lines 738–745), History commit rows + detail (lines
  760–807), Branches Local/Remote/Tags columns (lines 810–855), Stashes cards
  with Apply/Pop/Drop (lines 857–881), Remotes cards with Fetch/Push/Delete
  (lines 883–908).
  **Truth base:** mockup HTML.

## Approved design decisions

1. **Chosen scope: option C — extend backend.**
   Add thin system-Git reads for history and remotes: `history(repo, limit)`
   via bounded `git log`, `commit_detail` via `git show --numstat`,
   `remotes(repo)` via `git remote -v`, `fetch`, `delete_remote`, and amend
   support in the existing commit path. Parsing helpers get unit tests first.
   Identifiers are passed as separate `Command` arguments (no shell
   interpolation).
2. **UI:** internal `GitView::{Changes, History, Branches, Stashes, Remotes}`,
   compact view switcher with honest badges, Changes density + commit/amend box
   + separate diff pane, History list + selection detail, existing
   Branches/Stashes moved into views (plus stash Apply, already available
   service-side), Remotes cards + add form. Icons only from
   `crates/chronos-fm-ui/assets/icons/`; palette only `chronos_fm_ui::theme`.
3. **Safety:** T010 operations and busy guards preserved; errors stay visible;
   no fake history/remotes/ahead-behind values.
4. **Explicit residuals:** fancy merge-graph, checkout/cherry-pick of
   historical commits, remote ahead/behind — only if existing data provides
   them, else reported as residual with evidence.
5. **No Source fork changes required.**

Design document: `docs/superpowers/specs/2026-08-09-git-tab-mockup-parity-design.md`
(user review pending before implementation).

## Verification

- **Claim:** Implementation verified.
  **Evidence:** **Not claimed.** No code changes for T038 exist in the working
  tree at the time of this report.
  **Truth base:** none — not started.

- **Claim:** Visual parity proven by grim.
  **Evidence:** **Not claimed.** Requires release binary + grim after
  implementation, reviewed by a vision-capable model against the mockup.
  **Truth base:** T038 "Done when" criteria.

## Next gate

1. User reviews the approved design spec.
2. Write implementation plan (tests first: history/detail/remote parsers).
3. Implement backend + UI, run `cargo fmt --check`, targeted service/page
   tests, `cargo check -p chronos-fm-pages`, release build
   `cargo build --release -p chronos-fm`.
4. Live grim pack (Changes/History/Branches/Stashes/Remotes) + vision review.
5. Move this report to `report-log/` only after acceptance.
