//! Extensions page (T041 Phase V pixel-copy of
//! `docs/design/mockups/Chronos-Extensions-Tab.dc.html`).
//!
//! No plugin execution yet — the plugin host
//! (`chronos-fm-plugin-host`, wasmtime + WIT) lands in P4; this page
//! manages what `config.toml` already lists and is honest everywhere
//! else the mockup shows Marketplace/permissions/host data this codebase
//! doesn't have.
//!
//! Deliberate Phase V scope trims (documented, not silent):
//! - **No detail pane.** The mockup's 380px right column shows per-plugin
//!   version/description/readme/permission-grants. `config.plugins` is
//!   just `Vec<String>` ids for `core`/`community` (checked
//!   `chronos-fm-core::config`) — none of that metadata exists. Building
//!   a detail pane would mean fabricating it, the exact thing rule #2
//!   forbids ("no fake history/transfers/progress" generalizes to no fake
//!   plugin metadata).
//! - **Available is one honest empty state, always.** No marketplace
//!   backend exists (P5) — this mirrors the mockup's own `marketOnline:
//!   false` path (`Marketplace offline`) rather than the `true` path with
//!   its six hardcoded demo cards.
//! - **Permissions has no per-permission grant toggles.** No grant model
//!   exists in `config.toml` yet (checked — no `plugins.grants` field).
//!   Lists real community plugin ids with one honest policy line instead
//!   of the mockup's five-column allow/deny switch grid.

use chronos_fm_core::config::Config;
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::{
    Action, AnyElement, App, Context, Entity, FocusHandle, Focusable, FontWeight, MouseButton,
    Render, Window, div, prelude::*, px,
};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};
use gpui_component::{Disableable, Icon, Sizable};

use crate::s3::NavigateToSettings;

/// The mockup's four sub-nav views.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum ExtensionsView {
    #[default]
    Installed,
    Available,
    Permissions,
    Host,
}

impl ExtensionsView {
    const ALL: [ExtensionsView; 4] = [
        ExtensionsView::Installed,
        ExtensionsView::Available,
        ExtensionsView::Permissions,
        ExtensionsView::Host,
    ];

    fn label(self) -> &'static str {
        match self {
            ExtensionsView::Installed => "Installed",
            ExtensionsView::Available => "Available",
            ExtensionsView::Permissions => "Permissions",
            ExtensionsView::Host => "Host / Runtime",
        }
    }

    /// Under `crates/chronos-fm-ui/assets/icons/` only (rule #4).
    fn icon_path(self) -> &'static str {
        match self {
            ExtensionsView::Installed => "icons/puzzle.svg",
            ExtensionsView::Available => "icons/download.svg",
            ExtensionsView::Permissions => "icons/database.svg",
            ExtensionsView::Host => "icons/square-terminal.svg",
        }
    }

    /// Parses a sub-view name from `--page=extensions:<name>` (T046
    /// residual, case-insensitive). Mirrors `PageKind::from_cli_name`.
    fn from_cli_name(name: &str) -> Option<ExtensionsView> {
        match name.trim().to_lowercase().as_str() {
            "installed" => Some(ExtensionsView::Installed),
            "available" => Some(ExtensionsView::Available),
            "permissions" => Some(ExtensionsView::Permissions),
            "host" | "runtime" => Some(ExtensionsView::Host),
            _ => None,
        }
    }
}

pub struct ExtensionsPage {
    config: Config,
    focus_handle: FocusHandle,
    view: ExtensionsView,
    search_input: Entity<InputState>,
}

impl Focusable for ExtensionsPage {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ExtensionsPage {
    pub fn new(config: Config, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Search plugins…", window, cx);
            state
        });
        Self {
            config,
            focus_handle: cx.focus_handle(),
            view: ExtensionsView::default(),
            search_input,
        }
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    fn select_view(&mut self, view: ExtensionsView, cx: &mut Context<Self>) {
        self.view = view;
        cx.notify();
    }

    /// `--page=extensions:<name>` (T046 residual): select a sub-view by
    /// CLI name at startup. Silently warns and leaves the default view on
    /// an unrecognized name — never aborts the launch over a typo.
    pub(crate) fn set_initial_subview(&mut self, name: &str, cx: &mut Context<Self>) {
        match ExtensionsView::from_cli_name(name) {
            Some(view) => self.select_view(view, cx),
            None => tracing::warn!("ignoring unrecognized extensions sub-view {name:?}"),
        }
    }
}

impl Render for ExtensionsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::bg(cx))
            .track_focus(&self.focus_handle)
            .child(header(self, cx))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .child(sub_nav(self, cx))
                    .child(main_panel(self, cx)),
            )
    }
}

