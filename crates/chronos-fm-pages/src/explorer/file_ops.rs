//! Filesystem-mutating pane operations (copy, cut, paste, new folder,
//! delete), each backed by `chronos_fm_services::fs::ops` and followed by a
//! `reload()` so the listing reflects the result.

use std::path::Path;

use gpui::{Context, Window};
use gpui_component::WindowExt;
use gpui_component::button::ButtonVariant;
use gpui_component::dialog::DialogButtonProps;

use chronos_fm_services::fs::ops;

use super::ExplorerPane;
use super::clipboard::{self, ClipboardMode};
use super::types::{PaneEvent, StatusLevel};
use super::undo::{UndoEntry, transfer_entry};

impl ExplorerPane {
    /// Puts the current selection on the clipboard in Copy mode. No-op if
    /// nothing is selected.
    pub(crate) fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        clipboard::set_copy(paths, cx);
    }

    /// Puts the current selection on the clipboard in Cut mode. No-op if
    /// nothing is selected.
    pub(crate) fn cut_selection(&mut self, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        clipboard::set_cut(paths, cx);
        cx.notify();
    }

    /// Copies (or moves, for Cut) every clipboard path into the current
    /// directory. Destination collisions pause the paste behind the conflict
    /// dialog (T053) instead of silently auto-renaming; a Cut clipboard is
    /// cleared after the paste completes (even partially).
    pub(crate) fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        if self.conflict_dialog.is_some() {
            return;
        }
        let clip = clipboard::current(cx);
        let Some(mode) = clip.mode else {
            return;
        };
        let dst_dir = Path::new(&self.cwd).to_path_buf();
        let sources = clip
            .paths
            .iter()
            .map(Path::new)
            .map(Path::to_path_buf)
            .collect::<Vec<_>>();
        let transfer_mode = match mode {
            ClipboardMode::Copy => ops::TransferMode::Copy,
            ClipboardMode::Cut => ops::TransferMode::Move,
        };
        let is_cut = mode == ClipboardMode::Cut;
        let arg_sources = sources.clone();
        let arg_destination = dst_dir.clone();
        self.transfer_with_conflict_dialog(
            arg_sources,
            arg_destination,
            cx,
            move |pane, pane_cx, resolutions| {
                let report =
                    ops::transfer_paths_resolved(&sources, &dst_dir, transfer_mode, &resolutions);
                if let Some(entry) = transfer_entry(&report, transfer_mode) {
                    // T054: one window-level undo entry for the paste gesture
                    // (Overwrite successes are excluded inside `transfer_entry`).
                    pane_cx.emit(PaneEvent::Undoable(entry));
                }
                if is_cut {
                    clipboard::clear(pane_cx);
                }
                pane.reload();
                let success_count = report.successes.len();
                let failure_count = report.failures.len();
                if failure_count > 0 {
                    let errors = report
                        .failures
                        .iter()
                        .map(|failure| {
                            let source = failure
                                .source
                                .file_name()
                                .and_then(|name| name.to_str())
                                .map(str::to_owned)
                                .unwrap_or_else(|| failure.source.display().to_string());
                            format!("{source}: {}", failure.error)
                        })
                        .collect::<Vec<_>>();
                    pane.set_status(
                        StatusLevel::Error,
                        format!(
                            "Paste failed: {success_count} succeeded, {failure_count} failed; {}",
                            errors.join(", ")
                        ),
                    );
                }
                pane_cx.notify();
            },
        );
        cx.notify();
    }

    /// Creates a new, uniquely-named folder in the current directory and
    /// immediately starts an inline rename on it so the user can type a name.
    pub(crate) fn new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parent = Path::new(&self.cwd).to_path_buf();
        let name = ops::unique_name(&parent, "New Folder");
        let result = ops::create_dir(&parent, &name);
        self.reload();
        match result {
            Ok(path) => {
                // T054: undo deletes the created folder; a subsequent rename
                // pushes its own entry on top (architect decision #3).
                cx.emit(PaneEvent::Undoable(UndoEntry::CreateFolder { path: path.clone() }));
                let path_str = path.to_string_lossy().to_string();
                if let Some(ix) = self
                    .filtered_entries
                    .iter()
                    .position(|entry| entry.path == path_str)
                {
                    self.begin_rename(ix, window, cx);
                }
            }
            Err(error) => {
                self.set_status(StatusLevel::Error, format!("Could not create folder: {error}"));
            }
        }
        cx.notify();
    }

    /// Opens the shared trash-confirmation dialog for the current selection.
    /// Empty selections and repeated Delete events while a dialog is active
    /// are no-ops.
    pub(crate) fn confirm_delete_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        if paths.is_empty() || window.has_active_dialog(cx) {
            return;
        }

        let count = paths.len();
        let pane = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let paths = paths.clone();
            let pane = pane.clone();
            alert
                .title("Delete Selected Items?")
                .description(format!("{count} item(s) will be moved to Trash."))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Delete")
                        .ok_variant(ButtonVariant::Danger)
                        .show_cancel(true),
                )
                .on_ok(move |_, _window, cx| {
                    pane.update(cx, |pane, cx| pane.delete_paths(paths.clone(), cx));
                    true
                })
        });
    }

    /// Moves each of `paths` to the OS trash. Errors for individual paths are
    /// collected and reported together rather than aborting the whole batch.
    pub(crate) fn delete_paths(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        let mut errors: Vec<String> = Vec::new();
        let mut trashed: Vec<ops::TrashItem> = Vec::new();
        for path in &paths {
            match ops::trash_path_undoable(Path::new(path)) {
                Ok(item) => trashed.push(item),
                Err(error) => errors.push(format!("{path}: {error}")),
            }
        }
        if !trashed.is_empty() {
            // T054: one undo entry per delete gesture — restores from trash.
            cx.emit(PaneEvent::Undoable(UndoEntry::Trash { items: trashed }));
        }
        self.reload();
        if !errors.is_empty() {
            self.set_status(
                StatusLevel::Error,
                format!("Could not delete: {}", errors.join(", ")),
            );
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::new_explorer_for_tests;
    use super::clipboard;
    use gpui::TestAppContext;
    use std::fs;
    use tempfile::tempdir;

    #[gpui::test]
    async fn copy_then_paste_duplicates_the_file(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        fs::write(src_dir.path().join("a.txt"), "hello").unwrap();

        let window = new_explorer_for_tests(cx, src_dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();
        // Deliberate no-op: nothing is selected yet, so `copy_selection`'s
        // empty-selection guard returns without touching the clipboard.
        window
            .update(cx, |page, _window, cx| page.copy_selection(cx))
            .unwrap();
        // Select the one entry, copy, then point the pane at the destination.
        window
            .update(cx, |page, _window, cx| {
                page.selection.clear();
                page.selection.insert(0);
                page.copy_selection(cx);
                page.change_dir_for_test(dst_dir.path().to_string_lossy().to_string());
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();

        assert_eq!(
            fs::read_to_string(dst_dir.path().join("a.txt")).unwrap(),
            "hello"
        );
        assert!(src_dir.path().join("a.txt").exists(), "copy preserves source");
    }

    #[gpui::test]
    async fn cut_then_paste_moves_the_file(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        fs::write(src_dir.path().join("a.txt"), "hello").unwrap();

        let window = new_explorer_for_tests(cx, src_dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();
        window
            .update(cx, |page, _window, cx| {
                page.selection.clear();
                page.selection.insert(0);
                page.cut_selection(cx);
                page.change_dir_for_test(dst_dir.path().to_string_lossy().to_string());
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();

        assert!(!src_dir.path().join("a.txt").exists(), "cut removes source");
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("a.txt")).unwrap(),
            "hello"
        );
    }

    #[gpui::test]
    async fn paste_collision_opens_conflict_dialog_and_rename_keeps_both(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("file.txt");
        fs::write(&src, "new").unwrap();
        fs::write(dst_dir.path().join("file.txt"), "existing").unwrap();

        let window = new_explorer_for_tests(cx, dst_dir.path());
        window
            .update(cx, |_page, _window, cx| {
                clipboard::set_copy(vec![src.to_string_lossy().to_string()], cx);
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();

        // The collision pauses the paste behind the conflict dialog (T053)
        // instead of silently auto-renaming — nothing has transferred yet.
        assert!(
            window
                .update(cx, |page, _window, _cx| page.conflict_dialog.is_some())
                .unwrap(),
            "a colliding paste opens the conflict dialog"
        );
        assert!(src.exists(), "copy preserves source");
        assert!(!dst_dir.path().join("file (2).txt").exists());

        // The default decision (Rename) keeps both — the old auto-rename
        // outcome, now explicit.
        window
            .update(cx, |page, _window, cx| {
                page.conflict_decide(crate::explorer::conflict::ConflictChoice::Rename, cx);
            })
            .unwrap();

        assert!(src.exists(), "copy preserves source");
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("file.txt")).unwrap(),
            "existing"
        );
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("file (2).txt")).unwrap(),
            "new"
        );
    }

    #[gpui::test]
    async fn paste_collision_overwrite_replaces_the_destination(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("file.txt");
        fs::write(&src, "new").unwrap();
        fs::write(dst_dir.path().join("file.txt"), "existing").unwrap();

        let window = new_explorer_for_tests(cx, dst_dir.path());
        window
            .update(cx, |_page, _window, cx| {
                clipboard::set_copy(vec![src.to_string_lossy().to_string()], cx);
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();
        assert!(
            window
                .update(cx, |page, _window, _cx| page.conflict_dialog.is_some())
                .unwrap(),
            "a colliding paste opens the conflict dialog"
        );

        window
            .update(cx, |page, _window, cx| {
                page.conflict_decide(crate::explorer::conflict::ConflictChoice::Overwrite, cx);
            })
            .unwrap();

        assert_eq!(
            fs::read_to_string(dst_dir.path().join("file.txt")).unwrap(),
            "new",
            "Overwrite replaces the existing destination in place"
        );
        assert!(
            !dst_dir.path().join("file (2).txt").exists(),
            "Overwrite does not create a unique-name duplicate"
        );
    }

    #[gpui::test]
    async fn paste_collision_cancel_aborts_without_transferring(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("file.txt");
        fs::write(&src, "new").unwrap();
        fs::write(dst_dir.path().join("file.txt"), "existing").unwrap();

        let window = new_explorer_for_tests(cx, dst_dir.path());
        window
            .update(cx, |_page, _window, cx| {
                clipboard::set_copy(vec![src.to_string_lossy().to_string()], cx);
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();
        assert!(
            window
                .update(cx, |page, _window, _cx| page.conflict_dialog.is_some())
                .unwrap(),
            "a colliding paste opens the conflict dialog"
        );

        window
            .update(cx, |page, _window, cx| {
                page.conflict_decide(crate::explorer::conflict::ConflictChoice::Cancel, cx);
            })
            .unwrap();

        assert!(
            window
                .update(cx, |page, _window, _cx| page.conflict_dialog.is_none())
                .unwrap()
        );
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("file.txt")).unwrap(),
            "existing",
            "Cancel leaves the destination untouched"
        );
        assert!(!dst_dir.path().join("file (2).txt").exists());
        window
            .update(cx, |page, _window, _cx| {
                let (status, is_error) = page
                    .status_for_footer()
                    .expect("cancel reports a footer status");
                assert!(!is_error);
                assert!(status.contains("cancelled"), "{status}");
            })
            .unwrap();
    }

    #[gpui::test]
    async fn paste_collision_apply_to_all_skips_every_remaining_conflict(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let a = src_dir.path().join("a.txt");
        let b = src_dir.path().join("b.txt");
        fs::write(&a, "a-new").unwrap();
        fs::write(&b, "b-new").unwrap();
        fs::write(dst_dir.path().join("a.txt"), "a-old").unwrap();
        fs::write(dst_dir.path().join("b.txt"), "b-old").unwrap();

        let window = new_explorer_for_tests(cx, dst_dir.path());
        window
            .update(cx, |_page, _window, cx| {
                clipboard::set_copy(
                    vec![
                        a.to_string_lossy().to_string(),
                        b.to_string_lossy().to_string(),
                    ],
                    cx,
                );
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();
        assert!(
            window
                .update(cx, |page, _window, _cx| page.conflict_dialog.is_some())
                .unwrap(),
            "two colliding pastes open the conflict dialog"
        );

        // Apply-to-all on the first conflict (Skip) decides every remaining
        // conflict of the same operation silently (mockup §1.2).
        window
            .update(cx, |page, _window, cx| {
                page.toggle_conflict_apply_all(cx);
                page.conflict_decide(crate::explorer::conflict::ConflictChoice::Skip, cx);
            })
            .unwrap();

        assert_eq!(
            fs::read_to_string(dst_dir.path().join("a.txt")).unwrap(),
            "a-old",
            "Skip leaves the first destination untouched"
        );
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("b.txt")).unwrap(),
            "b-old",
            "apply-to-all skips the remaining conflict too"
        );
        assert!(!dst_dir.path().join("a (2).txt").exists());
        assert!(!dst_dir.path().join("b (2).txt").exists());
        assert!(a.exists() && b.exists(), "skipped sources are not consumed");
        assert!(
            window
                .update(cx, |page, _window, _cx| page.conflict_dialog.is_none())
                .unwrap(),
            "apply-to-all closes the dialog after the last conflict"
        );
    }

    #[gpui::test]
    async fn partial_cut_paste_reports_error_and_clears_clipboard(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let existing = src_dir.path().join("existing.txt");
        let missing = src_dir.path().join("missing.txt");
        fs::write(&existing, "payload").unwrap();

        let window = new_explorer_for_tests(cx, dst_dir.path());
        window
            .update(cx, |_page, _window, cx| {
                clipboard::set_cut(
                    vec![
                        existing.to_string_lossy().to_string(),
                        missing.to_string_lossy().to_string(),
                    ],
                    cx,
                );
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();

        assert!(!existing.exists(), "successful cut item is moved");
        assert!(dst_dir.path().join("existing.txt").exists());
        window
            .update(cx, |page, _window, cx| {
                let (status, is_error) = page
                    .status_for_footer()
                    .expect("partial paste reports a status");
                assert!(is_error);
                assert!(status.contains("1 succeeded, 1 failed"), "{status}");
                assert!(status.contains("missing.txt"), "{status}");
                let clip = clipboard::current(cx);
                assert!(clip.mode.is_none());
                assert!(clip.paths.is_empty());
            })
            .unwrap();
    }

    #[gpui::test]
    async fn new_folder_creates_a_uniquely_named_directory(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("New Folder")).unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        // Run the create + assertion + cancel in a single update closure:
        // painting a real `Input` (Task 6 inline rename) requires a
        // `gpui_component::Root` on the window, which the bare pane-rooted
        // test harness does not provide. A live rename left across a repaint
        // boundary panics in `Root::read`, so cancel before this update
        // returns (the rename tests follow the same pattern).
        window
            .update(cx, |page, window, cx| {
                page.new_folder(window, cx);
                assert!(page.renaming.is_some());
                page.cancel_rename(window, cx);
            })
            .unwrap();

        assert!(dir.path().join("New Folder (2)").is_dir());
    }

    #[gpui::test]
    async fn confirm_delete_with_empty_selection_is_a_noop(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        let window = new_explorer_for_tests(cx, dir.path());

        window
            .update(cx, |page, window, cx| {
                assert!(page.selected_paths().is_empty());
                page.confirm_delete_selection(window, cx);
            })
            .unwrap();
    }

    #[gpui::test]
    async fn delete_paths_reports_an_error_for_a_missing_path(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        let window = new_explorer_for_tests(cx, dir.path());

        window
            .update(cx, |page, _window, cx| {
                let missing = dir.path().join("does-not-exist.txt");
                page.delete_paths(vec![missing.to_string_lossy().to_string()], cx);
                assert!(page
                    .status_for_footer()
                    .is_some_and(|(_, is_error)| is_error));
            })
            .unwrap();
    }
}
