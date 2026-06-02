//! `TestAppContext`-based examples for the explorer page. These double as the
//! reference patterns documented in `docs/testing.md` / `CONTRIBUTING.md`:
//! build the view inside a test window, mutate it through `window.update`,
//! drive async work with the GPUI executor timer + `run_until_parked`, and
//! assert state with `window.read_with` — never `smol::Timer` / `tokio` sleeps,
//! which the GPUI scheduler does not track.

use std::time::Duration;

use gpui::{AppContext, TestAppContext, WindowHandle};
use gpui_component::input::InputState;
use gpui_component::resizable::ResizableState;
use nohrs_core::config;
use nohrs_services::fs::listing::FileEntryDto;

use super::types::{SortKey, StatusLevel, ViewMode};
use super::ExplorerPage;

/// Build a real `ExplorerPage` inside a test window. The sub-entities
/// (`ResizableState`, `InputState`) are window-bound, so the page is
/// constructed in the `add_window` build closure — the canonical pattern for
/// views whose dependencies need a `Window`.
fn new_explorer(cx: &mut TestAppContext) -> WindowHandle<ExplorerPage> {
    // gpui-component installs the `Theme` global and input/list subsystems its
    // widgets rely on; initialize it once before building any window.
    cx.update(gpui_component::init);
    cx.add_window(|window, cx| {
        let resizable = ResizableState::new(cx);
        let search_input = cx.new(|cx| InputState::new(window, cx));
        ExplorerPage::new(resizable, search_input, None, cx.focus_handle())
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
            assert!(page
                .filtered_entries
                .iter()
                .all(|entry| entry.name != ".hidden"));
            assert_eq!(page.filtered_entries.len(), 2);

            page.show_hidden = true;
            page.apply_filter();
            assert!(page
                .filtered_entries
                .iter()
                .any(|entry| entry.name == ".hidden"));
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
