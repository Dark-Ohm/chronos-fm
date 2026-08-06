# T006 Tasks 2–5 — Batch Rename dialog + entry point (REPORT)

**Тикет:** T006 — Batch rename (план `docs/superpowers/plans/2026-08-06-batch-rename.md`)
**Задачи:** Task 2 (диалог), Task 3 (проводка в pane), Task 4 (overlay), Task 5 (entry point)
**Статус:** ⬜ inbox → на приёмку (архитектор). Принят → `report-log/`.
**Дата:** 2026-08-06

## Что сделано

### Task 2 — диалог `crates/chronos-fm-pages/src/explorer/batch_rename.rs` (NEW)
`BatchRenameDialog` — модальный диалог, модель — `PropertiesDialog`:
- 4 `Entity<InputState>`: pattern / find / replace / start (по паттерну search_bar);
- **live preview**: подписка `cx.subscribe` на `InputEvent::Change` каждого инпута
  (подписки в поле `subs: Vec<Subscription>` — по конвенции `list_setup.rs:push(sub)`),
  каждый keystroke → `refresh_preview` → `services::fs::batch_rename::build_preview`;
- `can_apply()`: превью непусто + `last_error` пуст + есть строка с изменением;
- `apply()`: `ops::rename_in_place` по превью в порядке выделения, сбор per-file ошибок,
  `on_committed(cx, errors)` (Rc-колбэк, тип-алиасы `CommitCallback`/`CancelCallback` — clippy type_complexity);
- `cancel()` → `on_cancel(cx)`; `pattern_input()` — `#[cfg(test)]` для теста;
- `impl Render`: `elevated_card` + `section_header("Batch Rename", "N files")` + 4 `input_row`
  (label + hint + `Input`) + banner ошибки (`theme::danger`) + `preview_list`
  (`old → new`, `ResolvedCollision` dimmed + `(auto-resolved)`) + footer (Cancel + Apply);
- **Apply всегда виден**, при `!can_apply` — `opacity(0.5)` и без `on_click` (disabled;
  `Button::disabled` — приватный сеттер, поэтому через `.when`).
- `mod batch_rename` в `explorer.rs`.

### Task 3 — проводка в pane
- `state.rs`: поле `pub batch_rename: Option<Entity<BatchRenameDialog>>` (+ `None` в `new`);
- `batch_rename.rs`: `impl ExplorerPane { open_batch_rename(entries, window, cx),
  close_batch_rename(cx) }` — колбэки через `pane.update` (конвенция context-menu),
  on_committed: `batch_rename = None; reload(); set_status(Error)` (ошибка после reload — правило репо);
- `state.rs`: хелпер `filtered_entries_for_selection()` (выборка в порядке строк) — нужен Task 5;
- **тест** `batch_rename_dialog_previews_pattern_and_closes`: 2 файла → select → open →
  pattern `{name}_{n:3}.{ext}` через `pattern_input()` + `set_value` → preview
  `a_001.txt`/`b_002.txt` all `Ok` + `can_apply` → close → на диске ничего не тронуто.
  Всё в одном `window.update` closure — живой `Input` не остаётся на границе update
  (харнесс без `gpui_component::Root`, та же ловушка, что и `new_folder`).

### Task 4 — overlay `view.rs`
- `render_batch_rename_dialog(page, cx)` — mirror `render_properties_dialog`: scrim
  `bg(hsla(0,0,0,0.4))`, клик-вне → `close_batch_rename`, `.child(dialog.clone())`;
- Escape-ветка в pane key-handler до очистки selection.
- **Критический фикс по ревью**: карточка обёрнута в `div().on_mouse_down(stop_propagation())` —
  иначе клик внутри (фокус инпута, кнопка) bubble-ился на scrim и закрывал диалог на
  mousedown, а `Button::on_click` (mouse-up) никогда не срабатывал бы → Apply молча не работал.

### Task 5 — entry point `context_menu.rs`
- Пункт **Rename** больше не disabled при мультиселекте:
  `filtered_entries_for_selection().len() > 1` → `open_batch_rename` (диалог),
  иначе `begin_rename(ix)` (inline, b1);
- удалено мёртвое поле `single_selected` из `ContextMenuState` (struct + `for_file`/`for_directory`
  + snap в `navigation.rs::open_context_menu` + ассерт в тесте).

## Верификация
- `cargo test -p chronos-fm-pages -- batch_rename` → 1/1 (новый диалог-тест);
- `cargo test -p chronos-fm-pages` → **69/69** (68 + 1);
- `cargo test --workspace` → **4+51+69+48+16+30 = 218 passed, 0 failed** (EXIT=0);
- `cargo build -p chronos-fm` → EXIT=0; `cargo clippy -p chronos-fm-pages` → чисто в
  batch_rename.rs/context_menu.rs/state.rs/navigation.rs/view.rs.

## Код-ревью (code-reviewer-deepseek-flash)
- **Critical (исправлено)**: scrim-bubbling — клик внутри карточки закрывал диалог до
  `Button::on_click` → `stop_propagation` на карточке. Та же защита отсутствует у
  properties-диалога (read-only, не трогал — вне зоны T006, отмечено для T-тикета приёмки);
- **Minor (исправлено)**: `apply()` теперь `zip(entries, preview)` вместо `find by old_name`
  (build_preview сохраняет порядок; имена уникальны в одной директории) — O(n), без неоднозначности;
  Apply виден всегда (disabled-стиль через opacity), а не скрыт.
- Подписки в `subs`, без use-before-init; Escape-приоритет корректен.

## Оговорки
1. **Manual smoke не выполнялся** — GUI-сессии нет; только юнит-тесты + код-ревью.
2. **Escape-close** срабатывает, когда фокус на pane; при фокусе в инпуте — кнопка Cancel/scrim.
3. Диалог рендерит живые `Input` — в проде root обёрнут в `Root::new`, ок; харнесс-ограничение задокументировано.
4. ~~`properties` имеет ту же scrim-bubbling особенность~~ — **закрыто тем же фиксом**: `render_properties_dialog` обёрнут в `stop_propagation`-карточку (как batch_rename).

## Готовность
T006 Tasks 2–5 закрыты. План полностью исполнен (Tasks 1–5); остался Task 6 (verification pass —
по сути уже прогнан здесь) и финальный отчёт `T006-batch-rename-report.md` при создании T-тикета.
