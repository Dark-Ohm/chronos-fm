//! Keybinding behavior tests for the file-ops shortcuts (T049).
//!
//! These dispatch real keystrokes (`TestAppContext::simulate_keystrokes`)
//! through a **Root-wrapped** window — the same shape as the production app
//! (`chronos-fm/src/app.rs` wraps its view in `gpui_component::Root`) — so the
//! pane's `on_key_down` handler runs exactly as it does live, including the
//! input-consumption contract (keys typed into a focused input never reach the
//! pane). The pane-rooted harness used elsewhere can neither dispatch keys (it
//! never paints a dispatch tree) nor open the Delete-confirm dialog (which
//! requires a `Root`).

// Test fixtures write files directly; the synchronous-fs ban targets app code.
#![allow(clippy::disallowed_methods)]

use std::fs;

use gpui::{AppContext, Entity, TestAppContext, WindowHandle};
use gpui_component::input::InputState;
use gpui_component::resizable::ResizableState;
use gpui_component::{Root, WindowExt};
use tempfile::tempdir;

use super::ExplorerPane;
use super::clipboard::{self, ClipboardMode};

/// Builds a Root-wrapped window rooted at `cwd`. Rendering happens on the next
/// park (the first paint also focuses the pane — `view::render` calls
/// `cx.focus_self` on first render), so tests call `cx.run_until_parked()`
/// after building before dispatching keys.
fn root_window(cx: &mut TestAppContext, cwd: &std::path::Path) -> WindowHandle<Root> {
    cx.update(gpui_component::init);
    cx.update(super::clipboard::init);
    let cwd = cwd.to_string_lossy().to_string();
    cx.add_window(move |window, cx| {
        let resizable = cx.new(|_| ResizableState::default());
        let search_input = cx.new(|cx| InputState::new(window, cx));
        let pane = cx.new(|cx| {
            let mut pane = ExplorerPane::new(resizable, search_input, None, cx.focus_handle());
            pane.cwd = cwd.clone();
            pane
        });
        Root::new(pane, window, cx)
    })
}

/// Owned handle to the pane inside the Root, for mutation/assertion.
fn pane_of(root: &WindowHandle<Root>, cx: &TestAppContext) -> Entity<ExplorerPane> {
    root.read_with(cx, |root, _cx| {
        root.view()
            .clone()
            .downcast::<ExplorerPane>()
            .expect("the Root's view is the explorer pane")
    })
    .expect("window is alive")
}

/// Builds a rooted window with one file `a.txt`, waits for the first paint
/// (which focuses the pane), and selects row 0.
fn window_with_selected_file(cx: &mut TestAppContext) -> (WindowHandle<Root>, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "hello").unwrap();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();
    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        pane.select_single(0);
        cx.notify();
    });
    (root, dir)
}

#[gpui::test]
async fn ctrl_c_copies_selection(cx: &mut TestAppContext) {
    let (root, _dir) = window_with_selected_file(cx);
    cx.simulate_keystrokes(root.into(), "ctrl-c");

    let clip = cx.read(clipboard::current);
    assert_eq!(clip.mode, Some(ClipboardMode::Copy));
    assert_eq!(clip.paths.len(), 1);
    assert!(clip.paths[0].ends_with("a.txt"));
}

#[gpui::test]
async fn ctrl_x_cuts_selection(cx: &mut TestAppContext) {
    let (root, _dir) = window_with_selected_file(cx);
    cx.simulate_keystrokes(root.into(), "ctrl-x");

    let clip = cx.read(clipboard::current);
    assert_eq!(clip.mode, Some(ClipboardMode::Cut));
    assert_eq!(clip.paths.len(), 1);
    assert!(clip.paths[0].ends_with("a.txt"));
}

