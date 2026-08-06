# Git Tab Milestone A Implementation Plan (T010)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `git.rs` placeholder with a live Git status panel: repository
discovery from a directory, status classified into Staged / Modified / Untracked,
per-file stage/unstage, and a commit bar — with automatic refresh via a `.git`
watcher and a follow/pin directory mode.

**Architecture:** A new `chronos-fm-services::git` module is a pure gix layer
(`RepoStatus`/`RepoEntry` + stage/unstage/commit operations), unit-tested against
temp repositories created with the system `git` CLI as fixtures. `GitPage`
(`crates/chronos-fm-pages/src/git.rs`) is rewritten into a real panel: header with
path/branch/Refresh, three sections, per-row actions, commit bar. It holds a
`WeakEntity<ExplorerPage>` for follow-mode (reuses the existing public
`ExplorerPage::current_path(cx)`) plus an optional pinned path. Status refreshes run
on the background executor (T009 pattern: channel + foreground poll); a `GitWatcher`
(notify, mirroring `ConfigWatcher`) watches the found repo's `.git` directory.

**Tech Stack:** `gix = { version = "0.86.0", features = ["tree-editor"] }` (gitoxide —
pure Rust, same as Zed/Yazi per user choice), `notify = "8"` (already a
`chronos-fm-services` dependency), `gpui_component::Input` (pattern:
`explorer/view/listing/search_bar.rs`), T002 patterns (`elevated_card`/`section_header`
in `chronos-fm-ui`), `tempfile` (already a dev-dependency).

**API ground truth:** All gix 0.86 API calls below were verified empirically in a
working probe (`crates/chronos-fm-services/src/git/mod.rs`, run as
`cargo test -p chronos-fm-services git::probe_full_flow`, which produced
`BRANCH=main / STATUS1 staged=[] modified=["a.txt"] untracked=["b.txt"] /
STAGE_OK / STATUS2 staged=["b.txt"] / COMMIT_OK / UNSTAGE_OK staged=["a.txt"]`).
The probe is replaced by the real service layer in Task 1.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-06-git-tab-live.md` (Milestone A scope
  only; branches/diff/push/pull/stash are later milestones).
- Milestones B/C are out of scope — no branch listing, no diff panel, no
  push/pull/stash in this plan.
- `unsafe_code = deny`, `clippy::unwrap_used`/`expect_used = warn` outside
  `#[cfg(test)]`.
- Every new `pub` item needs a doc comment (`missing_docs = "warn"`).
- No AI trailers in commits.
- Outside a git repo: friendly empty state, never a visible error.
- Git operations must never block the UI thread: all gix work in
  `background_executor` (T009 pattern).

---

### Task 1: `chronos-fm-services::git` service layer (status / stage / unstage / commit)

**Files:**
- Replace: `crates/chronos-fm-services/src/git/mod.rs` (probe → real layer)
- Modify: `crates/chronos-fm-services/Cargo.toml` (already: `gix = { version = "0.86.0", features = ["tree-editor"] }`)
- Modify: `crates/chronos-fm-services/src/chronos_fm_services.rs` (already has `pub mod git;`)

**Interfaces:**
```rust
/// One changed file, already split into the three status lists.
pub struct RepoEntry {
    /// Repository-relative path with `/` separators.
    pub path: String,
}

/// Snapshot of the working tree relative to HEAD/index.
pub struct RepoStatus {
    pub staged: Vec<RepoEntry>,
    pub modified: Vec<RepoEntry>,
    pub untracked: Vec<RepoEntry>,
    /// `shorten()`ed branch name; empty string if detached/initial.
    pub branch: String,
    /// Absolute workdir of the discovered repository.
    pub workdir: PathBuf,
    /// Absolute `.git` directory (for the watcher).
    pub git_dir: PathBuf,
}

/// Discover the nearest repository at or above `dir` (or from a file inside it).
pub fn open_repo(dir: &Path) -> Result<gix::Repository, GitError>;

/// Full status snapshot; `None` when `dir` is not inside any repository.
pub fn status(repo: &gix::Repository) -> Result<RepoStatus, GitError>;

/// Stage a repository-relative path (adds or updates its index entry).
pub fn stage_path(repo: &gix::Repository, rela_path: &str) -> Result<(), GitError>;

/// Unstage a repository-relative path (removes its index entry).
pub fn unstage_path(repo: &gix::Repository, rela_path: &str) -> Result<(), GitError>;

/// Commit all staged entries under `reference` (e.g. `"HEAD"` or
/// `"refs/heads/main"` for the first commit).
pub fn commit(repo: &gix::Repository, message: &str) -> Result<(), GitError>;
```

