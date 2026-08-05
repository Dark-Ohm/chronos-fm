# T006 — Batch rename (паттерн на множество файлов)

**Приоритет:** P3 — b1 уже даёт single inline rename как временную
замену, это не блокер daily-driver, но нужно для полного Dolphin-паритета.
**Статус:** ТРЕКИНГ-ТИКЕТ, не начинать код. Нет ни design spec, ни
implementation plan — backlog-пункт #3 из `docs/superpowers/specs/2026
-08-06-removable-media-mount.md` (низ файла), только приоритет одной
строкой, не проработано.

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
