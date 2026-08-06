//! The settings page: a live editor for the `config.toml` fields that
//! actually take effect (theme/ui/explorer), plus disabled previews of
//! the draft sections that don't yet (keybindings/plugins/indexing/
//! search/launcher — spec's explicit "show, don't hide" decision).

use chronos_fm_core::config::patch::{ConfigField, default_config_path, patch_config_file};
use chronos_fm_core::config::{Config, SortOrder, ThemeMode};
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

fn explorer_section(config: &Config, cx: &gpui::App) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(cx, "Explorer", "split view"))
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
