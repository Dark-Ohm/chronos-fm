//! The settings page: a live editor for the `config.toml` fields that
//! actually take effect (theme/ui/explorer), plus disabled previews of
//! the draft sections that don't yet (keybindings/plugins/indexing/
//! search/launcher — spec's explicit "show, don't hide" decision).

use chronos_fm_core::config::patch::{ConfigField, default_config_path, patch_config_file};
use chronos_fm_core::config::{Config, SortOrder, SplitDirection, ThemeMode};
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::{AnyElement, Context, Render, Window, div, prelude::*, px};
use gpui_component::switch::Switch;

/// The settings page: a live editor for the `config.toml` fields that
/// actually take effect (theme/ui/explorer), plus disabled previews of
/// the draft sections that don't yet (keybindings/plugins/indexing/
/// search/launcher — spec's explicit "show, don't hide" decision).
pub struct SettingsPage {
    config: Config,
}

impl SettingsPage {
    /// Creates a new settings page seeded with the app's current config.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Re-seed the page with a freshly-loaded config.
    ///
    /// Called by `RootView::apply_config` on every hot reload (including
    /// writes the page itself triggers), so the active mode/sort/switch
    /// states always reflect the on-disk `config.toml` rather than the
    /// snapshot taken at construction.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// Re-reads `config.toml`, patches one field, and writes it back.
    /// The app's existing config-watcher (`root.rs`) picks up the file
    /// change and re-applies it — this function does not mutate
    /// `self.config` directly; the next full config reload does that.
    fn write_field(field: ConfigField) {
        match default_config_path().and_then(|path| {
            patch_config_file(&path, &field)?;
            Ok(())
        }) {
            Ok(()) => {}
            Err(error) => {
                tracing::error!("Failed to write config.toml: {error}");
            }
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(24.))
            .bg(theme::bg(cx))
            .child(theme_section(&self.config, cx))
            .child(ui_section(&self.config, cx))
            .child(explorer_section(&self.config, cx))
            .child(s3_section(&self.config, cx))
            .child(draft_sections(cx))
    }
}

fn mode_button(
    label: &'static str,
    mode: ThemeMode,
    active: bool,
    cx: &gpui::App,
) -> impl IntoElement {
    div()
        .id(label)
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |d| d.bg(theme::accent(cx)).text_color(theme::bg(cx)))
        .when(!active, |d| d.text_color(theme::fg_secondary(cx)))
        .on_click(move |_event, _window, _cx| {
            SettingsPage::write_field(ConfigField::ThemeMode(mode));
        })
        .child(label)
}

/// The swatch fill for a palette colour. `ACCENT_PALETTE` packs `0xRRGGBB`,
/// so this must go through `gpui::rgb` (opaque 6-digit). `gpui::rgba` reads
/// `0xRRGGBBAA` — feeding it a 6-digit value shifts every channel left by a
/// byte (red always 0, alpha = former blue), which was the T018 swatch-colour
/// bug. Extracted so the regression test exercises the exact production
/// conversion.
fn accent_fill(color: &chronos_fm_core::config::AccentColor) -> gpui::Hsla {
    gpui::Hsla::from(gpui::rgb(color.rgb))
}

fn accent_swatch(
    color: &chronos_fm_core::config::AccentColor,
    current: &str,
    cx: &gpui::App,
) -> impl IntoElement {
    // `name` is `&'static str` (lives in ACCENT_PALETTE) — copy it out so
    // neither the element id nor the click closure borrows the parameter.
    let name = color.name;
    let active = current == name;
    let fill = accent_fill(color);
    // Semi-transparent white reads as a highlight on any accent fill.
    let hover_border = gpui::Hsla::from(gpui::rgba(0xffffff99));
    div()
        .id(name)
        .size(px(20.))
        .rounded(px(6.))
        .cursor_pointer()
        .bg(fill)
        // Same border width for both states so selecting a swatch doesn't
        // shift the row by 1px; only the border colour varies.
        .border_1()
        .border_color(if active { theme::fg(cx) } else { gpui::Hsla::transparent_black() })
        .hover(|style| style.border_color(hover_border))
        .on_click(move |_event, _window, _cx| {
            SettingsPage::write_field(ConfigField::ThemeAccent(name.to_string()));
        })
}

