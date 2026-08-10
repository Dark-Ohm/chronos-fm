# T049 - File-ops keybindings report

**Status:** IMPLEMENTED - awaiting Architect review. Executor does not self-ACCEPT.

## Outcome

The focused explorer pane now handles the built-in file-manager shortcuts from
T049. Keyboard and context-menu entry points share the same rename and delete
paths. Empty selection/missing active row are no-ops, focused text inputs keep
editing shortcuts, and repeated Delete cannot stack confirmation dialogs.

## Claims and evidence

Claim: Ctrl+C, Ctrl+X, and Ctrl+V operate on the current selection and paste
into the pane cwd.

Evidence: `crates/chronos-fm-pages/src/explorer/view.rs:61`, `:65`, `:69` route
to the existing clipboard operations. Root-dispatch tests are at
`crates/chronos-fm-pages/src/explorer/keybindings.rs:74`, `:85`, and `:96`.
`lean-ctx raw "cargo test -p chronos-fm-pages --lib keybindings"` exited 0
with 16 passed, 0 failed. Live Hyprland automation copied `sample.txt` to a
collision-resolved name, then cut/pasted it again in `/tmp/t049-live`;
`docs/orchestration/tasks/report-log/T049-new-folder.png` shows the resulting
`sample (2) (2).txt` beside the original.

Truth base: Chronos-FM | runtime.

Claim: F2 performs inline rename for one selection and Batch Rename for a
multi-selection, using the same decision path as context-menu Rename.

Evidence: `crates/chronos-fm-pages/src/explorer/view.rs:77` and
`crates/chronos-fm-pages/src/explorer/context_menu.rs:215` both call
`rename_selection`, defined at `crates/chronos-fm-pages/src/explorer/rename.rs:37`.
Root-dispatch tests are at `keybindings.rs:141` and `:153`. Live release frames:
`docs/orchestration/tasks/report-log/T049-f2-single.png` and
`docs/orchestration/tasks/report-log/T049-f2-batch.png` (the latter shows three
selected rows and the three-file Batch Rename dialog).

Truth base: Chronos-FM | runtime.

Claim: Delete opens the existing trash confirmation path and repeated Delete
cannot stack confirmations.

Evidence: `crates/chronos-fm-pages/src/explorer/view.rs:83` and
`crates/chronos-fm-pages/src/explorer/context_menu.rs:349` both call
`confirm_delete_selection`, defined at
`crates/chronos-fm-pages/src/explorer/file_ops.rs:106`. The Root-dispatch dialog
test starts at `keybindings.rs:178`. The repeat regression at `:203` opens the
first dialog through Root keystroke dispatch, then models another handler
delivery while that dialog owns focus; it failed with the guard removed and
passed after restoration.
Live frame: `docs/orchestration/tasks/report-log/T049-delete-confirm.png` shows
the three-item trash confirmation; no destructive confirmation was issued.

Truth base: Chronos-FM | runtime.

Claim: Ctrl+A selects every filtered entry.

Evidence: the existing pane selection arm is at
`crates/chronos-fm-pages/src/explorer/view.rs:56`; Root-dispatch coverage starts
at `crates/chronos-fm-pages/src/explorer/keybindings.rs:257`.
`T049-f2-batch.png` shows all three filtered rows selected before F2 opens the
batch dialog.

Truth base: Chronos-FM | runtime.

Claim: Enter activates the current entry through the existing activation path.

Evidence: `crates/chronos-fm-pages/src/explorer/view.rs:87` calls
`activate_entry`; Root-dispatch coverage starts at `keybindings.rs:459`. Live
frame `docs/orchestration/tasks/report-log/T049-enter-directory.png` shows the
path changed from `/tmp/t049-live` to `/tmp/t049-live/New Folder` after Return.

Truth base: Chronos-FM | runtime.

Claim: Ctrl+Shift+N creates a uniquely named folder in the current directory
and starts inline rename.

Evidence: `crates/chronos-fm-pages/src/explorer/view.rs:73` calls the existing
`new_folder` method; Root-dispatch coverage starts at `keybindings.rs:123`.
Live frame `docs/orchestration/tasks/report-log/T049-new-folder.png` shows the
real `New Folder` directory and its focused inline editor in `/tmp/t049-live`.

Truth base: Chronos-FM | runtime.

Claim: Empty panes are safe, focused inputs retain editing keys, and Cmd mirrors
use the same operations.

Evidence: empty-state Root dispatch starts at `keybindings.rs:228`; focused
search, inline-rename, and batch-dialog isolation start at `:278`, `:364`, and
`:416`; search-mode and inline-rename Enter guards start at `:342` and `:401`.
The batch dialog now focuses its first input at `batch_rename.rs:263`.
`view.rs:37` defines the accepted modifier as
`modifiers.platform || modifiers.control`, and file-op arms at `view.rs:56-74`
also require pane focus, so every Ctrl arm accepts the platform/Cmd modifier
without intercepting a child input. Cmd was not live-tested on this Linux host.

Truth base: Chronos-FM.

## Verification

- `lean-ctx raw "cargo test -p chronos-fm-pages --lib keybindings"` - exit 0;
  16 passed, 0 failed.
- `lean-ctx raw "cargo test --workspace"` - exit 0; all workspace unit and doc
  test binaries completed without failure.
- The release build used for live evidence completed before final diff-noise
  cleanup. A final recompilation of `cargo build --release -p chronos-fm`
  exited 101 on the pre-existing `preview.rs:445` reference to undeclared
  release dependency `url`; no T049 file appears in the diagnostic.
- Scoped `git diff --check` over the seven T049 Rust files and report - exit 0.
- Runtime: release binary mapped as `class=chronos-fm`; Hyprland Lua
  `hl.dsp.send_shortcut` targeted `address:0x564168bb2f70` in the isolated
  `/tmp/t049-live` fixture. Five behavior grims plus the initial fixture grim
  are listed above.

## Runtime procedure

The release window was launched with cwd `/tmp/t049-live`. Shortcuts were sent
to that exact window with this command form (the listed modifier/key pairs were
run separately):

```bash
env HYPRLAND_INSTANCE_SIGNATURE=efb50993780079460b0cbed1363e2166a2de1d9f_1786369429_887750879 WAYLAND_DISPLAY=wayland-1 \
  hyprctl repl 'return hl.dispatch(hl.dsp.send_shortcut({ mods = "CTRL", key = "A", window = "address:0x564168bb2f70" }))'
```

The same command used `CTRL` with `C`, `X`, and `V`; `CTRL SHIFT` with `N`;
and an empty modifier with `F2`, `DELETE`, and `RETURN`. The observable sequence
was copy/paste, cut/paste, new folder, single F2, Ctrl+A plus F2, Ctrl+A plus
Delete, and Down plus Return. Delete confirmation was never accepted.

Filesystem observation after automation:

```bash
ls -la /tmp/t049-live
```

Exit 0; visible entries were `sample.txt`, collision-resolved
`sample (2) (2).txt`, and `New Folder`. Hidden `.chronos-fm`, `.config*`, and
`.local` directories belong to the isolated runtime environment.

Known pre-existing warnings remain in the local Source fork and pages crate
(`unexpected_cfgs`, unused/dead code, and future-incompatibility notices).
The unrelated release-only `url` dependency failure above remains for its owner;
T049 focused and workspace tests are green.

## Residual

User-editable keymap registration and Settings UI remain out of scope. T051,
T052, T053, and T054 are unchanged.
