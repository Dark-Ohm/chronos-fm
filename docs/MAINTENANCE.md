# chronos-fm CachyOS Maintenance Guide

## GPUI Commit Update Procedure

**Frequency**: Weekly check, update when meaningful Wayland/HiDPI fixes land.

1. Identify target commit on `zed-industries/zed#main`
2. Local test:
   ```bash
   cargo update -p gpui --precise <COMMIT>
   cargo update -p zed_gpui --precise <COMMIT>
   cargo build --release --locked -p chronos-fm
   ./script/verify-deps.sh
   # Manual Hyprland test (clipboard, focus, HiDPI)
   ```
3. Update `Cargo.lock` in repo:
   ```bash
   git add Cargo.lock
   git commit -m "chore(deps): update GPUI to <COMMIT> (Wayland fixes)"
   ```
4. Tag new CachyOS build:
   ```bash
   git tag -a v<UPSTREAM>-cachy<N+1> -m "GPUI update: <summary>"
   git push origin v<UPSTREAM>-cachy<N+1>
   ```
5. CI builds, tests, publishes to AUR.

## CachyOS ISO Release Rebuild

When CachyOS releases a new ISO (new toolchain: GCC, glibc, etc.):

1. Rebuild locally in clean container:
   ```bash
   docker run --rm -it -v $(pwd):/src archlinux/archlinux:base-devel
   # Inside: pacman -Syu && cd /src && cargo build --release -p chronos-fm
   ```
2. If build succeeds and tests pass, tag `v<SAME>-cachy<N+1>` and push.
3. No code changes needed unless toolchain breaks something.

## Monthly Dependency Audit

```bash
cargo audit
cargo deny check
# Review dependabot PRs
cargo update --dry-run  # check for updates
```

## User Issue Triage (label: cachyos)

| Symptom | First Check |
|---------|-------------|
| Blank window / crash | `vulkaninfo --summary` — missing ICD? |
| Clipboard broken | GPUI commit ≥ pinned? Try `WINIT_UNIX_BACKEND=wayland chronos-fm` |
| HiDPI wrong | `WINIT_SCALE_FACTOR=2 chronos-fm` |
| NVIDIA flicker | Kernel param `nvidia-drm.modeset=1` |
| Font issues | Install `noto-fonts`, `ttf-jetbrains-mono` |

## Rollback / Hotfix

If AUR build has critical regression:
1. `git revert <bad-commit>` or fix forward
2. Tag `v<SAME>-cachy<N+1>`
3. Push → CI publishes new AUR build
4. Add note to GitHub Release: "Fixes: #issue"

## AUR SSH Key Setup

1. Generate dedicated SSH key:
   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/chronos-fm-aur -C "chronos-fm-aur-bot"
   ```
2. Add public key to AUR account: https://aur.archlinux.org/account/sshkeys
3. Add private key as GitHub secret `AUR_SSH_KEY` in repository settings