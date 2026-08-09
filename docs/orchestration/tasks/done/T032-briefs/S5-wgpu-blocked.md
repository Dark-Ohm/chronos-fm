> **SUPERSEDED 2026-08-09** → `S5-wgpu-vendor.md` (разблокировано явным запросом архитектора, выполнено, принято). Оставлено как исторический след решения блокировка→разблокировка.

# Agent S5 — wgpu — ЗАБЛОКИРОВАН

**Статус:** не назначать  
**Тикет T032:** шаг 5 только после **отдельного явного запроса** пользователя/архитектора.

## Почему

- Ядро рендера; silent visual regression.
- Приёмка = grim + полные baselines, отдельный заход.
- I4 — только inventory (`I4-wgpu-inventory.md`).

## Если этот файл всё же выдали агенту

1. **Стоп.** Не менять deps.
2. Прочитать `T032-notes/I4-wgpu.md` если есть.
3. Вернуть сообщение: «S5 blocked; need explicit go-ahead».
4. Zero commits.

## Когда разблокируют (будущий бриф, не сейчас)

- I4 cost accepted
- grim baseline library agreed
- explicit user message to start wgpu vendoring
