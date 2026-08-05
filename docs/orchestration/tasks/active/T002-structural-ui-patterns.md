# T002 — перенос структурных UI-паттернов ChronOS (elevated_card, section_header)

**Приоритет:** P2 — визуальный полиш поверх T001 (принят, палитра живая).
**Роль:** реализация по образцу, самодостаточное задание.
**Источник:** ChronOS `crates/app/src/side_panel_right/tab/ui.rs` (T231
pattern), 1:1 портирование логики под наш стек (`gpui` builder + мост
`chronos-fm-ui/theme.rs` из T001, БЕЗ `gpui-rsx` — тут его нет).
**Зависит от:** T001 (`done/`, палитра `cx.theme()` живая и подтверждена
пипеткой обоих режимов).

## Контекст

T001 дал единый источник цвета (`gpui_component::Theme` + Chronos-палитра).
Сейчас компоненты (`pane.rs`, `file_list.rs`, `sidebar.rs`, listing/*.rs
и т.д.) красятся правильно, но верстаются кто во что горазд — нет общего
визуального языка карточки/секции, как в ChronOS после T231. Цель этого
тикета — тот же язык здесь: приподнятая карточка (`elevated_card`) и
заголовок секции с акцентной чертой (`section_header`), как общие хелперы,
которые дальше используются во ВСЕХ панелях эксплорера (sidebar, preview,
settings, extensions, git, s3), не только в одном месте.

**Референс-код (ChronOS, read-only, только для портирования логики — НЕ
копировать 0 строк дословно, у нас другой Theme-тип и нет gpui-rsx):**

```rust
// ChronOS crates/app/src/side_panel_right/tab/ui.rs — elevated_card
pub(crate) fn elevated_card(theme: Theme) -> Div {
    let elev = theme.elevation_popup();
    let card = div()
        .relative().w_full().flex().flex_col()
        .gap(px(16.)).px(px(16.)).py(px(16.))
        .bg(theme.bg.elevated)
        .border_1().border_color(theme.border.subtle)
        .rounded(elev.radius)
        .shadow(elev.shadows.to_vec());
    elevation_apply_light_chrome(&elev, card)
}

// section_header — акцентная черта 3×12px + семиболд title + муted mono subtitle
pub(crate) fn section_header(theme: Theme, title: &str, subtitle: &str) -> AnyElement {
    // div().w_full().flex_col().gap(px(4.))
    //   .child(div().flex().items_center().gap(px(6.))
    //     .child(div().w(px(3.)).h(px(12.)).rounded(px(1.5)).bg(theme.accent.primary.opacity(0.85)))
    //     .child(div().text_color(theme.text.primary).text_size(px(12.5))
    //            .font_weight(FontWeight::SEMIBOLD).child(title)))
    //   .child(div().text_color(theme.text.muted).text_xs()
    //          .font_family(theme.font_mono).child(subtitle))
}
```

Полные тела — читать в дереве ChronOS напрямую (путь выше), это не
исчерпывающая копия, а ориентир для верстки.

## Что нужно

1. Новый модуль `crates/chronos-fm-ui/src/patterns.rs` (или в
   `components/` — на усмотрение исполнителя, но один файл, не
   размазывать), публичные функции:
   - `pub fn elevated_card(cx: &App) -> Div` — читает `cx.theme()`
     (мост из T001), фон/бордер/радиус из палитры. У gpui-component
     свой билдер теней (`.shadow_md()`/`.shadow_lg()` — методы `Styled`
     из апстрим `gpui`, НЕ нужен свой `elevation_apply_light_chrome` —
     это ChronOS-специфичная обёртка, которой у нас нет и заводить не
     нужно, используем то что уже есть в `gpui`/`gpui-component`).
   - `pub fn section_header(cx: &App, title: &str, subtitle: &str) ->
     AnyElement` — акцентная черта + title + subtitle, тот же визуальный
     язык (черта 3×12px `rounded`, title семиболд `text_color`
     primary, subtitle muted+mono).
2. Применить оба хелпера минимум в **двух** реальных поверхностях
   эксплорера (не заводить хелперы «в стол») — на усмотрение
   исполнителя выбрать две, где сейчас голая свёрстка без карточки/
   заголовка режет глаз сильнее всего (кандидаты: `sidebar.rs` секции
   Favorites/Folder, `settings.rs` группы настроек, `preview.rs`
   пустая панель) — обосновать выбор в отчёте.

## Явно вне скоупа

- **Empty-state паттерн** (что показывать когда пусто/мало данных) —
  НЕ входит в этот тикет. В ChronOS это T252, ещё активен (не закрыт,
  ждёт отчёта от GPT 5.6 sol на 2026-08-05), паттерн там ещё не решён
  окончательно — портировать нечего. Заведём отдельным T-тикетом после
  того как T252 закроется в ChronOS.
- `elevated_card`/`section_header` — только визуальная верстка, НЕ
  трогать функциональный трек `docs/agents/active/b1..b4`
  (clipboard/rename/file-ops/context-menu) — не пересекаться файлами
  с `row.rs`/`list.rs`/`grid.rs`/`listing.rs`, если они сейчас заняты
  параллельным исполнителем (сверить `docs/agents/report/` перед
  началом — если b3/b4 ещё в работе, эти файлы не трогать вообще).

## Зона файлов

- Новый `crates/chronos-fm-ui/src/patterns.rs` (или аналог).
- `crates/chronos-fm-ui/src/lib.rs` (или `chronos_fm_ui.rs`) — экспорт
  нового модуля.
- Ровно 2 файла-потребителя (выбор исполнителя, обосновать).
- **НЕ трогать:** `Source/gpui-component` (read-only), `crates/chronos-fm-ui/src/theme.rs`
  (мост T001, уже готов, не переделывать), `docs/agents/active/b1..b4`
  зоны (см. выше).

## Верификация

- `cargo build --workspace` + `cargo test --workspace` чисто.
- Живой запуск (релизная сборка, НЕ headless — если среда исполнителя
  headless, как в T001, явно пометить в отчёте и оставить финальную
  живую проверку архитектору, как в T001 §2.3–§4).
- Grim-кадры двух выбранных поверхностей, обе темы (dark/light) — 4
  кадра минимум, приложить к отчёту.
- Глазами: карточка визуально приподнята над фоном (видна тень/бордер),
  заголовок секции читается отдельно от контента, не сливается.

## Отчёт

`docs/orchestration/tasks/report/T002-structural-ui-patterns-report.md`
(inbox). Приёмка — архитектор лично (та же дисциплина, что T001: грепы/
диф/build/test + живой grim, отчёту на слово не верить). Принят →
`report-log/`, тикет → `done/`. Отклонён → `rejected/`.

## Коммит

`ui : elevated_card + section_header patterns from ChronOS T231 (T002)`
