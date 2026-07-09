#!/usr/bin/env bash
# Generates PKGBUILD from template for AUR publication
# Usage: ./script/gen-pkgbuild.sh v0.1.0-cachy1

set -euo pipefail

TAG="${1#v}"  # strip leading 'v' if present
VERSION="${TAG%%-cachy*}"
BUILD="${TAG##*-cachy}"

if [[ "${TAG}" != *-cachy* ]] || [[ -z "${VERSION}" || -z "${BUILD}" ]]; then
  echo "Usage: $0 <version-tag>  (e.g., v0.1.0-cachy1)"
  exit 1
fi

# Compute sha256 of source tarball
SOURCE_URL="https://github.com/Dark-Ohm/chronos-fm/archive/refs/tags/v${TAG}.tar.gz"
SHA256=$(curl -sL "${SOURCE_URL}" | sha256sum | cut -d' ' -f1)

# Extract GPUI commit from Cargo.lock (from git source)
GPUI_COMMIT=$(grep -A5 'name = "gpui"' Cargo.lock | grep 'revision' | head -1 | cut -d'"' -f2 || true)
if [[ -z "${GPUI_COMMIT}" ]]; then
  echo "⚠️  Could not find GPUI commit in Cargo.lock, using fallback"
  GPUI_COMMIT="a1b2c3d4e5f678901234567890abcdef12345678"
fi

# Render template
mkdir -p pkgbuild
cat > pkgbuild/PKGBUILD <<EOFPKG
# Maintainer: chronos-fm contributors <chronos-fm@chronos-fm.app>
pkgname=chronos-fm-cachy
pkgver=${VERSION}
pkgrel=${BUILD}
pkgdesc="Launcher × Explorer — CachyOS build (GPUI/wgpu, optimized)"
arch=(x86_64)
url="https://chronos-fm.app"
license=(MIT)
depends=(
  gcc-libs
  glibc
  openssl
  wayland
  wayland-protocols
  libxkbcommon
  libxcb
  libxkbcommon-x11
  vulkan-icd-loader
)
makedepends=(
  git
  cargo
  clang
  lld
  xorg-server-xvfb
)
optdepends=(
  'vulkan-radeon: AMD Vulkan driver'
  'vulkan-intel: Intel Vulkan driver'
  'nvidia-utils: NVIDIA Vulkan driver'
  'lib32-nvidia-utils: NVIDIA 32-bit Vulkan (for 32-bit apps)'
)
_GPUI_COMMIT="${GPUI_COMMIT}"
source=(
  "\${pkgname%-cachy}-\${pkgver}.tar.gz::https://github.com/Dark-Ohm/chronos-fm/archive/refs/tags/v\${pkgver}-cachy\${pkgrel}.tar.gz"
  "gpui::git+https://github.com/zed-industries/zed.git#commit=\${_GPUI_COMMIT}"
)
sha256sums=('${SHA256}' 'SKIP')

export RUSTFLAGS="-C opt-level=3 -C lto=thin -C codegen-units=1 \
  -C link-arg=-fuse-ld=lld -C panic=abort"
export CARGO_PROFILE_RELEASE_LTO=thin
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_PANIC=abort

prepare() {
  cd "\${pkgname%-cachy}-\${pkgver}"
  cat >> Cargo.toml <<EOFPATCH

[patch.crates-io]
gpui = { path = "../gpui/crates/gpui" }
zed_gpui = { path = "../gpui/crates/zed_gpui" }
EOFPATCH
  cargo fetch --locked --target x86_64-unknown-linux-gnu
}

build() {
  cd "\${pkgname%-cachy}-\${pkgver}"
  cargo build --release --locked -p chronos-fm
}

check() {
  cd "\${pkgname%-cachy}-\${pkgver}"
  xvfb-run -a --server-args="-screen 0 1024x768x24" \
    timeout 10 ./target/release/chronos-fm --version
  grep -q "\${_GPUI_COMMIT}" Cargo.lock && echo "✅ GPUI commit verified"
}

package() {
  cd "\${pkgname%-cachy}-\${pkgver}"
  install -Dm755 target/release/chronos-fm "\${pkgdir}/usr/bin/chronos-fm"
  install -Dm644 packaging/arch/chronos-fm.desktop "\${pkgdir}/usr/share/applications/chronos-fm.desktop"
  for size in 16 32 48 64 128 256 512; do
    install -Dm644 "assets/icons/chronos-fm-\${size}.png" \
      "\${pkgdir}/usr/share/icons/hicolor/\${size}x\${size}/apps/chronos-fm.png"
  done
  install -Dm644 assets/icons/chronos-fm.svg \
    "\${pkgdir}/usr/share/icons/hicolor/scalable/apps/chronos-fm.svg"
  install -Dm644 LICENSE "\${pkgdir}/usr/share/licenses/\${pkgname}/LICENSE"
}
EOFPKG

echo "✅ Generated pkgbuild/PKGBUILD for ${TAG}"
echo "   Version: ${VERSION}-${BUILD}"
echo "   GPUI commit: ${GPUI_COMMIT}"
echo "   SHA256: ${SHA256}"