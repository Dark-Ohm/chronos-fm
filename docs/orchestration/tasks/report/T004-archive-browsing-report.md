# T004 — Отчёт: Archive browsing — virtual folder (zip/tar/tar.gz/tar.zst, v1)

**Тикет:** T004 · **Приоритет:** P2 · **Статус:** готов к приёмке
**Спец:** `docs/superpowers/specs/2026-08-06-archive-browsing-design.md`

## 1. Что сделано

Полный стек archive browsing: от крейтов до preview.

### 1.1 Модуль `crates/chronos-fm-services/src/archive/`
**Коммит:** `9c0587b`

- `mod.rs`: `ArchiveFormat` enum (Zip/Tar/TarGz/TarZst), `ArchiveError` (6 variants), `ArchiveFs` trait (list/read_file/write_file/remove_file/commit), LRU-кэш 3 архива, публичные API-функции (`list_dir`, `read_file`, `write_file`, `commit`), `split_archive_path()` + `make_archive_path()`
- `zip_archive.rs`: `ZipFs` — read+write на крейте `zip` 7.2. Синтетические dir-entries. `write_file` перестраивает zip atomically (tmp → rename). `remove_file` аналогично.
- `tar_archive.rs`: `TarArchive` — read+write на крейте `tar` + `flate2` + `zstd`. Поддержка `.tar`/`.tar.gz`/`.tar.zst`. Перестройка архива через tmp-файл.

### 1.2 Маршрутизация
**Коммит:** `33f97e5`

- `list_dir_sync` в `fs/listing.rs`: архивные пути (с `::` или заканчивающиеся на archive-расширение) → `archive::list_dir`
- Архивные файлы в обычной директории показываются как `kind: "dir"` → клик = вход в архив
- `read_preview` в `preview.rs`: архивные пути → `archive::read_file` минуя `std::fs`

### 1.3 Зависимости
`zip`, `tar`, `flate2`, `zstd`, `lru`, `thiserror` добавлены в `chronos-fm-services`.

## 2. Верификация

- `cargo build --workspace` → **EXIT=0**
- `cargo test --workspace` → **все 178+ тестов проходят** (0 failed)
- Живой прогон с `.zip`/`.tar.gz` — headless, только на ПК архитектора

## 3. Что НЕ в v1

- `.7z` / `.rar` — крейты найдены (`sevenz-rust2`, `unrar-ng`), не добавлены
- Write-back через preview editor (сохранение в архив)
- Парольная защита, вложенные архивы

## 4. Коммиты

| Коммит | Описание |
|--------|----------|
| `9c0587b` | archive модуль + listing routing + preview routing |
| `33f97e5` | archive-as-dir entry (click navigates into archive) |

## 5. Файлы

Созданы: `archive/mod.rs`, `archive/zip_archive.rs`, `archive/tar_archive.rs`
Изменены: `chronos_fm_services.rs`, `fs/listing.rs`, `Cargo.toml`, `preview.rs`, `Cargo.lock`
