//! Filesystem mutation operations: copy, move, rename, create, trash, and
//! permanent delete, with cross-volume awareness and conflict-name resolution.
//!
//! These functions are synchronous filesystem IO. Callers that must stay
//! responsive during large operations should run them on a background executor
//! (mirroring how `search` offloads work via `cx.background_spawn`). The UI
//! layer is expected to route all mutations through this module rather than
//! calling `std::fs` directly (see `docs/explorer-essentials.md` §8).

use chronos_fm_core::errors::{Error, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

// Re-exported so the undo stack (T054) can hold trash entries without
// depending on the trash crate itself; restore still routes through this module.
pub use trash::TrashItem;

/// How a [`move_path`] operation was carried out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    /// The move stayed on a single filesystem and used `rename(2)`.
    Rename,
    /// Source and destination were on different filesystems, so the move was
    /// performed as a recursive copy followed by deleting the source.
    CrossVolume,
}

/// Whether a batch transfer copies or moves its sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferMode {
    /// Preserve each source after creating its destination copy.
    Copy,
    /// Remove each source after moving it to the destination.
    Move,
}

/// One successfully transferred source and its resolved destination.
#[derive(Debug)]
pub struct TransferSuccess {
    /// Original source path.
    pub source: PathBuf,
    /// Final destination path after conflict-name resolution.
    pub destination: PathBuf,
    /// Whether conflict resolution changed the destination file name.
    pub renamed: bool,
    /// How a move was performed, or `None` for a copy.
    pub move_kind: Option<MoveKind>,
    /// Whether the transfer replaced an existing destination entry (a T053
    /// `Overwrite` resolution that actually hit an occupied path). Overwritten
    /// transfers destroy the destination's prior contents and are therefore
    /// excluded from the undo stack (T054 architect decision #1).
    pub overwrote: bool,
}

/// One source that could not be transferred.
#[derive(Debug)]
pub struct TransferFailure {
    /// Original source path.
    pub source: PathBuf,
    /// Error returned while validating or transferring the source.
    pub error: Error,
}

/// Structured outcomes for a batch transfer, including partial completion.
#[derive(Debug)]
pub struct TransferReport {
    /// Sources that were transferred successfully.
    pub successes: Vec<TransferSuccess>,
    /// Sources that failed validation or transfer.
    pub failures: Vec<TransferFailure>,
}

/// How a name collision at the destination should be resolved when copying or
/// moving. [`transfer_paths_resolved`] applies these decisions per source;
/// this module also provides the building blocks ([`would_conflict`],
/// [`unique_name`]) for callers that pre-flight conflicts themselves (T053).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep both items by writing to a non-colliding name (see [`unique_name`]).
    Rename,
    /// Replace the existing destination.
    Overwrite,
    /// Leave the destination untouched and skip this item.
    Skip,
}

/// Returns whether `dst` is already occupied, the condition that triggers
/// conflict resolution before a copy or move.
///
/// Unlike [`Path::exists`], this probes the entry itself rather than following
/// symlinks, so a dangling symlink at `dst` still counts as occupied. An
/// ambiguous error (e.g. a permission failure while stat-ing) is treated
/// conservatively as occupied so the caller surfaces the conflict path rather
/// than silently overwriting.
pub fn would_conflict(dst: &Path) -> bool {
    path_occupied(dst)
}

// Whether a filesystem entry exists at `path`, detecting the entry itself
// (including a broken symlink) rather than following links. Ambiguous errors
// are reported as occupied; only a definitive "not found" is reported as free.
fn path_occupied(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}

// Validates that `name` is a single, normal path component — not empty, not `.`
// or `..`, and free of path separators — so child-name inputs cannot escape the
// target directory when joined.
fn ensure_plain_name(name: &str) -> Result<()> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(component)), None) if component == std::ffi::OsStr::new(name) => {
            Ok(())
        }
        _ => Err(Error::Other(format!("invalid file name: {name:?}"))),
    }
}

