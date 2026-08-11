use crate::explorer::ExplorerPane;
use gpui::*;
use chronos_fm_ui::theme::theme;

/// The explorer header with navigation controls and the path bar.
pub mod header;
/// The main file listing, in list or grid mode, with the search bar.
pub mod listing;
/// The file preview pane.
pub mod preview;
/// The explorer sidebar with quick-access locations.
pub mod sidebar;

/// Renders the explorer page: header, sidebar, listing, and preview panes.
pub fn render(
    page: &mut ExplorerPane,
    window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    page.ensure_loaded(window, cx);
    page.update_editor_search(window, cx);
    if !page.focus_requested {
        page.focus_requested = true;
        cx.focus_self(window);
    }

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(theme::bg(cx))
        .relative()
        .track_focus(&page.focus_handle)
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
            let key_lc = event.keystroke.key.to_lowercase();
            let modifiers = event.keystroke.modifiers;
            let with_modifier = modifiers.platform || modifiers.control;
            let is_f = key_lc == "f" || event.keystroke.key == "KeyF";
            let close_with_escape = key_lc == "escape" && this.search_visible;
            if (is_f && with_modifier) || close_with_escape {
                this.toggle_search(window, cx);
                cx.stop_propagation();
                return;
            }
            // Escape closes the batch-rename dialog or cancels the conflict
            // dialog before doing anything else (the properties dialog has no
            // Escape handler of its own).
            if key_lc == "escape"
                && (this.batch_rename.is_some() || this.conflict_dialog.is_some())
            {
                if this.batch_rename.is_some() {
                    this.close_batch_rename(cx);
                }
                if this.conflict_dialog.is_some() {
                    this.cancel_conflict_transfer(cx);
                }
                cx.stop_propagation();
                return;
            }
            // Selection model (§5). Escape while searching is handled above, so
            // here it only clears the selection.
            let pane_focused = this.focus_handle.is_focused(window);
            match key_lc.as_str() {
                "a" if with_modifier && pane_focused => {
                    this.select_all();
                    cx.stop_propagation();
                    cx.notify();
                }
                "c" if with_modifier && pane_focused => {
                    this.copy_selection(cx);
                    cx.stop_propagation();
                }
                "x" if with_modifier && pane_focused => {
                    this.cut_selection(cx);
                    cx.stop_propagation();
                }
                "v" if with_modifier && pane_focused => {
                    this.paste_clipboard(cx);
                    cx.stop_propagation();
                }
                "n" if with_modifier && modifiers.shift && pane_focused => {
                    this.new_folder(window, cx);
                    cx.stop_propagation();
                }
                "f2" if pane_focused => {
                    if let Some(ix) = this.active_index {
                        this.rename_selection(ix, window, cx);
                    }
                    cx.stop_propagation();
                }
                "delete" if pane_focused => {
                    this.confirm_delete_selection(window, cx);
                    cx.stop_propagation();
                }
                "enter" if pane_focused => {
                    // With the conflict dialog open, Enter is the default
                    // (Rename) decision per mockup §1.2. `conflict_decide`
                    // drives the pane's own pending-transfer queue directly
                    // (never re-entering the pane's entity), so it is safe to
                    // call from inside this pane update.
                    if this.conflict_dialog.is_some() {
                        this.conflict_decide(
                            crate::explorer::conflict::ConflictChoice::Rename,
                            cx,
                        );
                        cx.stop_propagation();
                        return;
                    }
                    let overlay_open = this.batch_rename.is_some()
                        || this.properties_dialog.is_some()
                        || this.context_menu.is_some()
                        || this.conflict_dialog.is_some();
                    if !overlay_open && !this.search_visible {
                        if let Some(ix) = this.active_index {
                            if let Some(item) = this.filtered_entries.get(ix).cloned() {
                                this.activate_entry(item, window, cx);
                            }
                        }
                    }
                    cx.stop_propagation();
                }
                "i" if with_modifier => {
                    this.show_properties(cx);
                    cx.stop_propagation();
                    cx.notify();
                }
                // Search-close Escape is handled by the branch above (which
                // returns early), so here Escape only clears a selection — and
                // only consumes the event when there was one to clear, leaving
                // an empty-selection Escape free to bubble.
                "escape" if !this.selection.is_empty() => {
                    this.clear_selection();
                    cx.stop_propagation();
                    cx.notify();
                }
                "up" => {
                    this.move_active(-1, modifiers.shift);
                    cx.stop_propagation();
                    cx.notify();
                }
                "down" => {
                    this.move_active(1, modifiers.shift);
                    cx.stop_propagation();
                    cx.notify();
                }
                _ => {}
            }
        }))
        .on_mouse_move(
            cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                if this.resizing_column.is_some() {
                    this.update_column_resize(event.position);
                    cx.notify();
                }
            }),
        )
        .on_mouse_up(
            gpui::MouseButton::Left,
            cx.listener(|this, _event, _window, cx| {
                if this.resizing_column.is_some() {
                    this.stop_column_resize();
                    cx.notify();
                }
            }),
        )
        .child(header::render(page, window, cx))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_grow_1()
                .min_h(px(0.0))
                .child(
                    // Sidebar: bypass resizable panel (h_resizable doesn't
                    // allocate space for the sidebar panel — see T043
                    // investigation). Use a simple flex child with fixed
                    // width instead.
                    if page.sidebar_visible {
                        div()
                            .w(px(212.0))
                            .h_full()
                            .overflow_hidden()
                            .border_r_1()
                            .border_color(theme::border(cx))
                            .child(sidebar::render(page, window, cx))
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    },
                )
                // T044: the sidebar bypass above (T043) replaced this whole
                // row's children instead of just the sidebar slot, silently
                // dropping the listing and preview panes from the render
                // tree entirely — the empty content pane was never a
                // layout/virtualization bug, the panes just weren't there.
                // Restored via `h_resizable` with its original two panels
                // (listing + preview); only the sidebar panel had the
                // allocation bug, so drag-resize between listing/preview is
                // kept.
                .child(
                    gpui_component::resizable::h_resizable("file-explorer")
                        .with_state(&page.resizable)
                        .child(gpui_component::resizable::resizable_panel().child(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .min_h(px(0.0))
                                .overflow_hidden()
                                .child(listing::render(page, window, cx)),
                        ))
                        .child(
                            gpui_component::resizable::resizable_panel()
                                .size(px(240.0))
                                .size_range(px(240.0)..px(2000.0))
                                .child(
                                    div()
                                        .size_full()
                                        .overflow_hidden()
                                        .border_l_1()
                                        .border_color(theme::border(cx))
                                        .child(preview::render(page, window, cx)),
                                ),
                        )
                        .into_any_element(),
                ),
        )
        .child(render_properties_dialog(page, cx))
        .child(render_batch_rename_dialog(page, cx))
        .child(render_conflict_dialog(page, cx))
        .child(render_context_menu(page, window, cx))
}

