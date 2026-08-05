use gpui::{AnyElement, Context, Render, Window, div, prelude::*, px};
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;

/// The settings page, a placeholder for future application configuration.
pub struct SettingsPage;

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPage {
    /// Creates a new settings page.
    pub fn new() -> Self {
        Self
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .bg(theme::bg(cx))
            .child(
                elevated_card(cx)
                    .w(px(420.))
                    .child(section_header(cx, "Settings", "application preferences"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme::fg(cx))
                                    .child("⚙️ Settings"),
                            )
                            .child(
                                div()
                                    .text_base()
                                    .text_color(theme::fg_secondary(cx))
                                    .child("Application settings to be implemented"),
                            ),
                    ),
            )
    }
}

impl crate::Page for SettingsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}
