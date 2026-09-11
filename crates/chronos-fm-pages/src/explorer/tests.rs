//! `TestAppContext`-based behavior tests for the explorer page: navigation and
//! history, search lifecycle, preview loading, sorting/filtering, column resize,
//! and status reporting. They also serve as the reference patterns documented in
//! `docs/testing.md` / `CONTRIBUTING.md`: build the view inside a test window,
//! mutate it through `window.update`, drive async work with the GPUI executor
//! timer + `run_until_parked`, and assert state with `window.read_with` — never
//! `smol::Timer` / `tokio` sleeps, which the GPUI scheduler does not track.

// Test fixtures write files directly; the synchronous-fs ban targets app code.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use gpui::{AppContext, Entity, TestAppContext, WindowHandle, point, px};
use gpui_component::input::InputState;
use gpui_component::resizable::ResizableState;
use chronos_fm_core::config;
use chronos_fm_core::errors::{Error, Result};
use chronos_fm_services::devices::{Device, DeviceBackend};
use chronos_fm_services::fs::listing::{FileEntryDto, ListResult};
use chronos_fm_services::fs::provider::FileSystemProvider;
use chronos_fm_services::search::exclusions::Excludes;
use chronos_fm_services::search::SearchService;
use chronos_fm_store::{KvStore, RedbKvStore, StoreLogConfig};

use chronos_fm_core::config::SplitDirection;

use super::types::{SortKey, StatusLevel, ViewMode};
use super::{ExplorerPage, ExplorerPane};

/// Build a real `ExplorerPane` inside a test window. The sub-entities
/// (`ResizableState`, `InputState`) are window-bound, so the page is
/// constructed in the `add_window` build closure — the canonical pattern for
/// views whose dependencies need a `Window`.
fn new_explorer(cx: &mut TestAppContext) -> WindowHandle<ExplorerPane> {
    // gpui-component installs the `Theme` global and input/list subsystems its
    // widgets rely on; initialize it once before building any window. The
    // clipboard global is registered too: the row/grid renderers consult it
    // for cut-row dimming (b3/b4), and the app registers it at startup.
    cx.update(gpui_component::init);
    cx.update(super::clipboard::init);
    cx.add_window(|window, cx| {
        let resizable = cx.new(|_| ResizableState::default());
        let search_input = cx.new(|cx| InputState::new(window, cx));
        ExplorerPane::new(resizable, search_input, None, cx.focus_handle())
    })
}

/// Builds an `ExplorerPane` rooted at `cwd` (a real directory, typically a
/// `tempfile::tempdir()`), for tests that exercise filesystem operations
/// (rename, copy/cut/paste, delete). Also registers the `FileClipboard`
/// global, which those operations read — note this **resets** the clipboard
/// to its empty default, so tests must set copy/cut state after calling it.
///
/// **Harness limitation:** the window's root is the pane itself, not a
/// `gpui_component::Root`. Rendering a live `Input` (inline rename, visible
/// search bar) across an `update` boundary panics in `Root::read`, because
/// the input element's `paint` requires a Root on the window. Tests that
/// start an inline rename must clear it (`commit_rename`/`cancel_rename`)
/// within the same `window.update` closure (see the rename and file_ops
/// tests); the production app wraps its window root in `Root::new`, so this
/// is purely a test-harness constraint.
pub(crate) fn new_explorer_for_tests(
    cx: &mut TestAppContext,
    cwd: &std::path::Path,
) -> WindowHandle<ExplorerPane> {
    cx.update(gpui_component::init);
    cx.update(super::clipboard::init);
    let cwd = cwd.to_string_lossy().to_string();
    cx.add_window(move |window, cx| {
        let resizable = cx.new(|_| ResizableState::default());
        let search_input = cx.new(|cx| InputState::new(window, cx));
        let mut pane = ExplorerPane::new(resizable, search_input, None, cx.focus_handle());
        pane.cwd = cwd;
        pane
    })
}

/// Build the split-view container (`ExplorerPage`), which owns its panes. No KV
/// store, so session save/restore is inert.
fn new_explorer_page(cx: &mut TestAppContext) -> WindowHandle<ExplorerPage> {
    cx.update(gpui_component::init);
    cx.update(super::clipboard::init);
    cx.add_window(|window, cx| {
        let resizable = cx.new(|_| ResizableState::default());
        ExplorerPage::new(resizable, None, None, false, window, cx)
    })
}

/// Build the container backed by a `store`, optionally restoring its session,
/// for the persistence round-trip tests.
fn new_explorer_page_with_store(
    cx: &mut TestAppContext,
    store: Arc<dyn KvStore>,
    restore_tabs: bool,
) -> WindowHandle<ExplorerPage> {
    cx.update(gpui_component::init);
    cx.update(super::clipboard::init);
    cx.add_window(|window, cx| {
        let resizable = cx.new(|_| ResizableState::default());
        ExplorerPage::new(resizable, None, Some(store), restore_tabs, window, cx)
    })
}

fn file(name: &str, kind: &str, size: u64) -> FileEntryDto {
    FileEntryDto {
        name: name.to_string(),
        path: format!("/tmp/{name}"),
        kind: kind.to_string(),
        size,
        modified: 0,
    }
}

#[gpui::test]
async fn explorer_starts_with_default_view_state(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .read_with(cx, |page, _cx| {
            assert!(page.sort_key == SortKey::Name);
            assert!(page.sort_asc);
            assert!(!page.show_hidden);
            assert!(page.view_mode == ViewMode::List);
        })
        .unwrap();
}

#[gpui::test]
async fn set_sort_key_toggles_direction_then_resets(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            // Re-selecting the active key flips the direction...
            page.set_sort_key(SortKey::Name);
            assert!(page.sort_key == SortKey::Name);
            assert!(!page.sort_asc);
            // ...and a new key resets to ascending.
            page.set_sort_key(SortKey::Size);
            assert!(page.sort_key == SortKey::Size);
            assert!(page.sort_asc);
        })
        .unwrap();
}

#[gpui::test]
async fn apply_config_ui_reflects_ui_section(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, cx| {
            let ui = config::Ui {
                default_sort: config::SortOrder::Size,
                show_hidden: true,
                icon_pack: "default".to_string(),
            };
            page.apply_config_ui(&ui, cx);
            assert!(page.show_hidden);
            assert!(page.sort_key == SortKey::Size);
        })
        .unwrap();
}