#[gpui::test]
async fn ctrl_v_pastes_clipboard_into_cwd(cx: &mut TestAppContext) {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    fs::write(src.path().join("a.txt"), "hello").unwrap();
    let root = root_window(cx, src.path());
    cx.run_until_parked();

    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        pane.select_single(0);
        pane.copy_selection(cx);
        // Point the pane at the destination: Ctrl+V must paste into the cwd.
        pane.cwd = dst.path().to_string_lossy().to_string();
        cx.notify();
    });

    cx.simulate_keystrokes(root.into(), "ctrl-v");
    assert_eq!(
        fs::read_to_string(dst.path().join("a.txt")).unwrap(),
        "hello",
        "Ctrl+V pastes the clipboard into the current directory"
    );
    assert!(src.path().join("a.txt").exists(), "copy keeps the source");
}

#[gpui::test]
async fn ctrl_shift_n_creates_a_new_folder(cx: &mut TestAppContext) {
    let dir = tempdir().unwrap();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();
    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        cx.notify();
    });

    cx.simulate_keystrokes(root.into(), "ctrl-shift-n");
    assert!(
        dir.path().join("New Folder").is_dir(),
        "Ctrl+Shift+N creates a folder in the cwd"
    );
}

#[gpui::test]
async fn f2_starts_inline_rename_on_a_single_selection(cx: &mut TestAppContext) {
    let (root, _dir) = window_with_selected_file(cx);
    cx.simulate_keystrokes(root.into(), "f2");

    let pane = pane_of(&root, cx);
    pane.read_with(cx, |pane, _cx| {
        assert!(pane.renaming.is_some(), "F2 starts an inline rename");
        assert!(pane.batch_rename.is_none());
    });
}

#[gpui::test]
async fn f2_on_a_multi_selection_opens_batch_rename(cx: &mut TestAppContext) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "x").unwrap();
    fs::write(dir.path().join("b.txt"), "y").unwrap();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();
    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        pane.select_single(0);
        pane.select_range_to(1);
        cx.notify();
    });

    cx.simulate_keystrokes(root.into(), "f2");
    pane.read_with(cx, |pane, _cx| {
        assert!(
            pane.batch_rename.is_some(),
            "F2 on a multi-selection opens Batch Rename"
        );
        assert!(pane.renaming.is_none());
    });
}

#[gpui::test]
async fn delete_opens_the_trash_confirm_dialog(cx: &mut TestAppContext) {
    let (root, _dir) = window_with_selected_file(cx);
    cx.simulate_keystrokes(root.into(), "delete");

    // The dialog lives on the window's `Root`; checking it needs a bare
    // `&mut Window`, which `VisualTestContext` provides without re-entrantly
    // updating the Root entity (unlike `WindowHandle<Root>::update`).
    let mut vcx = gpui::VisualTestContext::from_window(root.into(), cx);
    vcx.update(|window, cx| {
        assert!(
            window.has_active_dialog(cx),
            "Delete opens the trash-confirmation dialog"
        );
    });
    drop(vcx);

    // Nothing was deleted without confirming — the dialog does not act on its
    // own.
    let pane = pane_of(&root, cx);
    pane.read_with(cx, |pane, _cx| {
        assert_eq!(pane.filtered_entries.len(), 1, "no trash until confirmed");
    });
}

#[gpui::test]
async fn repeated_delete_does_not_stack_confirm_dialogs(cx: &mut TestAppContext) {
    let (root, _dir) = window_with_selected_file(cx);
    let pane = pane_of(&root, cx);

    // Open the first confirmation through the production Root dispatch tree.
    cx.simulate_keystrokes(root.into(), "delete");

    let mut vcx = gpui::VisualTestContext::from_window(root.into(), cx);
    vcx.update(|window, cx| {
        // Model a repeated handler delivery while the first dialog owns focus.
        // The helper guard must still prevent a second confirmation from being
        // installed, independently of focus/bubbling behavior.
        pane.update(cx, |pane, cx| {
            pane.confirm_delete_selection(window, cx);
        });
        assert!(window.has_active_dialog(cx));
        window.close_dialog(cx);
        assert!(
            !window.has_active_dialog(cx),
            "closing one confirmation must leave no stacked dialog"
        );
    });
}

