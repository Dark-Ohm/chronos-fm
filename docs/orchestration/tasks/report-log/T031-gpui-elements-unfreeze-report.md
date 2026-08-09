# T031 — Отчёт: расконсервация `gpui_elements`

> ## ✅ ARCHITECT VERDICT: **ACCEPT + DELETE** (2026-08-09)
>
> Investigation accepted: 6 unique API-drift errors, all crate-side vs
> Zed-fork gpui. Crate unreferenced; Input covered by gpui-component.
> **Decision: remove crate** (not re-port). Executed in Source
> `fa2d64f` (`chore: remove unreferenced gpui_elements`).
>
> No Source/gpui API additions (Wall 1). Ticket → `done/`.

**Дата:** 2026-08-09
**Исполнитель:** Buffy (executor)
**Статус:** ✅ Расследование завершено. Рекомендация: удалить крейт.

## Фактические ошибки (сегодня, не из комментария)

Комментарий `Source/Cargo.toml:36-39` утверждал «7 API-drift errors».
После включения крейта в `members` и `cargo check -p gpui_elements`
фактический вывод: **7 ошибок компилятора, 6 уникальных** —
комментарий актуален, расхождений не добавилось.

```
error[E0433]: cannot find `ActionBindingCollection` in `gpui`
error[E0425]: cannot find type `ActionBindingCollection` in crate `gpui`
error[E0599]: no method named `set_scroll_offset` found for struct `Interactivity`
error[E0599]: no associated function `evaluate_wrap_width` found for struct `TextLayout`
error[E0599]: no associated function `evaluate_overflow` found for struct `TextLayout`
error[E0599]: no associated function `apply_truncation` found for struct `TextLayout`
error[E0599]: no method named `is_nearly_eq` found for struct `gpui::Point<T>`
```

## Классификация по каждой ошибке

| # | API | Файл в крейте | Категория | Обоснование |
|---|-----|---------------|-----------|-------------|
| 1 | `ActionBindingCollection` | `actions.rs:83,124,125` | **Drift in crate** | Крейт использует gpui-ce keybinding API; наш форк на Zed-основе использует другую action-систему |
| 2 | `Interactivity::set_scroll_offset` | `element.rs:304` | **Drift in crate** | Метод скролла из gpui-ce; в форке `Interactivity` не имеет этого метода |
| 3 | `TextLayout::evaluate_wrap_width` | `element.rs:590` | **Drift in crate** | Метод из gpui-ce text layout pipeline; форк использует другой API |
| 4 | `TextLayout::evaluate_overflow` | `element.rs:597` | **Drift in crate** | То же |
| 5 | `TextLayout::apply_truncation` | `element.rs:607` | **Drift in crate** | То же |
| 6 | `Point::is_nearly_eq` | `state.rs:567` | **Drift in crate** | Тривиальная утилита (`point.is_nearly_eq(&prev, EPSILON)`), отсутствует в форке. По духу — «gap in fork», но Стена 1 запрещает правки в `Source/gpui`. |

**Категории:**
- **Drift in crate (6/6):** все ошибки — крейт использует gpui-ce API, которых в нашем Zed-форке нет
- **Gap in fork (0/6):** API, которые были бы осмысленны в форке, но отсутствуют — `is_nearly_eq` подходит, но заблокирован Стеной 1
- **Dead code (0/6):** все вызовы осмысленны для функциональности крейта

## Вердикт

**Рекомендация: удалить крейт** (Стена 3 — «крейт может оказаться не нужен»).

Обоснование:
1. **Не используется никем.** Ни один крейт в Source/ не ссылается на `gpui_elements`.
2. **Стоимость починки высока.** Чтобы закрыть 6 ошибок, потребуется переписать взаимодействие крейта с gpui в четырёх подсистемах: keybinding, scroll, text layout, geometry. Это не «подправить импорты», а полноценный рефакторинг ~250 строк крейта под API форка.
3. **Ценность низкая.** Крейт предоставляет `<input>`/`<textarea`>-подобные элементы, но в экосистеме уже есть `gpui_component::input::Input` (из `Source/gpui-component`), который используется в Chronos-FM (S3-таб, Git-таб) и ChronOS.
4. **API будет дрейфовать дальше.** Даже если починить сегодня, следующий пулл из апстрима сломает крейт снова.

Альтернатива (не рекомендуется): починить все 6 ошибок в крейте (~250 строк рефакторинга). Это отложит проблему до следующего расхождения API.

## Базовые линии

| Чек | До | После |
|-----|----|-------|
| `cargo check --workspace` (Source) | ✅ | ✅ (без изменений) |
| `cargo check --workspace` (Chronos-FM) | ✅ | ✅ (без изменений) |

Изменений в код не вносилось — крейт был временно включён для замера, затем возвращён в excluded. `git -C Source diff` пуст.

## Что дальше

1. Архитектор принимает решение: удалить крейт физически (rm -rf + git rm) или оставить excluded навсегда.
2. При удалении: убрать `gpui_elements/` из `Source/`, удалить комментарий из `Cargo.toml`.
