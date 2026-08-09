# T015 — Сайдбар: убрать мёртвый блок, свести «Места» и «Устройства» к одному ритму

> ## ✅ ARCHITECT VERDICT: **ACCEPT** (2026-08-09)
>
> Architect stamp. Sync-only close. See
> `report-log/T015-sidebar-redesign-report.md`. Ticket → `done/`.

**Приоритет:** P2 — не блокер функционала, а polish UI.
**Родитель:** самостоятельный, не T014.

## 6/6 пунктов и где они закрыты (sync-fresh, 2026-08-09)

| # | Пункт из ТЗ | Где закрыто | Комментарий |
|---|---|---|---|
| 1 | Удалить мёртвый блок Home/Favorites/Recent/Trash | `9403f57` (`sidebar.rs:1-10, 28-30`) | `sidebar_item()` для top block удалён; четыре строки исчезли из render-цепи |
| 2 | Убрать дубль `Home` (был и в верхнем блоке, и в Folders) | `9403f57` | Один `Home` остался — в быстром доступе как первый row, навигирует |
| 3 | Свести три карточки (обманка + Folders + Devices) к одному ритму | `9403f57` | Один `elevated_card("Places")` содержит Folders + Devices через единую `ListItem`-цепочку |
| 4 | Карточка Places заканчивается на середине — дисбаланс ритма | `9403f57` (`sidebar.rs:30-31`) | `elevated_card(cx).h_full()` — карточка тянется до низа сайдбара. **Visual proof не снят** в этой сессии (`pidof chronos-fm` пуст) |
| 5 | Устройства: эллипсис пути + padding от правого края + иконки вместо текста | `18ab839` (`T019`) | `flex_1` + `min_w(0)` + `text_ellipsis()` на колонке mount-point; `device_action_icons()` с hover-revealed `Minus`/`ArrowUp`. Лог `could not find asset` чист |
| 6 | Убрать I/O из рендера — `get_shortcuts()` с 4× `Path::exists` на кадр | `9403f57` (`state.rs:compute_shortcuts()` + `36 +`) | Shortcuts считаются один раз при construction панели, не на кадр |

## Что было → что стало (высота 2026-08-09)

**Было (pre‑Approach A, до `9403f57`):**
- Dead top block (`Home/Favorites/Recent/Trash`) — четыре кнопки, ноль
  `on_click` (`sidebar_item()` имел только hover).
- Два `Home`: один мёртвый в верхнем блоке, один живой в `Folders`.
- Три карточки друг под другом: пустышка «Places» + `Folders` + `Devices` —
  без общего ритма; правая кромка `unmount`/`eject` упиралась в край
  карточки.
- `get_shortcuts()` — синхронный `Path::exists()` на каждом рендере
  сайдбара.

**Стало (post‑`9403f57` + `18ab839`):**
- Один `elevated_card("Places")` с `section_header`, вмещающий
  `[…folders]` + `[…devices]` в одной `ListItem`-цепочке.
- `compute_shortcuts()` при construction, кешируется в `ExplorerPane`
  (`state.rs:+36`).
- Hover‑revealed иконки `Minus` / `ArrowUp` с `stop_propagation` —
  клик по строке нavigирует, клик по иконке делает действие и не
  всплывает в row‑listener.
- `flex_1 + min_w(0) + text_ellipsis` на колонке mount‑point — пути
  не толкают action icons за пределы карточки.

## Forward‑looking (записано, не блокер)

- **Punkt #4 visual proof не снят в этой сессии**, потому что
  `target/release/chronos-fm` не запущен. Если на live прогоне
  обнаружится, что `h_full()` не дотягивает карточку (родительский
  контейнер не имеет `h_full`, или `elevated_card` pattern перезаписывает
  size‑цепочку), это **отдельный тикет**, не правка T015. Заводится с
  гримом‑скриншотом и тегом «T015-residual».
- **Drop‑zone как новая фича** — отдельный тикет (если вообще нужен;
  пока не поступало user request). Архитектурный вопрос «это осмысленная
  зона для drop‑target в дизайн‑системе Dolphin» — решается в этом
  отдельном тикете. К T015 отношения не имеет.

## Коллизии (final)

- T019 закрыт, не конфликт.
- T022 закрыт; правки в `sidebar.rs` не делал (только `engine.rs` и
  `row.rs`). Коллизия, обозначенная в T015 message чекпоинта #4,
  не состоялась.
- T010/T011 (active) — `sidebar.rs` мимо скоупа, не пересекаются.
- `chronos-fm-ui::patterns` / `theme` в этой итерации не трогали —
  координация с T0xx, которые могут править `patterns.rs`, остаётся
  в силе на будущее.

## Отчёт

`docs/orchestration/tasks/report/T015-sidebar-redesign-report.md`
(inbox). **Приёмка — архитектор:** принят → тикет и отчёт →
соответственно `done/` и `report-log/`. До приёмки всё лежит в
inbox (`active/` + `report/`); я **не двигаю** файлы сам, потому что
это решение архитектора.
