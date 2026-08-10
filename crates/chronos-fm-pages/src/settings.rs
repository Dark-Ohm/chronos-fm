//! The settings page (T040 Phase V): a mockup-parity category shell —
//! left nav + scrollable category body — wired to the `config.toml`
//! fields that actually take effect (theme/ui/explorer), with every
//! mockup category still present and honestly disabled where nothing
//! backs it yet (`docs/design/mockups/Chronos-File-Manager-Settings.dc.html`).
//!
//! Deliberate Phase V scope trims (documented, not silent):
//! - The mockup's 430px live-preview FM column is not built here — it
//!   would need its own mocked file-listing subsystem, which is exactly
//!   the "mega-backend in a V PR" the ticket says to avoid. Categories
//!   are individually honest instead (see below).
//! - Keybindings has no backing keymap registry in this codebase yet, so
//!   rather than fabricate per-row key chips (the mockup's own static
//!   demo data), this page shows one honest empty state.
//! - **S3** is kept as a ninth category, additional to the mockup's eight
//!   — it's real, already-wired functionality (T011); Phase V's own rule
//!   is "wire already-real data, do not delete IA", and deleting a
//!   working feature to match a mockup that predates it would violate
//!   that rule in spirit even though the mockup literally has no S3 row.

use chronos_fm_core::config::patch::{ConfigField, default_config_path, patch_config_file};
use chronos_fm_core::config::{Config, SortOrder, SplitDirection, ThemeMode};
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, MouseButton, Render, Window, div, prelude::*, px,
};
use gpui_component::input::{Input, InputState};
use gpui_component::switch::Switch;
use gpui_component::{Icon, Sizable};

/// The nine sidebar categories (mockup's eight + S3, see module doc).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum SettingsCategory {
    #[default]
    Files,
    Appearance,
    Preview,
    Behavior,
    Terminal,
    Plugins,
    Keybindings,
    S3,
    About,
}

impl SettingsCategory {
    const ALL: [SettingsCategory; 9] = [
        SettingsCategory::Files,
        SettingsCategory::Appearance,
        SettingsCategory::Preview,
        SettingsCategory::Behavior,
        SettingsCategory::Terminal,
        SettingsCategory::Plugins,
        SettingsCategory::Keybindings,
        SettingsCategory::S3,
        SettingsCategory::About,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsCategory::Files => "Files & Folders",
            SettingsCategory::Appearance => "Appearance",
            SettingsCategory::Preview => "Preview Panel",
            SettingsCategory::Behavior => "Behavior",
            SettingsCategory::Terminal => "Terminal",
            SettingsCategory::Plugins => "Plugins",
            SettingsCategory::Keybindings => "Keybindings",
            SettingsCategory::S3 => "S3",
            SettingsCategory::About => "About",
        }
    }

    /// Under `crates/chronos-fm-ui/assets/icons/` only — never a mockup
    /// SVG path (Phase V rule #4).
    fn icon_path(self) -> &'static str {
        match self {
            SettingsCategory::Files => "icons/folder.svg",
            SettingsCategory::Appearance => "icons/palette.svg",
            SettingsCategory::Preview => "icons/panel-bottom-open.svg",
            SettingsCategory::Behavior => "icons/settings.svg",
            SettingsCategory::Terminal => "icons/square-terminal.svg",
            SettingsCategory::Plugins => "icons/puzzle.svg",
            SettingsCategory::Keybindings => "icons/replace.svg",
            SettingsCategory::S3 => "icons/cloud.svg",
            SettingsCategory::About => "icons/info.svg",
        }
    }

    /// Parses a category name from `--page=settings:<name>` (T046
    /// residual, case-insensitive). Short keys, not the full display
    /// labels (`"preview"`, not `"Preview Panel"`) — matches the other
    /// three pages' sub-view CLI vocabulary.
    fn from_cli_name(name: &str) -> Option<SettingsCategory> {
        match name.trim().to_lowercase().as_str() {
            "files" => Some(SettingsCategory::Files),
            "appearance" => Some(SettingsCategory::Appearance),
            "preview" => Some(SettingsCategory::Preview),
            "behavior" => Some(SettingsCategory::Behavior),
            "terminal" => Some(SettingsCategory::Terminal),
            "plugins" => Some(SettingsCategory::Plugins),
            "keybindings" => Some(SettingsCategory::Keybindings),
            "s3" => Some(SettingsCategory::S3),
            "about" => Some(SettingsCategory::About),
            _ => None,
        }
    }
}

