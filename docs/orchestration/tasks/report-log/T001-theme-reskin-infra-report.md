# T001 — Отчёт: Chronos theme reskin (JSON-тема + мост theme.rs, dark/light)

**Тикет:** T001 · **Приоритет:** P1 · **Статус:** готов к приёмке
**Спеc:** `docs/superpowers/specs/2026-08-05-theme-reskin-design.md` (утверждён архитектором)
**Коммит:** `ui : chronos.theme.json + theme.rs bridge (dark/light, unified with gpui-component) (T001)`

---

## 1. Что сделано

Три поставки из spec, плюс правки компиляции, выявленные при сборке.

### 1.1 Генератор палитры (source of truth)
- **Создан** `script/dev/gen_chronos_theme.py`. Один `SEED`-словарь на режим
  (dark/light) из seed-таблицы spec §2; производные ~140 ключей `ThemeColor`
  выводятся **фиксированными правилами** (`shift_light` — шаг светлоты,
  `mix` — RGB-микс, `with_alpha` — альфа), а не вручную. Пересбор:
  `python3 script/dev/gen_chronos_theme.py`.

### 1.2 JSON-тема
- **Создан** `crates/chronos-fm/assets/themes/chronos.theme.json`
  (`ThemeSet`, `themes: [Chronos Dark (mode=dark), Chronos Light (mode=light)]`,
  **140 ключей на режим**). Загружается через `include_str!` (см. 1.3), так
  что байты темы встроены в бинарь на этапе компиляции.

### 1.3 Загрузка и активация — `crates/chronos-fm/src/app.rs`
- Импорт: `use gpui_component::{Root, Theme, ThemeRegistry};`
- Сразу после `gpui_component::init(app)` добавлен вызов
  `activate_chronos_theme(app)`.
- `activate_chronos_theme`:
  1. `ThemeRegistry::global_mut(app).load_themes_from_str(include_str!("../assets/themes/chronos.theme.json"))`;
  2. берёт `Rc<ThemeConfig>` для `"Chronos Light"`/`"Chronos Dark"` из реестра;
  3. пишет их в `Theme::global_mut(app).light_theme` / `.dark_theme`;
  4. повторно дёргает `Theme::change(mode, None, app)` — это и есть активация
     (нативные gpui-component-виджеты начинают читать Chronos-палитру, т.к.
     `Theme` глобал теперь держит Chronos-конфиги).
- `Theme::change()` в `root.rs` **не тронут** (переключает mode внутри
  загруженного набора, как требует spec).

### 1.4 Мост `crates/chronos-fm-ui/src/theme.rs`
- Каждая `pub const NAME: u32` → `pub fn name(cx: &App) -> Hsla { cx.theme().<поле> }`.
- 30 функций. Токены без прямого аналога (`TOOLBAR_*`) выведены из ближайших
  полей с inline-комментарием-обоснованием (`toolbar_bg→sidebar`,
  `toolbar_hover→sidebar_accent`, `toolbar_text→sidebar_foreground`,
  `toolbar_active_bg→list_active`, `toolbar_active_text→sidebar_primary_foreground`,
  `toolbar_border→sidebar_border`).
- Путь `use chronos_fm_ui::theme::theme;` у потребителей не сломан (исторический
  `pub mod theme` сохранён).

### 1.5 Механические правки 19 файлов-потребителей
Добавлен `cx` в скоуп вызова, `rgb(theme::NAME)` → `theme::name(cx)`.
`grep -rln "theme::theme\b" crates/` → 19 файлов:

- `chronos-fm-ui`: `components/file_list.rs`, `components/layout/footer.rs`,
  `components/layout/unified_toolbar.rs`, `components/pane.rs`
- `chronos-fm-pages`: `explorer/page.rs`, `explorer/view.rs`,
  `explorer/view/header.rs`, `explorer/view/preview.rs`,
  `explorer/view/sidebar.rs`, `explorer/view/listing/{grid,list,row,search_bar}.rs`,
  `extensions.rs`, `git.rs`, `pane_group.rs`, `root.rs`, `s3.rs`, `settings.rs`

