# Agent S4 — xim-rs (zed-xim)

**Тип:** реализация, шаг 4 (ввод)  
**Старт:** после `T032-notes/I3-xim.md` + merge S1  
**Worktree:** рекомендуется

## Цель

Снять git `zed-industries/xim-rs@16f35a2` из `gpui_linux`:

- path-vendor workspace (`zed-xim` + `xim-ctext` + `xim-parser` как нужно)
- или crates.io, если I3 подтвердил published pin

Файл: `Source/gpui_linux/Cargo.toml` (~xim dep) + root workspace если path.

## Стены

1. 4 consumer baselines.
2. Ввод/IME: компилятор **не** ловит регресс. Минимум:
   - grim окна с X11 feature path **если** стенд X11 доступен;
   - на pure Wayland — в note явно: «xim не exercised; only compile + tree».
3. NOTICE + PATCHES.md при vendor.

## Не трогать

- wgpu, font-kit (если S3 параллельно — разные worktree, merge осторожно)
- gpui_macos

## Выход

- коммит в Source
- `T032-notes/S4-xim.md` — smoke limitations + tree proof

## Приёмка

- [ ] `cargo tree -i zed-xim` → path или crates.io, не zed git
- [ ] 4 baselines
- [ ] live/smoke documented (or honest Wayland gap)