#[gpui::test]
async fn file_ops_keys_are_noops_without_selection(cx: &mut TestAppContext) {
    let dir = tempdir().unwrap();
    let cwd = dir.path().to_string_lossy().to_string();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();

    cx.simulate_keystrokes(root.into(), "ctrl-c ctrl-x ctrl-v ctrl-a f2 delete enter");

    let clip = cx.read(clipboard::current);
    assert!(clip.mode.is_none());
    assert!(clip.paths.is_empty());

    let pane = pane_of(&root, cx);
    pane.read_with(cx, |pane, _cx| {
        assert!(pane.selection.is_empty());
        assert!(pane.renaming.is_none());
        assert!(pane.batch_rename.is_none());
        assert_eq!(pane.cwd, cwd);
    });

    let mut vcx = gpui::VisualTestContext::from_window(root.into(), cx);
    vcx.update(|window, cx| {
        assert!(!window.has_active_dialog(cx));
    });
}

/// Ctrl+A on the pane selects every filtered entry (regression guard for the
/// T049 additions — the arm predates them but lives next to the new ones).
#[gpui::test]
async fn ctrl_a_selects_all_filtered_entries(cx: &mut TestAppContext) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "x").unwrap();
    fs::write(dir.path().join("b.txt"), "y").unwrap();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();
    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        cx.notify();
    });

    cx.simulate_keystrokes(root.into(), "ctrl-a");
    pane.read_with(cx, |pane, _cx| {
        assert_eq!(pane.selection.len(), 2, "Ctrl+A selects every entry");
    });
}

/// T049's input-consumption contract: while the search bar's input holds
/// focus, editing keys must not reach the pane's file-operation handler.
#[gpui::test]
async fn focused_search_input_consumes_file_ops_keys(cx: &mut TestAppContext) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("b.txt"), "b").unwrap();
    let cwd = dir.path().to_string_lossy().to_string();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();
    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        pane.select_single(0);
        pane.copy_selection(cx);
        cx.notify();
    });

    // Open the search bar — `open_search` focuses its input.
    let mut vcx = gpui::VisualTestContext::from_window(root.into(), cx);
    vcx.update(|window, cx| {
        pane.update(cx, |pane, cx| {
            pane.open_search(window, cx);
            pane.search_input.update(cx, |input, cx| {
                input.set_value("query", window, cx);
            });
        });
    });
    drop(vcx);

    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("query".to_string())));
    cx.simulate_keystrokes(root.into(), "ctrl-a ctrl-c ctrl-x delete");
    let clip = cx.read(clipboard::current);
    assert_eq!(clip.mode, Some(ClipboardMode::Copy));
    assert_eq!(
        clip.paths.len(),
        1,
        "Ctrl+X must not replace file clipboard"
    );

    let mut vcx = gpui::VisualTestContext::from_window(root.into(), cx);
    vcx.update(|window, cx| {
        assert!(
            !window.has_active_dialog(cx),
            "Delete in the search input must not open the trash dialog"
        );
    });
    drop(vcx);

    pane.read_with(cx, |pane, _cx| {
        assert!(pane.search_visible);
        assert!(
            pane.selection.len() < 2,
            "Ctrl+A must not select every file row"
        );
        assert_eq!(pane.cwd, cwd);
    });
    assert!(dir.path().join("a.txt").exists());
    assert!(dir.path().join("b.txt").exists());
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        2,
        "filesystem untouched"
    );
}

