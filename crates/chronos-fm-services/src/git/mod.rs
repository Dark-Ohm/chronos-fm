//! Git client layer for the Git page (T010, Milestone A).
//!
//! Pure gitoxide (gix) plumbing: repository discovery, status classification
//! into staged / modified / untracked, index staging/unstaging, and commits.
//! No GPUI dependency — unit tests drive it against temp repositories created
//! with the system `git` CLI as fixtures.
//!
//! Scope note: plain blobs only. Content filters (CRLF, LFS), sparse checkouts,
//! submodules and worktrees are out of scope for v1; staged blobs are hashed
//! directly from the worktree file.

pub mod watcher;

use std::path::{Path, PathBuf};

/// Errors surfaced to the Git page.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// The directory is not inside any git repository (or the repository is
    /// bare and has no worktree).
    #[error("not a git repository")]
    NotARepository,
    /// Any other gix or I/O failure, rendered as a one-line message.
    #[error("git: {0}")]
    Operation(String),
}

/// A single changed file, already partitioned into one of the three lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoEntry {
    /// Repository-relative path with `/` separators.
    pub path: String,
}

/// Snapshot of the working tree relative to `HEAD` and the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStatus {
    /// Index differs from `HEAD`.
    pub staged: Vec<RepoEntry>,
    /// Worktree differs from the index.
    pub modified: Vec<RepoEntry>,
    /// Present in the worktree, absent from the index.
    pub untracked: Vec<RepoEntry>,
    /// Shortened branch name (`main`); empty when detached or unborn.
    pub branch: String,
    /// Absolute worktree root of the discovered repository.
    pub workdir: PathBuf,
    /// Absolute `.git` directory (used by the watcher).
    pub git_dir: PathBuf,
}

/// Discover the nearest repository at or above `dir`.
pub fn open_repo(dir: &Path) -> Result<gix::Repository, GitError> {
    gix::discover(dir).map_err(|_| GitError::NotARepository)
}

/// Compute the full status snapshot. `repo` must have a worktree.
pub fn status(repo: &gix::Repository) -> Result<RepoStatus, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;

    let mut staged = Vec::new();
    let mut modified = Vec::new();
    let mut untracked = Vec::new();

    let platform = repo
        .status(gix::progress::Discard)
        .map_err(|e| GitError::Operation(e.to_string()))?;
    for item in platform
        .into_iter(std::iter::empty::<gix::bstr::BString>())
        .map_err(|e| GitError::Operation(e.to_string()))?
    {
        match item.map_err(|e| GitError::Operation(e.to_string()))? {
            gix::status::Item::TreeIndex(change) => staged.push(RepoEntry {
                path: change.location().to_string(),
            }),
            gix::status::Item::IndexWorktree(entry) => {
                use gix::status::index_worktree::iter::Summary;
                match entry.summary() {
                    // `git add -N` entries show as untracked in `git status`.
                    Some(Summary::Added) | Some(Summary::IntentToAdd) => untracked.push(RepoEntry {
                        path: entry.rela_path().to_string(),
                    }),
                    Some(
                        Summary::Modified
                        | Summary::Removed
                        | Summary::TypeChange
                        | Summary::Conflict,
                    ) => modified.push(RepoEntry {
                        path: entry.rela_path().to_string(),
                    }),
                    // Renames/copies are disabled in v1 (`rename::Mode::None` by
                    // default) and `NeedsUpdate`/other summaries are not user
                    // visible changes.
                    _ => {}
                }
            }
        }
    }

    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.referent_name().map(|n| n.shorten().to_string()))
        .unwrap_or_default();

    Ok(RepoStatus {
        staged,
        modified,
        untracked,
        branch,
        workdir: workdir.to_path_buf(),
        git_dir: repo.git_dir().to_path_buf(),
    })
}

