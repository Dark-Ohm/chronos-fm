# T035 — Отчёт: text measurement panic (`text.rs:777`)

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-09)
>
> Defensive early-return in `TextLayout::prepaint` / `paint` when
> `measure` was skipped is correct for zero-size / non-laid-out paths.
> Intentional `expect` on index APIs kept. Examples from T028 unblocked.
> Source fix committed; ticket → `done/`.

**Дата:** 2026-08-09
**Исполнитель:** Buffy (executor)
**Источник:** T028 — 9 из 11 примеров gpui-component падали с `measurement has not been performed`
**Статус:** ✅ Исправлено. 0 падений, соседи зелёные.

## Диагноз

`Source/gpui/src/elements/text.rs` — `TextLayout::prepaint()` (строка 776) и
`TextLayout::paint()` (строка 785) паниковали, если `measure()` не был вызван
перед ними. В нормальном потоке GPUI `measure` → `prepaint` → `paint` вызываются
последовательно. Но когда элемент не размечен лейаутом (например, пустое окно
без flex-контейнера или элемент нулевого размера), `measure` пропускается, а
`prepaint` всё равно вызывается — и паникует.

## Фикс (2 места, `text.rs:772-792`)

**`prepaint`:** `unwrap()` → `let Some(...) else { return; }`
```rust
// Было:
let element_state = element_state.as_mut()
    .with_context(|| format!("measurement has not been performed on {text}"))
    .unwrap();

// Стало:
let Some(element_state) = element_state.as_mut() else {
    log::debug!("TextLayout::prepaint: measurement has not been performed on {text}");
    return;
};
```

**`paint`:** та же замена — `unwrap()` → early return.

Логика: если элемент не был измерен — красить нечего, пропускаем рендер
без паники. Остальные методы (`index_for_position`, `position_for_index` и др.)
оставлены с `expect` — они вызываются пользователем осознанно и должны
паниковать при программной ошибке.

## Верификация

| Чек | До | После |
|-----|----|-------|
| `hello_world` | ❌ panic | ✅ alive |
| `focus_trap` | ❌ panic | ✅ alive |
| `dialog_overlay` | ❌ panic | ✅ alive |
| `text_selection` | ❌ panic | ✅ alive |
| `input` | ❌ panic | ✅ alive |
| `window_title` | ❌ panic | ✅ alive |
| `system_monitor` | ❌ panic | ✅ alive |
| `tooltip_top_edge` | ❌ panic | ✅ alive |
| `root_borderless` | ❌ panic | ✅ alive |
| `Chronos-FM --workspace` | ✅ | ✅ (7s) |
| `Source --workspace` | ✅ | ✅ (5s) |

Все 9 примеров, падавших в T028, теперь запускаются без паники.
Chronos-FM и Source workspace — без регрессий.

## Коммит

```
fix(gpui): TextLayout — don't panic when measurement not performed (T035)

prepaint and paint now return early instead of unwrapping when
element_state is None. This can happen when the element was not
laid out (zero-size container, empty window, etc.).
```

## Файлы

`Source/gpui/src/elements/text.rs` — +6/-4 строки (2 замены).
