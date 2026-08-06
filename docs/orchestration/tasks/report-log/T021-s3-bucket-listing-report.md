# T021 — Отчёт: после connect список бакетов не запрашивается

> ## ✅ ВЕРДИКТ ПРИЁМКИ: VERIFIED (2026-08-07)
>
> Живой прогон против локального RustFS проведён — **работает**.
>
> - После connect, **без всякой навигации**: `s3: > rustfs@`, `2 items`,
>   `chronos-empty` и `chronos-smoke`.
> - Заход в бакет: `data/`, `notes/`, `readme.txt` **31 B** — байт в байт
>   то, что положил `seed_s3.py`. Такие данные могут прийти только с
>   сервера.
> - В логе приложения **ноль** строк `entity has no current window`.
> - `cargo test -p chronos-fm-pages` → **87 passed, 0 failed**.
>
> **Два критерия из брифа оказались непригодны — это ошибка брифа, не
> исполнителя.** RustFS не ведёт access-лог (5 строк за весь запуск, все
> стартовые), а модуль `chronos_fm_services::s3` не пишет ни одной
> debug-строки, поэтому «запросы в логе сервера» и «строки модуля s3»
> проверить нечем. Доказательством служат сами данные: имена бакетов и
> размер объекта совпадают с засеянными. Для будущих S3-приёмок критерий
> формулировать так.
>
> **Мелочь на будущее:** префиксы-каталоги показывают `Modified
> 1970/01/01` — у S3 common prefixes нет даты, стоит рисовать прочерк.

**Тикет:** T021 · **Приоритет:** P1 · **Статус:** фикс готов, ждёт живой приёмки
**Корень:** установлен в брифе (B.4 `loaded = false` vs B.1 no-op `reload()` +
звено S3Page → панель, сломавшееся в `32cb1ac`).

## 0. Контекст

Первый заход (`32cb1ac`, wip) заменил `pane.loaded = false` на
`pane.downgrade().update_in(...)`, что молча падало с «entity has no current
window»: `WeakEntity::update_in` резолвит окно через `current_window_by_entity`,
которая заполняется только из `defer_in`/`observe_in`/`subscribe_in`, а панель
создана через `cx.new`. Сервер RustFS не получал ни одного запроса.

## 1. Что сделано

### 1.1 Фикс звена S3Page → панель (`s3.rs`)

В колбэке `Ok(client)` асинхронного connect-флоу `start_connect`:

- Окно **больше не теряется**: внешний колбэк принимает `window`
  (`|this, window, cx|`) вместо `_window` и прокидывает его вниз.
- Вместо `pane.downgrade().update_in(...).log_err()` — извлечённый метод
  `S3Page::wire_pane(window, &pane, provider, s3_root, cx)`, который делает
  обычный `pane.update(cx, |pane, cx| { set_provider; cwd = s3_root;
  reload_provider(window, cx); })` — `Entity::update` не требует резолва окна
  по entity и работает из контекста S3Page (механизм ровно как в брифе;
  `WeakEntity::update_in` в любом виде не возвращался).
- `reload_provider` вызывается **синхронно в колбэке**, а не через render-путь:
  `loaded = false` для provider-панелей — no-op (B.1), поэтому отложенный
  листинг показывал бы пустой список навсегда.
- Ошибки идут через `tracing::error!` с внятным текстом: падение
  `this.update_in` (страница освобождена до прихода клиента) и провал
  `S3Client::from_profile` (реальная сетевая ошибка, ранее невидимая в логах).
  `.log_err()` удалён вместе с импортом `LogErr` — он писал в `log::Level::Error`,
  выходящий через мост на WARN с таргетом `chronos_fm_core::telemetry`, и греп
  по `ERROR` его не находил.

### 1.2 Что сохранено из `32cb1ac` (переделывать не требовалось)

- `ensure_loaded(window, cx)` направляет provider-панели в `reload_provider`
  вместо молчаливой пометки loaded (navigation.rs + view.rs).
- Инлайн-баннер ошибки листинга в `view/listing.rs`.
- `CountingProvider` + три теста в `explorer/tests.rs` (стали `pub(crate)` с
  док-комментариями, чтобы S3-тест переиспользовал тот же фейк).

### 1.3 Новый тест — через S3Page, а не через панель

`s3::tests::connect_callback_wires_pane_and_kicks_off_bucket_listing`
(`#[gpui::test]`):

- строит `S3Page` (конфиг с профилем `rustfs`, как в live-`config.toml`) в
  тестовом окне;
