# Explorer Right-Click Context Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a working right-click context menu to the chronos-fm explorer (list and grid views) — Open/Rename/Copy/Cut/Copy Path/Delete on rows, New Folder/Paste/Refresh on empty space — wired to the real (already-written, already-tested) filesystem operations in `chronos-fm-services::fs::ops`.

**Architecture:** Reuse `gpui-component`'s `ContextMenuExt::context_menu()` / `PopupMenu` widget (an in-window overlay, verified byte-identical against the pinned `Chronos-GPUI@ee80b72` rev — no fork changes needed). A new `FileClipboard` `Global` holds copy/cut state; new `impl ExplorerPane` modules (`rename.rs`, `file_ops.rs`) add the mutating methods; `row.rs`/`list.rs`/`grid.rs` are extended to wrap rows and the empty-area background in `.context_menu(...)`, using `Entity<ExplorerPane>::update(cx, ...)` inside menu-item `on_click` closures (menu closures only get `&mut App`, not `&mut Context<ExplorerPane>` — this is the standard gpui-component pattern for reaching entity state from a raw event callback).

**Tech Stack:** Rust, GPUI (`gpui-component` fork `Chronos-GPUI@ee80b72`), existing `chronos-fm-services::fs::ops`.

## Global Constraints

- `gpui-component` git dep is pinned to `Chronos-GPUI@ee80b72` (see root `Cargo.toml`). All APIs used below were read directly from `~/.cargo/git/checkouts/chronos-gpui-*/ee80b72/gpui-component` and `.../ee80b72/gpui`, not assumed from docs or a sibling checkout.
- `unsafe_code = "deny"`, `missing_docs = "warn"` (every `pub` item needs a doc comment), `clippy::unwrap_used`/`expect_used = "warn"` (allowed only in `#[cfg(test)]` code, per `clippy.toml`).
- `disallowed-methods` bans `std::fs::read`/`write`/`read_to_string` in app code — all filesystem mutation must go through `chronos_fm_services::fs::ops`, never raw `std::fs` calls in `chronos-fm-pages`.
- Any element wrapped in `.context_menu(...)` must carry an explicit `.id(...)` first — `ContextMenuExt` falls back to a pointer-address-derived id when the wrapped element has none, which is not guaranteed stable across renders and can desync the menu's open/closed state.
- `PopupMenuItem::on_click` handlers run as `Fn(&ClickEvent, &mut Window, &mut App)` — no `Context<ExplorerPane>`. To mutate the pane, capture `let pane = cx.entity();` (an `Entity<ExplorerPane>`, `Clone`) in the *caller's* scope (which does have `Context<ExplorerPane>`) and call `pane.update(cx, |pane, cx| { ... })` inside the handler.
- Reference module for the “fs op → reload → status” pattern: `chronos_fm_pages::explorer::navigation::reload` (`crates/chronos-fm-pages/src/explorer/navigation.rs:17`) always overwrites `status_message` (clears it on success). New methods that call `reload()` must call `set_status` for a failure *after* `reload()`, not before, or the success path will silently erase the error.
- Test pattern for anything needing `Window`/`Context<ExplorerPane>`: `crates/chronos-fm-pages/src/explorer/tests.rs`'s `new_explorer(cx) -> WindowHandle<ExplorerPane>`, driven via `window.update(cx, |page, window, cx| { ... })` / `window.read_with(cx, |page, cx| { ... })`.

---

## Task 1: `FileClipboard` global

**Files:**
- Create: `crates/chronos-fm-pages/src/explorer/clipboard.rs`
- Modify: `crates/chronos-fm-pages/src/explorer.rs:9` (add `pub mod clipboard;`)

**Interfaces:**
- Produces: `pub struct FileClipboard { pub paths: Vec<String>, pub mode: Option<ClipboardMode> }`, `pub enum ClipboardMode { Copy, Cut }` (both `Clone`, `PartialEq`), `pub fn init(cx: &mut App)`, `pub fn set_copy(paths: Vec<String>, cx: &mut App)`, `pub fn set_cut(paths: Vec<String>, cx: &mut App)`, `pub fn clear(cx: &mut App)`, `pub fn current(cx: &App) -> FileClipboard`. Tasks 4, 5, 6, 7 depend on all of these.

- [ ] **Step 1: Write the failing test**

Create `crates/chronos-fm-pages/src/explorer/clipboard.rs`:

