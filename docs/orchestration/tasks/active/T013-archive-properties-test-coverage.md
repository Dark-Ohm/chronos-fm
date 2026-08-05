# T013 — юнит-тесты + живая проверка archive browsing (T004) и Properties dialog (T005)

**Приоритет:** P2 — оба фичи приняты с оговоркой, работают на вид, но
без тест-покрытия и без живого клик-подтверждения.
**Роль:** дожим, самодостаточное задание.
**Источник:** `docs/orchestration/tasks/report-log/T004-archive-browsing-report.md`
§6, `docs/orchestration/tasks/report-log/T005-properties-dialog-report.md` §6.

## Контекст

T004/T005 приняты с оговоркой: build/test чисто, но ноль новых юнит-
тестов на ~250+300 строк новой логики, и живой клик (войти в архив,
Ctrl+I открыть диалог) не подтверждён — архитектор не рисковал
синтетическим вводом на занятом рабочем столе.

## Что нужно

1. **Тесты `archive` модуля** (`crates/chronos-fm-services/src/archive/`):
   - `split_archive_path`/`make_archive_path` — чистые функции без I/O,
     покрыть round-trip (`make` затем `split` возвращает исходное) +
     edge cases (путь без `::`, вложенный `::` внутри inner-path).
   - `ArchiveFormat::from_path` — все расширения (`.zip`, `.tar`,
     `.tar.gz`, `.tgz`, `.tar.zst`, `.tar.zstd`) + отрицательный случай.
   - `ZipFs`/`TarArchive` — на реальных мини-архивах, собранных прямо в
     тесте (`tempfile` уже есть в dev-deps, `zip`/`tar` крейты уже
     прямые зависимости) — list/read хотя бы одного файла внутри.
2. **Тесты `PropertiesDialog`** (`crates/chronos-fm-pages/src/explorer/properties.rs`):
   - Метаданные на реальном временном файле/папке (`tempfile`) —
     owner/permissions/size для файла без рекурсии.
   - Рекурсивный размер — маленькое дерево (`tempfile::TempDir` +
     несколько файлов) — `SizeStatus` доходит до `Done` с верной суммой.
3. **Живая проверка (GUI-машина, без посторонней активности на столе):**
   - Клик в `test.zip`/`test.tar.gz` (или создать заново) — реально
     заходит внутрь, показывает содержимое, `read_preview` показывает
     содержимое файла из архива.
   - `Ctrl+I` на файле → диалог открывается, метаданные корректны
     (сверить с `stat`/`ls -la` вручную).
   - `Ctrl+I` на папке → рекурсивный размер действительно считается и
     сходится с `du -sb`.
4. Заодно почистить warnings из отчётов: неиспользуемый `Entity` import,
   `window` → `_window` в `show_properties`, недостижимый
   `SizeStatus::Error` (либо использовать, либо убрать вариант).

## Зона файлов

`crates/chronos-fm-services/src/archive/*.rs` (тесты),
`crates/chronos-fm-pages/src/explorer/properties.rs` (тесты + косметика),
`crates/chronos-fm-pages/src/explorer/navigation.rs` (косметика).

## Верификация

`cargo build --workspace` + `cargo test --workspace` чисто, новые тесты
видны в счётчике (не просто «178+» как раньше — конкретное новое число
в отчёте), живой grim обоих сценариев (архив открыт, Properties диалог
открыт с реальными метаданными).

## Отчёт

`docs/orchestration/tasks/report/T013-archive-properties-test-coverage-report.md`
(inbox). Приёмка — архитектор лично. Принят → `report-log/`, тикет →
`done/`. Отклонён → `rejected/`.
