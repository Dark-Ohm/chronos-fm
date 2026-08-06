# T006 — Batch rename (паттерн на множество файлов)

**Приоритет:** P3 — b1 уже даёт single inline rename как временную
замену, это не блокер daily-driver, но нужно для полного Dolphin-паритета.
**Статус:** реализация закрыта (2026-08-06), на приёмку архитектора.
Прогресс процесса:

- ✅ **Brainstorm (2026-08-06)** — решения зафиксированы: свой мини-DSL
  (Dolphin-стиль `{name}`/`{ext}`/`{n}`/`{n:W}`), живой preview
  «было → станет» ДО применения (критично, тикет), коллизии
  авто-резолвятся (`ops::unique_name`/`would_conflict`) и видны в preview.
- ✅ **Design spec** → `docs/superpowers/specs/2026-08-06-batch-rename-design.md`.
- ✅ **Implementation plan** → `docs/superpowers/plans/2026-08-06-batch-rename.md`
  (написан против уже сданного API b1 — блокер снят).
- ✅ **b1–b4 (context-menu chain)** разблокированы и реализованы слитым
  способом (см. `docs/orchestration/tasks/report-log/T006-unblock-b1-b4-chain-report.md`).
- ✅ **Task 1** (чистая DSL `services/fs/batch_rename.rs`) —
  `docs/orchestration/tasks/report-log/T006-task1-batch-rename-dsl-report.md`.
- ✅ **Tasks 2–5** (диалог/проводка/overlay/entry point) —
  `docs/orchestration/tasks/report-log/T006-tasks2-5-batch-rename-dialog-report.md`.
- ✅ **Task 6** (verification pass) — прогнан внутри отчёта Tasks 2–5
  (`cargo test --workspace` 218/218, clippy чисто, build чисто).

Все отчёты приняты архитектором с оговоркой (live-смоук не проводился —
headless-окружение); детали — в соответствующих `report-log/`-файлах.

## Что нужно ПЕРЕД кодом

1. `brainstorming` skill: множественный выбор → «Rename» (или отдельный
   пункт «Batch Rename») → диалог с паттерном (`{name}_{n}.{ext}`,
   find/replace, нумерация с заданным стартом/шириной). Решить: живой
   preview результата ДО применения (список «было → станет»,
   критично — массовое переименование без предпросмотра опасно), какой
   синтаксис паттерна (свой мини-DSL vs готовый крейт).
2. Design spec → `docs/superpowers/specs/`.
3. Implementation plan → `docs/superpowers/plans/`.
4. Только после этого — T-тикет на код.

## Зависимости

**Жёстко зависит от b1** (`docs/agents/active/b1-clipboard-rename-foundation.md`)
— переиспользует rename-инфраструктуру (`page.begin_rename`/
`commit_rename`/`cancel_rename`), не изобретать заново. Не заводить
implementation plan для этого тикета, пока b1 не в `report-log/`/`done/`.

## Не раздавать как есть

Этот файл — не инструкция для исполнителя-кодера, сначала брейншторм —
и сначала дождаться b1.
