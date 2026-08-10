//! Right-click context menu (merged T007 "Open With" + b3/b4 file-ops).
//!
//! One overlay handles both targets:
//! - a file/directory row: Open / Open With (apps) / Rename / Copy / Cut /
//!   Copy Path / Delete / Properties
//! - empty listing space: New Folder / Paste / Refresh
//!
//! Renders as an absolutely-positioned overlay anchored to the click position.

use chronos_fm_services::mime::{open_with, set_default_app, DesktopApp};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;

use super::ExplorerPane;
use super::clipboard;

/// State for a context menu anchored at a right-click position.
#[derive(Clone)]
pub struct ContextMenuState {
    /// Position where the menu should appear (in window coordinates).
    pub position: gpui::Point<Pixels>,
    /// Path of the file the menu was opened for, or `None` for the empty-area
    /// (directory) menu.
    pub file_path: Option<String>,
    /// Row index of `file_path` into `filtered_entries`, for inline rename.
    pub index: Option<usize>,
    /// Detected MIME type of the file (empty for the directory menu).
    pub mime_type: String,
    /// Applications that claim to handle this MIME type.
    pub apps: Vec<DesktopApp>,
}

impl ContextMenuState {
    /// Create a context menu state for the given file path and its row index.
    /// Detects the MIME type and finds matching applications synchronously.
    pub fn for_file(file_path: &str, index: usize, position: gpui::Point<Pixels>) -> Self {
        let mime_type = chronos_fm_services::mime::detect_mime_type(file_path)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let apps = chronos_fm_services::mime::find_apps_for_mime(&mime_type);
        Self {
            position,
            file_path: Some(file_path.to_owned()),
            index: Some(index),
            mime_type,
            apps,
        }
    }

    /// Create a context menu state for right-clicking empty listing space.
    pub fn for_directory(position: gpui::Point<Pixels>) -> Self {
        Self {
            position,
            file_path: None,
            index: None,
            mime_type: String::new(),
            apps: Vec::new(),
        }
    }
}

/// Render the context menu overlay for either a file or the empty area.
pub fn render(
    state: &ContextMenuState,
    _window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement {
    let bg = theme::bg(cx);
    let border = theme::border(cx);

    let menu_width = px(260.0);
    let x = state.position.x;
    let y = state.position.y;

    let items: Vec<AnyElement> = if let Some(file_path) = &state.file_path {
        file_menu_items(state, file_path, cx)
    } else {
        directory_menu_items(cx)
    };

    div()
        .absolute()
        .ml(x)
        .mt(y)
        .w(menu_width)
        .bg(bg)
        .border_1()
        .border_color(border)
        .rounded(px(8.0))
        .shadow_lg()
        .flex()
        .flex_col()
        .py(px(4.0))
        .children(items)
}

/// Builds the per-file menu items (T007 open actions + b3/b4 file-ops).
fn file_menu_items(
    state: &ContextMenuState,
    file_path: &str,
    cx: &mut Context<ExplorerPane>,
) -> Vec<AnyElement> {
    let fg = theme::fg(cx);
    let hover_bg = theme::bg_hover(cx);
    let muted = theme::muted(cx);
    let accent = theme::accent(cx);
    let border = theme::border(cx);
    let row_height = px(30.0);

    let mut items: Vec<AnyElement> = Vec::new();

    // "Open" item — launches the system default handler.
    let open_path = file_path.to_string();
    items.push(
        menu_row("Open", fg, hover_bg, row_height)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    this.open_with_default(&open_path);
                    this.close_context_menu(cx);
                }),
            )
            .into_any_element(),
    );

    items.push(separator(border).into_any_element());

    // "Open With" header
    items.push(
        div()
            .h(row_height)
            .px(px(12.0))
            .flex()
            .items_center()
            .text_xs()
            .text_color(muted)
            .child("Open With")
            .into_any_element(),
    );

    // Application entries
    for app in &state.apps {
        let app = app.clone();
        let app_for_default = app.clone();
        let name = app.name.clone();
        let file_path = file_path.to_string();
        let mime_type = state.mime_type.clone();
        items.push(
            div()
                .h(row_height)
                .pl(px(28.0))
                .pr(px(8.0))
                .flex()
                .items_center()
                .gap_2()
                .text_sm()
                .text_color(fg)
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg))
                .child(div().flex_1().overflow_hidden().text_ellipsis().child(name))
                // "Set as default" — registers this app as the default handler
                // for the file's MIME type.
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .cursor_pointer()
                        .hover(move |s| s.text_color(accent))
                        .child("Set as default")
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _event, _window, cx| {
                                if let Err(error) = set_default_app(&mime_type, &app_for_default)
                                {
                                    tracing::error!(
                                        "Failed to set default for {}: {}",
                                        mime_type,
                                        error
                                    );
                                }
                                this.close_context_menu(cx);
                                cx.stop_propagation();
                            }),
                        ),
                )
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        if let Err(error) = open_with(&app, &file_path) {
                            tracing::error!(
                                "Failed to open {} with {}: {}",
                                file_path,
                                app.name,
                                error
                            );
                        }
                        this.close_context_menu(cx);
                    }),
                )
                .into_any_element(),
        );
    }

    items.push(separator(border).into_any_element());

    // Rename — a single selection renames inline; a multi-selection opens the
    // Batch Rename dialog (T006 Task 5). F2 uses the same path.
    let rename_index = state.index;
    items.push(
        menu_row("Rename", fg, hover_bg, row_height)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, window, cx| {
                    if let Some(ix) = rename_index {
                        this.rename_selection(ix, window, cx);
                    }
                    this.close_context_menu(cx);
                }),
            )
            .into_any_element(),
    );

    // Copy — operates on the whole selection.
    items.push(
        menu_row("Copy", fg, hover_bg, row_height)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    this.copy_selection(cx);
                    this.close_context_menu(cx);
                }),
            )
            .into_any_element(),
    );

    // Cut — operates on the whole selection.
    items.push(
        menu_row("Cut", fg, hover_bg, row_height)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    this.cut_selection(cx);
                    this.close_context_menu(cx);
                }),
            )
            .into_any_element(),
    );

    // Copy Path — copies the right-clicked file's absolute path.
    let copy_path = file_path.to_string();
    items.push(
        menu_row("Copy Path", fg, hover_bg, row_height)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_path.clone()));
                    this.close_context_menu(cx);
                }),
            )
            .into_any_element(),
    );

    items.push(separator(border).into_any_element());

    // Delete — confirm dialog, then trashes the whole selection.
    items.push(delete_row(cx, row_height));

    items.push(separator(border).into_any_element());

    // "Properties" item
    let props_path = file_path.to_string();
    items.push(
        menu_row("Properties", fg, hover_bg, row_height)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    this.show_properties_for(&props_path, cx);
                    this.close_context_menu(cx);
                }),
            )
            .into_any_element(),
    );

    items
}

