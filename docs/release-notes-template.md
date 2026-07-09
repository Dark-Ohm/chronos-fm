# Release Notes Template — v0.1.0-cachy1 (CachyOS / Hyprland Preview)

This template documents the **manual Hyprland testing checklist** and **known issues** for the CachyOS preview release of chronos-fm.

---

## Release Information

| Field | Value |
|-------|-------|
| **Version** | `v0.1.0-cachy1` |
| **Target** | CachyOS (Hyprland / Wayland) |
| **Date** | `{{DATE}}` |
| **Built From** | `{{GIT_SHA}}` (`{{BRANCH}}`) |
| **GPUI Commit** | `{{GPUI_COMMIT}}` |

---

## Testing Checklist (Manual Verification)

> **Note:** These tests require a CachyOS / Hyprland (Wayland) environment with GUI. CI cannot run them.

| # | Test | Command | Expected | Status |
|---|------|---------|----------|--------|
| 1 | Build release binary | `cargo build --release --locked -p chronos-fm` | Binary at `target/release/chronos-fm` | ☐ PASS / ☐ FAIL |
| 2 | Version flag | `./target/release/chronos-fm --version` | Prints version | ☐ PASS / ☐ FAIL |
| 3 | **Hyprland (Wayland)** — Window opens | `WINIT_UNIX_BACKEND=wayland ./target/release/chronos-fm` | Window appears, no crash | ☐ PASS / ☐ FAIL |
| 4 | File listing works | (in window) navigate directories | Files/folders listed | ☐ PASS / ☐ FAIL |
| 5 | Clipboard: Copy path | Right-click → Copy → `wl-paste` | Path in clipboard | ☐ PASS / ☐ FAIL |
| 6 | Focus: Alt+Tab away/back | Alt+Tab away, then back | Search field focused | ☐ PASS / ☐ FAIL |
| 7 | HiDPI scaling | `WINIT_SCALE_FACTOR=2 ./target/release/chronos-fm` | 2× scaled window | ☐ PASS / ☐ FAIL |
| 8 | **X11 (XWayland / native X)** — Window opens | `WINIT_UNIX_BACKEND=x11 ./target/release/chronos-fm` | Window appears | ☐ PASS / ☐ FAIL |
| 9 | X11: File listing works | (in window) navigate | Files listed | ☐ PASS / ☐ FAIL |
| 10 | X11: Clipboard works | Right-click → Copy | Path in clipboard | ☐ PASS / ☐ FAIL |
| 11 | X11: Focus works | Alt+Tab away/back | Search focused | ☐ PASS / ☐ FAIL |
| 12 | **Vulkan backend** | `WGPU_BACKEND=vulkan ./target/release/chronos-fm --version` | Prints version, no crash | ☐ PASS / ☐ FAIL |
| 13 | **OpenGL backend** | `WGPU_BACKEND=gl ./target/release/chronos-fm --version` | Prints version, no crash | ☐ PASS / ☐ FAIL |
| 14 | Dependency verification | `./script/verify-deps.sh` | All checks pass | ☐ PASS / ☐ FAIL |

**Tester:** `{{TESTER_NAME}}`  
**Date:** `{{TEST_DATE}}`  
**Environment:** `{{ENV_DESCRIPTION}}` (e.g., "CachyOS 2024-12, Hyprland 0.45, NVIDIA 560.xx, Wayland")

---

## Known Issues (CachyOS / Hyprland)

The following issues are **known and documented** for this preview release. They stem from upstream GPUI limitations and the Wayland protocol.

| # | Issue | Workaround | Upstream Tracking |
|---|-------|------------|-------------------|
| 1 | **Clipboard**: `Ctrl+C` may not copy; use right-click → Copy | Right-click context menu → Copy | [GPUI #50406](https://github.com/zed-industries/zed/issues/50406) |
| 2 | **Window focus**: `Alt+Tab` away and back may not restore focus to search field immediately | Click into window or press `Tab` | GPUI upstream (in progress) |
| 3 | **HiDPI**: No dynamic scaling; must set `WINIT_SCALE_FACTOR=N` at launch | Launch with `WINIT_SCALE_FACTOR=2` (or desired factor) | GPUI / winit upstream |
| 4 | **NVIDIA Wayland**: Flickering or black window on some NVIDIA drivers | Add `nvidia-drm.modeset=1` to kernel parameters | NVIDIA / Wayland known issue |
| 5 | **Drag-and-drop**: File drag-out to other Wayland apps may not work | Copy path via context menu instead | GPUI / winit upstream |
| 6 | **IME / Input Method**: Non-Latin input (e.g., Japanese, Korean) may not work | Use system-level IME workaround | GPUI upstream |

---

## Verification Commands

```bash
# 1. Build release
cargo build --release --locked -p chronos-fm
./target/release/chronos-fm --version

# 2. Test on Hyprland (Wayland)
WINIT_UNIX_BACKEND=wayland ./target/release/chronos-fm

# 3. Test on X11 / XWayland
WINIT_UNIX_BACKEND=x11 ./target/release/chronos-fm

# 4. Test GPU backend selection
WGPU_BACKEND=vulkan ./target/release/chronos-fm --version
WGPU_BACKEND=gl ./target/release/chronos-fm --version

# 5. Verify dependencies
./script/verify-deps.sh
```

---

## Release Notes (Markdown for GitHub Release)

```markdown
# chronos-fm v0.1.0-cachy1 — CachyOS / Hyprland Preview

## What's New
- First preview build targeting CachyOS with Hyprland (Wayland)
- Native Wayland support via GPUI
- File explorer with directory navigation, copy path, and basic operations

## Known Issues
- **Clipboard**: `Ctrl+C` may not work; use right-click → Copy (GPUI upstream #50406)
- **Window focus**: Alt+Tab may not restore focus immediately (GPUI upstream in progress)
- **HiDPI**: Use `WINIT_SCALE_FACTOR=N` at launch (no dynamic scaling yet)
- **NVIDIA Wayland**: Add `nvidia-drm.modeset=1` to kernel params if flickering
- **Drag-and-drop**: File drag-out to other Wayland apps may not work
- **IME**: Non-Latin input methods may not work

## Installation
```bash
# From AUR (when published)
yay -S chronos-fm-cachyos

# Or run the binary directly
./chronos-fm
```

## Testing
See [docs/release-testing-checklist.md](docs/release-testing-checklist.md) for the full manual test matrix.

## Feedback
Report issues at: https://github.com/chronos-fm/chronos-fm/issues
```

---

## Sign-off

| Role | Name | Signature | Date |
|------|------|-----------|------|
| Release Engineer | | | |
| QA (Manual) | | | |
| Maintainer | | | |