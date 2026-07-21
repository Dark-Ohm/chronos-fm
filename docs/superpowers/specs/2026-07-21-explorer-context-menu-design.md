# Explorer Right-Click Context Menu — Design Document

**Date**: 2026-07-21
**Project**: chronos-fm (`crates/chronos-fm-pages` explorer view)
**Scope**: Add a functional right-click context menu to the file listing (list and grid views), backed by real file operations.

---

## 1. Problem & Goal

The explorer currently only lists files — there is no right-click context menu at all, and none of the existing filesystem operations in `chronos-fm-services::fs::ops` (`copy_path`, `move_path`, `rename_in_place`, `create_dir`, `trash_path`, `delete_permanent`) are wired into the UI. This design adds:

- A per-row context menu (Open, Rename, Copy, Cut, Paste, Copy Path, Delete)
- An empty-area context menu on the listing background (New Folder, Paste, Refresh)
- Inline rename
- A clipboard for copy/cut/paste, shared across panes
- A confirm dialog gating Delete

## 2. Dependency Verification

`gpui-component` is pinned in the workspace `Cargo.toml` as a git dependency: `Chronos-GPUI @ ee80b72`. The local checkout at `~/.cargo/git/checkouts/chronos-gpui-*/ee80b72/gpui-component` was diffed byte-for-byte against `Source/gpui-component` (rev `99cab5e`) for the four files this design relies on — `menu/context_menu.rs`, `menu/popup_menu.rs`, `dialog/alert_dialog.rs`, `window_ext.rs` — and all four are identical. The API described below is confirmed to exist at the pinned rev, not assumed from a possibly-drifted sibling checkout.

Note: `gpui-component` here is a widget-level, in-window popup (`ContextMenuExt::context_menu()` renders an `anchored`/`deferred` overlay inside the current window), not a native `WindowKind::AnchoredPopup` from the `Source/` gpui-ce fork. This is the correct layer for chronos-fm — chronos-fm is a consumer of `gpui-component`, not a modifier of the fork itself, and no fork-level work is needed for this feature.

## 3. Architecture

### 3.1 Per-row menu (`row.rs`)

Wrap the row's outer `div()`:
1. `.on_mouse_down(MouseButton::Right, cx.listener(...))` fires first, to normalize selection *before* the menu opens: if the row is not part of the current selection, replace the selection with just this row (`select_single(ix)`); if it's already selected (single or multi), leave the selection as-is.
2. `.context_menu(|menu, window, cx| { ... })` builds the `PopupMenu` using `page.selected_paths()` (already exists at `state.rs:377`) captured via a weak/cloned entity handle, following the existing `Context::listener`/`Context::processor` weak-handle pattern used elsewhere in this pane.

Menu items, in order:
- **Open** — reuses `activate_entry` (`navigation.rs:128`)
- **Rename** — only enabled when exactly one path is selected; triggers inline rename (3.3)
- **Copy** — sets the global clipboard to `{ paths: selected_paths(), mode: Copy }`
- **Cut** — sets the global clipboard to `{ paths: selected_paths(), mode: Cut }`
- **Copy Path** — writes the absolute path(s) to the system clipboard (`cx.write_to_clipboard`)
- separator
- **Delete** — opens `window.open_alert_dialog(...)` (confirm), on confirm calls `trash_path` for each selected path, then `reload()`

`Paste` is not offered on a row — only on empty space, matching common file-manager conventions (pasting "into" a row would ambiguously mean "into this folder" vs "next to this file"; deferred, not in scope for this pass).

### 3.2 Empty-area menu

The listing container div (`list.rs` / `grid.rs`, the scrollable region background, not an individual row) gets its own `.context_menu(...)`:
- **New Folder** — `create_dir(current_dir, "New Folder")`, then `reload()`, then immediately enter inline-rename on the new entry so the user can type a name right away
- **Paste** — enabled only if the global clipboard is non-empty; copies (or moves, for Cut) each clipboard path into the current directory via `copy_path`/`move_path`, then `reload()`. Cut clears the clipboard after a successful paste.
- **Refresh** — calls `reload()` (`navigation.rs:17`)

### 3.3 Inline rename

Add to `ExplorerPane` state: `renaming: Option<(usize, Entity<InputState>)>` — row index plus a `gpui_component::input::InputState` pre-filled with the current filename (extension included, matching most file managers' default of selecting the base name only — out of scope to auto-select just the stem for this pass).

When `renaming` is `Some((ix, _))` and the row being rendered is `ix`, `row.rs` renders the `Input` in place of the static filename text instead of the `StyledText`. `on_confirm` (Enter) calls `rename_in_place(path, new_name)`, clears `renaming`, and `reload()`s. `Escape` or losing focus without confirming clears `renaming` without renaming.

### 3.4 Clipboard — global entity

```rust
#[derive(Clone)]
enum ClipboardMode { Copy, Cut }

#[derive(Clone, Default)]
struct FileClipboard {
    paths: Vec<String>,
    mode: Option<ClipboardMode>,
}

impl Global for FileClipboard {}
```

Registered once at app init (`cx.set_global(FileClipboard::default())`). Living as a global (rather than per-`ExplorerPane` field) means copying in one pane and pasting in another — including a different split pane or tab — works without extra plumbing, per your choice to keep clipboard app-level rather than per-pane.

Rows currently in "Cut" state are visually dimmed: `row.rs` checks `cx.global::<FileClipboard>()` and if the row's path is present with `mode: Cut`, renders the row at reduced opacity (e.g. `.opacity(0.5)`), matching Explorer/Nautilus/Finder convention.

### 3.5 Delete confirmation

```rust
window.open_alert_dialog(cx, move |alert, _, _| {
    alert
        .warning()
        .title("Delete Selected Items?")
        .description(format!("{} item(s) will be moved to Trash.", paths.len()))
        .show_cancel(true)
        .on_ok(move |_, window, cx| { /* trash_path each, reload */ })
});
```

Delete always trashes (`trash_path`), never permanently deletes, in this pass — "Delete Permanently" is an explicit non-goal, deferred to a follow-up if requested.

## 4. Error Handling

`chronos-fm-services::fs::ops` functions return `Result<_>`. Failures (permission denied, path no longer exists, cross-volume move failure, etc.) surface via the pane's existing `set_status(StatusLevel::Error, ...)` (`state.rs:233`) rather than a new mechanism — this matches how other explorer errors are already reported to the user.

## 5. Testing

- Unit tests in `chronos-fm-services` for `ops.rs` already exist (13 `#[test]` functions covering `copy_path`, `move_path`, `rename_in_place`, `create_dir`, `trash_path`, `delete_permanent`, `would_conflict`, `unique_name`) — no new coverage needed there.
- New unit tests for: selection normalization on right-click (unselected row → single-select; already-selected row → selection preserved), clipboard copy/cut/paste round-trip against a temp directory, and rename validation (empty name, name collision — reuses `unique_name`/`would_conflict` from `ops.rs`).
- No GUI screenshot testing is set up for this crate; per `verification-before-completion`, this will be called out explicitly as unverified-by-screenshot when the implementation is reported done, unless a headless GUI smoke path is found during implementation.

## 6. Non-Goals (this pass)

- Drag-and-drop
- "Open With…" / application picker
- Properties/Info panel
- Delete-permanently shortcut
- Native OS clipboard interop for Copy/Cut (paste *within* chronos-fm only; "Copy Path" does use the system clipboard since that's just text)
- Multi-select paste-into-specific-row disambiguation
