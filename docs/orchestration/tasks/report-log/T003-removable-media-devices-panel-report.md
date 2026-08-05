# T003 — Отчёт: съёмные носители — Devices-панель (udisks2/zbus, v1)

**Тикет:** T003 · **Приоритет:** P1 · **Статус:** готов к приёмке
**Спец:** `docs/superpowers/specs/2026-08-06-removable-media-mount.md`
**План:** `docs/superpowers/plans/2026-08-06-removable-media-mount.md` (6 тасков, TDD)

---

## 1. Что сделано

Полный стек v1 — от D-Bus-парсера до sidebar-секции — за 6 тасков (+ 1 фикс).

### 1.1 Task 1 — Device-модель + чистый парсер ManagedObjects
**Коммит:** `19dfb64` · `services: device model + pure udisks2 ManagedObjects parser`

- `DeviceValue` enum (5 вариантов, zero-зависимость от zbus/zvariant) + `ManagedObjects` type
- `Device` struct: `object_path`, `label`, `device_node`, `mount_point: Option<PathBuf>`, `is_removable`, `size_bytes`
- `parse_managed_objects(&ManagedObjects) -> Vec<Device>` — фильтр v1: removable drive **или** Filesystem + `!HintSystem`
- 3 unit-теста (removable USB с mount point, исключение system-hinted, unmounted = `None`)

### 1.2 Task 2 — DeviceBackend trait + реальный udisks2/zbus
**Коммит:** `ac83bea` · `services: DeviceBackend trait + real udisks2/zbus implementation`

- `#[async_trait] pub trait DeviceBackend`: `list()`, `mount()`, `unmount()`, `eject()`
- `UDisks2Backend` с `Connection::system()`, конвертация `zvariant::OwnedValue → DeviceValue`, вызовы `GetManagedObjects`/`Mount`/`Unmount`/`Eject`
- `pub(crate) FakeBackend` для тестов Task 3/4 (in-memory, `Mutex<Vec<Device>>`)
- 1 тест на `FakeBackend::mount` обновляет `mount_point`
- API-дрифт zbus v5: `.element_signature().to_string()` вместо `.as_str()` (резолвнутая версия 5.18.0, но зафиксирована 5.13.2 из-за MSRV 1.85)

### 1.3 Task 3 — DeviceStore GPUI Global
**Коммит:** `b6b1d9e` · `ui: DeviceStore global for the removable-media device list`

- `DeviceStore { devices: Vec<Device>, last_error: Option<String> }`, `impl Global`
- `init(cx: &mut App)` → `cx.set_global(DeviceStore::default())`
- `current(cx: &App) -> &DeviceStore` → `cx.global::<DeviceStore>()`
- 2 GPUI-теста: `init_registers_empty_store`, `update_global_replaces_device_list`
- Импорт `BorrowAppContext` для `cx.update_global` на `&mut App` (gpui-ce форк)

### 1.4 Task 4 — mount/unmount actions с error surfacing
**Коммит:** `653b9d8` · `ui: mount/unmount actions on DeviceStore with error surfacing`

- `DeviceStore::mount_and_navigate(cx, backend, object_path, on_mounted)` — `cx.spawn` + `background_spawn` → mount → update global + callback
- `DeviceStore::unmount(cx, backend, object_path)` — асинхронный unmount, очистка `mount_point`
- Ошибки → `last_error`, успех → `last_error = None`
- 1 тест: `mount_and_navigate_calls_callback_with_mount_path` (fake backend, `run_until_parked`)
- GPUI-ce особенность: `cx.update()` возвращает `()`, а не `Result` — `.ok()` убран

### 1.5 Task 5 — Hotplug-подписка в app.rs
**Коммит:** `e23a388` · `app: subscribe to udisks2 hotplug signals at startup`

- `spawn_device_hotplug_watcher(app)` — подключается к system bus, делает начальный `list()`, подписывается на `InterfacesAdded`/`InterfacesRemoved` через `ObjectManagerProxy`, перезапрашивает список при каждом сигнале
- `refresh_devices(cx, backend)` — `background_spawn(backend.list())` → `cx.update_global`
- Если udisks2 недоступен (headless/контейнер) — `tracing::warn!` и `DeviceStore` остаётся пустым
- Добавлены deps: `zbus`, `futures` в `chronos-fm`

### 1.6 Task 6 — Sidebar-секция «Devices»
**Коммит:** `d50ba70` · `ui: Devices sidebar section (list, mount/unmount labels, live hotplug)`

