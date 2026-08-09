# I4 — wgpu inventory findings

**Agent:** I4 (read-only)  
**Date:** 2026-08-09  
**Rev:** `357a0c56e0070480ad9daea5d2eaa83150b79e88`

---

## 1. Checkout size / workspace crates

| Metric | Value |
|---|---|
| Disk size | **116 MB** |
| Top-level crates/dirs | **18** (`naga`, `wgpu`, `wgpu-core`, `wgpu-hal`, `wgpu-types`, `wgpu-info`, `wgpu-macros`, `wgpu-naga-bridge`, `player`, `cts_runner`, `deno_webgpu`, `xtask`, `lock-analyzer`, `naga-cli`, `naga-test`, `benches`, `docs`, `examples`) |
| Workspace members (Cargo.toml count) | ~18 |

## 2. zed-only delta vs upstream v29.0.3

**Exactly 1 commit ahead** of `gfx-rs/wgpu` tag `v29.0.3`:

```
a466bc382 Add XCB display handle support to EGL backend
```

- **Files changed:** 1 — `wgpu-hal/src/gles/egl.rs`
- **Lines:** +23 / -1
- **Nature:** adds XCB display handle support to the EGL backend (needed for Hyprland/X11 on some drivers)

**Risk assessment of the delta:** Tiny surface area. But it's a *behavioral* change in `wgpu-hal` (the HAL layer) — dropping it would regress X11/EGL display selection. Not a cosmetic or doc-only patch.

## 3. Dependency chain in ChronOS

```
wgpu (zed fork, git rev)
└── gpui_wgpu (local path crate, 6 .rs files + 2 .wgsl shaders)
    └── gpui_linux
```

- `Cargo.toml:88` pins `wgpu` via git rev
- `gpui_wgpu` is the sole consumer in the workspace — no other crate depends on wgpu directly
- Shaders: `shaders.wgsl`, `shaders_subpixel.wgsl` (compiled at build time, not vendored binaries)

### gpui_wgpu source map

| File | Role |
|---|---|
| `gpui_wgpu.rs` | module root, re-exports |
| `wgpu_renderer.rs` (2265L) | `WgpuRenderer` — the main renderer, pipelines, instance buffer, blur passes |
| `wgpu_context.rs` (487L) | `WgpuContext` — adapter/device selection, surface format negotiation |
| `wgpu_atlas.rs` | texture atlas management |
| `cosmic_text_system.rs` | text shaping integration |
| `shaders.wgsl`, `shaders_subpixel.wgsl` | GPU shaders |

## 4. Silent visual regression risks

**If switching to upstream `gfx-rs/wgpu` v29.0.3:**

1. **XCB/EGL display handle loss** — the single zed-only commit adds XCB support. On Hyprland with X11 backend or nvidia proprietary drivers using EGL, surface creation could fail or fall back to a different path. **High severity if it triggers, hard to catch in CI** (no real GPU in headless test).

2. **Shader compatibility** — gpui_wgpu ships custom WGSL shaders compiled via naga. Upstream naga changes between v29.0.x patch releases could reject or reinterpret shader semantics. The delta is tiny (v29.0.3 → v29.0.3+1commit), so naga is effectively identical. **Low risk for now.**

3. **Surface format selection** — `wgpu_context.rs` negotiates `TextureFormat` with the adapter. Any upstream change in format preference order could shift sRGB/gamma behavior. **Low risk** given the delta.

4. **Instance buffer / pipeline layout** — `WgpuRenderer` uses raw wgpu pipeline creation. ABI-stable within v29, but patch releases have fixed pipeline validation before. **Low risk.**

## 5. What's needed for future S5 (vendor/path)

To move from git-pin to path-dependency (full vendor):

- [ ] Fork `zed-industries/wgpu` → `chronos-ecosystem/wgpu` (preserve the XCB commit)
- [ ] Add `naga` toolchain to build pipeline (shader compilation at build time — already the case)
- [ ] Update `Cargo.toml:88` from `git = ..., rev = ...` to `path = "../wgpu"` (or workspace member)
- [ ] Verify `wgpu-macros` proc-macro builds from local source (no registry assumption)
- [ ] Set up CI with `VK_ICD_FILENAMES` or swiftshader for headless render smoke test
- [ ] Grim baselines: capture reference frames on known scenes before and after the switch, diff in CI
- [ ] Lockfile regeneration (`cargo generate-lockfile`) — no upstream network at build

## Cost estimate

| Item | Effort |
|---|---|
| Workspace build from local source | **Low** — 18 crates, mostly same as git build |
| Maintaining the fork | **Low** — 1 commit ahead, rebasing onto new v29 patches is trivial until upstream breaks |
| Risk mitigation (grim baselines + CI) | **Medium** — needs a reference GPU or swiftshader setup |
| **Overall** | **Low-Medium** — the delta is tiny; the cost is in CI/render-verification, not in the code |

## ⛔ Не имплементировать без отдельного запроса

Этот note — только разведка. Любой переход на path-dependency или vendorинг требует:
1. Отдельного явного запроса (шаг 5 T032)
2. Согласования с командой (влияние на build reproducibility)
3. grim baselines до и после

---

**Definition of done:** ✅ note delivered, zero git changes
