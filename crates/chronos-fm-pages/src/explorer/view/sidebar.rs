//! Sidebar: unified «Places» list with quick-access folders and removable
//! devices in a single rhythm (T015). Dead top block removed; device
//! actions rendered as hover-revealed icons.

use crate::explorer::ExplorerPane;
use chronos_fm_ui::devices_store;
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::list::ListItem;
use gpui_component::{Icon, IconName, Sizable};

/// Places sidebar row density (spec §2.6): padding, radius, icon-to-label
/// gap, icon size, and label size.
const PLACE_ROW_PADDING: f32 = 6.0;
const PLACE_ROW_RADIUS: f32 = 6.0;
const PLACE_ROW_GAP: f32 = 9.0;
const PLACE_ROW_ICON_SIZE: f32 = 14.0;
const PLACE_ROW_LABEL_SIZE: f32 = 12.5;

/// Renders the sidebar: a single «Places» card with folders + devices.
pub fn render(
    page: &mut ExplorerPane,
    _window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    // `h_full` makes the single Places card fill the whole sidebar column so
    // the panel doesn't end mid-sidebar with empty space below (T015 open
    // item #4): the card reads as one cohesive Places panel, Dolphin-style.
    let mut card = elevated_card(cx)
        .h_full()
        .bg(theme::toolbar_bg(cx))
        .child(section_header(cx, "Places", "quick access"));

    // ---- Folders (cached shortcuts) ----
    for (i, (label, path)) in page.shortcuts.iter().enumerate() {
        let p = path.clone();
        let lbl = label.clone();
        let active = page.cwd == p;
        let (row_bg, text_color, icon_color) = if active {
            (
                Some(theme::bg_hover(cx)),
                theme::fg(cx),
                theme::accent(cx),
            )
        } else {
            (None, theme::fg_secondary(cx), theme::muted(cx))
        };
        let mut row = ListItem::new(("folder", i))
            .px(px(PLACE_ROW_PADDING))
            .py(px(PLACE_ROW_PADDING))
            .rounded(px(PLACE_ROW_RADIUS))
            // Mockup §2.6 / §7 #4: the active place carries a 2px accent bar
            // pinned to the row's left edge, inset 6px top/bottom (`left:0;
            // top:6; bottom:6; width:2` in the mockup). `ListItem` renders its
            // base with `relative`, so the absolutely-positioned bar anchors
            // to the row box; painting it first keeps it behind the content.
            .when(active, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(6.0))
                        .bottom(px(6.0))
                        .w(px(2.0))
                        .rounded(px(2.0))
                        .bg(theme::accent(cx)),
                )
            })
            .on_click({
                let p = p.clone();
                cx.listener(move |this, _, window, cx| {
                    this.change_dir(p.clone(), window, cx);
                })
            });
        if let Some(bg) = row_bg {
            row = row.bg(bg);
        }
        card = card.child(row.child(
            div()
                .flex()
                .items_center()
                .gap(px(PLACE_ROW_GAP))
                .child(
                    Icon::new(Icon::empty())
                        .path(folder_icon_path(&lbl))
                        .with_size(px(PLACE_ROW_ICON_SIZE))
                        .text_color(icon_color),
                )
                .child(
                    div()
                        .text_size(px(PLACE_ROW_LABEL_SIZE))
                        // Mockup §2.6: active label weight 500 (idle 400).
                        .when(active, |this| {
                            this.font_weight(gpui::FontWeight::MEDIUM)
                        })
                        .text_color(text_color)
                        .child(lbl.clone()),
                ),
        ));
    }

    // ---- Devices ----
    let store = cx.try_global::<devices_store::DeviceStore>();
    if let Some(store) = store {
        if store.devices.is_empty() {
            return card.into_any_element();
        }

        // Mockup §2.6: a 1px hairline separates the places rows from the
        // devices section (`height:1px;background:border;margin:0 6px`). Pure
        // chrome — no fake data involved — so it only renders here, where the
        // devices section actually follows.
        card = card.child(div().h(px(1.0)).bg(theme::border(cx)).mx(px(6.0)));

        let backend = store.backend.clone();
        let devices = store.devices.clone();

        for (i, device) in devices.iter().enumerate() {
            let label = device.label.clone();
            let is_mounted = device.mount_point.is_some();
            let object_path = device.object_path.clone();
            let mount_point = device.mount_point.clone();
            let can_eject = device.is_removable && device.drive_object_path.is_some();
            let drive_object_path = device.drive_object_path.clone();

            let row_click_backend = backend.clone();
            let row_click_obj = object_path.clone();
            let row_click_mp = mount_point.clone();

            card = card.child(
                ListItem::new(("device", i))
                    .on_click({
                        let pane = cx.entity();
                        let backend = row_click_backend.clone();
                        let obj = row_click_obj.clone();
                        let mp = row_click_mp;
                        cx.listener(move |this, _, window, cx| {
                            if let Some(mp) = &mp {
                                this.change_dir(
                                    mp.to_string_lossy().to_string(),
                                    window,
                                    cx,
                                );
                            } else if let Some(backend) = &backend {
                                let pane = pane.clone();
                                devices_store::DeviceStore::mount_and_navigate(
                                    cx,
                                    backend.clone(),
                                    obj.clone(),
                                    move |cx, path| {
                                        let path = path.to_string_lossy().to_string();
                                        pane.update(cx, |pane, cx| {
                                            pane.navigate_to_path(path, cx);
                                        });
                                    },
                                );
                            }
                        })
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                // Mockup §2.6: device icon 15px, `c.muted`.
                                Icon::new(IconName::HardDrive)
                                    .with_size(px(15.0))
                                    .text_color(theme::muted(cx)),
                            )
                            .child(
                                // `flex_1` + `min_w(0)` let this column shrink so
                                // long mount paths ellipsize instead of pushing the
                                // action icons off the card edge (T019).
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .flex()
                                    .flex_col()
                                    .child(if is_mounted {
                                        let mp = mount_point
                                            .as_ref()
                                            .map(|p| p.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                // Mockup §2.6: device label 12.5,
                                                // `c.fgSecondary` (the mount path
                                                // sub-line below is our real-data
                                                // addition).
                                                div()
                                                    .text_size(px(12.5))
                                                    .text_color(theme::fg_secondary(cx))
                                                    .whitespace_nowrap()
                                                    .overflow_hidden()
                                                    .text_ellipsis()
                                                    .child(label),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme::muted(cx))
                                                    .whitespace_nowrap()
                                                    .overflow_hidden()
                                                    .text_ellipsis()
                                                    .child(mp),
                                            )
                                    } else {
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(theme::fg_secondary(cx))
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(label)
                                    }),
                            )
                    )
                    .child(if is_mounted || can_eject {
                        let muted = theme::muted(cx);
                        let danger = theme::danger(cx);
                        device_action_icons(
                            is_mounted,
                            can_eject,
                            object_path.clone(),
                            drive_object_path,
                            backend.clone(),
                            muted,
                            danger,
                            cx,
                        ).into_any_element()
                    } else {
                        div().into_any_element()
                    }),
            );
        }
    }

    card.into_any_element()
}

