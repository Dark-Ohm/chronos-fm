use crate::explorer::ExplorerPane;
use chronos_fm_ui::devices_store;
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::list::ListItem;
use gpui_component::{Icon, IconName};

/// Renders the explorer sidebar listing quick-access locations.
pub fn render(
    page: &mut ExplorerPane,
    _window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(theme::bg(cx))
        .py(px(16.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .px(px(8.0))
                .child(sidebar_item(IconName::Folder, "Home", true, cx))
                .child(sidebar_item(IconName::Star, "Favorites", false, cx))
                .child(sidebar_item(IconName::File, "Recent", false, cx))
                .child(sidebar_item(IconName::Folder, "Trash", false, cx)),
        )
        .child(
            elevated_card(cx)
                .mt(px(16.0))
                .child(section_header(cx, "Folders", "quick access"))
                .child(render_shortcuts(page, cx)),
        )
        .child(render_devices_section(cx))
}

fn sidebar_item(
    icon: IconName,
    label: &str,
    _active: bool,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    let label = label.to_string();
    div()
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .px(px(12.0))
        .py(px(8.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(|this| this.bg(theme::bg_hover(cx)))
        .child(Icon::new(icon).size_4().text_color(theme::gray_600(cx)))
        .child(div().text_sm().text_color(theme::fg(cx)).child(label))
}

fn render_shortcuts(
    _page: &mut ExplorerPane,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    let shortcuts = get_shortcuts();
    let mut shortcuts_el = div().flex().flex_col().gap_1().px(px(8.0));

    for (i, (label, path)) in shortcuts.into_iter().enumerate() {
        let p = path.clone();
        let icon = match label.as_str() {
            "Home" => IconName::Folder,
            "Desktop" => IconName::Folder,
            "Downloads" => IconName::Folder,
            "Documents" => IconName::Folder,
            "Pictures" => IconName::Folder,
            _ => IconName::Folder,
        };
        let label_str = label.clone();

        shortcuts_el = shortcuts_el.child(
            ListItem::new(("shortcut", i))
                .on_click(
                    cx.listener(move |this, _, window, cx| this.change_dir(p.clone(), window, cx)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(Icon::new(icon).size_4().text_color(theme::gray_600(cx)))
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme::fg(cx))
                                .child(label_str.clone()),
                        ),
                ),
        );
    }

    shortcuts_el
}

fn get_shortcuts() -> Vec<(String, String)> {
    let mut v = Vec::new();
    let home = std::env::var("HOME").ok();
    #[cfg(target_os = "windows")]
    let home = home.or_else(|| std::env::var("USERPROFILE").ok());
    if let Some(h) = home {
        let p = |s: &str| {
            std::path::Path::new(&h)
                .join(s)
                .to_string_lossy()
                .to_string()
        };
        v.push(("Home".into(), h.clone()));
        for (label, sub) in [
            ("Desktop", "Desktop"),
            ("Downloads", "Downloads"),
            ("Documents", "Documents"),
            ("Pictures", "Pictures"),
        ] {
            let path = p(sub);
            if std::path::Path::new(&path).exists() {
                v.push((label.into(), path));
            }
        }
    }
    v
}

/// Renders the "Devices" sidebar section: one row per removable/internal
/// volume from `DeviceStore` (Task 3), using the T002 elevated-card
/// pattern already applied to the "Folders" section above it. Empty when
/// `DeviceStore` has no devices (no removable media / udisks2
/// unavailable) — renders nothing rather than an empty card.
/// Also returns nothing if the `DeviceStore` global hasn't been registered
/// yet (e.g. in tests that don't initialise the full app startup path).
pub(crate) fn render_devices_section(cx: &App) -> impl IntoElement {
    let store = cx.try_global::<devices_store::DeviceStore>();
    let Some(store) = store else {
        return div().into_any_element();
    };
    if store.devices.is_empty() {
        return div().into_any_element();
    }

    let mut card = elevated_card(cx)
        .mt(px(16.0))
        .child(section_header(cx, "Devices", "removable media"));

    if let Some(error) = &store.last_error {
        card = card.child(
            div()
                .text_color(theme::danger(cx))
                .text_xs()
                .child(error.clone()),
        );
    }

    for device in &store.devices {
        let label = device.label.clone();
        let is_mounted = device.mount_point.is_some();

        card = card.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .child(div().text_color(theme::fg(cx)).child(label))
                .child(
                    div()
                        .text_color(theme::muted(cx))
                        .text_xs()
                        .child(if is_mounted { "eject" } else { "mount" }),
                ),
        );
    }

    card.into_any_element()
}
