# T010-C — Git push / pull / stash (design spec)

**Date:** 2026-08-09
**Status:** approved-with-conditions (architect review 2026-08-09)
**Depends on:** T010-A (stage/commit), T010-B (branches/diff)

## 0. Architect review (2026-08-09)

**Verdict: APPROVE with normative conditions below.** Approach matches
Milestone B (`checkout_branch` → system `git`). Do **not** reimplement
HTTP/SSH/credential helpers in gix for v1.

### Conditions (must land in implementation)

| # | Topic | Requirement |
|---|---|---|
| C1 | **Push refspec** | Use `git -C <workdir> push <remote> HEAD` (not bare `git push <remote>`). Avoids “no upstream” failure on first push of a local-only branch. Optional later: `-u` only when no upstream — not required for v1. |
| C2 | **Pull refspec** | Use `git -C <workdir> pull --ff-only <remote> HEAD` (or `pull --ff-only <remote> <current-branch>`). Bare `git pull --ff-only origin` fails when upstream is unset. |
| C3 | **Workdir** | All `Command`s set `.current_dir(repo.workdir())` (same as checkout). Prefer also prefixing with `git -C` only if workdir is None edge case — workdir required. |
| C4 | **No shell** | Args as separate `Command` args only. Stash message never goes through `sh -c`. `stash@{N}` as single argv element. |
| C5 | **Empty stash message** | If `message.trim().is_empty()`, run `git stash push` **without** `-m` (git’s default message). Do not pass `-m ""`. |
| C6 | **In-flight UI** | While any push/pull/stash op runs: set a `busy: Option<&'static str>` (or reuse `refreshing` + label) so buttons are disabled and double-click cannot queue parallel network ops. |
| C7 | **Credentials / TTY** | Document in UI error path: interactive password prompts may hang or fail (no TTY). Prefer system credential helper / SSH agent; surface stderr if git exits non-zero. No modal password UI in C. |
| C8 | **Stash section always visible** when `status.is_some()` — not “only when non-empty or focused”. Focus-tracking for empty-stash is brittle in GPUI; empty list + push row is fine. |
| C9 | **stash_list parse** | Accept both `stash@{N}: WIP on <branch>: <msg>` and `stash@{N}: On <branch>: <msg>`. Unknown lines: skip or single entry with raw remainder — never panic. |
| C10 | **Apply vs Pop** | Spec lists both; **v1 UI ships Pop + Drop only** (apply optional later). Keeps row actions to two buttons as in wireframe. Service may still implement `stash_apply` for tests/completeness. |

### Non-blocking notes

- `Result<String>` = combined stdout+stderr trimmed for success toast / error body — good; cap length (~4 KiB) if noisy.
- Remote UI selection remains out of scope; page hardcodes `"origin"`.
- Syntect still out of scope (B residual).
- Page unit tests for buttons deferred — OK; service bare-remote fixtures required and match existing git test style.

## 1. Approach

All new operations delegate to the **system `git` binary** via
`std::process::Command` — the same pattern already used for
`checkout_branch` in Milestone B. This is the only approach that
handles credential helpers, SSH-agent, and `.netrc` out of the box
without fragile reimplementation of transport protocols in Rust.

Remote is hard-coded to `"origin"` at the **page** layer (v1 scope);
service functions still take `remote: &str` for testability. Pull uses
`--ff-only` to avoid merge conflicts; the user resolves divergence
manually. Push/pull refspecs follow **C1/C2** above.

## 2. New service functions (`crates/chronos-fm-services/src/git/mod.rs`)

| Function | Signature | System command |
|---|---|---|
| `push` | `fn push(repo: &gix::Repository, remote: &str) -> Result<String, GitError>` | `git push <remote> HEAD` |
| `pull` | `fn pull(repo: &gix::Repository, remote: &str) -> Result<String, GitError>` | `git pull --ff-only <remote> HEAD` |
| `stash_push` | `fn stash_push(repo: &gix::Repository, message: &str) -> Result<String, GitError>` | `git stash push` [`-m <message>` if non-empty] |
| `stash_pop` | `fn stash_pop(repo: &gix::Repository, index: usize) -> Result<String, GitError>` | `git stash pop stash@{<index>}` |
| `stash_apply` | `fn stash_apply(repo: &gix::Repository, index: usize) -> Result<String, GitError>` | `git stash apply stash@{<index>}` (service; UI optional) |
| `stash_list` | `fn stash_list(repo: &gix::Repository) -> Result<Vec<StashEntry>, GitError>` | `git stash list` + parse |
| `stash_drop` | `fn stash_drop(repo: &gix::Repository, index: usize) -> Result<String, GitError>` | `git stash drop stash@{<index>}` |

