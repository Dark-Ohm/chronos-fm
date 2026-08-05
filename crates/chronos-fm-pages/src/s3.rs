use gpui::{AnyElement, Context, Render, Window, div, prelude::*, px};
use chronos_fm_ui::theme::theme;

/// The S3 page, a placeholder for future object-storage browsing.
pub struct S3Page;

impl Default for S3Page {
    fn default() -> Self {
        Self::new()
    }
}

impl S3Page {
    /// Creates a new S3 page.
    pub fn new() -> Self {
        Self
    }
}

impl Render for S3Page {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .bg(theme::bg(cx))
            .child(
                div()
                    .text_2xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme::fg(cx))
                    .child("☁️ S3"),
            )
            .child(
                div()
                    .mt(px(16.0))
                    .text_base()
                    .text_color(theme::fg_secondary(cx))
                    .child("S3 compatible storage integration to be implemented"),
            )
    }
}

impl crate::Page for S3Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}