/// Titlebar: page name + search (client-side filter over installed
/// plugin ids — real, needs no backend) + an honest host-status pill
/// (mirrors the mockup's `tb-toggle`, always "pre-P4" — never faked on).
fn header(page: &ExtensionsPage, cx: &mut Context<ExtensionsPage>) -> impl IntoElement {
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
                .child("🧩 Extensions"),
        )
        .child(div().flex_1())
        .child(
            div()
                .w(px(220.))
                .child(Input::new(&page.search_input)),
        )
        .child(
            div()
                .px(px(9.))
                .py(px(4.))
                .rounded(px(7.))
                .border_1()
                .border_color(theme::border(cx))
                .text_xs()
                .font_family("monospace")
                .text_color(theme::muted(cx))
                .child("pre-P4"),
        )
}

fn sub_nav(page: &ExtensionsPage, cx: &mut Context<ExtensionsPage>) -> impl IntoElement {
    let active = page.view;
    let installed_count = page.config.plugins.core.len() + page.config.plugins.community.len();
    let community_count = page.config.plugins.community.len();
    div()
        .w(px(190.))
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
        .children(ExtensionsView::ALL.into_iter().map(|view| {
            let is_active = view == active;
            let count = match view {
                ExtensionsView::Installed => installed_count,
                ExtensionsView::Available => 0,
                ExtensionsView::Permissions => community_count,
                ExtensionsView::Host => 0,
            };
            div()
                .id(view.label())
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
                        this.select_view(view, cx);
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
                        .path(view.icon_path())
                        .with_size(px(16.))
                        .text_color(if is_active { theme::accent(cx) } else { theme::muted(cx) }),
                )
                .child(div().text_sm().flex_1().child(view.label()))
                .when(count > 0, |d| {
                    d.child(
                        div()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(theme::muted(cx))
                            .child(count.to_string()),
                    )
                })
        }))
}

fn main_panel(page: &mut ExtensionsPage, cx: &mut Context<ExtensionsPage>) -> impl IntoElement {
    let view = page.view;
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
                .child(view.label()),
        )
        .child(
            div()
                .flex_1()
                .min_h(px(0.))
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(view_body(page, view, cx)),
        )
}

fn view_body(page: &ExtensionsPage, view: ExtensionsView, cx: &mut Context<ExtensionsPage>) -> AnyElement {
    match view {
        ExtensionsView::Installed => installed_view(page, cx),
        ExtensionsView::Available => available_view(cx),
        ExtensionsView::Permissions => permissions_view(page, cx),
        ExtensionsView::Host => host_view(cx),
    }
}

fn empty_state(cx: &App, title: &str, body: &str) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .py(px(40.))
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::BOLD)
                .text_color(theme::fg_secondary(cx))
                .child(title.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme::muted(cx))
                .max_w(px(320.))
                .child(body.to_string()),
        )
        .into_any_element()
}

// ---- Installed ----

fn installed_view(page: &ExtensionsPage, cx: &mut Context<ExtensionsPage>) -> AnyElement {
    let plugins = &page.config.plugins;
    let query = page.search_input.read(cx).text().to_string().to_lowercase();

    let core: Vec<&String> = plugins
        .core
        .iter()
        .filter(|id| query.is_empty() || id.to_lowercase().contains(&query))
        .collect();
    let community: Vec<&String> = plugins
        .community
        .iter()
        .filter(|id| query.is_empty() || id.to_lowercase().contains(&query))
        .collect();

    if core.is_empty() && community.is_empty() {
        return card(cx)
            .child(section_header(cx, "Installed", "config.toml [plugins]"))
            .child(if plugins.core.is_empty() && plugins.community.is_empty() {
                empty_state(
                    cx,
                    "No plugins configured",
                    "Add them in config.toml — [plugins]\ncore = [\"git\", \"calculator\"]\ncommunity = [\"user/repo\"]",
                )
            } else {
                empty_state(cx, "No matches", "No installed plugin id matches your search.")
            })
            .into_any_element();
    }

    card(cx)
        .child(section_header(cx, "Installed", "config.toml [plugins]"))
        .children(core.iter().map(|id| installed_row(id, "core", cx)))
        .children(community.iter().map(|id| installed_row(id, "community", cx)))
        .child(
            div()
                .mt(px(4.))
                .child(
                    Button::new("ext-install")
                        .label("Install…")
                        .disabled(true), // needs the plugin host (P4) — never a fake success
                ),
        )
        .into_any_element()
}

