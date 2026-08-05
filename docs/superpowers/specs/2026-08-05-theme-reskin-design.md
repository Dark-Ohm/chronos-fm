# Chronos-FM Reskin — Theme Infrastructure v1

**Дата:** 2026-08-05. **Автор:** Архитектор (сессия перенесена из ChronOS,
контекст оркестрации ChronOS оставлен намеренно как справка, не как
задача этого репо).

## Контекст и цель

Chronos-FM сейчас красится через `crates/chronos-fm-ui/src/theme.rs` —
20 плоских `pub const` (u32 hex), одна светлая палитра, без runtime
переключения. Параллельно приложение уже тянет `gpui-component`
(Longbridge toolkit) как UI-библиотеку, и та несёт СВОЙ полноценный
theme-движок — `gpui_component::Theme` (global, `Deref`/`DerefMut` на
`ThemeColor`), `ThemeMode` (System/Light/Dark), `ThemeRegistry` с
JSON-based темами (`ThemeSet` → `serde_json::from_str`, формат как
`Source/gpui-component/themes/aurora.json`). `Theme::change()` УЖЕ
дёргается в `crates/chronos-fm-pages/src/root.rs::apply_config()` при
смене `config.theme.mode` — то есть toggle-инфраструктура наполовину
подключена, просто красит дефолтную gpui-component-палитру, а не
Chronos-цвета, и вообще не касается собственных 20 файлов chronos-fm-ui.

Цель этого spec — свести оба мира красок к одному источнику правды на
палитре ChronOS (Mocha-тёмная + "Light C" светлая, 1:1 из
`../ChronOS/crates/ui/src/theme/schemes.rs`), с реальным runtime
light/dark/system toggle. Это подготовительный шаг к «перекрасить в
Хронос»; перенос структурных UI-паттернов (elevated_card/section_header/
empty-state) и функциональная доработка до замены Thunar (b1–b4,
context-menu/clipboard/file-ops) — отдельные, не в этом spec.

## Архитектура

### 1. Палитра как JSON-тема gpui-component

Новый файл `crates/chronos-fm/assets/themes/chronos.theme.json` (или
`assets/` на уровне корня — сверить с существующей раскладкой assets/
при реализации), схема — как `Source/gpui-component/.theme-schema.json`
/ `themes/aurora.json`: один файл, `"themes": [...]` с двумя записями
`"Chronos Dark"` (`mode: "dark"`) и `"Chronos Light"` (`mode: "light"`).

Загружается в `crates/chronos-fm/src/app.rs` при старте, рядом с
существующим `Root::new(...)` — `ThemeRegistry::global_mut(cx)
.load_themes_from_str(include_str!(...))`, затем регистрируется как
активный набор (сверить точный API `ThemeRegistry` при реализации —
`load_themes_from_str` подтверждён, точку "сделать активной" уточнить
по `crates/story/src/themes.rs::init` как рабочему примеру в дереве).
`Theme::change(mode, window, cx)` в `root.rs` остаётся без изменений —
он переключает MODE внутри уже загруженного набора, не сам набор.

### 2. Seed-палитра и вывод ~100 ключей

`ThemeColor`/JSON-схема темы несёт ~100 dot-path ключей
(`button.primary.background`, `accordion.hover.background`,
`danger.foreground`, …). Не расписывать их с потолка вручную —
зафиксировать Base16-подобный seed (10-16 значений, тот же принцип,
что `DEFAULT_BASE16` в ChronOS) и вывести производные по фиксированным
правилам (hover = seed ±шаг светлоты, active = ещё шаг, foreground = 
контрастный текст на заливке). Seed на реализацию:

**Dark ("Chronos Dark", = ChronOS `DEFAULT_BASE16`, Catppuccin Mocha):**
| роль | hex |
|---|---|
| bg.primary | `1e1e2e` |
| bg.secondary | `25253b` |
| bg.tertiary | `181825` |
| bg.elevated | `313244` |
| text.disabled/interactive.active | `45475a` |
| text.muted | `6c7086` |
| text.secondary | `a6adc8` |
| text.primary | `cdd6f4` |
| status.error | `f38ba8` |
| status.warning | `f9e2af` |
| status.warning(alt)/info-teal | `94e2d5` |
| status.success | `a6e3a1` |
| status.info | `89b4fa` |
| accent.primary / border.focused | `007acc` |
| accent.hover | `cba6f7` |
| active/emph | `f38ba8` |

