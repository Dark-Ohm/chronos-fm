# T052 — Drag-and-drop external (Chronos-FM ↔ other apps) — report

**Epic:** T048 · **Ticket:** `active/T052-dnd-external.md` · **Date:** 2026-08-11
**Status:** implemented, unit + live verified — awaiting architect stamp

## 1. Design gate (deliverable before Source patches)

`docs/superpowers/specs/2026-08-11-dnd-external-design-gate.md`

Key findings:
- **Drop-in** needed **zero** Source changes: gpui already translates the OS
  `text/uri-list` file drop into the internal drag pipeline
  (`window.rs` `FileDropEvent::Entered` → `active_drag` with an
  `ExternalPaths` payload — the same mechanism T051 uses). The app simply
  had **no handlers** for that payload type, so external drops were no-ops.
- **Drag-out** did not exist in Source: `wl_data_source` was used only for
  clipboard; no `start_drag` call, no `PlatformWindow` API.

## 2. What was built

### Source (what / why / зачем)
| Change | Why |
|---|---|
| `gpui::PlatformWindow::start_external_drag` (default no-op) + `Window::start_external_drag` | public API so the app can initiate an OS drag without platform knowledge |
| `gpui_linux` Wayland `WaylandWindow::start_external_drag` → `wl_data_source` offering `FILE_LIST_MIME_TYPE`, stored in `DragState.source_paths`, `data_device.start_drag` with `MousePress` serial | drag-out source role; no invented API, straight Wayland |
| `WlDataSource::Send` writes a **percent-encoded** (RFC 8089) `file://` uri-list; `Cancelled` clears + destroys | valid URIs for paths with spaces / reserved chars; receivers decode via `Url::to_file_path` |
| X11 backend untouched (receive-only) | XDND receive already existed; drag-out left as follow-up |

### App (`crates/chronos-fm-pages/src/explorer`)
- `dnd.rs`: `can_accept_external_drop`, `can_accept_listing_cwd_external_drop`,
  `begin_external_drop`, `complete_external_drop` — mirror the T051 API for
  payloads with **no source pane**; reuse `validate_drop`, `transfer_paths`,
  `unique_name`, `finish_target_drop`, `drop_failure_status` (extracted from
  `complete_file_drop`, behavior-preserving).
- `listing.rs` / `row.rs` / `grid.rs`: `ExternalPaths` handlers
  (`on_drag_move` cursor, `drag_over` highlight, `on_drop` transfer) on the
  **cwd background** and on **folder rows/tiles** (list + grid).
- One **merged `can_drop`** predicate per element covering both `FileDrag` and
  `ExternalPaths` — `can_drop` is a *single* gate per element, a second call
  overwrites the first (found during testing; the initial two-`can_drop`
  version silently disabled in-app drops → caught by T051 e2e tests).
- `on_drag` constructors (row/tile) now also call
  `window.start_external_drag(paths)` — drag-out wired into the existing T051
  drag initiation. Provider (S3) panes never reach `on_drag`
  (`file_drag_for_item` returns `None`), so no virtual drag-out.

### Policies
Move default; **Ctrl at drop time = Copy** (same as T051). Name conflicts →
`unique_name` (until T053). External drops land with the drop-time modifier
state (`window.modifiers()`), so Ctrl survives the synthesized mouse-up.

## 3. Tests

- 3 new (`dnd.rs`): `external_can_accept_requires_local_payload_and_directory`,
  `external_drop_moves_files_into_pane_cwd`, `external_drop_ctrl_copies_preserving_source`.
- `cargo test -p chronos-fm-pages`: **183 passed** (180 existing incl. all T051
  e2e in-app drag tests + 3 new).
- `cargo test --workspace`: **453 passed, 0 failed** (was 450 before T052).
- Clippy: no new warnings in changed files (3 remaining in `dnd.rs` are
  pre-existing T051 code).
- Release build + `cargo check -p gpui_linux`: clean.

## 4. Live verification (Hyprland 0.56, Wayland)

Manual run on the live desktop, `class=chronos-fm`:
- **Drop-in (Thunar → Chronos-FM):** dragging files from a `Thunar /tmp/t052_src`
  window into the Chronos-FM listing — **file landed in the target**, confirmed.
- **Drag-out (Chronos-FM → Thunar):** dragging files out of Chronos-FM into
  Thunar — **file landed in the target**, confirmed.
- Both directions move the real payload (uri-list over `wl_data_device`),
  multi-file selection supported (all selected paths offered).

Setup script used: `/tmp/t052_drag_test.sh` (fake `HOME`/XDG isolate the app;
Thunar as the other FM).

## 5. Known limitations / residuals (honest)

1. **Same-window in-app drag on Wayland (T051) not re-verified live** after
   wiring drag-out. Cross-window both directions are proven; the same-surface
   path relies on the compositor delivering `wl_data_device` events back to the
   source surface (standard behavior; the payload stays `FileDrag` because
   `window.rs` doesn't replace an active drag, so no double transfer). e2e
   tests pass (test platform: `start_external_drag` no-op). **Recommended
   follow-up: one manual same-window drag check on Wayland.**
2. **X11 drag-out unsupported** — `start_external_drag` is a no-op on X11
   (receive-only backend). Wayland proof covers the supported direction per the
   ticket's partial-ACCEPT clause.
3. **Header/breadcrumb area has no `ExternalPaths` drop handler** — external
   drops onto the breadcrumb are ignored (only listing background + folder
   rows/tiles accept). Minor; can be added with the same pattern.
4. `DragState.source_paths` lingers after a successful drop — this wayland-client
   version has no `Event::Finished` variant (attempted cleanup reverted); a
   finished source never sends again, so it is harmless until the next
   drag-out/cancel.
5. Non-UTF-8 filenames: encoded byte-wise (`%XX`) via `OsStrExt`, so the round
   trip survives; no `display()` lossy conversion on send.

## 6. Files changed

- `Source/gpui/src/platform.rs` — `PlatformWindow::start_external_drag` (default)
- `Source/gpui/src/window.rs` — `Window::start_external_drag`
- `Source/gpui_linux/src/linux/wayland/window.rs` — Wayland override
- `Source/gpui_linux/src/linux/wayland/client.rs` — `start_external_drag`,
  `WlDataSource::Send` uri-list + `percent_encode_path`, `Cancelled` cleanup
- `crates/chronos-fm-pages/src/explorer/dnd.rs` — external drop core + 3 tests
- `crates/chronos-fm-pages/src/explorer/view/listing.rs` — cwd ExternalPaths handlers
- `crates/chronos-fm-pages/src/explorer/view/listing/row.rs` — folder-row handlers + drag-out
- `crates/chronos-fm-pages/src/explorer/view/listing/grid.rs` — folder-tile handlers + drag-out
- `docs/superpowers/specs/2026-08-11-dnd-external-design-gate.md` — design gate

Evidence: `cargo test` runs above · manual live drags (client-observed, both
directions, files landed) · no grim needed beyond the manual run.
