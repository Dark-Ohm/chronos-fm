# T008 — Devices-панель: клики mount/unmount + фикс object_path у eject — отчёт

**Статус:** inbox, на приёмку архитектору (P1).
**Приоритет:** P1 — клики делают панель T003 функционально полезной.
**Связанные:** `docs/orchestration/tasks/report-log/T003-removable-media-devices-panel-report.md` (§3.5, §3.6, §6), spec `docs/superpowers/specs/2026-08-06-removable-media-mount.md`.

## Контекст

T003 принят с оговоркой: панель показывала устройства, но клики не были
подключены, а `eject` звал `Drive.Eject` на Block/Filesystem-пути (в этой
архитектуре `Device::object_path` — ВСЕГДА Block-путь, не Drive-путь). Данный
тикет закрывает оба гэпа. Живой прогон с реальной флешкой НЕ выполнялся —
физической тестовой флешки нет, реальные внутренние разделы (VTOYEFI/Ventoy
на `sdb`) не трогались (тикет: «трогать чужой размеченный диск без явного
запроса пользователя недопустимо»). Покрытие — mock-тесты (FakeBackend-стиль)
+ код-ревью, как и предписывает секция «Верификация» тикета.

## Что сделано

### 1. Клики по строке устройства — `sidebar.rs::render_devices_section`

- Функция переведена с `&App` на `&mut Context<ExplorerPane>` — повторён тот же
  путь получения контекста клика, что у секции «Folders» (shortcuts рендерятся
  через `cx.listener` в этом же файле; параллельный механизм не изобретался).
- **Backend доставлен в UI-слой** через новое поле
  `DeviceStore.backend: Option<Arc<dyn DeviceBackend>>`. Живой инстанс
  регистрируется в `spawn_device_hotplug_watcher` (app.rs) сразу после
  `connect()` — **до** первого `refresh_devices`, поэтому строки никогда не
  рендерятся без backend. Новое D-Bus соединение на клик не создаётся.
- Строка устройства (`.id(...)` + `.on_click`):
  - смонтированный том → клик = `change_dir(device.mount_point)` (без вызова
    backend);
  - немонтированный том → клик = `DeviceStore::mount_and_navigate(cx, backend,
    object_path, |cx, path| pane.navigate_to_path(path, cx))`.
- **Отдельная кликабельная зона «unmount»** (правый край строки, только для
  смонтированных): `DeviceStore::unmount(cx, backend, object_path)` +
  `cx.stop_propagation()` — unmount не превращается в навигационный клик.
  При отсутствии backend зона рендерится как пассивная метка (и строка без
  обработчиков клика).
- **Ре-рендер после асинхронных операций:** `subscribe_tab` (page.rs) навешивает
  на каждую вкладку `cx.observe_global::<DeviceStore>`; `update_global`
  авто-пушит `NotifyGlobalObservers` — вкладка перерисовывается, лейбл
  mount/unmount и строка `last_error` обновляются по завершении mount/unmount.
- `on_click` в этом gpui-форке живёт на `StatefulInteractiveElement` — каждая
  кликабельная строка/зона получила `.id(...)` (уникальный по `object_path`).

### 2. Фикс `eject` — `backend.rs` + `parse.rs`

- `Device` получил поле `drive_object_path: Option<String>`; заполняется из
  `Block.Drive` в `parse_managed_objects` (вариант тикета №1 — дешевле
  точечного `Get` и переиспользует уже собранные данные; то же поле, что
  читает `parse.rs` для `is_removable`). `None` — когда Drive-ссылки нет.
- Сигнатура трейта: `eject(&self, object_path: &str, drive_object_path:
  Option<&str>)` — `Drive.Eject` теперь вызывается на резолвленном Drive-пути,
  а не на Block/Filesystem-пути панели. Фолбэк `drive_object_path.unwrap_or(
  object_path)` вынесен в тестируемую свободную функцию `eject_target`.
- Все реализации трейта обновлены: `UDisks2Backend`, `FakeBackend` (services),
  `MountOkBackend` (ui), `DevicesFakeBackend` (pages-тесты).

### 3. Навигация без `Window` — `navigation.rs`

- Новый `ExplorerPane::navigate_to_path(path, cx)` — колбэк mount имеет только
  `&mut App` (без `Window`); эмитит `PaneEvent::Navigated` (синопные панели
  следуют, сессия сохраняется).
- Рефакторинг: общий приватный `navigate_without_window(path, emit, cx)` —
  `navigate_to_path` и `navigate_to_synced` больше не дублируют логику
  (замечание код-ревью).

### 4. Прочее

- `show_properties` (navigation.rs) — убран неиспользуемый параметр `window`
  (вызов в view.rs обновлён).
- `properties.rs` — убран неиспользуемый импорт `Entity` (pre-existing warning).

## Тесты (mock-покрытие, без D-Bus)