/// Builds the empty-area menu items (New Folder / Paste / Refresh).
fn directory_menu_items(cx: &mut Context<ExplorerPane>) -> Vec<AnyElement> {
    let fg = theme::fg(cx);
    let hover_bg = theme::bg_hover(cx);
    let muted = theme::muted(cx);
    let row_height = px(30.0);
    let has_clipboard = clipboard::current(cx).mode.is_some();

    let mut items: Vec<AnyElement> = Vec::new();

    items.push(
        menu_row("New Folder", fg, hover_bg, row_height)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, window, cx| {
                    this.new_folder(window, cx);
                    this.close_context_menu(cx);
                }),
            )
            .into_any_element(),
    );

    if has_clipboard {
        items.push(
            menu_row("Paste", fg, hover_bg, row_height)
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        this.paste_clipboard(cx);
                        this.close_context_menu(cx);
                    }),
                )
                .into_any_element(),
        );
    } else {
        items.push(disabled_menu_row("Paste", muted, row_height).into_any_element());
    }

    items.push(
        menu_row("Refresh", fg, hover_bg, row_height)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    this.reload();
                    this.close_context_menu(cx);
                    cx.notify();
                }),
            )
            .into_any_element(),
    );

    items
}

/// The Delete menu row with its trash-confirmation alert dialog.
fn delete_row(cx: &mut Context<ExplorerPane>, row_height: Pixels) -> AnyElement {
    let fg = theme::fg(cx);
    let hover_bg = theme::bg_hover(cx);
    menu_row("Delete", fg, hover_bg, row_height)
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this, _event, window, cx| {
                this.confirm_delete_selection(window, cx);
                this.close_context_menu(cx);
            }),
        )
        .into_any_element()
}

fn separator(border: Hsla) -> impl IntoElement {
    div()
        .h(px(1.0))
        .mx(px(8.0))
        .my(px(4.0))
        .bg(border)
}

/// A full-width menu row with the standard height, padding, and hover style.
fn menu_row(label: &str, fg: Hsla, hover_bg: Hsla, row_height: Pixels) -> Div {
    let label = label.to_string();
    div()
        .h(row_height)
        .px(px(12.0))
        .flex()
        .items_center()
        .text_sm()
        .text_color(fg)
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .child(label)
}

/// A muted, non-interactive menu row for actions that are unavailable in the
/// current context (e.g. Paste while the file clipboard is empty).
fn disabled_menu_row(label: &str, muted: Hsla, row_height: Pixels) -> Div {
    let label = label.to_string();
    div()
        .h(row_height)
        .px(px(12.0))
        .flex()
        .items_center()
        .text_sm()
        .text_color(muted)
        .child(label)
}
