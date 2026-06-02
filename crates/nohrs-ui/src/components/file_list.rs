use crate::theme::theme;
use gpui::{div, px, rgb, ParentElement, Styled, Window};
use gpui_component::list::{List, ListDelegate, ListItem};
use gpui_component::{Icon, IconName, IndexPath};
use nohrs_models::file_entry::FileEntryDto;
use std::sync::Arc;

pub type ConfirmCallback = Arc<dyn Fn(&FileEntryDto) + 'static>;

pub struct FileListDelegate {
    pub items: Vec<FileEntryDto>,
    pub selected: Option<IndexPath>,
    // Callback hooks
    pub on_confirm: Option<ConfirmCallback>,
}

impl Default for FileListDelegate {
    fn default() -> Self {
        Self::new()
    }
}

impl FileListDelegate {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: None,
            on_confirm: None,
        }
    }

    pub fn set_items(&mut self, items: Vec<FileEntryDto>) {
        self.items = items;
        self.selected = None;
    }

    pub fn get_selected(&self) -> Option<&FileEntryDto> {
        self.selected.and_then(|ix| self.items.get(ix.row))
    }
}

impl ListDelegate for FileListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &gpui::App) -> usize {
        self.items.len()
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut gpui::Context<List<Self>>,
    ) -> Option<Self::Item> {
        let item = self.items.get(ix.row)?;

        // Icon based on file type
        let icon_name = match item.kind.as_str() {
            "dir" => IconName::Folder,
            _ => IconName::File,
        };

        // Alternate row background for zebra striping
        let bg_color = if ix.row % 2 == 0 {
            theme::BG
        } else {
            theme::GRAY_50
        };

        let file_type = get_file_type(&item.name, &item.kind);

        let mut row = ListItem::new(ix)
            .py(px(6.0)) // Reduced from 12.0 for compact rows
            .px(px(24.0))
            .bg(rgb(bg_color))
            .child(
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .child(
                        // Name column with icon - flexible, takes remaining space
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .flex_1()
                            .min_w(px(150.0))
                            .child(
                                Icon::new(icon_name)
                                    .size_4()
                                    .text_color(rgb(theme::GRAY_600)),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(theme::FG))
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(item.name.clone()),
                            ),
                    )
                    .child(
                        // Type column - compact
                        div()
                            .w(px(70.0))
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(rgb(theme::FG_SECONDARY))
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(file_type),
                    )
                    .child(
                        // Size column - compact
                        div()
                            .w(px(70.0))
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(rgb(theme::FG_SECONDARY))
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(match item.kind.as_str() {
                                "file" => human_bytes(item.size),
                                "dir" => "-".to_string(),
                                other => other.to_string(),
                            }),
                    )
                    .child(
                        // Modified column - compact
                        div()
                            .w(px(90.0))
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(rgb(theme::FG_SECONDARY))
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(format_date(&item.modified)),
                    )
                    .child(
                        // Actions column - compact
                        div()
                            .w(px(40.0))
                            .flex_shrink_0()
                            .flex()
                            .justify_end()
                            .child(
                                Icon::new(IconName::File)
                                    .size_4()
                                    .text_color(rgb(theme::MUTED))
                                    .cursor_pointer(),
                            ),
                    ),
            );

        // enable click to confirm
        if let Some(on_confirm) = self.on_confirm.as_ref() {
            let on_confirm = Arc::clone(on_confirm);
            let item_clone = item.clone();
            row = row.on_click(move |_, _, _| {
                on_confirm(&item_clone);
            });
        }
        Some(row)
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut gpui::Context<List<Self>>,
    ) {
        self.selected = ix;
    }

    fn confirm(
        &mut self,
        _secondary: bool,
        _window: &mut Window,
        _cx: &mut gpui::Context<List<Self>>,
    ) {
        if let Some(ix) = self.selected {
            if let Some(item) = self.items.get(ix.row) {
                if let Some(cb) = &self.on_confirm {
                    cb(item);
                }
            }
        }
    }
}

pub fn human_bytes(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;
    let s = size as f64;
    if s >= GB {
        format!("{:.1} GB", s / GB)
    } else if s >= MB {
        format!("{:.1} MB", s / MB)
    } else if s >= KB {
        format!("{:.1} KB", s / KB)
    } else {
        format!("{} B", size)
    }
}

pub fn format_date(timestamp: &u64) -> String {
    use time::macros::format_description;
    use time::OffsetDateTime;

    let format = format_description!("[year]/[month]/[day]");
    match OffsetDateTime::from_unix_timestamp(*timestamp as i64) {
        Ok(datetime) => datetime.format(&format).unwrap_or_else(|_| "-".to_string()),
        Err(_) => "-".to_string(),
    }
}

pub fn get_file_type(name: &str, kind: &str) -> String {
    match kind {
        "dir" => "Folder".to_string(),
        "file" => {
            if let Some(ext) = std::path::Path::new(name)
                .extension()
                .and_then(|e| e.to_str())
            {
                ext.to_uppercase()
            } else {
                "File".to_string()
            }
        }
        "symlink" => "Link".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{format_date, get_file_type, human_bytes};
    use proptest::prelude::*;

    #[test]
    fn human_bytes_picks_the_expected_unit() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn get_file_type_maps_kinds_and_extensions() {
        assert_eq!(get_file_type("photo.png", "file"), "PNG");
        assert_eq!(get_file_type("Makefile", "file"), "File");
        assert_eq!(get_file_type("src", "dir"), "Folder");
        assert_eq!(get_file_type("link", "symlink"), "Link");
    }

    #[test]
    fn format_date_is_stable_for_the_epoch() {
        assert_eq!(format_date(&0), "1970/01/01");
    }

    proptest! {
        // A representative property: the size formatter must never panic and
        // must always emit a non-empty string for any `u64` input.
        #[test]
        fn human_bytes_never_panics(size in any::<u64>()) {
            let rendered = human_bytes(size);
            prop_assert!(!rendered.is_empty());
        }
    }
}
