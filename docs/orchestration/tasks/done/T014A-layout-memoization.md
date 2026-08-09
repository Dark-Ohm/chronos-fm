# T014-A — GPUI layout memoization (skip taffy when layout tree unchanged)

> ## ✅ ARCHITECT VERDICT: **ACCEPT** (2026-08-09)
>
> Layout memoization in Source/gpui (`6c1c8c3`). Unit tests green;
> chronos-fm check green. Ticket → `done/`. Report → `report-log/`.

**Приоритет:** P1 — следующий шаг umbrella T014 после AFTER-пакета.
**Родитель:** T014. Разблокирован AFTER (main-thread taffy ~50% UI self).
**Скоуп:** `../Source/gpui` (форк). Chronos-FM app code **не** трогать, кроме
опционального content-key в text path если нужен API.

## Цель

Если layout-relevant дерево (стили → taffy, структура, content-key для
measured leaves) совпадает с предыдущим кадром — **не** вызывать
`taffy::compute_layout_with_measure`; переиспользовать absolute bounds.

Paint-only изменения (`.hover(|s| s.bg(...))`, border color) **не** входят
в layout fingerprint → hover без relayout.

## Не делать

- Variant B (node reuse / no clear) — отдельное решение
- Менять Chronos-FM listing/sidebar
- Pause-on-unfocus watcher

## Приёмка

1. `cargo check -p gpui` / workspace check that Chronos-FM still builds
2. Unit test in `taffy.rs`: two frames same styles → second skips compute
   (use internal counter or test hook)
3. Live: hover idle — taffy share on main thread drops vs AFTER baseline
   (optional same-session perf; not ship-blocker if unit proves skip)

## Вердикт после кода

Architect stamp after report in `report/`.
