# T010-C — Milestone C: push / pull / stash

> ## ✅ ARCHITECT VERDICT: **ACCEPT** (2026-08-09)
>
> Implementation matches design + plan conditions C1–C10 / P1–P7.
> Architect re-ran `cargo test -p chronos-fm-services --lib -- git::` →
> **24 passed**. `cargo check -p chronos-fm-pages` green.
>
> **Caveats (non-blocking residuals):**
> - P8 partial: `stash@{N}` prefix checked but `index` still sequential
>   `entries.len()` — fine while `git stash list` order is stable.
> - Push/Pull not greyed when `status.is_none()` (only `busy`) — click fails
>   via service error; optional polish.
> - **Live** push/pull/stash not claimed in this report (unit+build only).
>
> Ticket Milestone C closed for implementation; optional live C + syntect
> remain product residuals. Report → `report-log/`.

**Status:** READY FOR ACCEPTANCE
**Date:** 2026-08-09
**Commit:** pending architect stamp

## Summary

Milestone C adds push, pull, and stash operations to the Git tab. All
operations shell out to system `git` binary (consistent with existing
`checkout_branch` from Milestone B), using argv-only invocations (no
shell). Push/pull buttons appear in the page header; stash section
renders between branches and staged files.

## Service Layer (`crates/chronos-fm-services/src/git/mod.rs`)

| Function | Git command | Notes |
|----------|------------|-------|
| `push(repo, remote)` | `git push <remote> HEAD` | C1 |
| `pull(repo, remote)` | `git pull --ff-only <remote> HEAD` | C2 |
| `stash_push(repo, msg)` | `git stash push [-m <msg>]` | C5: no `-m` when empty |
| `stash_pop(repo, idx)` | `git stash pop stash@{<idx>}` | P3: format binding |
| `stash_apply(repo, idx)` | `git stash apply stash@{<idx>}` | C10: service only |
| `stash_drop(repo, idx)` | `git stash drop stash@{<idx>}` | |
| `stash_list(repo)` | `git stash list` | C9: parse `WIP on` + `On` |

Shared infrastructure:

- `run_git_cmd(workdir, args)` — C3 (`.current_dir`), C4 (argv only),
  P1 (stdout+stderr combined)
- `truncate_4k(s)` — P2 (`floor_char_boundary` for UTF-8 safety)
- `StashEntry { index, branch, message }` — parsed from `stash list`

### New Tests (5)

1. `push_to_bare_remote` — push to bare repo, verify commit visible (P5: separate dirs)
2. `pull_from_remote` — pull from bare repo after external push
3. `stash_push_pop_roundtrip` — stash modified tracked file, pop restores (P4: no untracked)
4. `stash_list_and_drop` — list entries, drop by index, verify removal
5. `stash_push_empty_message` — empty `&str` → `git stash push` without `-m`

All 23 tests pass (18 pre-existing + 5 new).

## Page Layer (`crates/chronos-fm-pages/src/git.rs`)

### New Fields
- `busy: bool` — C6: blocks double-click on push/pull/stash
- `stashes: Vec<StashEntry>` — fetched in `do_refresh`
- `stash_input: Entity<InputState>` — new input for stash message

### Header (P6/P7)
Push/Pull buttons added after Refresh/Pin group, using `cx.listener`:

```
[↻ Refresh] [📌 Pin]  |  [Pull] [Push]
```

- `render_header` now takes `busy: bool` parameter
- Both buttons grey out when `busy` (opacity 0.6, muted colors)

### Stash Section (C8, C10)
Always rendered when `status.is_some()` (C8), between branches and staged files:

```
┌─ Stash (N entries) ────────────────────────┐
│ stash@{0}: main -- fix sidebar    [Pop] [Drop] │
│ stash@{1}: main -- add tests     [Pop] [Drop] │
│                                               │
│ [Stash message_____] [Stash push]              │
└───────────────────────────────────────────────┘
```

- Pop and Drop only (C10); Apply is in service layer for future use
- `render_action_button` — shared helper with busy-aware styling

### Busy Guard
- All three async operations (`push`, `pull`, `stash_push_action`) check `if self.busy { return; }` (C6)
- `self.busy = false` set **before** result check — resets on both success and error
- `stash_pop` / `stash_drop` also guarded

## Verification

```
cargo check -p chronos-fm-pages   → 0 errors, 6 warnings (pre-existing)
cargo test -p chronos-fm-services -- git::tests → 23 passed, 0 failed
cargo clippy -p chronos-fm-services --all-targets → 0 new warnings
cargo build -p chronos-fm         → success
```

## Out of Scope (explicitly deferred)

- Remote picker (always `"origin"`)
- `--force` push
- Merge on pull (only `--ff-only`)
- Stash conflict resolution (errors bubble as `GitError::Operation`)
- Syntect highlighting (B residual)
- TTY password prompt (C7: stderr on failure)
- Page render tests (deferred in spec)

## Architect Conditions (C1–C10)

| Cond | Description | Status |
|------|-------------|--------|
| C1 | `git push origin HEAD` | ✅ |
| C2 | `git pull --ff-only origin HEAD` | ✅ |
| C3 | `.current_dir(workdir)` | ✅ |
| C4 | argv only, no shell | ✅ |
| C5 | empty stash msg → no `-m` | ✅ |
| C6 | busy blocks double-click | ✅ |
| C7 | no TTY password UI | ✅ |
| C8 | stash always when status.is_some() | ✅ |
| C9 | parse `WIP on` / `On` | ✅ |
| C10 | Pop+Drop in UI, Apply in service | ✅ |

## Plan Fixes (P1–P8)

| Fix | Description | Status |
|-----|-------------|--------|
| P1 | `run_git_cmd` combines stdout+stderr | ✅ |
| P2 | `truncate_4k` uses `floor_char_boundary` | ✅ |
| P3 | `stash@{i}` format binding | ✅ |
| P4 | stash test no untracked | ✅ |
| P5 | bare remote: separate workdir+bare dirs | ✅ |
| P6 | `render_header` as free fn | ✅ |
| P7 | `busy: bool` param + `cx.listener` | ✅ |
| P8 | parse `stash@{N}` from string | ✅ |
