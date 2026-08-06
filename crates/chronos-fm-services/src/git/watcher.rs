//! Minimal `.git` directory watcher used for live repo status refresh.
//!
//! Mirrors the pattern in `chronos-fm-core::config::watcher` — same thread,
//! same `mpsc` channel + foreground poll approach. We watch the `.git`
//! directory recursively because changes happen inside `HEAD`, `index`, `refs/`
//! and `logs/`. The callback runs on a background thread so callers must
//! marshal back to GPUI.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};

/// Owns the OS watch on a repository's `.git` directory. Dropping stops it.
pub struct GitWatcher {
    _watcher: RecommendedWatcher,
}

impl GitWatcher {
    /// Watch `git_dir` recursively, invoking `on_change` for each create /
    /// modify / remove event. Note: `git_dir` may be a **file** (worktree
    /// `.git` pointer); in that case we watch the parent directory
    /// non-recursively instead (simplified — full worktree support is deferred
    /// to a later milestone).
    pub fn new(git_dir: &Path, on_change: impl Fn() + Send + 'static) -> notify::Result<Self> {
        let target = git_dir.to_path_buf();
        let (watch_dir, mode) = if target.is_dir() {
            (target.clone(), RecursiveMode::Recursive)
        } else {
            // Worktree `.git` file — watch the containing directory.
            (
                target
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from(".")),
                RecursiveMode::NonRecursive,
            )
        };

        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<Event>| match result {
                Ok(event) if is_relevant(&event) => {
                    if event.paths.iter().any(|p| p == &target || p.starts_with(&target)) {
                        on_change();
                    }
                }
                Err(error) => tracing::warn!("git watcher error: {error}"),
                _ => {}
            })?;

        watcher.watch(&watch_dir, mode)?;

        Ok(Self { _watcher: watcher })
    }
}

fn is_relevant(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_methods)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::mpsc;
    use std::time::Duration;
    use tempfile::tempdir;

    fn git(dir: &Path, args: &[&str]) {
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn pinged_on_index_change() {
        let td = tempdir().unwrap();
        let root = std::fs::canonicalize(td.path()).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        std::fs::write(root.join("a.txt"), "one").unwrap();
        git(&root, &["add", "a.txt"]);
        git(&root, &["commit", "-q", "-m", "init"]);

        let (sender, receiver) = mpsc::channel();
        let _watcher = GitWatcher::new(&root.join(".git"), move || {
            sender.send(()).ok();
        })
        .unwrap();

        // Let the OS watcher settle.
        std::thread::sleep(Duration::from_millis(300));

        // Touching a.txt and `git add` updates the index.
        std::fs::write(root.join("a.txt"), "two").unwrap();
        git(&root, &["add", "a.txt"]);

        assert!(
            receiver
                .recv_timeout(Duration::from_secs(10))
                .is_ok(),
            "watcher did not fire"
        );
    }
}
