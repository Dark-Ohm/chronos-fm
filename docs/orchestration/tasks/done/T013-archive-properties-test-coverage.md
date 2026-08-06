# T013 — юнит-тесты + живая проверка archive browsing (T004),
# Properties dialog (T005), Open With / service menu (T007)
# и Devices-панель клики/eject (T008)

**Приоритет:** P2 — все четыре фичи приняты с оговоркой, работают на
вид, но без полного тест-покрытия и/или без живого подтверждения.
**Роль:** дожим, самодостаточное задание.
**Источник:** `docs/orchestration/tasks/report-log/T004-archive-browsing-report.md`
§6, `docs/orchestration/tasks/report-log/T005-properties-dialog-report.md`
§6, `docs/orchestration/tasks/report-log/T007-open-with-report.md` §2–3,
`docs/orchestration/tasks/report-log/T008-devices-panel-click-wiring-report.md`
§«Верификация»/«Известные ограничения» (все — приняты
accepted-with-concerns).

## Контекст

- T004/T005 приняты с оговоркой: build/test чисто, но ноль новых юнит-
  тестов на ~250+300 строк новой логики, и живой клик (войти в архив,
  Ctrl+I открыть диалог) не подтверждён — архитектор не рисковал
  синтетическим вводом на занятом рабочем столе.
- T007 (Open With / service menu, «Set as Default») принят с оговоркой:
  7 юнит-тестов уже есть для чистой логики (`expand_exec`,
  `shell_escape`, форма вызова `set_default_app`), но системозависимые
  функции (`detect_mime_type`, `find_apps_for_mime`, `open_with`) не
  покрыты, и живой клик (правая кнопка → Open / Open With / Properties /
  Set as Default) не подтверждён по той же headless-причине.
- T008 (Devices-панель: клики mount/unmount + фикс `eject`) принят с
  оговоркой: mock-покрытие (`FakeBackend`-стиль) есть на резолв
  Drive-пути и на цепочку клика (`mount_and_navigate` → `navigate_to_path`),
  но живой прогон с **реальной флешкой** осознанно пропущен — не headless-
  ограничение, а отсутствие тестового устройства (внутренние размеченные
  разделы на `sdb` трогать нельзя без явного запроса пользователя).

## Что нужно

### 1. Тесты `archive` модуля (T004)

`crates/chronos-fm-services/src/archive/`:
- `split_archive_path`/`make_archive_path` — чистые функции без I/O,
  покрыть round-trip (`make` затем `split` возвращает исходное) +
  edge cases (путь без `::`, вложенный `::` внутри inner-path).
- `ArchiveFormat::from_path` — все расширения (`.zip`, `.tar`,
  `.tar.gz`, `.tgz`, `.tar.zst`, `.tar.zstd`) + отрицательный случай.
- `ZipFs`/`TarArchive` — на реальных мини-архивах, собранных прямо в
  тесте (`tempfile` уже есть в dev-deps, `zip`/`tar` крейты уже прямые
  зависимости) — list/read хотя бы одного файла внутри.

### 2. Тесты `PropertiesDialog` (T005)

`crates/chronos-fm-pages/src/explorer/properties.rs`:
- Метаданные на реальном временном файле/папке (`tempfile`) —
  owner/permissions/size для файла без рекурсии.
- Рекурсивный размер — маленькое дерево (`tempfile::TempDir` +
  несколько файлов) — `SizeStatus` доходит до `Done` с верной суммой.

### 3. Тесты `mime` модуля (T007)

`crates/chronos-fm-services/src/mime.rs`:
- `detect_mime_type` — на реальном временном файле (`tempfile`) с
  известным расширением (`.txt`, `.png` и т.п.) — проверить, что
  возвращённый MIME-тип совпадает с ожидаемым (система должна иметь
  `shared-mime-info`; если недоступно — `#[ignore]` с комментарием, не
  падать в CI без него).
- `find_apps_for_mime` — зарегистрировать/использовать заведомо
  существующий MIME-тип (`text/plain`) и проверить непустой список; либо
  замокать реестр `.desktop`, если `freedesktop-desktop-entry` это
  позволяет без реальной ФС.