/// Returns `true` when `src` and `dst_dir` reside on different filesystems, in
/// which case a move cannot use `rename(2)` and must copy then delete.
///
/// On non-Unix platforms device ids are not consulted, so this conservatively
/// returns `false`; [`move_path`] still detects the cross-device error from
/// `rename` and falls back to copy + delete regardless.
pub fn is_cross_volume(src: &Path, dst_dir: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // `rename(2)` acts on the source directory entry itself, so use the
        // entry's own device (don't follow a symlink). The destination is the
        // directory the entry lands in, so its resolved device is what matters.
        let src_dev = fs::symlink_metadata(src)?.dev();
        let dst_dev = fs::metadata(dst_dir)?.dev();
        Ok(src_dev != dst_dev)
    }
    #[cfg(not(unix))]
    {
        let _ = (src, dst_dir);
        Ok(false)
    }
}

/// Produces a file name within `dir` that does not collide with an existing
/// entry, deriving it from `name` by inserting ` (N)` before the extension
/// (`report.pdf` becomes `report (2).pdf`), trying `N = 2, 3, ...` until a free
/// name is found. Returns `name` unchanged when there is no collision.
pub fn unique_name(dir: &Path, name: &str) -> String {
    if !path_occupied(&dir.join(name)) {
        return name.to_string();
    }
    let path = Path::new(name);
    let extension = path.extension().and_then(|ext| ext.to_str());
    // `file_stem` is `None` only for empty names; fall back to the full name so
    // we never panic and always make progress.
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let mut counter: u32 = 2;
    loop {
        let candidate = match extension {
            Some(extension) => format!("{stem} ({counter}).{extension}"),
            None => format!("{stem} ({counter})"),
        };
        if !path_occupied(&dir.join(&candidate)) {
            return candidate;
        }
        counter += 1;
    }
}

/// Copies or moves every source into `destination`, keeping both entries on a
/// name collision and preserving successes alongside failures in one report.
/// Equivalent to [`transfer_paths_resolved`] with no explicit resolutions.
pub fn transfer_paths(
    sources: &[PathBuf],
    destination: &Path,
    mode: TransferMode,
) -> TransferReport {
    let resolutions = vec![None; sources.len()];
    transfer_paths_resolved(sources, destination, mode, &resolutions)
}

/// Copies or moves every source into `destination`, honoring an explicit
/// per-source conflict-resolution plan produced by a caller that pre-flighted
/// the conflicts (T053). `resolutions` is parallel to `sources`:
///
/// - `Some(Rename)` — keep both via [`unique_name`], recomputed at transfer
///   time (so a destination change between dialog and transfer stays safe).
/// - `Some(Overwrite)` — write to the original destination name, replacing it.
/// - `Some(Skip)` — leave the destination untouched; the source is omitted
///   from both successes and failures.
/// - `None` — no conflict seen by the caller (or no decision made): falls back
///   to the historic [`unique_name`] auto-rename.
pub fn transfer_paths_resolved(
    sources: &[PathBuf],
    destination: &Path,
    mode: TransferMode,
    resolutions: &[Option<ConflictResolution>],
) -> TransferReport {
    let mut report = TransferReport {
        successes: Vec::new(),
        failures: Vec::new(),
    };

    for (index, source) in sources.iter().enumerate() {
        let Some(name) = source.file_name().and_then(|name| name.to_str()) else {
            report.failures.push(TransferFailure {
                source: source.clone(),
                error: Error::Other(format!(
                    "cannot transfer path without a valid final name: {}",
                    source.display()
                )),
            });
            continue;
        };

        // T053: honor the caller's explicit decision for this source.
        if matches!(resolutions.get(index), Some(Some(ConflictResolution::Skip))) {
            continue;
        }
        let overwrite = matches!(
            resolutions.get(index),
            Some(Some(ConflictResolution::Overwrite))
        );
        let resolved_name = if overwrite {
            name.to_string()
        } else {
            unique_name(destination, name)
        };
        let renamed = resolved_name != name;
        let resolved_destination = destination.join(resolved_name);
        let result = match mode {
            TransferMode::Copy => copy_path(source, &resolved_destination).map(|()| None),
            TransferMode::Move => move_path(source, &resolved_destination).map(Some),
        };

        match result {
            Ok(move_kind) => {
                // Only an Overwrite decision that actually replaced an occupied
                // destination destroys prior contents; a stale Overwrite plan
                // against a freed path is a plain transfer.
                let overwrote = overwrite && path_occupied(&resolved_destination);
                report.successes.push(TransferSuccess {
                    source: source.clone(),
                    destination: resolved_destination,
                    renamed,
                    move_kind,
                    overwrote,
                });
            }
            Err(error) => report.failures.push(TransferFailure {
                source: source.clone(),
                error,
            }),
        }
    }

    report
}