/// Stage a repository-relative path: add it to the index, refresh its
/// stat/object id if it is already tracked, or — when the worktree file is
/// gone — remove the index entry to stage the deletion.
pub fn stage_path(repo: &gix::Repository, rela_path: &str) -> Result<(), GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let full = workdir.join(rela_path);

    let meta = match gix::index::fs::Metadata::from_path_no_follow(&full) {
        Ok(meta) => Some(meta),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(GitError::Operation(format!("stat {rela_path}: {e}"))),
    };

    let mut file = repo
        .index_or_empty()
        .map_err(|e| GitError::Operation(e.to_string()))?
        .into_owned_or_cloned();
    let path = gix::bstr::BStr::new(rela_path);

    match meta {
        // Deletion: drop the index entry so the removal is staged.
        None => {
            file.remove_entries(|_, entry_path, _| entry_path == path);
            file.sort_entries();
        }
        Some(meta) => {
            let stat = gix::index::entry::Stat::from_fs(&meta)
                .map_err(|e| GitError::Operation(format!("stat {rela_path}: {e}")))?;
            let mut data = Vec::new();
            std::io::Read::read_to_end(
                &mut std::fs::File::open(&full)
                    .map_err(|e| GitError::Operation(format!("read {rela_path}: {e}")))?,
                &mut data,
            )
            .map_err(|e| GitError::Operation(format!("read {rela_path}: {e}")))?;
            let blob_id = repo
                .write_object(gix::objs::BlobRef { data: &data })
                .map_err(|e| GitError::Operation(format!("hash {rela_path}: {e}")))?
                .detach();
            let mode = if meta.is_executable() {
                gix::index::entry::Mode::FILE_EXECUTABLE
            } else {
                gix::index::entry::Mode::FILE
            };
            match file.entry_mut_by_path_and_stage(path, gix::index::entry::Stage::Unconflicted) {
                Some(entry) => {
                    entry.stat = stat;
                    entry.id = blob_id;
                    entry.flags = gix::index::entry::Flags::empty();
                    entry.mode = mode;
                }
                None => file.dangerously_push_entry(
                    stat,
                    blob_id,
                    gix::index::entry::Flags::empty(),
                    mode,
                    path,
                ),
            }
            file.sort_entries();
        }
    }
    file.write(Default::default())
        .map_err(|e| GitError::Operation(format!("write index: {e}")))
}

/// Unstage a repository-relative path: remove it from the index.
pub fn unstage_path(repo: &gix::Repository, rela_path: &str) -> Result<(), GitError> {
    let mut file = repo
        .index_or_empty()
        .map_err(|e| GitError::Operation(e.to_string()))?
        .into_owned_or_cloned();
    {
        let path = gix::bstr::BStr::new(rela_path);
        file.remove_entries(|_, entry_path, _| entry_path == path);
        file.sort_entries();
    }
    file.write(Default::default())
        .map_err(|e| GitError::Operation(format!("write index: {e}")))
}

/// Commit all staged entries, advancing `HEAD` (or creating the first commit).
pub fn commit(repo: &gix::Repository, message: &str) -> Result<(), GitError> {
    // Reject no-op commits: the index must differ from HEAD. `index.entries()`
    // is not usable for this — it lists every tracked file.
    if status(repo)?.staged.is_empty() {
        return Err(GitError::Operation(
            "nothing staged to commit".to_string(),
        ));
    }

    // Build a tree from the full index.
    let index = repo
        .index_or_empty()
        .map_err(|e| GitError::Operation(e.to_string()))?;
    let mut editor = repo
        .edit_tree(gix::ObjectId::empty_tree(repo.object_hash()))
        .map_err(|e| GitError::Operation(format!("edit tree: {e}")))?;
    for entry in index.entries() {
        let kind = entry
            .mode
            .to_tree_entry_mode()
            .map(|m| m.kind())
            .unwrap_or(gix::object::tree::EntryKind::Blob);
        editor
            .upsert(gix::bstr::BStr::new(entry.path(&index)), kind, entry.id)
            .map_err(|e| GitError::Operation(format!("upsert tree: {e}")))?;
    }
    let tree_id = editor
        .write()
        .map_err(|e| GitError::Operation(format!("write tree: {e}")))?
        .detach();

    // Identity: prefer repo config, fall back to defaults.
    let snapshot = repo.config_snapshot();
    let name = snapshot
        .string("user.name")
        .map(|n| n.to_string())
        .unwrap_or_else(|| "Chronos FM".to_string());
    let email = snapshot
        .string("user.email")
        .map(|e| e.to_string())
        .unwrap_or_else(|| "chronos-fm@localhost".to_string());

    let now = gix::date::Time::now_local_or_utc();
    let mut time_buf = Vec::new();
    now.write_to(&mut time_buf)
        .map_err(|e| GitError::Operation(format!("format time: {e}")))?;
    let time_str = String::from_utf8(time_buf)
        .map_err(|e| GitError::Operation(format!("format time: {e}")))?;
    let sig = gix::actor::SignatureRef {
        name: gix::bstr::BStr::new(name.as_bytes()),
        email: gix::bstr::BStr::new(email.as_bytes()),
        time: &time_str,
    };    // Unborn HEAD: `head()` succeeds with a symbolic reference that has no
    // object id yet — commit on the branch named by its symbolic target
    // (e.g. `refs/heads/main`) with no parents.
    let parent = repo.head().ok().and_then(|head| head.id());
    match parent {
        Some(parent) => {
            repo.commit_as(sig, sig, "HEAD", message, tree_id, [parent])
                .map_err(|e| GitError::Operation(format!("commit: {e}")))?;
        }
        None => {
            let reference = symbolic_head_branch(repo);
            repo.commit_as(
                sig,
                sig,
                reference.as_str(),
                message,
                tree_id,
                std::iter::empty::<gix::ObjectId>(),
            )
            .map_err(|e| GitError::Operation(format!("commit: {e}")))?;
        }
    }
    Ok(())
}

