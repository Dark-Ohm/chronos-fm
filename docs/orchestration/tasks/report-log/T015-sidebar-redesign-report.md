# T015 — Отчёт: сайдбар редизайн

> ## ✅ ARCHITECT VERDICT: **ACCEPT** (2026-08-09)
>
> Stamp by architect (not exec). Sync-only: 6/6 closed in
> `9403f57` + `18ab839`; `h_full()` present at `sidebar.rs:25-26`.
> No code change this iteration. Visual proof optional residual
> only if live shows card not filling (new ticket, not reopen T015).
> Report → `report-log/`; ticket → `done/`.

## §0. Что делала эта итерация

T015 был уже реализован — Approach A в commits `9403f57` (T015
основной) + `18ab839` (T019 follow-up). Ticket message чекпоинта #4
(2026-08-06) говорил «остался открытый пункт #4 — карточка Places
заканчивается на середине сайдбара»; проверка показала, что `h_full()`
уже стоит в `crates/chronos-fm-pages/src/explorer/view/sidebar.rs:30-31`
с явным комментарием, ссылающимся именно на T015 open item #4. Sync
восстановлен, никаких правок в коде.

## §1. Evidence — что закрыто чем

| Пункт из ТЗ | Закрыт в | Citation |
|---|---|---|
| 1. Удалить мёртвый блок Home/Favorites/Recent/Trash | `9403f57` | Коммит‑message явно: "Dead Home/Favorites/Recent/Trash removed." Diff показывает удаление `sidebar_item()` для четырёх записей. |
| 2. Убрать дубль `Home` | `9403f57` | Один `Home` остаётся в `compute_shortcuts()` как первый row. |
| 3. Один ритм секций (Folders + Devices внутри единой Places-карточки) | `9403f57` | `git show 9403f57`: "single elevated_card Places replacing dead top block + Folders + Devices". |
| 4. Карточка Places растягивается до низа сайдбара | `9403f57` | `crates/chronos-fm-pages/src/explorer/view/sidebar.rs:30-31` — `elevated_card(cx).h_full()`. **Visual proof не снят** — `pidof chronos-fm` пуст. |
| 5. Эллипсис пути, правый padding, иконки действий | `18ab839` (T019) | `flex_1 + min_w(0) + text_ellipsis()` на колонке mount-point; `device_action_icons()` с hover-revealed `Minus`/`ArrowUp`. Лог `could not find asset` чист. |
| 6. `get_shortcuts()` убрать из рендера | `9403f57` | `state.rs + 36`: `compute_shortcuts()` вызывается один раз при construction панели. |

## §2. Sync‑gap (drift, который чинили в этой итерации)

**Drift в ticket message vs код:**

- Ticket `docs/orchestration/tasks/active/T015-sidebar-redesign.md`,
  чекпоинт #4 (2026-08-06), утверждал «остался ровно один пункт — карточка
  Places заканчивается на середине сайдбара».
- Этот ticket был написан до момента, когда Approach A в commit
  `9403f57` синхронизировался с деревом. После merge в main сообщение
  тикета не обновили.
- При проверке в этой сессии (2026-08-09) выяснилось, что код уже
  содержит фикс — `h_full()` в `sidebar.rs:30-31`. Diff `git diff HEAD`
  на `sidebar.rs` — пуст. То есть коммит уже в истории, не лежит в
  worktree.
- **Действие:** ticket message переписан. Top‑секция теперь содержит
  статус sync + recommendation, не self-verdict. Секция «Осталось
  ровно одно» удалена. Секция «Что не так» помечена как исторический
  backlog.

**Урок в ретроспективе:** после каждого коммита, который закрывает
пункты тикета, надо переписывать верхнюю секцию тикета, не оставлять
intermediate чекпоинты как описание состояния. Это уже не первый раз
(та же беда была в T019 vs T015); процесс исправим, но факт уже в
истории.

## §3. Что НЕ делали (явно)

Никаких новых правок в коде в этой итерации. Не правили:

- `crates/chronos-fm-pages/src/explorer/view/sidebar.rs` — он уже в
  желаемом виде post‑`9403f57`.
- `crates/chronos-fm-pages/src/explorer/state.rs` — `compute_shortcuts()`
  уже там.
- `crates/chronos-fm-ui/src/patterns.rs` — паттерн `elevated_card()` /
  `section_header()` без изменений.

Если позже понадобится любой из этих файлов править — синхронизация с
T015 не требуется, тикет в этом не участвует.

## §4. Forward‑looking (записано, не блокер; **не часть verdict'а**)

### 4.1 Visual proof для пункта #4

`h_full()` в коде стоит, но **в этой сессии нет live screenshot**:
`pidof chronos-fm` пуст. Если при следующем live прогоне обнаружится,
что карточка всё ещё обрывается — это отдельный тикет
(T015-residual / T0xx), не правка этого T015. Скоуп такого тикета:

1. Снять grim + измерить реальную высоту карточки vs высоту сайдбара.
2. Если не дотягивает — копать parent layout (вероятно `elevated_card`
   pattern перезаписывает size‑цепочку, либо контейнер‑родитель не
   имеет `h_full`).
3. Фикс — минимальный по строке (вероятно: добавить `.h_full()` на
   `div()`‑holder в `view.rs:126` где `sidebar::render(...)` сидит как child).

### 4.2 Drop‑zone как самостоятельная фича

В этой сессии (2026-08-09) рассматривался вариант «под карточкой
Places сделать drop‑target для файлов/папок», но после sync‑gap
выяснилось, что задача для этого пункта уже не существует (карточка
тянется на всю высоту). Drop‑zone остаётся legitimate idea, но это:

- **Не в скоупе T015.** В start‑of‑session я ошибочно завёл вопрос
  пользователю как «выбор дизайна для остаточного пункта», но этот
  пункт уже закрыт — отсюда pivot и решение «6/6, sync only».
- Требует полноценного brainstorming + spec + plan (Type A UI feature).
  Достаточно объёма, чтобы быть отдельным тикетом, если вообще будет
  user‑запрос.

## §5. Files (текущее размещение; финальный move — после architect verdict)

- `docs/orchestration/tasks/active/T015-sidebar-redesign.md` — тикет.
- `docs/orchestration/tasks/report/T015-sidebar-redesign-report.md` —
  этот отчёт.
- Все правки в коде лежат в git history (commits `9403f57` + `18ab839`).
  Worktree чист.

## §6. Coordination

- T019 — closed, не конфликт.
- T022 — closed, не правил `sidebar.rs` (только `engine.rs` и `row.rs`),
  объявленная в T015 коллизия не состоялась.
- T010/T011 — active, `sidebar.rs` мимо их скоупа.
- T014 — render‑perf, кросс‑ссылок на T015 нет; forward‑зависимость
  тоже не возникает.

## §7. Рекомендация exec для архитектора

**ACCEPT** тикета. Sync only, правок в коде не было, evidence на 6/6
пунктов пин‑поинтом указывает на коммиты в git history.

После приёмки вердиктом — move:
- `docs/orchestration/tasks/active/T015-sidebar-redesign.md` →
  `docs/orchestration/tasks/done/T015-sidebar-redesign.md`
- `docs/orchestration/tasks/report/T015-sidebar-redesign-report.md` →
  `docs/orchestration/tasks/report-log/T015-sidebar-redesign-report.md`

Это **не моя работа** — делаешь ты.
