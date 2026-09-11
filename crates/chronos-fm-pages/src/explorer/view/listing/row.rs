use super::truncate_middle;

use crate::explorer::ExplorerPane;
use crate::explorer::clipboard::{self, ClipboardMode};
use crate::explorer::dnd::{
    DropTarget, DropTargetKind, FileDrag, file_drag_for_item, set_file_drag_cursor,
};
use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::list::ListItem;
use gpui_component::{ActiveTheme, ElementExt, Icon, IconName};

/// Maps a file/dir name (by extension) and entry kind to an icon asset path
/// under `assets/icons/`. Single source of truth for file-type icon mapping
/// (T037 §D) — `grid.rs` reuses this rather than duplicating the extension
/// list. `IconName::*` variants are NOT used here on purpose: `IconName` is
/// generated from gpui-component's own asset pack and has no file-type
/// variants for our pack, so the only valid path is
/// `Icon::new(Icon::empty()).path(..)` (see `root.rs` for the same pattern).
pub fn icon_path_for(name: &str, kind: &str) -> &'static str {
    if kind == "dir" {
        return "icons/folder.svg";
    }
    let ext = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "json" | "toml" | "yaml" | "yml"
        | "sh" => "icons/file-code.svg",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => "icons/file-image.svg",
        "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" => "icons/file-archive.svg",
        "md" | "txt" | "rst" | "log" => "icons/file-text.svg",
        _ => "icons/file.svg",
    }
}

