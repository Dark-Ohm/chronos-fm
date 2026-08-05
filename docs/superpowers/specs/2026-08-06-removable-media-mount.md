# Removable Media — Devices Panel (v1: list + mount/unmount/eject)

**Дата:** 2026-08-06. **Автор:** Архитектор.

## Контекст и цель

Цель сессии — довести Chronos-FM до паритета не только с Thunar, но с
Dolphin. Из 6 неспеканных гэпов (съёмные носители, архивы-как-папка,
properties/permissions, «open with», batch rename, встроенный терминал)
этот spec закрывает первый и самый критичный: **без монтирования
съёмных носителей файловым менеджером ежедневно пользоваться нельзя**
(USB-флешка воткнута — её надо увидеть и открыть, не лезть в `mount(8)`
руками). Остальные 5 — backlog, см. низ файла, не в этом spec.

## Архитектура

### 1. D-Bus клиент — `crates/chronos-fm-services/src/devices/`

Новый модуль, тот же уровень, что `fs/` и `search/`. Библиотека —
`zbus` (не `dbus-rs`) — не тянет `tokio` как обязательную зависимость
(своя `async-io`/`smol`-подложка), совместимо с уже пройденной для
этого проекта дисциплиной «tokio вычищен из app core, P2-веха» (см.
`crates/chronos-fm-services/src/search/engine.rs` — комментарий про
`cx.background_spawn` вместо `tokio::task`).

Интерфейс — `org.freedesktop.UDisks2` (system bus, не session — udisks2
всегда на system):

- `org.freedesktop.UDisks2.ObjectManager.GetManagedObjects` — снимок
  всех `Drive`/`Block`/`Filesystem` объектов при старте.
- `org.freedesktop.UDisks2.Filesystem.Mount()` / `.Unmount()`.
- `org.freedesktop.UDisks2.Drive.Eject()` (для реально съёмных
  приводов; для USB-накопителей unmount уже физически безопасен).
- Подписка на `InterfacesAdded`/`InterfacesRemoved`
  (`org.freedesktop.DBus.ObjectManager`) — живой hotplug без ручного
  refresh (воткнул флешку → она сама появляется в списке).

Публичный тип — `DeviceStore` (GPUI global, тот же паттерн, что
`FileClipboard` из b1 — `init(cx)`/`.update()`/read через
`cx.global::<DeviceStore>()`), поле — `Vec<Device>`:

```rust
pub struct Device {
    pub object_path: OwnedObjectPath,   // udisks2 id, стабильный ключ
    pub label: String,                  // filesystem label или device name
    pub device_node: String,            // /dev/sdb1 — для лога/диагностики
    pub mount_point: Option<PathBuf>,   // None = не смонтирован
    pub is_removable: bool,             // Drive.Removable — фильтр съёмности
    pub size_bytes: u64,
}
```

Подключение к D-Bus и первичный опрос — на `cx.background_spawn`, не
блокировать GPUI main thread (тот же принцип, что `search::engine`).
Сигналы hotplug — отдельная долгоживущая background-задача, шлёт
обновления в `DeviceStore` через `cx.update(...)` (или канал +
`cx.spawn`, как уже принято в проекте для похожих long-running
подписок — сверить точный идиом в `search/indexer.rs` при реализации).

### 2. Фильтр v1

Показываем: `Drive.Removable == true` (USB/SD-карты/внешние диски) **и**
внутренние разделы с `Filesystem` интерфейсом, у которых
`HintSystem == false` (исключает `/boot`, `/`, swap, LUKS-контейнеры до
разблокировки — те просто не показываем в v1, не крашим и не путаем).
LUKS-разблокировка — backlog, не v1.

### 3. Sidebar — секция «Devices»

`crates/chronos-fm-pages/src/explorer/view/sidebar.rs`, сразу под
секцией «Folders» (та же T002-связка `elevated_card`+`section_header`,
переиспользуется как есть, не форкать). Один пункт списка на `Device`:

- Иконка тома (USB-стик или диск по `is_removable`).
- `label` (или `device_node`, если `label` пустая — не все флешки
  форматированы с меткой).
- Немонтированный → клик = mount() + сразу navigate в
  `mount_point` (после успешного mount).
- Смонтированный → клик = просто navigate.
- Отдельная иконка справа (eject/unmount) на каждой строке — не
  требует захода внутрь тома, чтобы безопасно вынуть.
- Live-обновление списка при hotplug — без перезапуска приложения и
  без ручного refresh sidebar.

### 4. Ошибки

`Mount()`/`Unmount()` от udisks2 может вернуть D-Bus error (permission
denied — polkit-политика на этой машине, устройство busy — открыт файл
с него и т.п.). Показать как inline toast/status-bar сообщение (тот же
канал, что использует остальной UI для ошибок — сверить
`config_status`-паттерн в `root.rs::apply_config`), не паниковать и не
глушить молча.

## Вне скоупа v1

- LUKS-разблокировка зашифрованных томов (`UDisks2.Encrypted`).
- Форматирование/переразбиение (`UDisks2.Block.Format`).
- Прожиг оптических дисков.
- Сетевые протоколы (SFTP/SMB/MTP через `gvfs`/`udisks2`-аналоги) —
  концептуально похоже, но другой D-Bus интерфейс/крейт, отдельная
  задача, не путать с этим spec.
- Обои/автозапуск при вставке носителя (`x-content/*` handler,
  «что делать с новой SD-картой с фото») — Nautilus/Dolphin имеют,
  сознательно не в v1, слишком нишево для MVP.

## Верификация

- `cargo build --workspace` + `cargo test --workspace` чисто.
- Живой прогон: реальная USB-флешка воткнута архитектором физически —
  секция «Devices» показывает её без перезапуска приложения (hotplug
  сигнал сработал), клик монтирует и переходит в неё, `ls
  /run/media/$USER/...` (или где реально монтирует udisks2 на этой
  системе — сверить) подтверждает точку монтирования, клик eject —
  флешка размонтирована, `lsblk` подтверждает.
- Grim-кадры секции «Devices» до/после вставки, обе темы (dark/light —
  палитра из T001, паттерн из T002, эта секция не должна выглядеть
  инородно).

## Коммит

`services+ui : removable media devices panel via udisks2/zbus (v1: list
+ mount/unmount/eject)` — после реализации по плану, не в этом spec.

---

## Backlog — остальные 5 гэпов до Dolphin-паритета (без спеков, приоритет сверху вниз)

1. **Archive browsing** — открыть `.zip`/`.tar.*` как виртуальную папку
   без распаковки на диск (аналог KIO `tar:/`/`zip:/`). Второй по
   критичности после съёмных носителей — частый повседневный кейс.
2. **Properties / Permissions диалог** — правый клик → Properties:
   размер (рекурсивно для папок), владелец, права `chmod` в
   человекочитаемом + octal виде.
3. **Batch rename** — множественный выбор → паттерн-переименование
   (`{name}_{n}.{ext}` и т.п.). b1 даёт только single inline rename —
   рабочая замена, не финал.
4. **«Open with» / service menu** — выбор приложения из
   `.desktop`-реестра (`xdg-mime`/`freedesktop.org` Shared MIME
   database), не только дефолтный `xdg-open`.
5. **Встроенная терминальная панель** (F4-аналог) — открывает shell в
   текущей директории эксплорера, без выхода в отдельное окно.