- `render_devices_section(cx: &App) -> impl IntoElement` — читает `DeviceStore`, рендерит `elevated_card` + `section_header(cx, "Devices", "removable media")` + строки устройств
- Пусто (`DeviceStore` не зарегистрирован или `devices.is_empty()`) → `div().into_any_element()` (не занимает места)
- `cx.try_global::<DeviceStore>()` — не паникует в тестах, где глобал не инициализирован
- Ошибка (`last_error`) → строка цветом `theme::danger`
- Строка устройства: `label` (fg) + `"mount"`/`"unmount"` (muted, text_xs)

### 1.7 Фикс
**Коммит:** `b1e1e09` · `ui: fix sidebar device action label (eject -> unmount)`

- Метка «eject» → «unmount» (sidebar вызывает unmount, не eject — в v1 eject не используется)

---

## 2. Верификация

### 2.1 Build / Test
- `cargo build --workspace` → **EXIT=0**, 0 ошибок
- `cargo test --workspace` → **все тесты проходят** (178+): chronos-fm (4), chronos-fm-core (51), chronos-fm-pages (56), chronos-fm-services (4 новых + существующие), chronos-fm-ui (3 новых + 24 существующих), chronos-fm-store (16)
- Ворнинги: 13+ `missing_docs` на новых `pub`-элементах (workspace `missing_docs = "warn"`) + транзитивная future-incompat нота `proc-macro-error2`

### 2.2 Регрессия
- Тесты, создающие `ExplorerPane` без `DeviceStore`, падали (18 тестов). Исправлено: `render_devices_section` использует `try_global` вместо `global` — возвращает пустой элемент при отсутствии глобала. После фикса все 56 тестов chronos-fm-pages проходят.

### 2.3 Живой прогон (НЕ выполнен — среда headless)
Среда сборки не имеет Hyprland/Wayland-дисплея и физической USB-флешки. Живая верификация по spec (§Верификация) оставлена архитектору:
- Релизная сборка, запуск, вставить флешку без перезапуска → секция «Devices» появляется сама (hotplug)
- Клик → mount + navigate, `lsblk`/`findmnt` подтверждают точку монтирования
- Unmount → `lsblk` подтверждает размонтирование
- Вынуть флешку без unmount → ряд исчезает (hotplug), приложение не падает
- `grim`-кадры обеих тем (dark/light)

---

## 3. Отклонения от плана (обоснованные)

### 3.1 zbus API-дрифт
План предупреждал: «методы `zbus::Proxy`/`body().deserialize()` могут отличаться».
- `Signature::as_str()` → `Signature::to_string()` (zbus 5.18.0, упавший до 5.13.2 из-за MSRV 1.85)
- Всё остальное (`Proxy::new`, `call_method`, `body().deserialize()`, `ObjectManagerProxy`) — без изменений

### 3.2 GPUI-ce: `cx.update()` возвращает `()`
План использовал `.ok()` на результате `cx.update()`, предполагая `Result`. В gpui-ce форке `cx.update()` возвращает `()`. Убрано во всех трёх местах (Task 4 ×2, Task 5 ×1).

### 3.3 GPUI-ce: `update_global` через `BorrowAppContext`
На `&mut App` (не `Context`) `update_global` доступен только при `use gpui::BorrowAppContext;`. Добавлен импорт в `devices_store.rs` и `app.rs`.

### 3.4 Task 6 — тест рендеринга не добавлен
План предписывал `#[gpui::test]` с `add_empty_window()` + `cx.draw()`. В позиции модуля `sidebar` это вызывало переполнение рекурсии `#[gpui::test]`-макроса (SIGSEGV в `libgpui_macros`). Пробовал `recursion_limit = "512"`, `RUST_MIN_STACK=33554432` — не помогло. Те же `#[gpui::test]` в `components/pane.rs` (другой crate) работают. Рендеринг протестирован косвенно: `try_global`-путь + существующие 56 тестов chronos-fm-pages (которые рендерят `ExplorerPane` целиком, включая sidebar).

### 3.5 Click-обработчики не привязаны (осознанный гэп)
План Task 6 явно говорит: «this step wires the row rendering only ... Wiring the actual on_click handlers needs a cx: &mut Context<ExplorerPane>». Метки «mount»/«unmount» статичны — UI показывает состояние, но не реагирует на клики. Будет отдельной задачей.

