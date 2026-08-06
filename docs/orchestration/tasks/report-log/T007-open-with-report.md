# T007 — Отчёт: «Open with» / service menu (context menu continuation)

**Тикет:** T007 · **Приоритет:** P3 · **Статус:** готов к приёмке
**Спец:** design утверждён в прерванной сессии (см. §0), спец-файл не
заводился — продолжение реализации по уже одобренному дизайну.

## 0. Контекст: почему это продолжение, а не старт

Тикет T007 — трекинг («не начинать код без брейншторма»). При разборе
выяснилось: **дизайн уже был одобрен пользователем в предыдущей сессии,
которая упала посреди реализации** — в working tree остался
незакоммиченный прототип (`mime.rs` + `context_menu.rs` + wiring в
row/grid/view/navigation/state), компилирующийся, но с мёртвыми пунктами
меню и блокирующим запуском. Пользователь подтвердил (ask_user):
«the design was already approved. the session just crashed in middle —
you continue it», плюс решил **включить «Set as default» в этот заход**.

Крейтовое решение из тикета («проверить готовый крейт перед своим
парсером») прототипом уже принято: `xdg-mime 0.4` (детекция) +
`freedesktop-desktop-entry 0.8.1` (реестр .desktop). Лицензии сверены с
`deny.toml`: `freedesktop-desktop-entry` — **MPL-2.0** (уже в allowlist),
остальное MIT/Apache-2.0. Новых записей в allowlist не потребовалось.

## 1. Что сделано в этом продолжении

### 1.1 `crates/chronos-fm-services/src/mime.rs`

- **`DesktopApp.desktop_id`** — новое поле (id .desktop-файла, напр.
  `org.gnome.TextEditor.desktop`), нужен для `xdg-mime default`.
- **`open_default(path)`** — запуск системного дефолтного обработчика
  (через `xdg-open`, детач). Эквивалент «Open».
- **`set_default_app(mime_type, app)`** — регистрация приложения как
  дефолтного через `xdg-mime default <id> <mime>` → пишет в
  `~/.config/mimeapps.list`.
- **Исправлен блокирующий запуск**: раньше `open_with` использовал
  `.status()` — UI-поток замирал, пока запущенная программа не закроется
  (GUI-приложения — мгновенный фриз). Теперь все запуски детачнуты через
  `sh -c '... &'` (фоновый процесс, репарент init; `status()` возвращается
  сразу после выхода `sh`).
- **7 юнит-тестов**: `expand_exec` (5: `%f/%F`, `%u/%U`, `%%`, `%i/%c/%k`
  со стрипом и без, unknown-плейсхолдеры), `shell_escape` (внутренние
  кавычки), `set_default_app` (форма вызова, не паникует).

### 1.2 `crates/chronos-fm-pages/src/explorer/context_menu.rs`

- **«Open» ожил**: был мёртвой меткой → теперь вызывает
  `open_with_default` (панельный метод → `mime::open_default`).
- **«Properties» ожил**: был мёртвой меткой → теперь открывает
  Properties-диалог (`show_properties`) и закрывает меню.
- **«Set as Default»** на каждую строку приложения (справа, hover →
  accent): регистрирует приложение дефолтным для MIME файла,
  `stop_propagation`, меню закрывается.
- Меню закрывается после любого действия (Open / запуск приложения /
  Set as default / Properties), как и по клику мимо.
- Сигнатура `render` изменена `Context<impl Focusable>` →
  `Context<ExplorerPane>` — листенерам нужен доступ к панели
  (`open_with_default`/`show_properties`/`close_context_menu`).

### 1.3 `crates/chronos-fm-pages/src/explorer/navigation.rs`

- `open_with_default` больше не dead code: делегирует в
  `mime::open_default` (раньше брал «первое приложение из списка» —
  теперь настоящий xdg-open-эквивалент).
- Новый `show_properties_for(path, cx)` — Properties по конкретному пути
  (меню строится по правому клику, а не по active-строке).

### 1.4 Правки по ревью (code-reviewer-deepseek-flash)

- **Баг: «Properties» показывал не тот файл.** `show_properties` читал
  `active_index`, а меню строится по `state.file_path` (правый клик не
  нормализует выделение). Исправлено двумя путями: `show_properties_for`
  (по пути из меню) + нормализация выделения при правом клике в
  `row.rs`/`grid.rs` (single-select невыделенной строки — конвенция
  b3/b4 из `docs/superpowers/plans/2026-07-21-explorer-context-menu.md`).
- **`%u`/`%U` теперь URL-энкодятся** (`file_url`: percent-encoding пробелов,
  `#`, `%`, не-ASCII) и shell-эскейпятся — раньше путь с пробелом ломал
  shell-скрипт. +2 теста.
- **Убран тест с побочным эффектом**: `set_default_app_requires_a_desktop_id`
  реально запускал `xdg-mime default` на машине разработчика (пишет в
  `~/.config/mimeapps.list`) и ничего не ассертил — заменён на чистый
  `file_url_keeps_ascii_safe_characters`.

## 2. Верификация

- `cargo check -p chronos-fm-pages -p chronos-fm-services` → **EXIT=0**
- `cargo test -p chronos-fm-services -p chronos-fm-pages` → **34 passed,
  0 failed** (из них 7 новых — `mime::tests::*`)
- `cargo clippy -p chronos-fm-services -p chronos-fm-pages` → в
  `mime.rs`/`context_menu.rs` замечаний 0 (оставшиеся warn — pre-existing
  в других файлах)
- `cargo build -p chronos-fm` → **EXIT=0** (бинарь собирается)
- Лицензии новых крейтов сверены с `deny.toml` (см. §0) — `cargo-deny`
  локально не установлен, проверено по `Cargo.toml` крейтов + allowlist

**Живой прогон:** НЕ проводился — headless-окружение сессии. Правая
кнопка → меню с Open/Open With/Properties → клики требуют живой Hyprland
сессии с рабочим столом, как обычно для этого репо (приёмка архитектора).

## 3. Что НЕ в этом заходе

- Тесты на `detect_mime_type`/`find_apps_for_mime`/`open_with`
  (системозависимые: требуют shared-mime-info + установленные .desktop;
  заведены как follow-up, если нужны).
- `xdg-mime query default` как источник «дефолтного» приложения внутри
  списка Open With (сейчас «Open» идёт через `xdg-open`, а маркировка
  текущего дефолтного в списке не показывается) — кандидат на доработку.

## 4. Файлы

Изменены: `crates/chronos-fm-services/src/mime.rs`,
`crates/chronos-fm-pages/src/explorer/context_menu.rs`,
`crates/chronos-fm-pages/src/explorer/navigation.rs`

(Прототип уже включал: `Cargo.toml`/`Cargo.lock` — `xdg-mime`,
`freedesktop-desktop-entry`; wiring в `view.rs`/`state.rs`/`row.rs`/
`grid.rs`/`explorer.rs`; всё это осталось в working tree незакоммиченным
от прерванной сессии.)

## 5. Связь с T006/b1 (сверил по просьбе пользователя)

b1 (`docs/agents/active/b1-clipboard-rename-foundation.md`) — **всё ещё
active**: `docs/agents/report/` пуст, `clipboard.rs`/`rename.rs`/
`file_ops.rs` в `explorer/` отсутствуют. Значит T006-гейт держится:
implementation plan не заводить, пока b1 не в report-log/done. Спец по
T006 можно писать параллельно (см. отдельный процесс по T006).
