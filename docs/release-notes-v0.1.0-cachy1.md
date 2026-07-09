# chronos-fm v0.1.0-cachy1 Release Notes

**Release Date:** 2026-07-09  
**Version:** 0.1.0-cachy1  
**Target:** CachyOS (Arch-based, x86-64)  
**Git SHA:** `d1dabb2`  
**GPUI Commit:** `69e2130295c2649963eb639fc70b4f2ee8ea1624` (gpui-v0.2.2)

---

## Overview

This is the **first CachyOS-native release** of chronos-fm — a fast, extensible, plugin-ready file launcher and explorer built on GPUI (via wgpu). This release packages chronos-fm as a native CachyOS application with optimized build flags, full Wayland/Hyprland support, and desktop integration.

---

## Key Features

- **GPUI wgpu backend** — Modern Vulkan-based rendering (no legacy Blade backend)
- **Wayland-first** — Native support for Hyprland, GNOME, KDE Plasma, and other Wayland compositors
- **X11 fallback** — Works on X11 and XWayland
- **HiDPI support** — Manual scaling via `WINIT_SCALE_FACTOR` and `CHRONOS_FM_SCALE` (until GPUI upstream adds dynamic scaling)
- **Desktop integration** — `.desktop` entry, hicolor icons, MIME type (`inode/directory`), `xdg-open` handler
- **CachyOS-optimized** — LTO=thin, single codegen unit, panic=abort, x86-64-v3 baseline
- **Vulkan drivers** — AMD (vulkan-radeon), Intel (vulkan-intel), NVIDIA (nvidia-utils)

---

## Installation

### AUR (Recommended)
```bash
yay -S chronos-fm-cachy
# or
paru -S chronos-fm-cachy
```

### Manual (makepkg)
```bash
git clone https://aur.archlinux.org/chronos-fm-cachy.git
cd chronos-fm-cachy && makepkg -si
```

---

## Configuration

Create `~/.config/chronos-fm/config.toml`:

```toml
[window]
# Scale factor (alternative to WINIT_SCALE_FACTOR)
# scale_factor = 1.5

[search]
index_hidden = true
```

### HiDPI Scaling (Workaround until GPUI upstream lands dynamic scaling)

```bash
# 2x scaling (e.g., 4K monitor on 27")
WINIT_SCALE_FACTOR=2.0 chronos-fm

# 1.5x scaling (fractional)
WINIT_SCALE_FACTOR=1.5 chronos-fm

# Internal scale (future GPUI integration)
CHRONOS_FM_SCALE=2.0 chronos-fm
```

---

## Minimum System Requirements

| Component | Requirement |
|-----------|-------------|
| OS | CachyOS / Arch Linux (x86-64-v3 baseline) |
| GPU | Vulkan 1.1+ capable (AMD/Intel/NVIDIA) |
| Display Server | Wayland (Hyprland/GNOME/KDE) or X11/XWayland |
| RAM | 256 MB (recommended 512 MB+) |
| Storage | 50 MB for binary + config |

---

## Known Issues

| Issue | Workaround | Upstream Tracking |
|-------|------------|-------------------|
| Clipboard (Ctrl+C) may not work on Wayland | Use right-click → Copy | GPUI PR #50406 |
| Window focus may not restore after Alt+Tab | Click window to refocus | GPUI in progress |
| HiDPI: No dynamic scaling | Use `WINIT_SCALE_FACTOR=N` | GPUI FIXME |
| NVIDIA + Wayland flicker | Kernel param `nvidia-drm.modeset=1` | NVIDIA driver |
| Drag & Drop (Wayland) | Use XWayland fallback | wgpu/winit |
| IME/Text input (Wayland) | Use XWayland | GPUI text-input WIP |

---

## Verification Steps

```bash
# Verify version
chronos-fm --version
# → chronos-fm 0.1.0

# Check dependencies
./script/verify-deps.sh

# Test on Hyprland
WINIT_UNIX_BACKEND=wayland chronos-fm

# Test X11 fallback
WINIT_UNIX_BACKEND=x11 chronos-fm

# Test HiDPI
WINIT_SCALE_FACTOR=2.0 chronos-fm --version

# Test Vulkan backend selection
WGPU_BACKEND=vulkan chronos-fm --version
WGPU_BACKEND=gl chronos-fm --version
```

---

## Build Information

- **Rust**: 1.85+
- **Cargo Profile**: release (LTO=thin, codegen-units=1, panic=abort, strip=debuginfo)
- **GPUI**: `69e2130` (gpui-v0.2.2 tag, zed-industries/zed)
- **GPUI-component**: 0.5 (crates.io)
- **RUSTFLAGS**: `-C opt-level=3 -C lto=thin -C codegen-units=1 -C link-arg=-fuse-ld=lld -C panic=abort`

---

## Reporting Issues

File issues at GitHub with label `cachyos` and include output of:
```bash
./script/verify-deps.sh
```

---

## License

MIT — see [LICENSE](https://github.com/Dark-Ohm/chronos-fm/blob/main/LICENSE)