#[gpui::test]
async fn apply_filter_hides_dotfiles_until_enabled(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            page.entries = vec![
                file(".hidden", "file", 1),
                file("visible.txt", "file", 2),
                file("docs", "dir", 0),
            ];

            page.show_hidden = false;
            page.apply_filter();
            assert!(
                page.filtered_entries
                    .iter()
                    .all(|entry| entry.name != ".hidden")
            );
            assert_eq!(page.filtered_entries.len(), 2);

            page.show_hidden = true;
            page.apply_filter();
            assert!(
                page.filtered_entries
                    .iter()
                    .any(|entry| entry.name == ".hidden")
            );
            assert_eq!(page.filtered_entries.len(), 3);
        })
        .unwrap();
}

#[gpui::test]
async fn status_message_set_and_clear(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            page.set_status(StatusLevel::Error, "boom");
            assert_eq!(page.status_for_footer(), Some(("boom".to_string(), true)));
            page.clear_status();
            assert_eq!(page.status_for_footer(), None);
        })
        .unwrap();
}

#[gpui::test]
async fn open_preview_loads_text_off_thread(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("note.txt");
    std::fs::write(&file_path, "hello, gpui test world\n").unwrap();
    let path = file_path.to_string_lossy().to_string();

    let window = new_explorer(cx);
    window
        .update(cx, |page, window, cx| {
            page.open_preview(path.clone(), window, cx);
        })
        .unwrap();

    // `open_preview` reads the file on the background executor and applies the
    // result on the foreground thread. Drive both to completion: the executor
    // timer advances the test clock, then `run_until_parked` flushes the spawned
    // tasks. (Using `smol`/`tokio` sleeps here would not be tracked by the GPUI
    // scheduler and `run_until_parked` would return early.)
    cx.background_executor
        .timer(Duration::from_millis(50))
        .await;
    cx.run_until_parked();

    window
        .read_with(cx, |page, _cx| {
            assert_eq!(
                page.preview_text.as_deref(),
                Some("hello, gpui test world\n")
            );
            assert!(page.preview_editor.is_some());
        })
        .unwrap();
}

#[gpui::test]
async fn navigation_history_supports_back_forward_and_truncation(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("sub")).unwrap();
    std::fs::create_dir(root.join("other")).unwrap();
    std::fs::write(root.join("a.txt"), "x").unwrap();
    let root_s = root.to_string_lossy().to_string();
    let sub_s = root.join("sub").to_string_lossy().to_string();
    let other_s = root.join("other").to_string_lossy().to_string();

    let window = new_explorer(cx);
    window
        .update(cx, |page, window, cx| {
            page.change_dir(root_s.clone(), window, cx);
            assert!(page.entries.iter().any(|e| e.name == "a.txt"));
            assert!(page.entries.iter().any(|e| e.name == "sub"));

            // Navigating to the current directory is a no-op.
            let len = page.history.len();
            page.change_dir(root_s.clone(), window, cx);
            assert_eq!(page.history.len(), len);

            page.change_dir(sub_s.clone(), window, cx);
            assert_eq!(page.cwd, sub_s);

            page.go_back(window, cx);
            assert_eq!(page.cwd, root_s);
            page.go_forward(window, cx);
            assert_eq!(page.cwd, sub_s);

            // Back, then a new navigation drops the forward ("sub") entry.
            page.go_back(window, cx);
            page.change_dir(other_s.clone(), window, cx);
            assert_eq!(page.cwd, other_s);
            let cwd = page.cwd.clone();
            page.go_forward(window, cx);
            assert_eq!(page.cwd, cwd, "no forward history remains");
        })
        .unwrap();
}

#[gpui::test]
async fn activate_entry_routes_by_kind(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("folder")).unwrap();
    std::fs::write(root.join("file.txt"), "hi").unwrap();
    let folder = FileEntryDto {
        name: "folder".into(),
        path: root.join("folder").to_string_lossy().to_string(),
        kind: "dir".into(),
        size: 0,
        modified: 0,
    };
    let doc = FileEntryDto {
        name: "file.txt".into(),
        path: root.join("file.txt").to_string_lossy().to_string(),
        kind: "file".into(),
        size: 2,
        modified: 0,
    };

    let window = new_explorer(cx);
    window
        .update(cx, |page, window, cx| {
            page.activate_entry(folder.clone(), window, cx);
            assert_eq!(page.cwd, folder.path, "activating a dir navigates into it");
        })
        .unwrap();
    window
        .update(cx, |page, window, cx| {
            page.activate_entry(doc.clone(), window, cx);
            // open_preview records the target path synchronously before reading.
            assert_eq!(page.preview_path.as_deref(), Some(doc.path.as_str()));
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
async fn search_visibility_open_close_toggle(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, window, cx| {
            assert!(!page.search_visible);
            page.open_search(window, cx);
            assert!(page.search_visible);
            page.close_search(window, cx);
            assert!(!page.search_visible);
            page.toggle_search(window, cx);
            assert!(page.search_visible);
            page.toggle_search(window, cx);
            assert!(!page.search_visible);
        })
        .unwrap();
}

#[gpui::test]
async fn trigger_search_empty_clears_and_no_service_degrades(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, window, cx| {
            // Empty query clears results without raising an error.
            page.search_query = String::new();
            page.trigger_search(window, cx);
            assert!(page.search_results.is_none());
            assert!(page.status_for_footer().is_none());

            // A real query with no search service falls back to name filtering and
            // surfaces an error status.
            page.search_query = "needle".into();
            page.trigger_search(window, cx);
            assert!(page.search_results.is_none());
            let (_, is_error) = page.status_for_footer().expect("degraded status set");
            assert!(is_error);
        })
        .unwrap();
}

#[gpui::test]
async fn view_mode_and_match_toggles_flip_state(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, cx| {
            assert!(page.view_mode == ViewMode::List);
            page.set_view_mode(ViewMode::Grid, cx);
            assert!(page.view_mode == ViewMode::Grid);

            assert!(!page.match_case && !page.use_regex && !page.match_whole_word);
            page.toggle_match_case(cx);
            page.toggle_use_regex(cx);
            page.toggle_match_whole_word(cx);
            assert!(page.match_case && page.use_regex && page.match_whole_word);
        })
        .unwrap();
}

