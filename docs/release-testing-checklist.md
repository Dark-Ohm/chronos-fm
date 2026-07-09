# Release Testing Checklist — chronos-fm v0.1.0-cachy1

This checklist must be completed **manually** on a CachyOS / Hyprland (Wayland) system before cutting the `v0.1.0-cachy1` release. CI cannot run GUI tests.

---

## Prerequisites

- [ ] CachyOS (or Arch-based) with Hyprland ≥ 0.45
- [ ] `wl-clipboard` installed (`wl-copy`, `wl-paste`)
- [ ] `winit` with Wayland backend (default on Hyprland)
- [ ] NVIDIA drivers (if applicable) with `nvidia-drm.modeset=1` in kernel cmdline
- [ ] Rust toolchain matching `rust-version` in `Cargo.toml`
- [ ] Repository cloned at the release tag/commit

---

## Step 1: Build Release Binary

```bash
cargo build --release --locked -p chronos-fm
./target/release/chronos-fm --version
```

**Expected:** Binary builds successfully, `--version` prints `chronos-fm 0.1.0-cachy1 ({{GIT_SHA}})`.

- [ ] PASS
- [ ] FAIL — Notes: _______________

---

## Step 2: Hyprland (Wayland) — Core Functionality

Launch in a Hyprland session:

```bash
WINIT_UNIX_BACKEND=wayland ./target/release/chronos-fm
```

| Check | Expected | Result |
|-------|----------|--------|
| Window opens without crash | Visible window, title "chronos-fm" | ☐ PASS / ☐ FAIL |
| File listing works | Current directory contents shown | ☐ PASS / ☐ FAIL |
| Navigate into subdirectory | Listing updates | ☐ PASS / ☐ FAIL |
| Navigate up (parent dir) | Listing updates | ☐ PASS / ☐ FAIL |
| Hidden files toggle (if implemented) | Shows/hides dotfiles | ☐ PASS / ☐ FAIL / ☐ N/A |

---

## Step 3: Hyprland — Clipboard

| Action | Verification | Result |
|--------|--------------|--------|
| Right-click file → **Copy Path** | `wl-paste` outputs the absolute path | ☐ PASS / ☐ FAIL |
| `Ctrl+C` on selected file | `wl-paste` outputs path *(known issue: may fail)* | ☐ PASS / ☐ FAIL / ☐ KNOWN FAIL |
| Copy multiple files | `wl-paste` shows newline-separated paths | ☐ PASS / ☐ FAIL / ☐ N/A |

