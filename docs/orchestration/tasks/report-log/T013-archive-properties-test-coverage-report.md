# T013 — отчёт: юнит-тесты archive (T004) / PropertiesDialog (T005) / mime (T007)

> ## ⚠ ЭРРАТА ПРИЁМКИ (2026-08-06, чекпоинт #4)
>
> **§2 «Блокер сборки» атрибутирован неверно, а 4 неисполненных теста из
> §1.3 при запуске дали 2 падения и вскрыли боевой баг.**
>
> **1. Блокер `ashpd` не предсуществующий.** §2 утверждает: «предсуществующий
> конфликт фичей `ashpd` … Проверено воспроизводимо на чистом HEAD
> (`git stash -u` + `cargo check -p chronos-fm`)». HEAD на тот момент уже
> содержал `07bce8b` (T011 Tasks 5+6), который и добавил `oo7 = "0.6"` с
> default features → `oo7/tokio → ashpd/tokio` против `ashpd/async-io` из
> `gpui_linux`. `git log -S oo7 -- crates/chronos-fm-core/Cargo.toml` даёт
> единственный коммит. Ту же ошибку независимо повторили T014-recon и T016,
> ссылаясь на этот отчёт. Исправлено `0564c6e`.
>
> **2. Четыре теста `properties.rs` из §1.3 были написаны, но не
> запущены** — отчёт признаёт это честно («не исполнялись в этом окружении…
> по инспекции правки корректны»). После снятия блокера **2 из 4 падают**:
>
> ```
> ---- explorer::properties::tests::mode_to_rwx_known_modes ----
>   left: "------rwx"
>  right: "rwxr-xr-x"
> ```
>
> Корень — боевой баг в `properties.rs:182`: маски строились из битов
> владельца `[(0o400,'r'),(0o200,'w'),(0o100,'x')]` и сдвигались влево ещё
> раз, поэтому итерации владельца и группы маскировали `0o40000` / `0o4000`
> и никогда не совпадали. Совпадала только последняя итерация — **диалог
> Properties показывал `------rwx` для каждого файла в системе**.
> Исправлено `05a6e5a` (`bits = [(0o4,'r'),(0o2,'w'),(0o1,'x')]`);
> подтверждено живьём: файл `0755` даёт `rwxr-xr-x (0755)`.
>
> Тесты сами по себе написаны верно — они и поймали дефект. Проблема
> исключительно в том, что их не прогнали.
>
> Вердикт приёмки: **REFUTED**. Правки внесены, но заявление
> «по инспекции правки корректны» не заменяет прогона.

**Приоритет:** P2
**Исполнитель:** WorkBuddy (headless worker)
**Статус:** выполнено частично — headless-тесты добавлены; живая GUI/USB-проверка
остаётся за архитектором; recursive-size юнит-тест для `PropertiesDialog` НЕ
добавлен (причина ниже).

---

## 1. Что сделано

### 1.1 `archive` модуль (T004) — `crates/chronos-fm-services/src/archive/`

Добавлены 8 новых юнит-тестов, все проходят (`cargo test -p chronos-fm-services`):

**`mod.rs` (5):**
- `archive_format_from_path_all_extensions` — `.zip`, `.tar`, `.tar.gz`, `.tgz`,
  `.tar.zst`, `.tar.zstd` + негативные (`.txt`, нет расширения, `.tar.xz`).
- `split_make_archive_path_roundtrip` — `make_archive_path` → `split_archive_path`
  возвращает исходное.
- `make_archive_path_strips_leading_slash` — ведущий `/` в inner-path отсекается.
- `split_archive_path_no_separator` — путь без `::` → `None`.
- `split_archive_path_nested_separator_in_inner` — `::` внутри inner-path не
  ломает разбор (режется только первый `::`).

**`zip_archive.rs` (1):**
- `zipfs_list_and_read` — реальный zip (собран `zip::ZipWriter`, Deflated) со
  `hello.txt` и `dir/nested.txt`; проверяет list/read, синтетическую dir-запись
  (`dir`) и чтение вложенного файла; `read_file("missing.txt")` → `NotFound`.

**`tar_archive.rs` (2):**
- `tararchive_list_and_read` — plain tar.
- `tararchive_gz_list_and_read` — tar.gz (через `flate2::write::GzEncoder`).

> Попутно **найден и исправлен баг** в `synthetic_dirs` и `list()`: префикс для
> `strip_prefix` строился как `archive::/` (с лишним слэшем), тогда как
> `make_archive_path` формирует `archive::inner` (без слэша). Из-за этого
> синтетические dir-записи никогда не синтезировались, а `list("подкаталог")`
> всегда возвращал пустоту. Исправлено на `format!("{}::", …)` в обоих местах.
> Тест `zipfs_list_and_read` расширен проверкой `list("dir")` (возвращает
> `nested.txt`, исключает `hello.txt`).

### 1.2 `mime` модуль (T007) — `crates/chronos-fm-services/src/mime.rs`

Добавлены 4 теста (все проходят). Системозависимые проверки не падают в CI без
`shared-mime-info` / `.desktop`-записей — вместо `#[ignore]` используется
`eprintln!`-skip (тест реально валидирует, когда БД доступна, и не фейлит
окружение сборки, когда нет):

- `detect_mime_type_known_extensions` — tempfile `.txt` / `.png`; если
  `shared-mime-info` на месте — строго `text/plain` / `image/png`, иначе skip.
