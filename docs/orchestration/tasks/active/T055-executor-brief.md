# T055 — Executor brief: Open terminal here

**Ticket:** `active/T055-open-terminal-here.md` · **Epic:** T048
**Priority:** P2
**Revision 2** — architect-reviewed; v1 had an unlocked module path, a
missing call-site (grid), and unstamped decisions. Fixed below.

## Module (locked, don't leave it open)

- **Pure launch helper**: `crates/chronos-fm-services/src/terminal.rs`
  (new) — resolves which terminal binary to use and builds the argv
  `Command`, no GPUI dependency, unit-testable standalone (mirrors how
  `mime.rs` sits next to the thing it launches, but argv-safe — see the
  wall below).
- **Pane wiring**: `ExplorerPane::open_terminal_here(path: &str, cx)` in
  `crates/chronos-fm-pages/src/explorer` (put it in `file_ops.rs` next to
  the other pane-mutating actions, or a new small file if you prefer —
  the service call is what must be centralized, the pane method is a
  thin wrapper that reports status).

## Call sites — all four, not just the row (v1 of this brief missed grid/list)

| Surface | Path | Menu built by |
|---|---|---|
| List row (file/folder) | `row.rs:180` → `open_context_menu` | `file_menu_items` |
| Grid tile (file/folder) | `grid.rs:137` → `open_context_menu` | `file_menu_items` |
| Empty list area | `list.rs:66` → `open_context_menu_for_directory` | `directory_menu_items` |
| Empty grid area | `grid.rs:45` → `open_context_menu_for_directory` | `directory_menu_items` |

`directory_menu_items` (`context_menu.rs:288-339`) is the empty-area menu
(New Folder / Paste / Refresh) — add "Open Terminal Here" there using
`self.cwd` (already on `ExplorerPane`, `state.rs`), copying the
`menu_row(...).on_mouse_down(cx.listener(...))` shape used for "Refresh"
(`:325-336`). This one path covers **both** empty-area call sites (list
and grid) automatically, since they both build their menu through
`directory_menu_items`.

`file_menu_items` (`:98+`) is the per-row menu — this is the one that
needs folder detection (next section), and it's shared by **both** row
and grid call sites the same way, so one fix covers both surfaces.

## Folder detection — use the listing metadata you already have, don't stat the filesystem

v1 of this brief offered `is_dir` on `ContextMenuState` vs. a raw
`Path::is_dir()` stat as equal options. They're not — **prefer the
metadata that's already in hand**:

- `FileEntryDto.kind` (`chronos-fm-models/file_entry.rs:16`) is a
  `String` — `"dir"` / `"file"` / etc. — and `row.rs:81` already
  compares against it (`item.kind == "dir"`, used for drop targets).
  `filtered_entries[index]` on the pane has this for free at the moment
  `open_context_menu`/`open_context_menu(_for_directory)` is called.
- **Locked:** thread `is_dir` (derived from `item.kind == "dir"`) from
  the row/grid call site into `ContextMenuState::for_file` (new
  parameter), through `open_context_menu` (`navigation.rs:316-326`). Do
  not add a filesystem stat for this — the listing already paid for that
  metadata.
- **Symlinks (v1 stamp):** only show "Open Terminal Here" on the folder
  row when `kind == "dir"` exactly. Don't resolve symlink targets to
  decide — a symlink pointing at a directory is out of scope for v1,
  simpler and safer than guessing wrong on a broken link.

## The wall: don't copy `mime.rs`'s `sh -c` pattern — it's exactly what Must #4 forbids

`crates/chronos-fm-services/src/mime.rs` (`open_with`/`open_default`,
`:86-142`) is the closest existing "launch an external program, detached,
don't block the UI thread" precedent — and it goes through
`Command::new("sh").arg("-c").arg(&script)` with a hand-built,
`shell_escape()`-escaped command string. Not literally injectable (it
escapes), but the ticket explicitly requires **argv-array construction,
not `sh -c` string-building** for this new code. Do not port this
pattern just because it's the nearest example.

**Locked approach:**

