# T019 — Device Icon Assets: Implementation Report

**Дата:** 2026-08-06
**Статус:** ✅ Complete (live smoke остаётся за архитектором — headless)
**Связанные тикеты:** T015 (sidebar redesign, `9403f57`), T014 (perf), T003/T008 (devices)

## 0. Корень (подтверждён кодом)

T015 завёл `Icon(HardDrive)` (строка устройства) и `Icon(ArrowUp)` (eject), не
добавив SVG в `crates/chronos-fm-ui/assets/icons/`. Механика спама подтверждена
по gpui:

- `gpui::elements::svg.rs` — `Svg::paint` зовёт `window.paint_svg(...).log_err()`
  **на каждый кадр**; `paint_svg` → `svg_renderer.render_alpha_mask` →
  `asset_source.load(path)` (`chronos-fm-ui/src/assets.rs:15`), ошибка которого
  и печатается как `ERROR could not find asset at path "..."`.
- Неудачные загрузки **не кешируются** в sprite atlas (`get_or_insert_with` не
  инсертит при `Err`), поэтому попытка повторяется каждый кадр — две строки
  ERROR на кадр в смоке = hard-drive (каждое устройство) + arrow-up (Ventoy,
  eject). Это ещё и перф-нагрузка на рендер-потоке (T014).

## 1. Аудит всех используемых иконок

Полный список `IconName::*` в `crates/` (19 уникальных, комментарии вычтены):

| Используется | Файл | До фикса |
|---|---|---|
| `HardDrive` (sidebar) | `icons/hard-drive.svg` | ❌ отсутствовал |
| `ArrowUp` (eject, sidebar) | `icons/arrow-up.svg` | ❌ отсутствовал |
| `Info` (footer, только при `status_message`) | `icons/info.svg` | ❌ отсутствовал |
| Folder, File, Search, Close, Plus, Minus, ChevronRight/Down, CircleUser, Settings, SquareTerminal, Palette, GalleryVerticalEnd, LayoutDashboard, PanelBottomOpen | … | ✅ на месте |

`IconName::Sync` — только в комментарии footer.rs (`// Use a spinner icon if
available? IconName::Sync?`), не реальное использование.

**Третий пропущенный (`info.svg`) смок не заметил** — иконка Info в футере
рисуется только при наличии `status_message`, которого в смоке не было. Это
демонстрирует ценность теста-аудита: класс дефекта ловится целиком, а не по
двум известным файлам.

## 2. Что сделано

### 2.1 Новые SVG (lucide, стиль локального набора)

`crates/chronos-fm-ui/assets/icons/`: `hard-drive.svg`, `arrow-up.svg`,
`info.svg` — канонические lucide-пути из gpui-component-assets, оформлены как
остальные (лицензионный заголовок `@license lucide-static v0.561.0 - ISC`,
атрибут на строку).

### 2.2 Fail-fast: одна строка ERROR на путь, а не на кадр

`crates/chronos-fm-ui/src/assets.rs` — `Assets::load` теперь мемоизирует
отсутствующие пути в `static MISSING: OnceLock<Mutex<HashSet<String>>>`:

- Первый промах → `Err` (существующий `log_err` у gpui логирует один раз).
- Последующие → `Ok(None)` — gpui трактует как «нет ассета, ничего не рисуем»
  и молчит (`render_alpha_mask` → `Ok(None)` → `paint_svg` → `Ok(())`).

Lock poison-tolerant (`Err(poisoned) => poisoned.into_inner()`) — без новых
`unwrap`. После фикса в приложении ноль отсутствующих иконок, так что
мемоизация — страховочная сетка на будущее (при регрессии: 1 ERROR на путь +
один мутекс-лок на кадр на отсутствующий путь — ничтожно против прежнего
форматирования лога на каждый кадр).

### 2.3 Тест-аудит (весь класс дефекта)

`assets.rs` `#[cfg(test)]` — `every_used_icon_has_an_asset`:

- Обходит `crates/*/src` (кроме собственного файла — там тестовые заглушки и
  `format!`-шаблоны), вычитает `//` и `/* */` комментарии.
- Собирает `IconName::Variant` (→ kebab: `HardDrive` → `hard-drive`) и
  литералы `"icons/x.svg"` (`PageKind::icon_path` из root.rs).
- Ассертит, что каждый путь есть в embedded-бандле (`Assets::get`).

**Анти-проверка:** удаление всех трёх SVG → тест падает ровно с
`["icons/arrow-up.svg", "icons/hard-drive.svg", "icons/info.svg"]`;
восстановлено. Плюс юниты на сам сканер (комментарии, kebab, литералы) и на
поведение `load` (первый промах Err, дальше `Ok(None)`; существующий ассет
грузится).

### 2.4 Побочно: эллипсис пути в Places

`crates/chronos-fm-pages/src/explorer/view/sidebar.rs` — колонка
имя+путь устройства получила `flex_1().min_w(px(0.0))`, обе строки —
`whitespace_nowrap().overflow_hidden().text_ellipsis()`. Длинный
`/run/media/neo/Vento…` теперь обрезается с эллипсисом, кнопки действий не
выталкиваются за край карточки.

## 3. Загадка «—» под устройствами: расследование

Тикет предполагал «плейсхолдер размера/свободного места». По коду это
**опровергнуто**:

- Грепы `—` по всем `crates/` и по до-T015 сайдбару (`9403f57~1`) — ни одного
  в коде строки устройства; сайдбар вообще не рисует строку размера (хотя
  `Device.size_bytes` доступен — поле есть, рендера нет).
- `gpui-component` `Icon`/`ListItem` фолбэк-глиф не рисуют: при неудачном
  SVG-ассете `Svg::paint` ничего не рисует (только `log_err`).

Наиболее вероятный источник: **пустой layout-слот 16px** неотрисованного
`Icon(HardDrive)` (и невидимый eject), который визуально читается как одинокий
символ. После добавления SVG слоты заполняются иконками. Требует живого
подтверждения: если «—» сохранится — нужен скриншот для идентификации.

## 4. Верификация

| Чек | Результат |
|---|---|
| `cargo test -p chronos-fm-ui` | ✅ 34/34 (было 30, +4 новых) |
| `cargo test -p chronos-fm-pages --lib` | ✅ 86/86 |
| `cargo test --workspace` | ✅ 0 failures |
| `cargo clippy` в изменённых файлах | ✅ чисто (sidebar: только pre-existing `hover_bg` unused + `too_many_arguments` у нетронутой `device_action_icons`) |
| Анти-проверка аудита (удалены 3 SVG) | ✅ тест падает ровно с тремя путями |
| Live smoke (лог без `could not find asset`, иконка носителя + eject) | ⚠️ за архитектором (headless) |

## 5. Файлы

| Файл | Изменение |
|---|---|
| `crates/chronos-fm-ui/assets/icons/hard-drive.svg` | +новый (lucide) |
| `crates/chronos-fm-ui/assets/icons/arrow-up.svg` | +новый (lucide) |
| `crates/chronos-fm-ui/assets/icons/info.svg` | +новый (lucide) |
| `crates/chronos-fm-ui/src/assets.rs` | fail-fast мемоизация + 4 теста |
| `crates/chronos-fm-pages/src/explorer/view/sidebar.rs` | эллипсис пути (flex_1/min_w/text_ellipsis) |

## 6. Коммит-подпись (предложение)

```
fix: T019 — add missing device icons (hard-drive/arrow-up/info), fail-fast asset logging, icon-coverage audit test + sidebar path ellipsis
```