#[gpui::test]
async fn column_resize_applies_delta_with_min_clamp(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            let start = page.col_name_width;
            page.start_column_resize(0, point(px(100.0), px(0.0)));
            page.update_column_resize(point(px(150.0), px(0.0)));
            assert_eq!(page.col_name_width, start + 50.0);

            // A large negative drag clamps to the minimum column width.
            page.update_column_resize(point(px(-10_000.0), px(0.0)));
            assert_eq!(page.col_name_width, chronos_fm_core::config::MIN_COLUMN_WIDTH);

            page.stop_column_resize();
            assert!(page.resizing_column.is_none());
        })
        .unwrap();
}

#[gpui::test]
async fn item_sizes_track_entries_and_table_width_sums_columns(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            page.filtered_entries = vec![file("a", "file", 1), file("b", "file", 2)];
            page.update_item_sizes();
            assert_eq!(page.item_sizes.len(), 2);

            let expected = page.col_name_width
                + page.col_type_width
                + page.col_size_width
                + page.col_modified_width
                + page.col_action_width
                + chronos_fm_core::config::TABLE_HORIZONTAL_PADDING;
            assert_eq!(page.total_table_width(), expected);

            page.record_click(1, 2);
            assert!(page.last_click_info.is_some());
        })
        .unwrap();
}

#[gpui::test]
async fn reload_reports_error_for_unreadable_dir(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            page.cwd = "/nonexistent/chronos-fm/dir".to_string();
            page.loaded = false;
            page.reload();
            let (_, is_error) = page.status_for_footer().expect("error status set");
            assert!(is_error);
            assert!(page.entries.is_empty());
        })
        .unwrap();
}

#[gpui::test]
async fn explorer_page_starts_with_single_pane(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .read_with(cx, |page, _cx| {
            assert_eq!(page.pane_count(), 1);
            assert_eq!(page.active_index(), 0);
            assert!(!page.is_synced());
        })
        .unwrap();
}

#[gpui::test]
async fn split_opens_second_pane_then_reorients(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            assert_eq!(page.pane_count(), 2, "first split opens a second pane");
            assert_eq!(page.active_index(), 1, "the new pane becomes active");
            assert_eq!(page.direction(), SplitDirection::Vertical);

            // The opposite split shortcut flips orientation without adding a pane
            // (2-way cap, §3.1).
            page.split(SplitDirection::Horizontal, window, cx);
            assert_eq!(page.pane_count(), 2);
            assert_eq!(page.direction(), SplitDirection::Horizontal);
        })
        .unwrap();
}

#[gpui::test]
async fn close_pane_keeps_at_least_one(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            assert_eq!(page.pane_count(), 2);

            page.close_pane(1, window, cx);
            assert_eq!(page.pane_count(), 1);
            assert_eq!(page.active_index(), 0);

            // The final pane can never be closed.
            page.close_pane(0, window, cx);
            assert_eq!(page.pane_count(), 1);
        })
        .unwrap();
}

#[gpui::test]
async fn set_active_selects_pane_in_bounds(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            page.set_active(0, window, cx);
            assert_eq!(page.active_index(), 0);

            // Out-of-range indices are ignored rather than panicking.
            page.set_active(5, window, cx);
            assert_eq!(page.active_index(), 0);
        })
        .unwrap();
}

#[gpui::test]
async fn panes_navigate_independently_by_default(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("left")).unwrap();
    std::fs::create_dir(root.join("right")).unwrap();
    let left = root.join("left").to_string_lossy().to_string();
    let right = root.join("right").to_string_lossy().to_string();

    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            let pane0 = page.pane(0);
            let pane1 = page.pane(1);
            pane0.update(cx, |pane, cx| pane.change_dir(left.clone(), window, cx));
            pane1.update(cx, |pane, cx| pane.change_dir(right.clone(), window, cx));
        })
        .unwrap();
    cx.run_until_parked();
    window
        .read_with(cx, |page, cx| {
            assert_eq!(page.pane_cwd(0, cx).as_deref(), Some(left.as_str()));
            assert_eq!(page.pane_cwd(1, cx).as_deref(), Some(right.as_str()));
        })
        .unwrap();
}

#[gpui::test]
async fn synced_panes_mirror_navigation(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("shared")).unwrap();
    let shared = root.join("shared").to_string_lossy().to_string();

    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            let synced = config::Explorer {
                split_direction: SplitDirection::Vertical,
                synced_panes: true,
                restore_tabs: true,
            };
            page.apply_config_explorer(&synced, cx);
            assert!(page.is_synced());

            let pane0 = page.pane(0);
            pane0.update(cx, |pane, cx| pane.change_dir(shared.clone(), window, cx));
        })
        .unwrap();
    // Navigation events are delivered on the next effect flush; mirror happens there.
    cx.run_until_parked();
    window
        .read_with(cx, |page, cx| {
            assert_eq!(page.pane_cwd(0, cx).as_deref(), Some(shared.as_str()));
            assert_eq!(
                page.pane_cwd(1, cx).as_deref(),
                Some(shared.as_str()),
                "sibling pane mirrors the active pane's path"
            );
        })
        .unwrap();
}

#[gpui::test]
async fn synced_navigation_clears_stale_search_state(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("shared")).unwrap();
    let shared = root.join("shared").to_string_lossy().to_string();

    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            let synced = config::Explorer {
                split_direction: SplitDirection::Vertical,
                synced_panes: true,
                restore_tabs: true,
            };
            page.apply_config_explorer(&synced, cx);

            // Leave the mirrored pane with a stale filter and results from a
            // prior search so the reset is actually exercised (not vacuous).
            page.pane(1).update(cx, |pane, _cx| {
                pane.search_query = "stale".to_string();
                pane.search_visible = true;
                pane.search_results = Some(Vec::new());
            });
            page.pane(0)
                .update(cx, |pane, cx| pane.change_dir(shared.clone(), window, cx));
        })
        .unwrap();
    cx.run_until_parked();
    window
        .read_with(cx, |page, cx| {
            let pane1 = page.pane(1);
            let pane1 = pane1.read(cx);
            assert!(
                pane1.search_query.is_empty(),
                "stale filter cleared on sync"
            );
            assert!(!pane1.search_visible);
            assert!(pane1.search_results.is_none());
        })
        .unwrap();
}

