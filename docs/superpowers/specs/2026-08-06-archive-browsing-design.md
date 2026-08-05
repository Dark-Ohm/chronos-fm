# Archive Browsing — Virtual Folder (read + write, v1)

**Дата:** 2026-08-06. **Автор:** Архитектор (brainstorm с Buffy).
**Тикет:** T004.

## Контекст и цель

Второй по критичности гэп до Dolphin-паритета: открыть `.zip`/`.tar.gz`/`.tar.zst`
как виртуальную папку в эксплорере без ручной распаковки на диск, с
возможностью редактирования файлов внутри архива и обратной записи.

## 1. Архитектура

### 1.1 Модуль `crates/chronos-fm-services/src/archive/`

Новый модуль, тот же уровень, что `fs/` и `devices/` (T003).

```
archive/
  mod.rs          — ArchiveFormat, split_archive_path(), ArchiveFs trait, LRU-кэш
  zip_archive.rs  — ZipArchive: impl ArchiveFs на крейте `zip`
  tar_archive.rs  — TarArchive: impl ArchiveFs на крейте `tar` + `flate2` + `zstd`
```

Крейты: `zip` (read + write, pure Rust), `tar` + `flate2` + `zstd` (read + write,
streaming). Все — без `unsafe`, совместимы с `unsafe_code = "deny"`.

### 1.2 Пути — виртуальная схема `::`

Файлы внутри архива адресуются синтетическими путями:

- Архив: `/home/neo/docs.zip`
- Корень архива: `/home/neo/docs.zip::/`
- Файл внутри: `/home/neo/docs.zip::/reports/report.txt`
- Поддиректория: `/home/neo/docs.zip::/reports/`

Парсер `split_archive_path(path: &str) -> Option<(PathBuf, String)>`:
если путь содержит `::`, возвращает `(archive_path, inner_path)`.
Если нет — `None` (обычная файловая система).

### 1.3 `ArchiveFs` trait

```rust
pub trait ArchiveFs: Send + Sync {
    /// List the root directory of the archive (or a subdirectory, if the
    /// implementation supports hierarchical listing — v1: flat list with
    /// directories synthesised from file paths).
    fn list(&self, inner_dir: &str) -> Result<Vec<FileEntryDto>>;

    /// Read a file's contents from the archive.
    fn read_file(&self, path_in_archive: &str) -> Result<Vec<u8>>;

    /// Write (replace or create) a file inside the in-memory archive state.
    fn write_file(&mut self, path_in_archive: &str, data: &[u8]) -> Result<()>;

    /// Remove a file from the in-memory archive state.
    fn remove_file(&mut self, path_in_archive: &str) -> Result<()>;

    /// Add a new file to the archive.
    fn add_file(&mut self, path_in_archive: &str, data: &[u8]) -> Result<()>;

    /// Flush the in-memory state back to the archive file on disk.
    fn commit(&mut self) -> Result<()>;
}
```

`list()` возвращает `Vec<FileEntryDto>` с путями вида
`"docs.zip::/reports/report.txt"`, `kind: "file"` / `"dir"`, `size` и
`modified` из метаданных архива.

### 1.4 `list_dir_sync` — прозрачная маршрутизация

`list_dir_sync` в `crates/chronos-fm-services/src/fs/listing.rs` модифицируется:

- Если путь содержит `::` → `split_archive_path()` → `archive::list_dir(archive_path, inner_path)`
- Если путь — обычный → существующее поведение (`std::fs::read_dir`)
- Если путь заканчивается на `.zip`/`.tar.gz`/`.tar.zst` (первый вход в архив) →
  `archive::open(archive_path)` + `list("/")` с путями вида `"archive.zip::/file.txt"`

`FileEntryDto` не меняется. Explorer (`navigation.rs`, `activate_entry`) не меняется:
`kind == "dir"` → `change_dir(path)`, иначе `open_preview(path)`.

### 1.5 Чтение файлов из архива (preview)

