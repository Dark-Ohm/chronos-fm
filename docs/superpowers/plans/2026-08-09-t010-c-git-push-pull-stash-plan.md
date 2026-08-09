# T010-C — Git push / pull / stash Implementation Plan

> **For agentic workers:** Execute task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add push, pull, and stash operations to the Git page using system `git` binary.

**Architecture:** All operations shell out to `git` via `Command::new("git").args([...]).current_dir(workdir)`. Push/pull buttons in header; stash section between branches and staged files. Busy guard prevents double-click.

**Tech Stack:** Rust + `gix` (existing), `std::process::Command` (system git)

**Status:** architect-reviewed 2026-08-09 — **APPROVED with plan fixes below** (see § Architect review). Implement only after applying those fixes to the code samples in Tasks 1–5.

## Architect review (2026-08-09)

**Verdict: APPROVE with fixes.** C1–C10 are correctly restated. Several code samples would fail compile or tests as written — treat the following as **normative overrides** of the snippets:

| ID | Issue | Fix |
|---|---|---|
| P1 | **Git success often on stderr** (`Everything up-to-date`, `To url…`) | Shared helper: on success return `truncate_4k(stdout + stderr)`; on failure prefer stderr, fallback stdout. |
| P2 | **`truncate_4k` char boundary** | `&s[..4096]` can panic mid-UTF-8 → use `s.floor_char_boundary(4096)` (Rust 1.80+) or `chars().take(...)`. |
| P3 | **`run_stash_cmd` + `format!` temporary** | `run_stash_cmd(repo, &["stash", "pop", &format!(...)])` does not compile. Bind `let r = format!("stash@{{{index}}}");` then pass `&r`. |
| P4 | **`stash_push_pop_roundtrip` untracked** | Default `git stash push` does **not** remove untracked. Drop `new.txt` from the test (only dirty tracked `a.txt`). Do not use `-u` (out of scope). |
| P5 | **`push_to_bare_remote` layout** | Prefer separate dirs: `workdir/` + `bare.git/` under `tempdir` (not bare as child of the same tree as `init_repo` if init order confuses). Ensure `init_repo` runs on the non-bare workdir only. |
| P6 | **`render_header` is a free function** | Pass `busy: bool` into `render_header(...)` — snippets must not use `self.busy` inside free fns. |
| P7 | **Listeners** | Wire clicks with `cx.listener(...)` (existing git.rs pattern), not raw `Fn` into `on_mouse_down` without listener. |
| P8 | **Index from `stash@{N}`** | Prefer parsing `N` from the line when present; fallback to `entries.len()`. |

Optional polish (non-blocking): `busy: Option<&'static str>` for status label (“Pushing…”); keep `bool` if simpler.

## Global Constraints

- C1: `git push origin HEAD` — not bare `git push origin`
- C2: `git pull --ff-only origin HEAD` — works without upstream configured
- C3: `.current_dir(workdir)` on every Command
- C4: Only argv, no shell (`Command::new("git").args([...])`, no `sh -c`)
- C5: Empty stash message → skip `-m` flag
- C6: `busy` flag prevents double-click on push/pull/stash
- C7: No TTY password UI; show stderr on failure
- C8: Stash section ALWAYS when `status.is_some()` (not only when non-empty)
- C9: Parse both `WIP on` and `On` in stash list
- C10: UI: Pop + Drop only; Apply only in service layer (not exposed in UI)

---

### Task 1: Service — `push` + `pull`

**Files:**
- Modify: `crates/chronos-fm-services/src/git/mod.rs`

**Interfaces:**
- Consumes: `gix::Repository`, `GitError` (existing)
- Produces: `pub fn push(repo: &gix::Repository, remote: &str) -> Result<String, GitError>`, `pub fn pull(repo: &gix::Repository, remote: &str) -> Result<String, GitError>`

- [ ] **Step 1: Add `push` function**

