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

# Pin the fork at its current local HEAD. Dev truth lives in ../Source (path
# deps in Cargo.toml); the PKGBUILD fetches the fork from git so users who
# download the software link to the published repo at this exact commit.
# Locate the Chronos-GPUI checkout: $CHRONOS_FORK_DIR, or the sibling
# checkout (../Source — local dev), or Source/ inside the repo (CI).
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FORK_DIR="${CHRONOS_FORK_DIR:-}"
if [[ -z "${FORK_DIR}" || ! -d "${FORK_DIR}/.git" ]]; then
  if [[ -d "${REPO_ROOT}/../Source/.git" ]]; then
    FORK_DIR="${REPO_ROOT}/../Source"
  elif [[ -d "${REPO_ROOT}/Source/.git" ]]; then
    FORK_DIR="${REPO_ROOT}/Source"
  else
    FORK_DIR=""
  fi
fi
if [[ -z "${FORK_DIR}" ]]; then
  echo "⚠️  Could not locate the Chronos-GPUI checkout (set CHRONOS_FORK_DIR)"
  GPUI_COMMIT="a1b2c3d4e5f678901234567890abcdef12345678"
else
  GPUI_COMMIT=$(git -C "${FORK_DIR}" rev-parse HEAD)
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
  "gpui::git+https://github.com/Dark-Ohm/Chronos-GPUI.git#commit=\${_GPUI_COMMIT}"
)
sha256sums=('${SHA256}' 'SKIP')

export RUSTFLAGS="-C opt-level=3 -C lto=thin -C codegen-units=1 \
  -C link-arg=-fuse-ld=lld -C panic=abort"
export CARGO_PROFILE_RELEASE_LTO=thin
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_PANIC=abort

prepare() {
  cd "\${pkgname%-cachy}-\${pkgver}"
  # Cargo.toml carries dev-only path deps to ../Source. Rewrite them to the
  # fork fetched into \$srcdir/gpui and regenerate the lock from that graph.
  sed -i 's|path = "../Source/|path = "../gpui/|g' Cargo.toml
  rm -f Cargo.lock
  cargo fetch
}

build() {
  cd "\${pkgname%-cachy}-\${pkgver}"
  cargo build --release -p chronos-fm
}

check() {
  cd "\${pkgname%-cachy}-\${pkgver}"
  test -d ../gpui/gpui && echo "✅ GPUI fork fetched at \${_GPUI_COMMIT}"
  xvfb-run -a --server-args="-screen 0 1024x768x24" \
    timeout 10 ./target/release/chronos-fm --version
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