fn render_batch_rename_dialog(
    page: &mut ExplorerPane,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement {
    if let Some(dialog) = &page.batch_rename {
        return div()
            .absolute()
            .inset_0()
            .bg(gpui::hsla(0.0, 0.0, 0.0, 0.4))
            .flex()
            .items_center()
            .justify_center()
            // Click-outside closes. The card itself stops propagation so a
            // click inside (focusing an input, pressing a button) never reaches
            // this scrim — otherwise the dialog would close on mouse-down and
            // `Button::on_click` (mouse-up) would never fire.
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.close_batch_rename(cx);
                }),
            )
            .child(
                div().on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _event, _window, cx| {
                        cx.stop_propagation();
                    }),
                )
                .child(dialog.clone()),
            )
            .into_any_element();
    }
    div().into_any_element()
}

fn render_conflict_dialog(
    page: &mut ExplorerPane,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement {
    if let Some(dialog) = &page.conflict_dialog {
        return div()
            .absolute()
            .inset_0()
            .bg(gpui::hsla(0.0, 0.0, 0.0, 0.4))
            .flex()
            .items_center()
            .justify_center()
            // Click-outside cancels the whole operation. The card stops
            // propagation so clicking a button never reaches this scrim.
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.cancel_conflict_transfer(cx);
                }),
            )
            .child(
                div().on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _event, _window, cx| {
                        cx.stop_propagation();
                    }),
                )
                .child(dialog.clone()),
            )
            .into_any_element();
    }
    div().into_any_element()
}