**Implementation notes (all verified):**

- `open_repo`: `gix::discover(dir)` returns `Result<gix::Repository>`; a
  `NotFound`-style error maps to `GitError::NotARepository`.
- `status`: `repo.status(gix::progress::Discard)` → `Platform`;
  `platform.into_iter(std::iter::empty::<gix::bstr::BString>())` → `Iter` of
  `Result<gix::status::Item>`. Match:
  - `Item::TreeIndex(ti)` → staged; path = `ti.location()`.
  - `Item::IndexWorktree(iw)` → `iw.summary()` from
    `gix::status::index_worktree::iter::Summary`:
    - `Some(Added)` → untracked (includes `DirectoryContents` untracked entries)
    - `Some(Modified | Removed | TypeChange | Conflict)` → modified
    - `_` (e.g. `NeedsUpdate`, renames when disabled) → skip.
  - Branch: `repo.head()?.referent_name().map(|n| n.shorten().to_string())`,
    `unwrap_or_default()`. Workdir: `repo.workdir()` (canonical). Git dir:
    `repo.git_dir()`.
- `stage_path` (index plumbing, all methods verified):
  1. `let mut file = repo.index()?.into_owned_or_cloned();` (unique snapshot).
  2. `let meta = gix::index::fs::Metadata::from_path_no_follow(&workdir.join(rela))?;`
  3. `let stat = gix::index::entry::Stat::from_fs(&meta)?;`
  4. `let data = std::fs::read(&workdir.join(rela))?;`
  5. `let blob_id = repo.write_object(&gix::objs::BlobRef { data: &data })?.detach();`
     (plain blob — filters/LFS are out of scope for v1; document in module docs).
  6. If an entry already exists for the path: mutate it in place via
     `entry_mut_by_path_and_stage(path, gix::index::entry::Stage::Unmerged)` —
     set `stat`/`id`/`flags`; else
     `file.dangerously_push_entry(stat, blob_id, gix::index::entry::Flags::empty(),
     gix::index::entry::Mode::FILE, gix::bstr::BStr::new(rela))`.
  7. `file.sort_entries();` then `file.write(Default::default())?;`
- `unstage_path`: `file.remove_entries(|_, path, _| path == BStr::new(rela))`,
  `file.sort_entries()`, `file.write(Default::default())`. (Verified: `remove_entries`
  signature is `FnMut(usize, &BStr, &mut Entry) -> bool`, true = remove.)
- `commit`:
  1. `let index = repo.index()?;`
  2. `let mut editor = repo.edit_tree(gix::ObjectId::empty_tree(repo.object_hash()))?;`
     (requires the `tree-editor` feature — already enabled).
  3. For each `entry in index.entries()`:
     `let kind = entry.mode.to_tree_entry_mode().map(|m| m.kind()).unwrap_or(EntryKind::Blob);`
     then `editor.upsert(gix::bstr::BStr::new(entry.path(&index)), kind, entry.id)?;`
  4. `let tree_id = editor.write()?.detach();`
  5. Identity: read `user.name`/`user.email` from `repo.config_snapshot()` (see
     sub-item), fall back to `"Chronos FM"` / `"chronos-fm@localhost"`.
  6. `let sig = gix::actor::SignatureRef { name: &BStr, email: &BStr, time: &str };`
     where `time` is formatted from
     `let now = gix::date::Time::now_local_or_utc(); let mut b = Vec::new();
     now.write_to(&mut b).unwrap(); String::from_utf8(b)?` (verified: `time` is a
     raw `&str`, not a `Time`).
  7. Non-first commit: `repo.commit_as(sig, sig, "HEAD", message, tree_id, [repo.head_id()?])?;`
  8. **First commit** (empty repo, `head()` errors): use
     `repo.commit_as(sig, sig, "refs/heads/main", message, tree_id, std::iter::empty())`
     — extend the probe test to cover this path and assert `git log` shows it.