- создаёт встроенную панель ровно как `start_connect` (`cx.new`, `loaded = true`);
- выполняет **тот же колбэк**, что connect-флоу на `Ok(client)` —
  `wire_pane` — с фейковым `CountingProvider`;
- ассерты: `list_dir` вызван **ровно 1 раз**, `pane.cwd == "s3://rustfs@"`,
  бакеты применены (`entries.len() == 2`), состояние `Browsing`.

Именно это звено было сломано: три теста `32cb1ac` дёргали панель напрямую и
дефект не ловили. Живой след для проверки утверждения «запрос отправляется» —
`CountingProvider::list_dir` считается атомарно.

### 1.4 Находка: recursion limit у `#[gpui::test]`

Новый тест упирался в `recursion limit reached while expanding #[test]`.
Биссекция показала: дело **не в глубине тела теста**, а в `use super::*`
внутри `#[cfg(test)] mod tests`. Родительская `s3.rs` имеет `use gpui::*`,
а `gpui` (feature `test-support`) ре-экспортирует attribute-macro `test` —
глоб-импорт приносит `test` (proc-macro) в область видимости тест-модуля, и
сгенерированный внутри `#[gpui::test]` builtin-`#[test]` начинает резолвиться
в proc-macro → бесконечная самовыдача → recursion limit.

**Практическое правило для будущих тест-модулей в этом крейте:** не
использовать `use super::*`, импортировать явно (как `explorer/tests.rs`).
Запись в отчёте T010 («требует `#![recursion_limit = "256"]`») описывала тот же
симптом для git-render-теста; вероятный триггер там тот же самый. Атрибут
`#![recursion_limit]` в корень крейта **не добавлялся** — после явных импортов
не нужен.

## 2. Верификация

- `cargo check -p chronos-fm-pages` → чисто (0 ошибок; 3 pre-existing
  missing-docs в `s3.rs` — S3Page/`new`/`set_config`, не из этого захода).
- `cargo test -p chronos-fm-pages` → **87 passed, 0 failed** (все
  существующие + новый S3Page-тест + три теста `32cb1ac`).
- `cargo clippy -p chronos-fm-pages --all-targets` → exit 0; по
  `s3.rs`/`explorer/tests.rs` новых замечаний нет (`CountingProvider` получил
  доки под `missing_docs = "warn"`).

## 3. Что НЕ сделано (осталось архитектору)

- **Живой прогон** против локального RustFS: три критерия брифа
  (скриншот + запросы в `podman logs chronos-rustfs` + строки
  `chronos_fm_services::s3` в логе без «entity has no current window»).
- Сборка `cargo build --release -p chronos-fm` (вне `default-members`) и
  сверка mtime бинаря перед прогоном — по разделу «Сборка» брифа.
- Решение по коммиту/приёмке (изменения в рабочем дереве, см. §5).

## 4. Файлы

- `crates/chronos-fm-pages/src/s3.rs` — фикс колбэка, `wire_pane`, ошибки через
  `tracing::error!`, тест-модуль с S3Page-кросс-тестом.
- `crates/chronos-fm-pages/src/explorer/tests.rs` — `CountingProvider`
  `pub(crate)` + доки (переиспользование из s3-теста).

Нетто-изменений в `chronos_fm_pages.rs` нет (эксперимент с
`#![recursion_limit]` откачен).

## 5. Коммиты

Закоммичен только `32cb1ac` (wip). Текущий фикс **не закоммичен**:
`git diff` — `s3.rs` (+203/−24) и `explorer/tests.rs` (+13/−1).

## 6. Чек-лист живого прогона (из брифа)

1. `podman run -d --name chronos-rustfs -p 9000:9000 -p 9001:9001 \
   -v chronos-rustfs-data:/data -v chronos-rustfs-logs:/logs \
   docker.io/rustfs/rustfs:latest`; ключи `rustfsadmin`/`rustfsadmin`.
2. `seed_s3.py` (boto3, `force_path_style`): два бакета, объект в корне,
   два общих префикса.
3. `RUST_LOG=chronos_fm_services=debug cargo run -p chronos-fm` →
   S3-таб → Connect.
4. Критерий: бакеты видны без навигации; заход в бакет показывает `data/`,
   `notes/`, `readme.txt`.
5. Три подтверждения: `grim`; `podman logs chronos-rustfs` содержит запросы;
   в логе приложения есть строки `chronos_fm_services::s3` и нет ни одной
   `entity has no current window` (грепать по `WARN`, не только `ERROR`).
