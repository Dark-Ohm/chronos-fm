//! Extensions page — P4 landing page (T012).
//!
//! Shows the WASM plugin architecture, configured plugins from
//! `config.toml`, and the P4/P5 roadmap. No plugin execution yet —
//! the plugin host (`chronos-fm-plugin-host`) lands in P4.

use chronos_fm_core::config::Config;
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;

pub struct ExtensionsPage {
    config: Config,
    focus_handle: FocusHandle,
}

impl Focusable for ExtensionsPage {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ExtensionsPage {
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

impl Render for ExtensionsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let plugins = &self.config.plugins;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::bg(cx))
            .track_focus(&self.focus_handle)
            // Header
            .child(
                div()
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(theme::border(cx))
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme::fg(cx))
                    .child("🧩 Extensions"),
            )
            // Content
            .child(
                div()
                    .flex_1()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(20.0))
                    // Section 1: Architecture
                    .child(
                        elevated_card(cx).p(px(24.0)).child(
                            section_header(
                                cx,
                                "Plugin Architecture",
                                "How plugins work in chronos-fm",
                            ),
                        ).child(
                            div()
                                .mt(px(12.0))
                                .text_base()
                                .text_color(theme::fg_secondary(cx))
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child("WASM Component Model + wasmtime-wasi — secure, sandboxed execution")
                                .child("Core plugins — Rust native, bundled with chronos-fm")
                                .child("Community plugins — WASM, user-granted permissions per plugin")
                                .child(
                                    div()
                                        .mt(px(10.0))
                                        .text_sm()
                                        .text_color(theme::fg_secondary(cx))
                                        .child(
                                            "Docs: Plugin Overview · API Reference · Permissions",
                                        ),
                                ),
                        ),
                    )
                    // Section 2: Configured Plugins
                    .child(
                        elevated_card(cx).p(px(24.0)).child(
                            section_header(
                                cx,
                                "Your Plugins (config.toml)",
                                "Plugins listed in your configuration",
                            ),
                        ).child(if plugins.core.is_empty() && plugins.community.is_empty() {
                            div()
                                .mt(px(16.0))
                                .flex()
                                .flex_col()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .text_base()
                                        .text_color(theme::fg_secondary(cx))
                                        .child(
                                            "No plugins configured. Add them in config.toml:",
                                        ),
                                )
                                .child(
                                    div()
                                        .font_family("monospace")
                                        .bg(theme::bg_secondary(cx))
                                        .p(px(12.0))
                                        .rounded(px(6.0))
                                        .text_sm()
                                        .text_color(theme::fg(cx))
                                        .child(
                                            "[plugins]\ncore = [\"git\", \"calculator\"]\ncommunity = [\"user/repo\"]",
                                        ),
                                )
                                .into_any_element()
                        } else {
                            div()
                                .mt(px(16.0))
                                .flex()
                                .flex_col()
                                .gap(px(12.0))
                                .child(plugin_list("core", &plugins.core, cx))
                                .child(plugin_list("community", &plugins.community, cx))
                                .into_any_element()
                        }),
                    )
                    // Section 3: Roadmap
                    .child(
                        elevated_card(cx).p(px(24.0)).child(
                            section_header(cx, "Roadmap", "When can I use plugins?"),
                        ).child(
                            div()
                                .mt(px(12.0))
                                .text_base()
                                .text_color(theme::fg_secondary(cx))
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child("P4: Plugin host (wasmtime-wasi) — install, permissions, activation")
                                .child("P5: Plugin marketplace — browse, one-click install, Go templates")
                                .child(
                                    div()
                                        .mt(px(6.0))
                                        .text_sm()
                                        .text_color(theme::fg_secondary(cx))
                                        .child(
                                            "The [plugins] section in config.toml is already parsed and hot-reloaded.",
                                        ),
                                ),
                        ),
                    ),
            )
    }
}

/// Renders one list of plugins (core or community) with a sub-header
/// and per-plugin rows.
fn plugin_list(
    group: &str,
    plugins: &[String],
    cx: &mut App,
) -> impl IntoElement + use<> {
    if plugins.is_empty() {
        return div().into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::BOLD)
                .text_color(theme::fg(cx))
                .child(format!("{group}:")),
        )
        .children(plugins.iter().map(|id| {
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(
                    div()
                        .text_base()
                        .text_color(theme::accent(cx))
                        .child(format!("🔌 {id}")),
                )
                .child(
                    div()
                        .text_xs()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(theme::bg_secondary(cx))
                        .text_color(theme::fg_secondary(cx))
                        .child("P4"),
                )
        }))
        .into_any_element()
}

impl crate::Page for ExtensionsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}
