use super::truncate_middle;
use crate::explorer::ExplorerPane;
use crate::explorer::clipboard::{self, ClipboardMode};
use crate::explorer::dnd::{
    DropTarget, DropTargetKind, FileDrag, file_drag_for_item, set_file_drag_cursor,
};
use crate::explorer::marquee::intersects_closed;
use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::{ElementExt, Icon};

/// Renders the file listing as a grid of icon tiles.
pub fn render(
    page: &mut ExplorerPane,
    window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> AnyElement {
    let items = page.filtered_entries.clone();
    let scroll_handle = page.virtual_scroll_handle.clone();
    let mut grid = div()
        .flex()
        .flex_wrap()
        .gap_4()
        .items_start()
        .min_h(px(0.0));

    for (ix, item) in items.into_iter().enumerate() {
        let selected = page.is_selected(ix);
        grid = grid.child(render_grid_item(page, item, ix, selected, window, cx));
    }

    div()
        .id("grid-scroll")
        .flex_1()
        .overflow_scroll()
        .track_scroll(&scroll_handle)
        .px(px(16.0))
        .py(px(16.0))
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _window, cx| {
                this.open_context_menu_for_directory(event.position, cx);
                cx.stop_propagation();
            }),
        )
        .child(grid)
        .into_any_element()
}

