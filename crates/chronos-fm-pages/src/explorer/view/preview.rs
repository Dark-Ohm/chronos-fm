use crate::explorer::ExplorerPane;
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;

/// Renders the preview pane for the selected file, showing a text editor,
/// image, status message, or an empty placeholder.
pub fn render(page: &mut ExplorerPane, _window: &mut Window, cx: &App) -> impl IntoElement + use<> {
    let title = page
        .preview_path
        .as_ref()
        .map(|p| path_name(p))
        .unwrap_or_else(|| "Preview".to_string());

    let content = if let Some(editor) = &page.preview_editor {
        div()
            .flex_1()
            .min_h(px(0.))
            .child(editor.clone())
            .into_any_element()
    } else if let Some(image) = page.preview_image_data.clone() {
        image_frame(
            cx,
            img(image)
                .max_w_full()
                .max_h_full()
                .object_fit(ObjectFit::Contain),
        )
    } else if let Some(image_path) = &page.preview_image_path {
        // Pass PathBuf, not String: String maps to Embedded assets, not disk files.
        let path = std::path::PathBuf::from(image_path);
        image_frame(
            cx,
            img(path)
                .max_w_full()
                .max_h_full()
                .object_fit(ObjectFit::Contain),
        )
    } else if let Some(msg) = &page.preview_message {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .px(px(16.))
            .text_color(theme::muted(cx))
            .child(msg.clone())
            .into_any_element()
    } else {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme::muted(cx))
            .child("No file selected")
            .into_any_element()
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(theme::bg(cx))
        .child(
            div()
                .px(px(16.0))
                .py(px(12.0))
                .border_b_1()
                .border_color(theme::border(cx))
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::fg(cx))
                        .child(title),
                ),
        )
        .child(content)
}

fn image_frame(cx: &App, image: impl IntoElement) -> AnyElement {
    div()
        .flex_1()
        .min_h(px(0.))
        .flex()
        .items_center()
        .justify_center()
        .p(px(8.))
        .bg(theme::bg_secondary(cx))
        .child(image)
        .into_any_element()
}

fn path_name(p: &str) -> String {
    std::path::Path::new(p)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| p.to_string())
}

/// The text editor used to render file previews.
pub mod editor;
