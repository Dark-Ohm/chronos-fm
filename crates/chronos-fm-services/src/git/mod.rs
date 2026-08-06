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
}
