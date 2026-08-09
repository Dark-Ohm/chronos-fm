# S3 — font-kit: crates.io (option A)

**Решение:** **A — crates.io** `zed-font-kit = "0.14.1-zed"` (рекомендация I2).
**Статус:** DONE — коммит `5535764` в Source, tree clean.
**Основание:** `T032-notes/I2-font-kit.md` — published API идентичен пину 94b0f28,
разница только `dirs ^5.0` vs `6.0` (не-API); zed сам публикует форк (mikayla-maki).

## Что сделано

- `Source/Cargo.toml` [workspace.dependencies]: `font-kit = { version = "0.14.1-zed", package = "zed-font-kit" }`.
- `Source/gpui/Cargo.toml` (macOS block): git-пин → `font-kit = { workspace = true, optional = true }`.
- `Source/gpui_wgpu/Cargo.toml` (unconditional dep, путь font-kit в Linux graph): то же.
- Cargo.lock: `zed-font-kit` git → registry; добавлены `dirs 5.0.1`/`dirs-sys 0.4.1`
  (`dirs 6.0.0` остаётся для других потребителей). Оба lock (Source + gpui-component).
- git `zed-font-kit` остаётся в resolve-graph только через cfg-gated `gpui_macos`
  (на Linux не собирается — тот же паттерн, что и ambiguity у `gpui`).

## Grim before/after (smoke: `gpui/examples/text.rs`, тот же бинарь, тот же сценарий)

App: gpui text example («GPUI Typography» window) — гоняет `cosmic-text` +
`font_kit::matching::find_best_match` (путь шрифтов, что и у ChronOS bar).
Живой шелл ChronOS не трогался (paralell-session правило, HANDOFF).

| | before (git 94b0f28) | after (crates.io 0.14.1-zed) |
|---|---|---|
| commit Source | `3a0fe17` | `5535764` |
| кадры | `/tmp/t032-s3-before/screen.png` + `window.png` | `/tmp/t032-s3-after/screen2.png` + `window.png` |
| window size | 945x585 | 1265x692 (Hyprland затайлил иначе) |
| colors | 8170 | 9155 |
| stddev | 7525.58 | 9611.83 |
| run.log | 0 bytes (нет паник) | 0 bytes (нет паник) |

Оба окна рендерят реальные системные глифы (8–9k уникальных цветов, высокий
контраст — не tofu/не пусто). Font discovery работает (dirs 5.0 не сломал
подгрузку шрифтов — caveat I2 закрыт).

Сравнение: normalized compare (after → 945x585, fuzz 8%) = **24 323 px из 552 825
(4.4%)** — разница от разной ширины окна (wrap) + resampling, не от шрифтов.
Pixel-exact при той же геометрии сделать не удалось: `hyprctl dispatch movewindowpixel` падает на Lua-парсере Hyprland 0.56.1 (известный HANDOFF-кейс), и `hyprctl eval` тоже (dispatch не экспортирован в eval-контекст).

## Стены (baselines)

| команда | exit |
|---|---|
| `cargo check --workspace` (ChronOS) | 0 |
| `cargo check --workspace` (Chronos-lm) | 0 |
| `cargo test --workspace` (Chronos-FM) | 0 |
| `cargo build -p gpui-component-story` | 0 |
| `cargo build -p gpui --example text` (crates.io font-kit) | 0 |

## Осталось на git

- `zed-font-kit` git через cfg-gated `gpui_macos`/`gpui_windows` (не тянутся на Linux).
- wgpu pin, xim, scap (DEFER S2), reqwest (S1 path), http_client (T030).
