# T024 — Отчёт: pattern-port remainder decisions (ratified)

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-09)
>
> Four decisions ratified (see DECISIONS.log § T024). rejected/ ported;
> notes/ and MIGRATION.md not ported; skills on-demand. Ticket → done/.

**Дата:** 2026-08-09
**Исполнитель:** Buffy (executor)
**Статус:** ✅ Все 4 решения приняты архитектором и записаны в `DECISIONS.log`

## Решения

| # | Пункт | Решение | Действие |
|---|-------|---------|----------|
| 1 | `rejected/` | **Портировать** | Создана `docs/orchestration/tasks/rejected/`, T011 REFUTED перенесён как `T011-s3-tab-live-REFUTED.md` |
| 2 | `notes/` | **Не портировать** | Покрыто `.workbuddy/memory/` — дубль не нужен |
| 3 | `MIGRATION.md` | **Не портировать** | FM сразу на per-task учёте, артефакт нерелевантен |
| 4 | Скиллы ChronOS (11) | **По потребности** | Портировать при появлении задачи; первые кандидаты: `tokio-coop-budget`, `rtl-text-rendering` |

## Что сделано

1. **`DECISIONS.log`** — запись T024 добавлена (4 решения + контекст + верификация)
2. **`docs/orchestration/tasks/rejected/`** — директория создана
3. **`T011-s3-tab-live-REFUTED.md`** — первая запись в `rejected/` (копия из `report-log/`, эррата сохранена в исходном)
4. **`MIGRATION.md` / `notes/`** — не созданы (решение «не портировать»)
5. **Скиллы** — правило «по потребности» записано в DECISIONS.log; действия не требуются

## Коммит

```
docs: T024 — pattern-port remainder decisions (ratified)

- DECISIONS.log: T024 entry (4 decisions)
- rejected/: create directory, T011 REFUTED as first inhabitant
- notes/, MIGRATION.md: NOT ported (decisions 2, 3)
- Per-project skills: on-demand rule (decision 4)
```

## Верификация

- Все 4 решения записаны в `DECISIONS.log` с контекстом и обоснованием ✅
- `rejected/` создан, первый REFUTED отчёт перенесён ✅
- Код не требуется (decision-only ticket) ✅