/// Renders a single listing row for the given entry at row index `ix`.
pub fn render(
    page: &ExplorerPane,
    item: &FileEntryDto,
    ix: usize,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    use chronos_fm_ui::components::file_list::{format_date, get_file_type, human_bytes};

    let selected = page.is_selected(ix);
    let icon_path = icon_path_for(&item.name, &item.kind);
    let icon_color = if selected {
        theme::accent(cx)
    } else {
        theme::fg_secondary(cx)
    };

    let bg_color = if selected {
        theme::accent_light(cx)
    } else if ix % 2 == 0 {
        theme::bg(cx)
    } else {
        theme::gray_50(cx)
    };

    let clip = clipboard::current(cx);
    let is_cut = clip.mode == Some(ClipboardMode::Cut) && clip.paths.contains(&item.path);

    let file_type = get_file_type(&item.name, &item.kind);

    let max_chars = (page.col_name_width / 8.0) as usize;
    let display_name = truncate_middle(&item.name, max_chars.max(20));

    let total_width = page.total_table_width();
    let item_for_preview = item.clone();
    let item_for_activate = item.clone();
    let context_menu_path = item_for_preview.path.clone();
    let context_menu_is_dir = item_for_preview.kind == "dir";
    let entity = cx.entity().clone();
    let geometry_entity = entity.clone();
    let file_drag = file_drag_for_item(page, item, ix, entity.clone());
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

    // Check if query matches filename (for highlighting)
    let query_lower = page.search_query.to_lowercase();
    let has_filename_match =
        !page.search_query.is_empty() && item.name.to_lowercase().contains(&query_lower);

    // Check if there are content matches (for expand arrow)
    let has_content_matches = page
        .search_results
        .as_ref()
        .map(|results| {
            results
                .iter()
                .any(|r| r.path == item.path && !r.matches.is_empty())
        })
        .unwrap_or(false);

    let is_expanded = page.expanded_search_files.contains(&item.path);

    let match_snippets: Vec<(usize, String)> = if is_expanded && has_content_matches {
        page.search_results
            .as_ref()
            .and_then(|results| {
                results.iter().find(|r| r.path == item.path).map(|r| {
                    r.matches
                        .iter()
                        .take(10) // Limit to 10 snippets
                        .map(|m| (m.line_number, m.line_content.clone()))
                        .collect()
                })
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let query = page.search_query.clone();
    let path_for_toggle = item.path.clone();
    let expand_icon = if is_expanded {
        IconName::ChevronDown
    } else {
        IconName::ChevronRight
    };

    // Create styled filename with highlighted matches
    let styled_name = if has_filename_match && !page.search_query.is_empty() {
        let highlights = crate::explorer::view::find_query_highlights(&display_name, &query);
        StyledText::new(display_name.clone()).with_highlights(highlights)
    } else {
        StyledText::new(display_name.clone())
    };

    // ... rendering ...
    // Note: page.total_table_width() method is needed.
    // I need to make `total_table_width` pub on ExplorerPane. I did make fields pub, but method?
    // I should check if `total_table_width` is a method. Yes (line 347 in original).
    // I need to ensure that method is `pub` or copy logic.
    // Copying logic is safer: `page.col_name_width + ...`.
    // I'll copy the logic.

    // T022 Part C deliberately keeps the outer `div()` wrapper here. The
    // outer carries the right-click handler, the cut-row dimming, the row
    // width, AND the `.children(snippet_divs)` for expanded search snippets
    // that render *as siblings of the ListItem* in a flex_col flow. Folding
    // it into ListItem would re-parent the snippets inside the ListItem's
    // own children collection (different layout semantics in gpui-component)
    // and risks a visual diff. Documented in the T022 report — see the
    // calibration section: this codebase is already flatter than the
    // ~15–25 % savings the report estimated.
    let row = div()
        .id(("file-row-menu", ix))
        .flex_col()
        .w(px(total_width))
        .when(is_cut, |el| el.opacity(0.5))
        .on_prepaint(move |bounds, _window, cx| {
            geometry_entity.update(cx, |pane, _cx| {
                if let Some(token) = pane.geometry_token.clone() {
                    pane.record_item_bounds(ix, bounds, token);
                }
            });
        })
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _window, cx| {
                // Normalize selection to the right-clicked row (b3/b4
                // convention): single-select unless it is already selected.
                if !this.is_selected(ix) {
                    this.select_single(ix);
                }
                this.open_context_menu(
                    context_menu_path.clone(),
                    ix,
                    context_menu_is_dir,
                    event.position,
                    cx,
                );
                cx.stop_propagation();
            }),
        )
        .child(
            ListItem::new(("file-row", ix))
                .w(px(total_width))
                .h(px(28.0))
                .px(px(10.0))
                .bg(bg_color)
                .on_click(
                    cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                        if cx.has_active_drag() {
                            return;
                        }
                        if let gpui::ClickEvent::Mouse(mouse) = event {
                            if mouse.up.button == gpui::MouseButton::Left {
                                this.record_click(ix, mouse.up.click_count);
                                let modifiers = mouse.up.modifiers;
                                if modifiers.shift {
                                    this.select_range_to(ix);
                                } else if modifiers.platform || modifiers.control {
                                    this.toggle_select(ix);
                                } else {
                                    this.select_single(ix);
                                }
                                if item_for_preview.kind == "file" {
                                    this.open_preview(item_for_preview.path.clone(), window, cx);
                                }
                                if mouse.up.click_count >= 2 {
                                    this.activate_entry(item_for_activate.clone(), window, cx);
                                }
                                cx.notify();
                            }
                        }
                    }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .w_full()
                        .h_full()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .w(px(page.col_name_width))
                                .flex_shrink_0()
                                .when(has_content_matches, |this| {
                                    this.child(
                                        div()
                                            .cursor_pointer()
                                            .hover(|s| s.bg(theme::bg_hover(cx)).rounded(px(4.0)))
                                            .p(px(2.0))
                                            .on_mouse_down(
                                                gpui::MouseButton::Left,
                                                cx.listener({
                                                    let path = path_for_toggle.clone();
                                                    move |this, _, _, cx| {
                                                        if this
                                                            .expanded_search_files
                                                            .contains(&path)
                                                        {
                                                            this.expanded_search_files
                                                                .remove(&path);
                                                        } else {
                                                            this.expanded_search_files
                                                                .insert(path.clone());
                                                        }
                                                        this.update_item_sizes();
                                                        cx.notify();
                                                    }
                                                }),
                                            )
                                            .child(
                                                Icon::new(expand_icon)
                                                    .size_3()
                                                    .text_color(theme::gray_600(cx)),
                                            ),
                                    )
                                })
                                .when(!has_content_matches, |this| this.child(div().w(px(20.0))))
                                .child(
                                    Icon::new(Icon::empty())
                                        .path(icon_path)
                                        .size_4()
                                        .text_color(icon_color),
                                )
                                .child({
                                    let renaming_input = page
                                        .renaming
                                        .as_ref()
                                        .filter(|(renaming_ix, _)| *renaming_ix == ix)
                                        .map(|(_, input)| input.clone());
                                    if let Some(input) = renaming_input {
                                        div()
                                            .flex_1()
                                            .on_key_down(cx.listener(
                                                move |this, event: &gpui::KeyDownEvent, window, cx| {
                                                    if event.keystroke.key == "enter" {
                                                        this.commit_rename(window, cx);
                                                    } else if event.keystroke.key == "escape" {
                                                        this.cancel_rename(window, cx);
                                                    }
                                                },
                                            ))
                                            .child(Input::new(&input))
                                            .into_any_element()
                                    } else {
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(if selected {
                                                theme::fg(cx)
                                            } else {
                                                theme::fg_secondary(cx)
                                            })
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .whitespace_nowrap()
                                            .child(styled_name)
                                            .into_any_element()
                                    }
                                }),
                        )
                        .child(
                            // Mockup §2.7 / §7 #5: Type/Size/Modified are mono
                            // 10.5, `c.muted`, right-aligned. (Name stays the
                            // proportional UI font at 12.5.)
                            div()
                                .w(px(page.col_type_width))
                                .flex_shrink_0()
                                .text_size(px(10.5))
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_color(theme::muted(cx))
                                .text_right()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(file_type),
                        )
                        .child(
                            div()
                                .w(px(page.col_size_width))
                                .flex_shrink_0()
                                .text_size(px(10.5))
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_color(theme::muted(cx))
                                .text_right()
                                .child(match item.kind.as_str() {
                                    "file" => human_bytes(item.size),
                                    "dir" => "-".to_string(),
                                    other => other.to_string(),
                                }),
                        )
                        .child(
                            div()
                                .w(px(page.col_modified_width))
                                .flex_shrink_0()
                                .text_size(px(10.5))
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_color(theme::muted(cx))
                                .text_right()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(format_date(&item.modified)),
                        ),
                ),
        )
        .children(
            match_snippets
                .into_iter()
                .map(|(line_num, content)| {
                    let highlights = crate::explorer::view::find_query_highlights(&content, &query);
                    let styled = StyledText::new(content.clone()).with_highlights(highlights);
                    let path = item.path.clone();
                    div()
                        .id(SharedString::from(format!(
                            "snippet-{}-{}",
                            path.clone(),
                            line_num
                        )))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.scroll_to_line(line_num, window, cx);
                            cx.notify();
                        }))
                        .h(px(24.0))
                        .pl(px(48.0))
                        .pr(px(24.0))
                        .bg(theme::gray_50(cx))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::muted(cx))
                                .w(px(32.0))
                                .child(format!("{}", line_num)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::fg_secondary(cx))
                                .flex_1()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(styled),
                        )
                })
                .collect::<Vec<_>>(),
        );

    row.when_some(file_drag, |row, drag| {
        row.on_drag(drag, |drag, _offset, window, cx| {
            drag.activate(cx);
            // T052 drag-out: also start the OS-level drag so the payload can
            // leave the app (Wayland data-device source). In-app drops still
            // complete through the internal `FileDrag` path below.
            window.start_external_drag(drag.paths().to_vec().into());
            cx.new(|_cx| drag.preview())
        })
    })
    .when_some(folder_target, |row, target| {
        let target_id = entity.entity_id();
        let pane_for_move = entity.clone();
        let move_target = target.clone();
        let pane_for_can_drop = entity.clone();
        let can_drop_target = target.clone();
        let pane_for_style = entity.clone();
        let style_target = target.clone();
        let drop_target = target.clone();
        // T052 drop-in: OS-originated payloads (`ExternalPaths`) can also land
        // on a folder row; move-closures need their own captures, so the
        // external handlers use dedicated clones below.
        let external_pane_for_move = entity.clone();
        let external_move_target = target.clone();
        let external_pane_for_can_drop = entity.clone();
        let external_can_drop_target = target.clone();
        let external_pane_for_style = entity.clone();
        let external_style_target = target.clone();
        let external_drop_target = target.clone();

        row.on_drag_move::<FileDrag>(move |event, window, cx| {
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
        .drag_over::<FileDrag>(move |style, drag, window, cx| {
            if pane_for_style.read(cx).can_accept_file_drop(
                target_id,
                drag,
                &style_target,
                window.modifiers(),
                cx,
            ) {
                style
                    .border_1()
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
        .on_drag_move::<ExternalPaths>(move |event, window, cx| {
            if event.bounds.contains(&event.event.position)
                && external_pane_for_move.read(cx).can_accept_external_drop(
                    event.drag(cx).paths(),
                    &external_move_target,
                    event.event.modifiers,
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
        .drag_over::<ExternalPaths>(move |style, paths, window, cx| {
            if external_pane_for_style.read(cx).can_accept_external_drop(
                paths.paths(),
                &external_style_target,
                window.modifiers(),
            ) {
                style
                    .border_1()
                    .border_color(theme::accent(cx))
                    .bg(theme::accent_light(cx))
            } else {
                style
            }
        })
        .on_drop(cx.listener(move |pane, paths: &ExternalPaths, window, cx| {
            if pane.can_accept_external_drop(paths.paths(), &external_drop_target, window.modifiers())
            {
                pane.begin_external_drop(
                    paths.paths().to_vec(),
                    external_drop_target.clone(),
                    window.modifiers(),
                    cx,
                );
            }
        }))
        // The drop gate is a single predicate per element, so it must accept
        // both payload types.
        .can_drop(move |value, window, cx| {
            if let Some(drag) = value.downcast_ref::<FileDrag>() {
                pane_for_can_drop.read(cx).can_accept_file_drop(
                    target_id,
                    drag,
                    &can_drop_target,
                    window.modifiers(),
                    cx,
                )
            } else if let Some(paths) = value.downcast_ref::<ExternalPaths>() {
                external_pane_for_can_drop.read(cx).can_accept_external_drop(
                    paths.paths(),
                    &external_can_drop_target,
                    window.modifiers(),
                )
            } else {
                false
            }
        })
    })
}

#[cfg(test)]
mod icon_path_tests {
    use super::icon_path_for;

    #[test]
    fn directories_get_the_folder_icon_regardless_of_name() {
        assert_eq!(icon_path_for("src", "dir"), "icons/folder.svg");
        assert_eq!(icon_path_for("archive.zip", "dir"), "icons/folder.svg");
    }

    #[test]
    fn known_extensions_map_to_their_category_icon() {
        for ext in [
            "rs", "py", "js", "ts", "go", "c", "cpp", "h", "json", "toml", "yaml", "yml", "sh",
        ] {
            assert_eq!(
                icon_path_for(&format!("main.{ext}"), "file"),
                "icons/file-code.svg",
                "ext={ext}"
            );
        }
        for ext in ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"] {
            assert_eq!(
                icon_path_for(&format!("pic.{ext}"), "file"),
                "icons/file-image.svg",
                "ext={ext}"
            );
        }
        for ext in ["zip", "tar", "gz", "xz", "zst", "7z", "rar"] {
            assert_eq!(
                icon_path_for(&format!("bundle.{ext}"), "file"),
                "icons/file-archive.svg",
                "ext={ext}"
            );
        }
        for ext in ["md", "txt", "rst", "log"] {
            assert_eq!(
                icon_path_for(&format!("notes.{ext}"), "file"),
                "icons/file-text.svg",
                "ext={ext}"
            );
        }
    }

    #[test]
    fn unknown_or_missing_extension_falls_back_to_the_generic_file_icon() {
        assert_eq!(icon_path_for("Makefile", "file"), "icons/file.svg");
        assert_eq!(icon_path_for("data.bin", "file"), "icons/file.svg");
    }

    #[test]
    fn extension_matching_is_case_insensitive() {
        assert_eq!(icon_path_for("Main.RS", "file"), "icons/file-code.svg");
    }
}
