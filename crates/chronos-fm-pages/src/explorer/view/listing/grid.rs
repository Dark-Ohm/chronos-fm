use super::truncate_middle;
use crate::explorer::ExplorerPane;
use crate::explorer::clipboard::{self, ClipboardMode};
use gpui::prelude::*;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::Icon;
use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_ui::theme::theme;

/// Renders the file listing as a grid of icon tiles.
pub fn render(
    page: &mut ExplorerPane,
    window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> AnyElement {
    let items = page.filtered_entries.clone();
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

    div()
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
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _window, cx| {
                // Normalize selection to the right-clicked tile (b3/b4
                // convention): single-select unless it is already selected.
                if !this.is_selected(ix) {
                    this.select_single(ix);
                }
                this.open_context_menu(
                    context_menu_path.clone(),
                    ix,
                    event.position,
                    cx,
                );
                cx.stop_propagation();
            }),
        )
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                this.record_click(ix, event.click_count);
                let modifiers = event.modifiers;
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
                if event.click_count >= 2 {
                    this.activate_entry(activation_item.clone(), window, cx);
                }
                cx.notify();
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
        })
        .into_any_element()
}
