# Properties / Permissions Dialog (v1: metadata + editable permissions)

**Дата:** 2026-08-06. **Автор:** Архитектор (brainstorm с Buffy).
**Тикет:** T005.

## Контекст и цель

Третий по критичности гэп до Dolphin-паритета: правый клик по файлу/папке →
диалог Properties с метаданными и редактируемыми правами доступа. Без этого
невозможно посмотреть размер папки, проверить владельца или поменять права без
терминала.

## 1. Триггер

- **Правый клик** по файлу/папке в listing (list/grid/row) → контекстное меню →
  пункт «Properties»
- **Горячая клавиша:** `Ctrl+I`
- Контекстное меню — минимальное v1: пункт «Properties» (расширится в T006
  «Open with»). Открывается как anchored popup над строкой файла.

## 2. Архитектура

### 2.1 Модуль `crates/chronos-fm-pages/src/explorer/properties.rs`

Новый модуль, сосед `navigation.rs` / `state.rs` / `preview.rs`.

```rust
pub struct PropertiesDialog {
    item: FileEntryDto,
    metadata: Option<Metadata>,
    owner_name: Option<String>,
    group_name: Option<String>,
    recursive_size: Option<RecursiveSize>,
    size_status: SizeStatus,     // Idle | Counting | Done | Error(String)
    permissions_dirty: bool,
    edited_mode: Option<u32>,    // Изменённый octal mode
    error: Option<String>,       // Ошибка Apply
    saved: bool,                 // Галочка «Permissions updated»
}
```

### 2.2 Открытие — `Root::dialog(...)`

Использует встроенный диалоговый слой gpui-component (`Root::render_dialog_layer`
уже подключён в `root.rs`).

Открытие:
```rust
Root::dialog::<PropertiesDialog>(item, window, cx);
```

Поведение модала:
- Центрированная карточка на `elevated_card` + `section_header(cx, "Properties", path)`
- Затемнённый фон (`overlay`)
- Закрытие: Escape, клик по фону, кнопка «Close»
- Ширина: `px(420.)`

### 2.3 Сбор метаданных

При создании диалога:
1. `std::fs::metadata(&item.path)` → `Metadata`
2. `users::get_user_by_uid(meta.uid())` → имя владельца
3. `users::get_group_by_gid(meta.gid())` → имя группы
4. Если папка → `cx.background_spawn(walkdir)` для рекурсивного размера

## 3. Поля диалога

| Поле | Источник | Edit |
|------|----------|------|
| **Name** | `FileEntryDto.name` | нет |
| **Path** | родительский путь из `FileEntryDto.path` | нет |
| **Kind** | `"File"` / `"Directory"` / `"Symlink"` — с иконкой | нет |
| **Size** | `meta.len()` для файлов; рекурсивный подсчёт для папок | нет |
| **Modified** | `meta.modified()` → human-readable `"2026-08-06 14:22"` | нет |
| **Created** | `meta.created()` → human-readable (Linux: `MetadataExt::created`) | нет |
| **Owner** | `users::get_user_by_uid(meta.uid())` | да (TextInput) |
| **Group** | `users::get_group_by_gid(meta.gid())` | да (TextInput) |
| **Permissions** | `rwxr-xr-x` (человекочитаемое) + `755` (octal, editable) | да (TextInput octal) |

## 4. Рекурсивный размер папки

- `cx.background_spawn` запускает `walkdir::WalkDir` с подсчётом файлов и суммированием размеров
- В UI показывается `SizeStatus::Counting` → `"Counting… 1,432 files…"` (обновляется периодически)
- `SizeStatus::Done` → `"12.3 MB (1,432 files, 87 folders)"`
- Ошибка доступа к подпапке → `"12.3 MB (some files unreadable)"`
- `SizeStatus::Error(e)` → сообщение об ошибке
- Не кэшируется — каждый раз свежий подсчёт

## 5. Сохранение прав

- Кнопка **«Apply»** — активна только при `permissions_dirty == true`
- Octal-поле: валидация (только цифры 0-7, 3 или 4 символа)
- `chmod` через `std::fs::set_permissions(path, Permissions::from_mode(new_mode))` — не shell out
- `chown` через `std::os::unix::fs::chown(path, uid, gid)` — если uid/gid изменились
- Ошибка → `error: Some("chown failed: Permission denied")`, подсвечивается в диалоге
- Успех → `saved: true`, кнопка Apply disabled, зелёная галочка «Permissions updated»

## 6. Контекстное меню

Новый компонент в `explorer/view/` — `ContextMenu`:

- `on_secondary_mouse` на `ListItem`/строке файла → открывает anchored popup
- Пункты: `["Properties… (Ctrl+I)"]`
- В v1 — только один пункт. Закрывается по клику вне или Escape.
- Использует `anchored_popups` из gpui-ce форка (popup window, anchored к элементу)

## 7. Вне скоупа v1

- Multiple selection → aggregate properties («3 files selected, total…»)
- Изменение дат (touch)
- Расширенные атрибуты (xattr, ACL)
- Checksums (md5/sha256)
- «Open with» (T006, отдельный тикет)
- Рекурсивный chmod (применить права ко всем вложенным файлам)

## 8. Зависимости

- `T002` — `elevated_card` + `section_header` для вёрстки диалога
- `walkdir` (crate) — для рекурсивного обхода папок
- `users` (crate) — для резолва uid/gid → имя пользователя/группы
- `std::os::unix::fs::{MetadataExt, PermissionsExt, chown}` — Linux-specific
- `gpui_component::Root::dialog` — модальный слой
- `anchored_popups` skill — для контекстного меню

## 9. Верификация

- `cargo build --workspace` + `cargo test --workspace` чисто
- Юнит-тесты: парсинг octal → mode, mode → rwx-строку, валидация octal-поля
- Живой прогон: правый клик по файлу → Properties → видим метаданные, меняем
  права → Apply → `ls -l` подтверждает. Для папки — рекурсивный размер
  подсчитывается и отображается корректно.

## 10. Коммит

`pages : Properties dialog — metadata + editable permissions (T005, v1)`

---

## Принятые решения (из brainstorm)

1. **Modal в текущем окне** — через `Root::dialog(...)`, не отдельное окно
2. **Рекурсивный размер папки** — `background_spawn` + `walkdir` + индикатор «Counting…»
3. **Права редактируемые** — octal-поле, Apply → chmod/chown
4. **Контекстное меню** — минимальное v1 (один пункт), anchored popup
5. **Single file** — v1 не поддерживает свойства для множественного выбора