fn installed_row(id: &str, kind: &'static str, cx: &App) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .py(px(10.))
        .border_b_1()
        .border_color(theme::border(cx))
        .child(
            div()
                .text_base()
                .text_color(theme::accent(cx))
                .child(format!("🔌 {id}")),
        )
        .child(div().flex_1())
        .child(
            div()
                .text_xs()
                .px(px(6.))
                .py(px(2.))
                .rounded(px(4.))
                .bg(theme::bg_secondary(cx))
                .text_color(theme::fg_secondary(cx))
                .child(kind),
        )
        .child(
            div()
                .text_xs()
                .px(px(6.))
                .py(px(2.))
                .rounded(px(4.))
                .bg(theme::bg_secondary(cx))
                .text_color(theme::muted(cx))
                .child("needs host · P4"),
        )
        .into_any_element()
}

// ---- Available (no marketplace backend — always the honest offline path) ----

fn available_view(cx: &mut Context<ExtensionsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Available", "marketplace (P5)"))
        .child(empty_state(
            cx,
            "Marketplace not available yet",
            "Plugin discovery/browse (P5) hasn't landed — this isn't a network error, there is no catalog to reach yet. Your installed plugins above are unaffected.",
        ))
        .into_any_element()
}

// ---- Permissions ----

fn permissions_view(page: &ExtensionsPage, cx: &mut Context<ExtensionsPage>) -> AnyElement {
    let community = &page.config.plugins.community;
    if community.is_empty() {
        return card(cx)
            .child(section_header(cx, "Permissions", "community plugin grants"))
            .child(empty_state(
                cx,
                "No community plugins",
                "Only core plugins are installed — they run native, outside the grant model.",
            ))
            .into_any_element();
    }
    card(cx)
        .child(section_header(cx, "Permissions", "community plugin grants"))
        .children(community.iter().map(|id| {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .py(px(10.))
                .border_b_1()
                .border_color(theme::border(cx))
                .child(div().text_color(theme::fg(cx)).child(format!("🔌 {id}")))
                .child(div().flex_1())
                .child(
                    div()
                        .text_xs()
                        .font_family("monospace")
                        .text_color(theme::muted(cx))
                        .child("deny-by-default · not yet enforced"),
                )
        }))
        .child(
            div()
                .mt(px(8.))
                .text_xs()
                .text_color(theme::muted(cx))
                .child("No per-permission grant model exists in config.toml yet — this policy is stated intent, not an enforced runtime control."),
        )
        .into_any_element()
}

// ---- Host / Runtime ----

fn host_view(cx: &mut Context<ExtensionsPage>) -> AnyElement {
    card(cx)
        .child(section_header(cx, "Host / Runtime", "pre-P4"))
        .child(
            div()
                .flex()
                .gap(px(11.))
                .p(px(12.))
                .rounded(px(9.))
                .border_1()
                .border_color(theme::accent(cx))
                .bg(theme::bg_secondary(cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .text_color(theme::fg(cx))
                                .child("Plugin host not running — no fake status"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::fg_secondary(cx))
                                .child(
                                    "Planned architecture: wasmtime + WIT components, isolated \
                                     runtime, per-plugin grants (P4). Marketplace browse (P5) \
                                     and install land after the host. No wasmtime dependency is \
                                     linked into this build yet.",
                                ),
                        ),
                ),
        )
        .child(
            div()
                .mt(px(4.))
                .text_base()
                .text_color(theme::fg_secondary(cx))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child("Core plugins — Rust native, bundled with chronos-fm, run outside the sandbox")
                .child("Community plugins — WASM/Luau, user-granted permissions per plugin")
                .child("The [plugins] section in config.toml is already parsed and hot-reloaded."),
        )
        .child(
            div().mt(px(8.)).child(
                Button::new("ext-open-settings")
                    .label("Open Settings → Plugins")
                    .on_click(move |_event, window, cx| {
                        window.dispatch_action(NavigateToSettings.boxed_clone(), cx);
                    }),
            ),
        )
        .into_any_element()
}

fn card(cx: &App) -> gpui::Div {
    elevated_card(cx).p(px(20.0))
}

impl crate::Page for ExtensionsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, IntoElement, TestAppContext, point, px, size};

    #[gpui::test]
    async fn extensions_page_renders_without_panicking(cx: &mut TestAppContext) {
        cx.update(|cx| gpui_component::init(cx));
        let cx = cx.add_empty_window();
        cx.draw(
            point(px(0.0), px(0.0)),
            size(px(900.0), px(700.0)),
            |window, cx| {
                let page = cx.new(|cx| {
                    super::ExtensionsPage::new(chronos_fm_core::config::Config::default(), window, cx)
                });
                page.into_any_element()
            },
        );
    }

    #[test]
    fn every_view_has_a_label_and_icon() {
        for view in super::ExtensionsView::ALL {
            assert!(!view.label().is_empty());
            assert!(view.icon_path().starts_with("icons/"));
        }
    }

    #[test]
    fn all_four_views_are_distinct() {
        let mut labels: Vec<&str> = super::ExtensionsView::ALL.iter().map(|v| v.label()).collect();
        let before = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), before, "no two views share a label");
    }
}
