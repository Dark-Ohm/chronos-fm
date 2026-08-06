//! PROBE — temporary. Verifies gix 0.86 API surface for the T010 Milestone A plan.
//!
//! This module is a compile+run reference implementation. It will be replaced by
//! the real service layer per the T010 implementation plan.

use std::path::Path;

/// Open the nearest repository at or above `dir`.
pub fn open_repo(dir: &Path) -> anyhow::Result<gix::Repository> {
    Ok(gix::discover(dir)?)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_methods, clippy::pedantic)]
mod tests {
    use super::*;
    use gix::status::index_worktree::iter::Summary;
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

    /// Classify status items into (staged_paths, modified_paths, untracked_paths).
    fn status_lists(
        repo: &gix::Repository,
    ) -> anyhow::Result<(Vec<String>, Vec<String>, Vec<String>)> {
        use gix::status::Item;
        let mut staged = Vec::new();
        let mut modified = Vec::new();
        let mut untracked = Vec::new();
        let platform = repo.status(gix::progress::Discard)?;
        for item in platform.into_iter(std::iter::empty::<gix::bstr::BString>())? {
            match item? {
                Item::TreeIndex(ti) => staged.push(ti.location().to_string()),
                Item::IndexWorktree(iw) => match iw.summary() {
                    Some(Summary::Added) => untracked.push(iw.rela_path().to_string()),
                    Some(
                        Summary::Modified
                        | Summary::Removed
                        | Summary::TypeChange
                        | Summary::Conflict,
                    ) => modified.push(iw.rela_path().to_string()),
                    _ => {}
                },
            }
        }
        Ok((staged, modified, untracked))
    }

    #[test]
    fn probe_full_flow() {
        let td = tempdir().unwrap();
        let root = std::fs::canonicalize(td.path()).unwrap();
        init_repo(&root);
        std::fs::write(root.join("a.txt"), "one").unwrap();
        git(&root, &["add", "a.txt"]);
        git(&root, &["commit", "-q", "-m", "init"]);

        // discover from a subdir
        let sub = root.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let repo = open_repo(&sub).unwrap();

        // head branch
        let head = repo.head().unwrap();
        let branch = head
            .referent_name()
            .map(|n| n.shorten().to_string())
            .unwrap_or_default();
        println!("BRANCH={branch}");

        // modify + untracked
        std::fs::write(root.join("a.txt"), "two").unwrap();
        std::fs::write(root.join("b.txt"), "new").unwrap();

        // status 1: a.txt modified, b.txt untracked, nothing staged
        let (staged, modified, untracked) = status_lists(&repo).unwrap();
        println!("STATUS1 staged={staged:?} modified={modified:?} untracked={untracked:?}");

        // stage b.txt via index plumbing
        let mut file = repo.index().unwrap().into_owned_or_cloned();
        {
            let worktree = repo.workdir().unwrap();
            let full = worktree.join("b.txt");
            let meta = gix::index::fs::Metadata::from_path_no_follow(&full).unwrap();
            let stat = gix::index::entry::Stat::from_fs(&meta).unwrap();
            let data = std::fs::read(&full).unwrap();
            let blob_id = repo
                .write_object(&gix::objs::BlobRef { data: &data })
                .unwrap()
                .detach();
            file.dangerously_push_entry(
                stat,
                blob_id,
                gix::index::entry::Flags::empty(),
                gix::index::entry::Mode::FILE,
                gix::bstr::BStr::new(b"b.txt"),
            );
            file.sort_entries();
        }
        file.write(Default::default()).unwrap();
        println!("STAGE_OK");

        // status 2: b.txt staged
        let (staged, modified, untracked) = status_lists(&repo).unwrap();
        println!("STATUS2 staged={staged:?} modified={modified:?} untracked={untracked:?}");

        // commit via tree editor + commit_as
        {
            let index = repo.index().unwrap();
            let mut editor = repo
                .edit_tree(gix::ObjectId::empty_tree(repo.object_hash()))
                .unwrap();
            for entry in index.entries() {
                let kind = entry
                    .mode
                    .to_tree_entry_mode()
                    .map(|m| m.kind())
                    .unwrap_or(gix::object::tree::EntryKind::Blob);
                editor
                    .upsert(gix::bstr::BStr::new(entry.path(&index)), kind, entry.id)
                    .unwrap();
            }
            let tree_id = editor.write().unwrap().detach();
            let now = gix::date::Time::now_local_or_utc();
            let mut time_buf = Vec::new();
            now.write_to(&mut time_buf).unwrap();
            let time_str = String::from_utf8(time_buf).unwrap();
            let sig = gix::actor::SignatureRef {
                name: gix::bstr::BStr::new(b"Test"),
                email: gix::bstr::BStr::new(b"test@example.com"),
                time: &time_str,
            };
            let parent = repo.head_id().unwrap();
            repo.commit_as(sig, sig, "HEAD", "commit two", tree_id, [parent])
                .unwrap();
        }
        println!("COMMIT_OK");
        let log = git_output(&root, &["log", "--oneline", "-1"]);
        println!("LOG={log}");

        // unstage a.txt (reset): remove entry from index
        let mut file = repo.index().unwrap().into_owned_or_cloned();
        {
            let before = file.entries().len();
            file.remove_entries(|_, path, _| path == gix::bstr::BStr::new(b"a.txt"));
            assert!(file.entries().len() < before, "a.txt should be removed");
            file.sort_entries();
        }
        file.write(Default::default()).unwrap();
        let (staged, _, _) = status_lists(&repo).unwrap();
        println!("UNSTAGE_OK staged={staged:?}");
    }
}
