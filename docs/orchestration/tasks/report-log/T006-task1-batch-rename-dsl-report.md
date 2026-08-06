# T006 Task 1 — Pure batch-rename DSL: `batch_rename.rs` (REPORT)

**Тикет:** T006 — Batch rename (план `docs/superpowers/plans/2026-08-06-batch-rename.md`)
**Задача:** Task 1 — чистая DSL-логика, без UI
**Статус:** ⬜ inbox → на приёмку (архитектор). Принят → `report-log/`.
**Дата:** 2026-08-06

## Что сделано

### Файлы
- **NEW** `crates/chronos-fm-services/src/fs/batch_rename.rs` — весь модуль (логика + 10 тестов).
- **EDIT** `crates/chronos-fm-services/src/fs.rs` — добавлен `pub mod batch_rename;` (строка 9, рядом с `listing`/`ops`).

> Проводка сделана в `fs.rs` (файл-модуль), а не в `chronos_fm_services.rs`, потому что в этом
> репо `mod fs` — это файл `src/fs.rs`, объявляющий свои подмодули. План это предвидел:
> «(Adjust to the file's actual `mod fs { ... }` shape)».

### API модуля (всё `pub`, с док-комментариями — `missing_docs = warn`)
- `RenameStatus` — `Ok` | `ResolvedCollision` (результат резолва коллизии).
- `RenamePreview` — `old_name`, `new_name`, `status`.
- `PatternError` — `UnknownPlaceholder`, `UnclosedBrace`, `InvalidWidth`.
- `Template` — мини-DSL: `{name}`, `{ext}`, `{n}`, `{n:W}` (zero-padded), экранирование `{{`/`}}`;
  `parse` возвращает `Result` — диалог (Task 2) гасит Apply на `Err`.
- `split_stem_ext` — первая точка = stem, последний суффикс = ext (`archive.tar.gz` → `(archive, gz)`).
- `apply_find_replace` — find/replace по stem; пустой find = no-op.
- `build_preview(entries, template, find, replace, start)` — чистый превью в порядке выделения,
  с резолвом коллизий (внутри батча + на диске через `ops::would_conflict`).

### Отклонения от эскиза плана (обоснованные)
1. **`split_stem_ext`:** в плане был `rsplit_once`-эскиз, дающий `archive.tar.gz → (archive.tar, gz)`.
   Спек (`docs/superpowers/specs/2026-08-06-batch-rename-design.md` §3) решил: stem = всё до
   **первой** точки, ext = последний суффикс → `(archive, gz)`. Реализовано по спеке; тест
   `split_stem_ext_last_extension_rule` пинит семантику. Обработаны `.hidden`/`noext` (пустой ext).
2. **Резолв коллизий:** план-эскиз `unique_within` проверял только `taken`-сет батча и не видел
   файлы на диске (тест `build_preview_resolves_disk_collision` это ловил). Финальный
   `resolve_collision` проверяет **оба** скоупа: `taken` + `ops::would_conflict(parent.join(...))`,
   с bump-последовательностью `name (2).ext`, `name (3).ext`, … — как задумал план-`resolve_target`
   («checking both taken and would_conflict, like the plan's resolve_target sketch intended»).
3. **План-эскиз `resolve_target` (мёртвый код)** не включён в финальный модуль — его роль
   выполняет `resolve_collision` + `build_preview`. План прямо отметил `resolve_target` как
   «plan-level sketch»; включать не стал (dead code).

## Верификация (Task 1 Step 5 + гейты)
- `cargo test -p chronos-fm-services batch_rename::` → **10 passed; 0 failed** (EXIT=0).
- `cargo test -p chronos-fm-services` (полный) → **44 passed; 0 failed** (34 старых + 10 новых).
- `cargo clippy -p chronos-fm-services` → чисто в `batch_rename.rs` (0 предупреждений, EXIT=0).
- Код-ревью (code-reviewer-deepseek-flash): 1 actionable замечание — док-строка модуля «No I/O here»
  противоречила `would_conflict` (read-only stat-проверка). **Исправлено**: «No disk *mutation*
  here — renames live in `ops`; the only I/O is a read-only existence check…». Прочее (коллизии,
  порядок, no-op guard `candidate != entry.name`, отсутствие бесконечного цикла в `resolve_collision`)
  — подтверждено корректным.

## Тесты (10, по плану)
`parse_accepts_all_placeholders_and_literals`, `parse_handles_escaped_braces`,
`parse_rejects_unknown_placeholder`, `parse_rejects_unclosed_and_stray_braces`,
`parse_rejects_bad_width`, `split_stem_ext_last_extension_rule`,
`apply_find_replace_basic`, `build_preview_numbers_from_custom_start_with_padding`,
`build_preview_resolves_batch_internal_collision`, `build_preview_resolves_disk_collision`.

## Оговорки / на заметку Task 2
- `build_preview` делает stat-вызов `would_conflict` на каждую строку при каждом keystroke —
  на масштабе диалога это ок, но live-preview в Task 2 стоит дебаунсить.
- Пустой паттерн `""` парсится в пустой `Template` → рендер пустых имён. Диалог (Task 2) должен
  блокировать Apply на пустых/неизменённых именах (план это уже требует: «Apply must be disabled…
  or no row's `new_name != old_name`»).
- `{n:0}` — не падает, рендерит без паддинга (безопасно).

## Готовность
Task 1 закрыт. Task 2 (диалог `chronos-fm-pages/src/explorer/batch_rename.rs`) разблокирован —
API (`build_preview`/`Template`/`RenamePreview`) подтверждён код-ревью как удобный для UI.
