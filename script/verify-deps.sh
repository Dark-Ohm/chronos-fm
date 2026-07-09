#!/usr/bin/env bash
# Verify runtime dependencies for chronos-fm on CachyOS / Arch

set -euo pipefail

echo "=== chronos-fm Dependency Verification ==="
echo

# 1. Vulkan ICD
echo "🔍 Vulkan ICD Loader:"
if command -v vulkaninfo &>/dev/null; then
  vulkaninfo --summary 2>/dev/null | grep -E "(deviceName|driverVersion|Vulkan)" | head -5 || echo "  ⚠️  No Vulkan devices found"
else
  echo "  ❌ vulkaninfo not installed (install vulkan-tools)"
fi
echo

# 2. libvulkan.so.1
echo "🔍 libvulkan.so.1:"
ldconfig -p 2>/dev/null | grep libvulkan.so.1 | head -3 || echo "  ❌ libvulkan.so.1 not found in ldconfig cache"
echo

# 3. Wayland
echo "🔍 Wayland:"
echo "  WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-unset}"
echo "  XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-unset}"
pkg-config --exists wayland-client && echo "  ✅ wayland-client (pkg-config)" || echo "  ❌ wayland-client missing"
echo

# 4. X11 fallback
echo "🔍 X11:"
pkg-config --exists xcb && echo "  ✅ xcb (pkg-config)" || echo "  ❌ xcb missing"
pkg-config --exists xkbcommon && echo "  ✅ xkbcommon (pkg-config)" || echo "  ❌ xkbcommon missing"
echo

# 5. wgpu backend detection
echo "🔍 wgpu Backend Test:"
if command -v chronos-fm &>/dev/null; then
  WGPU_BACKEND=vulkan chronos-fm --version 2>&1 | head -3
else
  echo "  ⚠️  chronos-fm binary not in PATH (build first)"
fi
echo

# 6. GPU vendor detection
echo "🔍 GPU Vendor (lspci):"
lspci -nn | grep -i -E "(vga|3d|display)" | head -3
echo

echo "=== Done ==="