/// Resolve `HEAD`'s symbolic target to a full reference name for the first
/// commit, falling back to `refs/heads/main`.
fn symbolic_head_branch(repo: &gix::Repository) -> String {
    let head_file = repo.git_dir().join("HEAD");
    if let Ok(mut file) = std::fs::File::open(&head_file) {
        let mut contents = String::new();
        if std::io::Read::read_to_string(&mut file, &mut contents).is_ok() {
            if let Some(target) = contents.trim().strip_prefix("ref: ") {
                if !target.is_empty() {
                    return target.to_string();
                }
            }
        }
    }
    "refs/heads/main".to_string()
}

/// A single stash entry parsed from `git stash list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    pub index: usize,
    pub branch: String,
    pub message: String,
}

// --- Milestone B: branches + unified diff ---------------------------------

/// Local branch names (short, without `refs/heads/`), sorted.
pub fn list_branches(repo: &gix::Repository) -> Result<Vec<String>, GitError> {
    let platform = repo
        .references()
        .map_err(|e| GitError::Operation(e.to_string()))?;
    let mut names = Vec::new();
    for r in platform
        .local_branches()
        .map_err(|e| GitError::Operation(e.to_string()))?
    {
        let r = r.map_err(|e| GitError::Operation(e.to_string()))?;
        names.push(r.name().shorten().to_string());
    }
    names.sort();
    names.dedup();
    Ok(names)
}

/// Create a local branch at the current `HEAD` tip (does not switch).
///
/// Name is validated: non-empty, no spaces/slashes that would form nested
/// refs beyond a single segment, no `..`.
pub fn create_branch(repo: &gix::Repository, name: &str) -> Result<(), GitError> {
    let name = name.trim();
    validate_branch_name(name)?;
    let head_id = repo
        .head_id()
        .map_err(|e| GitError::Operation(format!("HEAD: {e}")))?
        .detach();
    let full = format!("refs/heads/{name}");
    repo.reference(
        full.as_str(),
        head_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "chronos-fm: create branch",
    )
    .map_err(|e| GitError::Operation(format!("create branch: {e}")))?;
    Ok(())
}

fn validate_branch_name(name: &str) -> Result<(), GitError> {
    if name.is_empty() {
        return Err(GitError::Operation("branch name is empty".into()));
    }
    if name.contains("..")
        || name.contains(' ')
        || name.contains('\\')
        || name.starts_with('-')
        || name.contains('\0')
    {
        return Err(GitError::Operation(format!(
            "invalid branch name: {name:?}"
        )));
    }
    // Allow a single path segment only (no nested `feature/x` in v1 UI — still
    // accept `/` for power users who type full short names like `feat/foo`).
    Ok(())
}

