# T015 — Sidebar Redesign: Implementation Plan

**Date:** 2026-08-06
**Spec:** `docs/superpowers/specs/2026-08-06-sidebar-redesign.md`
**Approach:** A — Unified Places list

## Tasks

### Task 1 — Cache shortcuts in ExplorerPane (state.rs + navigation.rs)

- Add `shortcuts: Vec<(String, String)>` field to `ExplorerPane`
- Populate in `ExplorerPane::build()` via `get_shortcuts()`
- Remove `get_shortcuts()` from sidebar.rs (move logic to state.rs as a private fn)
- ~15 lines

### Task 2 — Rewrite sidebar.rs: unified Places list

- Delete dead top block (`sidebar_item()` function + its 4 call sites)
- Delete `render_shortcuts()` function (replaced by unified list)
- New `render()`: single `elevated_card` with `section_header("Places")` + unified list
- New `render_places_list(page, cx)`: iterates `page.shortcuts` + `DeviceStore` devices
- New `place_row()`: icon | label | right_zone — uniform height, `bg_hover` on hover
- Folders: click → `change_dir(path)`
- Devices: click → mounted: `change_dir(mount_point)`, unmounted: `mount_and_navigate`
- ~120 lines

### Task 3 — Device actions as hover-revealed icons

- Replace `text_xs` "unmount"/"eject" labels with `Icon` elements
- Icons: `IconName::Eject` for eject, appropriate icon for unmount
- Visibility: `opacity(0.0)` → `opacity(1.0)` on `.hover()` of parent row
- Each icon has `on_click` with `cx.stop_propagation()`
- Mount status indicator: small coloured dot or text label
- ~60 lines

### Task 4 — Verify

- `cargo check -p chronos-fm-pages` clean
- `cargo test -p chronos-fm-pages -- explorer::` green
- Visual: no dead buttons, icons don't touch edge, unified rhythm

## Global Constraints

- Only `sidebar.rs`, `state.rs` — no changes to `DeviceStore`, `theme`, or `patterns`
- `IconName` enum from `gpui_component` — use existing variants only
- Device row click wiring preserved from T003/T008 (mount/navigate/unmount/eject)
- In-flight mount guard in `DeviceStore::mount_and_navigate` — already exists, not in scope
- Right padding: card content `px(8.0)` minimum, action icons within bounds
