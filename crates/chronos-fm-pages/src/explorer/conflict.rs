//! Paste/drop conflict detection and decision queue (T053).
//!
//! The dialog runs on the UI thread; the transfer runs on the background
//! executor. This module therefore pre-flights destination conflicts and
//! collects a per-source resolution plan ([`ConflictQueue::resolutions`])
//! that is handed to `ops::transfer_paths_resolved` once every conflict has
//! been decided — or aborted, when the user cancels (nothing transfers).
//!
//! Everything here is pure (only [`detect_conflicts`] touches the filesystem,
//! and only through `ops::would_conflict` + metadata reads), so the decision
//! matrix is unit-testable without a GPUI window — mirroring `dnd.rs`'s
//! `can_accept_*` functions.

use chronos_fm_services::fs::ops::{ConflictResolution, would_conflict};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// One destination conflict presented to the user (mockup §1.2).
#[derive(Clone, Debug)]
pub(crate) struct ConflictItem {
    /// File name shown in the dialog (equals the colliding destination name).
    pub name: String,
    /// Destination directory display name (e.g. the pane cwd's last segment).
    pub destination_dir: String,
    /// Source size in bytes, for the dialog's detail line.
    pub size: u64,
    /// Source modification time in unix seconds, for the dialog's detail line.
    pub modified: i64,
}

/// A user choice for one conflict. `Cancel` aborts the whole operation; the
/// rest are recorded as [`ConflictResolution`] decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConflictChoice {
    Skip,
    Rename,
    Overwrite,
    Cancel,
}

/// Signal that the user cancelled the operation (nothing transfers).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConflictAborted;

/// Finds every source whose destination entry is already occupied, in transfer
/// order. Returns `(source_index, item)` pairs for the dialog queue.
pub(crate) fn detect_conflicts(
    sources: &[PathBuf],
    destination: &Path,
) -> Vec<(usize, ConflictItem)> {
    let destination_dir = match destination.file_name().and_then(|name| name.to_str()) {
        Some(name) => name.to_string(),
        None => destination.to_string_lossy().into_owned(),
    };
    sources
        .iter()
        .enumerate()
        .filter_map(|(index, source)| {
            let name = source.file_name().and_then(|name| name.to_str())?;
            if !would_conflict(&destination.join(name)) {
                return None;
            }
            let metadata = std::fs::symlink_metadata(source).ok();
            let size = metadata.as_ref().map(|meta| meta.len()).unwrap_or(0);
            let modified = metadata
                .as_ref()
                .and_then(|meta| meta.modified().ok())
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);
            Some((
                index,
                ConflictItem {
                    name: name.to_string(),
                    destination_dir: destination_dir.clone(),
                    size,
                    modified,
                },
            ))
        })
        .collect()
}

/// Pure decision queue for one paste/drop operation.
#[derive(Clone, Debug)]
pub(crate) struct ConflictQueue {
    /// Source indices that conflict, in transfer order.
    conflict_indices: Vec<usize>,
    /// Index into `conflict_indices` of the next conflict to present.
    cursor: usize,
    /// Resolutions decided so far, parallel to the original sources slice.
    /// `None` = no conflict (or no decision yet); non-conflicting sources stay
    /// `None` forever so the service keeps its auto-rename fallback for them.
    pub resolutions: Vec<Option<ConflictResolution>>,
    /// Decision stored when the user checked "apply to all".
    pub apply_all: Option<ConflictResolution>,
}

impl ConflictQueue {
    /// Builds a queue for `source_count` sources with conflicts at
    /// `conflict_indices` (the indices [`detect_conflicts`] returned).
    pub(crate) fn new(source_count: usize, conflict_indices: Vec<usize>) -> Self {
        Self {
            conflict_indices,
            cursor: 0,
            resolutions: vec![None; source_count],
            apply_all: None,
        }
    }

    /// Conflicts still awaiting a decision (0 once apply-to-all fills them).
    pub(crate) fn remaining(&self) -> usize {
        self.conflict_indices.len() - self.cursor
    }

    /// The source index of the next conflict to present, if any.
    pub(crate) fn next_conflict_index(&self) -> Option<usize> {
        self.conflict_indices.get(self.cursor).copied()
    }