```rust
/// Run `git` in the repo workdir; combine stdout+stderr (P1); map status (C7).
fn run_git(repo: &gix::Repository, args: &[&str]) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(workdir) // C3
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| GitError::Operation(format!("git {}: {e}", args.first().unwrap_or(&"?"))))?;
    let combined = {
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        let err = String::from_utf8_lossy(&out.stderr);
        if !err.trim().is_empty() {
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(err.trim_end());
        }
        s.trim().to_string()
    };
    if !out.status.success() {
        return Err(GitError::Operation(if combined.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            combined
        }));
    }
    Ok(truncate_4k(&combined)) // P1
}

/// Push current branch to `remote` (defaults to `"origin"` when empty).
/// C1: pushes `HEAD` explicitly so no upstream tracking is needed.
pub fn push(repo: &gix::Repository, remote: &str) -> Result<String, GitError> {
    let remote = if remote.is_empty() { "origin" } else { remote };
    run_git(repo, &["push", remote, "HEAD"]) // C1, C4
}
```

- [ ] **Step 2: Add `pull` function**

```rust
/// Fetch + fast-forward using remote `HEAD` into the current branch.
/// C2: uses `<remote> HEAD` so no upstream tracking is required.
pub fn pull(repo: &gix::Repository, remote: &str) -> Result<String, GitError> {
    let remote = if remote.is_empty() { "origin" } else { remote };
    run_git(repo, &["pull", "--ff-only", remote, "HEAD"]) // C2
}
```

- [ ] **Step 3: Add `truncate_4k` helper (P2: char-safe)**

```rust
fn truncate_4k(s: &str) -> String {
    if s.len() <= 4096 {
        return s.to_string();
    }
    let end = s.floor_char_boundary(4096);
    format!("{}…\n[truncated {} bytes]", &s[..end], s.len() - end)
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check -p chronos-fm-services
```

Expected: `Finished` with 0 errors.

---

### Task 2: Service — `stash_push` + `stash_pop` + `stash_apply` + `stash_list` + `stash_drop`

**Files:**
- Modify: `crates/chronos-fm-services/src/git/mod.rs`

**Interfaces:**
- Produces: `pub struct StashEntry { pub index: usize, pub branch: String, pub message: String }`, `pub fn stash_push(...)`, `pub fn stash_pop(...)`, `pub fn stash_apply(...)`, `pub fn stash_list(...)`, `pub fn stash_drop(...)`

- [ ] **Step 1: Add `StashEntry` struct**

```rust
/// A single stash entry parsed from `git stash list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    pub index: usize,
    pub branch: String,
    pub message: String,
}
```

- [ ] **Step 2: Add `stash_push`**

```rust
/// Push working-tree changes onto the stash stack.
/// C5: when `message` is empty, omit the `-m` flag.
pub fn stash_push(repo: &gix::Repository, message: &str) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let mut cmd = std::process::Command::new("git");
    cmd.args(["stash", "push"]);                     // C4
    let trimmed = message.trim();
    if !trimmed.is_empty() {
        cmd.args(["-m", trimmed]);                   // C5
    }
    let out = cmd
        .current_dir(workdir)                         // C3
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| GitError::Operation(format!("git stash push: {e}")))?;
    if !out.status.success() {
        return Err(GitError::Operation(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),  // C7
        ));
    }
    Ok(truncate_4k(&String::from_utf8_lossy(&out.stdout).trim()))
}
```

- [ ] **Step 3: Add `stash_pop` + `stash_apply` + `stash_drop`**

```rust
/// Pop (apply + drop) stash entry at `index`.
pub fn stash_pop(repo: &gix::Repository, index: usize) -> Result<String, GitError> {
    let r = format!("stash@{{{index}}}"); // P3: bind temporary
    run_git(repo, &["stash", "pop", &r])
}

/// Apply stash entry at `index` without dropping. C10: service-only, no UI.
pub fn stash_apply(repo: &gix::Repository, index: usize) -> Result<String, GitError> {
    let r = format!("stash@{{{index}}}");
    run_git(repo, &["stash", "apply", &r])
}