**Note:** `Ctrl+C` is a known upstream GPUI issue (#50406). Document as known issue if it fails.

---

## Step 4: Hyprland — Window Focus

| Action | Verification | Result |
|--------|--------------|--------|
| Launch chronos-fm, note search field focus | Cursor blinking in search | ☐ PASS / ☐ FAIL |
| `Alt+Tab` away to another window | Focus leaves chronos-fm | ☐ PASS / ☐ FAIL |
| `Alt+Tab` back to chronos-fm | Search field regains focus *(known issue: may not)* | ☐ PASS / ☐ FAIL / ☐ KNOWN FAIL |
| Click into chronos-fm window | Focus restored | ☐ PASS / ☐ FAIL |

**Note:** Focus restoration on Alt+Tab is a known GPUI upstream issue. Document as known issue if it fails.

---

## Step 5: Hyprland — HiDPI Scaling

```bash
WINIT_SCALE_FACTOR=2 WINIT_UNIX_BACKEND=wayland ./target/release/chronos-fm
```

| Check | Expected | Result |
|-------|----------|--------|
| Window opens at 2× scale | UI elements doubled in size | ☐ PASS / ☐ FAIL |
| Text is crisp (not blurry) | No pixelation | ☐ PASS / ☐ FAIL |
| Mouse clicks hit correct targets | Interaction works at scaled coords | ☐ PASS / ☐ FAIL |

**Note:** Dynamic scaling (changing factor at runtime) is not supported. Must set at launch.

---

## Step 6: X11 / XWayland — Core Functionality

```bash
WINIT_UNIX_BACKEND=x11 ./target/release/chronos-fm
```

| Check | Expected | Result |
|-------|----------|--------|
| Window opens | Visible window | ☐ PASS / ☐ FAIL |
| File listing works | Contents shown | ☐ PASS / ☐ FAIL |
| Navigation works | Subdirs, parent dir | ☐ PASS / ☐ FAIL |
| Clipboard (right-click → Copy) | `wl-paste` or `xclip -o` shows path | ☐ PASS / ☐ FAIL |
| Focus (Alt+Tab away/back) | Search field focused | ☐ PASS / ☐ FAIL |

---

## Step 7: GPU Backend Selection

Test both wgpu backends (should not crash on `--version`):

```bash
WGPU_BACKEND=vulkan ./target/release/chronos-fm --version
WGPU_BACKEND=gl ./target/release/chronos-fm --version
```

| Backend | Expected | Result |
|---------|----------|--------|
| `vulkan` | Version printed, no crash | ☐ PASS / ☐ FAIL |
| `gl` (OpenGL) | Version printed, no crash | ☐ PASS / ☐ FAIL |

**Note:** On Hyprland, Vulkan is preferred. OpenGL works via XWayland or native.

---

## Step 8: Dependency Verification

```bash
./script/verify-deps.sh
```

**Expected:** All checks pass (system deps, Rust toolchain, cargo-audit, etc.)

- [ ] PASS
- [ ] FAIL — Notes: _______________

---

## Step 9: Additional Edge Cases (Optional but Recommended)

| Test | Expected | Result |
|------|----------|--------|
| Launch with non-ASCII path (e.g., `~/文档`) | Opens, displays correctly | ☐ PASS / ☐ FAIL / ☐ N/A |
| Very deep directory nesting (50+ levels) | No stack overflow, renders | ☐ PASS / ☐ FAIL / ☐ N/A |
| Directory with 10,000+ files | Loads within reasonable time | ☐ PASS / ☐ FAIL / ☐ N/A |
| Symlink navigation | Follows symlinks correctly | ☐ PASS / ☐ FAIL / ☐ N/A |
| Permission-denied directory | Shows error, doesn't crash | ☐ PASS / ☐ FAIL / ☐ N/A |
| Rapid open/close (5x) | No memory leak, no crash | ☐ PASS / ☐ FAIL / ☐ N/A |

---

## Step 10: NVIDIA-Specific (If Applicable)

| Check | Expected | Result |
|-------|----------|--------|
| `nvidia-drm.modeset=1` in kernel cmdline | Verified via `cat /proc/cmdline` | ☐ PASS / ☐ FAIL / ☐ N/A |
| No flickering on window open/close | Stable rendering | ☐ PASS / ☐ FAIL / ☐ N/A |
| No black window on launch | Content visible | ☐ PASS / ☐ FAIL / ☐ N/A |

---

## Summary

| Category | Total | Pass | Fail | Known Fail | N/A |
|----------|-------|------|------|------------|-----|
| Core (Steps 1-2) | | | | | |
| Clipboard (Step 3) | | | | | |
| Focus (Step 4) | | | | | |
| HiDPI (Step 5) | | | | | |
| X11 (Step 6) | | | | | |
| GPU Backends (Step 7) | | | | | |
| Deps (Step 8) | | | | | |
| Edge Cases (Step 9) | | | | | |
| NVIDIA (Step 10) | | | | | |

**Overall Result:** ☐ **RELEASE READY** / ☐ **BLOCKED** / ☐ **RELEASE WITH KNOWN ISSUES**

---

## Sign-off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Tester | | | |
| Reviewer | | | |
| Release Manager | | | |

---

## Notes / Observations

> Record any unexpected behavior, performance concerns, or qualitative observations here.

```
```