/// Recursively copies `src` (a file or directory) to `dst`, where `dst` is the
/// full destination path rather than its parent directory. Missing parent
/// directories are created; an existing destination file is overwritten.
pub fn copy_path(src: &Path, dst: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(src)?;
    if metadata.is_dir() {
        copy_dir_all(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        Ok(())
    }
}

// Recursively copies the contents of directory `src` into `dst`, creating
// `dst` and any intermediate directories. Symlinks are copied via `fs::copy`
// (following the link) rather than recursed into, to avoid cycles.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Moves `src` to `dst`. Uses `rename(2)` when both reside on the same
/// filesystem; otherwise (a cross-volume move) copies `src` recursively to
/// `dst` and then removes the source, reporting which path was taken.
pub fn move_path(src: &Path, dst: &Path) -> Result<MoveKind> {
    match fs::rename(src, dst) {
        Ok(()) => Ok(MoveKind::Rename),
        Err(error) if is_cross_device(&error) => {
            copy_path(src, dst)?;
            delete_permanent(src)?;
            Ok(MoveKind::CrossVolume)
        }
        Err(error) => Err(Error::Io(error)),
    }
}

// Whether an IO error from `rename` indicates the source and destination are on
// different devices, the signal to fall back to copy + delete.
#[cfg(unix)]
fn is_cross_device(error: &std::io::Error) -> bool {
    // EXDEV ("Invalid cross-device link") is 18 on Linux and macOS.
    error.raw_os_error() == Some(18)
}

#[cfg(windows)]
fn is_cross_device(error: &std::io::Error) -> bool {
    // ERROR_NOT_SAME_DEVICE.
    error.raw_os_error() == Some(17)
}

#[cfg(not(any(unix, windows)))]
fn is_cross_device(_error: &std::io::Error) -> bool {
    false
}

/// Renames `src` to `new_name` within its current directory, returning the new
/// full path. `new_name` must be a bare file name, not a path with separators.
pub fn rename_in_place(src: &Path, new_name: &str) -> Result<PathBuf> {
    ensure_plain_name(new_name)?;
    let parent = src.parent().ok_or_else(|| {
        Error::Other(format!(
            "cannot rename path without a parent: {}",
            src.display()
        ))
    })?;
    let dst = parent.join(new_name);
    fs::rename(src, &dst)?;
    Ok(dst)
}

/// Creates a new directory named `name` inside `parent`, returning its full
/// path. Fails if a file or directory of that name already exists.
pub fn create_dir(parent: &Path, name: &str) -> Result<PathBuf> {
    ensure_plain_name(name)?;
    let dst = parent.join(name);
    fs::create_dir(&dst)?;
    Ok(dst)
}

/// Moves `path` to the operating system's trash/recycle bin.
pub fn trash_path(path: &Path) -> Result<()> {
    trash::delete(path).map_err(|error| Error::Other(format!("failed to move to trash: {error}")))
}

/// Moves `path` to the OS trash and returns its [`TrashItem`], so the move can
/// later be undone by restoring it (T054).
///
/// `trash::delete` discards everything about where the item went, and restore
/// (`trash::os_limited::restore_all`) consumes [`TrashItem`]s obtained from
/// `trash::os_limited::list()` — so the entry is captured here by matching the
/// original path right after the delete, at the caller's expense of one list.
pub fn trash_path_undoable(path: &Path) -> Result<TrashItem> {
    trash_path(path)?;
    let items = trash::os_limited::list()
        .map_err(|error| Error::Other(format!("failed to list trash: {error}")))?;
    items
        .into_iter()
        .find(|item| item.original_path() == path)
        .ok_or_else(|| {
            Error::Other(format!(
                "moved {} to trash but could not locate its trash entry",
                path.display()
            ))
        })
}

/// Restores `items` from the OS trash to their original locations (T054 undo).
///
/// Each recorded item is restored by identity when its trashinfo is still
/// present. When a recorded item's trashinfo no longer exists — undo → redo
/// re-trashes the file and creates a *new* trash entry, so the recorded id is
/// stale — the restore falls back to the live entry whose original path
/// matches (T054 redo of a trash op). A recorded item with neither a live
/// trashinfo nor a matching entry is passed through untouched so the restore
/// error surfaces honestly.
pub fn restore_trash_items(items: Vec<TrashItem>) -> Result<()> {
    let live = trash::os_limited::list()
        .map_err(|error| Error::Other(format!("failed to list trash: {error}")))?;
    let resolved = items
        .into_iter()
        .map(|item| {
            if live.iter().any(|candidate| candidate.id == item.id) {
                item
            } else {
                live.iter()
                    .find(|candidate| candidate.original_path() == item.original_path())
                    .cloned()
                    .unwrap_or(item)
            }
        })
        .collect::<Vec<_>>();
    trash::os_limited::restore_all(resolved)
        .map_err(|error| Error::Other(format!("failed to restore from trash: {error}")))
}

/// Deletes a directory only when it is empty, with a user-facing error when it
/// is not. Undo of "New Folder" must never recurse into a folder the user has
/// since populated (T054) — `delete_permanent` is recursive and would destroy
/// their files.
pub fn delete_empty_dir(path: &Path) -> Result<()> {
    fs::remove_dir(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::DirectoryNotEmpty {
            Error::Other(format!("{} is not empty; undo aborted", path.display()))
        } else {
            Error::Io(error)
        }
    })
}

