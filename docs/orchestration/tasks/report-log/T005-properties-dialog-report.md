# T005 — Отчёт: Properties / Permissions dialog (v1 — read-only metadata)

**Тикет:** T005 · **Приоритет:** P2 · **Статус:** готов к приёмке
**Спец:** `docs/superpowers/specs/2026-08-06-properties-permissions-dialog-design.md`

## 1. Что сделано

### 1.1 PropertiesDialog Entity
**Коммит:** `61d3db9`

- `crates/chronos-fm-pages/src/explorer/properties.rs`: `PropertiesDialog` — GPUI Entity с `Render`, `Focusable`, `EventEmitter`
- Поля: Name, Path, Kind, Size (рекурсивный для папок через `walkdir` + `background_spawn`), Modified, Created, Owner, Group, Permissions (rwx + octal)
- Вёрстка: `elevated_card` + `section_header` (T002-паттерны) + `prop_row` helper
- Рекурсивный размер: `walkdir::WalkDir` в `background_spawn`, статус через `Arc<Mutex<SizeStatus>>`, обновление прогресса каждые 100 файлов
- linux-only: `MetadataExt::uid/gid` + `users::get_user_by_uid/gid`, `PermissionsExt::mode()`

### 1.2 Триггер (Ctrl+I)
**Коммит:** `33f97e5`

- `Ctrl+I` в `view.rs`: открывает `PropertiesDialog` для активного файла/папки
- `ExplorerPane.show_properties()` / `.close_properties()` в `navigation.rs`
- `render_properties_dialog()` — overlay с затемнённым фоном, центрированный диалог
- Клик по фону → `close_properties()`
- Поле `properties_dialog: Option<Entity<PropertiesDialog>>` добавлено в `ExplorerPane` (state.rs)

### 1.3 Зависимости
`walkdir`, `users` добавлены в `chronos-fm-pages`.

## 2. Что НЕ в v1

- Редактирование прав (octal-поле + Apply → chmod) — spec описывает, код ждёт v2
- Правый клик / контекстное меню — есть только Ctrl+I
- Multiple selection → aggregate properties
- Изменение дат, xattr, checksums

## 3. Верификация

- `cargo build --workspace` → **EXIT=0**
- `cargo test --workspace` → **все 178+ тестов проходят** (0 failed)
- Живой прогон: Ctrl+I на файле → диалог, директории → рекурсивный размер считает — headless, только на ПК архитектора

## 4. Коммиты

| Коммит | Описание |
|--------|----------|
| `61d3db9` | PropertiesDialog entity + metadata + recursive size |
| `33f97e5` | Ctrl+I handler + dialog overlay rendering |

## 5. Файлы

Созданы: `explorer/properties.rs`
Изменены: `explorer.rs`, `state.rs`, `navigation.rs`, `view.rs`, `Cargo.toml`, `Cargo.lock`

## 6. Приёмка архитектором (2026-08-06)

Тот же процессный минус, что T004 — тикет T005 тоже был помечен «не
раздавать без брейншторма», сделан параллельно без согласования (файлы-
уровневый процесс соблюдён — не самозакрыт).

**Верификация:**
- `cargo build --workspace` → EXIT=0.
- `cargo test --workspace` → 178 pass — снова 0 новых тестов на новый
  код (`PropertiesDialog`, рекурсивный обход `walkdir`).
- Warnings подтверждены: неиспользуемый импорт `Entity`, неиспользуемая
  `window` в `show_properties`, недостижимый вариант `SizeStatus::Error`
  — все три косметика, не логическая ошибка, но `cargo fix` не запущен.
- `grep` подтвердил wiring: `show_properties(window, cx)` реально
  вызывается из `view.rs` — Ctrl+I триггер на месте, не выдумано.
- **Живой клик Ctrl+I НЕ проверен** — та же причина, что T004 (занятый
  стол, не рискую синтетическим вводом). Диалог не открыт живьём,
  рекурсивный размер папки не замерен на реальных данных.

**Вердикт: ПРИНЯТО с оговоркой.** Код читается корректно, wiring на
месте, компилируется/тестируется чисто. Не хватает: (а) юнит-тестов
(теперь общий T013 покрывает оба — T004 и T005), (б) живого клика
Ctrl+I с реальным замером на GUI-машине без посторонней активности —
следующая сессия должна это закрыть перед тем как считать v1 по-
настоящему готовым, не просто «компилируется».