#[gpui::test]
async fn split_pane_inherits_applied_ui_config(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            let ui = config::Ui {
                default_sort: config::SortOrder::Size,
                show_hidden: true,
                icon_pack: "default".to_string(),
            };
            page.apply_config_ui(&ui, cx);

            // A pane opened by a later split should pick up the applied config
            // rather than reverting to pane defaults.
            page.split(SplitDirection::Vertical, window, cx);
            let pane1 = page.pane(1);
            let pane1 = pane1.read(cx);
            assert!(pane1.show_hidden, "new pane inherits show_hidden");
            assert!(pane1.sort_key == SortKey::Size, "new pane inherits sort");
        })
        .unwrap();
}

#[gpui::test]
async fn root_pane_shows_sidebar_but_split_pane_hides_it(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            assert!(
                page.pane(0).read(cx).sidebar_visible,
                "the root pane shows its sidebar by default (§2)"
            );

            page.split(SplitDirection::Vertical, window, cx);
            assert!(
                !page.pane(1).read(cx).sidebar_visible,
                "a pane opened by a split starts with the sidebar collapsed"
            );
            assert!(
                page.pane(0).read(cx).sidebar_visible,
                "splitting does not disturb the original pane's sidebar"
            );
        })
        .unwrap();
}

#[gpui::test]
async fn toggle_sidebar_is_independent_per_pane(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            let pane0 = page.pane(0); // sidebar visible (root)
            let pane1 = page.pane(1); // sidebar hidden (split)

            pane1.update(cx, |pane, cx| pane.toggle_sidebar(cx));
            assert!(pane1.read(cx).sidebar_visible, "pane 1 toggled on");
            assert!(
                pane0.read(cx).sidebar_visible,
                "toggling pane 1 leaves pane 0 untouched"
            );

            pane0.update(cx, |pane, cx| pane.toggle_sidebar(cx));
            assert!(!pane0.read(cx).sidebar_visible, "pane 0 toggled off");
            assert!(pane1.read(cx).sidebar_visible, "pane 1 unchanged");
        })
        .unwrap();
}

#[gpui::test]
async fn close_pane_keeps_subscriptions_aligned(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let shared = dir.path().join("shared");
    std::fs::create_dir(&shared).unwrap();
    let shared = shared.to_string_lossy().to_string();

    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            assert_eq!(page.pane_count(), 2);
            assert_eq!(page.subscription_count(), 2, "one subscription per pane");

            // Closing index 0 must drop the subscription at the same index so the
            // Vec stays aligned with the surviving pane.
            page.close_pane(0, window, cx);
            assert_eq!(page.pane_count(), 1);
            assert_eq!(page.subscription_count(), 1);

            // Reuse the container: split again and enable sync. If the
            // subscription Vec had desynced, the mirror below would target the
            // wrong pane (or none).
            page.split(SplitDirection::Vertical, window, cx);
            assert_eq!(page.subscription_count(), 2);
            let synced = config::Explorer {
                split_direction: SplitDirection::Vertical,
                synced_panes: true,
                restore_tabs: true,
            };
            page.apply_config_explorer(&synced, cx);
            page.pane(0)
                .update(cx, |pane, cx| pane.change_dir(shared.clone(), window, cx));
        })
        .unwrap();
    cx.run_until_parked();
    window
        .read_with(cx, |page, cx| {
            assert_eq!(
                page.pane_cwd(1, cx).as_deref(),
                Some(shared.as_str()),
                "the sibling still mirrors after a close/split cycle"
            );
        })
        .unwrap();
}

#[gpui::test]
async fn new_tab_appends_and_activates(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            assert_eq!(page.tab_count(0), 1);
            page.test_new_tab(0, window, cx);
            assert_eq!(page.tab_count(0), 2, "a new tab is appended");
            assert_eq!(page.active_tab(0), 1, "the new tab becomes active");
            // A new tab defaults to home (§4).
            if let Ok(home) = std::env::var("HOME") {
                assert_eq!(page.tab_cwd(0, 1, cx).as_deref(), Some(home.as_str()));
            }

            // Switching back to the first tab makes it active again.
            page.test_activate_tab(0, 0, window, cx);
            assert_eq!(page.active_tab(0), 0);
        })
        .unwrap();
}

#[gpui::test]
async fn close_tab_keeps_pane_when_others_remain(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.test_new_tab(0, window, cx);
            page.test_new_tab(0, window, cx);
            assert_eq!(page.tab_count(0), 3);
            page.test_close_tab(0, 1, window, cx);
            assert_eq!(
                page.tab_count(0),
                2,
                "closing a non-last tab keeps the pane"
            );
            assert_eq!(page.pane_count(), 1);
        })
        .unwrap();
}

#[gpui::test]
async fn closing_last_tab_closes_its_pane(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            page.split(SplitDirection::Vertical, window, cx);
            assert_eq!(page.pane_count(), 2);
            assert_eq!(page.tab_count(1), 1);

            // Closing pane 1's only tab closes the pane (§3.1 / §4).
            page.test_close_tab(1, 0, window, cx);
            assert_eq!(page.pane_count(), 1);

            // The final tab of the final pane is never closed.
            page.test_close_tab(0, 0, window, cx);
            assert_eq!(page.pane_count(), 1);
            assert_eq!(page.tab_count(0), 1);
        })
        .unwrap();
}

#[gpui::test]
async fn reorder_tab_swaps_order_and_follows_active(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    let first = first.to_string_lossy().to_string();
    let second = second.to_string_lossy().to_string();

    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            // Tab 0 -> first, tab 1 (new, active) -> second.
            page.pane(0)
                .update(cx, |pane, _cx| pane.cwd = first.clone());
            page.test_new_tab(0, window, cx);
            page.pane(0)
                .update(cx, |pane, _cx| pane.cwd = second.clone());
            assert_eq!(page.tab_cwd(0, 0, cx).as_deref(), Some(first.as_str()));
            assert_eq!(page.tab_cwd(0, 1, cx).as_deref(), Some(second.as_str()));
            assert_eq!(page.active_tab(0), 1);

            // Drag tab 0 to position 1; the active tab (second) follows its entity.
            page.test_reorder_tab(0, 0, 1, cx);
            assert_eq!(page.tab_cwd(0, 0, cx).as_deref(), Some(second.as_str()));
            assert_eq!(page.tab_cwd(0, 1, cx).as_deref(), Some(first.as_str()));
            assert_eq!(page.active_tab(0), 0, "the active tab follows the reorder");
        })
        .unwrap();
}

