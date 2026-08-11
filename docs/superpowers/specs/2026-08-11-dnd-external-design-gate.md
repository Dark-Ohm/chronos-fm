# T052 — External DnD: design gate (what exists / what must be added)

**Date:** 2026-08-11. **Ticket:** `docs/orchestration/tasks/active/T052-dnd-external.md`.
Research-first note, per the ticket's design gate: before large `Source/`
patches — which gpui/platform API exists today, what must be added, what/why/зачем.

## What already exists in the fork (Source/gpui) — drop-in is platform-ready

**Receive side (external file drop → app) is fully implemented on BOTH Linux
backends and translated into the same internal drag machinery T051 uses.**

| Layer | What exists | Where |
|---|---|---|
| Wayland protocol | `wl_data_device` bound; `Enter` reads `text/uri-list`, parses `file://` → `FileDropEvent::Entered { position, paths: ExternalPaths }`; non-file drops filtered; `Motion`→Pending, `Leave`→Exited, `Drop`→Submit + `finish()` | `Source/gpui_linux/src/linux/wayland/client.rs` (wl_data_device dispatch; `state.drag`) |
| X11 protocol | Full XDND state machine (XdndEnter/Leave/Position/Drop + SelectionNotify → uri-list) → same `FileDropEvent` stream | `Source/gpui_linux/src/linux/x11/client.rs` (`Xdnd` state, `atoms.XdndEnter` etc.) |
| gpui core | `FileDropEvent { Entered/Pending/Submit/Exited }`, `ExternalPaths(pub SmallVec<[PathBuf; 2]>)`; `PlatformInput::FileDrop` | `Source/gpui/src/interactive.rs:685,703` |
| Window translation | `FileDrop::Entered` → `cx.active_drag = AnyDrag { value: Arc<ExternalPaths>, … }` + synthetic `MouseMove`(Left pressed); `Submit` → `MouseUp`; `Exited` → clears active_drag. **External drops become internal drags — the exact `active_drag` mechanism T051 in-app DnD uses.** | `Source/gpui/src/window.rs:4742` |

**Conclusion:** drop-in needs **no new platform protocol code**. The
uri-list already arrives as an `ExternalPaths` drag payload at the element
level, on the same pipeline as `FileDrag`.

## What is missing

### 1. App side (Chronos-FM): external payload not accepted (drop-in)
Today the explorer's drop handlers are **typed to the internal `FileDrag`**
(payload carries the source pane): `row.rs:417 on_drag_move::<FileDrag>`,
`row.rs:464 on_drop(... &FileDrag ...)` (same in `grid.rs`). An
`ExternalPaths` drag doesn't match the type, so **external drops silently
do nothing** right now.

**What to add (Chronos-FM, no Source change needed for drop-in):**
- A listing-level handler accepting `ExternalPaths` payloads
  (`on_drag_move::<ExternalPaths>` + `on_drop` with a typed closure).
- Validate: every path is a real local file/dir (`Path::exists`), payload
  non-empty, target is a non-provider pane's listing (cwd) or a folder row.
- Policy: **Copy** (matches the platform's advertised `DndAction::Copy` in
  the Wayland Enter handler — moving would violate what the source was
  told), conflict policy `unique_name` (no overwrite) until T053.
- Execute through the shared pipeline `chronos_fm_services::fs::ops::transfer_paths`
  (same as paste/drop T051); failures → status/error, never hang.
- Multi-file: `ExternalPaths` already carries all lines of the uri-list.

### 2. Source/gpui: drag-out (source role) does not exist
`WlDataSource` dispatch exists (`Send`/`Cancelled` handled), and
`create_data_source` is used for **clipboard** only (`client.rs:1048`,
`set_selection`). **No `start_drag` call anywhere** → the app cannot start
an external drag. X11 same story: XDND is receive-only.

**What to add (Source), what / why / зачем:**
- `gpui_linux` Wayland: `Window::start_external_drag(paths)`-style API →
  create `wl_data_source`, offer `text/uri-list` (+ `text/plain` of paths),
  `start_drag` with current serial; on `Send` write the uri-list to the fd.
  — **зачем:** drag-out to editors/chat/desktop (ticket Must #2).
- gpui-core: expose the trigger through `Window`/`Platform` so the app can
  call it from the drag handle (T051's initiation point).
- X11 XDND source role: send `XdndEnter` to targets — optional; this
  machine is Wayland (Hyprland), so Wayland source is the live-proof path.

## Recommendation / decision needed (architect)

1. **Drag-out scope:** implement the Wayland source role (real Source work,
   medium size) for a live drag-out proof — or land drop-in first
   (zero Source changes) and treat drag-out as the follow-up with evidence
   of the supported direction only (ticket's partial-ACCEPT clause)?
2. Everything else (copy-default, unique_name, shared transfer pipeline)
   follows existing policy — no separate decision.
