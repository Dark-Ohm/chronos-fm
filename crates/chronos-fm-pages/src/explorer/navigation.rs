use chronos_fm_core::config;
use chronos_fm_services::fs::listing::{FileEntryDto, ListParams, list_dir_sync};
use chronos_fm_services::fs::provider::FileSystemProvider;

use gpui::{AppContext, Context, Window};

use super::ExplorerPane;
use super::entries;
use super::types::{PaneEvent, StatusLevel};

impl ExplorerPane {
    /// Set the virtual filesystem provider (T011). When `Some`, all
    /// directory listing and file reading is dispatched through the
    /// provider instead of the local filesystem.
    pub fn set_provider(
        &mut self,
        provider: std::sync::Arc<dyn FileSystemProvider>,
    ) {
        self.provider = Some(provider);
    }

    pub(crate) fn ensure_loaded(&mut self) {
        if !self.loaded {
            self.reload();
        }
    }

    pub(crate) fn reload(&mut self) {
        // Mark as loaded regardless of outcome so an empty or unreadable
        // directory is not re-read on every subsequent render.
        self.loaded = true;
        let result = if let Some(provider) = &self.provider {
            provider.list_dir(&self.cwd, config::DIR_LISTING_LIMIT as usize, None)
        } else {
            list_dir_sync(ListParams {
                path: &self.cwd,
                limit: config::DIR_LISTING_LIMIT,
                cursor: None,
            })
        };
        match result {
            Ok(res) => {
                let mut e = res.entries;
                entries::sort_entries(&mut e, self.sort_key, self.sort_asc);
                self.entries = e;
                self.apply_filter();
                self.update_item_sizes();
                self.preview_text = None;
                self.preview_path = None;
                self.preview_editor = None;
                self.preview_image_path = None;
                self.preview_message = None;
                self.clear_status();
            }
            Err(e) => {
                tracing::error!("Failed to list directory '{}': {}", self.cwd, e);
                self.entries = Vec::new();
                self.filtered_entries = Vec::new();
                self.update_item_sizes();
                self.set_status(
                    StatusLevel::Error,
                    format!("Cannot open '{}': {}", self.cwd, e),
                );
            }
        }
    }

    pub(crate) fn change_dir(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        if path == self.cwd {
            return;
        }
        self.close_search(window, cx);
        self.push_history(path.clone());
        self.cwd = path;
        self.entries.clear();
        self.reload();
        cx.emit(PaneEvent::Navigated(self.cwd.clone()));
        cx.notify();
    }

    /// Test-only: sets `cwd` directly without navigation history, search
    /// reset, or emitting `PaneEvent::Navigated` — for tests that only need
    /// `paste_clipboard`/`new_folder` to act on a different directory.
    #[cfg(test)]
    pub(crate) fn change_dir_for_test(&mut self, path: String) {
        self.cwd = path;
    }

    // Records a forward navigation in the back/forward history, seeding the
    // starting directory on first use and dropping any forward entries.
    fn push_history(&mut self, path: String) {
        if self.history.is_empty() {
            self.history.push(self.cwd.clone());
            self.history_index = 0;
        }
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(path);
        self.history_index += 1;
    }

    /// Window-less navigation shared by [`navigate_to_path`] and
    /// [`navigate_to_synced`]: clears search state, records history, and
    /// reloads the listing. `emit` controls whether `PaneEvent::Navigated` is
    /// raised — the devices mount callback emits so synced panes follow and the
    /// session saves; mirrored sync navigation does not, to avoid driving the
    /// originating pane back into an update loop.
    fn navigate_without_window(&mut self, path: String, emit: bool, cx: &mut Context<Self>) {
        if path == self.cwd {
            return;
        }
        // Clear search state so the destination doesn't show a stale filter or
        // full-text results from the previous directory (mirrors the
        // `close_search` reset on `change_dir`, minus the window-bound editor
        // sync — these callers have no `Window`).
        self.search_visible = false;
        self.search_results = None;
        self.search_query.clear();
        self.push_history(path.clone());
        self.cwd = path;
        self.entries.clear();
        self.reload();
        if emit {
            cx.emit(PaneEvent::Navigated(self.cwd.clone()));
        }
        cx.notify();
    }