#[gpui::test]
async fn subscriptions_track_every_tab(cx: &mut TestAppContext) {
    let window = new_explorer_page(cx);
    window
        .update(cx, |page, window, cx| {
            assert_eq!(page.subscription_count(), 1);
            page.test_new_tab(0, window, cx);
            assert_eq!(page.subscription_count(), 2, "one subscription per tab");
            page.split(SplitDirection::Vertical, window, cx);
            assert_eq!(page.subscription_count(), 3, "the split's pane adds a tab");
            page.test_close_tab(0, 1, window, cx);
            assert_eq!(
                page.subscription_count(),
                2,
                "closing a tab drops its subscription"
            );
        })
        .unwrap();
}

#[gpui::test]
async fn tabs_round_trip_through_the_store(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let alpha = dir.path().join("alpha");
    std::fs::create_dir(&alpha).unwrap();
    let alpha = alpha.to_string_lossy().to_string();
    let store: Arc<dyn KvStore> =
        Arc::new(RedbKvStore::open_in_memory(&StoreLogConfig::default()).unwrap());

    // First session: open a second tab and navigate it, then let the debounced
    // save fire and write to the store.
    let window = new_explorer_page_with_store(cx, store.clone(), true);
    window
        .update(cx, |page, window, cx| {
            page.test_new_tab(0, window, cx);
            page.pane(0)
                .update(cx, |pane, cx| pane.change_dir(alpha.clone(), window, cx));
        })
        .unwrap();
    // Drive the debounce timer past `SAVE_DEBOUNCE`, then flush the spawned save.
    cx.background_executor
        .timer(Duration::from_millis(600))
        .await;
    cx.run_until_parked();

    // Second session with the same store restores both tabs.
    let restored = new_explorer_page_with_store(cx, store.clone(), true);
    restored
        .read_with(cx, |page, cx| {
            assert_eq!(page.tab_count(0), 2, "both tabs are restored");
            assert_eq!(page.tab_cwd(0, 1, cx).as_deref(), Some(alpha.as_str()));
        })
        .unwrap();
}

#[gpui::test]
async fn restore_disabled_ignores_saved_session(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let beta = dir.path().join("beta");
    std::fs::create_dir(&beta).unwrap();
    let beta = beta.to_string_lossy().to_string();
    let store: Arc<dyn KvStore> =
        Arc::new(RedbKvStore::open_in_memory(&StoreLogConfig::default()).unwrap());

    let window = new_explorer_page_with_store(cx, store.clone(), true);
    window
        .update(cx, |page, window, cx| {
            page.test_new_tab(0, window, cx);
            page.pane(0)
                .update(cx, |pane, cx| pane.change_dir(beta.clone(), window, cx));
        })
        .unwrap();
    cx.background_executor
        .timer(Duration::from_millis(600))
        .await;
    cx.run_until_parked();

    // With restore disabled, a fresh session starts with a single default tab.
    let fresh = new_explorer_page_with_store(cx, store.clone(), false);
    fresh
        .read_with(cx, |page, _cx| {
            assert_eq!(page.tab_count(0), 1, "restore disabled: no extra tabs");
            assert_eq!(page.pane_count(), 1);
        })
        .unwrap();
}

#[gpui::test]
async fn selection_single_toggle_range_and_paths(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            page.filtered_entries = vec![
                file("a", "file", 1),
                file("b", "file", 2),
                file("c", "file", 3),
                file("d", "file", 4),
            ];

            // Single selection re-anchors and becomes active.
            page.select_single(1);
            assert!(page.is_selected(1));
            assert_eq!(page.selection.len(), 1);
            assert_eq!(page.active_index, Some(1));

            // Shift range spans from the anchor (row 1) to row 3 inclusive.
            page.select_range_to(3);
            assert_eq!(
                page.selection.iter().copied().collect::<Vec<_>>(),
                vec![1, 2, 3]
            );
            assert_eq!(page.active_index, Some(3));

            // Cmd/Ctrl toggle removes one without clearing the rest and adds
            // another, re-anchoring at the toggled row.
            page.toggle_select(2);
            assert!(!page.is_selected(2) && page.is_selected(1) && page.is_selected(3));
            page.toggle_select(0);
            assert!(page.is_selected(0));

            // `selected_paths` follows row order (rows 0, 1, 3).
            assert_eq!(page.selected_paths(), vec!["/tmp/a", "/tmp/b", "/tmp/d"]);
        })
        .unwrap();
}

#[gpui::test]
async fn selection_all_clear_and_arrow_navigation(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            page.filtered_entries = vec![
                file("a", "file", 1),
                file("b", "file", 2),
                file("c", "file", 3),
            ];

            page.select_all();
            assert_eq!(page.selection.len(), 3);
            assert_eq!(page.active_index, Some(2));

            page.clear_selection();
            assert!(page.selection.is_empty());
            assert_eq!(page.active_index, None);
            assert_eq!(page.selection_anchor, None);

            // Arrow-down from an empty selection lands on the first row.
            page.move_active(1, false);
            assert_eq!(page.active_index, Some(0));
            page.move_active(1, false);
            assert_eq!(page.active_index, Some(1));

            // Shift+down extends the range from the anchor; up is clamped at 0.
            page.move_active(1, true);
            assert_eq!(
                page.selection.iter().copied().collect::<Vec<_>>(),
                vec![1, 2]
            );
            page.select_single(0);
            page.move_active(-1, false);
            assert_eq!(page.active_index, Some(0));
        })
        .unwrap();
}

#[gpui::test]
async fn apply_filter_resets_stale_selection(cx: &mut TestAppContext) {
    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, _cx| {
            page.entries = vec![file("a", "file", 1), file("b", "file", 2)];
            page.apply_filter();
            page.select_all();
            assert!(!page.selection.is_empty());

            // Re-filtering rebuilds the row set, so the selection must reset to
            // avoid pointing at stale indices.
            page.apply_filter();
            assert!(page.selection.is_empty());
            assert_eq!(page.active_index, None);
        })
        .unwrap();
}

