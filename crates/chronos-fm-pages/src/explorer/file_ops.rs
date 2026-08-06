//! Filesystem-mutating pane operations (copy, cut, paste, new folder,
//! delete), each backed by `chronos_fm_services::fs::ops` and followed by a
//! `reload()` so the listing reflects the result.

use std::path::Path;

use gpui::{Context, Window};

use chronos_fm_services::fs::ops;

use super::ExplorerPane;
use super::clipboard::{self, ClipboardMode};
use super::types::StatusLevel;

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
    /// directory, resolving name collisions via `ops::unique_name`. A Cut
    /// clipboard is cleared after the paste completes (even partially).
    pub(crate) fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        let clip = clipboard::current(cx);
        let Some(mode) = clip.mode else {
            return;
        };
        let dst_dir = Path::new(&self.cwd).to_path_buf();
        let mut errors: Vec<String> = Vec::new();
        for src_path in &clip.paths {
            let src = Path::new(src_path);
            let Some(name) = src.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let name = ops::unique_name(&dst_dir, name);
            let dst = dst_dir.join(&name);
            let result = match mode {
                ClipboardMode::Copy => ops::copy_path(src, &dst),
                ClipboardMode::Cut => ops::move_path(src, &dst).map(|_| ()),
            };
            if let Err(error) = result {
                errors.push(format!("{name}: {error}"));
            }
        }
        if mode == ClipboardMode::Cut {
            clipboard::clear(cx);
        }
        self.reload();
        if !errors.is_empty() {
            self.set_status(
                StatusLevel::Error,
                format!("Paste failed for {}", errors.join(", ")),
            );
        }
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

    /// Moves each of `paths` to the OS trash. Errors for individual paths are
    /// collected and reported together rather than aborting the whole batch.
    pub(crate) fn delete_paths(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        let mut errors: Vec<String> = Vec::new();
        for path in &paths {
            if let Err(error) = ops::trash_path(Path::new(path)) {
                errors.push(format!("{path}: {error}"));
            }
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
                page.cancel_rename(cx);
            })
            .unwrap();

        assert!(dir.path().join("New Folder (2)").is_dir());
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