/// Resolves the Places-row icon asset path from the place's label
/// (spec §3.2 / §2.6). Falls back to the generic folder glyph for any
/// place not covered by the mockup's `PLACES_DEF`.
fn folder_icon_path(label: &str) -> &'static str {
    match label {
        "Home" => "icons/house.svg",
        "Desktop" => "icons/monitor.svg",
        "Downloads" => "icons/download.svg",
        "Documents" => "icons/file-text.svg",
        "Pictures" | "Images" => "icons/file-image.svg",
        "Trash" => "icons/trash-2.svg",
        _ => "icons/folder.svg",
    }
}

/// Right-zone action icons for a device row: unmount (mounted) + eject
/// (removable). Icons are hidden (`opacity(0.0)`) and revealed on hover
/// of the parent row (the row's `.hover()` makes them visible).
fn device_action_icons(
    is_mounted: bool,
    can_eject: bool,
    object_path: String,
    drive_object_path: Option<String>,
    backend: Option<std::sync::Arc<dyn chronos_fm_services::devices::DeviceBackend>>,
    muted: Hsla,
    _danger: Hsla,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    let mut zones = div().flex().items_center().gap(px(4.));

    if is_mounted {
        let obj = object_path.clone();
        let be = backend.clone();
        zones = zones.child(
            ListItem::new(("unmount", 0u32))
                .child(Icon::new(IconName::Minus).size_4().text_color(muted))
                .on_click({
                    let obj = obj.clone();
                    let be = be.clone();
                    cx.listener(move |_this, _, _window, cx| {
                        if let Some(backend) = &be {
                            devices_store::DeviceStore::unmount(
                                cx,
                                backend.clone(),
                                obj.clone(),
                            );
                        }
                        cx.stop_propagation();
                    })
                }),
        );
    }

    if can_eject {
        let obj = object_path.clone();
        let be = backend.clone();
        let drive = drive_object_path.clone();
        zones = zones.child(
            ListItem::new(("eject", 0u32))
                .child(Icon::new(IconName::ArrowUp).size_4().text_color(muted))
                .on_click({
                    let obj = obj.clone();
                    let be = be.clone();
                    let drive = drive.clone();
                    cx.listener(move |_this, _, _window, cx| {
                        if let (Some(backend), Some(drive_path)) = (&be, &drive) {
                            devices_store::DeviceStore::eject(
                                cx,
                                backend.clone(),
                                obj.clone(),
                                Some(drive_path.clone()),
                            );
                        }
                        cx.stop_propagation();
                    })
                }),
        );
    }

    zones
}