fn render_context_menu(
    page: &mut ExplorerPane,
    window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement {
    if let Some(menu) = &page.context_menu.clone() {
        return div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.close_context_menu(cx);
                }),
            )
            .child(crate::explorer::context_menu::render(menu, window, cx))
            .into_any_element();
    }
    div().into_any_element()
}

fn render_properties_dialog(
    page: &mut ExplorerPane,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement {
    if let Some(dialog) = &page.properties_dialog {
        return div()
            .absolute()
            .inset_0()
            .bg(gpui::hsla(0.0, 0.0, 0.0, 0.4))
            .flex()
            .items_center()
            .justify_center()
            // Click-outside closes. The card stops propagation so a click inside
            // (selecting text, hovering a row) never reaches this scrim — same
            // guard as the batch-rename dialog, which needs it for its inputs
            // and buttons.
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _event, _window, cx| {
                this.close_properties(cx);
            }))
            .child(
                div().on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_this, _event, _window, cx| {
                        cx.stop_propagation();
                    }),
                )
                .child(dialog.clone()),
            )
            .into_any_element();
    }
    div().into_any_element()
}

/// Returns highlight ranges for every case-insensitive occurrence of `query`
/// within `text`, for emphasizing search matches.
pub fn find_query_highlights(
    text: &str,
    query: &str,
) -> Vec<(std::ops::Range<usize>, gpui::HighlightStyle)> {
    let mut highlights = Vec::new();
    if query.is_empty() {
        return highlights;
    }

    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let text_chars: Vec<(usize, char)> = text.char_indices().collect();

    let mut i = 0;
    while i < text_chars.len() {
        let mut match_found = true;
        let mut q_idx = 0;

        let mut current_t_offset = 0;

        while q_idx < query_lower.len() {
            if i + current_t_offset >= text_chars.len() {
                match_found = false;
                break;
            }

            let (_, t_char) = text_chars[i + current_t_offset];
            let t_lower = t_char.to_lowercase();

            for tc in t_lower {
                if q_idx >= query_lower.len() || query_lower[q_idx] != tc {
                    match_found = false;
                    break;
                }
                q_idx += 1;
            }

            if !match_found {
                break;
            }
            current_t_offset += 1;
        }

        if match_found && q_idx == query_lower.len() {
            let start_byte = text_chars[i].0;
            let end_byte = if i + current_t_offset < text_chars.len() {
                text_chars[i + current_t_offset].0
            } else {
                text.len()
            };

            highlights.push((
                start_byte..end_byte,
                gpui::HighlightStyle {
                    background_color: Some(gpui::Hsla::from(gpui::Rgba {
                        r: 1.0,
                        g: 0.9,
                        b: 0.0,
                        a: 0.5,
                    })),
                    color: Some(gpui::Hsla::from(gpui::Rgba {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    })),
                    ..Default::default()
                },
            ));

            i += current_t_offset;
        } else {
            i += 1;
        }
    }

    highlights.retain(|(range, _)| {
        let start_ok = text.is_char_boundary(range.start);
        let end_ok = text.is_char_boundary(range.end);
        if !start_ok || !end_ok {
            if std::env::var("NOHR_DEBUG").is_ok() {
                tracing::error!("[CRITICAL] find_query_highlights: Removing invalid highlight: {:?} (start_ok={}, end_ok={}) in text len {}", range, start_ok, end_ok, text.len());
            }
            return false;
        }
        true
    });

    highlights
}