/// The settings page.
pub struct SettingsPage {
    config: Config,
    category: SettingsCategory,
    search_input: Entity<InputState>,
}

impl SettingsPage {
    /// Creates a new settings page seeded with the app's current config.
    pub fn new(config: Config, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Search settings…", window, cx);
            state
        });
        Self {
            config,
            category: SettingsCategory::default(),
            search_input,
        }
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

    fn select_category(&mut self, category: SettingsCategory, cx: &mut Context<Self>) {
        self.category = category;
        cx.notify();
    }

    /// `--page=settings:<name>` (T046 residual): select a category by CLI
    /// name at startup. Silently warns and leaves the default category on
    /// an unrecognized name — never aborts the launch over a typo.
    pub(crate) fn set_initial_subview(&mut self, name: &str, cx: &mut Context<Self>) {
        match SettingsCategory::from_cli_name(name) {
            Some(category) => self.select_category(category, cx),
            None => tracing::warn!("ignoring unrecognized settings category {name:?}"),
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::bg(cx))
            .child(header(self, cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .child(sidebar(self, cx))
                    .child(main_panel(self, cx)),
            )
    }
}

/// Titlebar: page name + search box (filters the sidebar's category list,
/// pure client-side state — no backend needed, matches the mockup's own
/// live-filter behavior).
fn header(page: &SettingsPage, cx: &mut Context<SettingsPage>) -> impl IntoElement {
    div()
        .px(px(16.0))
        .py(px(12.0))
        .flex()
        .items_center()
        .gap(px(12.))
        .border_b_1()
        .border_color(theme::border(cx))
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(theme::fg(cx))
                .child("⚙️ Settings"),
        )
        .child(div().flex_1())
        .child(
            div()
                .w(px(220.))
                .child(Input::new(&page.search_input)),
        )
}

fn sidebar(page: &SettingsPage, cx: &mut Context<SettingsPage>) -> impl IntoElement {
    let query = page.search_input.read(cx).text().to_string().to_lowercase();
    let active = page.category;
    div()
        .w(px(236.))
        .flex_none()
        .h_full()
        .bg(theme::bg_secondary(cx))
        .border_r_1()
        .border_color(theme::border(cx))
        .flex()
        .flex_col()
        .py(px(8.))
        .px(px(8.))
        .gap(px(2.))
        .children(SettingsCategory::ALL.into_iter().filter_map(|cat| {
            if !query.is_empty() && !cat.label().to_lowercase().contains(&query) {
                return None;
            }
            let is_active = cat == active;
            Some(
                div()
                    .id(cat.label())
                    .cursor_pointer()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(34.))
                    .px(px(10.))
                    .rounded(px(7.))
                    .when(is_active, |d| {
                        d.bg(theme::bg_hover(cx)).text_color(theme::fg(cx))
                    })
                    .when(!is_active, |d| {
                        d.text_color(theme::fg_secondary(cx))
                            .hover(|s| s.bg(theme::bg_hover(cx)))
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _ev, _window, cx| {
                            this.select_category(cat, cx);
                        }),
                    )
                    .when(is_active, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left(px(-8.))
                                .top(px(7.))
                                .bottom(px(7.))
                                .w(px(3.))
                                .rounded(px(2.))
                                .bg(theme::accent(cx)),
                        )
                    })
                    .child(
                        Icon::new(Icon::empty())
                            .path(cat.icon_path())
                            .with_size(px(16.))
                            .text_color(if is_active { theme::accent(cx) } else { theme::muted(cx) }),
                    )
                    .child(div().text_sm().flex_1().child(cat.label())),
            )
        }))
}