#[gpui::test]
async fn enter_does_not_activate_a_row_while_search_is_visible(cx: &mut TestAppContext) {
    let (root, dir) = window_with_selected_file(cx);
    let cwd = dir.path().to_string_lossy().to_string();
    let pane = pane_of(&root, cx);

    let mut vcx = gpui::VisualTestContext::from_window(root.into(), cx);
    vcx.update(|window, cx| {
        pane.update(cx, |pane, cx| {
            pane.open_search(window, cx);
            pane.focus_handle.focus(window, cx);
        });
    });
    drop(vcx);

    cx.simulate_keystrokes(root.into(), "enter");
    pane.read_with(cx, |pane, _cx| {
        assert!(pane.search_visible);
        assert_eq!(pane.cwd, cwd, "Enter must not activate a row during search");
    });
}

#[gpui::test]
async fn focused_inline_rename_consumes_editing_keys(cx: &mut TestAppContext) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("b.txt"), "b").unwrap();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();
    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        pane.select_single(0);
        pane.copy_selection(cx);
        cx.notify();
    });

    cx.simulate_keystrokes(root.into(), "f2 ctrl-a ctrl-c ctrl-x ctrl-v delete");

    let clip = cx.read(clipboard::current);
    assert_eq!(clip.mode, Some(ClipboardMode::Copy));
    assert_eq!(
        clip.paths.len(),
        1,
        "Ctrl+X must not replace file clipboard"
    );
    pane.read_with(cx, |pane, _cx| {
        assert!(pane.renaming.is_some(), "editing keys stay in rename input");
        assert_eq!(pane.selection.len(), 1, "Ctrl+A must not select file rows");
    });
    let mut vcx = gpui::VisualTestContext::from_window(root.into(), cx);
    vcx.update(|window, cx| assert!(!window.has_active_dialog(cx)));
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        2,
        "Ctrl+V is input-only"
    );
}

#[gpui::test]
async fn enter_in_inline_rename_does_not_activate_the_row(cx: &mut TestAppContext) {
    let (root, dir) = window_with_selected_file(cx);
    let cwd = dir.path().to_string_lossy().to_string();
    let pane = pane_of(&root, cx);

    cx.simulate_keystrokes(root.into(), "f2 enter");

    pane.read_with(cx, |pane, _cx| {
        assert!(pane.renaming.is_none(), "Enter commits the inline editor");
        assert_eq!(pane.cwd, cwd, "Enter must not activate the file row");
    });
    assert!(dir.path().join("a.txt").exists());
}

#[gpui::test]
async fn focused_batch_rename_input_consumes_editing_keys(cx: &mut TestAppContext) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("b.txt"), "b").unwrap();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();
    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        pane.select_single(0);
        pane.select_range_to(1);
        cx.notify();
    });

    cx.simulate_keystrokes(root.into(), "f2");
    pane.update(cx, |pane, cx| pane.copy_selection(cx));
    cx.simulate_keystrokes(root.into(), "ctrl-a ctrl-c ctrl-x ctrl-v delete");

    let clip = cx.read(clipboard::current);
    assert_eq!(clip.mode, Some(ClipboardMode::Copy));
    assert_eq!(
        clip.paths.len(),
        2,
        "Ctrl+X must not replace file clipboard"
    );
    pane.read_with(cx, |pane, _cx| {
        assert!(
            pane.batch_rename.is_some(),
            "editing keys stay in dialog input"
        );
        assert_eq!(pane.selection.len(), 2);
    });
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        2,
        "Ctrl+V is input-only"
    );
}

/// T049's Enter row: with pane focus, Enter opens the active row — a
/// directory navigates into it. (The listing's own confirm path is unwired,
/// so this pane-level arm is the only Enter handler.)
#[gpui::test]
async fn enter_opens_the_active_directory(cx: &mut TestAppContext) {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    let sub_s = sub.to_string_lossy().to_string();
    let root = root_window(cx, dir.path());
    cx.run_until_parked();
    let pane = pane_of(&root, cx);
    pane.update(cx, |pane, cx| {
        pane.reload();
        pane.select_single(0);
        cx.notify();
    });

    cx.simulate_keystrokes(root.into(), "enter");
    pane.read_with(cx, |pane, _cx| {
        assert_eq!(pane.cwd, sub_s, "Enter navigates into the active directory");
    });
}