```rust
std::process::Command::new(&terminal_bin)
    .current_dir(&cwd)   // preferred: terminal-agnostic, no per-terminal
                          // flag dispatch needed for the common case —
                          // most terminals start a shell that inherits
                          // the launching process's cwd.
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()   // NOT .status() — spawn() doesn't block; the child is
               // left running and reparents to init on exit, same
               // detach effect as mime.rs's `sh -c "... &"` without a
               // shell at all.
```

**Verify `current_dir()` alone works live before building a per-terminal
flag table.** If (and only if) a specific terminal in the candidate list
doesn't respect `current_dir()` in your live test, add that terminal's
own flag (`kitty --directory`, `alacritty --working-directory`, `foot
-D`, `gnome-terminal --working-directory=`) as a documented exception,
not a default.

**Detach/zombie note (soft, documented not solved):** children left
running via `spawn()` without `wait()`/`try_wait()` can become zombies
once they exit while this process keeps running. Terminals are normally
long-lived (user closes them), so this is low real-world risk. Document
this as the accepted v1 tradeoff — do not reintroduce `sh -c "... &"`
just to dodge it, that's a strictly worse tradeoff (shell injection
surface) for a smaller problem (zombie reaping).

## `$TERMINAL` must be a single binary, never a command line

Some users set `TERMINAL="kitty --single-instance"` expecting a shell to
split it. **Stamped:** `$TERMINAL` is treated as a single executable
path/name only. If it contains whitespace (looks like a command line
with arguments), treat it as invalid and fall through to the candidate
list — report this in status if nothing ultimately launches. **Do not
split-and-argv it either** — that's guessing at shell-quoting semantics
for a value that was never meant to be parsed. A bare binary name/path
only.

## Must (from the ticket, with v1 scope locked)

1. Context menu: empty listing (pane `self.cwd`) **and** folder row
   (that row's path) — all four call sites in the table above.
2. Terminal resolution order: `$TERMINAL` (single-token, see above) →
   `kitty` → `alacritty` → `foot` → `gnome-terminal` → `xterm`. Prefer a
   PATH-lookup check (`std::env::var("PATH")` + join + exists, or
   equivalent) over attempt-and-catch spawn failures, so the resolution
   logic is a pure function testable without actually spawning anything.
3. Failure → status bar error naming the command that failed (e.g.
   "Failed to launch kitty: No such file or directory").
4. **Argv array, never `sh -c` with path concatenation** — see wall
   above; `current_dir()` + bare `Command::new(bin)`.

## v1 scope — locked, not open questions

- **Context menu only.** No `Ctrl+Alt+T` keybinding this round — the
  ticket's "or configurable later" already defers it; don't add a
  keybinding nobody asked to land yet.
- **No Settings Terminal wiring.** `settings.rs:575-584`'s Terminal
  category stays "not wired yet" — confirmed fully unwired, matches the
  ticket's own "v1 can skip settings UI" allowance.
- **No in-app embedded terminal.** This ticket spawns an external
  terminal emulator; it does not build a terminal-in-a-pane.
- **Spec authority:** `docs/explorer-essentials.md` has zero terminal
  mentions — the ticket (`T055-open-terminal-here.md`) is the sole
  product authority here, don't go hunting for a missing spec section.

## Tests (mandatory)

- Pure resolution-order logic: given `$TERMINAL` set (valid single-token
  / invalid multi-token) vs. unset, and a fake "is this binary on PATH"
  predicate, assert which candidate gets chosen — no real spawning.
- Argv construction: given a cwd, assert the built `Command` carries the
  path via `current_dir()` (or an explicit arg for a documented
  exception terminal), never interpolated into a shell string — this is
  the test that would catch a `mime.rs`-style regression if someone
  copied that pattern by mistake.
- `cargo test --workspace` green.

## Done when

Live proof for at least the empty-area case (grim of the menu + `pwd`
output visible in the opened terminal, or a log line with the resolved
binary + cwd it launched with); folder-row case proven the same way if
implemented. Report with Claim → Evidence blocks covering all four call
sites (which were wired, and confirm none were silently skipped). No
self-ACCEPT, ticket stays in `active/` until architect stamp.

## Related

T048 (epic) · `mime.rs` (precedent to learn from, not copy) ·
`settings.rs:575-584` (confirmed out of scope for v1) ·
`docs/explorer-essentials.md` (no terminal section — ticket is sole
authority)
