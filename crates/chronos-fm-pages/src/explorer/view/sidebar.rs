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
use gpui_component::{Icon, IconName};

/// Renders the sidebar: a single «Places» card with folders + devices.
pub fn render(
    page: &mut ExplorerPane,
    _window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    let fg = theme::fg(cx);

    // `h_full` makes the single Places card fill the whole sidebar column so
    // the panel doesn't end mid-sidebar with empty space below (T015 open
    // item #4): the card reads as one cohesive Places panel, Dolphin-style.
    let mut card = elevated_card(cx)
        .h_full()
        .child(section_header(cx, "Places", "quick access"));

    // ---- Folders (cached shortcuts) ----
    for (i, (label, path)) in page.shortcuts.iter().enumerate() {
        let p = path.clone();
        let lbl = label.clone();
        card = card.child(
            ListItem::new(("folder", i))
                .on_click({
                    let p = p.clone();
                    cx.listener(move |this, _, window, cx| {
                        this.change_dir(p.clone(), window, cx);
                    })
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(Icon::new(folder_icon(&lbl)).size_4().text_color(theme::gray_600(cx)))
                        .child(div().text_sm().text_color(fg).child(lbl.clone())),
                ),
        );
    }

    // ---- Devices ----
    let store = cx.try_global::<devices_store::DeviceStore>();
    if let Some(store) = store {
        if store.devices.is_empty() {
            return card.into_any_element();
        }

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
                                Icon::new(IconName::HardDrive)
                                    .size_4()
                                    .text_color(theme::gray_600(cx)),
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
                                                div()
                                                    .text_sm()
                                                    .text_color(fg)
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
                                            .text_sm()
                                            .text_color(fg)
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

fn folder_icon(label: &str) -> IconName {
    match label {
        "Home" => IconName::Folder,
        _ => IconName::Folder,
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