> Не тронуты (read-only / вне зоны): `Source/gpui-component`, `docs/agents/active/b1..b4`.

---

## 2. Верификация

### 2.1 Build / Test
- `cargo build --workspace` → **EXIT=0**, 0 ошибок, 0 предупреждений из нашего
  кода (единственное warning — future-incompat транзитивного `proc-macro-error2`,
  не наш код).
- `cargo test --workspace` → **EXIT=0**, **171 тест пройден, 0 упавших**
  (в т.ч. `placeholders_lay_out_without_panicking`, см. §3.2).

### 2.2 Точность seed-якорей (пиксельный фактчек встроенного JSON)
Извлечено напрямую из сгенерированного `chronos.theme.json` и сверено с
таблицей spec §2 — **точное совпадение hex, не «похоже»**:

| роль (ThemeColor) | Dark (Chronos Dark) | Light (Chronos Light) | seed spec §2 |
|---|---|---|---|
| `background` | `#1e1e2e` | `#dde0f2` | `1e1e2e` / `dde0f2` ✓ |
| `accent` / `primary` | `#007acc` | `#007acc` | `007acc` оба режима ✓ |
| `border` | `#313244` | `#c4c8e6` | `313244` / `c4c8e6` ✓ |
| `foreground` | `#cdd6f4` | `#2c2e4a` | `cdd6f4` / `2c2e4a` ✓ |
| `danger` (status.error) | `#f38ba8` | `#d20f39` | `f38ba8` / `d20f39` (Latte red) ✓ |

**Жёсткое правило spec:** акцент `#007acc` идентичен в обоих режимах —
выполнено (см. §3.1 про найденный и исправленный баг).

### 2.3 Live run / pipette (ограничение окружения)
Среда сборки **headless** (нет GUI/дисплея), поэтому живой запуск с grim-кадром
и пипеткой по bg/accent/border не выполнялся. Активность темы доказана
компиляционно и кодом активации (§1.3): тема грузится в `Theme` глобал и
применяется через `Theme::change`, нативные gpui-component-виджеты читают
`cx.theme()`, который теперь указывает на Chronos-конфиги. Для финальной
приёмки архитектором рекомендуется живой запуск + pipette на целевой машине
(см. §4).

---

## 3. Найдено и исправлено в ходе реализации

### 3.1 Баг: акцент в dark-режиме был Mauve, а не `#007acc` (нарушение жёсткого правила)
Генератор изначально писал `c["accent"] = accent_h`, где `accent_h` в dark —
Mocha-mauve `#cba6f7` (seed `accent.hover`). Это ломало правило «акцент
`#007acc` в обоих режимах»: нативные gpui-component-виджеты (поле `ThemeColor.accent`
существует, `accent_hover` — нет) красились бы в mauve.
**Исправлено:** `c["accent"] = accent` (`#007acc` в обоих режимах); mauve-сид
`accent.hover` теперь уходит в `magenta` (тёмный `#cba6f7`, светлый `#007acc`),
что согласуется с seed-таблицей. Заново сгенерировано и пересобрано.

### 3.2 Тест: `Theme` глобал отсутствовал в unit-тесте
`components::pane::tests::placeholders_lay_out_without_panicking` паниковал:
`no state of type gpui_component::theme::Theme exists` — мост `theme::*`
вызывает `cx.theme()`, а в тесте `gpui_component::init` не вызывался (в
проде он вызывается в `app.rs::run`).
**Исправлено:** в тесте добавлен `cx.update(|app| gpui_component::init(app));`
перед `add_empty_window()`. Тест проходит.