| Тест | Что проверяет |
|---|---|
| `devices::backend::tests::eject_target_resolves_drive_path_with_fallback` | резолв Drive-пути + фолбэк на Block-путь |
| `explorer::tests::navigate_to_path_moves_pane_and_clears_search_without_window` | навигация без Window, сброс поискового состояния, no-op на тот же путь |
| `explorer::tests::device_mount_then_navigate_moves_pane_into_mount_path` | полная цепочка клика: `mount_and_navigate` (in-memory fake) → `pane.navigate_to_path` — ровно то, что делает `.on_click` строки |
| `devices_store::tests::unmount_clears_mount_point_on_success` | ручка store, которую диспатчит клик «unmount» |
| `parse.rs` × 2 | `drive_object_path` заполняется из `Block.Drive` / `None` без Drive-ссылки |

`async-trait` добавлен в dev-dependencies `chronos-fm-pages` (для fake-реализации
`DeviceBackend` в тестах).

## Верификация

- `cargo check -p chronos-fm-services -p chronos-fm-ui -p chronos-fm-pages -p chronos-fm` — чисто.
- `cargo test --workspace` — **все зелёные** (pages 58, services 38, ui 25, остальные), EXIT=0.
- `cargo build -p chronos-fm` — EXIT=0.
- `cargo clippy` — в затронутых файлах новых предупреждений нет (только
  pre-existing `missing_docs` из T003).
- Живой прогон — **осознанно пропущен** (нет тестовой флешки; внутренние разделы
  не трогаем). Замена — mock-тесты выше + ревью архитектора на корректность
  wiring-а. При появлении флешки — полный цикл spec (клик → mount → navigate →
  `lsblk` → клик unmount → `lsblk`).

## Известные ограничения (v1)

- После успешного eject строка остаётся видимой (с очищенным `mount_point`)
  на несколько кадров, пока hotplug-watcher не переспросит список и не уберёт
  выброшенный том. Приемлемо; локально чистим только `mount_point`.

### Eject action (follow-up)

«Eject» теперь вызывается из UI: у каждой строки съёмного тома (фильтр
`is_removable && drive_object_path.is_some()` — внутренние несъёмные диски вроде
VTOYEFI/Ventoy не получают eject, тикет T008: чужие внутренние разделы не
трогаем) справа появилась отдельная кликабельная зона «eject» рядом с
mount/unmount, с `stop_propagation` (не срабатывает навигационный клик строки).

- `DeviceStore::eject(cx, backend, object_path, drive_object_path)` — новый
  метод store: фоновый `backend.eject(object_path, drive.as_deref())` (T008-фикс:
  `Drive.Eject` зовётся на резолвленном Drive-пути), успех → `mount_point`
  очищен (udisks2 сам размонтирует как часть eject), ошибка → `last_error`.
- In-flight guard симметричен mount/unmount (`ejecting`), плюс **cross-операция**:
  eject не пойдёт, пока тот же том монтируется, и mount/unmount не пойдут, пока
  том выбрасывается (`Drive.Eject` vs `Filesystem.Mount` гонку закрыли).
- Тесты (×3): `eject_forwards_drive_path_and_clears_mount_point` (резолвленный
  Drive-путь реально доходит до backend — фиксирует сам T008-фикс),
  `eject_drops_duplicate_while_in_flight` (guard + release),
  `eject_is_dropped_while_mount_is_in_flight` (cross-операция).

### In-flight guard (follow-up)

> Guard теперь покрывает и eject (см. ниже); mount/unmount также не идут,
> пока том выбрасывается.

Быстрый двойной клик по немонтированному тому больше не запускает дублирующий
`backend.mount()`: `DeviceStore` держит `mounting`/`unmounting` — множества
`object_path` с операцией в полёте. Путь регистрируется синхронно (до `cx.spawn`,
поэтому второй клик в том же кадре уже заблокирован) и снимается и на успехе, и
на ошибке (том снова кликабелен после завершения операции). Симметричный guard
для unmount — иначе второй клик звал бы `backend.unmount()` уже на размонтированном
томе и показывал бы ложную ошибку «Unmount failed». Покрыто двумя тестами
(`mount_and_navigate_drops_duplicate_while_in_flight`,
`unmount_drops_duplicate_while_in_flight`) с `CountingBackend`, считающим вызовы
бэкенда: два клика в одном кадре → один вызов; после `run_until_parked` → guard
снят, следующий клик работает.

## Файлы

- `crates/chronos-fm-pages/src/explorer/view/sidebar.rs` (клики, id, unmount-зона)
- `crates/chronos-fm-pages/src/explorer/navigation.rs` (`navigate_to_path`, `navigate_without_window`, `show_properties`)
- `crates/chronos-fm-pages/src/explorer/page.rs` (`observe_global::<DeviceStore>`)
- `crates/chronos-fm-pages/src/explorer/tests.rs` (+ `async-trait` dev-dep)
- `crates/chronos-fm-pages/src/explorer/view.rs` (вызов `show_properties`)
- `crates/chronos-fm-pages/src/explorer/properties.rs` (лишний импорт)
- `crates/chronos-fm-services/src/devices/parse.rs` (`drive_object_path` + тесты)
- `crates/chronos-fm-services/src/devices/backend.rs` (`eject_target`, фикс `eject`)
- `crates/chronos-fm-ui/src/devices_store.rs` (`backend`-поле, тест unmount)
- `crates/chronos-fm/src/app.rs` (регистрация backend в watcher)

## Коммит

`ui : wire Devices panel clicks (mount/unmount/navigate) + fix eject object_path (T008)`