fn main_panel(page: &mut SettingsPage, cx: &mut Context<SettingsPage>) -> impl IntoElement {
    let category = page.category;
    div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(46.))
                .flex_none()
                .flex()
                .items_center()
                .px(px(16.))
                .border_b_1()
                .border_color(theme::border(cx))
                .text_color(theme::fg(cx))
                .font_weight(FontWeight::BOLD)
                .child(category.label()),
        )
        .child(
            div()
                .flex_1()
                .min_h(px(0.))
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(category_body(page, category, cx)),
        )
}

fn category_body(page: &SettingsPage, category: SettingsCategory, cx: &mut Context<SettingsPage>) -> AnyElement {
    match category {
        SettingsCategory::Files => files_category(&page.config, cx),
        SettingsCategory::Appearance => appearance_category(&page.config, cx),
        SettingsCategory::Preview => preview_category(cx),
        SettingsCategory::Behavior => behavior_category(&page.config, cx),
        SettingsCategory::Terminal => terminal_category(cx),
        SettingsCategory::Plugins => plugins_category(cx),
        SettingsCategory::Keybindings => keybindings_category(cx),
        SettingsCategory::S3 => s3_section(&page.config, cx).into_any_element(),
        SettingsCategory::About => about_category(cx),
    }
}

// ---- row chrome ----

/// One settings row: label + description on the left, a control on the
/// right — the mockup's `.set-row` density.
fn settings_row(cx: &App, label: &str, desc: &str, control: AnyElement) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.))
        .py(px(11.))
        .px(px(4.))
        .border_b_1()
        .border_color(theme::border(cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(div().text_sm().text_color(theme::fg(cx)).child(label.to_string()))
                .child(
                    div()
                        .text_xs()
                        .font_family("monospace")
                        .text_color(theme::muted(cx))
                        .child(desc.to_string()),
                ),
        )
        .child(control)
        .into_any_element()
}

/// A row for a mockup field this build doesn't back yet — same label/desc
/// density as `settings_row`, but the control slot is an honest "not
/// wired" pill instead of a functional (or worse, a silently-fake)
/// control. Never deleted from the category, per Phase V rule #3.
fn unwired_row(cx: &App, label: &str, desc: &str) -> AnyElement {
    settings_row(
        cx,
        label,
        desc,
        div()
            .px(px(8.))
            .py(px(3.))
            .rounded(px(999.))
            .text_xs()
            .bg(theme::bg_secondary(cx))
            .text_color(theme::disabled(cx))
            .child("not wired yet")
            .into_any_element(),
    )
}

fn card(cx: &App) -> gpui::Div {
    elevated_card(cx)
}

// ---- Files & Folders ----

fn files_category(config: &Config, cx: &mut Context<SettingsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Files & Folders", "listing behavior"))
        .child(settings_row(
            cx,
            "Show hidden files",
            "Dotfiles — .config, .git, .cache",
            Switch::new("show-hidden")
                .checked(config.ui.show_hidden)
                .on_click(|checked, _window, _cx| {
                    SettingsPage::write_field(ConfigField::UiShowHidden(*checked));
                })
                .into_any_element(),
        ))
        .child(settings_row(
            cx,
            "Default sort column",
            "Column used on folder open",
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
                .child(sort_button("Kind", SortOrder::Kind, config.ui.default_sort, cx))
                .into_any_element(),
        ))
        .child(unwired_row(cx, "Show system & OS files", "Cross-hatched /proc /sys /dev entries"))
        .child(unwired_row(cx, "Directories first", "Sort folders above files"))
        .child(unwired_row(cx, "Case-sensitive sort", "A sorts before a"))
        .child(unwired_row(cx, "Single-click to open", "Replace double-click default"))
        .child(unwired_row(cx, "Confirm before delete", "Safety gate on Shift+Delete"))
        .child(unwired_row(cx, "Confirm before move to trash", "Prompt when trashing files"))
        .child(unwired_row(cx, "File size units", "Binary (KiB) vs decimal (KB)"))
        .into_any_element()
}

// ---- Appearance ----