### 2.1 `StashEntry` struct

```rust
pub struct StashEntry {
    pub index: usize,
    pub branch: String,
    pub message: String,
}
```

Parsed from `git stash list` output (format: `stash@{0}: WIP on <branch>: <message>`).

### 2.2 Error handling

All functions return `GitError::Operation(String)` on non-zero exit,
with stderr included in the message. The caller (git page) surfaces
errors inline in the status bar.

## 3. UI changes (`crates/chronos-fm-pages/src/git.rs`)

### 3.1 Push + Pull buttons in header

Two new buttons in `render_header`, aligned with Refresh/Pin on the
right side:

```
[↻ Refresh] [📌 Pin]  │  [↓ Pull] [↑ Push]
```

- Disabled (opacity 0.5, no cursor) when `status.is_none()` **or**
  `busy.is_some()` (C6)
- Pull label: `"↓ Pull"`; Push label: `"↑ Push"` (remote is always origin)
- On click: set `busy`, spawn background task → clear busy, refresh
  status/branches/stashes after completion
- Success: optional short status in `error`-slot restyled as info, or
  clear error; non-zero git exit → `error` with stderr
- Error output shown in `error` field (inline below header)

### 3.2 Stash section

New section between Branch list and Staged files; **always shown when
`status.is_some()`** (C8):

```
┌─ Stash (N entries) ──────────────────────────────┐
│ stash@{0}: WIP on main: fix sidebar    [Pop][Drop]│
│ stash@{1}: WIP on feature: add tests   [Pop][Drop]│
│                                                    │
│ [________________] [Stash push]                    │
└────────────────────────────────────────────────────┘
```

- Empty list: section still visible with “No stashes” line + push row
- Each row: description + **Pop** / **Drop** only (Apply deferred)
- Bottom: Input (placeholder: "Stash message") + "Stash push"
- List refreshed inside `do_refresh()` via `stash_list`
- Pop/Drop/Push set `busy` and refresh after completion

## 4. Refresh integration

`do_refresh()` adds `git::stash_list(&repo).unwrap_or_default()` to
the background task, alongside `status` and `branches`. Store in a
new `stashes: Vec<StashEntry>` field on `GitPage`.

## 5. Testing

### 5.1 Service tests (unit)

In `mod.rs` test module:

- `push_to_bare_remote` — create bare repo, commit locally, push, verify remote has commit
- `pull_from_remote` — push to bare, clone to temp, fetch+ff via pull
- `stash_push_pop_roundtrip` — dirty worktree, stash push, verify clean, stash pop, verify restored
- `stash_list_empty_and_populated` — list returns Vec, entries match stash state
- `stash_drop_removes_entry` — push two stashes, drop index 0, verify list shrinks

### 5.2 Page tests (existing)

Existing page tests in `git.rs` remain green. New page-level tests for
push/pull buttons are deferred — they require a bare remote fixture
and are flaky on CI without git binary preinstalled (already the case
for checkout tests).

## 6. Out of scope

- Remote selection UI (v1: `"origin"` only)
- `--force` push
- Merge/rebase on pull (v1: `--ff-only` only)
- Stash branch creation / `git stash branch`
- `git stash --keep-index` / `--include-untracked`
- In-app password / SSH passphrase modal (C7)
- `stash_apply` in the UI (service OK)
- Setting upstream (`-u`) heuristics beyond `push … HEAD`

## 7. Implementation gate

Implement only after this file’s **Status** is
`approved-with-conditions` (this revision). Ticket text in
`active/T010-git-tab-live.md` should point here for Milestone C.
