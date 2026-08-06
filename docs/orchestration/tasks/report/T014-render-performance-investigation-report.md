# T014 — Отчёт расследования: рендер-перформанс / 144 fps (perf-замер)

**Приоритет:** P1
**Дата:** 2026-08-06
**Статус:** РАССЛЕДОВАНИЕ ЗАВЕРШЕНО — нужен архитектурный выбор.

---

## 0. Окружение

| Параметр | Значение |
|---|---|
| Монитор | Samsung LC32G5xT, DP-1, **2560×1440 @ 144 Гц** |
| GPU | NVIDIA RTX 3070 (Vulkan, DiscreteGpu) |
| Композитор | Hyprland (Wayland) |
| Present mode | Mailbox (triple-buffering, без блокировки) |
| VRR | Выключен |
| Бинарь | Release, `debug=1 strip=none`, 353 MB с DWARF |
| perf | `/usr/bin/perf`, `-F 1000 -g`, 3 × 10s замеров |

**Вывод:** 144 fps физически достижимы. Монитор 144 Гц, GPU дискретный, софт-рендера нет, present mode Mailbox (не блокирует). Проблема в коде.

---

## 1. Perf-замер: топ-потребители CPU

### Режим 1: ПОКОЙ (idle) — 16,873 сэмпла за 10s

| Потребитель | Доля | Детали |
|---|---|---|
| `notify-rs` debouncer thread | ~30% | `next_tick`, `hash<PathBuf>`, `write<Sip13Rounds>` — file watcher хеширует пути в фоне |
| `taffy` layout engine | ~40% | `calculate_cross_size`, `determine_container_main_size`, `compute_preliminary` — **flexbox layout пересчитывается даже в покое** |
| `libc` mutex/futex | ~15% | `__lll_lock_wake_private` — блокировки в notify-rs |

### Режим 2: СКРОЛЛ (scroll) — 17,565 сэмплов за 10s

| Потребитель | Доля | Детали |
|---|---|---|
| `taffy` layout engine | **~70%** | `compute_child_layout`, `determine_hypothetical_cross_size`, `distribute_remaining_free_space`, `determine_flex_base_size`, `generate_anonymous_flex_items` — **полный пересчёт flexbox-дерева** |
| `request_layout` → `slotmap::insert` | ~10% | `new_with_children`, `try_insert_with_key` — **создание НОВЫХ taffy-нод при скролле** (не переиспользование) |
| `__libc_malloc` | ~10% | Аллокации под render, taffy-ноды |

### Режим 3: HOVER — 17,431 сэмпл за 10s

| Потребитель | Доля | Детали |
|---|---|---|
| `taffy` layout engine | **~65%** | `compute_inner` (block), `determine_flex_base_size`, `cache_get` |
| Text layout cache | ~15% | `layout_wrapped_line`, `CacheKey` `remove_entry`, `find_inner` — **ротация двухфреймового кеша текста** |
| `should_insert_hitbox` | ~5% | Hit-testing при движении мыши |
| `drop<str>` / `drop_slow` | ~10% | Деаллокация строк из element tree предыдущего кадра |

---

## 2. Root Cause Analysis

### 2.1 GPUI — immediate-mode архитектура

GPUI перестраивает **всё** дерево элементов каждый кадр:

```
Каждый кадр (draw):
  1. draw_roots() → root_element.request_layout()
  2. Каждый элемент → request_layout() → taffy.new_leaf/new_with_children
  3. compute_layout_with_measure() → полный пересчёт flexbox
  4. layout_engine.clear() → удаление ВСЕХ taffy-нод
```

Даже в покое (idle), если окно dirty (любое событие мыши), происходит полный цикл. При ~400 элементах и 144 fps это 57,600 taffy-нод/сек.

### 2.2 Gate по is_dirty() уже есть — но не помогает

В `window.rs:1529`:
```rust
if invalidator.is_dirty() || request_frame_options.force_render {
    window.draw(cx);  // ← вызывается только если dirty
}
```

