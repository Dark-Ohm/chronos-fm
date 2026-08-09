use anyhow::Result;
use ignore::WalkBuilder;
use notify::RecursiveMode;
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use std::path::PathBuf;
use std::time::Duration;
// `notify` / `notify-debouncer-mini` are runtime-agnostic (they drive their own
// std::thread). The debounce callback runs on that thread, so it forwards
// batches over an `async-channel` sender with the blocking `send_blocking`
// (async-runtime.md §2/§4).
use async_channel::Sender;

use super::exclusions::Excludes;

/// Watches a directory tree and forwards debounced batches of changed paths.
pub struct FileWatcher {
    // Keep debouncer alive
    _debouncer: Debouncer<notify::RecommendedWatcher>,
}

impl FileWatcher {
    /// Starts watching `root` recursively, sending debounced change batches on `tx`
    /// with the given debounce `timeout`.
    ///
    /// A single unreadable descendant (e.g. a podman volume owned by another
    /// uid) makes `notify`'s recursive add abort entirely. To keep search alive
    /// we first try a plain recursive watch (full coverage, including newly
    /// created subdirs on healthy systems); if that fails we fall back to
    /// watching each *readable* directory individually, skipping the ones we
    /// cannot read and pruning heavy/foreign trees via `excludes`.
    ///
    /// **T033 — feedback-loop fix.** The debouncer callback now filters out
    /// paths that match `excludes` *before* they ever reach the consumer
    /// thread. The motivating case is `~/.chronos-fm/index/**`: tantivy lives
    /// under the watched `$HOME`, so without this filter every tantivy commit
    /// retriggers the watcher → `process_changes` → `writer.commit()` →
    /// write `meta.json` → loop (T014 §2). Filtering at the callback level
    /// also means an empty batch (debounce window contained only excluded
    /// changes) is dropped without waking the consumer. This preserves the
    /// Recursive fast-path coverage (newly created unexcluded dirs still get
    /// watched automatically on healthy systems).
    pub fn new(
        root: PathBuf,
        tx: Sender<Vec<PathBuf>>,
        timeout: Duration,
        excludes: Excludes,
    ) -> Result<Self> {
        // T033: the callback runs on notify's own thread and receives every
        // event in the watched tree. We filter against `excludes` *before* any
        // user of the channel ever sees a path, so the feedback loop
        // (`meta.json` write → notify → process_changes → commit → write)
        // never trips itself. `Excludes: Clone`, so we just take a clone for
        // the closure and leave the original `excludes` for the fallback walker.
        let excludes_for_cb = excludes.clone();
        let mut debouncer = new_debouncer(timeout, move |res: DebounceEventResult| {
            match res {
                Ok(events) => {
                    let paths: Vec<PathBuf> = events
                        .into_iter()
                        .map(|e| e.path)
                        .filter(|p| !excludes_for_cb.matches(p))
                        .collect();
                    // Drop the batch entirely if everything was excluded:
                    // there's no work for the consumer and sending an empty
                    // `Vec` would force a futex wake + `recv_blocking` for
                    // nothing.
                    if paths.is_empty() {
                        return;
                    }
                    // We run on notify's own thread, so a blocking send is correct here.
                    if let Err(e) = tx.send_blocking(paths) {
                        tracing::warn!("Failed to send watcher events: {}", e);
                        // Receiver dropped, we can't do much.
                    }
                }
                Err(e) => {
                    tracing::warn!("Watcher error: {:?}", e);
                }
            }
        })?;

        // Fast path: a single recursive watch. On healthy systems this gives
        // full coverage including directories created later.
        let recursive_ok = debouncer
            .watcher()
            .watch(&root, RecursiveMode::Recursive)
            .is_ok();

        if !recursive_ok {
            // A permission error somewhere in the tree aborted the recursive add.
            // Watch each readable directory individually and swallow per-dir
            // errors so one bad branch can't take search down (T016).
            tracing::debug!(
                "Recursive watch of {:?} failed; falling back to per-directory watch with excludes",
                root
            );
            let walker = WalkBuilder::new(&root)
                .hidden(false)
                .git_ignore(true)
                .filter_entry(move |e| !excludes.matches(e.path()))
                .build();
            for entry in walker.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if let Err(e) = debouncer.watcher().watch(p, RecursiveMode::NonRecursive) {
                        tracing::debug!("Skipping unwatchable directory {:?}: {}", p, e);
                    }
                }
            }
        }

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    // T033 — Path 4 verification: when the user rewrites tantivy's own
    // `meta.json`, the debouncer callback must filter that path so it never
    // reaches the consumer thread (this is what kills the watcher↔self-index
    // feedback loop). At the same time, real user edits in *non*-excluded
    // subdirectories must still produce events.
    #[cfg(unix)]
    #[test]
    fn watcher_callback_filters_self_index_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        // Two parallel subtrees: a normal user file and the simulated
        // tantivy index. Both will be touched; only the user file should
        // surface in an rx batch.
        fs::write(root.join("user.txt"), b"hello").unwrap();
        let index_dir = root.join(".chronos-fm").join("index");
        fs::create_dir_all(&index_dir).unwrap();
        let meta = index_dir.join("meta.json");
        fs::write(&meta, b"{\"version\":1}").unwrap();

        let (tx, rx) = async_channel::bounded(64);
        let result = FileWatcher::new(
            root.clone(),
            tx,
            Duration::from_millis(100),
            Excludes::default(),
        );
        assert!(result.is_ok(), "watcher must start under T033 excludes");

        // Drain any in-flight events from the initial scan / first-batch
        // burst so the test reads only what we deliberately write next.
        std::thread::sleep(Duration::from_millis(200));
        while rx.try_recv().is_ok() {}

        // Touch both files within a single debounce window so they end up in
        // the same batch and we can assert the filter behavior on one batch.
        fs::write(&meta, b"{\"version\":2}").unwrap();
        fs::write(root.join("user.txt"), b"hello updated").unwrap();

        // Wait up to 2 s for the batch, then collect anything else within
        // an extra 300 ms so a second debounce window's worth of events also
        // gets drained — and we can confirm none of them are excluded-path.
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut first_batch = None;
        let mut leaked_excluded: Vec<PathBuf> = Vec::new();
        while std::time::Instant::now() < deadline {
            match rx.try_recv() {
                Ok(paths) => {
                    if first_batch.is_none() {
                        first_batch = Some(paths.clone());
                    }
                    leaked_excluded.extend(
                        paths
                            .iter()
                            .filter(|p| p.components().any(|c| {
                                c.as_os_str().to_string_lossy() == ".chronos-fm"
                            }))
                            .cloned(),
                    );
                }
                Err(async_channel::TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(async_channel::TryRecvError::Closed) => break,
            }
        }

        let batch = first_batch.expect("watcher should have produced at least one batch");
        assert!(
            batch.iter().any(|p| p.ends_with("user.txt")),
            "user edit must reach the consumer; got {:?}",
            batch
        );
        assert!(
            !batch.iter().any(|p| p.ends_with("meta.json")),
            "self-index write must be filtered at the callback; got {:?}",
            batch
        );
        assert!(
            leaked_excluded.is_empty(),
            "no batch may contain a path under .chronos-fm/; leaked {:?}",
            leaked_excluded
        );
    }

    #[cfg(unix)]
    #[test]
    fn watcher_survives_unreadable_subdir() {
        // Skip under root: chmod 000 does not restrict root, so the
        // unreadable-directory scenario cannot be reproduced and the test is
        // meaningless (see T016).
        let probe = std::env::temp_dir().join(format!("cfm_root_probe_{}", std::process::id()));
        let _ = fs::create_dir_all(&probe);
        let _ = fs::set_permissions(&probe, fs::Permissions::from_mode(0o000));
        let can_still_read = fs::read_dir(&probe).is_ok();
        let _ = fs::set_permissions(&probe, fs::Permissions::from_mode(0o755));
        let _ = fs::remove_dir_all(&probe);
        if can_still_read {
            // Running as root; nothing to assert here.
            return;
        }

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fs::write(root.join("a.txt"), b"hello").unwrap();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/b.txt"), b"world").unwrap();

        // Unreadable subdir owned by another (simulated) uid.
        let locked = root.join("locked");
        fs::create_dir_all(&locked).unwrap();
        fs::write(locked.join("secret.txt"), b"secret").unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

        let (tx, rx) = async_channel::bounded(64);
        let result = FileWatcher::new(root.clone(), tx, Duration::from_millis(50), Excludes::default());
        assert!(
            result.is_ok(),
            "watcher must not abort on an unreadable subdir"
        );

        // The readable part of the tree must still be watched: modifying a.txt
        // should produce a change event (poll with a deadline to avoid hanging).
        fs::write(root.join("a.txt"), b"hello updated").unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut got = false;
        while std::time::Instant::now() < deadline {
            if rx.try_recv().is_ok() {
                got = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            got,
            "watcher should still report changes in readable parts of the tree"
        );

        // Restore perms so the temp dir can be cleaned up.
        let _ = fs::set_permissions(&locked, fs::Permissions::from_mode(0o755));
    }
}