fn theme_section(config: &Config, cx: &gpui::App) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(cx, "Theme", "appearance"))
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child(mode_button(
                    "System",
                    ThemeMode::System,
                    config.theme.mode == ThemeMode::System,
                    cx,
                ))
                .child(mode_button(
                    "Light",
                    ThemeMode::Light,
                    config.theme.mode == ThemeMode::Light,
                    cx,
                ))
                .child(mode_button(
                    "Dark",
                    ThemeMode::Dark,
                    config.theme.mode == ThemeMode::Dark,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("Accent"))
                .child(
                    div().flex().gap(px(8.)).children(
                        chronos_fm_core::config::ACCENT_PALETTE
                            .iter()
                            .map(|color| accent_swatch(color, &config.theme.accent, cx)),
                    ),
                ),
        )
}

fn ui_section(config: &Config, cx: &gpui::App) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(cx, "UI", "listing behavior"))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("Show hidden files"))
                .child(
                    Switch::new("show-hidden")
                        .checked(config.ui.show_hidden)
                        .on_click(|checked, _window, _cx| {
                            SettingsPage::write_field(ConfigField::UiShowHidden(*checked));
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child(sort_button("Name", SortOrder::Name, config.ui.default_sort, cx))
                .child(sort_button(
                    "Modified",
                    SortOrder::Modified,
                    config.ui.default_sort,
                    cx,
                ))
                .child(sort_button("Size", SortOrder::Size, config.ui.default_sort, cx))
                .child(sort_button("Kind", SortOrder::Kind, config.ui.default_sort, cx)),
        )
}

fn sort_button(
    label: &'static str,
    sort: SortOrder,
    current: SortOrder,
    cx: &gpui::App,
) -> impl IntoElement {
    let active = sort == current;
    div()
        .id(label)
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |d| d.bg(theme::accent(cx)).text_color(theme::bg(cx)))
        .when(!active, |d| d.text_color(theme::fg_secondary(cx)))
        .on_click(move |_event, _window, _cx| {
            SettingsPage::write_field(ConfigField::UiDefaultSort(sort));
        })
        .child(label)
}

fn split_button(
    label: &'static str,
    direction: SplitDirection,
    current: SplitDirection,
    cx: &gpui::App,
) -> impl IntoElement {
    let active = direction == current;
    div()
        .id(label)
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |d| d.bg(theme::accent(cx)).text_color(theme::bg(cx)))
        .when(!active, |d| d.text_color(theme::fg_secondary(cx)))
        .on_click(move |_event, _window, _cx| {
            SettingsPage::write_field(ConfigField::ExplorerSplitDirection(direction));
        })
        .child(label)
}

fn explorer_section(config: &Config, cx: &gpui::App) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(cx, "Explorer", "split view"))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("New split orientation"))
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .child(split_button(
                            "Side by side",
                            SplitDirection::Vertical,
                            config.explorer.split_direction,
                            cx,
                        ))
                        .child(split_button(
                            "Stacked",
                            SplitDirection::Horizontal,
                            config.explorer.split_direction,
                            cx,
                        )),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("Synced panes"))
                .child(
                    Switch::new("synced-panes")
                        .checked(config.explorer.synced_panes)
                        .on_click(|checked, _window, _cx| {
                            SettingsPage::write_field(ConfigField::ExplorerSyncedPanes(*checked));
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("Restore tabs on restart"))
                .child(
                    Switch::new("restore-tabs")
                        .checked(config.explorer.restore_tabs)
                        .on_click(|checked, _window, _cx| {
                            SettingsPage::write_field(ConfigField::ExplorerRestoreTabs(*checked));
                        }),
                ),
        )
}

