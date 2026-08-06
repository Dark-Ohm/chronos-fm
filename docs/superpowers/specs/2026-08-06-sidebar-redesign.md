# T015 — Sidebar Redesign: Design Spec

**Date:** 2026-08-06
**Source:** `docs/orchestration/tasks/active/T015-sidebar-redesign.md`
**Approach:** A — Unified Places list (Dolphin parity)

## 0. Current State (Problems)

`crates/chronos-fm-pages/src/explorer/view/sidebar.rs`:

1. **Dead top block** (l.28-31): `Home`/`Favorites`/`Recent`/`Trash` — `sidebar_item()` has no `on_click`, only `hover`. Four non-functional buttons.
2. **Duplicate `Home`**: exists in both the dead top block and the working Folders section.
3. **Three mismatched cards**: dead block → `elevated_card("Folders")` → `elevated_card("Devices")`. No unified rhythm.
4. **Device actions as raw text**: `"unmount"` / `"eject"` in `text_xs` + `muted`, no border/background/icon. Reads as one phrase with the volume label.
5. **Font size imbalance**: volume name large, actions `text_xs` — actions visually disappear.
6. **I/O in render**: `get_shortcuts()` does 4 `Path::exists()` calls per frame.

## 1. Target State

```
Sidebar
└── Places (single elevated_card)
    ├── section_header("Places")
    ├── [Folders]
    │   ├── 🏠 Home        → change_dir($HOME)
    │   ├── 📁 Desktop     → change_dir($HOME/Desktop)   (if exists)
    │   ├── 📁 Downloads   → change_dir($HOME/Downloads)  (if exists)
    │   ├── 📁 Documents   → change_dir($HOME/Documents)  (if exists)
    │   └── 📁 Pictures    → change_dir($HOME/Pictures)   (if exists)
    ├── [Divider]  (only if devices present)
    └── [Devices]
        ├── 💾 VTOYEFI [/run/media/...]   [⏏] [⬆]  (hover-revealed icons)
        └── 💾 Ventoy  [not mounted]      [⏏]       (hover-revealed icons)
```

## 2. Key Design Decisions

### 2.1 Dead items — removed, tracking tickets for future

`Home`/`Favorites`/`Recent`/`Trash` top block deleted. `Home` lives in the Folders list (as today). `Favorites`, `Recent`, `Trash` will be separate tracking tickets; when their backends exist, they'll be added back as rows in the unified Places list using the same `sidebar_row()` component.

### 2.2 Unified `sidebar_row()` component

One function renders all rows — folders and devices alike:

- **Layout:** `icon(16px) | label(flex_1) | right_zone`
- **Height:** uniform (40px target, matching ListItem)
- **Hover:** `bg_hover` background
- **Click:** full-row clickable. Folders → `change_dir`. Mounted devices → `change_dir(mount_point)`. Unmounted → `mount_and_navigate`.

### 2.3 Device actions — icons on hover

Replaces raw text `"unmount"`/`"eject"`:

| Action | Icon | Visibility | Click |
|---|---|---|---|
| Unmount | `IconName::Eject` (rotated or `ArrowUpFromLine`) | Hover only | `unmount(object_path)` + `stopPropagation` |
| Eject | `IconName::Eject` | Hover only, only for removable with `drive_object_path` | `eject(object_path, drive_object_path)` + `stopPropagation` |

Icons: 16px, same colour as label on hover (`fg`), `muted` otherwise. Each icon has its own `on_click` with `cx.stop_propagation()` so clicking eject/unmount doesn't trigger the row's navigate/mount handler.

### 2.4 In-flight mount guard

Double-click protection: `DeviceStore::mount_and_navigate` already has an internal guard (T008 post-review fix). No additional changes needed in sidebar.

### 2.5 Cache shortcuts

`get_shortcuts()` currently calls `Path::exists()` on every render frame. Fix: compute once at pane construction, store in `ExplorerPane`, invalidate only on HOME change (practically never). The shortcuts list becomes a field on `ExplorerPane` populated in `build()`.

### 2.6 Right padding

Device action icons must not touch the card edge. The `elevated_card`'s content area gets consistent right padding (already `px(8.0)` on child list; ensure the zones div doesn't overflow it).

## 3. Component Contract

### `fn render(page, window, cx) -> impl IntoElement`

Replaces the three-section layout (dead block + Folders card + Devices card) with a single `elevated_card` containing `section_header("Places")` + unified list.

### `fn unified_list(page, cx) -> impl IntoElement`

Iterates over `page.shortcuts` (cached) + `DeviceStore` devices, rendering each as a `sidebar_row()`.

### `fn sidebar_row(icon, label, subtitle, on_click, actions) -> impl IntoElement`

Generic row: icon | label(+subtitle) | actions(vec of icon+click pairs). Actions revealed on hover via `.hover()` + `.visible_on_hover()` or CSS opacity trick.

### `fn device_actions(device, backend, cx) -> impl IntoElement`

Returns the right-zone for a device row: mount status indicator + unmount icon (if mounted) + eject icon (if removable).

## 4. What's NOT in scope

- Favorites/Recent/Trash backend implementation (separate tracking tickets)
- DeviceStore API changes (render-layer only)
- Collapsible sections / accordion
- Drag-and-drop reordering

## 5. Files

| File | Change |
|---|---|
| `crates/chronos-fm-pages/src/explorer/view/sidebar.rs` | Rewrite: ~200 lines net (−35 dead code, +85 unified row, +50 device icons) |
| `crates/chronos-fm-pages/src/explorer/state.rs` | Add `shortcuts: Vec<(String, String)>` field, populate in `build()` |
| `crates/chronos-fm-pages/src/explorer/navigation.rs` | Invalidate shortcuts on HOME change (practically a no-op; field added for completeness) |

## 6. Verification

- `cargo check -p chronos-fm-pages` clean
- Live run: no dead buttons in sidebar, unmount/eject icons appear on hover, don't touch card edge, sections read as one rhythm
- Click every folder row → navigates correctly
- Click device row → mount (if unmounted) or navigate (if mounted)
- Click eject icon → `stopPropagation`, eject called
- Click unmount icon → `stopPropagation`, unmount called
- `cargo test --workspace` green