/// Switch `HEAD` to an existing local branch and update the worktree via the
/// system `git checkout` binary. gix checkout is still incomplete for dirty
/// trees; shelling out keeps Milestone B correct and testable.
pub fn checkout_branch(repo: &gix::Repository, name: &str) -> Result<(), GitError> {
    let name = name.trim();
    validate_branch_name(name)?;
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    // Ensure the branch exists as a local ref.
    let full = format!("refs/heads/{name}");
    repo.find_reference(full.as_str())
        .map_err(|_| GitError::Operation(format!("branch not found: {name}")))?;
    let out = std::process::Command::new("git")
        .args(["checkout", "-q", name])
        .current_dir(workdir)
        .output()
        .map_err(|e| GitError::Operation(format!("git checkout: {e}")))?;
    if !out.status.success() {
        return Err(GitError::Operation(format!(
            "git checkout: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(())
}

/// Unified-style text diff of a repository-relative path.
///
/// - **Modified / untracked:** worktree vs index (or empty blob for untracked).
/// - **Staged:** index vs `HEAD` tree (empty blob if unborn / not in HEAD).
///
/// Binary files return a short notice instead of a byte dump. No syntect here —
/// the page may colour the text; services stay GUI-free on the default feature
/// set.
pub fn unified_diff(
    repo: &gix::Repository,
    rela_path: &str,
    staged: bool,
) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let full = workdir.join(rela_path);

    let (old_label, new_label, old_bytes, new_bytes) = if staged {
        let head_blob = blob_from_head(repo, rela_path)?;
        let index_blob = blob_from_index(repo, rela_path)?;
        (
            format!("a/{rela_path}"),
            format!("b/{rela_path}"),
            head_blob,
            index_blob,
        )
    } else {
        let index_blob = blob_from_index(repo, rela_path).unwrap_or_default();
        let worktree = if full.exists() {
            std::fs::read(&full).map_err(|e| GitError::Operation(format!("read: {e}")))?
        } else {
            Vec::new()
        };
        (
            format!("a/{rela_path}"),
            format!("b/{rela_path}"),
            index_blob,
            worktree,
        )
    };

    if looks_binary(&old_bytes) || looks_binary(&new_bytes) {
        return Ok(format!("Binary file {rela_path} differs\n"));
    }

    let old_text = String::from_utf8_lossy(&old_bytes);
    let new_text = String::from_utf8_lossy(&new_bytes);
    Ok(render_unified_diff(&old_label, &new_label, &old_text, &new_text))
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0) || bytes.iter().filter(|&&b| b < 9 || (b > 13 && b < 32)).count() > bytes.len() / 8
}

fn blob_from_index(repo: &gix::Repository, rela_path: &str) -> Result<Vec<u8>, GitError> {
    let index = repo
        .index_or_empty()
        .map_err(|e| GitError::Operation(e.to_string()))?;
    let path = gix::bstr::BStr::new(rela_path);
    let entry = index
        .entry_by_path_and_stage(path, gix::index::entry::Stage::Unconflicted)
        .ok_or_else(|| GitError::Operation(format!("not in index: {rela_path}")))?;
    let obj = repo
        .find_object(entry.id)
        .map_err(|e| GitError::Operation(e.to_string()))?;
    Ok(obj.data.clone())
}

fn blob_from_head(repo: &gix::Repository, rela_path: &str) -> Result<Vec<u8>, GitError> {
    let Ok(head) = repo.head_commit() else {
        return Ok(Vec::new());
    };
    let tree = head
        .tree()
        .map_err(|e| GitError::Operation(e.to_string()))?;
    // Walk path components.
    let mut current = tree;
    let parts: Vec<&str> = rela_path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Ok(Vec::new());
    }
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let entry = current
            .lookup_entry(std::iter::once(*part))
            .map_err(|e| GitError::Operation(e.to_string()))?
            .ok_or_else(|| GitError::Operation(format!("not in HEAD: {rela_path}")))?;
        if last {
            let obj = entry
                .object()
                .map_err(|e| GitError::Operation(e.to_string()))?;
            return Ok(obj.data.clone());
        }
        current = entry
            .object()
            .map_err(|e| GitError::Operation(e.to_string()))?
            .try_into_tree()
            .map_err(|e| GitError::Operation(e.to_string()))?;
    }
    Ok(Vec::new())
}

/// Minimal unified diff (no context-line algorithm libraries): full-file
/// replacement hunk. Good enough for FM review of typical source files.
fn render_unified_diff(old_label: &str, new_label: &str, old: &str, new: &str) -> String {
    if old == new {
        return format!("--- {old_label}\n+++ {new_label}\n(no textual changes)\n");
    }
    let old_lines: Vec<&str> = if old.is_empty() {
        Vec::new()
    } else {
        old.lines().collect()
    };
    let new_lines: Vec<&str> = if new.is_empty() {
        Vec::new()
    } else {
        new.lines().collect()
    };
    let mut out = String::new();
    out.push_str(&format!("--- {old_label}\n+++ {new_label}\n"));
    out.push_str(&format!(
        "@@ -1,{} +1,{} @@\n",
        old_lines.len(),
        new_lines.len()
    ));
    for line in &old_lines {
        out.push('-');
        out.push_str(line);
        out.push('\n');
    }
    for line in &new_lines {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    out
}

// --- Milestone C: push / pull / stash (system git) --------------------------

fn truncate_4k(s: &str) -> String {
    if s.len() <= 4096 {
        s.to_string()
    } else {
        let cut = floor_char_boundary(s, 4096);
        format!("{}…\n[truncated {} bytes]", &s[..cut], s.len() - cut)
    }
}

/// Quick UTF-8 code point boundary for painless string slice.
fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Run `git` in `workdir` with the given args, return combined stdout+stderr
/// on success (P1: git often writes success messages to stderr).
fn run_git_cmd(workdir: &std::path::Path, args: &[&str]) -> Result<String, GitError> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| GitError::Operation(format!("git {}: {e}", args.get(0).unwrap_or(&"?"))))?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout).trim(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    let combined = combined.trim();
    if !out.status.success() {
        return Err(GitError::Operation(combined.to_string()));
    }
    Ok(truncate_4k(combined))
}