    /// Records a user decision for the current conflict. With `apply_to_all`
    /// the same decision is adopted for every *remaining conflict* (never for
    /// non-conflicting sources). Returns `Err(ConflictAborted)` when the user
    /// cancelled — the operation is aborted and no resolution is recorded.
    pub(crate) fn apply(
        &mut self,
        decision: ConflictChoice,
        apply_to_all: bool,
    ) -> Result<(), ConflictAborted> {
        if decision == ConflictChoice::Cancel {
            return Err(ConflictAborted);
        }
        let resolution = match decision {
            ConflictChoice::Skip => ConflictResolution::Skip,
            ConflictChoice::Rename => ConflictResolution::Rename,
            ConflictChoice::Overwrite => ConflictResolution::Overwrite,
            ConflictChoice::Cancel => unreachable!("cancel handled above"),
        };
        let Some(index) = self.next_conflict_index() else {
            return Ok(());
        };
        self.resolutions[index] = Some(resolution);
        self.cursor += 1;
        if apply_to_all {
            self.apply_all = Some(resolution);
            for conflict_index in &self.conflict_indices[self.cursor..] {
                self.resolutions[*conflict_index] = Some(resolution);
            }
            self.cursor = self.conflict_indices.len();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_fm_services::fs::ops::ConflictResolution as R;

    fn queue(conflicts: &[usize], source_count: usize) -> ConflictQueue {
        ConflictQueue::new(source_count, conflicts.to_vec())
    }

    #[test]
    fn remaining_starts_at_conflict_count_and_decrements() {
        let mut q = queue(&[1, 3, 5], 6);
        assert_eq!(q.remaining(), 3);
        assert_eq!(q.next_conflict_index(), Some(1));
        q.apply(ConflictChoice::Rename, false).unwrap();
        assert_eq!(q.remaining(), 2);
        assert_eq!(q.next_conflict_index(), Some(3));
        q.apply(ConflictChoice::Skip, false).unwrap();
        q.apply(ConflictChoice::Overwrite, false).unwrap();
        assert_eq!(q.remaining(), 0);
        assert_eq!(q.next_conflict_index(), None);
    }

    #[test]
    fn skip_rename_overwrite_record_only_the_current_conflict() {
        let mut q = queue(&[0, 1, 2], 3);
        q.apply(ConflictChoice::Rename, false).unwrap();
        q.apply(ConflictChoice::Skip, false).unwrap();
        q.apply(ConflictChoice::Overwrite, false).unwrap();
        assert_eq!(q.resolutions, vec![Some(R::Rename), Some(R::Skip), Some(R::Overwrite)]);
        assert_eq!(q.apply_all, None);
    }

    #[test]
    fn apply_to_all_fills_remaining_conflicts_but_never_clean_sources() {
        // 4 sources; only indices 0 and 2 conflict. Applying Overwrite to all
        // must resolve BOTH conflicts but leave the clean sources untouched.
        let mut q = queue(&[0, 2], 4);
        q.apply(ConflictChoice::Overwrite, true).unwrap();
        assert_eq!(q.remaining(), 0);
        assert_eq!(
            q.resolutions,
            vec![Some(R::Overwrite), None, Some(R::Overwrite), None]
        );
        assert_eq!(q.apply_all, Some(R::Overwrite));
    }

    #[test]
    fn apply_to_all_at_second_conflict_mixes_decisions() {
        let mut q = queue(&[0, 1, 2], 3);
        q.apply(ConflictChoice::Rename, false).unwrap();
        q.apply(ConflictChoice::Skip, true).unwrap();
        assert_eq!(q.resolutions, vec![Some(R::Rename), Some(R::Skip), Some(R::Skip)]);
        assert_eq!(q.apply_all, Some(R::Skip));
        assert_eq!(q.remaining(), 0);
    }

    #[test]
    fn cancel_aborts_without_recording_or_advancing() {
        let mut q = queue(&[0, 1], 2);
        q.apply(ConflictChoice::Rename, false).unwrap();
        let before = q.clone();
        let result = q.apply(ConflictChoice::Cancel, false);
        assert_eq!(result, Err(ConflictAborted));
        assert_eq!(q.remaining(), before.remaining());
        assert_eq!(q.resolutions, before.resolutions);
        assert_eq!(q.apply_all, before.apply_all);
    }

    #[test]
    fn detect_conflicts_finds_only_occupied_destinations() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let fresh = src_dir.path().join("fresh.txt");
        let colliding = src_dir.path().join("taken.txt");
        std::fs::write(&fresh, "x").unwrap();
        std::fs::write(&colliding, "x").unwrap();
        std::fs::write(dst_dir.path().join("taken.txt"), "old").unwrap();

        let sources = vec![fresh, colliding];
        let conflicts = detect_conflicts(&sources, dst_dir.path());
        assert_eq!(conflicts.len(), 1);
        let (index, item) = &conflicts[0];
        assert_eq!(*index, 1, "only the colliding source conflicts");
        assert_eq!(item.name, "taken.txt");
        assert_eq!(item.size, 1);
    }
}