fn render_grid_item(
    page: &mut ExplorerPane,
    item: FileEntryDto,
    ix: usize,
    selected: bool,
    _window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> AnyElement {
    let icon_path = super::row::icon_path_for(&item.name, &item.kind);

    // Mockup §2.8: a grid tile carries icon + name only. Type/size/modified
    // live in the list view and the preview pane — at 88px wide they render as
    // ellipsised fragments, which is why the mockup drops them here.
    let name = truncate_middle(&item.name, 28);
    let activation_item = item.clone();
    let preview_item = item.clone();
    let context_menu_path = item.path.clone();
    let entity = cx.entity().clone();
    let geometry_entity = entity.clone();
    let file_drag = file_drag_for_item(page, &item, ix, entity.clone());
    let folder_target = (item.kind == "dir"
        && page.provider.is_none()
        && !page
            .renaming
            .as_ref()
            .is_some_and(|(renaming_ix, _)| *renaming_ix == ix))
    .then(|| DropTarget {
        directory: item.path.clone().into(),
        kind: DropTargetKind::FolderItem,
    });

    let bg_color = if selected {
        theme::bg_hover(cx)
    } else {
        theme::bg(cx)
    };

    let border_color = if selected {
        theme::accent(cx)
    } else {
        theme::border(cx)
    };

    let clip = clipboard::current(cx);
    let is_cut = clip.mode == Some(ClipboardMode::Cut) && clip.paths.contains(&item.path);

    let tile = div()
        .id(("grid-item-menu", ix))
        .w(px(88.0))
        .px(px(6.0))
        .py(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(border_color)
        .bg(bg_color)
        .hover(|this| this.bg(theme::bg_hover(cx)))
        .cursor_pointer()
        .flex()
        .flex_col()
        // Mockup §2.8: the tile is icon + caption, centred, height driven by
        // content — the old `min_h(140)` was sized for the four metadata lines
        // below and would turn the 88px tile into a well once they are gone.
        .items_center()
        .gap(px(7.0))
        .when(is_cut, |el| el.opacity(0.5))
        .on_prepaint(move |bounds, _window, cx| {
            geometry_entity.update(cx, |pane, _cx| {
                if let Some(token) = pane.geometry_token.clone() {
                    if intersects_closed(bounds, token.viewport) {
                        pane.record_item_bounds(ix, bounds, token);
                    } else if pane.geometry_token.as_ref() == Some(&token) {
                        pane.measured_items.remove(&ix);
                    }
                }
            });
        })
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _window, cx| {
                // Normalize selection to the right-clicked tile (b3/b4
                // convention): single-select unless it is already selected.
                if !this.is_selected(ix) {
                    this.select_single(ix);
                }
                this.open_context_menu(context_menu_path.clone(), ix, event.position, cx);
                cx.stop_propagation();
            }),
        )
        .on_click(
            cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                if cx.has_active_drag() {
                    return;
                }
                if let gpui::ClickEvent::Mouse(mouse) = event
                    && mouse.up.button == gpui::MouseButton::Left
                {
                    this.record_click(ix, mouse.up.click_count);
                    let modifiers = mouse.up.modifiers;
                    if modifiers.shift {
                        this.select_range_to(ix);
                    } else if modifiers.platform || modifiers.control {
                        this.toggle_select(ix);
                    } else {
                        this.select_single(ix);
                    }
                    if preview_item.kind == "file" {
                        this.open_preview(preview_item.path.clone(), window, cx);
                    }
                    if mouse.up.click_count >= 2 {
                        this.activate_entry(activation_item.clone(), window, cx);
                    }
                    cx.notify();
                }
            }),
        )
        .child(
            Icon::new(Icon::empty())
                .path(icon_path)
                .size_8()
                .text_color(theme::gray_600(cx)),
        )
        .child({
            // Grid-mode inline rename rendering (Task 6 parity with the list
            // row): swap the tile's name label for a live input when this tile
            // is the one being renamed, Enter commits, Escape cancels.
            let renaming_input = page
                .renaming
                .as_ref()
                .filter(|(renaming_ix, _)| *renaming_ix == ix)
                .map(|(_, input)| input.clone());
            if let Some(input) = renaming_input {
                div()
                    .w_full()
                    .on_key_down(cx.listener(
                        move |this, event: &gpui::KeyDownEvent, _window, cx| {
                            if event.keystroke.key == "enter" {
                                this.commit_rename(cx);
                            } else if event.keystroke.key == "escape" {
                                this.cancel_rename(cx);
                            }
                        },
                    ))
                    .child(Input::new(&input))
                    .into_any_element()
            } else {
                div()
                    .w_full()
                    .text_size(px(10.5))
                    .text_center()
                    .text_color(theme::fg(cx))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(name)
                    .into_any_element()
            }
        });

    tile.when_some(file_drag, |tile, drag| {
        tile.on_drag(drag, |drag, _offset, _window, cx| {
            drag.activate(cx);
            cx.new(|_cx| drag.preview())
        })
    })
    .when_some(folder_target, |tile, target| {
        let target_id = entity.entity_id();
        let pane_for_move = entity.clone();
        let move_target = target.clone();
        let pane_for_can_drop = entity.clone();
        let can_drop_target = target.clone();
        let pane_for_style = entity.clone();
        let style_target = target.clone();
        let drop_target = target.clone();

        tile.on_drag_move::<FileDrag>(move |event, window, cx| {
            if event.bounds.contains(&event.event.position)
                && pane_for_move.read(cx).can_accept_file_drop(
                    target_id,
                    event.drag(cx),
                    &move_target,
                    event.event.modifiers,
                    cx,
                )
            {
                let cursor = if crate::explorer::dnd::drop_mode(event.event.modifiers)
                    == crate::explorer::dnd::DropMode::Copy
                {
                    gpui::CursorStyle::DragCopy
                } else {
                    gpui::CursorStyle::ClosedHand
                };
                set_file_drag_cursor(cursor, window, cx);
            }
        })
        .can_drop(move |value, window, cx| {
            value.downcast_ref::<FileDrag>().is_some_and(|drag| {
                pane_for_can_drop.read(cx).can_accept_file_drop(
                    target_id,
                    drag,
                    &can_drop_target,
                    window.modifiers(),
                    cx,
                )
            })
        })
        .drag_over::<FileDrag>(move |style, drag, window, cx| {
            if pane_for_style.read(cx).can_accept_file_drop(
                target_id,
                drag,
                &style_target,
                window.modifiers(),
                cx,
            ) {
                style
                    .border_color(theme::accent(cx))
                    .bg(theme::accent_light(cx))
            } else {
                style
            }
        })
        .on_drop(cx.listener(move |pane, drag: &FileDrag, window, cx| {
            let target_id = cx.entity().entity_id();
            if pane.can_accept_file_drop(target_id, drag, &drop_target, window.modifiers(), cx) {
                pane.begin_file_drop(drag.clone(), drop_target.clone(), window.modifiers(), cx);
            }
        }))
    })
    .into_any_element()
}