/// Push current branch to `remote` (defaults to `"origin"` when empty).
/// C1: pushes `HEAD` explicitly so no upstream tracking is needed.
pub fn push(repo: &gix::Repository, remote: &str) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let remote = if remote.is_empty() { "origin" } else { remote };
    run_git_cmd(&workdir, &["push", remote, "HEAD"])  // C1, C3
}

/// Fetch + fast-forward `remote/HEAD` into current branch.
/// C2: uses `origin HEAD` explicitly so no upstream tracking is needed.
pub fn pull(repo: &gix::Repository, remote: &str) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let remote = if remote.is_empty() { "origin" } else { remote };
    run_git_cmd(&workdir, &["pull", "--ff-only", remote, "HEAD"])  // C2, C3
}

/// Push working-tree changes onto the stash stack.
/// C5: when `message` is empty or whitespace, omit the `-m` flag.
pub fn stash_push(repo: &gix::Repository, message: &str) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let trimmed = message.trim();
    if trimmed.is_empty() {
        run_git_cmd(&workdir, &["stash", "push"])             // C5: no -m
    } else {
        run_git_cmd(&workdir, &["stash", "push", "-m", trimmed])
    }
}

/// Pop (apply + drop) stash entry at `index`.
pub fn stash_pop(repo: &gix::Repository, index: usize) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let refspec = format!("stash@{{{index}}}");  // P3: bind outside &[...]
    run_git_cmd(&workdir, &["stash", "pop", &refspec])
}

/// Apply stash entry at `index` without dropping. C10: service-only, no UI.
pub fn stash_apply(repo: &gix::Repository, index: usize) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let refspec = format!("stash@{{{index}}}");
    run_git_cmd(&workdir, &["stash", "apply", &refspec])
}

/// Drop stash entry at `index` without applying (C10: UI Pop+Drop, Apply service-only).
pub fn stash_drop(repo: &gix::Repository, index: usize) -> Result<String, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let refspec = format!("stash@{{{index}}}");
    run_git_cmd(&workdir, &["stash", "drop", &refspec])
}