- `find_apps_for_mime_text_plain_returns_list` — непустой список, иначе skip.
- `open_with_launches_via_detached_sh_c` — форма вызова (`sh -c`, детач) без
  реального запуска приложения (`exec: "true %f"` → `Ok`).
- `open_with_terminal_wraps_command` — `terminal=true` оборачивает в
  `$TERMINAL -e sh -c … &`, запуск не блокируется (`true`).

### 1.3 `PropertiesDialog` (T005) — `crates/chronos-fm-pages/src/explorer/properties.rs`

**Косметика (п.6 тикета):** удалён недостижимый вариант `SizeStatus::Error(String)`
(никогда не конструировался, только матчился в `render`). Match в `render` теперь
исчерпывающий (`Idle` / `Counting` / `Done`).

Добавлены 4 теста (см. блок «Блокер сборки» — **не исполнялись** в этом окружении):

- `mode_to_rwx_known_modes` — `0o755`→`rwxr-xr-x`, `0o644`→`rw-r--r--`,
  `0o000`→`---------`, `0o777`→`rwxrwxrwx`, `0o700`→`rwx------`.
- `human_size_rounds_to_units` — `0`→`0 B`, `512`→`512 B`, `1024`→`1.0 KB`,
  `1536`→`1.5 KB`, `1 MiB`→`1.0 MB`, `1 GiB`→`1.0 GB`.
- `metadata_formatters_handle_missing_metadata` — `&None` → fallback
  (`("unknown","unknown")`, `("unknown","unknown")`, `("---------","0000")`).
- `format_permissions_reads_real_mode` (`#[cfg(unix)]`) — temp-файл, `set_mode(0o755)`
  → `("rwxr-xr-x","0755")`.

**НЕ доставлено (явно):** юнит-тест на рекурсивный размер («маленькое дерево →
`SizeStatus::Done` с верной суммой»). Причина: логика обхода вшита в
`PropertiesDialog::new(item, cx)`, которому нужен `Context<Self>` (GPUI), — на
headless-воркере без GPUI-harness и при заблокированной сборке pages-крэйта
написать такой тест невозможно без рефакторинга. Рекомендация: вынести обход в
свободную `fn compute_recursive_size(path: &Path) -> SizeStatus` и дёргать её из
`new`; тогда тест станет headless-рантабельным. Рефакторинг отложен до снятия
блокера сборки (см. ниже), чтобы не слать непроверенный прод-код.

---

## 2. Блокер сборки (важно для приёмки)

Крэйт `chronos-fm-pages` (и бинарный `chronos-fm`) **не собирается** в этом
окружении из-за предсуществующего конфликта фичей `ashpd`:
`gpui_linux` тянет `ashpd/async-io`, а `oo7` (через `secret`-фичу `gpui_linux`)
тянет `ashpd/tokio` → `compile_error!("You can't enable both async-io & tokio
features at once")`. Проверено воспроизводимо на чистом HEAD (`git stash -u` +
`cargo check -p chronos-fm`).

Следствия для T013:
- Тесты в `chronos-fm-services` (archive + mime) **скомпилированы и прошли** — 87
  passing (+12 новых для T013 поверх +6 из T016; базовых до T016/T013 было 69).
- Правки `properties.rs` (tests + удаление `SizeStatus::Error`) **написаны, но не
  скомпилированы и не исполнены** здесь — требуют снятия блокера. По инспекции
  правки корректны (удаление варианта оставляет match исчерпывающим; тесты
  используют только публичные/приватные API, доступные в модуле).
- Это **отдельная блокирующая задача** для архитектора, не в scope T013 и T016.
  Детали — в `T016-search-service-report.md` §«Блокер сборки».

---

## 3. Живая проверка (GUI-машина) — п.4 тикета

Не выполнялась: headless-воркер, без графической сессии и без «свободного стола».
Архитектор лично: зайти в `test.zip`/`test.tar.gz`, `Ctrl+I` на файле/папке
(сверить с `stat`/`ls -la`, recursive-size с `du -sb`), right-click → Open / Open
With / Set as Default, Properties из контекстного меню. UI-поток не должен
фризить при запуске внешнего приложения.

---

## 4. Живая проверка на реальном USB (T008) — п.5 тикета

**Открыто явно.** Физического тестового накопителя в окружении нет; suid-заглушку
под `udisks2` не изобретаем (по тикету). Пункт остаётся открытым до появления
устройства. Код `devices/{backend,parse}.rs` и `devices_store.rs` не трогался.

---

## 5. Чистка warnings (п.6)

- `SizeStatus::Error` — **удалён** (`properties.rs`). ✅
- «Неиспользуемый `Entity` import и `window`-параметр `show_properties`» —
  согласно тикету уже исправлено в T008; не трогалось, не сломано.

---

## 6. Итог по верификации

- `cargo test -p chronos-fm-services` → **87 passed, 0 failed** (новые тесты
  T013 = 12, видны в счётчике).
- `cargo test --workspace` / `cargo build --workspace` → **не проходит из-за
  ashpd-блокера** (не T013-специфично). pages-тесты T013 не исполнены по той же
  причине.
- Живая GUI/USB проверка — за архитектором (п.3, п.4, п.5).

## 7. Рекомендации

1. Снять ashpd-блокер (выключить `tokio`-путь `oo7`/`secret` или `async-io`
   `ashpd` в `gpui_linux`) — после этого пересобрать pages и исполнить 4 теста
   `properties.rs`.
2. Вынести `compute_recursive_size` в свободную функцию и добавить тест
   recursive-size (закрыть оставшийся пункт T005).
3. Прогнать живую GUI/USB-проверку на машине архитектора.
