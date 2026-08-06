# T016 — Отчёт: поиск не отключается целиком из-за одной нечитаемой поддиректории

**Статус:** выполнено (код + юнит-тесты). Приёмка — архитектор лично.
**Приоритет:** P1.

## Корень (подтверждён кодом)

`FileWatcher::new` вешал рекурсивный `notify`-watch на весь `$HOME`
(`watcher.rs:39`). `notify` обходит дерево и при первой же
`PermissionDenied` (podman-volume `.../containers/storage/...`, UID 100999,
`0700`) возвращает `Err`, который `?` пробрасывался вверх:
`SearchEngine::new()?` → `SearchService::new()?` → `app.rs`, и весь поиск
умирал, оставляя `search_service: None`. Наружу — только строка в логе.

## Что сделано

### 1. Watcher переживает нечитаемые ветки (`search/watcher.rs`)
Двухфазный подход:
- **Быстрый путь:** пробуем один рекурсивный `watch` (на здоровых
  системах — полное покрытие, включая вновь созданные поддиректории).
- **Фолбэк:** если рекурсивный `watch` упал, обходим дерево сами
  (`ignore::WalkBuilder` + `filter_entry` по списку исключений) и
  вешаем `watch` на каждую **читаемую** директорию по отдельности,
  глотая `PermissionDenied` на каждой (`debug`, не `error`). Поиск
  остаётся жив, нечитаемая ветка просто не watch'ится.

`FileWatcher::new` теперь принимает `excludes: Excludes`.

### 2. Индексатор (`search/indexer.rs`)
`Excludes` проброшен в `IndexManager`; обе прогулки (`ignore::WalkBuilder`
для подсчёта и для индексации) получили `.filter_entry(|e| !excludes.matches(e.path()))`
— тяжёлые/чужие деревья (`containers`, `node_modules`, `target`, `.git`,
`.cache`, FUSE/gvfs) не обходятся вообще. На отдельных записях ошибки и так
переживались (`if let Err` в `index_home`, `read_dir`-ошибки в `WalkBuilder`
логируются) — это сохранено.

### 3. Исключения по умолчанию (`search/exclusions.rs`, новый модуль)
- `DEFAULT_EXCLUDE_COMPONENTS` — хардкод-денлист по имени компонента
  (`containers` ловит `~/.local/share/containers/...`, плюс
  `node_modules`, `target`, `.git`, `.cache`, `.gvfs`, `.cargo`, `.npm`,
  `.rustup`, `.Trash`).
- `Excludes::from_config(paths, globs)` — подхватывает существующую секцию
  `[indexing.exclude]` из `config.toml` (paths + globs), которая раньше
  никуда не была подключена. Простой glob-матчер (`**/target/**`,
  `node_modules`, `build/*`).
- `Excludes::matches(path)` применяет и то, и другое.

`Excludes` проброшен: `app.rs` (строит из `config.indexing.exclude`) →
`SearchService::new(excludes)` → `SearchEngine::new(excludes)` →
`IndexManager::new` + `FileWatcher::new`.

### 4. Отказ виден в UI (`pages/.../listing/search_bar.rs`)
Когда `search_service == None`, в поисковой строке показывается
`⚠ Full-text search unavailable — index failed to load` (цвет `theme::danger`).
Молча��ивая деградация устранена. Фильтр по имени файла работает и без
индекса.

### 5. Проверка其它 рекурсивных обходов
- `properties.rs` (рекурсивный размер): **уже толерантен** — `Err(_) =>
  errors += 1`, продолжается. Без изменений.
- `fs/ops.rs` `copy_tree`: `read_dir(src)` падает на явно выбранном
  пользователем пути — корректное поведение для пользовательского действия,
  не меняем.

## Тесты (юнит, воспроизводимы без GUI)

`crates/chronos-fm-services/src/search/`:
- `watcher::tests::watcher_survives_unreadable_subdir` — временное дерево с
  `chmod 000` поддиректорией; `FileWatcher::new` **не** возвращает `Err`,
  и изменение читаемого файла даёт событие изменения. `#[cfg(unix)]`,
  skip под root (поведенческая проверка `read_dir` на chmod 000).
- `indexer::tests::index_home_tolerates_unreadable_subdir` — индексация
  дерева с нечитаемой веткой возвращает `Ok`, доступный файл попадает в
  индекс, ошибки нет. `#[cfg(unix)]`, skip под root.
- `exclusions::tests::*` — 4 теста на `DEFAULT_EXCLUDE_COMPONENTS`,
  абсолютный/относительный `paths` и glob `**/target/**`.

## Верификация

- `cargo test -p chronos-fm-services --lib search` — **6/6 тестов прошли**
  (включая два регрессионных T016). `crates/chronos-fm-services`
  собирается без ошибок.
- **БЛОКЕР (предсуществующий, не из T016):** в этом окружении
  `chronos-fm` / `chronos-fm-pages` **не собираются** из-за конфликта
  фич зависимостей в форке gpui: `gpui_linux` включает `ashpd/async-io`,
  а `oo7` (через `secret`-фичу `gpui_linux`) включает `ashpd/tokio` —
  взаимоисключающие, feature-unification включает оба →
  `compile_error!("You can't enable both async-io & tokio features at once")`.
  Воспроизводится на чистом HEAD (проверено через `git stash -u` +
  `cargo check -p chronos-fm`). Из-за этого правки в `app.rs` и
  `search_bar.rs` **проверены по коду, но не собраны в этом окружении**
  (редакции механические и корректны по типам). Нужен фикс в форке gpui
  (например, перевести `oo7` на `async-io`) или флаг фич — вне зоны T016.
- Живой прогон на машине с podman-volume и проверка отсутствия
  `Failed to initialize search service` в логе — **за архитектором**
  (headless + build-blocked здесь).

## Коммит

`fix : search survives unreadable $HOME subdirs (T016)`
Файлы: `search/exclusions.rs` (новый), `search.rs`, `search/watcher.rs`,
`search/indexer.rs`, `search/engine.rs`, `crates/chronos-fm/src/app.rs`,
`crates/chronos-fm-pages/src/explorer/view/listing/search_bar.rs`.