fn appearance_category(config: &Config, cx: &mut Context<SettingsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Appearance", "theme"))
        .child(settings_row(
            cx,
            "Theme mode",
            "System / Light / Dark",
            div()
                .flex()
                .gap(px(8.))
                .child(mode_button("System", ThemeMode::System, config.theme.mode == ThemeMode::System, cx))
                .child(mode_button("Light", ThemeMode::Light, config.theme.mode == ThemeMode::Light, cx))
                .child(mode_button("Dark", ThemeMode::Dark, config.theme.mode == ThemeMode::Dark, cx))
                .into_any_element(),
        ))
        .child(settings_row(
            cx,
            "Accent",
            "FM accent color role",
            div()
                .flex()
                .gap(px(8.))
                .children(
                    chronos_fm_core::config::ACCENT_PALETTE
                        .iter()
                        .map(|color| accent_swatch(color, &config.theme.accent, cx)),
                )
                .into_any_element(),
        ))
        .child(settings_row(
            cx,
            "Icon pack",
            "Glyph set for file/folder icons",
            div()
                .flex()
                .gap(px(8.))
                .child(icon_pack_button("Default", "default", &config.ui.icon_pack, cx))
                .child(icon_pack_button("Nerd Font", "nerd", &config.ui.icon_pack, cx))
                .into_any_element(),
        ))
        .child(unwired_row(cx, "Grid view", "Icon grid instead of list"))
        .child(unwired_row(cx, "Show thumbnails", "Render image previews in grid"))
        .child(unwired_row(cx, "Icon size", "List & grid glyph px"))
        .child(unwired_row(cx, "Row height", "List row px"))
        .child(unwired_row(cx, "Date format", "Relative vs absolute mtime"))
        .child(unwired_row(cx, "Show preview panel", "Inspector beside the file list"))
        .child(unwired_row(cx, "Preview side", "Inspector dock position"))
        .into_any_element()
}

fn icon_pack_button(label: &'static str, value: &'static str, current: &str, cx: &App) -> impl IntoElement {
    let active = current == value;
    div()
        .id(label)
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |d| d.bg(theme::accent(cx)).text_color(theme::bg(cx)))
        .when(!active, |d| d.text_color(theme::fg_secondary(cx)))
        .on_click(move |_event, _window, _cx| {
            SettingsPage::write_field(ConfigField::UiIconPack(value.to_string()));
        })
        .child(label)
}

// ---- Preview Panel (fully unwired) ----

fn preview_category(cx: &mut Context<SettingsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Preview Panel", "inspector columns"))
        .child(unwired_row(cx, "Show file size", "Size column in list view"))
        .child(unwired_row(cx, "Show modified time", "mtime column"))
        .child(unwired_row(cx, "Show permissions", "Unix mode string (rwx)"))
        .child(unwired_row(cx, "Show owner", "user:group column"))
        .child(unwired_row(cx, "Image thumbnails", "Render PNG/JPG in preview"))
        .child(unwired_row(cx, "Syntax highlight", "Tokenized-color text previews"))
        .into_any_element()
}

// ---- Behavior (mixed: real Explorer pane settings + mockup's unwired rows) ----

fn behavior_category(config: &Config, cx: &mut Context<SettingsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Behavior", "split view"))
        .child(settings_row(
            cx,
            "New split orientation",
            "Layout for a newly opened split pane",
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
                ))
                .into_any_element(),
        ))
        .child(settings_row(
            cx,
            "Synced panes",
            "Both split panes navigate together",
            Switch::new("synced-panes")
                .checked(config.explorer.synced_panes)
                .on_click(|checked, _window, _cx| {
                    SettingsPage::write_field(ConfigField::ExplorerSyncedPanes(*checked));
                })
                .into_any_element(),
        ))
        .child(settings_row(
            cx,
            "Restore tabs on restart",
            "Reopen the last session's tabs",
            Switch::new("restore-tabs")
                .checked(config.explorer.restore_tabs)
                .on_click(|checked, _window, _cx| {
                    SettingsPage::write_field(ConfigField::ExplorerRestoreTabs(*checked));
                })
                .into_any_element(),
        ))
        .child(unwired_row(cx, "Default open handler", "Program for unknown types"))
        .child(unwired_row(cx, "Use trash (not rm)", "Delete → trash by default"))
        .child(unwired_row(cx, "Drag & drop", "Reorder / copy via pointer"))
        .child(unwired_row(cx, "Paste overwrite", "Conflict resolution policy"))
        .child(unwired_row(cx, "Select on focus", "Highlight name on row focus"))
        .child(unwired_row(cx, "Focus new tab", "Jump to new tab on open"))
        .into_any_element()
}