`open_preview` в `preview.rs` читает файл через `std::fs::read_to_string(path)`.
Для архивных путей нужна маршрутизация: если `path` содержит `::` → читать
через `archive::read_file(archive_path, inner_path)`. Точка врезки — в
`preview.rs`, рядом с существующим `std::fs::read_to_string`.

### 1.6 Write-back при редактировании

Когда пользователь редактирует файл внутри архива и сохраняет:
1. Обработчик сохранения проверяет путь на `::`
2. Если archive-путь → `archive::write_file(archive_path, inner_path, data)`
3. При выходе из архива (или при вытеснении из кэша) → `commit()`,
   перезаписывающая архивный файл на диске

### 1.7 Кэш открытых архивов (LRU)

Внутренняя деталь модуля `archive`. Глобальный `Mutex<LruCache<PathBuf, Box<dyn ArchiveFs>>>`.
Лимит — 3 открытых архива. При вытеснении: `commit()` (если были изменения), затем `drop`.

Архив открывается лениво при первом `list()`/`read_file()`.
Повторные операции в пределах сессии внутри архива не перечитывают файл с диска.

Если архив удалён/перемещён с диска пока мы в нём — следующая операция возвращает
`Err`, запись в кэш инвалидируется.

## 2. Форматы v1

| Формат | Крейт | Read | Write |
|--------|-------|------|-------|
| `.zip` | `zip` | ✓ | ✓ |
| `.tar` | `tar` | ✓ | ✓ |
| `.tar.gz` | `tar` + `flate2` | ✓ | ✓ |
| `.tar.zst` | `tar` + `zstd` | ✓ | ✓ |

**Вне v1:** `.7z`, `.rar`, `.tar.bz2`, `.tar.xz` — backlog.
Также вне v1: архивы внутри архивов, парольная защита/шифрование, потоковое
чтение (весь архив в памяти — для типичных размеров ок).

## 3. Explorer integration

`ExplorerPane::change_dir` — без изменений. Путь `"docs.zip::/subdir"` передаётся
как `cwd`, `reload()` вызывает `list_dir_sync(cwd)`, которая через `::`-маршрутизацию
возвращает `Vec<FileEntryDto>` из архива.

Back/forward, история — работают без изменений (это просто строки).

Sidebar (Folders/Devices) — архивы не показываются как специальные элементы.
Пользователь заходит в архив через основную область файлов (клик по `.zip`).

## 4. Error-обработка

- Повреждённый архив → `list_dir_sync` → `Err("corrupt archive: ...")`,
  Explorer показывает status error
- Неподдерживаемый формат → `Err("unsupported archive format: .7z")`
- Файл внутри архива не найден → `Err("file not found in archive: ...")`
- Ошибка записи (диск полон, permissions) → `Err(...)`, `commit()` возвращает ошибку,
  изменения не сброшены
- Архив удалён/перемещён с диска пока мы в нём → следующая операция `Err`,
  кэш инвалидируется

## 5. Верификация

- `cargo build --workspace` + `cargo test --workspace` чисто
- Юнит-тесты: создать `.zip`/`.tar.gz` в tempdir, открыть через `list_dir_sync`,
  проверить состав, прочитать файл, записать файл, `commit()`, переоткрыть —
  изменения сохранились
- Живой прогон: реальный `.zip` в эксплорере, клик → вход как в папку,
  просмотр файла, редактирование + сохранение, выход → переоткрытие — изменения
  на месте

## 6. Коммит

`services+pages : archive browsing — virtual folder for zip/tar.gz/tar.zst (v1: read + write, T004)`

---

## Принятые решения (из brainstorm)

1. **Read + write** (editable) — не только просмотр
2. **Форматы:** `.zip` + `.tar.gz` + `.tar.zst`
3. **Интеграция:** внутри `list_dir_sync` — прозрачно для Explorer
4. **Путь:** виртуальная схема `archive.zip::/inner/file.txt`
5. **Кэш:** LRU, 3 открытых архива, внутренняя деталь модуля `archive`
6. **FileEntryDto:** без изменений — существующих полей достаточно