Окно становится dirty при любом событии мыши (move/hover), даже если визуально ничего не меняется. В idle без движения мыши окно **не** dirty — `draw()` не вызывается. Но малейшее движение → dirty → полный relayout.

### 2.3 Почему ховер вызывает полный relayout

Ховер меняет стиль (`.hover(|s| s.bg(...))`) → элемент dirty → `request_layout()` → новый taffy-нод → `compute_layout` всего дерева. Хотя изменился только цвет фона, GPUI не различает «layout-изменения» и «paint-изменения».

### 2.4 Text cache — работает корректно, но дорого

Двухфреймовый кеш `line_layout` правильно кеширует текст. `remove_entry` в perf — это нормальная ротация кеша (перемещение записей из previous_frame в current_frame). Не баг, а architectural overhead.

---

## 3. Варианты решения

### Вариант A: GPUI — Layout Memoization (рекомендован)

**Суть:** Хешировать дерево стилей элементов. Если хеш совпадает с предыдущим кадром — пропустить `compute_layout_with_measure`, переиспользовать cached bounds.

**Где:** `taffy.rs` + `element.rs`
**Сложность:** Средняя (~80 строк)
**Выигрыш:** Убирает taffy overhead в idle и при hover (цвет меняется, layout — нет)
**Риск:** Нужно корректно определить что входит в хеш (стили, влияющие на layout vs paint)

### Вариант B: GPUI — Node Reuse / Reconciliation

**Суть:** Не удалять taffy-ноды между кадрами. При `request_layout()` искать существующий нод с теми же style+children — переиспользовать его LayoutId вместо создания нового.

**Где:** `taffy.rs`, `element.rs`, `window.rs`
**Сложность:** Высокая (~200 строк)
**Выигрыш:** Убирает создание/удаление нод (malloc/free overhead), ускоряет скролл
**Риск:** Архитектурный — меняет fundamental invariant GPUI (immutable element tree per frame)

### Вариант C: Chronos-FM — Flatten Element Tree

**Суть:** Уменьшить вложенность div-ов в рендере (меньше taffy-нод = меньше работы). Батчить `cx.notify()`.

**Где:** `view.rs`, `list.rs`, `sidebar.rs`, `row.rs`, `grid.rs`
**Сложность:** Низкая
**Выигрыш:** ~15-25% меньше taffy-нод
**Риск:** Минимальный

### Вариант D: Chronos-FM — Убрать notify-rs overhead

**Суть:** Увеличить debounce timeout file watcher (сейчас 2s → 5s) или выключить watcher когда окно не в фокусе.

**Где:** `search/engine.rs:100`
**Сложность:** Тривиальная (1 строка)
**Выигрыш:** ~30% CPU в idle (фоновый поток)
**Риск:** Поиск будет обновляться реже

---

## 4. Рекомендация

**Краткосрочно:** D (notify-rs timeout) + C (flatten tree) — дают быстрый выигрыш без риска.

**Среднесрочно:** A (layout memoization) в GPUI-форке — самый большой выигрыш при разумной сложности.

**Долгосрочно:** B (node reuse) — максимальный выигрыш, но требует архитектурного решения и полного регрессионного тестирования GPUI.

---

## 5. Данные замера

| Файл | Режим | Сэмплов | Размер |
|---|---|---|---|
| `/tmp/perf_idle.data` | Покой | 16,873 | 1.5 MB |
| `/tmp/perf_scroll.data` | Скролл | 17,565 | 1.6 MB |
| `/tmp/perf_hover.data` | Hover | 17,431 | 1.5 MB |

Бинарь: `target/release/chronos-fm`, 353 MB, `with debug_info, not stripped`.
Профиль сборки: `opt-level=3 lto=thin codegen-units=1 debug=1 strip=none`.

---

## 6. Коммиты

Правок коду **нет** (recon-only). Тикет требует архитектурного решения перед кодом.
