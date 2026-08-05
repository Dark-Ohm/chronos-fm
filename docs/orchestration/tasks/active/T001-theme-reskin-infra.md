# T001 — Chronos theme reskin: JSON-тема + мост theme.rs (dark/light)

**Приоритет:** P1 — первый шаг реюзника «перекрасить FM в Chronos»,
блокирует T002 (структурные паттерны, ещё не заведён — см. Очередь).
**Роль:** реализация по готовому spec, самодостаточное задание.
**Источник:** `docs/superpowers/specs/2026-08-05-theme-reskin-design.md`
(design одобрен архитектором, не пересматривать без нового запроса
пользователя — только уточнять implementation-детали, явно помеченные
в spec как «сверить при реализации»).

**Нумерация:** это первый T-тикет в Chronos-FM — до сих пор здесь была
своя b-схема (`docs/agents/active/b1..b4`, независимый функциональный
трек clipboard/rename/file-ops/context-menu, НЕ трогать, не пересекается
файлами). T-ID здесь локальный для этого репо, начат с T001, не путать
с T-нумерацией в соседнем ChronOS (разные счётчики, разные репо).

## Контекст

Сейчас палитра Chronos-FM — `crates/chronos-fm-ui/src/theme.rs`, 20
плоских `pub const` (u32 hex), одна светлая тема, без runtime-toggle.
Параллельно UI уже тянет `gpui-component` (Longbridge), у которого
СВОЙ полноценный theme-движок: `gpui_component::Theme` (global),
`ThemeMode` (System/Light/Dark), `ThemeRegistry` с JSON-темами
(формат — `Source/gpui-component/themes/aurora.json`,
`.theme-schema.json`). `Theme::change()` УЖЕ вызывается в
`crates/chronos-fm-pages/src/root.rs::apply_config()` при смене
`config.theme.mode` — toggle наполовину подключён, просто красит
дефолтную gpui-component-палитру, не Chronos-цвета, и не касается
20 файлов chronos-fm-ui вообще.

Полный разбор архитектуры, seed-палитра (Base16-сид dark/light 1:1 из
`../ChronOS/crates/ui/src/theme/schemes.rs`), таблица цветов и явные
границы скоупа — в spec-файле выше. Этот тикет — реализация, не
повторяю таблицы здесь, читать spec целиком перед началом.

## Что нужно

1. **JSON-тема.** Новый файл (путь на реализацию — сверить с текущей
   раскладкой `assets/` в `crates/chronos-fm/`, если её ещё нет —
   завести) `themes/chronos.theme.json`, схема как
   `Source/gpui-component/themes/aurora.json` /
   `.theme-schema.json`. Два варианта: `"Chronos Dark"` (`mode:
   "dark"`), `"Chronos Light"` (`mode: "light"`). Значения — из
   seed-таблицы spec §2, ~100 производных ключей вывести по фикс-
   правилам светлота±шаг (не с потолка, алгоритм зафиксировать в коде/
   комментарии генератора, если пишется программно, либо явно
   расписать в комментарии JSON, если руками).
2. **Загрузка и активация.** `crates/chronos-fm/src/app.rs`, рядом с
   существующим `Root::new(...)`: `ThemeRegistry::global_mut(cx)
   .load_themes_from_str(include_str!(...))` + сделать "Chronos"
   активным набором тем. Рабочий пример механики в дереве (read-only,
   только для справки, не Chronos-FM код) —
   `../Source/gpui-component/crates/story/src/themes.rs::init`.
   `Theme::change()` в `root.rs` НЕ трогать — он переключает mode
   внутри уже загруженного набора.
3. **Мост `theme.rs`.** Каждая `pub const NAME: u32` →
   `pub fn name(cx: &App) -> Hsla { cx.theme().<поле> }`. Токены без
   прямого аналога в `ThemeColor` (`TOOLBAR_*`) — вывести из ближайших
   полей с комментарием-обоснованием. В 20 файлах-потребителях (список
   — `grep -rln "theme::theme\b" crates/` от корня репо) — механическая
   правка вызовов: добавить `cx` в скоуп, `rgb(theme::NAME)` →
   `theme::name(cx)`.

## Зона файлов

- `crates/chronos-fm/src/app.rs` — загрузка/активация темы.
- `crates/chronos-fm-ui/src/theme.rs` — мост.
- 20 файлов-потребителей (`chronos-fm-ui`, `chronos-fm-pages`) —
  только механическая правка вызовов theme::*, без побочных правок.
- Новый `themes/chronos.theme.json` (путь уточнить при реализации).
- **НЕ трогать:** `Source/gpui-component` (read-only зависимость, та
  же дисциплина что при миграции тулкита GROK #1), `docs/agents/active/
  b1..b4` (параллельный функциональный трек, независимая зона файлов —
  `row.rs`/`list.rs`/`grid.rs`/`listing.rs` и clipboard/file-ops модули
  тебя не касаются).

## Верификация

- `cargo build --workspace` + `cargo test --workspace` чисто.
- Живой запуск, `config.theme.mode = "dark"` → grim-кадр, пипетка по
  bg/accent/border — совпадает с seed-таблицей spec §2 (точное hex, не
  «похоже»).
- То же для `"light"`.
- Toggle `dark → light → system` без падения/зависания; нативные
  gpui-component виджеты (если уже где-то задействованы в UI) меняют
  цвет вместе с остальным — не остаются на дефолтной палитре
  gpui-component (проверка, что тема реально активирована, не просто
  загружена в реестр).
- Отчёт — скриншоты обоих режимов + пиксельный фактчек таблицей (как
  практика T223 в ChronOS), не просто «работает».

## Отчёт

`docs/orchestration/tasks/report/T001-theme-reskin-infra-report.md`
(inbox). Приёмка — архитектор лично: грепы/дифф/build/test + живой
grim/пипетка, отчёту на слово не верить. Принят → `report-log/`,
тикет → `done/`. Отклонён → `rejected/`.

## Коммит

`ui : chronos.theme.json + theme.rs bridge (dark/light, unified with
gpui-component) (T001)`

## Очередь после этого тикета

- T002 (не заведён) — структурные паттерны из ChronOS T231/T252
  (elevated_card/section_header/empty-state), заводить после приёмки
  T001 — нужна живая палитра, чтобы проверять контраст.
- b1–b4 — независимо, функциональный трек к замене Thunar, продолжать
  параллельно без пересечения зон файлов.
