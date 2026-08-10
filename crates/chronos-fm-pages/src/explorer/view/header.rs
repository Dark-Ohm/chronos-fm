use super::super::types::ViewMode;
use crate::explorer::ExplorerPane;
use gpui::prelude::*;
use gpui::*;
use gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_component::list::ListItem;
use gpui_component::ActiveTheme;
use gpui_component::{Icon, IconName};
use chronos_fm_ui::theme::theme;

/// Renders the explorer header with navigation buttons, the breadcrumb path
/// bar, and view-mode controls.
pub fn render(
    page: &mut ExplorerPane,
    _window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    let parts = path_parts(&page.cwd);

    let (display_parts, is_truncated) = if parts.len() > 5 {
        (parts[(parts.len() - 5)..].to_vec(), true)
    } else {
        (parts.clone(), false)
    };

    let mut bc = Breadcrumb::new();

    if is_truncated {
        bc = bc.child(BreadcrumbItem::new("…").on_click(cx.listener(move |_this, _, _, _| {})));
    }

    let start_idx = if is_truncated { parts.len() - 5 } else { 0 };

    for (display_i, p) in display_parts.iter().enumerate() {
        let actual_i = start_idx + display_i;
        let text = if p.is_empty() {
            String::from("/")
        } else {
            p.clone()
        };

        let mut path_here = String::new();
        for (j, part) in parts.iter().enumerate() {
            if j == 0 {
                path_here = if part.is_empty() {
                    "/".to_string()
                } else {
                    part.clone()
                };
            } else {
                path_here.push(std::path::MAIN_SEPARATOR);
                path_here.push_str(part);
            }
            if j >= actual_i {
                break;
            }
        }
        if path_here.is_empty() {
            path_here = page.cwd.clone();
        }

        bc = bc.child(BreadcrumbItem::new(text).on_click(
            cx.listener(move |this, _, window, cx| this.change_dir(path_here.clone(), window, cx)),
        ));
    }

    let can_go_back = page.history_index > 0;
    let can_go_forward = page.history_index + 1 < page.history.len();

    // Store search_visible for use in search toggle style
    let search_visible = page.search_visible;
    let entry_count = page.filtered_entries.len();

    div()
        .bg(theme::toolbar_bg(cx))
        .border_b_1()
        .border_color(theme::border(cx))
        .flex()
        .items_center()
        .text_color(theme::fg(cx))
        .px(px(14.0))
        .py(px(6.0))
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .flex_shrink_0()
                .child(nav_button(
                    "nav-back",
                    "icons/chevron-left.svg",
                    can_go_back,
                    cx.listener(|view, _, window, cx| view.go_back(window, cx)),
                    cx,
                ))
                .child(nav_button(
                    "nav-forward",
                    "icons/chevron-right.svg",
                    can_go_forward,
                    cx.listener(|view, _, window, cx| view.go_forward(window, cx)),
                    cx,
                ))
                .child(
                    div()
                        .w(px(1.0))
                        .h(px(20.0))
                        .bg(theme::border(cx))
                        .mx(px(4.0)),
                ),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(Icon::empty())
                        .path("icons/folder.svg")
                        .w(px(12.0))
                        .h(px(12.0))
                        .flex_none()
                        .text_color(theme::muted(cx)),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .min_w(px(0.0))
                        .bg(theme::bg(cx))
                        .border_1()
                        .border_color(theme::border(cx))
                        .rounded(px(5.0))
                        .px(px(8.0))
                        .py(px(3.0))
                        .text_size(px(11.5))
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(theme::fg_secondary(cx))
                        .child(div().flex().items_center().child(bc)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .flex_shrink_0()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::fg_secondary(cx))
                        .whitespace_nowrap()
                        .child(format!("{} items", entry_count)),
                )
                .child(render_view_mode_toggle(page, cx))
                .child(
                    ListItem::new("search-toggle")
                        .px(px(8.0))
                        .py(px(6.0))
                        .rounded(px(6.0))
                        .on_click(cx.listener(|view, _, window, cx| {
                            view.toggle_search(window, cx);
                        }))
                        .child(Icon::new(IconName::Search).size_4().text_color(
                            if search_visible {
                                theme::accent(cx)
                            } else {
                                theme::gray_600(cx)
                            },
                        )),
                ),
        )
}

fn nav_button(
    id: &'static str,
    icon_path: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement {
    div()
        .id(id)
        .w(px(22.0))
        .h(px(22.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(theme::bg_hover(cx)))
                .on_click(on_click)
        })
        .child(
            Icon::new(Icon::empty())
                .path(icon_path)
                .w(px(13.0))
                .h(px(13.0))
                .text_color(if enabled {
                    theme::gray_600(cx)
                } else {
                    theme::disabled(cx)
                }),
        )
}

fn render_view_mode_toggle(
    page: &mut ExplorerPane,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(view_mode_button(
            page,
            ViewMode::List,
            "view-mode-list",
            IconName::PanelBottomOpen,
            "List",
            cx,
        ))
        .child(view_mode_button(
            page,
            ViewMode::Grid,
            "view-mode-grid",
            IconName::LayoutDashboard,
            "Grid",
            cx,
        ))
}

fn view_mode_button(
    page: &mut ExplorerPane,
    mode: ViewMode,
    id: &'static str,
    icon: IconName,
    label: &'static str,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    let is_active = page.view_mode == mode;
    ListItem::new(id)
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .when(is_active, |this| this.bg(theme::bg_hover(cx)))
        .on_click(cx.listener(move |this, _, _, cx| this.set_view_mode(mode, cx)))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(Icon::new(icon).size_4().text_color(if is_active {
                    theme::accent(cx)
                } else {
                    theme::gray_600(cx)
                }))
                .child(
                    div()
                        .text_xs()
                        .text_color(if is_active {
                            theme::fg(cx)
                        } else {
                            theme::fg_secondary(cx)
                        })
                        .child(label),
                ),
        )
}

fn path_parts(path: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for c in std::path::Path::new(path).components() {
        parts.push(c.as_os_str().to_string_lossy().to_string());
    }
    if parts.is_empty() {
        parts.push(path.to_string());
    }
    parts
}
