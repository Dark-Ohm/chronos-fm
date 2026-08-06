# T008 — Devices-панель: клики mount/unmount + фикс object_path у eject

**Приоритет:** P1 — без этого T003 функционально бесполезен (панель
только показывает устройства, ничего не делает по клику).
**Роль:** реализация, точечная, самодостаточное задание.
**Источник:** `docs/orchestration/tasks/report-log/T003-removable-media
-devices-panel-report.md` §3.5, §3.6, §6 (приёмка архитектора).

## Контекст

T003 принят с оговоркой: весь пайплайн (парсер/backend/store/hotplug/
рендер) рабочий и живьём подтверждён, но:

1. **Клики не подключены.** `crates/chronos-fm-pages/src/explorer/view/
   sidebar.rs::render_devices_section` рендерит строки устройств с
   текстовыми метками «mount»/«unmount», но без `.on_click`. Это гэп в
   исходном implementation plan (Task 6 Step 3 сам план скопировал
   только «rendering only», конкретный шаг wiring-а не был расписан) —
   не недоработка предыдущего исполнителя, он сделал ровно то, что
   было в плане.
2. **`UDisks2Backend::eject` дремлющий баг.** Вызывает
   `org.freedesktop.UDisks2.Drive.Eject` на `object_path`, который в
   этой архитектуре — это ВСЕГДА Block/Filesystem-путь (см.
   `Device::object_path` в `parse.rs`), не Drive-путь. В v1 `eject` не
   вызывается из UI (sidebar показывает только mount/unmount), поэтому
   баг не проявлялся до сих пор — но должен быть починен раньше, чем
   что-либо начнёт вызывать eject.

Живой прогон архитектора (реальный udisks2, без физической флешки)
показал, что панель уже сейчас видит реальные внутренние разделы
(«VTOYEFI»/«Ventoy» на внутреннем `sdb`) — фильтр v1 намеренно этому не
препятствует (см. spec §2), но означает, что клик «mount» по неверно
понятому разделу — не безобидное действие. Тестировать этот тикет
живьём стоит ОСТОРОЖНО (см. Верификация).

## Что нужно

1. **Клик по строке устройства** (`crates/chronos-fm-pages/src/
   explorer/view/sidebar.rs::render_devices_section`):
   - Нужен доступ к `cx: &mut Context<ExplorerPane>` (не голый `&App`,
   как сейчас) и к `Arc<dyn DeviceBackend>` — прочитать, как
   `render`/`sidebar_item` для секции «Folders» в этом же файле
   получают контекст клика, повторить тот же путь (не изобретать
   параллельный механизм).
   - Немонтированный том → клик = `DeviceStore::mount_and_navigate(cx,
   backend, device.object_path.clone(), |cx, path| { /* navigate
   ExplorerPane в path — сверить, как это делает остальной sidebar
   (клик по Folders-элементу уже как-то навигирует) */ })`.
   - Смонтированный том → клик = просто navigate в `device.mount_point`
   (без вызова backend).
   - Отдельная кликабельная зона на строке (не весь ряд) для
   unmount — `DeviceStore::unmount(cx, backend, device.object_path.clone())`.
   - `Arc<dyn DeviceBackend>` для UI-слоя — где взять живой инстанс на
   момент клика: сверить, как `spawn_device_hotplug_watcher` в `app.rs`
   держит свой `Arc<UDisks2Backend>` — либо провести его через
   `DeviceStore`/отдельный `App`-глобал (`Arc<dyn DeviceBackend>` как
   ещё один Global, зарегистрированный рядом с `devices_store::init`),
   либо другим способом, но НЕ создавать новое D-Bus соединение на
   каждый клик.
2. **Фикс `eject`** (`crates/chronos-fm-services/src/devices/backend.rs`):
   `eject()` должен резолвить Drive-object-path через `Block.Drive`
   свойство (то же поле, что уже читает `parse.rs` для `is_removable`),
   а не звать `Drive.Eject` прямо на переданном `object_path`. Нужен
   доступ к `Device.object_path` → drive path — либо хранить
   `drive_object_path: Option<String>` в `Device` (добавить поле,
   заполнить в `parse_managed_objects`), либо резолвить на лету внутри
   `eject()` через ещё один `GetManagedObjects`/точечный `Get` вызов
   свойства `Block.Drive` для конкретного `object_path`. Первый вариант
   (поле в `Device`) дешевле и переиспользует уже собранные данные.
   Добавить unit-тест на `parse.rs`, что `drive_object_path` заполняется
   корректно (используя существующие тестовые фикстуры как образец).

## Зона файлов

`crates/chronos-fm-pages/src/explorer/view/sidebar.rs` (клики),
`crates/chronos-fm-services/src/devices/parse.rs` (+ `drive_object_path`
поле + тест), `crates/chronos-fm-services/src/devices/backend.rs`
(фикс `eject`), возможно `crates/chronos-fm/src/app.rs` (если backend
надо сделать доступным как Global). Не пересекается с b1-b4.

## Верификация

- `cargo build --workspace` + `cargo test --workspace` чисто.
- Живой прогон **ОСТОРОЖНО**: если физической флешки по-прежнему нет —
  НЕ кликать «mount» по реальным внутренним разделам (VTOYEFI/Ventoy на
  `sdb` — трогать чужой размеченный диск без явного запроса
  пользователя недопустимо). Вместо этого — либо дождаться, пока
  пользователь физически вставит тестовую флешку, либо ограничиться
  `cargo test` покрытием (`FakeBackend` в `backend.rs` уже даёт
  безопасный путь протестировать вызов клика через mock, без реального
  D-Bus) + код-ревью архитектора на предмет корректности wiring-а.
- Если тестовая флешка есть — полный цикл как в spec (клик → mount →
  navigate → `lsblk` подтверждает → клик unmount → `lsblk` подтверждает).

## Отчёт

`docs/orchestration/tasks/report/T008-devices-panel-click-wiring-report.md`
(inbox). Приёмка — архитектор лично. Принят → `report-log/`, тикет →
`done/`. Отклонён → `rejected/`.

## Коммит

`ui : wire Devices panel clicks (mount/unmount/navigate) + fix eject
object_path (T008)`
