# S4 — xim-rs (zed-xim): crates.io swap + proof

**Тип:** реализация, шаг 4 (ввод). **Старт:** после T032-notes/I3-xim.md (+ merge S1/S2/S3).
**Дата:** 2026-08-09 · коммит в Source: (см. git log)

## Что сделано

Снят git `zed-industries/xim-rs@16f35a2` из `gpui_linux`, заменён на **crates.io `zed-xim = "0.4.0-zed"`** (рекомендация I3: option A — published pin, без path-vendor и без NOTICE/PATCHES, т.к. это crates.io, не вендор).

Изменения:
- `Source/Cargo.toml` `[workspace.dependencies]` — добавлено: `xim = { version = "0.4.0-zed", package = "zed-xim" }` (комментарий T032/S4).
- `Source/gpui_linux/Cargo.toml` — замена inline git-пина на `xim = { workspace = true, features = ["x11rb-xcb","x11rb-client"], optional = true }` (вызов подключается feature `x11`, который в `gpui default`).
- `Source/Cargo.lock` и `Source/gpui-component/Cargo.lock` — переразрешены: `zed-xim`/`xim-parser`/`xim-ctext` теперь из **registry** (а не git). Subcrates НЕ вендорены, приходят транзитивно из crates.io.

## Proof: `cargo tree -i zed-xim`

```
$ cargo tree --offline -i zed-xim -e normal,build,dev   # из Source/
zed-xim v0.4.0-zed          # БЕЗ git-суффикса → crates.io
└── gpui_linux v0.1.0 (/home/neo/projects/chronos-ecosystem/Source/gpui_linux)
    └── gpui_platform v0.1.0 … (…→ gpui)
```

`Cargo.lock` (main + gpui-component) → `zed-xim v0.4.0-zed`:
```
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0c0b46ed118eba34d9ba53d94ddc0b665e0e06a2cf874cfa2dd5dec278148642"
```
checksum совпадает с published-кратом из I3 (crates.io API). `grep zed-industries/xim-rs` в обоих lock → **0 совпадений**; `grep xim-rs.git` → 0 совпадений.

## Компиляционные baseline (что прогнано)

| Baseline | Команда | Результат |
|---|---|---|
| xim-consumer crate | `cargo check --offline -p gpui_linux --features x11` | ✅ Finished |
| полный path-gpui graph | `cargo check --offline -p gpui_platform --features x11,wayland,font-kit` | ✅ Finished |
| локальный container consumer | `cargo check --offline --manifest-path gpui-component/Cargo.toml` (background) | ✅ Finished (2.99s) |

(только pre-existing `nightly_coverage` cfg-warnings; новых ошибок нет)

## Ввод/IME — честные ограничения (smoke)

**`xim` не exercised: only compile + tree.**
- Компилятор **не** ловит IME-регресс (ввод строк — runtime-путь). Живой кампарс X11/IME на этом стенде **не доступен** (нет дисплейного X11 + fcitx5/ibus-сессии в данном прогоне).
- **Pure Wayland НЕ докажет xim**: `xim` активен только на X11-пути (`gpui_linux/x11` → `zed-xim client`). Wayland-прогон — только baseline «не крашит», он здесь не выполнялся (headless).
- **Parity-caveat из I3:** published `0.4.0-zed` формально может быть на 1 merge-коммит раньше пина `16f35a2` (пара IME-фиксов: fcitx4 empty-reply, ctext encodings, Feedback bitflag). API идентичен; точную равноценность IME-фиксов подтверждает **приёмка S4 живым X11+fcitx5/ibus вводом** (CJK/JP: PreeditDraw → commit), как только стенд X11 доступен или этот worktree идёт на машину с дисплеем.
- Внешние consumers (ChronOS, Chronos-FM — path `../Source/gpui`; Chronos-IDE — git-снапшот `ee80b72`, до этого изменения) здесь в отдельные репо **не пересобирались** — они затронуты только transitive-источником (без смены API), канонично проверить `cargo check` в каждом. Chronos-lm — **N/A** (нет gpui/xim).

## Приёмка (checklist)

- [x] `cargo tree -i zed-xim` → crates.io (не zed git) — quote выше
- [x] baselines компиляции: gpui_linux/x11, gpui_platform/x11+wayland+font-kit, gpui-component — все ✅
- [x] live/smoke: **честный Wayland/headless gap** зафиксирован выше («xim не exercised; only compile + tree»)
- [x] NOTICE/PATCHES не требовались (crates.io, не vendor)
- [x] zero side-effects: wgpu/font-kit/gpui_macos не тронуты

## Файлы (коммит)

`Cargo.toml`, `gpui_linux/Cargo.toml`, `Cargo.lock`, `gpui-component/Cargo.lock`