/// In-memory `DeviceBackend` for the Devices panel click-chain tests (T008):
/// `mount` always succeeds with a fixed path, so the wiring the sidebar row
/// click runs (mount → navigate) is exercised against a mock instead of a
/// live D-Bus connection (ticket verification: `FakeBackend`-style coverage).
struct DevicesFakeBackend {
    mounted_at: PathBuf,
}

#[async_trait::async_trait]
impl DeviceBackend for DevicesFakeBackend {
    async fn list(&self) -> anyhow::Result<Vec<Device>> {
        Ok(vec![])
    }
    async fn mount(&self, _object_path: &str) -> anyhow::Result<PathBuf> {
        Ok(self.mounted_at.clone())
    }
    async fn unmount(&self, _object_path: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn eject(&self, _object_path: &str, _drive_object_path: Option<&str>) -> anyhow::Result<()> {
        Ok(())
    }
}

#[gpui::test]
async fn navigate_to_path_moves_pane_and_clears_search_without_window(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let a = a.to_string_lossy().to_string();
    let b = b.to_string_lossy().to_string();

    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, cx| {
            page.cwd = a.clone();
            page.search_query = "stale".to_string();
            page.search_visible = true;
            page.search_results = Some(Vec::new());

            // `navigate_to_path` is the target of the devices mount callback
            // (T008): it navigates without a `Window` and clears search state.
            page.navigate_to_path(b.clone(), cx);
            assert_eq!(page.cwd, b);
            assert!(page.search_query.is_empty(), "stale filter cleared");
            assert!(!page.search_visible);
            assert!(page.search_results.is_none());

            // Same-path navigation is a no-op (no duplicate history entry).
            let len = page.history.len();
            page.navigate_to_path(b.clone(), cx);
            assert_eq!(page.history.len(), len);
        })
        .unwrap();
}

#[gpui::test]
async fn device_mount_then_navigate_moves_pane_into_mount_path(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_string_lossy().to_string();
    let mount_dir = dir.path().join("mnt");
    std::fs::create_dir(&mount_dir).unwrap();
    let mount_path = mount_dir.to_string_lossy().to_string();

    // Register the live backend on the store, as `spawn_device_hotplug_watcher`
    // does in the real app; the sidebar click handler reads it from there.
    let backend: Arc<dyn DeviceBackend> = Arc::new(DevicesFakeBackend {
        mounted_at: mount_dir.clone(),
    });
    cx.update(|cx| {
        let store = chronos_fm_ui::devices_store::DeviceStore {
            devices: Vec::new(),
            last_error: None,
            backend: Some(backend.clone()),
            mounting: std::collections::HashSet::new(),
            unmounting: std::collections::HashSet::new(),
            ejecting: std::collections::HashSet::new(),
        };
        cx.set_global(store);
    });

    let window = new_explorer(cx);
    window
        .update(cx, |page, _window, cx| {
            page.cwd = root.clone();
            // Exactly what `render_devices_section`'s unmounted-row `.on_click`
            // runs: mount via the store, then navigate the pane into the mount
            // point. The callback has only `&mut App` (no `Window`).
            let pane = cx.entity();
            chronos_fm_ui::devices_store::DeviceStore::mount_and_navigate(
                cx,
                backend.clone(),
                "/org/freedesktop/UDisks2/block_devices/sdb1".to_string(),
                move |cx, path| {
                    let path = path.to_string_lossy().to_string();
                    pane.update(cx, |pane, cx| pane.navigate_to_path(path, cx));
                },
            );
        })
        .unwrap();

    cx.run_until_parked();
    window
        .read_with(cx, |page, _cx| {
            assert_eq!(page.cwd, mount_path, "pane follows the mounted volume");
        })
        .unwrap();
}

#[test]
fn context_menu_for_directory_has_no_file_target() {
    let state = super::context_menu::ContextMenuState::for_directory(point(px(10.0), px(10.0)));
    assert!(state.file_path.is_none());
    assert!(state.index.is_none());
    assert!(state.apps.is_empty());
    assert!(state.mime_type.is_empty());
}

#[test]
fn context_menu_for_file_sets_path_and_index() {
    let state = super::context_menu::ContextMenuState::for_file(
        "/tmp/a.txt",
        3,
        false,
        point(px(10.0), px(10.0)),
    );
    assert_eq!(state.file_path.as_deref(), Some("/tmp/a.txt"));
    assert_eq!(state.index, Some(3));
    assert!(!state.is_dir, "a file row is not a directory (T055)");
}

#[test]
fn context_menu_for_file_carries_is_dir_for_folders() {
    // T055: the menu shows "Open Terminal Here" on folder rows only; `is_dir`
    // is threaded from the listing's `kind == "dir"`, never a filesystem stat.
    let state = super::context_menu::ContextMenuState::for_file(
        "/tmp/a",
        0,
        true,
        point(px(10.0), px(10.0)),
    );
    assert!(state.is_dir);
    let non_dir = super::context_menu::ContextMenuState::for_file(
        "/tmp/a.txt",
        0,
        false,
        point(px(10.0), px(10.0)),
    );
    assert!(!non_dir.is_dir);
}

/// In-memory `FileSystemProvider` for the T021 regression tests: counts
/// `list_dir` invocations and returns a fixed listing, so a test can assert
/// that a provider-backed pane actually polls its provider after being wired
/// up — the bug was exactly "provider connected but never asked".
/// `pub(crate)` so the S3 page's tests can drive the same fake across the
/// S3Page → pane boundary (T021).
pub(crate) struct CountingProvider {
    list_dir_calls: AtomicUsize,
    entries: Vec<FileEntryDto>,
    fail: bool,
}

impl CountingProvider {
    /// A provider that succeeds with `entries` and counts `list_dir` calls.
    pub(crate) fn new(entries: Vec<FileEntryDto>) -> Self {
        Self {
            list_dir_calls: AtomicUsize::new(0),
            entries,
            fail: false,
        }
    }

    /// A provider whose `list_dir` always fails (error-banner path).
    pub(crate) fn failing() -> Self {
        Self {
            list_dir_calls: AtomicUsize::new(0),
            entries: Vec::new(),
            fail: true,
        }
    }

    /// How many times `list_dir` has been invoked so far.
    pub(crate) fn call_count(&self) -> usize {
        self.list_dir_calls.load(Ordering::SeqCst)
    }
}

