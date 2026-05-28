use crate::core::config;
use crate::services::fs::listing::{list_dir_sync, FileEntryDto, ListParams};

use gpui::{Context, Window};

use super::entries;
use super::types::StatusLevel;
use super::ExplorerPage;

impl ExplorerPage {
    pub(crate) fn ensure_loaded(&mut self) {
        if self.entries.is_empty() {
            self.reload();
        }
    }

    pub(crate) fn reload(&mut self) {
        match list_dir_sync(ListParams {
            path: &self.cwd,
            limit: config::DIR_LISTING_LIMIT,
            cursor: None,
        }) {
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
        if self.history.is_empty() {
            self.history.push(self.cwd.clone());
            self.history_index = 0;
        }
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(path.clone());
        self.history_index += 1;
        self.cwd = path;
        self.entries.clear();
        self.reload();
    }

    pub(crate) fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.history_index > 0 {
            self.history_index -= 1;
            if let Some(p) = self.history.get(self.history_index).cloned() {
                self.cwd = p;
                self.entries.clear();
                self.close_search(window, cx);
                self.reload();
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
}
