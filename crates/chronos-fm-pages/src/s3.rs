//! S3 page — object-storage browser (T011).

use chronos_fm_core::config::Config;
use chronos_fm_services::s3;
use chronos_fm_ui::patterns::elevated_card;
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;

use crate::explorer::ExplorerPane;

pub struct S3Page {
    config: Config,
    focus_handle: FocusHandle,
}

impl Focusable for S3Page {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl S3Page {
    pub fn new(config: Config, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            config,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }
}

impl Render for S3Page {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_profiles = !self.config.s3.profiles.is_empty();
        let default_profile = self
            .config
            .s3
            .default_profile
            .clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::bg(cx))
            .track_focus(&self.focus_handle)
            .child(
                // Header
                div()
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(theme::border(cx))
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme::fg(cx))
                    .child("☁️ S3"),
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if !has_profiles {
                        elevated_card(cx)
                            .p(px(32.0))
                            .child("No S3 profiles configured")
                            .child(
                                div()
                                    .mt(px(12.0))
                                    .text_color(theme::fg_secondary(cx))
                                    .child(
                                        "Add a profile in Settings to connect \
                                         to your S3-compatible storage.",
                                    ),
                            )
                            .into_any_element()
                    } else if default_profile.is_empty() {
                        elevated_card(cx)
                            .p(px(32.0))
                            .child("S3 connection not configured")
                            .child(
                                div()
                                    .mt(px(12.0))
                                    .text_color(theme::fg_secondary(cx))
                                    .child(
                                        "Set up credentials in the Settings tab \
                                         under S3 section.",
                                    ),
                            )
                            .into_any_element()
                    } else {
                        let profile = self
                            .config
                            .s3
                            .profiles
                            .get(&default_profile);
                        let endpoint = profile
                            .map(|p| p.endpoint.as_str())
                            .unwrap_or("unknown");
                        elevated_card(cx)
                            .p(px(32.0))
                            .child(format!(
                                "Profile: {default_profile} ({endpoint})"
                            ))
                            .child(
                                div()
                                    .mt(px(12.0))
                                    .text_color(theme::fg_secondary(cx))
                                    .child(
                                        "Credentials needed — enter them in \
                                         Settings or configure via environment \
                                         variables (AWS_ACCESS_KEY_ID).",
                                    ),
                            )
                            .into_any_element()
                    }),
            )
    }
}

impl crate::Page for S3Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}