impl FileSystemProvider for CountingProvider {
    fn root_label(&self) -> String {
        "fake".into()
    }

    fn list_dir(&self, _path: &str, _limit: usize, _cursor: Option<&str>) -> Result<ListResult> {
        self.list_dir_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(Error::Other("fake list_dir failed".into()));
        }
        Ok(ListResult {
            entries: self.entries.clone(),
            next_cursor: None,
        })
    }

    fn read_file(&self, _path: &str) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn metadata(&self, _path: &str) -> Result<FileEntryDto> {
        self.entries
            .first()
            .cloned()
            .ok_or_else(|| Error::Other("no entries".into()))
    }

    fn create_dir(&self, _parent: &str, _name: &str) -> Result<()> {
        Ok(())
    }

    fn delete(&self, _path: &str, _is_dir: bool) -> Result<()> {
        Ok(())
    }

    fn rename(&self, _from: &str, _to: &str) -> Result<()> {
        Ok(())
    }

    fn write_file(&self, _path: &str, _content: &[u8]) -> Result<()> {
        Ok(())
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn scheme(&self) -> &str {
        "fake"
    }
}

#[gpui::test]
async fn provider_backed_pane_lists_immediately_after_set_provider(cx: &mut TestAppContext) {
    // T021 regression: wiring a provider and setting `cwd` must poll the
    // provider right away (the S3 connect callback now calls `reload_provider`
    // explicitly), instead of leaving `loaded = false` for a render path that
    // no-ops for provider-backed panes.
    let window = new_explorer(cx);
    let provider = Arc::new(CountingProvider::new(vec![
        file("bucket-a", "dir", 0),
        file("bucket-b", "dir", 0),
    ]));
    window
        .update(cx, |page, window, cx| {
            page.set_provider(provider.clone());
            page.cwd = "s3://rustfs@".to_string();
            page.reload_provider(window, cx);
        })
        .unwrap();

    cx.background_executor
        .timer(Duration::from_millis(50))
        .await;
    cx.run_until_parked();

    assert_eq!(provider.call_count(), 1, "provider polled exactly once");
    window
        .read_with(cx, |page, _cx| {
            assert!(page.loaded);
            assert_eq!(page.entries.len(), 2);
            assert!(page.entries.iter().any(|e| e.name == "bucket-a"));
        })
        .unwrap();

    // `loaded` is set synchronously by `reload_provider`, so a subsequent
    // render's `ensure_loaded` must not poll again (no double-load).
    window
        .update(cx, |page, window, cx| {
            page.ensure_loaded(window, cx);
        })
        .unwrap();
    assert_eq!(
        provider.call_count(),
        1,
        "ensure_loaded after a completed load must not re-poll"
    );
}

#[gpui::test]
async fn ensure_loaded_routes_provider_pane_to_reload_provider(cx: &mut TestAppContext) {
    // T021: `loaded = false` on a provider-backed pane must not silently
    // short-circuit through `reload()` (which returns early when a provider is
    // set); the render path's `ensure_loaded` has to route into the async
    // provider load.
    let window = new_explorer(cx);
    let provider = Arc::new(CountingProvider::new(vec![]));
    window
        .update(cx, |page, window, cx| {
            page.set_provider(provider.clone());
            page.cwd = "s3://rustfs@".to_string();
            page.loaded = false;
            page.ensure_loaded(window, cx);
        })
        .unwrap();

    cx.background_executor
        .timer(Duration::from_millis(50))
        .await;
    cx.run_until_parked();

    assert_eq!(
        provider.call_count(),
        1,
        "ensure_loaded polls the provider instead of no-oping"
    );
}

#[gpui::test]
async fn provider_listing_error_surfaces_in_status(cx: &mut TestAppContext) {
    // T021: a provider `list_dir` failure must reach the UI as an error status
    // (rendered as an inline banner by the listing) rather than looking like
    // an empty directory.
    let window = new_explorer(cx);
    let provider = Arc::new(CountingProvider::failing());
    window
        .update(cx, |page, window, cx| {
            page.set_provider(provider.clone());
            page.cwd = "s3://rustfs@".to_string();
            page.reload_provider(window, cx);
        })
        .unwrap();

    cx.background_executor
        .timer(Duration::from_millis(50))
        .await;
    cx.run_until_parked();

    window
        .read_with(cx, |page, _cx| {
            let (text, is_error) = page.status_for_footer().expect("error status set");
            assert!(is_error);
            assert!(text.contains("fake list_dir failed"));
            assert!(page.entries.is_empty());
        })
        .unwrap();
}

#[gpui::test]
async fn search_service_arrives_after_window_opens(cx: &mut TestAppContext) {
    // T058 structural check: the search service (index open + recursive
    // `$HOME` watcher — ~35 s on a real home) must NOT be constructed on the
    // window-building path. The page builds with no service at all, and the
    // finished service is injected afterwards via `set_search_service`, which
    // propagates it to every pane — existing ones and tabs opened later.
    let window = new_explorer_page(cx);

    // Window opened without a service: panes are honest about the pending
    // background initialization instead of looking like a working search.
    window
        .read_with(cx, |page, cx| {
            let pane = page.pane(0);
            assert!(pane.read(cx).search_service.is_none());
            assert!(pane.read(cx).search_initializing);
        })
        .unwrap();

    // Background initialization "completes": a real service rooted at a small
    // tempdir home, never the real `$HOME` (that is the 35 s tree).
    let tmp = tempfile::tempdir().unwrap();
    let service =
        SearchService::new_with_home(tmp.path().to_path_buf(), Excludes::default())
            .expect("search service builds against a tempdir home");

    window
        .update(cx, |page, _window, cx| {
            page.set_search_service(Some(Arc::new(service)), cx);
        })
        .unwrap();

    // Existing pane now has the service and is no longer "initializing".
    window
        .read_with(cx, |page, cx| {
            let pane = page.pane(0);
            assert!(pane.read(cx).search_service.is_some());
            assert!(!pane.read(cx).search_initializing);
        })
        .unwrap();

    // A tab opened after injection inherits the service through the shared
    // slot — the pane factory must not snapshot `None` at page construction.
    window
        .update(cx, |page, window, cx| {
            page.test_new_tab(0, window, cx);
        })
        .unwrap();
    window
        .read_with(cx, |page, cx| {
            assert_eq!(page.tab_count(0), 2);
            let new_tab = page.pane(0); // the freshly added tab is active
            assert!(new_tab.read(cx).search_service.is_some());
            assert!(!new_tab.read(cx).search_initializing);
        })
        .unwrap();

    // Failure path: initialization reporting `None` flips panes out of
    // "initializing" so the UI shows "unavailable" rather than "starting"
    // forever.
    window
        .update(cx, |page, _window, cx| {
            page.set_search_service(None, cx);
        })
        .unwrap();
    window
        .read_with(cx, |page, cx| {
            let pane = page.pane(0);
            assert!(pane.read(cx).search_service.is_none());
            assert!(!pane.read(cx).search_initializing);
        })
        .unwrap();
}

