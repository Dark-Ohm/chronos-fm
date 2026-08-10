# T036 — Отчёт: story-web WASM (nightly + rustup)

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-09)
>
> Path A executed. Verified: rustup+nightly present, active default still
> stable; wasm artifacts 85MB/120KB; `Source/assets` symlink; font stubs;
> vite 200 on gallery + wasm. Browser frame (Vivaldi, not Chrome) shows
> titled page + dark canvas — honest residual of 0-byte fonts, not
> toolchain. No Chronos-FM product path depends on live web gallery.
> T036 closed; T028 part-2 residual cleared.


**Дата:** 2026-08-09
**Исполнитель:** Buffy (executor)
**Источник:** T028 PARTIAL-ACCEPT (Wall 2.2), путь A (rustup + nightly)
**Статус:** ✅ WASM собран, dev-сервер запущен. Браузерный кадр невозможен (Chrome не установлен).

## Решение: путь A — rustup + nightly for wasm

### Инфраструктура

| Компонент | До | После |
|-----------|----|-------|
| `rustup` | ❌ отсутствовал | ✅ `~/.rustup/`, `~/.cargo/bin/rustup` v1.29.0 |
| nightly toolchain | ❌ | ✅ `nightly-x86_64-unknown-linux-gnu` (rustc 1.99.0-nightly 2026-08-08) |
| `wasm32-unknown-unknown` | ✅ было (системный) | ✅ есть и в nightly |
| `wasm-bindgen` CLI | ✅ 0.2.121 | ✅ (без изменений) |
| `bun` | ✅ `~/.bun/bin/bun` | ✅ (без изменений) |

**Изоляция:** `PATH="$HOME/.cargo/bin:$PATH"` — системный Rust `/usr/bin/rustc` 1.97.1 **не тронут**. Для wasm-сборки используется `RUSTUP_TOOLCHAIN=nightly`. Пять проектов на stable не затронуты.

### Попутные находки

1. **`Source/assets/` → symlink.** `gpui_web/src/platform.rs` использует `include_bytes!("../../../assets/fonts/...")` — путь разрешается в `Source/assets/`. Директории не было; создан symlink `Source/assets → ../assets`.

2. **Font stubs.** 8 из 8 шрифтов (`Lilex-*`, `IBMPlexSans-*`) — 0-байтовые файлы-заглушки. Для `include_bytes!` это допустимо (файл существует). Недостающие варианты (Italic, Bold, SemiBold) добавлены symlink'ами к Regular.

### Сборка

```
$ RUSTUP_TOOLCHAIN=nightly bash scripts/build-wasm.sh
   Compiling gpui_web v0.1.0
   Compiling gpui-component-story-web v0.5.1
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.81s

Step 2: Generating JavaScript bindings...
✓ Build completed successfully!
```

**Вывод:**
- `www/src/wasm/gpui_component_story_web_bg.wasm` — 85 MB
- `www/src/wasm/gpui_component_story_web.js` — 120 KB

### Dev-сервер

```
$ cd www && bun install && bun run dev
35 packages installed [1064ms]
VITE v8.0.0  ready in 223 ms
➜  Local:   http://localhost:3000/gpui-component/gallery/
```

### Браузерный кадр

Chrome не установлен в системе — `browser-use` агент недоступен. Для верификации визуально нужно открыть `http://localhost:3000/gpui-component/gallery/` в браузере вручную.

### Что изменилось в системе

| Изменение | Тип |
|-----------|-----|
| `~/.rustup/` + `~/.cargo/bin/rustup` | `curl | sh` (без sudo) |
| `~/.rustup/toolchains/nightly-*` | `rustup toolchain install` |
| `~/.rustup/toolchains/nightly-*/lib/rustlib/wasm32-*` | `rustup target add` |
| `Source/assets` → `../assets` | symlink |
| `assets/fonts/lilex/Lilex-{Italic,Bold,BoldItalic}.ttf` → `Regular.ttf` | symlink |
| `assets/fonts/ibm-plex-sans/IBMPlexSans-{Italic,SemiBold,SemiBoldItalic}.ttf` → `Regular.ttf` | symlink |

Ни одной правки в `.rs`-файлах. `git -C Source status` покажет только symlink `Source/assets`.
