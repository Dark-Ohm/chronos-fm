# T019 — Нет ассетов иконок устройств: невидимые кнопки + ERROR каждый кадр

**Приоритет:** P2 — кнопки unmount/eject в сайдбаре невидимы, лог
засоряется двумя строками ERROR на каждый отрисованный кадр.
**Статус:** баг с установленным корнем, правка — добавить два SVG.
**Источник:** живой смок релизного бинаря 2026-08-06, приёмка T015.

## Симптом

В логе запуска, повторяется на каждый кадр:

```
ERROR could not find asset at path "icons/hard-drive.svg"
ERROR could not find asset at path "icons/arrow-up.svg"
```

В сайдбаре у записей Ventoy / VTOYEFI не отрисованы ни иконка носителя,
ни кнопка eject — под именем тома видно только имя, путь (обрезанный
краем карточки) и одинокий символ `—`.

## Корень

`crates/chronos-fm-ui/assets/icons/` содержит:

```
arrow-right  case-sensitive  chevron-down  chevron-left  chevron-right
circle-user  circle-x  clock  close  database  file  folder
gallery-vertical-end  github  home  house  layout-dashboard  loader
minus  palette  panel-bottom-open  plus  replace  search  settings
square-terminal  star  trash-2
```

`hard-drive.svg` и `arrow-up.svg` отсутствуют. T015 (`9403f57`) завёл
`Icon(HardDrive)` для строки устройства и `Icon(ArrowUp)` для eject, не
добавив сами файлы. `minus.svg` на месте — значит unmount-иконка, скорее
всего, рисуется, а eject нет; проверить живьём на смонтированном томе.

Отчёт `T015-sidebar-redesign-report.md` ставит галочку «Иконки действий
не упираются в край ✅» — она получена чтением кода, живой прогон в том
же отчёте помечен как не проводившийся.

## Что нужно

1. Добавить `hard-drive.svg` и `arrow-up.svg` в
   `crates/chronos-fm-ui/assets/icons/` (набор — lucide, как остальные).
2. Проверить весь список `Icon(...)`, используемых в UI, против
   содержимого каталога ассетов — молчаливо отсутствующих может быть
   больше. Грепнуть конструкторы иконок и сверить с `ls assets/icons`.
3. Рассмотреть fail-fast: отсутствующий ассет сейчас деградирует в
   ERROR-спам по кадру. Как минимум — логировать один раз на путь, а не
   на каждый кадр (это ещё и перф-нагрузка, см. T014).

## Побочно (в этот же заход, если дёшево)

- Пути устройств в карточке Places обрезаются краем без эллипсиса:
  `/run/media/neo/Vento`, `/run/media/neo/VTOY`.
- Под каждым устройством висит одинокий `—` — предположительно
  плейсхолдер размера/свободного места, который не заполняется.

## Тесты

Юнит, проходящий по списку используемых имён иконок и проверяющий, что
для каждого есть файл в `assets/icons/`. Это ловит весь класс дефекта, а
не два конкретных файла.

## Зона файлов

`crates/chronos-fm-ui/assets/icons/` (новые SVG),
`crates/chronos-fm-pages/src/explorer/view/sidebar.rs` (эллипсис пути, `—`).

## Верификация

- Живой прогон: лог запуска без `could not find asset`; иконка носителя и
  кнопка eject видны у смонтированного тома.
- `cargo test --workspace` зелёный, новый тест виден в счётчике.

## Отчёт

`docs/orchestration/tasks/report/T019-device-icon-assets-report.md`.
Приёмка — архитектор лично. Принят → `report-log/`, тикет → `done/`.