/// Drop stash entry at `index` without applying.
pub fn stash_drop(repo: &gix::Repository, index: usize) -> Result<String, GitError> {
    let r = format!("stash@{{{index}}}");
    run_git(repo, &["stash", "drop", &r])
}
```

Prefer reusing `run_git` (P1) instead of a second helper.

- [ ] **Step 4: Add `stash_list` with C9 parsing**

```rust
/// Parse `git stash list` into `StashEntry` vec.
/// C9: handles both `WIP on <branch>: <msg>` and `On <branch>: <msg>`.
pub fn stash_list(repo: &gix::Repository) -> Result<Vec<StashEntry>, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let out = std::process::Command::new("git")
        .args(["stash", "list"])
        .current_dir(workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| GitError::Operation(format!("git stash list: {e}")))?;
    if !out.status.success() {
        return Err(GitError::Operation(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format: "stash@{0}: WIP on main: fix bug"
        //       or "stash@{0}: On main: fix bug"
        let after_colon = match line.find(": ") {
            Some(pos) => &line[pos + 2..],
            None => continue,
        };
        let (branch, message) = if let Some(rest) = after_colon.strip_prefix("WIP on ") {
            // "WIP on <branch>: <message>"
            parse_branch_msg(rest)
        } else if let Some(rest) = after_colon.strip_prefix("On ") {
            // "On <branch>: <message>"
            parse_branch_msg(rest)
        } else {
            ("unknown".to_string(), after_colon.to_string())
        };
        // P8: prefer parse stash@{N}; fallback sequential.
        let index = line
            .strip_prefix("stash@{")
            .and_then(|s| s.split('}').next())
            .and_then(|n| n.parse().ok())
            .unwrap_or(entries.len());
        entries.push(StashEntry {
            index,
            branch,
            message,
        });
    }
    Ok(entries)
}

fn parse_branch_msg(rest: &str) -> (String, String) {
    match rest.find(": ") {
        Some(pos) => (rest[..pos].to_string(), rest[pos + 2..].to_string()),
        None => (rest.to_string(), String::new()),
    }
}
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p chronos-fm-services
```

Expected: `Finished` with 0 errors.

---

### Task 3: Service — Unit tests

**Files:**
- Modify: `crates/chronos-fm-services/src/git/mod.rs` (test module)

- [ ] **Step 1: `push_to_bare_remote` test**

```rust
#[test]
fn push_to_bare_remote() {
    let td = tempdir().unwrap();
    let root = std::fs::canonicalize(td.path()).unwrap();
    let bare = root.join("bare.git");
    git(&root, &["init", "--bare", "-q", "bare.git"]);
    init_repo(&root);
    std::fs::write(root.join("a.txt"), "one").unwrap();
    git(&root, &["add", "a.txt"]);
    git(&root, &["commit", "-q", "-m", "init"]);
    git(&root, &["remote", "add", "origin", bare.to_str().unwrap()]);

    let repo = open_repo(&root).unwrap();
    let out = push(&repo, "origin").unwrap();
    assert!(!out.is_empty(), "push should produce output: {out}");

    // Verify bare repo has the commit.
    let log = git_output(&bare, &["log", "--oneline"]);
    assert!(log.contains("init"), "bare repo should have commit: {log}");
}
```

- [ ] **Step 2: `pull_from_remote` test**

```rust
#[test]
fn pull_from_remote() {
    let td = tempdir().unwrap();
    let root = std::fs::canonicalize(td.path()).unwrap();
    let bare = root.join("bare.git");
    git(&root, &["init", "--bare", "-q", "bare.git"]);
    // First repo: commit + push.
    init_repo(&root);
    std::fs::write(root.join("a.txt"), "v1").unwrap();
    git(&root, &["add", "a.txt"]);
    git(&root, &["commit", "-q", "-m", "c1"]);
    git(&root, &["remote", "add", "origin", bare.to_str().unwrap()]);
    git(&root, &["push", "-q", "origin", "HEAD:refs/heads/main"]);

    // Second repo: clone then pull.
    let clone = root.join("clone");
    git(&root, &["clone", "-q", bare.to_str().unwrap(), clone.to_str().unwrap()]);

    // Push a second commit from first repo.
    std::fs::write(root.join("a.txt"), "v2").unwrap();
    git(&root, &["add", "a.txt"]);
    git(&root, &["commit", "-q", "-m", "c2"]);
    git(&root, &["push", "-q", "origin", "HEAD:refs/heads/main"]);

    // Pull from clone.
    let repo = open_repo(&clone).unwrap();
    let out = pull(&repo, "origin").unwrap();
    assert!(!out.is_empty(), "pull should produce output");

    let log = git_output(&clone, &["log", "--oneline"]);
    assert!(log.contains("c2"), "clone should have c2 after pull: {log}");
}
```

- [ ] **Step 3: `stash_push_pop_roundtrip` test**

```rust
#[test]
fn stash_push_pop_roundtrip() {
    let (_td, root) = repo_with_base_commit();
    // Dirty tracked file only (P4: default stash does not include untracked).
    std::fs::write(root.join("a.txt"), "modified").unwrap();

    let repo = open_repo(&root).unwrap();
    let out = stash_push(&repo, "test stash").unwrap();
    // Message may be on stderr — combined output (P1).
    assert!(
        out.contains("Saved working directory") || out.contains("WIP") || !out.is_empty(),
        "{out}"
    );

    // Tracked worktree restored.
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        "one",
        "stash should restore tracked file"
    );

    let out = stash_pop(&repo, 0).unwrap();
    assert!(
        out.contains("Dropped") || out.contains("modified") || !out.is_empty(),
        "{out}"
    );
    assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "modified");
}
```

- [ ] **Step 4: `stash_list_and_drop` test**

```rust
#[test]
fn stash_list_and_drop() {
    let (_td, root) = repo_with_base_commit();
    std::fs::write(root.join("a.txt"), "v1").unwrap();
    let repo = open_repo(&root).unwrap();
    stash_push(&repo, "first").unwrap();
    std::fs::write(root.join("a.txt"), "v2").unwrap();
    stash_push(&repo, "second").unwrap();

    let entries = stash_list(&repo).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].message, "second");
    assert_eq!(entries[1].message, "first");

    stash_drop(&repo, 0).unwrap();
    let entries = stash_list(&repo).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].message, "first");
}
```

- [ ] **Step 5: `stash_push_empty_message` test (C5)**

```rust
#[test]
fn stash_push_empty_message() {
    let (_td, root) = repo_with_base_commit();
    std::fs::write(root.join("a.txt"), "v").unwrap();
    let repo = open_repo(&root).unwrap();
    // Should not fail — C5: empty message → no -m flag.
    stash_push(&repo, "").unwrap();
    stash_push(&repo, "  ").unwrap();
    let entries = stash_list(&repo).unwrap();
    assert_eq!(entries.len(), 2);
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p chronos-fm-services -- git::tests
```

Expected: all tests pass (5 new + all existing green).

---

### Task 4: Page — `busy` guard + push/pull buttons

**Files:**
- Modify: `crates/chronos-fm-pages/src/git.rs`

- [ ] **Step 1: Add `busy` field to `GitPage`**

In the struct definition, add after `selected_diff`:

```rust
    /// C6: prevents double-click on push/pull/stash operations.
    busy: bool,
```

In `GitPage::new`, add: `busy: false,`

- [ ] **Step 2: Add push/pull action methods**

```rust
    fn push(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy { return; }                    // C6
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor()
                    .spawn(async move {
                        let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                        let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                        git::push(&repo, "origin").map_err(|e| e.to_string())
                    }).await;
                this.update_in(&mut cx, |page, _window, cx| {
                    page.busy = false;
                    if let Err(msg) = result {
                        page.error = Some(format!("push: {msg}"));
                    }
                    page.refresh(cx);
                }).ok();
            }
        }).detach();
    }

    fn pull(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy { return; }                    // C6
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor()
                    .spawn(async move {
                        let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                        let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                        git::pull(&repo, "origin").map_err(|e| e.to_string())
                    }).await;
                this.update_in(&mut cx, |page, _window, cx| {
                    page.busy = false;
                    if let Err(msg) = result {
                        page.error = Some(format!("pull: {msg}"));
                    }
                    page.refresh(cx);
                }).ok();
            }
        }).detach();
    }
