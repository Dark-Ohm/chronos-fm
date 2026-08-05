# T002 — перенос структурных UI-паттернов ChronOS (elevated_card, section_header)

**Статус:** DONE (build + test чисто). Живой grim-прогон помечен для архитектора (среда headless, см. §4).
**Принято от:** T001 (`done/` — палитра `cx.theme()` живая).
**Сделал:** портирование двух паттернов из ChronOS T231 на наш стек (`gpui` builder + мост `chronos-fm-ui/theme.rs`), без `gpui-rsx`.

## 1. Что сделано

Новый модуль `crates/chronos-fm-ui/src/patterns.rs` с двумя публичными хелперами:

- `pub fn elevated_card(cx: &App) -> Div` — приподнятая карточка: фон
  `theme::bg_secondary(cx)` (elevated surface), `border` 1px, радиус `px(12.)`,
  тень `.shadow_md()`. Читает палитру через мост T001 → одинаково в dark/light.
- `pub fn section_header(cx: &App, title: &str, subtitle: &str) -> AnyElement` —
  акцентная черта 3×12px (`theme::accent(cx).opacity(0.85)`), title семиболд
  (`theme::fg`, `SEMIBOLD`, `px(12.5)`), subtitle muted mono
  (`theme::muted`, `text_xs`, `cx.theme().mono_font_family`).

Оба хелпера экспортированы из `crates/chronos-fm-ui/src/chronos_fm_ui.rs`
(`pub mod patterns;`).

Применены в **двух** поверхностях эксплорера (оба хелпера в каждой):

1. **`sidebar.rs`** — секция «Folder»/shortcuts. Раньше был голый `div()` с
   label «Folder» (`text_xs`, `SEMIBOLD`, `fg_secondary`), визуально не
   отделённый от контента. Теперь секция обёрнута в `elevated_card` и получила
   `section_header(cx, "Folders", "quick access")`. Это самый «режущий глаз»
   участок — голый заголовок без карточки среди плоского списка.
2. **`settings.rs`** — пустая панела настроек. Раньше — центрированный текст
   «⚙️ Settings» + подпись на голом фоне. Теперь контент лежит в
   `elevated_card` (`w(px(420.))`) с `section_header(cx, "Settings",
   "application preferences")`. Превращает placeholder в осмысленную панель.

Обоснование выбора: обе поверхности — чистая верстка без общего визуального
языка карточки/заголовка (именно то, что T002 закрывает), и обе безопасны —
**не** пересекаются с зоной `b3`/`b4` (`row.rs`/`list.rs`/`grid.rs`/`listing.rs`).

## 2. Зона файлов

| Файл | Действие |
|------|----------|
| `crates/chronos-fm-ui/src/patterns.rs` | **новый** — оба паттерна |
| `crates/chronos-fm-ui/src/chronos_fm_ui.rs` | `pub mod patterns;` |
| `crates/chronos-fm-pages/src/explorer/view/sidebar.rs` | `elevated_card` + `section_header` на секции Folders |
| `crates/chronos-fm-pages/src/settings.rs` | `elevated_card` + `section_header` на панели настроек |
| `docs/orchestration/tasks/report/T002-structural-ui-patterns-report.md` | отчёт |

**НЕ тронуто (по контракту):**
- `Source/gpui-component` (read-only) — не изменяли.
- `crates/chronos-fm-ui/src/theme.rs` (мост T001) — не изменяли.
- `docs/agents/active/b1..b4` зоны — `row.rs`/`list.rs`/`grid.rs`/`listing.rs`
  **не тронуты** (b3/b4 всё ещё в `active/`, `report/` пуст → считаем в работе).

## 3. Отклонения от референса (обоснованные)

- **Нет `elevation_apply_light_chrome`.** В ChronOS это обёртка вокруг
  `theme.elevation_popup()`; у gpui-component её нет и заводить не надо.
  Использовали встроенный `.shadow_md()` (две тени: offset y=4/2, black @0.1,
  blur 6/4, spread −1/−2) — это и есть эквивалент «приподнятости» в нашем стеке.
  Радиус зафиксирован `px(12.)` (в ChronOS брался из `elev.radius`).
- **Тень в тёмной теме слабо видна** (Tailwind-тени — чёрные). Это ожидаемо;
  видимый край приподнятости в dark даёт `border`. Так же ведёт себя сам
  gpui-component.
- **`section_header` возвращает `AnyElement`.** Внутри строки `title`/`subtitle`
  клонируются в `String` — `AnyElement` требует `'static` содержимого
  (`E0521` при передаче заимствованного `&str`). Фикс: `let title = title.to_string();`.

## 4. Верификация

- `cargo build --workspace` → **EXIT=0, 0 errors, 0 warnings** (наш код; только
  транзитивная future-incompat нота от `proc-macro-error2`, не наше).
- `cargo test --workspace` → **171 passed, 0 failed** (те же 171, что и в T001
  базовой линии — новых тестов паттерны не добавляют, регрессий нет).
- **Живой grim + пипетка (§4 спеки): СРЕДА HEADLESS — не выполнено.** Как и в
  T001, среда исполнителя не имеет дисплея. 4 grim-кадра (sidebar/settings ×
  dark/light) **не сняты** — финальная живая проверка (карточка визуально
  приподнята, заголовок читается отдельно) оставлена архитектору на GUI-машине.
  Программно подтверждено: хелперы читают `cx.theme()` (живой мост T001),
  `accent` = `#007acc` в обеих темах (из T001), `bg_secondary`/`border` берутся
  из той же палитры.

## 5. Коммит

`ui : elevated_card + section_header patterns from ChronOS T231 (T002)`
(5 файлов: patterns.rs, chronos_fm_ui.rs, sidebar.rs, settings.rs, report).
Не запушено (пуш не запрашивался).