**Light ("Chronos Light" = ChronOS "Light C", НЕ инверсия Latte):**
| роль | hex |
|---|---|
| bg.primary (pageBg) | `dde0f2` |
| bg.secondary (cardBg) | `e6e9fa` |
| bg.tertiary (cardBase) | `eceefa` |
| bg.elevated (hoverBg) | `e0e3f4` |
| text.primary | `2c2e4a` |
| text.secondary | `5a5d80` |
| text.muted | `7d80a6` |
| text.disabled/placeholder | `9a9dc0` |
| border.default (cardBorder) | `c4c8e6` |
| border.subtle | `d4d7ee` |
| border.focused/accent | `007acc` (НЕ переопределяется — правило) |
| status.error | `d20f39` (Catppuccin Latte red — Mocha-пастель на светлом фоне нечитаема, см. ChronOS 2026-07-20 живой смок) |
| status.warning | `df8e1d` |
| status.success | `40a02b` |
| status.info | `1e66f5` |

**Правило:** акцент (`007acc`) идентичен в обоих режимах — светлая тема
НЕ красит акцент (перенесено из ChronOS `docs/design.md`/DECISIONS.log
дословно, тот же архитектор, то же обоснование).

Таблицу вывода производных ключей (button.*/danger.*/accordion.* и
т.д. из seed) зафиксировать в implementation plan как явный алгоритм
(не "на глаз"), чтобы dark/light были предсказуемо согласованы.

### 3. Мост `chronos-fm-ui/theme.rs`

20 файлов-потребителей (`chronos-fm-ui` + `chronos-fm-pages`) НЕ
переписываются на прямые вызовы `cx.theme()` в этом заходе — слишком
большой блайн-рефактор за один присест, риск непропорционален пользе
v1. Вместо этого `theme::theme` остаётся публичным модулем, но каждая
`pub const NAME: u32` становится `pub fn name(cx: &App) -> Hsla { cx
.theme().<соответствующее поле> }`. Правки в 20 файлах — механические:
добавить `cx` в сигнатуру/скоуп вызова, убрать `rgb(theme::NAME)` →
`theme::name(cx)`. Токены без прямого аналога в `ThemeColor`
(`TOOLBAR_*`) — выводятся из ближайших полей (`background`/`accent`)
с комментарием, откуда взято.

### 4. Явно вне скоупа

- Структурные UI-паттерны из ChronOS T231/T252 (`elevated_card`,
  `section_header`, empty-state) — следующий spec, после того как
  палитра ляжет (нужно на чём проверять контраст живьём).
- Функциональный трек b1–b4 (`docs/agents/active/`, clipboard/rename/
  file-ops/context-menu) — не трогается, независимая работа, уже
  спланирована отдельно (`docs/superpowers/plans/2026-07-21-explorer
  -context-menu.md`).
- `gpui-component` (`Source/gpui-component`) — 0 правок, read-only
  зависимость (та же дисциплина, что применялась к `gpui`/`gpui_platform`
  при миграции тулкита, GROK #1).

## Верификация

- `cargo build --workspace` + `cargo test --workspace` чисто.
- Живой запуск, `config.theme.mode = "dark"` → grim-кадр, пипетка по
  bg/accent/border — совпадает с seed-таблицей выше (не "похоже",
  точное значение).
- То же для `"light"`.
- Toggle `dark → light → system` без падения/зависания, нативные
  gpui-component виджеты (кнопки/инпуты, если уже где-то
  задействованы) меняют цвет вместе с остальным UI — не остаются на
  дефолтной gpui-component палитре.

## Коммит

`ui : chronos.theme.json + theme.rs bridge (dark/light, unified with
gpui-component)` — после реализации по плану, не в этом spec.