```

- [ ] **Step 3: Extend `render_header` signature with `busy` (P6)**

```rust
fn render_header(
    status: Option<&RepoStatus>,
    no_repo: bool,
    error: Option<String>,
    refreshing: bool,
    busy: bool, // P6
    cx: &mut Context<GitPage>,
) -> impl IntoElement
```

Pass `self.busy` from `render_body`. Disable Push/Pull when `busy || status.is_none()`.

- [ ] **Step 4: Add push/pull buttons (P7: `cx.listener`)**

After Pin/Refresh group:

```rust
.child(
    div().flex().items_center().gap(px(8.))
        .child(
            div()
                .px(px(10.)).py(px(4.)).rounded(px(6.)).text_sm()
                .when(busy || status.is_none(), |this| {
                    this.bg(theme::border(cx)).text_color(theme::muted(cx)).opacity(0.6)
                })
                .when(!busy && status.is_some(), |this| {
                    this.bg(theme::bg_hover(cx)).text_color(theme::fg(cx))
                        .hover(|s| s.bg(theme::border(cx)))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _e, w, cx| {
                            this.pull(w, cx);
                        }))
                })
                .child("↓ Pull"),
        )
        .child( /* same for ↑ Push → this.push */ ),
)
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p chronos-fm-pages
```

Expected: `Finished` with 0 errors.

---

### Task 5: Page — Stash section + stash integration

**Files:**
- Modify: `crates/chronos-fm-pages/src/git.rs`

- [ ] **Step 1: Add `stashes` field + refresh integration**

Add field to `GitPage` struct:
```rust
    stashes: Vec<chronos_fm_services::git::StashEntry>,