/// Draft sections not yet wired to any subsystem (P3/P4) — shown, not
/// hidden, per spec's decision. Visually muted, no interactive controls.
fn s3_section(config: &Config, cx: &gpui::App) -> impl IntoElement {
    let default_profile = if config.s3.default_profile.is_empty() {
        "none".to_string()
    } else {
        config.s3.default_profile.clone()
    };

    let mut card = elevated_card(cx)
        .child(section_header(cx, "S3", "object storage"))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("Default profile"))
                .child(
                    div()
                        .text_color(if config.s3.default_profile.is_empty() {
                            theme::muted(cx)
                        } else {
                            theme::fg(cx)
                        })
                        .child(default_profile),
                ),
        );

    if config.s3.profiles.is_empty() {
        card = card.child(
            div()
                .mt(px(8.))
                .text_color(theme::muted(cx))
                .text_sm()
                .child("No profiles configured. Add them in config.toml:")
                .child(
                    div()
                        .mt(px(4.))
                        .px(px(8.))
                        .py(px(4.))
                        .rounded(px(4.))
                        .bg(theme::bg_secondary(cx))
                        .font_family("monospace")
                        .text_xs()
                        .child("[s3.profiles.personal]\nendpoint = \"https://s3.amazonaws.com\"\nregion = \"us-east-1\"\nforce_path_style = false"),
                ),
        );
    } else {
        for (name, profile) in &config.s3.profiles {
            card = card.child(profile_card(name.clone(), profile, cx));
        }
    }

    card
}

fn profile_card(name: String, profile: &chronos_fm_core::config::S3Profile, cx: &gpui::App) -> impl IntoElement {
    div()
        .mt(px(8.))
        .px(px(12.))
        .py(px(8.))
        .rounded(px(6.))
        .bg(theme::bg_secondary(cx))
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme::fg(cx))
                .mb(px(4.))
                .child(name),
        )
        .child(field_row("Endpoint", profile.endpoint.clone(), cx))
        .child(field_row("Region", profile.region.clone(), cx))
        .child(field_row(
            "Force path style",
            if profile.force_path_style { "on".to_string() } else { "off".to_string() },
            cx,
        ))
}

fn field_row(label: &'static str, value: String, cx: &gpui::App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .py(px(2.))
        .child(div().text_color(theme::muted(cx)).text_xs().child(label))
        .child(
            div()
                .text_color(theme::fg(cx))
                .text_xs()
                .font_family("monospace")
                .child(value),
        )
}

fn draft_sections(cx: &gpui::App) -> impl IntoElement {
    div()
        .opacity(0.5)
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_color(theme::fg_secondary(cx))
                .text_sm()
                .child("Keybindings, Plugins, Indexing, Search, Launcher — coming in P3/P4"),
        )
}

impl crate::Page for SettingsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, IntoElement, TestAppContext, point, px, size};

    /// T018 regression: the swatch fill is `Hsla::from(gpui::rgb(rgb))`
    /// (6-digit `0xRRGGBB`). The pre-fix code used `gpui::rgba`, which reads
    /// `0xRRGGBBAA` and shifted every channel left by a byte — red always 0,
    /// alpha = former blue. Mirroring the production conversion (rgb hex →
    /// Hsla → Rgba), every channel must survive the round trip within float
    /// tolerance; the byte-shift bug violates that by at least ~5× the
    /// tolerance for the least-affected palette colour (teal's red channel).
    #[test]
    fn accent_swatch_colors_match_palette() {
        for color in chronos_fm_core::config::ACCENT_PALETTE {
            let rgba = gpui::Rgba::from(super::accent_fill(color));
            let expected = |shift: u32| ((color.rgb >> shift) & 0xff) as f32 / 255.0;
            let eps = 0.01;
            assert!(
                (rgba.r - expected(16)).abs() < eps,
                "{}: red channel {:.3} != {:.3}",
                color.name,
                rgba.r,
                expected(16)
            );
            assert!(
                (rgba.g - expected(8)).abs() < eps,
                "{}: green channel {:.3} != {:.3}",
                color.name,
                rgba.g,
                expected(8)
            );
            assert!(
                (rgba.b - expected(0)).abs() < eps,
                "{}: blue channel {:.3} != {:.3}",
                color.name,
                rgba.b,
                expected(0)
            );
            assert!(
                (rgba.a - 1.0).abs() < eps,
                "{}: swatch must be opaque, alpha {:.3}",
                color.name,
                rgba.a
            );
        }
    }

    #[gpui::test]
    async fn settings_page_renders_without_panicking(cx: &mut TestAppContext) {
        cx.update(|cx| gpui_component::init(cx));
        let cx = cx.add_empty_window();
        cx.draw(
            point(px(0.0), px(0.0)),
            size(px(600.0), px(900.0)),
            |_window, cx| {
                let page = cx
                    .new(|_cx| super::SettingsPage::new(chronos_fm_core::config::Config::default()));
                page.into_any_element()
            },
        );
    }
}
