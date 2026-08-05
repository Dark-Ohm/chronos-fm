# T003 — съёмные носители: Devices-панель (udisks2/zbus, v1)

**Приоритет:** P1 — первый гэп до Dolphin-паритета (без монтирования
съёмных носителей ежедневно пользоваться нельзя).
**Роль:** реализация по готовому implementation plan, TDD, самодостаточное
задание.
**Источник:** design spec `docs/superpowers/specs/2026-08-06-removable
-media-mount.md` + implementation plan `docs/superpowers/plans/2026-08
-06-removable-media-mount.md` (6 тасков, каждый с полным кодом,
TDD-циклом RED/GREEN, точными командами и коммит-сообщениями — план
самодостаточен, читать его целиком перед началом, этот тикет не
дублирует код).

## Контекст

T001 (тема) и T002 (elevated_card/section_header) приняты и в `done/`.
Это следующий шаг к «Chronos-FM на равне с Dolphin» — Devices-секция в
sidebar, живой hotplug (voткнул флешку → появилась сама), mount/unmount/
eject. Полный дизайн и границы v1 (что НЕ входит — LUKS, форматирование,
прожиг, сетевые протоколы) — в spec, не повторяю здесь.

## Что нужно

Пройти все 6 тасков implementation plan **по порядку** (зависимости
последовательные — Task 2 использует Device из Task 1, Task 3 использует
DeviceBackend из Task 2, и т.д.):

1. `Device` модель + чистый парсер `ManagedObjects` → `Vec<Device>`
   (zero-dependency от zbus, TDD с готовыми тестами в плане).
2. `DeviceBackend` трейт + `UDisks2Backend` (реальный zbus-клиент) +
   `FakeBackend` для тестов.
3. `DeviceStore` — GPUI Global (тот же паттерн, что `FileClipboard` из
   b1, если он уже смёржен — не обязательная зависимость, просто
   стилистическая аналогия).
4. mount/unmount actions с surfacing ошибок в `DeviceStore.last_error`.
5. Hotplug-подписка на `InterfacesAdded`/`InterfacesRemoved`, wiring в
   `crates/chronos-fm/src/app.rs`.
6. Sidebar UI — секция «Devices» на T002-паттернах (`elevated_card`+
   `section_header`), под секцией «Folders».

**Важно (см. план, "Note for the implementer" в Task 2 и 5):** точные
имена методов zbus-байндинга (`Proxy`/`ObjectManagerProxy`/
`body().deserialize()`) могут отличаться от версии, которую резолвит
`cargo add zbus` на момент реализации — сам D-Bus протокол udisks2
стабильный (не гадание), но Rust-обёртка дрейфует между мажорками.
Сверяться с `cargo doc -p zbus --open` при реализации, план явно об
этом предупреждает — это не повод останавливаться, просто подправить
имена методов под резолвнутую версию.

## Зона файлов

Из плана: `crates/chronos-fm-services/src/devices/` (новый),
`crates/chronos-fm-ui/src/devices_store.rs` (новый),
`crates/chronos-fm/src/app.rs` (точечно), `crates/chronos-fm-pages/src/
explorer/view/sidebar.rs` (точечно, секция под «Folders»). Не пересекается
с b1-b4 (`row.rs`/`list.rs`/`grid.rs`/`listing.rs`/`clipboard.rs`/
`rename.rs`/`file_ops.rs`) и не пересекается с `theme.rs`/`patterns.rs`
(T001/T002, только читает их публичный API).

## Верификация

Полный список — в конце implementation plan, "Final Live Verification":
`cargo build --workspace`+`cargo test --workspace` чисто, живая проверка
с реальной USB-флешкой (вставить без перезапуска приложения — секция
должна появиться сама через hotplug-сигнал; клик монтирует и
переходит; `lsblk`/`findmnt` подтверждают точку монтирования; eject
размонтирует, подтверждено `lsblk`), grim-кадры обеих тем.

## Отчёт

`docs/orchestration/tasks/report/T003-removable-media-devices-panel-report.md`
(inbox). Приёмка — архитектор лично (грепы/дифф/build/test + живой
grim/lsblk с реальным носителем — отчёту на слово не верить, та же
дисциплина, что T001/T002). Принят → `report-log/`, тикет → `done/`.
Отклонён → `rejected/`.

## Коммит

По одному коммиту на таск, сообщения — в самом плане (например `services:
device model + pure udisks2 ManagedObjects parser` для Task 1). Не
сквошить в один коммит — план рассчитан на TDD-историю по шагам.
