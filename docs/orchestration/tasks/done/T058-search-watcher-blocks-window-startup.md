# T058 — Синхронный рекурсивный watcher на `$HOME` блокирует открытие окна (~35s)

**Приоритет:** P1 — приложение выглядит зависшим/не запустившимся при
каждом холодном старте на реальном `$HOME`, не косметика.
**Статус:** баг с установленным корнем, готов к исполнению.
**Источник:** живой прогон свежесобранного release-бинаря 2026-08-11,
лог запуска (`/tmp/chronos-fm-run.log`), root-cause по коду, не гипотеза.

## Симптом

После запуска `./target/release/chronos-fm` окно не появляется/не
готово к вводу заметно дольше, чем ожидалось. Лог показывает тишину:

```
06:06:14.458  gpui_wgpu: Selected GPU adapter: NVIDIA GeForce RTX 3070 (Vulkan)
   ... 35 секунд без единой строки в логе ...
06:06:49.169  tantivy::indexer::index_writer: Preparing commit
06:06:49.194  chronos_fm_services::search::engine: Index already has 151582
              documents, skipping initial indexing
```

35 секунд между выбором GPU и первым признаком жизни поискового
движка — не сеть, не диск, не GPU: за это время ничего не
залогировано вообще, то есть поток занят синхронной работой без
трассировки.

## Корень (установлен по коду)

1. `crates/chronos-fm/src/app.rs:100` — `SearchService::new(excludes)`
   вызывается **синхронно** внутри `app.open_window(...)`, до создания
   `RootView` — то есть блокирует построение окна/первого кадра.
2. `SearchService::new` → `SearchEngine::new` (`search/engine.rs:118`)
   → `FileWatcher::new(home_dir, tx, WATCHER_DEBOUNCE, excludes)`.
3. `search/watcher.rs:79-82`:
   ```rust
   let recursive_ok = debouncer
       .watcher()
       .watch(&root, RecursiveMode::Recursive)
       .is_ok();
   ```
   На Linux это `notify-rs`/inotify: `RecursiveMode::Recursive`
   **синхронно обходит всё дерево** `root` (= `$HOME`), навешивая
   inotify-watch на каждый подкаталог, прежде чем вернуть управление
   вызывающему коду.
4. Масштаб проверен: `find /home/neo -type d` → **303 565 каталогов**
   (`target/`, `.cargo`, `.worktrees`, podman/container-тома и т.д.).
   Обход именно этого дерева и есть измеренные ~35 секунд.

**Асимметрия с уже решённой частью той же функции:** первичная
индексация (`take_initial_indexing_job` → `job.run()`,
`app.rs:114-117`) явно вынесена в `cx.background_spawn` — в коде есть
комментарий, что это сделано намеренно, потому что операция дорогая.
Установка recursive watcher'а на `$HOME` рядом в той же цепочке вызовов
**не** вынесена, хотя дороже или сравнима по стоимости (T016 уже
зафиксировал, что `$HOME` этого пользователя содержит нечитаемые/чужие
поддеревья — тот же корень дерева, другая ось проблемы: там падение,
здесь блокировка).

## Что нужно

1. **Убрать watcher-setup с потока, строящего окно.** `FileWatcher::new`
   (или весь `SearchEngine::new`) должен уйти в `cx.background_spawn`
   так же, как `InitialIndexingJob::run`, а не выполняться внутри
   `app.open_window` до `RootView::new`.
2. **`search_service` в `RootView` должен появляться асинхронно.**
   Сейчас `Option<Arc<SearchService>>` передаётся в конструктор
   `RootView` уже готовым (`app.rs:100-124`) — окно ждёт его. Нужен
   путь, при котором окно открывается немедленно с
   `search_service: None`/«поиск инициализируется», а сервис
   подставляется, когда фоновая задача завершится (канал/сигнал в
   `Context`, `cx.spawn` + `cx.notify()` — обычный GPUI-паттерн, не
   первый в этой кодовой базе).
3. **Не терять уже сделанную работу T016.** Fallback-путь (watch по
   отдельным читаемым директориям с `excludes`) остаётся — просто
   должен исполняться в фоне, а не менять поведение при ошибках.
4. **Не завести новый регресс от переноса в фон:** до момента, пока
   watcher не поднялся, UI поиска должен честно показывать «индексация
   идёт» / «watcher не готов» — не выглядеть как рабочий поиск, который
   молча ничего не находит.

## Тесты (обязательны)

- Юнит на время: с моком/tempfile-деревом разумного размера убедиться,
  что `FileWatcher::new` вызывается не в конструкторе окна (структурный
  тест на факт `cx.background_spawn`/эквивалент, не таймер — таймер по
  реальному `$HOME` в CI нестабилен).
- Существующие тесты T016/T033 (`watcher.rs` — фильтрация self-index
  путей, переживание нечитаемых поддиректорий) должны остаться зелёными
  после переноса вызова в фон.
- `cargo test --workspace` зелёный.

## Зона файлов

`crates/chronos-fm/src/app.rs` (перенос вызова в background_spawn,
проброс готовности сервиса в `RootView`),
`crates/chronos-fm-services/src/search/engine.rs` (`SearchEngine::new`),
`crates/chronos-fm-services/src/search/watcher.rs` (сам вызов
`RecursiveMode::Recursive`), возможно `crates/chronos-fm-ui` /
`RootView` (приём отложенного `search_service`).

## Верификация (обязательно живым прогоном, не только юнитом)

- Свежий release-бинарь на **этой** машине (тот же `$HOME` с 300k+
  каталогов): время от старта процесса до появления окна на
  `hyprctl clients` — секунды, не десятки секунд.
- Лог: между `Selected GPU adapter` и первым признаком готовности окна
  не должно быть многосекундного молчания, вызванного watcher-setup;
  строки watcher/tantivy могут появляться позже, асинхронно.
- `cargo test --workspace` зелёный, новые/старые watcher-тесты видны в
  счётчике.

## Отчёт

`docs/orchestration/tasks/report/T058-search-watcher-startup-report.md`
(inbox). Приёмка — архитектор лично. Принят → `report-log/`, тикет →
`done/`. Отклонён → `rejected/`.