/// Permanently deletes `path`, whether it is a file, symlink, or directory
/// tree. This cannot be undone.
pub fn delete_permanent(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_methods)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn unique_name_returns_input_when_no_collision() {
        let dir = tempdir().unwrap();
        assert_eq!(unique_name(dir.path(), "report.pdf"), "report.pdf");
    }

    #[test]
    fn unique_name_inserts_counter_before_extension() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("report.pdf"), "a").unwrap();
        assert_eq!(unique_name(dir.path(), "report.pdf"), "report (2).pdf");

        fs::write(dir.path().join("report (2).pdf"), "b").unwrap();
        assert_eq!(unique_name(dir.path(), "report.pdf"), "report (3).pdf");
    }

    #[test]
    fn unique_name_handles_extensionless_names() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("folder")).unwrap();
        assert_eq!(unique_name(dir.path(), "folder"), "folder (2)");
    }

    #[test]
    fn would_conflict_reflects_existence() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("x");
        assert!(!would_conflict(&target));
        fs::write(&target, "x").unwrap();
        assert!(would_conflict(&target));
    }

    #[test]
    fn copy_path_copies_a_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("b.txt");
        fs::write(&src, "hello").unwrap();
        copy_path(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
        assert!(src.exists(), "source is preserved on copy");
    }

    #[test]
    fn copy_path_copies_a_directory_tree() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("top.txt"), "top").unwrap();
        fs::create_dir(src.join("nested")).unwrap();
        fs::write(src.join("nested").join("deep.txt"), "deep").unwrap();

        let dst = dir.path().join("dst");
        copy_path(&src, &dst).unwrap();

        assert_eq!(fs::read_to_string(dst.join("top.txt")).unwrap(), "top");
        assert_eq!(
            fs::read_to_string(dst.join("nested").join("deep.txt")).unwrap(),
            "deep"
        );
    }

    #[test]
    fn move_path_within_volume_renames_and_removes_source() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("a.txt");
        let dst = dir.path().join("sub").join("a.txt");
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(&src, "payload").unwrap();

        let kind = move_path(&src, &dst).unwrap();
        assert_eq!(kind, MoveKind::Rename);
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "payload");
    }

    #[test]
    fn transfer_paths_copy_keeps_both_with_unique_name() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        let destination = dir.path().join("destination");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&destination).unwrap();
        let src = source_dir.join("file.txt");
        fs::write(&src, "new").unwrap();
        fs::write(destination.join("file.txt"), "existing").unwrap();

        let report = transfer_paths(&[src.clone()], &destination, TransferMode::Copy);

        assert!(src.exists());
        assert_eq!(
            fs::read_to_string(destination.join("file.txt")).unwrap(),
            "existing"
        );
        assert_eq!(
            fs::read_to_string(destination.join("file (2).txt")).unwrap(),
            "new"
        );
        assert_eq!(report.successes.len(), 1);
        assert!(report.failures.is_empty());
        assert_eq!(report.successes[0].source, src);
        assert_eq!(
            report.successes[0].destination,
            destination.join("file (2).txt")
        );
        assert!(report.successes[0].renamed);
        assert_eq!(report.successes[0].move_kind, None);
    }

    #[test]
    fn transfer_paths_move_keeps_both_with_unique_name() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        let destination = dir.path().join("destination");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&destination).unwrap();
        let src = source_dir.join("file.txt");
        fs::write(&src, "moved").unwrap();
        fs::write(destination.join("file.txt"), "existing").unwrap();

        let report = transfer_paths(&[src.clone()], &destination, TransferMode::Move);

        assert!(!src.exists(), "move removes the source");
        assert_eq!(
            fs::read_to_string(destination.join("file.txt")).unwrap(),
            "existing"
        );
        assert_eq!(
            fs::read_to_string(destination.join("file (2).txt")).unwrap(),
            "moved"
        );
        assert_eq!(report.successes.len(), 1);
        assert!(report.failures.is_empty());
        assert_eq!(
            report.successes[0].destination,
            destination.join("file (2).txt")
        );
        assert!(report.successes[0].renamed);
        assert!(report.successes[0].move_kind.is_some());
    }

    #[test]
    fn transfer_paths_copy_without_collision_keeps_original_name() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        let destination = dir.path().join("destination");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&destination).unwrap();
        let src = source_dir.join("file.txt");
        fs::write(&src, "payload").unwrap();

        let report = transfer_paths(&[src.clone()], &destination, TransferMode::Copy);

        assert!(src.exists());
        assert!(destination.join("file.txt").exists());
        assert_eq!(report.successes.len(), 1);
        assert!(report.failures.is_empty());
        assert_eq!(
            report.successes[0].destination,
            destination.join("file.txt")
        );
        assert!(!report.successes[0].renamed);
        assert_eq!(report.successes[0].move_kind, None);
    }

    #[test]
    fn transfer_paths_rejects_sources_without_a_valid_final_component() {
        let destination = tempdir().unwrap();
        let empty = PathBuf::new();
        let root = PathBuf::from(std::path::MAIN_SEPARATOR.to_string());
        let mut sources = vec![empty.clone(), root.clone()];
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            sources.push(PathBuf::from(std::ffi::OsString::from_vec(vec![0xff])));
        }

        let report = transfer_paths(&sources, destination.path(), TransferMode::Copy);

        assert!(report.successes.is_empty());
        assert_eq!(report.failures.len(), sources.len());
        assert_eq!(report.failures[0].source, empty);
        assert_eq!(report.failures[1].source, root);
        assert!(
            report
                .failures
                .iter()
                .all(|failure| failure.error.to_string().contains("valid final name"))
        );
    }

    #[test]
    fn transfer_paths_report_preserves_total_failure() {
        let dir = tempdir().unwrap();
        let destination = dir.path().join("destination");
        fs::create_dir(&destination).unwrap();
        let missing_a = dir.path().join("missing-a.txt");
        let missing_b = dir.path().join("missing-b.txt");

        let report = transfer_paths(
            &[missing_a.clone(), missing_b.clone()],
            &destination,
            TransferMode::Copy,
        );

        assert!(report.successes.is_empty());
        assert_eq!(report.failures.len(), 2);
        assert_eq!(report.failures[0].source, missing_a);
        assert_eq!(report.failures[1].source, missing_b);
    }

    #[test]
    fn transfer_paths_report_preserves_partial_failure() {
        let dir = tempdir().unwrap();
        let destination = dir.path().join("destination");
        fs::create_dir(&destination).unwrap();
        let existing = dir.path().join("existing.txt");
        let missing = dir.path().join("missing.txt");
        fs::write(&existing, "payload").unwrap();

        let report = transfer_paths(
            &[existing.clone(), missing.clone()],
            &destination,
            TransferMode::Copy,
        );

        assert_eq!(report.successes.len(), 1);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.successes[0].source, existing);
        assert_eq!(report.failures[0].source, missing);
        assert!(destination.join("existing.txt").exists());
    }

    #[test]
    fn rename_in_place_changes_name_keeps_dir() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("old.txt");
        fs::write(&src, "x").unwrap();

        let dst = rename_in_place(&src, "new.txt").unwrap();
        assert_eq!(dst, dir.path().join("new.txt"));
        assert!(!src.exists());
        assert!(dst.exists());
    }

    #[test]
    fn create_dir_makes_a_new_directory() {
        let dir = tempdir().unwrap();
        let created = create_dir(dir.path(), "fresh").unwrap();
        assert_eq!(created, dir.path().join("fresh"));
        assert!(created.is_dir());
    }

    #[test]
    fn delete_permanent_removes_file_and_tree() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("f.txt");
        fs::write(&file, "x").unwrap();
        delete_permanent(&file).unwrap();
        assert!(!file.exists());

        let tree = dir.path().join("tree");
        fs::create_dir(&tree).unwrap();
        fs::write(tree.join("inner.txt"), "y").unwrap();
        delete_permanent(&tree).unwrap();
        assert!(!tree.exists());
    }

    #[test]
    fn would_conflict_detects_a_dangling_symlink() {
        let dir = tempdir().unwrap();
        let link = dir.path().join("broken");
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.path().join("missing-target"), &link).unwrap();
        #[cfg(not(unix))]
        std::fs::write(&link, "x").unwrap();
        // `Path::exists()` would report `false` for a dangling symlink; the
        // entry is nonetheless occupied.
        assert!(would_conflict(&link));
    }

    #[test]
    fn rename_and_create_reject_path_traversal_names() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("f.txt");
        fs::write(&src, "x").unwrap();
        for bad in ["../escape", "a/b", ".", "..", ""] {
            assert!(
                rename_in_place(&src, bad).is_err(),
                "rename allowed {bad:?}"
            );
            assert!(
                create_dir(dir.path(), bad).is_err(),
                "create allowed {bad:?}"
            );
        }
        // A plain name is still accepted.
        assert!(create_dir(dir.path(), "ok-dir").is_ok());
    }

    #[test]
    fn is_cross_volume_false_within_same_dir() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("a.txt");
        fs::write(&src, "x").unwrap();
        assert!(!is_cross_volume(&src, dir.path()).unwrap());
    }

    #[test]
    fn transfer_paths_resolved_mixed_plan_skips_overwrites_and_renames() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        fs::write(dst_dir.path().join("b.txt"), "old-b").unwrap();
        fs::write(dst_dir.path().join("c.txt"), "old-c").unwrap();
        fs::write(dst_dir.path().join("d.txt"), "old-d").unwrap();

        let a = src_dir.path().join("a.txt"); // no conflict → None
        let b = src_dir.path().join("b.txt"); // conflict → Overwrite
        let c = src_dir.path().join("c.txt"); // conflict → Skip
        let d = src_dir.path().join("d.txt"); // conflict → Rename
        fs::write(&a, "new").unwrap();
        fs::write(&b, "new").unwrap();
        fs::write(&c, "new").unwrap();
        fs::write(&d, "new").unwrap();

        let sources = [a, b, c, d];
        let resolutions = [
            None,
            Some(ConflictResolution::Overwrite),
            Some(ConflictResolution::Skip),
            Some(ConflictResolution::Rename),
        ];
        let report =
            transfer_paths_resolved(&sources, dst_dir.path(), TransferMode::Copy, &resolutions);

        assert_eq!(report.successes.len(), 3, "skipped source is not a success");
        assert_eq!(report.failures.len(), 0);

        // None → plain copy.
        assert_eq!(fs::read_to_string(dst_dir.path().join("a.txt")).unwrap(), "new");
        // Overwrite → replaced the existing destination in place.
        assert_eq!(fs::read_to_string(dst_dir.path().join("b.txt")).unwrap(), "new");
        // Skip → destination untouched, source still present.
        assert_eq!(fs::read_to_string(dst_dir.path().join("c.txt")).unwrap(), "old-c");
        assert!(sources[2].exists(), "skipped source is not consumed");
        // Rename → both kept under a unique name.
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("d (2).txt")).unwrap(),
            "new"
        );
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("d.txt")).unwrap(),
            "old-d"
        );

        // The report marks exactly the Overwrite success as `overwrote` (T054
        // uses it to exclude overwritten transfers from the undo stack).
        for success in &report.successes {
            if success.source == sources[1] {
                assert!(success.overwrote, "Overwrite success is marked");
            } else {
                assert!(!success.overwrote, "plain/renamed success is not an overwrite");
            }
        }
        assert!(report.successes.iter().any(|s| s.source == sources[0]));
        assert!(report.successes.iter().any(|s| s.source == sources[3]));
    }

    #[test]
    fn trash_path_undoable_and_restore_round_trip() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("trash-me.txt");
        fs::write(&file, "x").unwrap();

        let item = trash_path_undoable(&file).expect("delete to trash and locate entry");
        assert!(!file.exists(), "trash removes the original");
        assert_eq!(item.original_path(), file);

        restore_trash_items(vec![item]).expect("restore from trash");
        assert_eq!(fs::read_to_string(&file).unwrap(), "x", "restore brings the file back");
    }

    #[test]
    fn restore_trash_items_recovers_after_redo_retrash_created_a_new_entry() {
        // T054 undo → redo → undo on a trash op: redo re-trashes the file and
        // creates a *new* trash entry, so the recorded `TrashItem`'s id is
        // stale. Restoring the recorded item must fall back to the live entry
        // with the same original path instead of failing on the dead id.
        let dir = tempdir().unwrap();
        let file = dir.path().join("retrash.txt");
        fs::write(&file, "x").unwrap();

        let recorded = trash_path_undoable(&file).expect("first delete to trash");
        restore_trash_items(vec![recorded.clone()]).expect("first restore (undo)");
        assert!(file.exists());

        // Redo: re-trash without capturing (the redo path only re-trashes).
        trash_path(&file).expect("re-trash (redo)");
        assert!(!file.exists());

        // Second undo restores via the stale recorded item.
        restore_trash_items(vec![recorded]).expect("second restore through the stale id");
        assert!(file.exists(), "stale recorded item resolves to the live entry");
    }

    #[test]
    fn delete_empty_dir_only_removes_empty_directories() {
        let dir = tempdir().unwrap();
        let empty = dir.path().join("empty");
        fs::create_dir(&empty).unwrap();
        delete_empty_dir(&empty).expect("empty dir deletes");
        assert!(!empty.exists());

        let populated = dir.path().join("populated");
        fs::create_dir(&populated).unwrap();
        fs::write(populated.join("file.txt"), "x").unwrap();
        let error = delete_empty_dir(&populated).expect_err("non-empty dir refuses");
        assert!(
            error.to_string().contains("not empty"),
            "friendly message, got: {error}"
        );
        assert!(populated.exists(), "the folder and its contents survive");
        assert!(populated.join("file.txt").exists());
    }

    #[test]
    fn transfer_paths_resolved_rename_recomputed_at_transfer_time() {
        // The destination frees up between the plan and the transfer; a Rename
        // decision must fall back to the plain name instead of ` (2)`.
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("x.txt");
        fs::write(&src, "new").unwrap();
        fs::write(dst_dir.path().join("x.txt"), "old").unwrap();

        fs::remove_file(dst_dir.path().join("x.txt")).unwrap(); // freed after the plan
        let resolutions = [Some(ConflictResolution::Rename)];
        let report = transfer_paths_resolved(
            std::slice::from_ref(&src),
            dst_dir.path(),
            TransferMode::Copy,
            &resolutions,
        );

        assert_eq!(report.successes.len(), 1);
        assert_eq!(
            report.successes[0].destination,
            dst_dir.path().join("x.txt"),
            "rename resolves to the now-free plain name",
        );
        assert_eq!(report.successes[0].renamed, false);
    }
}
