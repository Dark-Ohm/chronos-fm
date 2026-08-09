# T028 — Отчёт: 11 примеров, `story-web` и `webview`

> ## ✅ ARCHITECT VERDICT: **PARTIAL-ACCEPT / CLOSED** (2026-08-09)
>
> Scope was verify-only (no code). **Part 1 build: ACCEPT** (11 examples +
> webview + story-web compile). **Part 1 run: honest fail** — 9/11 panic in
> `Source/gpui` text measurement (`text.rs:777`); not a kit defect. Wall 1
> forbids fix in T028 → residual **T035**. **Part 2 WASM: ACCEPT wall 2.2**
> (nightly/`wasm_thread` feature; no rustup) → residual **T036**.
>
> Ticket closed. No Source/Chronos-FM code changes required for T028.

**Дата:** 2026-08-09
**Исполнитель:** Buffy (executor)
**Статус:** ⚠️ Часть 1 — сборка зелёная, 9/11 падают (fork defect). Часть 2 — Стена 2.2 (nightly).

## Часть 1 — примеры и webview

### Сборка: ✅ все 11 примеров + webview + story-web

```
$ cargo build --workspace
   Compiling hello_world, focus_trap, tooltip_top_edge, dialog_overlay,
             sidebar, text_selection, input, window_title, app_assets,
             system_monitor, webview, root_borderless,
             gpui-wry, gpui-component-story-web
   Finished `dev` profile [unoptimized + debuginfo] in 47.02s
```

**Exit:** 0. Все 11 примеров + webview + story-web собрались без ошибок.

### Запуск: 9/11 падают — fork defect (`text.rs:777`)

Все примеры, рендерящие текст, падают с идентичной ошибкой:

```
thread 'main' panicked at Source/gpui/src/elements/text.rs:777:14:
called `Result::unwrap()` on an `Err` value: measurement has not been performed on <string>
```

Причина: `measurement has not been performed` — текст не размечен перед рендером. Это дефект форка `Source/gpui`, не kit'а и не примеров.

| Пример | Собрался | Запустился | Причина падения |
|--------|----------|------------|-----------------|
| `hello_world` | ✅ | ❌ | text measurement: "Hello, World!" |
| `focus_trap` | ✅ | ❌ | text measurement: "Focus Trap Example" |
| `tooltip_top_edge` | ✅ | ❌ | text measurement: "Hover for tooltip" |
| `dialog_overlay` | ✅ | ❌ | text measurement: "Dialog & Sheet" |
| `text_selection` | ✅ | ❌ | text measurement: "Hello! How can I help you today?" |
| `input` | ✅ | ❌ | text measurement: "" |
| `window_title` | ✅ | ❌ | text measurement: "App with Custom title bar" |
| `system_monitor` | ✅ | ❌ | text measurement: "System" |
| `root_borderless` | ✅ | ❌ | text measurement: "Root::bordered(false)" |
| `sidebar` | ✅ | ✅ | — |
| `app_assets` | ✅ | ✅ | — |
| `webview` | ✅ | ⚠️ | Gtk-CRITICAL warning (не panic) |

**Вердикт:** дефект форка. `text.rs:777` — `measurement has not been performed`. Затрагивает 9 из 11 примеров. Чинить здесь нельзя (Стена 1 — `Source/gpui` держит ChronOS, greeter, Chronos-FM, Chronos-IDE). Кандидат в отдельный тикет.

### Стены части 1

| Стена | Суть | Сработала? |
|-------|------|------------|
| 1.1 | `webview` → webkit2gtk | **Нет** — webkit2gtk найден, пример собрался |
| 1.2 | `--workspace` включает story | Соблюдена — story пересобран, но не засчитан за новый результат |
| 1.3 | `exit 0` ≠ работает | **Сработала** — 9 примеров падают при запуске несмотря на зелёную сборку |

## Часть 2 — WASM (`story-web`)

### Стена 2.2 — nightly required. Остановка.

```
$ bash scripts/build-wasm.sh
error[E0554]: `#![feature]` may not be used on the stable release channel
 --> wasm_thread-0.3.3/src/lib.rs:1:37
  |
1 | #![cfg_attr(target_arch = "wasm32", feature(stdarch_wasm_atomic_wait))]
  |                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Причина:** `wasm_thread 0.3.3` использует `#![feature(stdarch_wasm_atomic_wait)]` — nightly-only фичу. Системный Rust 1.97.1 — stable. `rustup` отсутствует.

**Решение по Стене 2.2:** остановка. Ставить rustup рядом с системным Rust — это смена тулчейна для 5 проектов, отдельное решение, не побочный эффект T028.

### Статус шагов (таблица из тикета)

| Шаг | Статус |
|-----|--------|
| 1. Поставить std под wasm | ✅ сделано до раздачи |
| 2. Поставить wasm-bindgen CLI 0.2.121 | ✅ сделано до раздачи |
| 3. `scripts/build-wasm.sh` | ❌ Стена 2.2 — nightly required |
| 4. `bun install && bun run dev` | 🔒 заблокирован шагом 3 |
| 5. Кадр в браузере | 🔒 заблокирован |

### Инфраструктура готова (проверено)

| Что | Состояние |
|-----|-----------|
| `rustc` 1.97.1 (stable) | ✅ |
| `wasm32-unknown-unknown` std | ✅ |
| `wasm-bindgen` CLI 0.2.121 | ✅ (совпадает с локом) |
| `wasm-component-ld` | ✅ |
| `bun` | ✅ |
| `rustup` | ❌ **отсутствует** — блокер |

## Итог

| Часть | Результат | Блокер |
|-------|-----------|--------|
| Сборка примеров | ✅ 11/11 + webview + story-web | — |
| Запуск примеров | ⚠️ 2/11 работают, 9 падают | Fork defect: `text.rs:777` text measurement |
| WASM сборка | ❌ | Стена 2.2: nightly required, rustup отсутствует |

**Кандидаты в тикеты:**
1. **text measurement panic** — `Source/gpui/src/elements/text.rs:777`, 9 примеров падают. Дефект форка.
2. **WASM/nightly** — установить rustup nightly для wasm-сборки или найти обход `wasm_thread::feature`.

**`git -C Source status`** — чист. Изменений не вносилось.