**Tests** (`#[cfg(test)]`, system `git` CLI as fixture — pattern already in probe):
1. `status_classifies_modified_and_untracked` — init, commit `a.txt`, modify it, add
   `b.txt`; assert lists exactly `modified=["a.txt"]`, `untracked=["b.txt"]`,
   `branch="main"`, empty staged.
2. `stage_unstage_roundtrip` — stage `b.txt`, assert staged contains it and
   untracked is empty; unstage `a.txt`, assert staged drops it.
3. `commit_creates_commit_and_clears_staged` — stage `b.txt`, commit `"msg"`, assert
   `git log --oneline -1` message matches and staged is empty.
4. `first_commit_on_empty_repo` — init (no commits), stage, commit via
   `refs/heads/main`, assert `git log` shows it and `head()` now succeeds.
5. `outside_repo_returns_none` — status on a temp dir with no `.git` → `GitError::NotARepository`.
6. `stage_then_modify_then_unstage_keeps_worktree_file` — content untouched by
   stage/unstage (assert file bytes identical).

---

### Task 2: `GitWatcher` — `.git` directory watcher (services)

**Files:**
- Create: `crates/chronos-fm-services/src/git/watcher.rs` (or extend `git/mod.rs`; prefer separate file)
- Modify: `crates/chronos-fm-services/src/git/mod.rs` (`pub mod watcher;`)

**Interface (mirror `ConfigWatcher` exactly — same shape, different watch root):**
```rust
/// Owns the OS watch on a repository's `.git` directory. Dropping stops it.
pub struct GitWatcher { _watcher: notify::RecommendedWatcher }

impl GitWatcher {
    /// Watch `.git` recursively, invoking `on_change` for mutating events
    /// (create/modify/remove). Note: callback runs on a background thread.
    pub fn new(git_dir: &Path, on_change: impl Fn() + Send + 'static) -> notify::Result<Self>;
}
```

**Implementation notes:**
- `notify::recommended_watcher` + `watcher.watch(git_dir, RecursiveMode::Recursive)` —
  recursive because HEAD, `index`, `refs/`, and `logs/` all live under `.git`.
- Filter `is_relevant` on `EventKind::Create/Modify/Remove` (copy the helper from
  `chronos-fm-core/src/config/watcher.rs`; it's tiny — no cross-crate export needed,
  but note the duplication in a comment).