/// Parse `git stash list` into `StashEntry` vec.
/// C9: handles both `WIP on <branch>: <msg>` and `On <branch>: <msg>`.
pub fn stash_list(repo: &gix::Repository) -> Result<Vec<StashEntry>, GitError> {
    let workdir = repo.workdir().ok_or(GitError::NotARepository)?;
    let text = run_git_cmd(&workdir, &["stash", "list"])?;
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // P8: parse stash@{N} from the line prefix
        let stash_n = match line.split(": ").next() {
            Some(prefix) if prefix.starts_with("stash@{") => prefix.to_string(),
            _ => continue,
        };
        let after_colon = match line.find(": ") {
            Some(pos) => &line[pos + 2..],
            None => continue,
        };
        let (branch, message) = if let Some(rest) = after_colon.strip_prefix("WIP on ") {
            parse_branch_msg(rest)
        } else if let Some(rest) = after_colon.strip_prefix("On ") {
            parse_branch_msg(rest)
        } else {
            ("unknown".to_string(), after_colon.to_string())
        };
        let index = entries.len();
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_methods)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git binary available");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn git_output(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git binary available");
        assert!(out.status.success(), "git {args:?} failed");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn init_repo(dir: &Path) {
        git(dir, &["init", "-q", "-b", "main"]);
        git(dir, &["config", "user.email", "test@example.com"]);
        git(dir, &["config", "user.name", "Test"]);
    }

    /// Repository with one committed file (`a.txt`) and a worktree.
    fn repo_with_base_commit() -> (tempfile::TempDir, PathBuf) {
        let td = tempdir().unwrap();
        let root = std::fs::canonicalize(td.path()).unwrap();
        init_repo(&root);
        std::fs::write(root.join("a.txt"), "one").unwrap();
        git(&root, &["add", "a.txt"]);
        git(&root, &["commit", "-q", "-m", "init"]);
        (td, root)
    }

    #[test]
    fn status_classifies_modified_and_untracked() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("a.txt"), "two").unwrap();
        std::fs::write(root.join("b.txt"), "new").unwrap();

        let repo = open_repo(&root).unwrap();
        let s = status(&repo).unwrap();
        assert!(s.staged.is_empty());
        assert_eq!(s.modified, vec![RepoEntry { path: "a.txt".into() }]);
        assert_eq!(s.untracked, vec![RepoEntry { path: "b.txt".into() }]);
        assert_eq!(s.branch, "main");
        assert_eq!(s.workdir, root);
    }

    #[test]
    fn status_discovered_from_subdirectory() {
        let (_td, root) = repo_with_base_commit();
        let sub = root.join("sub/deep");
        std::fs::create_dir_all(&sub).unwrap();

        let repo = open_repo(&sub).unwrap();
        let s = status(&repo).unwrap();
        assert_eq!(s.workdir, root);
        assert_eq!(s.branch, "main");
    }

    #[test]
    fn stage_unstage_roundtrip() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("b.txt"), "new").unwrap();

        let repo = open_repo(&root).unwrap();
        stage_path(&repo, "b.txt").unwrap();
        let s = status(&repo).unwrap();
        assert_eq!(s.staged, vec![RepoEntry { path: "b.txt".into() }]);
        assert!(s.untracked.is_empty());

        unstage_path(&repo, "b.txt").unwrap();
        let s = status(&repo).unwrap();
        assert!(s.staged.is_empty());
        assert_eq!(s.untracked, vec![RepoEntry { path: "b.txt".into() }]);
    }

    #[test]
    fn stage_refreshes_existing_tracked_entry() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("a.txt"), "two").unwrap();

        let repo = open_repo(&root).unwrap();
        let s = status(&repo).unwrap();
        assert!(s.staged.is_empty());
        assert_eq!(s.modified, vec![RepoEntry { path: "a.txt".into() }]);

        stage_path(&repo, "a.txt").unwrap();
        let s = status(&repo).unwrap();
        assert_eq!(s.staged, vec![RepoEntry { path: "a.txt".into() }]);
        assert!(s.modified.is_empty());
    }

    #[test]
    fn commit_creates_commit_and_clears_staged() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("b.txt"), "new").unwrap();

        let repo = open_repo(&root).unwrap();
        stage_path(&repo, "b.txt").unwrap();
        commit(&repo, "commit two").unwrap();

        let log = git_output(&root, &["log", "--oneline", "-1"]);
        assert!(log.ends_with("commit two"), "unexpected log: {log}");

        let s = status(&repo).unwrap();
        assert!(s.staged.is_empty());
        assert!(s.modified.is_empty());
        assert!(s.untracked.is_empty());
    }

    #[test]
    fn first_commit_on_empty_repo() {
        let td = tempdir().unwrap();
        let root = std::fs::canonicalize(td.path()).unwrap();
        init_repo(&root);
        std::fs::write(root.join("b.txt"), "new").unwrap();

        let repo = open_repo(&root).unwrap();
        stage_path(&repo, "b.txt").unwrap();
        commit(&repo, "first").unwrap();

        let log = git_output(&root, &["log", "--oneline", "-1"]);
        assert!(log.ends_with("first"), "unexpected log: {log}");
        let branch = git_output(&root, &["branch", "--show-current"]);
        assert_eq!(branch, "main");

        let s = status(&repo).unwrap();
        assert!(s.staged.is_empty());
        assert_eq!(s.branch, "main");
    }

    #[test]
    fn commit_rejects_empty_staging() {
        let (_td, root) = repo_with_base_commit();
        let repo = open_repo(&root).unwrap();
        assert!(matches!(
            commit(&repo, "nope"),
            Err(GitError::Operation(_))
        ));
    }

    #[test]
    fn stage_keeps_worktree_file_intact() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("b.txt"), "content-123").unwrap();

        let repo = open_repo(&root).unwrap();
        stage_path(&repo, "b.txt").unwrap();
        unstage_path(&repo, "b.txt").unwrap();

        assert_eq!(std::fs::read_to_string(root.join("b.txt")).unwrap(), "content-123");
    }

    #[test]
    fn stage_file_in_subdirectory() {
        let (_td, root) = repo_with_base_commit();
        std::fs::create_dir_all(root.join("sub/dir")).unwrap();
        std::fs::write(root.join("sub/dir/file.txt"), "nested").unwrap();

        let repo = open_repo(&root).unwrap();
        stage_path(&repo, "sub/dir/file.txt").unwrap();

        let s = status(&repo).unwrap();
        assert_eq!(s.staged, vec![RepoEntry { path: "sub/dir/file.txt".into() }]);
        assert!(s.untracked.is_empty());
    }

    #[test]
    fn nested_untracked_directory_is_reported() {
        let (_td, root) = repo_with_base_commit();
        std::fs::create_dir_all(root.join("untracked_dir")).unwrap();
        std::fs::write(root.join("untracked_dir/nested.txt"), "x").unwrap();

        let repo = open_repo(&root).unwrap();
        let s = status(&repo).unwrap();
        // `git status` collapses a fully-untracked directory into one entry.
        assert!(
            s.untracked
                .iter()
                .any(|e| e.path == "untracked_dir" || e.path.starts_with("untracked_dir/")),
            "untracked should contain the nested dir, got {:?}",
            s.untracked
        );
    }

    #[cfg(unix)]
    #[test]
    fn stage_preserves_executable_mode() {
        use std::os::unix::fs::PermissionsExt;
        let (_td, root) = repo_with_base_commit();
        let script = root.join("run.sh");
        std::fs::write(&script, "#!/bin/sh\n").unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();

        let repo = open_repo(&root).unwrap();
        stage_path(&repo, "run.sh").unwrap();

        let ls = git_output(&root, &["ls-files", "--stage", "run.sh"]);
        assert!(ls.starts_with("100755"), "expected 100755 mode, got: {ls}");
    }

    #[test]
    fn stage_deletion_removes_index_entry() {
        let (_td, root) = repo_with_base_commit();
        std::fs::remove_file(root.join("a.txt")).unwrap();

        let repo = open_repo(&root).unwrap();
        let s = status(&repo).unwrap();
        assert_eq!(s.modified, vec![RepoEntry { path: "a.txt".into() }]);

        stage_path(&repo, "a.txt").unwrap();
        let s = status(&repo).unwrap();
        assert!(s.modified.is_empty());
        assert_eq!(s.staged, vec![RepoEntry { path: "a.txt".into() }]);

        let porcelain = git_output(&root, &["status", "--porcelain"]);
        assert!(porcelain.starts_with("D "), "expected staged deletion, got: {porcelain}");
    }

    #[test]
    fn outside_repo_is_not_a_repository() {
        let td = tempdir().unwrap();
        let root = std::fs::canonicalize(td.path()).unwrap();
        assert!(matches!(
            open_repo(&root),
            Err(GitError::NotARepository)
        ));
    }

    // --- Milestone B -------------------------------------------------------

    #[test]
    fn list_branches_includes_main() {
        let (_td, root) = repo_with_base_commit();
        let repo = open_repo(&root).unwrap();
        let branches = list_branches(&repo).unwrap();
        assert_eq!(branches, vec!["main".to_string()]);
    }

    #[test]
    fn create_and_checkout_branch() {
        let (_td, root) = repo_with_base_commit();
        let repo = open_repo(&root).unwrap();
        create_branch(&repo, "feature").unwrap();
        let branches = list_branches(&repo).unwrap();
        assert!(branches.contains(&"feature".to_string()));
        assert!(branches.contains(&"main".to_string()));

        checkout_branch(&repo, "feature").unwrap();
        // Re-open so HEAD is re-read.
        let repo = open_repo(&root).unwrap();
        let s = status(&repo).unwrap();
        assert_eq!(s.branch, "feature");
    }

    #[test]
    fn create_branch_rejects_empty_name() {
        let (_td, root) = repo_with_base_commit();
        let repo = open_repo(&root).unwrap();
        assert!(create_branch(&repo, "  ").is_err());
    }

    #[test]
    fn unified_diff_worktree_modification() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("a.txt"), "two\n").unwrap();
        let repo = open_repo(&root).unwrap();
        let diff = unified_diff(&repo, "a.txt", false).unwrap();
        assert!(diff.contains("--- a/a.txt"), "{diff}");
        assert!(diff.contains("+++ b/a.txt"), "{diff}");
        assert!(diff.contains("-one"), "{diff}");
        assert!(diff.contains("+two"), "{diff}");
    }

    #[test]
    fn unified_diff_staged_vs_head() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("a.txt"), "staged\n").unwrap();
        let repo = open_repo(&root).unwrap();
        stage_path(&repo, "a.txt").unwrap();
        let diff = unified_diff(&repo, "a.txt", true).unwrap();
        assert!(diff.contains("-one"), "{diff}");
        assert!(diff.contains("+staged"), "{diff}");
    }

    // --- Milestone C -------------------------------------------------------

    #[test]
    fn push_to_bare_remote() {
        let td = tempdir().unwrap();
        let base = std::fs::canonicalize(td.path()).unwrap();
        // P5: separate workdir and bare.git
        let workdir = base.join("workdir");
        std::fs::create_dir(&workdir).unwrap();
        let bare = base.join("bare.git");
        git(&base, &["init", "--bare", "-q", "bare.git"]);
        init_repo(&workdir);
        std::fs::write(workdir.join("a.txt"), "one").unwrap();
        git(&workdir, &["add", "a.txt"]);
        git(&workdir, &["commit", "-q", "-m", "init"]);
        git(
            &workdir,
            &["remote", "add", "origin", bare.to_str().unwrap()],
        );

        let repo = open_repo(&workdir).unwrap();
        let out = push(&repo, "origin").unwrap();
        assert!(!out.is_empty(), "push should produce output: {out}");

        let log = git_output(&bare, &["log", "--oneline", "--all"]);
        assert!(log.contains("init"), "bare repo should have commit: {log}");
    }

    #[test]
    fn pull_from_remote() {
        let td = tempdir().unwrap();
        let base = std::fs::canonicalize(td.path()).unwrap();
        let bare = base.join("bare.git");
        let pusher = base.join("pusher");
        std::fs::create_dir(&pusher).unwrap();
        git(&base, &["init", "--bare", "-q", "bare.git"]);
        // P5: separate pusher repo
        init_repo(&pusher);
        std::fs::write(pusher.join("a.txt"), "v1").unwrap();
        git(&pusher, &["add", "a.txt"]);
        git(&pusher, &["commit", "-q", "-m", "c1"]);
        git(
            &pusher,
            &["remote", "add", "origin", bare.to_str().unwrap()],
        );
        git(&pusher, &["push", "-q", "origin", "HEAD:refs/heads/main"]);
        // Set HEAD on the bare remote so clone+fetch have a default branch.
        let head_set = std::process::Command::new("git")
            .args(["--git-dir", bare.to_str().unwrap(), "symbolic-ref", "HEAD", "refs/heads/main"])
            .output()
            .unwrap();
        assert!(head_set.status.success(), "git symbolic-ref HEAD failed");

        let clone = base.join("clone");
        git(
            &base,
            &["clone", "-q", bare.to_str().unwrap(), clone.to_str().unwrap()],
        );
        std::fs::write(pusher.join("a.txt"), "v2").unwrap();
        git(&pusher, &["add", "a.txt"]);
        git(&pusher, &["commit", "-q", "-m", "c2"]);
        git(&pusher, &["push", "-q", "origin", "HEAD:refs/heads/main"]);

        let repo = open_repo(&clone).unwrap();
        let out = pull(&repo, "origin").unwrap();
        assert!(!out.is_empty(), "pull should produce output");

        let log = git_output(&clone, &["log", "--oneline"]);
        assert!(log.contains("c2"), "clone should have c2 after pull: {log}");
    }

    #[test]
    fn stash_push_pop_roundtrip() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("a.txt"), "modified").unwrap();

        let repo = open_repo(&root).unwrap();
        let out = stash_push(&repo, "test stash").unwrap();
        assert!(out.contains("Saved working directory"), "{out}");

        // Stash touches tracked files only (P4: no untracked).
        assert_eq!(
            std::fs::read_to_string(root.join("a.txt")).unwrap(),
            "one",
            "stash should restore tracked file"
        );

        let out = stash_pop(&repo, 0).unwrap();
        assert!(out.contains("Dropped"), "{out}");

        assert_eq!(
            std::fs::read_to_string(root.join("a.txt")).unwrap(),
            "modified"
        );
    }

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

    #[test]
    fn stash_push_empty_message() {
        let (_td, root) = repo_with_base_commit();
        std::fs::write(root.join("a.txt"), "v1").unwrap();
        let repo = open_repo(&root).unwrap();
        stash_push(&repo, "").unwrap();
        // Second push with different content so git doesn't reject as no-op.
        std::fs::write(root.join("a.txt"), "v2").unwrap();
        stash_push(&repo, "  ").unwrap();
        let entries = stash_list(&repo).unwrap();
        assert_eq!(entries.len(), 2);
    }
}