```
Init in `new`: `stashes: Vec::new(),`

In `do_refresh`, add `stash_list` alongside status/branches. Change the background task to:
```rust
let result = cx.background_executor().spawn(async move {
    let repo = git::open_repo(&dir)?;
    let status = git::status(&repo)?;
    let branches = git::list_branches(&repo).unwrap_or_default();
    let stashes = git::stash_list(&repo).unwrap_or_default();
    Ok::<_, GitError>((status, branches, stashes))
}).await;
```

Update the match arm:
```rust
Ok((status, branches, stashes)) => {
    page.stashes = stashes;
    // ... existing status/branches assignment
}
```

- [ ] **Step 2: Add stash action methods**

```rust
    fn stash_push_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy { return; }                    // C6
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        let message = self.branch_input.read(cx).text().to_string();
        // Re-use branch_input for stash message? No — let me check. We need a separate
        // stash_input field. Actually per spec, stash has its own input.
        // Wait — looking at existing code, branch_input is for branch name.
        // We need a NEW input for stash message. Let me add stash_input.

        // Actually, let me re-read: the plan uses a SEPARATE stash_input.
        // Marking this as needing stash_input entity. See Step 3.
    }
```

Wait — I need to add a separate `stash_input` entity. Let me fix the steps:

- [ ] **Step 2: Add `stash_input` entity field**

In `GitPage` struct, add after `branch_input`:
```rust
    stash_input: Entity<InputState>,
```

In `new`, initialize:
```rust
let stash_input = cx.new(|cx| {
    let mut state = InputState::new(window, cx);
    state.set_placeholder("Stash message", window, cx);
    state
});
```
Add to struct init: `stash_input,`

- [ ] **Step 3: Add stash action methods**

```rust
    fn stash_push_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy { return; }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        let message = self.stash_input.read(cx).text().to_string();
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor().spawn(async move {
                    let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                    let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                    git::stash_push(&repo, &message).map_err(|e| e.to_string())
                }).await;
                this.update_in(&mut cx, |page, window, cx| {
                    page.busy = false;
                    if let Err(msg) = result {
                        page.error = Some(format!("stash: {msg}"));
                    } else {
                        page.stash_input.update(cx, |input, cx| {
                            input.set_value("", window, cx);
                        });
                    }
                    page.refresh(cx);
                }).ok();
            }
        }).detach();
    }

    fn stash_pop(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy { return; }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor().spawn(async move {
                    let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                    let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                    git::stash_pop(&repo, index).map_err(|e| e.to_string())
                }).await;
                this.update_in(&mut cx, |page, _window, cx| {
                    page.busy = false;
                    if let Err(msg) = result {
                        page.error = Some(format!("stash pop: {msg}"));
                    }
                    page.refresh(cx);
                }).ok();
            }
        }).detach();
    }

    fn stash_drop(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy { return; }
        self.busy = true;
        cx.notify();
        let dir = self.current_dir(cx);
        cx.spawn_in(window, move |this: WeakEntity<Self>, cx: &mut AsyncWindowContext| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor().spawn(async move {
                    let dir = dir.ok_or_else(|| "no repo dir".to_string())?;
                    let repo = git::open_repo(&dir).map_err(|e| e.to_string())?;
                    git::stash_drop(&repo, index).map_err(|e| e.to_string())
                }).await;
                this.update_in(&mut cx, |page, _window, cx| {
                    page.busy = false;
                    if let Err(msg) = result {
                        page.error = Some(format!("stash drop: {msg}"));
                    }
                    page.refresh(cx);
                }).ok();
            }
        }).detach();
    }