- `open_with` — тест на форму вызова (аргументы `sh -c`, детач через
  `&`), без реального запуска процесса — по аналогии с уже написанным
  тестом на `set_default_app`.

### 4. Живая проверка (GUI-машина, без посторонней активности на столе)

- Клик в `test.zip`/`test.tar.gz` (или создать заново) — реально
  заходит внутрь, показывает содержимое, `read_preview` показывает
  содержимое файла из архива.
- `Ctrl+I` на файле → диалог открывается, метаданные корректны (сверить
  с `stat`/`ls -la` вручную).
- `Ctrl+I` на папке → рекурсивный размер действительно считается и
  сходится с `du -sb`.
- Правая кнопка на файле → «Open» реально запускает системный дефолтный
  обработчик (не блокирует UI-поток — окно ChronOS остаётся отзывчивым
  пока запущенное приложение открыто).
- Правая кнопка → «Open With» → список приложений не пуст для файла с
  известным MIME (напр. `.txt`) → клик по приложению реально его
  запускает, детачнуто (тот же чек на неблокирующий UI).
- «Set as Default» на строке приложения → после клика повторный «Open»
  на файле того же MIME открывает именно это приложение (проверить
  `~/.config/mimeapps.list` или поведением).
- «Properties» из контекстного меню — открывает тот же диалог, что и
  Ctrl+I.

### 5. Живая проверка на реальном USB-накопителе (T008)

**Условно** — требует физического тестового устройства, а не headless-
воркэраунда. Если флешки нет к моменту дожима — оставить пункт открытым
явно в отчёте (не закрывать T013 молча по этому пункту), не изобретать
suid-заглушку под реальный udisks2.

- Вставить флешку → устройство появляется в Devices-панели (hotplug,
  `InterfacesAdded`).
- Клик по немонтированному тому → монтируется, панель переходит внутрь
  (`lsblk` подтверждает точку монтирования).
- Клик «unmount» → том отмонтирован, строка обновляется без ручного
  рефреша (`lsblk` подтверждает).
- Извлечь физически (или через `Drive.Eject`, если UI когда-нибудь
  получит кнопку) → `eject_target` резолвит именно Drive-путь, не
  Block/Filesystem — сверить логом/`lsblk` до и после.
- Быстрый двойной клик по немонтированному тому — задокументировать
  фактическое поведение (известное ограничение v1: возможен повторный
  параллельный mount) — не чинить в этом тикете, фикс отдельной задачей.

### 6. Заодно почистить warnings

- Неиспользуемый `Entity` import и `window`-параметр `show_properties`
  — **уже исправлено в T008** (`navigation.rs` — `show_properties`
  больше не берёт `window`), проверять не нужно, только не сломать
  заново.
- Недостижимый `SizeStatus::Error` (либо использовать, либо убрать
  вариант) — единственный, который остаётся открытым.

## Зона файлов

`crates/chronos-fm-services/src/archive/*.rs` (тесты),
`crates/chronos-fm-services/src/mime.rs` (тесты),
`crates/chronos-fm-pages/src/explorer/properties.rs` (тесты + косметика),
`crates/chronos-fm-pages/src/explorer/navigation.rs` (косметика),
`crates/chronos-fm-pages/src/explorer/context_menu.rs` (живая проверка,
без изменений кода, если тесты не потребуют иного),
`crates/chronos-fm-pages/src/explorer/view/sidebar.rs`,
`crates/chronos-fm-services/src/devices/{backend,parse}.rs`,
`crates/chronos-fm-ui/src/devices_store.rs` (живая проверка T008 на
реальной флешке, без изменений кода, если проверка не найдёт баг).

## Верификация

`cargo build --workspace` + `cargo test --workspace` чисто, новые тесты
видны в счётчике (конкретное новое число в отчёте, не просто «178+»),
живой grim всех сценариев (архив открыт, Properties диалог открыт с
реальными метаданными, Open/Open With/Set as Default отработали без
фриза UI, Devices-панель отмонтировала/смонтировала реальную флешку —
или пункт явно оставлен открытым при отсутствии устройства).

## Отчёт

`docs/orchestration/tasks/report/T013-archive-properties-test-coverage-report.md`
(inbox). Приёмка — архитектор лично. Принят → `report-log/`, тикет →
`done/`. Отклонён → `rejected/`.