// -- T054: window-level undo/redo --

/// Renames the pane's first visible row through the real inline-rename path
/// (begin_rename → set value → commit) inside the page's update context, so
/// the pane's `PaneEvent::Undoable` reaches the page's subscription
/// synchronously and lands on the window-level stack.
fn rename_first_row(
    pane: &Entity<ExplorerPane>,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<ExplorerPage>,
    new_name: &str,
) {
    pane.update(cx, |pane, cx| {
        pane.begin_rename(0, window, cx);
        let (_, input) = pane.renaming.clone().expect("inline rename started");
        input.update(cx, |state, cx| {
            state.set_value(new_name.to_string(), window, cx)
        });
        pane.commit_rename(window, cx);
    });
}

#[gpui::test]
async fn page_undo_redo_rename_round_trips(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("old.txt"), "x").unwrap();
    let window = new_explorer_page(cx);

    // The mutation runs in its own update; the pane's `PaneEvent::Undoable`
    // lands on the page's stack when gpui flushes effects at the end of it.
    window
        .update(cx, |page, window, cx| {
            let pane = page.pane(0);
            pane.update(cx, |pane, _cx| {
                pane.cwd = dir.path().to_string_lossy().to_string();
                pane.reload();
            });
            rename_first_row(&pane, window, cx, "new.txt");
        })
        .unwrap();
    assert!(dir.path().join("new.txt").exists());

    window
        .update(cx, |page, _window, cx| {
            assert!(page.can_undo(), "a rename records an undo entry on the page");
            assert!(!page.can_redo());
            page.undo(cx);
        })
        .unwrap();
    assert!(dir.path().join("old.txt").exists(), "undo restores the original name");
    assert!(!dir.path().join("new.txt").exists());

    window
        .update(cx, |page, _window, cx| {
            assert!(page.can_redo());
            page.redo(cx);
        })
        .unwrap();
    assert!(!dir.path().join("old.txt").exists(), "redo re-applies the rename");
    assert!(dir.path().join("new.txt").exists());
}

#[gpui::test]
async fn page_undo_paste_copy_removes_the_copy_and_redo_recopies(cx: &mut TestAppContext) {
    let src_dir = tempfile::tempdir().unwrap();
    let dst_dir = tempfile::tempdir().unwrap();
    std::fs::write(src_dir.path().join("a.txt"), "hello").unwrap();
    let window = new_explorer_page(cx);

    window
        .update(cx, |page, _window, cx| {
            let pane = page.pane(0);
            pane.update(cx, |pane, _cx| {
                pane.cwd = dst_dir.path().to_string_lossy().to_string();
                pane.reload();
            });
            pane.update(cx, |_pane, cx| {
                super::clipboard::set_copy(
                    vec![src_dir.path().join("a.txt").to_string_lossy().to_string()],
                    cx,
                );
            });
            pane.update(cx, |pane, cx| pane.paste_clipboard(cx));
        })
        .unwrap();
    assert!(dst_dir.path().join("a.txt").exists(), "the paste copied the file");

    window
        .update(cx, |page, _window, cx| {
            assert!(page.can_undo(), "a paste records one undo entry on the page");
            page.undo(cx);
        })
        .unwrap();
    assert!(
        !dst_dir.path().join("a.txt").exists(),
        "undo of a copy removes the destination"
    );
    assert!(src_dir.path().join("a.txt").exists(), "the source survives undo");

    window
        .update(cx, |page, _window, cx| page.redo(cx))
        .unwrap();
    assert!(dst_dir.path().join("a.txt").exists(), "redo re-copies the file");
}

#[gpui::test]
async fn page_undo_new_folder_deletes_it(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let window = new_explorer_page(cx);

    window
        .update(cx, |page, window, cx| {
            let pane = page.pane(0);
            pane.update(cx, |pane, _cx| {
                pane.cwd = dir.path().to_string_lossy().to_string();
                pane.reload();
            });
            pane.update(cx, |pane, cx| {
                pane.new_folder(window, cx);
                // Drop the inline rename before repaint: the pane-rooted page
                // harness has no `gpui_component::Root`, and a live `Input`
                // across a repaint boundary panics (same pattern as the
                // file_ops new-folder test).
                pane.cancel_rename(window, cx);
            });
        })
        .unwrap();
    assert!(dir.path().join("New Folder").is_dir());

    window
        .update(cx, |page, _window, cx| {
            assert!(page.can_undo(), "new folder records an undo entry");
            page.undo(cx);
        })
        .unwrap();

    assert!(
        !dir.path().join("New Folder").exists(),
        "undo deletes the created folder"
    );
}

#[gpui::test]
async fn page_undo_is_window_scoped_across_panes(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("old.txt"), "x").unwrap();
    let window = new_explorer_page(cx);

    window
        .update(cx, |page, window, cx| {
            // The op happens in pane 1; the stack lives on the page (§1.3: one
            // stack across panes), so undo still reverses it from anywhere.
            page.split(SplitDirection::Vertical, window, cx);
            let second = page.pane(1);
            second.update(cx, |pane, _cx| {
                pane.cwd = dir.path().to_string_lossy().to_string();
                pane.reload();
            });
            rename_first_row(&second, window, cx, "new.txt");
        })
        .unwrap();
    assert!(dir.path().join("new.txt").exists());

    window
        .update(cx, |page, _window, cx| {
            assert!(page.can_undo(), "a pane-1 rename lands on the page stack");
            page.undo(cx);
        })
        .unwrap();

    assert!(dir.path().join("old.txt").exists());
    assert!(!dir.path().join("new.txt").exists());
}
