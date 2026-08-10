//! Inline rename: turning a row's filename into an editable text field,
//! committing the new name via `chronos_fm_services::fs::ops::rename_in_place`,
//! or canceling without changing anything.

use gpui::{AppContext, Context, Window};
use gpui_component::input::InputState;

use super::ExplorerPane;
use super::types::StatusLevel;

impl ExplorerPane {
    /// Starts renaming the row at `ix`, pre-filling an input with its current
    /// name and focusing it. Does nothing if `ix` is out of range.
    pub(crate) fn begin_rename(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        // Guard against replacing an in-flight rename: dropping the old
        // `InputState` would lose the typed name without committing or
        // canceling it. The UI only calls this from a non-renaming state.
        if self.renaming.is_some() {
            return;
        }
        let Some(entry) = self.filtered_entries.get(ix) else {
            return;
        };
        let name = entry.name.clone();
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(name, window, cx);
            state.focus(window, cx);
            state
        });
        self.renaming = Some((ix, input));
        cx.notify();
    }

    /// Renames the current selection using the same single-versus-batch rule
    /// for both F2 and the context menu.
    pub(crate) fn rename_selection(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected = self.filtered_entries_for_selection();
        if selected.is_empty() {
            return;
        } else if selected.len() > 1 {
            self.open_batch_rename(selected, window, cx);
        } else {
            self.begin_rename(index, window, cx);
        }
    }

    /// Cancels an in-progress rename without changing anything on disk.
    pub(crate) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming = None;
        cx.notify();
    }

    /// Commits an in-progress rename: reads the input's text and renames the
    /// file on disk. A blank or unchanged name is treated as a no-op cancel,
    /// not an error.
    pub(crate) fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some((ix, input)) = self.renaming.take() else {
            return;
        };
        let Some(entry) = self.filtered_entries.get(ix).cloned() else {
            cx.notify();
            return;
        };
        let new_name = input.read(cx).text().to_string();
        if new_name.is_empty() || new_name == entry.name {
            cx.notify();
            return;
        }
        let src = std::path::Path::new(&entry.path);
        let result = chronos_fm_services::fs::ops::rename_in_place(src, &new_name);
        self.reload();
        if let Err(error) = result {
            self.set_status(StatusLevel::Error, format!("Rename failed: {error}"));
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    // Test fixtures write files directly; the synchronous-fs ban targets app code.
    #![allow(clippy::disallowed_methods)]

    use super::super::tests::new_explorer_for_tests;
    use gpui::TestAppContext;
    use std::fs;
    use tempfile::tempdir;

    #[gpui::test]
    async fn begin_rename_then_commit_renames_file_on_disk(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("old.txt"), "x").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| {
                assert_eq!(page.filtered_entries.len(), 1);
                page.begin_rename(0, window, cx);
                let (_, input) = page.renaming.clone().unwrap();
                input.update(cx, |state, cx| {
                    state.set_value("new.txt".to_string(), window, cx)
                });
                page.commit_rename(cx);
            })
            .unwrap();

        assert!(!dir.path().join("old.txt").exists());
        assert!(dir.path().join("new.txt").exists());
        window
            .read_with(cx, |page, _cx| assert!(page.renaming.is_none()))
            .unwrap();
    }

    #[gpui::test]
    async fn rename_selection_single_starts_inline_rename(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("old.txt"), "x").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| {
                page.select_single(0);
                page.rename_selection(0, window, cx);
                assert!(page.renaming.is_some());
                assert!(page.batch_rename.is_none());
                page.cancel_rename(cx);
            })
            .unwrap();
    }

    #[gpui::test]
    async fn rename_selection_without_selection_is_a_noop(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("old.txt"), "x").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| {
                page.rename_selection(0, window, cx);
                assert!(page.renaming.is_none());
                assert!(page.batch_rename.is_none());
            })
            .unwrap();
    }

    #[gpui::test]
    async fn rename_selection_multi_opens_batch_dialog(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "x").unwrap();
        fs::write(dir.path().join("b.txt"), "y").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| {
                page.select_single(0);
                page.select_range_to(1);
                page.rename_selection(0, window, cx);
                assert!(page.batch_rename.is_some());
                assert!(page.renaming.is_none());
                page.close_batch_rename(cx);
            })
            .unwrap();
    }

    #[gpui::test]
    async fn cancel_rename_clears_state_without_touching_disk(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("old.txt"), "x").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| {
                page.begin_rename(0, window, cx);
                page.cancel_rename(cx);
                assert!(page.renaming.is_none());
            })
            .unwrap();

        assert!(dir.path().join("old.txt").exists());
    }
}