```rust
//! Global clipboard for copy/cut/paste in the explorer, shared across all
//! panes and tabs so a copy in one pane can be pasted in another.

use gpui::{App, Global};

/// Whether a clipboard entry was set via Copy (kept at the source) or Cut
/// (removed from the source once pasted).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardMode {
    /// Paste duplicates the source; the source is left in place.
    Copy,
    /// Paste moves the source; the source is removed once the paste succeeds.
    Cut,
}

/// The current explorer clipboard contents: zero or more absolute paths and
/// whether they were copied or cut.
#[derive(Clone, Default)]
pub struct FileClipboard {
    /// Absolute paths of the files/directories on the clipboard.
    pub paths: Vec<String>,
    /// How the paths were placed on the clipboard, or `None` when empty.
    pub mode: Option<ClipboardMode>,
}

impl Global for FileClipboard {}

/// Registers the clipboard global with its empty default. Must be called once
/// before any window opens (see `chronos-fm/src/app.rs`).
pub fn init(cx: &mut App) {
    cx.set_global(FileClipboard::default());
}

/// Replaces the clipboard with `paths` in Copy mode.
pub fn set_copy(paths: Vec<String>, cx: &mut App) {
    cx.set_global(FileClipboard {
        paths,
        mode: Some(ClipboardMode::Copy),
    });
}

/// Replaces the clipboard with `paths` in Cut mode.
pub fn set_cut(paths: Vec<String>, cx: &mut App) {
    cx.set_global(FileClipboard {
        paths,
        mode: Some(ClipboardMode::Cut),
    });
}

/// Empties the clipboard.
pub fn clear(cx: &mut App) {
    cx.set_global(FileClipboard::default());
}

/// Returns a clone of the current clipboard contents.
pub fn current(cx: &App) -> FileClipboard {
    cx.global::<FileClipboard>().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn set_copy_then_clear_round_trips(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            set_copy(vec!["/a".to_string(), "/b".to_string()], cx);
            let state = current(cx);
            assert_eq!(state.mode, Some(ClipboardMode::Copy));
            assert_eq!(state.paths, vec!["/a".to_string(), "/b".to_string()]);

            clear(cx);
            assert!(current(cx).mode.is_none());
            assert!(current(cx).paths.is_empty());
        });
    }

    #[gpui::test]
    async fn set_cut_replaces_a_prior_copy(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            set_copy(vec!["/a".to_string()], cx);
            set_cut(vec!["/b".to_string()], cx);
            let state = current(cx);
            assert_eq!(state.mode, Some(ClipboardMode::Cut));
            assert_eq!(state.paths, vec!["/b".to_string()]);
        });
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-fm-pages clipboard:: -- --nocapture`
Expected: FAIL to compile — `explorer::clipboard` module does not exist yet (Step 1 only created the file; it isn't wired into `explorer.rs` until Step 3).

- [ ] **Step 3: Wire the module in and run again**

Edit `crates/chronos-fm-pages/src/explorer.rs`, adding the new module declaration next to the other `mod` lines (after `mod entries;`, alphabetical with the rest):

```rust
mod entries;
```
becomes
```rust
/// Global clipboard for copy/cut/paste, shared across panes and tabs.
pub mod clipboard;
mod entries;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-fm-pages clipboard:: -- --nocapture`
Expected: PASS — `set_copy_then_clear_round_trips` and `set_cut_replaces_a_prior_copy` both pass.

- [ ] **Step 5: Commit**

```bash
git add crates/chronos-fm-pages/src/explorer.rs crates/chronos-fm-pages/src/explorer/clipboard.rs
git commit -m "feat(explorer): add FileClipboard global for copy/cut/paste"
```

---

## Task 2: Register the clipboard global at app startup

**Files:**
- Modify: `crates/chronos-fm/src/app.rs:52-53`

**Interfaces:**
- Consumes: `chronos_fm_pages::explorer::clipboard::init(cx: &mut App)` (Task 1).

- [ ] **Step 1: Add the call**

In `crates/chronos-fm/src/app.rs`, right after `gpui_component::init(app);` (line 53):

```rust
            gpui_component::init(app);
```
becomes
```rust
            gpui_component::init(app);
            chronos_fm_pages::explorer::clipboard::init(app);
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build -p chronos-fm`
Expected: builds cleanly (this crate is a GUI crate; see workspace `default-members` comment in root `Cargo.toml` — it is not built by a bare `cargo build`, so build it explicitly by package name).

- [ ] **Step 3: Commit**

```bash
git add crates/chronos-fm/src/app.rs
git commit -m "feat(explorer): register FileClipboard global at startup"
```

---

## Task 3: Inline rename state and pane methods

**Files:**
- Create: `crates/chronos-fm-pages/src/explorer/rename.rs`
- Modify: `crates/chronos-fm-pages/src/explorer.rs` (add `mod rename;`)
- Modify: `crates/chronos-fm-pages/src/explorer/state.rs` (add `renaming` field)

**Interfaces:**
- Consumes: `ExplorerPane.filtered_entries: Vec<FileEntryDto>` (existing, `state.rs:30`), `chronos_fm_services::fs::ops::rename_in_place(src: &Path, new_name: &str) -> Result<PathBuf>` (existing), `self.reload()` (existing, `navigation.rs:17`), `self.set_status`/`self.clear_status` (existing, `state.rs:233,240`).
- Produces: field `ExplorerPane.renaming: Option<(usize, Entity<InputState>)>`; methods `pub(crate) fn begin_rename(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>)`, `pub(crate) fn commit_rename(&mut self, cx: &mut Context<Self>)`, `pub(crate) fn cancel_rename(&mut self, cx: &mut Context<Self>)`. Tasks 4 (`new_folder` calls `begin_rename`) and 6/7 (row/grid rendering read `renaming`) depend on these exact names and the field name.

- [ ] **Step 1: Add the `renaming` field to `ExplorerPane`**

In `crates/chronos-fm-pages/src/explorer/state.rs`, add to the struct (after the `status_message` field, state.rs:132):

```rust
    // Transient message shown in the footer status bar.
    /// Transient message shown in the footer status bar.
    pub status_message: Option<StatusMessage>,
}
```
becomes
```rust
    // Transient message shown in the footer status bar.
    /// Transient message shown in the footer status bar.
    pub status_message: Option<StatusMessage>,
    /// The row currently being renamed inline (an index into
    /// `filtered_entries`) and the input state backing its text field, or
    /// `None` when no row is being renamed.
    pub renaming: Option<(usize, Entity<InputState>)>,
}
```

And in `ExplorerPane::new` (state.rs:229), add the initializer right after `status_message: None,`:

```rust
            status_message: None,
        }
    }
```
becomes
```rust
            status_message: None,
            renaming: None,
        }
    }
```

- [ ] **Step 2: Run to verify it fails to compile (missing methods)**

Run: `cargo build -p chronos-fm-pages`
Expected: builds fine at this point (the field alone is inert) — this step is a checkpoint, not a red test; proceed to Step 3 to add the failing test for the new methods.

- [ ] **Step 3: Write the failing test**

Create `crates/chronos-fm-pages/src/explorer/rename.rs`:

```rust
//! Inline rename: turning a row's filename into an editable text field,
//! committing the new name via `chronos_fm_services::fs::ops::rename_in_place`,
//! or canceling without changing anything.

use gpui::{Context, Window};
use gpui_component::input::InputState;

use super::ExplorerPane;
use super::types::StatusLevel;

impl ExplorerPane {
    /// Starts renaming the row at `ix`, pre-filling an input with its current
    /// name and focusing it. Does nothing if `ix` is out of range.
    pub(crate) fn begin_rename(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.filtered_entries.get(ix) else {
            return;
        };
        let name = entry.name.clone();
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(name, window, cx);
            state.focus(window, cx);
            state
        });
        self.renaming = Some((ix, input));
        cx.notify();
    }

    /// Cancels an in-progress rename without changing anything on disk.
    pub(crate) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming = None;
        cx.notify();
    }

    /// Commits an in-progress rename: reads the input's text and renames the
    /// file on disk. A blank or unchanged name is treated as a no-op cancel,
    /// not an error.
    pub(crate) fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some((ix, input)) = self.renaming.take() else {
            return;
        };
        let Some(entry) = self.filtered_entries.get(ix).cloned() else {
            cx.notify();
            return;
        };
        let new_name = input.read(cx).text().to_string();
        if new_name.is_empty() || new_name == entry.name {
            cx.notify();
            return;
        }
        let src = std::path::Path::new(&entry.path);
        let result = chronos_fm_services::fs::ops::rename_in_place(src, &new_name);
        self.reload();
        if let Err(error) = result {
            self.set_status(StatusLevel::Error, format!("Rename failed: {error}"));
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::new_explorer_for_tests;
    use chronos_fm_services::fs::listing::FileEntryDto;
    use gpui::TestAppContext;
    use std::fs;
    use tempfile::tempdir;

    fn entry(dir: &std::path::Path, name: &str) -> FileEntryDto {
        FileEntryDto {
            name: name.to_string(),
            path: dir.join(name).to_string_lossy().to_string(),
            kind: "file".to_string(),
            size: 0,
            modified: 0,
        }
    }

    #[gpui::test]
    async fn begin_rename_then_commit_renames_file_on_disk(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("old.txt"), "x").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());

        window
            .update(cx, |page, window, cx| {
                page.filtered_entries = vec![entry(dir.path(), "old.txt")];
                page.begin_rename(0, window, cx);
                assert!(page.renaming.is_some());
            })
            .unwrap();

        window
            .update(cx, |page, _window, cx| {
                let (_, input) = page.renaming.clone().unwrap();
                input.update(cx, |state, cx| {
                    state.set_value("new.txt".to_string(), _window_placeholder(), cx)
                });
            })
            .unwrap();
    }

    #[gpui::test]
    async fn cancel_rename_clears_state_without_touching_disk(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("old.txt"), "x").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());

        window
            .update(cx, |page, window, cx| {
                page.filtered_entries = vec![entry(dir.path(), "old.txt")];
                page.begin_rename(0, window, cx);
                page.cancel_rename(cx);
                assert!(page.renaming.is_none());
            })
            .unwrap();

        assert!(dir.path().join("old.txt").exists());
    }
}
```

(The first test above intentionally will not compile — `_window_placeholder()` does not exist and `new_explorer_for_tests` does not exist yet. This is expected: Step 4 fixes the test harness gap and rewrites the first test to something that actually compiles and exercises `commit_rename`.)

- [ ] **Step 2b: Add the `mod rename;` declaration**

In `crates/chronos-fm-pages/src/explorer.rs`, add next to the other private `mod` declarations:

```rust
mod navigation;
```
becomes
```rust
mod navigation;
mod rename;
```

- [ ] **Step 4: Add a `Window`-taking test helper and fix the tests**

The existing `new_explorer(cx)` helper in `crates/chronos-fm-pages/src/explorer/tests.rs:31-40` always starts at `std::env::current_dir()` and takes no `cwd` override, and it is private to `tests.rs`. Add a second, `pub(crate)` helper next to it (so `rename.rs`'s and Task 4's `file_ops.rs`'s tests can point the pane at a tempdir):

In `crates/chronos-fm-pages/src/explorer/tests.rs`, after `new_explorer` (line 40):

```rust
fn new_explorer(cx: &mut TestAppContext) -> WindowHandle<ExplorerPane> {
    // gpui-component installs the `Theme` global and input/list subsystems its
    // widgets rely on; initialize it once before building any window.
    cx.update(gpui_component::init);
    cx.add_window(|window, cx| {
        let resizable = cx.new(|_| ResizableState::default());
        let search_input = cx.new(|cx| InputState::new(window, cx));
        ExplorerPane::new(resizable, search_input, None, cx.focus_handle())
    })
}
```
becomes
```rust
fn new_explorer(cx: &mut TestAppContext) -> WindowHandle<ExplorerPane> {
    // gpui-component installs the `Theme` global and input/list subsystems its
    // widgets rely on; initialize it once before building any window.
    cx.update(gpui_component::init);
    cx.add_window(|window, cx| {
        let resizable = cx.new(|_| ResizableState::default());
        let search_input = cx.new(|cx| InputState::new(window, cx));
        ExplorerPane::new(resizable, search_input, None, cx.focus_handle())
    })
}

/// Builds an `ExplorerPane` rooted at `cwd` (a real directory, typically a
/// `tempfile::tempdir()`), for tests that exercise filesystem operations
/// (rename, copy/cut/paste, delete). Also registers the `FileClipboard`
/// global, which those operations read.
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
```

Now rewrite `crates/chronos-fm-pages/src/explorer/rename.rs`'s test module to use only verified, compiling calls — replace the whole `#[cfg(test)] mod tests { ... }` block from Step 3 with:

```rust
#[cfg(test)]
mod tests {
    use super::super::tests::new_explorer_for_tests;
    use gpui::TestAppContext;
    use std::fs;
    use tempfile::tempdir;

    #[gpui::test]
    async fn begin_rename_then_commit_renames_file_on_disk(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("old.txt"), "x").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| {
                assert_eq!(page.filtered_entries.len(), 1);
                page.begin_rename(0, window, cx);
                let (_, input) = page.renaming.clone().unwrap();
                input.update(cx, |state, cx| {
                    state.set_value("new.txt".to_string(), window, cx)
                });
                page.commit_rename(cx);
            })
            .unwrap();

        assert!(!dir.path().join("old.txt").exists());
        assert!(dir.path().join("new.txt").exists());
        window
            .read_with(cx, |page, _cx| assert!(page.renaming.is_none()))
            .unwrap();
    }

    #[gpui::test]
    async fn cancel_rename_clears_state_without_touching_disk(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("old.txt"), "x").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| {
                page.begin_rename(0, window, cx);
                page.cancel_rename(cx);
                assert!(page.renaming.is_none());
            })
            .unwrap();

        assert!(dir.path().join("old.txt").exists());
    }
}
```

`tests.rs`'s `mod tests` is currently declared `#[cfg(test)] mod tests;` in `explorer.rs:13-14` and is private (`mod tests;`, not `pub(crate) mod tests;`) — since `rename.rs` needs `super::super::tests::new_explorer_for_tests`, change its visibility. In `crates/chronos-fm-pages/src/explorer.rs`:

```rust
#[cfg(test)]
mod tests;
```
becomes
```rust
#[cfg(test)]
pub(crate) mod tests;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p chronos-fm-pages rename:: -- --nocapture`
Expected: PASS — both tests green.

- [ ] **Step 6: Commit**

```bash
git add crates/chronos-fm-pages/src/explorer.rs crates/chronos-fm-pages/src/explorer/state.rs crates/chronos-fm-pages/src/explorer/rename.rs crates/chronos-fm-pages/src/explorer/tests.rs
git commit -m "feat(explorer): add inline rename state and pane methods"
```

---

## Task 4: Copy / Cut / Paste / New Folder / Delete pane methods

**Files:**
- Create: `crates/chronos-fm-pages/src/explorer/file_ops.rs`
- Modify: `crates/chronos-fm-pages/src/explorer.rs` (add `mod file_ops;`)

**Interfaces:**
- Consumes: `self.selected_paths() -> Vec<String>` (existing, `state.rs:377`), `self.cwd: String` (existing field), `chronos_fm_services::fs::ops::{unique_name, copy_path, move_path, create_dir, trash_path}` (existing), `super::clipboard::{current, set_copy, set_cut, clear, ClipboardMode}` (Task 1), `self.begin_rename` (Task 3).
- Produces: `pub(crate) fn copy_selection(&mut self, cx: &mut Context<Self>)`, `pub(crate) fn cut_selection(&mut self, cx: &mut Context<Self>)`, `pub(crate) fn paste_clipboard(&mut self, cx: &mut Context<Self>)`, `pub(crate) fn new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>)`, `pub(crate) fn delete_paths(&mut self, paths: Vec<String>, cx: &mut Context<Self>)`. Tasks 5, 6, 7 (row/list/grid context menus) call these by these exact names.

- [ ] **Step 1: Write the failing tests**

Create `crates/chronos-fm-pages/src/explorer/file_ops.rs`:

```rust
//! Filesystem-mutating pane operations (copy, cut, paste, new folder,
//! delete), each backed by `chronos_fm_services::fs::ops` and followed by a
//! `reload()` so the listing reflects the result.

use std::path::Path;

use gpui::{Context, Window};

use chronos_fm_services::fs::ops;

use super::ExplorerPane;
use super::clipboard::{self, ClipboardMode};
use super::types::StatusLevel;

impl ExplorerPane {
    /// Puts the current selection on the clipboard in Copy mode. No-op if
    /// nothing is selected.
    pub(crate) fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        clipboard::set_copy(paths, cx);
    }

    /// Puts the current selection on the clipboard in Cut mode. No-op if
    /// nothing is selected.
    pub(crate) fn cut_selection(&mut self, cx: &mut Context<Self>) {
        let paths = self.selected_paths();
        if paths.is_empty() {
            return;
        }
        clipboard::set_cut(paths, cx);
        cx.notify();
    }

    /// Copies (or moves, for Cut) every clipboard path into the current
    /// directory, resolving name collisions via `ops::unique_name`. A Cut
    /// clipboard is cleared after the paste completes (even partially).
    pub(crate) fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        let clip = clipboard::current(cx);
        let Some(mode) = clip.mode else {
            return;
        };
        let dst_dir = Path::new(&self.cwd).to_path_buf();
        let mut errors: Vec<String> = Vec::new();
        for src_path in &clip.paths {
            let src = Path::new(src_path);
            let Some(name) = src.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let name = ops::unique_name(&dst_dir, name);
            let dst = dst_dir.join(&name);
            let result = match mode {
                ClipboardMode::Copy => ops::copy_path(src, &dst),
                ClipboardMode::Cut => ops::move_path(src, &dst).map(|_| ()),
            };
            if let Err(error) = result {
                errors.push(format!("{name}: {error}"));
            }
        }
        if mode == ClipboardMode::Cut {
            clipboard::clear(cx);
        }
        self.reload();
        if !errors.is_empty() {
            self.set_status(
                StatusLevel::Error,
                format!("Paste failed for {}", errors.join(", ")),
            );
        }
        cx.notify();
    }

    /// Creates a new, uniquely-named folder in the current directory and
    /// immediately starts an inline rename on it so the user can type a name.
    pub(crate) fn new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parent = Path::new(&self.cwd).to_path_buf();
        let name = ops::unique_name(&parent, "New Folder");
        let result = ops::create_dir(&parent, &name);
        self.reload();
        match result {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                if let Some(ix) = self
                    .filtered_entries
                    .iter()
                    .position(|entry| entry.path == path_str)
                {
                    self.begin_rename(ix, window, cx);
                }
            }
            Err(error) => {
                self.set_status(StatusLevel::Error, format!("Could not create folder: {error}"));
            }
        }
        cx.notify();
    }

    /// Moves each of `paths` to the OS trash. Errors for individual paths are
    /// collected and reported together rather than aborting the whole batch.
    pub(crate) fn delete_paths(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        let mut errors: Vec<String> = Vec::new();
        for path in &paths {
            if let Err(error) = ops::trash_path(Path::new(path)) {
                errors.push(format!("{path}: {error}"));
            }
        }
        self.reload();
        if !errors.is_empty() {
            self.set_status(StatusLevel::Error, format!("Could not delete: {}", errors.join(", ")));
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::new_explorer_for_tests;
    use gpui::TestAppContext;
    use std::fs;
    use tempfile::tempdir;

    #[gpui::test]
    async fn copy_then_paste_duplicates_the_file(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        fs::write(src_dir.path().join("a.txt"), "hello").unwrap();

        let window = new_explorer_for_tests(cx, src_dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();
        window
            .update(cx, |page, _window, cx| page.copy_selection(cx))
            .unwrap();
        // Nothing selected yet in a freshly-loaded pane; select the one entry.
        window
            .update(cx, |page, _window, cx| {
                page.selection.clear();
                page.selection.insert(0);
                page.copy_selection(cx);
                page.change_dir_for_test(dst_dir.path().to_string_lossy().to_string());
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();

        assert_eq!(
            fs::read_to_string(dst_dir.path().join("a.txt")).unwrap(),
            "hello"
        );
        assert!(src_dir.path().join("a.txt").exists(), "copy preserves source");
    }

    #[gpui::test]
    async fn cut_then_paste_moves_the_file(cx: &mut TestAppContext) {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        fs::write(src_dir.path().join("a.txt"), "hello").unwrap();

        let window = new_explorer_for_tests(cx, src_dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();
        window
            .update(cx, |page, _window, cx| {
                page.selection.clear();
                page.selection.insert(0);
                page.cut_selection(cx);
                page.change_dir_for_test(dst_dir.path().to_string_lossy().to_string());
            })
            .unwrap();
        window
            .update(cx, |page, _window, cx| page.paste_clipboard(cx))
            .unwrap();

        assert!(!src_dir.path().join("a.txt").exists(), "cut removes source");
        assert_eq!(
            fs::read_to_string(dst_dir.path().join("a.txt")).unwrap(),
            "hello"
        );
    }

    #[gpui::test]
    async fn new_folder_creates_a_uniquely_named_directory(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("New Folder")).unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| page.new_folder(window, cx))
            .unwrap();

        assert!(dir.path().join("New Folder (2)").is_dir());
        window
            .read_with(cx, |page, _cx| assert!(page.renaming.is_some()))
            .unwrap();
    }

    #[gpui::test]
    async fn delete_paths_reports_an_error_for_a_missing_path(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        let window = new_explorer_for_tests(cx, dir.path());

        window
            .update(cx, |page, _window, cx| {
                let missing = dir.path().join("does-not-exist.txt");
                page.delete_paths(vec![missing.to_string_lossy().to_string()], cx);
                assert!(page.status_for_footer().is_some_and(|(_, is_error)| is_error));
            })
            .unwrap();
    }
}
```

- [ ] **Step 2: Add the `mod file_ops;` declaration**

In `crates/chronos-fm-pages/src/explorer.rs`:

```rust
/// The split-view container that owns one or more panes (`docs/explorer-essentials.md` §3).
mod page;
```
becomes
```rust
mod file_ops;
/// The split-view container that owns one or more panes (`docs/explorer-essentials.md` §3).
mod page;
```

- [ ] **Step 3: Add the missing `change_dir_for_test` test helper**

The tests above call `page.change_dir_for_test(...)` to move the pane to a second tempdir without going through the `window`-requiring `change_dir` (navigation.rs:52), which also touches search state unnecessarily for this test. Add a tiny helper next to `change_dir` in `crates/chronos-fm-pages/src/explorer/navigation.rs`, after `change_dir` (line 63):

```rust
    // Records a forward navigation in the back/forward history, seeding the
    // starting directory on first use and dropping any forward entries.
    fn push_history(&mut self, path: String) {
```
becomes
```rust
    /// Test-only: sets `cwd` directly without navigation history, search
    /// reset, or emitting `PaneEvent::Navigated` — for tests that only need
    /// `paste_clipboard`/`new_folder` to act on a different directory.
    #[cfg(test)]
    pub(crate) fn change_dir_for_test(&mut self, path: String) {
        self.cwd = path;
    }

    // Records a forward navigation in the back/forward history, seeding the
    // starting directory on first use and dropping any forward entries.
    fn push_history(&mut self, path: String) {
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chronos-fm-pages file_ops:: -- --nocapture`
Expected: PASS — all four tests green. (`copy_then_paste_duplicates_the_file`'s first `copy_selection` call before the selection is populated is intentionally a no-op per the method's own guard; the assertions only check the state after the real, selected copy.)

- [ ] **Step 5: Commit**

```bash
git add crates/chronos-fm-pages/src/explorer.rs crates/chronos-fm-pages/src/explorer/file_ops.rs crates/chronos-fm-pages/src/explorer/navigation.rs
git commit -m "feat(explorer): add copy/cut/paste/new-folder/delete pane methods"
```

---

## Task 5: Row context menu, right-click selection normalization, cut-row dimming

**Files:**
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing/row.rs`
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing/list.rs` (thread `window` through)
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing.rs` (pass `window` to `list::render`)

**Interfaces:**
- Consumes: `page.is_selected(ix)`, `page.select_single(ix)` (existing, `state.rs:298,304`), `clipboard::current(cx)` (Task 1), `page.copy_selection`/`cut_selection`/`delete_paths` (Task 4), `page.begin_rename` (Task 3), `window.open_alert_dialog` / `gpui_component::dialog::{DialogButtonProps}` / `gpui_component::button::ButtonVariant` (verified in `gpui-component` at pinned rev).
- Produces: `row::render` signature changes to take `window: &mut Window` as its 4th parameter — Task 7 (grid.rs) does not depend on this since grid already threads `window` itself, but any other future caller of `row::render` (there are none besides `list.rs`) would need updating too.

- [ ] **Step 1: Thread `window` through `listing.rs` → `list.rs` → `row.rs`**

In `crates/chronos-fm-pages/src/explorer/view/listing.rs`, line 24:

```rust
        ViewMode::List => list::render(page, cx),
```
becomes
```rust
        ViewMode::List => list::render(page, window, cx),
```

In `crates/chronos-fm-pages/src/explorer/view/listing/list.rs`, the top of `render` (line 12):

```rust
pub fn render(page: &mut ExplorerPane, cx: &mut Context<ExplorerPane>) -> AnyElement {
```
becomes
```rust
pub fn render(page: &mut ExplorerPane, window: &mut Window, cx: &mut Context<ExplorerPane>) -> AnyElement {
```

And its call to `render_table_with_header` (list.rs:26-35):

```rust
        .child(render_table_with_header(
            page,
            table_width,
            col_name,
            col_type,
            col_size,
            col_modified,
            col_action,
            cx,
        ))
```
becomes
```rust
        .child(render_table_with_header(
            page,
            window,
            table_width,
            col_name,
            col_type,
            col_size,
            col_modified,
            col_action,
            cx,
        ))
```

`render_table_with_header`'s signature (list.rs:42-51) gains `window: &mut Window` as its 2nd parameter:

```rust
fn render_table_with_header(
    page: &mut ExplorerPane,
    table_width: f32,
```
becomes
```rust
fn render_table_with_header(
    page: &mut ExplorerPane,
    window: &mut Window,
    table_width: f32,
```

Inside it, the `v_virtual_list` closure (list.rs:64) currently ignores its `window` parameter — use it, and pass it (and a clone of the page entity, needed in Step 3) to `row::render`:

```rust
            move |view, visible_range, _window, cx| {
                visible_range
                    .filter_map(|ix| {
                        if ix == 0 {
                            Some(
                                render_header_row(
                                    view,
                                    table_width,
                                    col_name,
                                    col_type,
                                    col_size,
                                    col_modified,
                                    col_action,
                                    cx,
                                )
                                .into_any_element(),
                            )
                        } else {
                            let data_ix = ix - 1;
                            view.filtered_entries.get(data_ix).cloned().map(|item| {
                                row::render(view, &item, data_ix, cx).into_any_element()
                            })
                        }
                    })
                    .collect()
            },
```
becomes
```rust
            move |view, visible_range, window, cx| {
                visible_range
                    .filter_map(|ix| {
                        if ix == 0 {
                            Some(
                                render_header_row(
                                    view,
                                    table_width,
                                    col_name,
                                    col_type,
                                    col_size,
                                    col_modified,
                                    col_action,
                                    cx,
                                )
                                .into_any_element(),
                            )
                        } else {
                            let data_ix = ix - 1;
                            view.filtered_entries.get(data_ix).cloned().map(|item| {
                                row::render(view, &item, data_ix, window, cx).into_any_element()
                            })
                        }
                    })
                    .collect()
            },
```

And the empty-area context menu target for Task 7 — note for now that `render_table_with_header`'s own body (list.rs:59) wraps the virtual list in `div().flex_1().overflow_hidden()`; leave it as-is in this task, Task 7 revisits it.

- [ ] **Step 2: Verify it builds (signature-only change so far)**

Run: `cargo build -p chronos-fm-pages`
Expected: FAILS — `row::render` doesn't yet accept a `window` parameter. This is expected; fixed in Step 3.

- [ ] **Step 3: Update `row.rs`'s signature and add right-click selection normalization + the context menu**

In `crates/chronos-fm-pages/src/explorer/view/listing/row.rs`, update imports (top of file, lines 1-9):

```rust
use super::truncate_middle;

use crate::explorer::ExplorerPane;
use gpui::prelude::*;
use gpui::*;
use gpui_component::list::ListItem;
use gpui_component::{Icon, IconName};
use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_ui::theme::theme;
```
becomes
```rust
use super::truncate_middle;

use crate::explorer::ExplorerPane;
use crate::explorer::clipboard::{self, ClipboardMode};
use gpui::prelude::*;
use gpui::*;
use gpui_component::button::ButtonVariant;
use gpui_component::dialog::DialogButtonProps;
use gpui_component::list::ListItem;
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::{Icon, IconName, WindowExt};
use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_ui::theme::theme;
```

Update the function signature (row.rs:12-17):

```rust
pub fn render(
    page: &ExplorerPane,
    item: &FileEntryDto,
    ix: usize,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
```
becomes
```rust
pub fn render(
    page: &ExplorerPane,
    item: &FileEntryDto,
    ix: usize,
    window: &mut Window,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
```

(`window` is unused by the rest of `row::render`'s existing body except where noted below — this is expected; it is threaded through only for the context-menu-triggered rename/delete-dialog calls added in this step.)

Right after the existing `bg_color` computation (row.rs:29-35), compute the cut-dim flag and pre-build the entity handle + click paths needed by the menu (insert before `let file_type = ...` at row.rs:37):

```rust
    let clip = clipboard::current(cx);
    let is_cut = clip.mode == Some(ClipboardMode::Cut) && clip.paths.contains(&item.path);

    let pane = cx.entity();
    let item_path = item.path.clone();
```

Now wrap the outermost `div()` (row.rs:105-108) with an explicit id, the right-click selection normalizer, cut dimming, and the context menu itself:

```rust
    div()
        .flex()
        .flex_col()
        .w(px(total_width))
        .child(
```
becomes
```rust
    div()
        .id(("file-row-menu", ix))
        .flex()
        .flex_col()
        .w(px(total_width))
        .when(is_cut, |el| el.opacity(0.5))
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |this, _: &gpui::MouseDownEvent, _window, cx| {
                if !this.is_selected(ix) {
                    this.select_single(ix);
                    cx.notify();
                }
            }),
        )
        .context_menu({
            let pane = pane.clone();
            let item_path = item_path.clone();
            move |menu, _window, _cx| {
                let single_selected = pane.read(_cx).selection.len() <= 1;
                let rename_pane = pane.clone();
                let rename_ix = ix;
                let copy_pane = pane.clone();
                let cut_pane = pane.clone();
                let copy_path_text = item_path.clone();
                let delete_pane = pane.clone();
                menu.item(
                    PopupMenuItem::new("Rename")
                        .disabled(!single_selected)
                        .on_click(move |_, window, cx| {
                            rename_pane.update(cx, |pane, cx| {
                                pane.begin_rename(rename_ix, window, cx);
                            });
                        }),
                )
                .item(PopupMenuItem::new("Copy").on_click(move |_, _window, cx| {
                    copy_pane.update(cx, |pane, cx| pane.copy_selection(cx));
                }))
                .item(PopupMenuItem::new("Cut").on_click(move |_, _window, cx| {
                    cut_pane.update(cx, |pane, cx| pane.cut_selection(cx));
                }))
                .item(PopupMenuItem::new("Copy Path").on_click(move |_, _window, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_path_text.clone()));
                }))
                .separator()
                .item(PopupMenuItem::new("Delete").on_click(move |_, window, cx| {
                    let pane = delete_pane.clone();
                    let paths = pane.read(cx).selected_paths();
                    let count = paths.len();
                    window.open_alert_dialog(cx, move |alert, _, _| {
                        let paths = paths.clone();
                        let pane = pane.clone();
                        alert
                            .title("Delete Selected Items?")
                            .description(format!("{count} item(s) will be moved to Trash."))
                            .button_props(
                                DialogButtonProps::default()
                                    .ok_text("Delete")
                                    .ok_variant(ButtonVariant::Danger)
                                    .show_cancel(true),
                            )
                            .on_ok(move |_, _window, cx| {
                                pane.update(cx, |pane, cx| pane.delete_paths(paths.clone(), cx));
                                true
                            })
                    });
                }))
            }
        })
        .child(
```

- [ ] **Step 4: Verify it builds**

Run: `cargo build -p chronos-fm-pages`
Expected: builds cleanly.

- [ ] **Step 5: Manual smoke test (no GUI test harness exists for this crate)**

Run: `cargo run -p chronos-fm`, then in the running window: right-click a file row (menu appears with Rename/Copy/Cut/Copy Path/Delete), right-click a second, unselected row while the first is selected (selection should jump to the newly right-clicked row, matching the "normalize to single" behavior), select 2+ rows then right-click one of them (selection should stay multi — Rename should show disabled), Delete on a real scratch file (confirm dialog appears, OK moves it to trash and the row disappears from the listing).

- [ ] **Step 6: Commit**

```bash
git add crates/chronos-fm-pages/src/explorer/view/listing.rs crates/chronos-fm-pages/src/explorer/view/listing/list.rs crates/chronos-fm-pages/src/explorer/view/listing/row.rs
git commit -m "feat(explorer): add row context menu, right-click selection, cut dimming"
```

---

## Task 6: Inline rename rendering in the row

**Files:**
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing/row.rs`

**Interfaces:**
- Consumes: `page.renaming: Option<(usize, Entity<InputState>)>` (Task 3), `gpui_component::input::Input` (verified in `gpui-component` at pinned rev, used identically in `search_bar.rs:5,134`).

- [ ] **Step 1: Swap the filename for an `Input` when this row is being renamed**

Add to `row.rs`'s imports:

```rust
use gpui_component::input::Input;
```

Find the filename rendering (row.rs, inside the name-column `div()`, currently):

```rust
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(rgb(theme::FG))
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .whitespace_nowrap()
                                        .child(styled_name),
                                ),
```

Replace it with a conditional: when `page.renaming` matches this row's `ix`, render the `Input` (with Enter/Escape handling exactly mirroring the existing `search_bar.rs:125-133` pattern); otherwise render the existing styled name:

```rust
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
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(theme::FG))
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .whitespace_nowrap()
                                            .child(styled_name)
                                            .into_any_element()
                                    }
                                }),
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build -p chronos-fm-pages`
Expected: builds cleanly.

- [ ] **Step 3: Manual smoke test**

Run: `cargo run -p chronos-fm`. Right-click a file → Rename → the filename becomes an editable field, pre-filled and focused. Type a new name, press Enter → row updates to the new name and the field reverts to static text. Repeat and press Escape instead → reverts to the original name, no rename happens (verify on disk with `ls` in another terminal).

- [ ] **Step 4: Commit**

```bash
git add crates/chronos-fm-pages/src/explorer/view/listing/row.rs
git commit -m "feat(explorer): render inline rename input in the row"
```

---

## Task 7: Empty-area context menu (list + grid) and grid row context menu

**Files:**
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing/list.rs`
- Modify: `crates/chronos-fm-pages/src/explorer/view/listing/grid.rs`

**Interfaces:**
- Consumes: `page.new_folder`/`paste_clipboard` (Task 4), `page.reload()` (existing), `clipboard::current(cx).mode.is_some()` (Task 1), same `ContextMenuExt`/`PopupMenuItem` APIs as Task 5.

- [ ] **Step 1: Empty-area menu on the list background**

In `crates/chronos-fm-pages/src/explorer/view/listing/list.rs`, add imports:

```rust
use crate::explorer::clipboard;
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
```

Replace the whole function body of `render_table_with_header` (post-Task-5, i.e. already taking `window: &mut Window` as its 2nd parameter and already passing `window` into the `v_virtual_list` closure) with the following. This reuses the existing `let entity = cx.entity().clone();` (list.rs:52) as the handle for the new context menu instead of adding a second `cx.entity()` call, and adds one more level of `.child(...)` nesting than before (the old code closed the `div().flex_1().overflow_hidden().child(` with a single trailing `)`; the version below has the same `div()` chain gain a `.context_menu(...)` step before its `.child(...)`, so the same single trailing `)` still closes it — no extra paren is needed, `.context_menu()` returns a value of the same builder-chainable shape as the `div()` it wraps):

```rust
fn render_table_with_header(
    page: &mut ExplorerPane,
    window: &mut Window,
    table_width: f32,
    col_name: f32,
    col_type: f32,
    col_size: f32,
    col_modified: f32,
    col_action: f32,
    cx: &mut Context<ExplorerPane>,
) -> impl IntoElement + use<> {
    let entity = cx.entity().clone();

    let mut all_sizes = vec![gpui::size(px(table_width), px(48.0))];
    all_sizes.extend(page.item_sizes.as_ref().iter().copied());
    let all_sizes = Rc::new(all_sizes);
    let scroll_handle = page.virtual_scroll_handle.clone();

    let pane = entity.clone();
    div()
        .id("listing-empty-area")
        .flex_1()
        .overflow_hidden()
        .context_menu(move |menu, _window, cx| {
            let has_clipboard = clipboard::current(cx).mode.is_some();
            let new_folder_pane = pane.clone();
            let paste_pane = pane.clone();
            let refresh_pane = pane.clone();
            menu.item(PopupMenuItem::new("New Folder").on_click(move |_, window, cx| {
                new_folder_pane.update(cx, |pane, cx| pane.new_folder(window, cx));
            }))
            .item(
                PopupMenuItem::new("Paste")
                    .disabled(!has_clipboard)
                    .on_click(move |_, _window, cx| {
                        paste_pane.update(cx, |pane, cx| pane.paste_clipboard(cx));
                    }),
            )
            .item(PopupMenuItem::new("Refresh").on_click(move |_, _window, cx| {
                refresh_pane.update(cx, |pane, cx| {
                    pane.reload();
                    cx.notify();
                });
            }))
        })
        .child(
            v_virtual_list(
                entity,
                "file-table",
                all_sizes,
                move |view, visible_range, window, cx| {
                    visible_range
                        .filter_map(|ix| {
                            if ix == 0 {
                                Some(
                                    render_header_row(
                                        view,
                                        table_width,
                                        col_name,
                                        col_type,
                                        col_size,
                                        col_modified,
                                        col_action,
                                        cx,
                                    )
                                    .into_any_element(),
                                )
                            } else {
                                let data_ix = ix - 1;
                                view.filtered_entries.get(data_ix).cloned().map(|item| {
                                    row::render(view, &item, data_ix, window, cx).into_any_element()
                                })
                            }
                        })
                        .collect()
                },
            )
            .track_scroll(&scroll_handle),
        )
}
```

(Note `entity` here must be computed *before* `let pane = cx.entity();` is inserted, since both come from the same `cx` and the existing `let entity = cx.entity().clone();` at list.rs:52 already provides an `Entity<ExplorerPane>` clone — reuse that one directly instead of adding a second `cx.entity()` call: replace `let pane = cx.entity();` above with `let pane = entity.clone();` and move the context-menu block to after line 52's `let entity = ...` and the `let scroll_handle = ...` line, i.e. right before the `div()` construction, so both `entity` and `pane` are in scope.)

- [ ] **Step 2: Verify it builds**

Run: `cargo build -p chronos-fm-pages`
Expected: builds cleanly.

- [ ] **Step 3: Empty-area menu + row context menu on the grid**

In `crates/chronos-fm-pages/src/explorer/view/listing/grid.rs`, add imports:

```rust
use crate::explorer::clipboard::{self, ClipboardMode};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
```

Wrap the grid's scroll container (grid.rs:29-34):

```rust
    div()
        .id("grid-scroll")
        .flex_1()
        .overflow_scroll()
        .px(px(24.0))
        .py(px(16.0))
        .child(grid)
        .into_any_element()
```
becomes
```rust
    let pane = cx.entity();
    div()
        .id("grid-scroll")
        .flex_1()
        .overflow_scroll()
        .px(px(24.0))
        .py(px(16.0))
        .context_menu(move |menu, _window, cx| {
            let has_clipboard = clipboard::current(cx).mode.is_some();
            let new_folder_pane = pane.clone();
            let paste_pane = pane.clone();
            let refresh_pane = pane.clone();
            menu.item(PopupMenuItem::new("New Folder").on_click(move |_, window, cx| {
                new_folder_pane.update(cx, |pane, cx| pane.new_folder(window, cx));
            }))
            .item(
                PopupMenuItem::new("Paste")
                    .disabled(!has_clipboard)
                    .on_click(move |_, _window, cx| {
                        paste_pane.update(cx, |pane, cx| pane.paste_clipboard(cx));
                    }),
            )
            .item(PopupMenuItem::new("Refresh").on_click(move |_, _window, cx| {
                refresh_pane.update(cx, |pane, cx| {
                    pane.reload();
                    cx.notify();
                });
            }))
        })
        .child(grid)
        .into_any_element()
```

Now add a row-level context menu + cut dimming to `render_grid_item` (grid.rs, its `div()` builder starting at `.w(px(180.0))`, currently ending with the existing `.on_mouse_down(Left, ...)` block and further children). Add right after the existing `let border_color = ...` block (before `div()\n        .w(px(180.0))`):

```rust
    let clip = clipboard::current(cx);
    let is_cut = clip.mode == Some(ClipboardMode::Cut) && clip.paths.contains(&item.path);
    let pane = cx.entity();
    let item_path = item.path.clone();
```

And extend the tile's `div()` chain (immediately after `.gap_3()`, before `.on_mouse_down(`):

```rust
        .id(("grid-item-menu", ix))
        .when(is_cut, |el| el.opacity(0.5))
        .context_menu(move |menu, _window, cx| {
            let single_selected = pane.read(cx).selection.len() <= 1;
            let rename_pane = pane.clone();
            let copy_pane = pane.clone();
            let cut_pane = pane.clone();
            let copy_path_text = item_path.clone();
            let delete_pane = pane.clone();
            menu.item(
                PopupMenuItem::new("Rename")
                    .disabled(!single_selected)
                    .on_click(move |_, window, cx| {
                        rename_pane.update(cx, |pane, cx| pane.begin_rename(ix, window, cx));
                    }),
            )
            .item(PopupMenuItem::new("Copy").on_click(move |_, _window, cx| {
                copy_pane.update(cx, |pane, cx| pane.copy_selection(cx));
            }))
            .item(PopupMenuItem::new("Cut").on_click(move |_, _window, cx| {
                cut_pane.update(cx, |pane, cx| pane.cut_selection(cx));
            }))
            .item(PopupMenuItem::new("Copy Path").on_click(move |_, _window, cx| {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy_path_text.clone()));
            }))
            .separator()
            .item(PopupMenuItem::new("Delete").on_click(move |_, window, cx| {
                let pane = delete_pane.clone();
                let paths = pane.read(cx).selected_paths();
                let count = paths.len();
                window.open_alert_dialog(cx, move |alert, _, _| {
                    let paths = paths.clone();
                    let pane = pane.clone();
                    alert
                        .title("Delete Selected Items?")
                        .description(format!("{count} item(s) will be moved to Trash."))
                        .button_props(
                            gpui_component::dialog::DialogButtonProps::default()
                                .ok_text("Delete")
                                .ok_variant(gpui_component::button::ButtonVariant::Danger)
                                .show_cancel(true),
                        )
                        .on_ok(move |_, _window, cx| {
                            pane.update(cx, |pane, cx| pane.delete_paths(paths.clone(), cx));
                            true
                        })
                });
            }))
        })
```

Grid tiles do not currently render an editable name field distinct from `name` (a plain `truncate_middle` string in a `div()`), so inline rename in grid mode is out of scope for this task — the design's non-goals already exclude view-mode-specific polish beyond parity on the menu itself; renaming from the grid still works via the row's own equivalent in list view. (If grid-mode inline rename is wanted later, it needs the same `page.renaming` check `row.rs` Task 6 added, applied to `render_grid_item`'s name `div()`.)

- [ ] **Step 4: Verify it builds**

Run: `cargo build -p chronos-fm-pages`
Expected: builds cleanly.

- [ ] **Step 5: Manual smoke test**

Run: `cargo run -p chronos-fm`. In grid view: right-click empty space → New Folder creates a folder and reloads; Copy a file in list view, switch to grid view, right-click empty space → Paste is enabled and pastes; right-click a tile → Copy/Cut/Copy Path/Delete all behave the same as in list view.

- [ ] **Step 6: Commit**

```bash
git add crates/chronos-fm-pages/src/explorer/view/listing/list.rs crates/chronos-fm-pages/src/explorer/view/listing/grid.rs
git commit -m "feat(explorer): add empty-area context menu and grid row context menu"
```

---

## Task 8: Full verification pass

**Files:** none (verification only).

- [ ] **Step 1: Run the full test suite**

Run: `cargo test -p chronos-fm-pages`
Expected: PASS, including all tests added in Tasks 1, 3, 4, plus the pre-existing suite in `explorer/tests.rs`.

- [ ] **Step 2: Run clippy on the touched crates**

Run: `cargo clippy -p chronos-fm-pages -p chronos-fm --all-targets -- -D warnings`
Expected: no warnings. Pay particular attention to `missing_docs` (every new `pub` item needs a doc comment — all code above already includes one) and `unwrap_used`/`expect_used` outside `#[cfg(test)]`.

- [ ] **Step 3: Build the GUI binary**

Run: `cargo build -p chronos-fm`
Expected: builds cleanly (this exercises `row.rs`/`list.rs`/`grid.rs`, which are not covered by `chronos-fm-pages`'s own test suite in headless CI the same way, since they render real GPUI elements).

- [ ] **Step 4: Final manual walkthrough**

Run: `cargo run -p chronos-fm` and, in a scratch directory, exercise the full flow once end-to-end: New Folder → Rename it → Copy a file into it → Cut a different file into it → Delete the original cut source's now-empty trace → confirm the footer status bar shows no stray error after the happy path, and shows a clear error message if you force one (e.g. delete a file that another process removed first).

- [ ] **Step 5: Report**

No commit for this task — it is verification only. Summarize pass/fail per step when reporting completion, per `verification-before-completion`: this crate has no GUI screenshot harness, so Step 4 is a manual claim, not an automated one, and must be reported as such rather than implied to be test-covered.
