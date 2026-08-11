//! Window-level undo/redo stack for filesystem operations (T054).
//!
//! `docs/explorer-essentials.md` §1.3: undo/redo is **window-scoped** (one
//! stack shared across panes and tabs), session-only, and the spec names this
//! module (`explorer::undo`) as its home. Every undoable mutation is described
//! by an [`UndoEntry`] whose `reverse()` undoes it and `forward()` re-applies
//! it for redo. The stack itself is pure — no GPUI context — mirroring T053's
//! `conflict.rs::ConflictQueue`; `ExplorerPage` owns the stack and drives
//! execution, so a failed undo is dropped from the history instead of becoming
//! a redo.

use std::path::{Path, PathBuf};

use chronos_fm_services::fs::ops::{self, TransferMode, TransferReport, TrashItem};

/// How many entries the stack keeps before silently dropping the oldest. Set
/// by the ticket (T054); `docs/explorer-essentials.md` §1.3 does not specify
/// one, so this is not cited as a spec requirement.
pub(crate) const UNDO_CAPACITY: usize = 50;

/// One undoable filesystem mutation (T054). `reverse()` undoes it; `forward()`
/// re-applies it for redo.
///
/// Deliberately **not** undoable: permanent delete (no real UI call site
/// today), file content edits, and T053 `Overwrite` conflict resolutions
/// (they destroy the destination's prior contents — architect decision #1).
#[derive(Debug, Clone)]
pub(crate) enum UndoEntry {
    /// A single rename: `old` path → `new` path.
    Rename { old: PathBuf, new: PathBuf },
    /// One user gesture renaming many files, `(old, new)` pairs in application
    /// order. Undoes as a **single** `Ctrl+Z` (architect decision #2).
    BatchRename { renames: Vec<(PathBuf, PathBuf)> },
    /// Sources copied to destinations, `(source, destination)` pairs — one
    /// compound entry per paste/drop gesture.
    Copy { items: Vec<(PathBuf, PathBuf)> },
    /// Sources moved to destinations, `(source, destination)` pairs — one
    /// compound entry per paste/drop gesture.
    Move { items: Vec<(PathBuf, PathBuf)> },
    /// A directory created by New Folder.
    CreateFolder { path: PathBuf },
    /// Entries moved to the OS trash, restorable through their `TrashItem`s.
    Trash { items: Vec<TrashItem> },
}

