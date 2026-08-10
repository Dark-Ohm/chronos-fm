# T046 — `--page=page:sub` extension + history defect found

> ## ✅ ARCHITECT FINAL (2026-08-10): **ACCEPT / CLOSED**
>
> `--page=page:sub` **ACCEPT**. History empty defect → **T047 ACCEPT**.
> T046 closed; hypr residual waived. See `done/T046-shared-visual-proof.md`.

**Status:** IMPLEMENTED, NOT ACCEPTED. Defect described below is
**reported, not fixed** — out of this pass's scope, flagged for triage.
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5)
**Follows:** T046 PARTIAL-ACCEPT verdict — "Next (recommended):
`--page=settings:appearance` / `git:history` / … — закрывает sub-view
grims без hypr."

## 1 — `--page=page:sub` mechanism

Extended the T046 flag to a second, optional `:<sub>` segment:

- `GitView::from_cli_name`, `S3View::from_cli_name`,
  `SettingsCategory::from_cli_name`, `ExtensionsView::from_cli_name` — one
  pure parser per page, same shape as `PageKind::from_cli_name`. Settings
  uses short keys (`"preview"`, not `"Preview Panel"`) to match the other
  three pages' single-word vocabulary.
- Each page gained `pub(crate) fn set_initial_subview(&mut self, name:
  &str, cx)` — warns and leaves the default sub-view on an unrecognized
  name, same contract as an invalid `--theme`/`--page`.
- `Cli::initial_subview()` + a pure `split_page_arg` helper
  (`"settings:appearance"` → `("settings", Some("appearance"))`;
  `"git:"` → `("git", None)`, a trailing empty sub is absent, not `""`).
- `RootView::new` takes `initial_subview: Option<String>`, dispatches it
  to whichever page `initial_page` named right after constructing that
  page's entity; the other three pages are untouched.

```
cargo test -p chronos-fm-pages          → 103 passed; 0 failed
cargo test -p chronos-fm                → 13 passed; 0 failed (6 new: split/initial_subview)
cargo test --workspace --no-fail-fast   → 0 failed anywhere
cargo build --release -p chronos-fm     → clean
```

## 2 — Real sub-view grims

`script/dev/t046_page_smoke.sh 'settings:appearance' …` and `'git:history'
…` — both `GRIM_OK`, zero log errors.

- **`settings:appearance`** (`/tmp/t046_settings_appearance.png`) —
  Appearance category renders correctly: Theme mode (Dark active),
  Accent swatches (blue selected, visible border ring), **Icon pack**
  buttons (new-this-epic control, Nerd Font shown active — this
  environment's real `config.toml` has `icon_pack = "nerd"`, not a
  rendering artifact), then the five unwired rows with "not wired yet"
  pills. Matches T040's own description.
- **`git:history`** (`/tmp/t046_git_history.png`) — sub-nav renders
  correctly (Changes 14 / **History** active / Branches 1 / Stashes /
  Remotes 1) against the real repo at this process's cwd
  (`/home/neo/projects/chronos-ecosystem/Chronos-FM`, branch `main`).
  **This grim surfaced a real defect — see §3.**

Observation, not a new defect: the "active" state's white/very-light
background (Theme mode "Dark", Icon pack "Nerd Font", History tab,
sort-column "Modified" in the earlier default-view grim) reads oddly
against the mockup's blue accent — but the code for all of these
(`mode_button`/`sort_button`/`split_button`/`render_sub_nav`) predates
this session's T038/T040 work and just calls `theme::accent(cx)`, same as
everywhere else. Not touched, not diagnosed further — flagging in case
it's worth its own look, not claiming it's a defect.

## 3 — Defect found: Git History is empty for any repo with a non-trivial log, not a real "no commits"

**`git::history()` (`chronos-fm-services/src/git/mod.rs:836`) silently
returns an empty list for large `git log` output — including this very
repo (174 real commits) — because its output goes through the same
`run_git_cmd` → `truncate_4k` path as every other git subcommand, and a
truncated `RECORD_SEP`-delimited stream breaks `parse_commit_fields`'s
exact-7-field check on the cut-off final record.**

Confirmed, not guessed:

```
$ git log -50 --pretty=format:$'%h\x1f%H\x1f%s\x1f%an\x1f%ar\x1f%P\x1f%D\x1e' | wc -c
8879
```

`truncate_4k` caps combined stdout+stderr at 4096 bytes (`run_git_cmd`,
line ~727) — 8879 > 4096, so the last record in a 50-commit history is
cut mid-field. `parse_commit_fields` then hits its "expected 7 fields"
branch and returns `Err(GitError::Operation(...))`, `parse_history`
propagates that `Err` for the whole call (not just the broken record),
and `do_refresh` in `git.rs` does:

```rust
let history = git::history(&repo, HISTORY_LIMIT).unwrap_or_default();
```

— the exact same "best-effort, empty-repo and real-error look identical"
masking pattern the T038 code comment explicitly defends ("a repo with no
commits ... is not an error, matches `unwrap_or_default()`"). That
reasoning is correct for an *empty* repo; it's wrong for a *truncated*
one — the UI shows "no commits yet" for a repo that has 174 commits,
which is a false claim on screen, not an honest empty state.

**Blast radius:** any repo whose `git log -<HISTORY_LIMIT>` output (with
this specific 7-field format string) exceeds ~4KB — commonly true past
roughly 15-30 commits depending on subject-line length, i.e. most real
repos past their first few days. This is very likely the common case, not
an edge case; T038's History view has probably never actually shown
history for a repo this codebase's own size. `commit_detail` uses the
same `run_git_cmd`/4K cap and could have the same failure mode for a
commit with a large diff, though I did not confirm that one.

**Not fixed here** — out of T046's scope (a CLI/dev-tooling ticket) and a
real design question, not a one-line patch: `run_git_cmd`'s 4K cap is
presumably a deliberate defense (T038/history context mentions "Bounded
`git show --numstat`" for `commit_detail`, i.e. bounding output size is
intentional). Raising the cap just moves the cliff; the more correct fix
is likely either (a) a larger, `history`-specific cap since a 50-line log
is expected to be a few KB, or (b) surfacing the parse/truncation failure
as a real error in the UI instead of `unwrap_or_default()`-ing it into a
false empty state. Flagging for the architect to decide the shape, not
guessing at it.

## Recommendation

1. `--page=page:sub` mechanism: ready for review, same as the base
   `--page=` flag.
2. The Git History defect (§3) is real and worth its own ticket —
   candidate name: **T047 — Git History silently empty for
   `git log` output over 4KB (truncate_4k swallows a real parse error)**.
   Suggest fixing `run_git_cmd`'s cap-vs-error-visibility tradeoff for
   `history` (and checking `commit_detail`) before T038 can honestly claim
   the History view works on anything but a small repo.