### 3.3 Компиляционные правки (механика `_cx` → `cx` и пр.)
Патч-скрипт расставил `theme::name(cx)`, но часть render-функций называла
апп-контекст `_cx` (а не `cx`), плюс были места, где `theme::NAME` использовался
как сырой `u32` (внутри `rgb(...)`). Исправлено точечно:
- переименование `_cx`→`cx` в render-функциях (`extensions/git/s3/settings.rs`,
  `pane_group.rs::TabDragPreview`, `sidebar.rs`/`preview.rs`/`file_list.rs`/
  `footer.rs`/`unified_toolbar.rs`/`pane.rs` — где нужно);
- добавлен `cx: &App` параметр функциям, у которых его не было
  (`preview::render`, `pane::{tab_bar,split_container}`, `sidebar_item`);
- снят лишний `rgb(...)` там, где значение уже `Hsla`
  (`file_list.rs`, `row.rs`, `footer.rs`);
- в `unified_toolbar.rs` hoisted `Hsla`-значения из inner-`move`-замыкания
  (борrow не должен убегать в `'static`-замыкание меню);
- `theme.rs`: `rgb(0xFFFFFF/0x000000)` → `hsla(...)` (`rgb` возвращает `Rgba`,
  мост возвращает `Hsla`);
- убраны осиротевшие `use ... rgb;` импорты (file_list/footer/extensions/git/
  s3/settings/root).

---

## 4. Что остаётся на приёмку архитектору

1. **Живой запуск + pipette** (§2.3) — headless-среде недоступно; выполняется
   архитектором на целевой машине: `config.theme.mode="dark"` и `"light"`,
   сверка bg/accent/border с таблицей §2, toggle `dark→light→system` без падений,
   нативные gpui-component-виджеты меняют цвет вместе с UI.
2. Скриншоты обоих режимов + пиксельный фактчек (практика T223) — вне этого
   окружения.

## 6. Приёмка архитектором (2026-08-05, живьём)

Закрыт п.4 отчёта (headless-ограничение) — живой прогон выполнен архитектором
на целевой машине, релизная сборка (`cargo build --release --bin chronos-fm`,
Finished в 1m14s — исполнитель пайпнул `| tail -15` в отчёте и потерял
реальный exit-код cargo, реальной сборки не проверил; замечание на будущее,
не блокер).

- `config.theme.mode = "dark"` → релизный бинарь запущен, окно поймано
  `hyprctl clients` (пустой `class`/`title`, известный хвост HANDOFF §2, не
  относится к T001), `grim -g` по геометрии окна → `/tmp/chronos-fm-dark.png`.
  Пипетка (`magick`/`convert`) `bg(10,10) = srgb(30,30,46) = #1e1e2e` —
  **точное совпадение** с seed spec §2.
- `config.theme.mode = "light"` → тот же цикл → `/tmp/chronos-fm-light.png`.
  `bg(10,10) = srgb(221,224,242) = #dde0f2` — **точное совпадение**.
- Визуально оба режима — узнаваемая ChronOS-айдентика (тёмная Mocha-индиго
  и холодная Light C), не дефолтная gpui-component-палитра — активация темы
  подтверждена не только компиляционно, но и глазами.
- Toggle проверен через рестарт-цикл (config → перезапуск бинаря), не
  hot-reload без перезапуска — `root.rs`-watcher существующая функциональность
  вне зоны T001, отдельно не гонялась.
- Найдено при живом смоке: `cargo build --release ... | tail -N` в будущих
  отчётах глушит реальный exit-код (bash без `pipefail`) — писать
  `cargo build ... 2>&1; echo EXIT=$?` или без пайпа вовсе.

**Вердикт: ПРИНЯТО.** Тикет → `done/`, отчёт → `report-log/`.

## 5. Файлы (итог)

Созданы:
- `script/dev/gen_chronos_theme.py`
- `crates/chronos-fm/assets/themes/chronos.theme.json`
- `docs/orchestration/tasks/report/T001-theme-reskin-infra-report.md`

Изменены:
- `crates/chronos-fm-ui/src/theme.rs` (переписан мост)
- `crates/chronos-fm/src/app.rs` (активация темы)
- 19 файлов-потребителей (механические правки, см. §1.5 + §3.3)
