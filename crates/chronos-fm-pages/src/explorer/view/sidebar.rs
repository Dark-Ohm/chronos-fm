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
///
/// Click wiring (T008): clicking a row navigates to the mount point when
/// mounted, or mounts and then navigates when unmounted; a mounted row's
/// right-hand "unmount" label is its own click target that unmounts without
/// navigating, and removable volumes get an "eject" click target that ejects
/// the owning drive (via the resolved `Block.Drive` path). Rows render without
/// click handlers when no live backend is registered (headless/container —
/// nothing to call).
pub(crate) fn render_devices_section(cx: &mut Context<ExplorerPane>) -> impl IntoElement {
    let store = cx.try_global::<devices_store::DeviceStore>();
    let Some(store) = store else {
        return div().into_any_element();
    };
    if store.devices.is_empty() {
        return div().into_any_element();
    }
    let backend = store.backend.clone();
    let devices = store.devices.clone();
    let last_error = store.last_error.clone();

    let hover_bg = theme::bg_hover(cx);
    let fg = theme::fg(cx);
    let muted = theme::muted(cx);
    let danger = theme::danger(cx);

    let mut card = elevated_card(cx)
        .mt(px(16.0))
        .child(section_header(cx, "Devices", "removable media"));

    if let Some(error) = &last_error {
        card = card.child(
            div()
                .text_color(danger)
                .text_xs()
                .child(error.clone()),
        );
    }

    for device in &devices {
        let label = device.label.clone();
        let is_mounted = device.mount_point.is_some();
        let object_path = device.object_path.clone();
        let mount_point = device.mount_point.clone();

        // The pane entity handle, so the mount callback (which only has
        // `&mut App`, no `Window`) can navigate the pane after mounting.
        let pane = cx.entity();

        let row = div()
            .id(SharedString::from(format!("device-row-{object_path}")))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.))
            .cursor_pointer()
            .hover(move |this| this.bg(hover_bg));

        // Whole row: navigate when mounted, mount+navigate when unmounted.
        let row = if let Some(backend) = &backend {
            let backend = backend.clone();
            let object_path = object_path.clone();
            let mount_point = mount_point.clone();
            let pane = pane.clone();
            row.on_click(
                cx.listener(move |this, _, window, cx| {
                    if let Some(mp) = &mount_point {
                        this.change_dir(mp.to_string_lossy().to_string(), window, cx);
                    } else {
                        let pane = pane.clone();
                        devices_store::DeviceStore::mount_and_navigate(
                            cx,
                            backend.clone(),
                            object_path.clone(),
                            move |cx, path| {
                                let path = path.to_string_lossy().to_string();
                                pane.update(cx, |pane, cx| pane.navigate_to_path(path, cx));
                            },
                        );
                    }
                }),
            )
        } else {
            row
        };

        // Right-hand zones: separate click targets for unmount (mounted only)
        // and eject (removable drives only), so they don't double as the
        // navigate/mount click on the row itself.
        let mut zones = div().flex().items_center().gap(px(8.));

        // Mount/unmount target.
        zones = zones.child(if is_mounted {
            match &backend {
                Some(backend) => {
                    let backend = backend.clone();
                    let object_path = object_path.clone();
                    div()
                        .id(SharedString::from(format!("device-unmount-{object_path}")))
                        .text_color(muted)
                        .text_xs()
                        .cursor_pointer()
                        .hover(move |this| this.text_color(danger))
                        .child("unmount")
                        .on_click(cx.listener(move |_this, _, _window, cx| {
                            devices_store::DeviceStore::unmount(
                                cx,
                                backend.clone(),
                                object_path.clone(),
                            );
                            cx.stop_propagation();
                        }))
                }
                // No live backend: render the label as-is (no click target).
                // The `.id()` here exists only to match the `Some` arm's
                // `Stateful<Div>` type so the match arms unify — it carries no
                // interactivity.
                None => div()
                    .id(SharedString::from(format!("device-unmount-{object_path}")))
                    .text_color(muted)
                    .text_xs()
                    .child("unmount"),
            }
        } else {
            // Passive "mount" label (the row click itself mounts). The `.id()`
            // matches the `is_mounted` arm's `Stateful<Div>` type so the if/else
            // arms unify — it carries no interactivity.
            div()
                .id(SharedString::from(format!("device-mount-{object_path}")))
                .text_color(muted)
                .text_xs()
                .child("mount")
        });

        // Eject target: only for removable volumes that reference a Drive
        // object (`Drive.Eject` operates on the drive; internal non-removable
        // disks are left alone — T008: foreign internal partitions are off
        // limits without an explicit user request).
        let can_eject = device.is_removable && device.drive_object_path.is_some();
        if can_eject {
            let drive_object_path = device.drive_object_path.clone();
            zones = zones.child(match &backend {
                Some(backend) => {
                    let backend = backend.clone();
                    let object_path = object_path.clone();
                    let drive_object_path = drive_object_path.clone();
                    div()
                        .id(SharedString::from(format!("device-eject-{object_path}")))
                        .text_color(muted)
                        .text_xs()
                        .cursor_pointer()
                        .hover(move |this| this.text_color(danger))
                        .child("eject")
                        .on_click(cx.listener(move |_this, _, _window, cx| {
                            devices_store::DeviceStore::eject(
                                cx,
                                backend.clone(),
                                object_path.clone(),
                                drive_object_path.clone(),
                            );
                            cx.stop_propagation();
                        }))
                }
                // No live backend: passive label (id matches the `Some` arm's
                // `Stateful<Div>` type so the match arms unify).
                None => div()
                    .id(SharedString::from(format!("device-eject-{object_path}")))
                    .text_color(muted)
                    .text_xs()
                    .child("eject"),
            });
        }

        let row = row.child(div().text_color(fg).child(label)).child(zones);

        card = card.child(row);
    }

    card.into_any_element()
}