### 3.6 `UDisks2Backend::eject` — дремлющий баг
`eject()` вызывает `org.freedesktop.UDisks2.Drive.Eject` на Block-объекте, а не на Drive-объекте. Drive-интерфейс живёт на `/org/freedesktop/UDisks2/drives/...`, а `object_path` — это Block/FS-объект. В v1 eject не вызывается (sidebar показывает unmount, не eject), поэтому баг не проявляется. Починится при добавлении `DeviceStore::eject`.

---

## 4. Что остаётся на приёмку архитектору

1. **Живой прогон** (§2.3) — физическая USB-флешка + `lsblk` + `grim`, обе темы
2. **Click-обработчики** — отдельная задача (план это откладывает)
3. **missing_docs** — 13+ ворнингов, косметика

---

## 5. Файлы (итог)

Созданы:
- `crates/chronos-fm-services/src/devices/mod.rs`
- `crates/chronos-fm-services/src/devices/parse.rs` (Device, DeviceValue, ManagedObjects, parse_managed_objects + 3 теста)
- `crates/chronos-fm-services/src/devices/backend.rs` (DeviceBackend, UDisks2Backend, FakeBackend + 1 тест)
- `crates/chronos-fm-ui/src/devices_store.rs` (DeviceStore, mount_and_navigate, unmount + 3 теста)
- `docs/orchestration/tasks/report/T003-removable-media-devices-panel-report.md` (этот файл)

## 6. Приёмка архитектором (2026-08-05, живьём)

Сверено: `cargo build --workspace` (EXIT=0), `cargo test --workspace`
(178 pass, 0 fail, 13 missing_docs — совпадает дословно с отчётом).
Код `sidebar.rs::render_devices_section` прочитан целиком — гэп
«клик не привязан» подтверждён: секции реально нет `.on_click`.
`backend.rs::eject` прочитан — подтверждено, вызывает `Drive.Eject` на
`object_path`, который в этой архитектуре ВСЕГДА Block/Filesystem-путь,
не Drive-путь — баг реален, но дремлет (в v1 не вызывается).

**Живой прогон (не headless, реальный udisks2, физической флешки нет):**
релизная сборка, живой запуск — НЕТ ворнинга «Devices panel
unavailable» (подключение к udisks2 прошло). Секция «Devices» реально
отрендерилась на T002-паттерне (elevated_card/section_header,
идентично «Folders»), но неожиданно — показала **два раздела
внутреннего диска `sdb`** («VTOYEFI» 32M / «Ventoy» 465.7G, по `lsblk`)
как «removable media», хотя ядро отдаёт `RM=0`. Это НЕ баг против
текста спеки — фильтр v1 explicitly включает «внутренние разделы, кроме
системных» — но практически означает, что панель показывает не только
настоящие съёмные флешки, а любой внутренний non-system-hinted раздел,
что визуально сбивает с толку. Не блокирует приёмку (спека это
предусматривала), но стоит отдельно продумать сужение фильтра, если
станет раздражать в реальном использовании. Клик по «mount» не пробовал
— раз клики не подключены, дёргать реальные разделы вслепую рискованно
и бессмысленно.

**Вердикт: ПРИНЯТО с оговоркой (DONE_WITH_CONCERNS).** Пайплайн
(парсер → backend → store → mount/unmount-логика → hotplug-подписка →
рендер) полностью рабочий и живьём подтверждён. Не хватает: (а) клика
на строке устройства — это была недоработка САМОГО implementation plan
(Task 6 Step 3 явно скопировал «rendering only», wiring кликов остался
не расписанным конкретным шагом — моя ошибка при написании плана, не
исполнителя, который сделал ровно то, что было велено), (б) починки
`eject`-object_path. Заведён **T008** на оба пункта отдельным
follow-up. T003 → `done/`, отчёт → `report-log/`.

Изменены:
- `crates/chronos-fm-services/src/chronos_fm_services.rs` (+ `pub mod devices;`)
- `crates/chronos-fm-services/Cargo.toml` (+ `zbus`, `async-trait`, `tokio` dev)
- `crates/chronos-fm-ui/src/chronos_fm_ui.rs` (+ `pub mod devices_store;`)
- `crates/chronos-fm-ui/Cargo.toml` (+ `chronos-fm-services`, `async-trait` dev)
- `crates/chronos-fm/src/app.rs` (+ `spawn_device_hotplug_watcher`, `refresh_devices`, вызов `init` + watcher)
- `crates/chronos-fm/Cargo.toml` (+ `zbus`, `futures`)
- `crates/chronos-fm-pages/src/explorer/view/sidebar.rs` (+ `render_devices_section` + вызов из `render`)
- `Cargo.lock`
