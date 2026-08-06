//! Right-click context menu with "Open With" submenu (T007).
//!
//! Renders as an absolutely-positioned overlay anchored to the click position.

use chronos_fm_services::mime::{open_with, set_default_app, DesktopApp};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;

use super::ExplorerPane;

/// State for a context menu anchored at a right-click position.
#[derive(Clone)]
pub struct ContextMenuState {
    /// Position where the menu should appear (in window coordinates).
    pub position: gpui::Point<Pixels>,
    /// Path of the file that was right-clicked.
    pub file_path: String,
    /// Detected MIME type of the file.
    pub mime_type: String,
    /// Applications that claim to handle this MIME type.
    pub apps: Vec<DesktopApp>,
}

impl ContextMenuState {
    /// Create a new context menu state for the given file path.
    /// Detects the MIME type and finds matching applications synchronously.
    pub fn for_file(file_path: &str, position: gpui::Point<Pixels>) -> Self {
        let mime_type = chronos_fm_services::mime::detect_mime_type(file_path)
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let apps = chronos_fm_services::mime::find_apps_for_mime(&mime_type);
        Self {
            position,
            file_path: file_path.to_owned(),
            mime_type,
            apps,
        }
    }
}

/// Render the context menu overlay.
pub fn render(
    state: &ContextMenuState,
    _window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement {
    let bg = theme::bg(cx);
    let border = theme::border(cx);
    let fg = theme::fg(cx);
    let hover_bg = theme::bg_hover(cx);
    let muted = theme::muted(cx);
    let accent = theme::accent(cx);

    let menu_width = px(260.0);
    let row_height = px(30.0);
    let x = state.position.x;
    let y = state.position.y;
    let file_path = state.file_path.clone();
    let mime_type = state.mime_type.clone();

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
        // "Open" item — launches the system default handler
        .child({
            let open_path = file_path.clone();
            menu_row("Open", fg, hover_bg, row_height).on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    this.open_with_default(&open_path);
                    this.close_context_menu(cx);
                }),
            )
        })
        // Separator
        .child(separator(border))
        // "Open With" header
        .child(
            div()
                .h(row_height)
                .px(px(12.0))
                .flex()
                .items_center()
                .text_xs()
                .text_color(muted)
                .child("Open With"),
        )
        // Application entries
        .children(state.apps.iter().map(|app| {
            let app = app.clone();
            let app_for_default = app.clone();
            let name = app.name.clone();
            let file_path = file_path.clone();
            let mime_type = mime_type.clone();
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
                                if let Err(error) = set_default_app(&mime_type, &app_for_default) {
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
                            tracing::error!("Failed to open {} with {}: {}", file_path, app.name, error);
                        }
                        this.close_context_menu(cx);
                    }),
                )
        }))
        // Separator
        .child(separator(border))
        // "Properties" item
        .child({
            let props_path = file_path.clone();
            menu_row("Properties", fg, hover_bg, row_height).on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    this.show_properties_for(&props_path, cx);
                    this.close_context_menu(cx);
                }),
            )
        })
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