```

- [ ] **Step 4: Add stash section render**

In `render_body`, after `render_branches` and before `render_file_section("Staged", ...)`:

```rust
.when_some(status.as_ref(), |el, _s| {
    el.child(render_stash_section(
        &self.stashes,
        self.stash_input.clone(),
        self.busy,
        cx,
    ))
})
```

Add `render_stash_section` function:

```rust
fn render_stash_section(
    entries: &[chronos_fm_services::git::StashEntry],
    stash_input: Entity<InputState>,
    busy: bool,
    cx: &mut Context<GitPage>,
) -> impl IntoElement {
    // C8: always render when status.is_some()
    // C10: Pop + Drop only (no Apply in UI)
    elevated_card(cx)
        .child(section_header(cx, "Stash", &format!("{} entr{}", entries.len(),
            if entries.len() == 1 { "y" } else { "ies" })))
        .children(entries.iter().map(|entry| {
            let index = entry.index;
            let desc = format!("stash@{{{}}}: {} — {}", entry.index, entry.branch, entry.message);
            div()
                .flex().items_center().justify_between()
                .px(px(12.)).py(px(4.))
                .text_sm().text_color(theme::fg(cx))
                .child(div().child(desc))
                .child(
                    div().flex().gap(px(4.))
                        .child(stash_action_btn("Pop", busy, cx, move |this, _ev, window, cx| {
                            this.stash_pop(index, window, cx);
                        }))
                        .child(stash_action_btn("Drop", busy, cx, move |this, _ev, window, cx| {
                            this.stash_drop(index, window, cx);
                        })),
                )
        }))
        .child(
            div().mt(px(8.)).flex().items_center().gap(px(8.))
                .child(div().flex_1().child(Input::new(&stash_input)))
                .child(stash_action_btn("Stash push", busy, cx, move |this, _ev, window, cx| {
                    this.stash_push_action(window, cx);
                })),
        )
}

fn stash_action_btn(
    label: &str,
    busy: bool,
    cx: &mut Context<GitPage>,
    on_click: impl Fn(&mut GitPage, &MouseDownEvent, &mut Window, &mut Context<GitPage>) + 'static,
) -> impl IntoElement {
    div()
        .cursor_pointer()
        .px(px(8.)).py(px(2.)).rounded(px(4.))
        .text_xs().font_weight(gpui::FontWeight::MEDIUM)
        .when(busy, |this| {
            this.bg(theme::border(cx)).text_color(theme::muted(cx)).opacity(0.6)
        })
        .when(!busy, |this| {
            this.bg(theme::bg_hover(cx)).text_color(theme::fg(cx))
                .hover(|this| this.bg(theme::border(cx)))
        })
        .on_mouse_down(MouseButton::Left, on_click)
        .child(label)
}
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p chronos-fm-pages
```

Expected: `Finished` with 0 errors.

---

### Task 6: Final verification

- [ ] **Step 1: Run full service tests**

```bash
cargo test -p chronos-fm-services
```

Expected: all tests pass.

- [ ] **Step 2: Run page tests**

```bash
cargo test -p chronos-fm-pages
```

Expected: all tests pass.

- [ ] **Step 3: Clippy check**

```bash
cargo clippy -p chronos-fm-services -p chronos-fm-pages --all-targets
```

Expected: 0 warnings (or only pre-existing ones).

- [ ] **Step 4: Full build**

```bash
cargo build -p chronos-fm
```

Expected: `Finished` with 0 errors.