impl UndoEntry {
    /// Short label for the footer status text ("Undid copy" / "Redid rename").
    pub(crate) fn describe(&self) -> &'static str {
        match self {
            Self::Rename { .. } => "rename",
            Self::BatchRename { .. } => "batch rename",
            Self::Copy { .. } => "copy",
            Self::Move { .. } => "move",
            Self::CreateFolder { .. } => "new folder",
            Self::Trash { .. } => "move to trash",
        }
    }

    /// Undoes the mutation (`Ctrl+Z`). `Err` carries a user-facing message.
    pub(crate) fn reverse(&self) -> Result<(), String> {
        match self {
            Self::Rename { old, new } => {
                ops::rename_in_place(new, &plain_name(old)?).map(|_| ()).map_err(|e| e.to_string())
            }
            Self::BatchRename { renames } => {
                // Renames can form a permutation (a↔b); reversing in reverse
                // application order avoids intermediate name collisions.
                for (old, new) in renames.iter().rev() {
                    ops::rename_in_place(new, &plain_name(old)?).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Self::Copy { items } => {
                for (_, destination) in items {
                    ops::delete_permanent(destination).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Self::Move { items } => {
                // Reverse order keeps nested sources consistent.
                for (source, destination) in items.iter().rev() {
                    ops::move_path(destination, source).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Self::CreateFolder { path } => {
                // Empty-only delete: undoing a folder the user has since
                // populated must refuse rather than destroy their files
                // (`delete_permanent` would recurse).
                ops::delete_empty_dir(path).map_err(|e| e.to_string())
            }
            Self::Trash { items } => {
                ops::restore_trash_items(items.clone()).map_err(|e| e.to_string())
            }
        }
    }

    /// Re-applies the mutation (`Ctrl+Shift+Z` redo).
    pub(crate) fn forward(&self) -> Result<(), String> {
        match self {
            Self::Rename { old, new } => {
                ops::rename_in_place(old, &plain_name(new)?).map(|_| ()).map_err(|e| e.to_string())
            }
            Self::BatchRename { renames } => {
                for (old, new) in renames {
                    ops::rename_in_place(old, &plain_name(new)?).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Self::Copy { items } => {
                for (source, destination) in items {
                    ops::copy_path(source, destination).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Self::Move { items } => {
                for (source, destination) in items {
                    ops::move_path(source, destination).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Self::CreateFolder { path } => {
                let name = plain_name(path)?;
                let parent = path
                    .parent()
                    .ok_or_else(|| "cannot recreate a folder without a parent".to_string())?;
                ops::create_dir(parent, &name).map(|_| ()).map_err(|e| e.to_string())
            }
            Self::Trash { items } => {
                for item in items {
                    ops::trash_path(&item.original_path()).map_err(|e| e.to_string())?;
                }
                Ok(())
            }
        }
    }
}

// The bare final name of `path`, or a user-facing error when it has none.
fn plain_name(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("path has no usable file name: {}", path.display()))
}

/// Window-scoped undo/redo history (T054). Pure state: the page pops an entry,
/// executes `reverse`/`forward` on the filesystem, and reports success back
/// via [`UndoStack::record_undone`] / [`UndoStack::record_redone`] so a failed
/// undo or redo is dropped from the history rather than re-offered.
#[derive(Debug, Default)]
pub(crate) struct UndoStack {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
}

impl UndoStack {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Records a new mutation: clears the redo branch (a fresh op invalidates
    /// the redo history) and evicts the oldest entry once the stack exceeds
    /// [`UNDO_CAPACITY`] (silent, per the ticket — no user-visible warning).
    pub(crate) fn push(&mut self, entry: UndoEntry) {
        self.redo.clear();
        self.undo.push(entry);
        if self.undo.len() > UNDO_CAPACITY {
            self.undo.remove(0);
        }
    }

    /// Pops the most recent entry to undo. The caller executes `reverse()` and
    /// reports success via [`Self::record_undone`]; a failed undo is dropped.
    pub(crate) fn undo(&mut self) -> Option<UndoEntry> {
        self.undo.pop()
    }

    /// Pops the most recent entry to redo. The caller executes `forward()` and
    /// reports success via [`Self::record_redone`]; a failed redo is dropped.
    pub(crate) fn redo(&mut self) -> Option<UndoEntry> {
        self.redo.pop()
    }

    /// Moves a successfully-undone entry onto the redo branch.
    pub(crate) fn record_undone(&mut self, entry: UndoEntry) {
        self.redo.push(entry);
    }

    /// Moves a successfully-redone entry back onto the undo branch.
    pub(crate) fn record_redone(&mut self, entry: UndoEntry) {
        self.undo.push(entry);
    }

    #[cfg(test)]
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    #[cfg(test)]
    pub(crate) fn redo_len(&self) -> usize {
        self.redo.len()
    }
}

/// Builds the single undo entry for one completed paste/drop transfer gesture
/// from its report: one compound `Copy`/`Move` entry over every successful
/// item, excluding T053 `Overwrite` successes (architect decision #1 — an
/// overwrite destroys the destination's prior contents and is not undoable in
/// T054). Returns `None` when nothing undoable transferred.
pub(crate) fn transfer_entry(report: &TransferReport, mode: TransferMode) -> Option<UndoEntry> {
    let items = report
        .successes
        .iter()
        .filter(|success| !success.overwrote)
        .map(|success| (success.source.clone(), success.destination.clone()))
        .collect::<Vec<_>>();
    if items.is_empty() {
        return None;
    }
    Some(match mode {
        TransferMode::Copy => UndoEntry::Copy { items },
        TransferMode::Move => UndoEntry::Move { items },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // -- Stack semantics (pure, no filesystem) --

    #[test]
    fn stack_undo_pops_most_recent_and_redo_restores_it() {
        let mut stack = UndoStack::new();
        stack.push(entry_for("1"));
        stack.push(entry_for("2"));

        assert!(stack.can_undo());
        let popped = stack.undo().expect("an entry to undo");
        assert_eq!(popped_path(&popped), "/tmp/undo-2.txt");
        // The page confirms a successful undo before the entry is offered as a
        // redo; a failed undo is dropped.
        assert!(!stack.can_redo());
        stack.record_undone(popped);
        assert!(stack.can_redo());

        let redone = stack.redo().expect("an entry to redo");
        assert_eq!(popped_path(&redone), "/tmp/undo-2.txt");
        stack.record_redone(redone);
        assert!(!stack.can_redo());
        assert!(stack.can_undo());
    }

    #[test]
    fn stack_new_mutation_clears_redo_branch() {
        let mut stack = UndoStack::new();
        stack.push(entry_for("1"));
        let popped = stack.undo().expect("an entry");
        stack.record_undone(popped);
        assert!(stack.can_redo());

        stack.push(entry_for("3"));
        assert!(!stack.can_redo(), "a new op invalidates the redo history");
        // The undone op was discarded with the redo branch: only the new one
        // remains (the page pops without redo until `record_undone`).
        assert_eq!(stack.undo_len(), 1);
    }

    #[test]
    fn stack_evicts_oldest_over_capacity() {
        let mut stack = UndoStack::new();
        for i in 0..(UNDO_CAPACITY + 10) {
            stack.push(entry_for(&i.to_string()));
        }
        assert_eq!(stack.undo_len(), UNDO_CAPACITY);
        // Drain the stack; the oldest remaining entry is the 11th pushed — the
        // first ten were silently evicted.
        let mut oldest = None;
        while let Some(entry) = stack.undo() {
            oldest = Some(entry);
        }
        assert_eq!(first_path(&oldest.expect("an entry")), "/tmp/undo-10.old");
    }

    #[test]
    fn stack_record_undone_and_redone_round_trip() {
        let mut stack = UndoStack::new();
        stack.push(entry_for("1"));
        stack.push(entry_for("2"));
        let entry = stack.undo().expect("an entry");
        stack.record_undone(entry);
        assert!(stack.can_redo());

        let entry = stack.redo().expect("an entry");
        stack.record_redone(entry);
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_len(), 2, "redone entry returns to the undo branch");
    }

    #[test]
    fn stack_empty_undo_and_redo_are_noops() {
        let mut stack = UndoStack::new();
        assert!(stack.undo().is_none());
        assert!(stack.redo().is_none());
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    // -- Filesystem reversibility (reverse/forward) --

    #[test]
    fn rename_reverse_and_forward_round_trip() {
        let dir = tempdir().unwrap();
        let old = dir.path().join("old.txt");
        let new = dir.path().join("new.txt");
        fs::write(&old, "x").unwrap();

        let entry = UndoEntry::Rename {
            old: old.clone(),
            new: new.clone(),
        };
        // Forward (what the redo re-applies) from the original state.
        entry.forward().expect("forward re-applies the rename");
        assert!(!old.exists());
        assert!(new.exists());

        entry.reverse().expect("reverse undoes the rename");
        assert!(old.exists());
        assert!(!new.exists());
    }

    #[test]
    fn batch_rename_reverse_restores_original_names() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        let c = dir.path().join("c.txt");
        let d = dir.path().join("d.txt");
        fs::write(&a, "A").unwrap();
        fs::write(&b, "B").unwrap();

        // A realistic batch gesture renames each entry to a distinct new name
        // (the dialog's preview resolves collisions), so the reverse of
        // `a→c, b→d` restores both originals.
        let entry = UndoEntry::BatchRename {
            renames: vec![(a.clone(), c.clone()), (b.clone(), d.clone())],
        };
        entry.forward().expect("forward re-applies the batch gesture");
        assert!(c.exists() && d.exists());

        entry.reverse().expect("reverse restores every original name");
        assert_eq!(fs::read_to_string(&a).unwrap(), "A");
        assert_eq!(fs::read_to_string(&b).unwrap(), "B");
        assert!(!c.exists() && !d.exists());
    }

    #[test]
    fn copy_reverse_deletes_destination_and_forward_recopies() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("dest").join("source.txt");
        fs::write(&source, "payload").unwrap();
        ops::copy_path(&source, &destination).expect("fixture copy");

        let entry = UndoEntry::Copy {
            items: vec![(source.clone(), destination.clone())],
        };
        entry.reverse().expect("reverse deletes the copy");
        assert!(source.exists(), "reverse of a copy preserves the source");
        assert!(!destination.exists());

        entry.forward().expect("forward re-applies the copy");
        assert_eq!(fs::read_to_string(&destination).unwrap(), "payload");
    }

    #[test]
    fn move_reverse_moves_back_and_forward_removes() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("from");
        let dest_dir = dir.path().join("to");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir(&dest_dir).unwrap();
        let source = source_dir.join("file.txt");
        let destination = dest_dir.join("file.txt");
        fs::write(&source, "moved").unwrap();

        let entry = UndoEntry::Move {
            items: vec![(source.clone(), destination.clone())],
        };
        entry.forward().expect("forward re-applies the move");
        assert!(!source.exists());
        assert!(destination.exists());

        entry.reverse().expect("reverse moves the file back");
        assert!(source.exists());
        assert!(!destination.exists());
        assert_eq!(fs::read_to_string(&source).unwrap(), "moved");
    }

    #[test]
    fn create_folder_reverse_deletes_and_forward_recreates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("New Folder");
        ops::create_dir(dir.path(), "New Folder").expect("fixture folder");

        let entry = UndoEntry::CreateFolder { path: path.clone() };
        entry.reverse().expect("reverse deletes the folder");
        assert!(!path.exists());

        entry.forward().expect("forward recreates the folder");
        assert!(path.is_dir());
    }

    #[test]
    fn create_folder_reverse_refuses_a_populated_folder() {
        // A user who created a folder, dropped files in it, then hit Ctrl+Z
        // must not lose those files: the reverse only removes empty folders.
        let dir = tempdir().unwrap();
        let path = dir.path().join("New Folder");
        ops::create_dir(dir.path(), "New Folder").expect("fixture folder");
        fs::write(path.join("kept.txt"), "user data").unwrap();

        let entry = UndoEntry::CreateFolder { path: path.clone() };
        let error = entry.reverse().expect_err("populated folder refuses undo");
        assert!(error.contains("not empty"), "{error}");
        assert!(path.is_dir(), "the folder survives");
        assert!(
            fs::read_to_string(path.join("kept.txt")).unwrap() == "user data",
            "the user's file survives"
        );
    }

    #[test]
    fn transfer_entry_excludes_overwrites_and_returns_none_on_empty() {
        let report = TransferReport {
            successes: vec![
                success("a.txt", false),
                success("b.txt", true), // overwritten → not undoable
            ],
            failures: Vec::new(),
        };
        let entry = transfer_entry(&report, TransferMode::Copy).expect("a copy entry");
        match entry {
            UndoEntry::Copy { items } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].0, PathBuf::from("/a.txt"));
            }
            other => panic!("expected Copy entry, got {other:?}"),
        }

        let all_overwrites = TransferReport {
            successes: vec![success("b.txt", true)],
            failures: Vec::new(),
        };
        assert!(transfer_entry(&all_overwrites, TransferMode::Move).is_none());
        let empty = TransferReport {
            successes: Vec::new(),
            failures: Vec::new(),
        };
        assert!(transfer_entry(&empty, TransferMode::Copy).is_none());
    }

    // -- Fixtures --

    fn entry_for(tag: &str) -> UndoEntry {
        UndoEntry::Rename {
            old: PathBuf::from(format!("/tmp/undo-{tag}.old")),
            new: PathBuf::from(format!("/tmp/undo-{tag}.txt")),
        }
    }

    fn popped_path(entry: &UndoEntry) -> String {
        match entry {
            UndoEntry::Rename { new, .. } => new.to_string_lossy().into_owned(),
            _ => panic!("fixture entries are renames"),
        }
    }

    fn first_path(entry: &UndoEntry) -> String {
        match entry {
            UndoEntry::Rename { old, .. } => old.to_string_lossy().into_owned(),
            _ => panic!("fixture entries are renames"),
        }
    }

    fn success(name: &str, overwrote: bool) -> ops::TransferSuccess {
        ops::TransferSuccess {
            source: PathBuf::from(format!("/{name}")),
            destination: PathBuf::from(format!("/dst/{name}")),
            renamed: false,
            move_kind: None,
            overwrote,
        }
    }
}