    /// Navigates to `path` from an async completion callback that has no
    /// `Window` (the Devices panel's mount-then-navigate, T008). Emits
    /// `PaneEvent::Navigated` so synced panes follow and the session saves.
    pub(crate) fn navigate_to_path(&mut self, path: String, cx: &mut Context<Self>) {
        self.navigate_without_window(path, true, cx);
    }

    /// Mirrors a path requested by a sibling pane while `synced_panes` is on
    /// (§3.2). Unlike [`navigate_to_path`], it does not re-emit a navigation
    /// event, so the originating pane is not driven back into an update loop.
    pub(crate) fn navigate_to_synced(&mut self, path: String, cx: &mut Context<Self>) {
        self.navigate_without_window(path, false, cx);
    }

    pub(crate) fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_index > 0 {
            self.history_index -= 1;
            if let Some(p) = self.history.get(self.history_index).cloned() {
                self.cwd = p;
                self.entries.clear();
                self.close_search(window, cx);
                self.reload();
                cx.emit(PaneEvent::Navigated(self.cwd.clone()));
                cx.notify();
            }
        }
    }

    pub(crate) fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            if let Some(p) = self.history.get(self.history_index).cloned() {
                self.cwd = p;
                self.entries.clear();
                self.close_search(window, cx);
                self.reload();
                cx.emit(PaneEvent::Navigated(self.cwd.clone()));
                cx.notify();
            }
        }
    }

    pub(crate) fn activate_entry(
        &mut self,
        item: FileEntryDto,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if item.kind == "dir" {
            self.change_dir(item.path, window, cx);
        } else {
            self.open_preview(item.path, window, cx);
        }
    }

    /// Open the Properties dialog for the currently selected entry.
    pub(crate) fn show_properties(&mut self, cx: &mut gpui::Context<Self>) {
        let selected = self.active_index
            .and_then(|idx| self.filtered_entries.get(idx).cloned());
        let Some(item) = selected else {
            return;
        };
        let item_clone = item.clone();
        let dialog = cx.new(|cx| super::properties::PropertiesDialog::new(item_clone, cx));
        self.properties_dialog = Some(dialog);
        cx.notify();
    }

    /// Open the Properties dialog for the entry at `path` (used by the context
    /// menu, which is built for the right-clicked file rather than the
    /// currently active row). No-op if the path is not in the current listing.
    pub(crate) fn show_properties_for(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let Some(item) = self.filtered_entries.iter().find(|e| e.path == path).cloned() else {
            return;
        };
        let dialog = cx.new(|cx| super::properties::PropertiesDialog::new(item, cx));
        self.properties_dialog = Some(dialog);
        cx.notify();
    }

    /// Close the Properties dialog.
    pub(crate) fn close_properties(&mut self, cx: &mut gpui::Context<Self>) {
        self.properties_dialog = None;
        cx.notify();
    }

    /// Open the context menu for a file row at the given click position.
    /// `index` is the row's position in `filtered_entries`, used by Rename.
    pub(crate) fn open_context_menu(
        &mut self,
        file_path: String,
        index: usize,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut gpui::Context<Self>,
    ) {
        let state = super::context_menu::ContextMenuState::for_file(&file_path, index, position);
        self.context_menu = Some(state);
        cx.notify();
    }

    /// Open the empty-area context menu (New Folder / Paste / Refresh) at the
    /// given click position.
    pub(crate) fn open_context_menu_for_directory(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut gpui::Context<Self>,
    ) {
        let state = super::context_menu::ContextMenuState::for_directory(position);
        self.context_menu = Some(state);
        cx.notify();
    }

    /// Close the context menu.
    pub(crate) fn close_context_menu(&mut self, cx: &mut gpui::Context<Self>) {
        self.context_menu = None;
        cx.notify();
    }

    /// Launch the default handler for a file (xdg-open equivalent).
    pub(crate) fn open_with_default(&mut self, file_path: &str) {
        if let Err(error) = chronos_fm_services::mime::open_default(file_path) {
            tracing::error!("Failed to open {}: {}", file_path, error);
        }
    }
}