// ---- Terminal / Plugins (fully unwired — no backend exists yet) ----

fn terminal_category(cx: &mut Context<SettingsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Terminal", "spawned by the FM"))
        .child(unwired_row(cx, "Terminal emulator", "Spawned by the FM"))
        .child(unwired_row(cx, "Default shell", "Login shell inside terminal"))
        .child(unwired_row(cx, "Open at current dir", "Spawn in active folder"))
        .child(unwired_row(cx, "Split on launch", "Open terminal as a split pane"))
        .into_any_element()
}

fn plugins_category(cx: &mut Context<SettingsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Plugins", "Luau runtime"))
        .child(unwired_row(cx, "Luau hot-reload", "Reload plugins without restart"))
        .child(unwired_row(cx, "Luau sandbox", "Isolate plugin runtime"))
        .child(unwired_row(cx, "Filesystem watch", "Live refresh on external change"))
        .child(unwired_row(cx, "Image generation", "Generate wallpapers via ChronOS media"))
        .child(unwired_row(cx, "OCR on images", "Extract text from screenshots"))
        .child(unwired_row(cx, "Cloud mount", "Mount remote buckets inline"))
        .into_any_element()
}

// ---- Keybindings (honest empty state — no keymap registry exists) ----

fn keybindings_category(cx: &mut Context<SettingsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Keybindings", "not yet customizable"))
        .child(
            div()
                .py(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme::muted(cx))
                .text_sm()
                .child("Keybindings aren't backed by a live keymap registry yet — nothing to show here honestly."),
        )
        .into_any_element()
}

// ---- About ----

fn about_category(cx: &mut Context<SettingsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "About", "chronos-fm"))
        .child(settings_row(
            cx,
            "ChronOS File Manager",
            "GPUI-native desktop file browser",
            div()
                .text_xs()
                .font_family("monospace")
                .text_color(theme::muted(cx))
                .child(format!("v{}", env!("CARGO_PKG_VERSION")))
                .into_any_element(),
        ))
        .into_any_element()
}

// ---- shared control widgets ----

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

/// The S3 category — already-real profile display (T011), kept verbatim
/// from the pre-T040 page (see module doc: kept as a documented ninth
/// category rather than deleted to match the mockup's eight).
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
                .mt(px(8.0))
                .text_color(theme::muted(cx))
                .text_sm()
                .child("No profiles configured. Add them in config.toml:")
                .child(
                    div()
                        .mt(px(4.0))
                        .px(px(8.0))
                        .py(px(4.0))
                        .rounded(px(4.0))
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
        .mt(px(8.0))
        .px(px(12.0))
        .py(px(8.0))
        .rounded(px(6.0))
        .bg(theme::bg_secondary(cx))
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme::fg(cx))
                .mb(px(4.0))
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
        .py(px(2.0))
        .child(div().text_color(theme::muted(cx)).text_xs().child(label))
        .child(
            div()
                .text_color(theme::fg(cx))
                .text_xs()
                .font_family("monospace")
                .child(value),
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
            |window, cx| {
                let page = cx.new(|cx| {
                    super::SettingsPage::new(chronos_fm_core::config::Config::default(), window, cx)
                });
                page.into_any_element()
            },
        );
    }

    #[test]
    fn every_category_has_a_label_and_icon() {
        // T040: the sidebar must never render a blank row — every variant
        // needs a real label and a real (non-empty) icon path.
        for cat in super::SettingsCategory::ALL {
            assert!(!cat.label().is_empty());
            assert!(cat.icon_path().starts_with("icons/"));
        }
    }

    #[test]
    fn all_nine_categories_are_distinct() {
        let mut labels: Vec<&str> = super::SettingsCategory::ALL.iter().map(|c| c.label()).collect();
        let before = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), before, "no two categories share a label");
    }
}