- `.git` may be a **file** (worktrees/submodules: `gitdir:` pointer). If `git_dir`
  is not a directory, watch its parent (the worktree's `.git` file) — v1: watch the
  parent non-recursively; note the limitation in docs.
- Unit test mirroring `ConfigWatcher::new_invokes_callback_on_config_change`:
  init temp repo, create watcher on its `.git`, touch `index`, expect channel ping.

---

### Task 3: `GitPage` — live status panel UI

**Files:**
- Replace: `crates/chronos-fm-pages/src/git.rs`

**State:**
```rust
pub struct GitPage {
    /// Explorer to follow in follow-mode (reads `current_path(cx)`).
    explorer: WeakEntity<ExplorerPage>,
    /// Follow-mode is default; when `Some`, the panel is pinned to this path.
    pinned_path: Option<PathBuf>,
    /// Last computed status; `None` = not yet loaded or outside a repo.
    status: Option<RepoStatus>,
    /// Whether the last refresh found no repository (drives the empty state).
    no_repo: bool,
    /// One-line error (e.g. commit failure), cleared on next successful op.
    error: Option<String>,
    /// Whether a background refresh is in flight (disable Refresh while running).
    refreshing: bool,
    /// Commit-bar input state (gpui-component Input).
    message_input: InputState,
    /// Backend kept alive for the window lifetime.
    _watcher: Option<GitWatcher>,
}
```

**Constructor:** `GitPage::new(explorer: WeakEntity<ExplorerPage>, window, cx)` —
starts the refresh loop and the watcher (recreated whenever the repo path changes,
same "store `_watcher` field" pattern as `RootView::_config_watcher`).

**Refresh flow (never blocks the UI):**
1. `refresh()` computes the current dir: `pinned_path.clone().unwrap_or_else(|| explorer.current_path(cx).into())`.
2. Spawn on `cx.spawn_in` with `background_executor` (T009 pattern): `open_repo`
   then `status` — both `Send`. Back on the foreground: update `status`/`no_repo`,
   restart the watcher if `git_dir` changed, `cx.notify()`.
3. Watcher ping → foreground poll loop (400ms debounce, channel pattern copied from
   `RootView::start_config_watch`) → `refresh()`.

**Render** (see T009 settings.rs for layout idioms: `elevated_card`/`section_header`):
- **Outside a repo / not loaded:** centered empty state — icon, "Не git-репозиторий",
  hint "Откройте папку с git-репозиторием или закрепите путь" + (when pinned) a
  "Следовать за Explorer" button.
- **Header card:** repo `workdir` (truncated to middle), branch chip
  (e.g. `⏵ main`), `Refresh` button (disabled while `refreshing`).
- **Three sections** (only render non-empty ones; hidden empty sections per plan
  decision, noted in spec §deferred): Staged / Modified / Untracked, each a header
  with count badge + a list. Row layout (mirror explorer listing row style):
  - Status-colored path label (green/staged, amber/modified, gray/untracked —
    use `theme::success`/`theme::warning`-ish accents already in `chronos-fm-ui`).
  - Staged rows → `Убрать` (unstage) button; Modified/Untracked rows → `Добавить`
    (stage) button. `on_click` calls the service op, then `refresh()`.
- **Commit bar** (bottom card): `gpui_component::Input` (pattern:
  `explorer/view/listing/search_bar.rs`) + `Commit` button. Commit disabled when
  `status.staged.is_empty()` or message empty. On success: clear input + refresh.
- **Error line:** `error` rendered as one-line status text (red) in the header,
  pattern `RootView::config_status`.

**Micro-interactions:** row hover background, button hover states, disabled-button
opacity — follow existing explorer/settings conventions.

---

### Task 4: Wire `GitPage` into `RootView`

**Files:**
- Modify: `crates/chronos-fm-pages/src/root.rs`

**Changes:**
1. `RootView::new`: `let git = cx.new(|cx| GitPage::new(explorer.downgrade(), window, cx));`
   — `explorer` is already created above it in `new`; `Entity::downgrade()` gives the
   `WeakEntity<ExplorerPage>`.
2. Trigger a refresh when switching to the Git page: in `set_page`, when
   `page == PageKind::Git`, `self.git.update(cx, |page, cx| page.refresh(cx));`
   (ensures follow-mode picks up the current explorer dir without waiting for the
   watcher).
3. No change to `render_active_page` (already routes `PageKind::Git`).

---

### Task 5: Verification

**Commands:**
- `cargo test -p chronos-fm-services git::` (Task 1 unit suite, requires `git` binary)
- `cargo check -p chronos-fm-pages` and `cargo test -p chronos-fm-pages` (render test:
  `git_page_renders_without_panicking` on a temp repo path, following the
  `settings_page_renders_without_panicking` pattern from T009)
- `cargo clippy -p chronos-fm-services -p chronos-fm-pages` (no new warnings in touched files)
- `cargo build --workspace`

**Manual / live check (architect):** run the app, open a real repo folder, verify:
status lists populate; stage/unstage move rows between sections; commit creates a
commit visible in `git log`; editing a tracked file outside the app re-paints the
panel within ~1s (watcher); navigating to a non-repo folder shows the empty state
with no error; pin mode survives explorer navigation; no UI freeze during large-repo
status (background execution).

## Deferred to Milestones B/C (per spec)

Branches list/switch/create, unified diff panel, push/pull (credential
helper/SSH via gix), stash, file-content filters/LFS on stage, worktree/submodule
edge cases in the watcher.
