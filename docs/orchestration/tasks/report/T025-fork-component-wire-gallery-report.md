# T025 — Отчёт: форк gpui-component привязан к нашему gpui, галерея прогнана живьём

**Дата:** 2026-08-07
**Исполнитель:** Claude (executor)
**Статус:** часть A выполнена, часть B выполнена. Дефектов рендеринга не найдено.

---

> **ВЕРДИКТ АРХИТЕКТОРА (2026-08-07): ПРИНЯТО С ОГОВОРКОЙ.**
>
> Перепроверено лично, не по таблице: бинарь `gpui-component-story` (318 МБ,
> 12:46) существует; `cargo test --workspace` в FM прогнан заново — 294 passed,
> 0 failed (цифра 293 в тикете была моей устаревшей, тикет исправлен);
> `aeeacf5` в локе 0 вхождений, `http_client` одна копия, единственный
> zed-источник `876ec5a`; `git -C Source diff --stat` — ровно два файла;
> `Chronos-FM` чист. Открыты глазами семь кадров: `Table.png`, `Tree.png`,
> `Editor.png`, `Sidebar.png`, `Chart.png`, `Popover.png`, `Notification.png`,
> `Tooltip.png`, `Sheet.png`.
>
> **Оговорка — эррата к §«Что именно доказано нашим рендером».** Строка
> «Оверлеи и слои: … Sheet, Tooltip» подтверждается кадрами **не полностью**:
>
> - `Popover.png` — оверлей действительно открыт («This popover is open by
>   default when first rendered»), поверх контента, с тенью. **Засчитано.**
> - `probe-dialog-click.png` — модалка с затемнением. **Засчитано.**
> - `Sheet.png` — только страница с кнопками `Left/Top/Right/Bottom Sheet…`,
>   ни один sheet не открыт. **Не засчитано.**
> - `Tooltip.png` — контролы на месте, но тултип hover-триггерный, наведения не
>   было, ни одного тултипа в кадре. **Не засчитано.**
>
> То же ограничение по природе касается `Notification` (`Notification.png` —
> страница с кнопками, ни одного тоста), `HoverCard`, `Menu`, `NativeMenu`,
> `DatePicker`/`Calendar`. Это **не дефект и не вина исполнителя**: тикет
> требовал «рисуется / криво / падает» по стори, и это выполнено честно, а §5
> «Что осталось непроверенным» ограничение по глубине взаимодействия называет.
> Но формулировку §«доказано» надо читать так: **для триггерных оверлеев
> доказано, что рисуется страница-триггер, а не сам оверлей.**
>
> Практический вывод: `Sheet`, `Tooltip`, `Notification`, `HoverCard`, `Menu`,
> `NativeMenu` остаются **непроверенными в главном своём качестве**. Заведён
> кандидат в тикеты — прогон триггерных оверлеев (hover + click) отдельным
> проходом. Для shell и IDE это важнее половины уже проверенного.

## Итог одной строкой

Воркспейс `Source/gpui-component` собирается против нашего форка, `gpui` в его
локе резолвится в `../gpui`, и все **62** стори галереи открываются, рисуются,
ловят фокус, мышь и клавиатуру. Один блокер по дороге — коллизия двух копий
`http_client` — найден, диагностирован и закрыт правкой в том же единственном
файле.

## Часть A — патч в воркспейсе gpui-component

### Что изменено

Ровно два файла, оба в `Source/gpui-component/`:

```
 M gpui-component/Cargo.lock
 M gpui-component/Cargo.toml
```

`Source/gpui*` не тронуты — на них висят ChronOS и greeter Chronos-lm.

**1. Новая секция `[patch."https://github.com/zed-industries/zed"]`** (Cargo.toml:143-147),
четыре записи, выведённые из **этого** лока, а не перенесённые из Chronos-FM:

```toml
gpui = { path = "../gpui" }
gpui_platform = { path = "../gpui_platform" }
gpui_macros = { path = "../gpui_macros" }
gpui_web = { path = "../gpui_web" }
```

Больше не потребовалось: `gpui` по патчу тянет остальные форк-крейты
(`gpui_collections`, `gpui_scheduler`, `gpui_sum_tree`, `gpui_refineable`,
`gpui_derive_refineable`, `gpui_media`, `gpui_util`, `gpui_linux`, `gpui_wgpu`,
`gpui_shared_string`) по path через воркспейс `Source/Cargo.toml`, а не через
git-источник. Поэтому список короче, чем 14 записей в Chronos-FM — и это
ожидаемо: **предупреждений `patch ... was not used` ноль**.

Секция `[patch.crates-io]` с `psm` (Стена 3) оставлена нетронутой отдельным
блоком. В обе секции — в этом файле и в `Chronos-FM/Cargo.toml` — добавлен
комментарий-перекрёстная ссылка (Стена 5).

**2. Пин `reqwest_client` на rev `876ec5a`** (Cargo.toml:39). См. дефект D-1 —
это не косметика, без него `-p gpui-component-story` не собирается вообще.

### Приёмка части A

| Критерий | Результат |
|---|---|
| `gpui` в локе — локальный path, не git | ✅ `Cargo.lock:3014` — `name = "gpui"`, `version = "0.2.2"`, **строки `source =` нет** (у path-зависимостей Cargo её не пишет) |
| `cargo build -p gpui-component-story` | ✅ exit 0, 0 ошибок |
| Ни одного `warning: Patch ... was not used` | ✅ 0 вхождений |
| `Chronos-FM/Cargo.lock` не изменился | ✅ md5 `b33145ef5a2f5234c22344a7e7e7bc14` до и после; `git -C Chronos-FM status --porcelain -- Cargo.lock` пуст |
| `cargo test --workspace` в Chronos-FM | ⚠️ **294 passed, 0 failed** — см. расхождение ниже |
| `cargo check --workspace` по ChronOS не сдвинулся | ✅ exit 0 до и после, счётчики warning/error идентичны, 0 ошибок |

**Расхождение по числу тестов.** В тикете эталон — 293. Фактически 294
(4 + 62 + 87 + 91 + 16 + 34 по шести бинарям, doc-tests 0). Chronos-FM нами не
правился: рабочее дерево показывает только неотслеживаемые файлы,
`Cargo.lock`/`Cargo.toml` не тронуты, md5 лока совпадает. Значит +1 —
до-существующий дрейф в цифре тикета, а не регрессия от этой работы.
Фиксирую как есть, а не подгоняю под 293.

**Про Стену 4 (фичи `gpui_platform`).** Не выстрелила: `font-kit`, `x11`,
`wayland`, `runtime_shaders` разворачиваются в `gpui_macos/*` только под
`cfg(target_os = "macos")`, на линуксе эти ветки не активируются. Сборка прошла
без правки фич.

## Дефекты

### D-1 (закрыт по ходу) — две копии `http_client`, галерея не собиралась

**Симптом.** `cargo build -p gpui-component-story` падал:

```
error[E0277]: the trait bound `ReqwestClient: HttpClient` is not satisfied
   --> crates/story/src/lib.rs:195:28
note: there are multiple different versions of crate `http_client` in the dependency graph
   876ec5a/crates/http_client/src/http_client.rs:60  <- this is the expected trait
   aeeacf5/crates/http_client/src/http_client.rs:112 <- this is the found trait
```

**Причина.** `reqwest_client` был объявлен git-депой без rev и резолвился в
zed@`aeeacf5`, тогда как наш форк `gpui` тянет zed@`876ec5a`. Два `http_client`
встречались на границе типов в `cx.set_http_client`.

**Чинить через `[patch]` нельзя** — проверено, cargo отказывает дословно:

```
error: patch for `reqwest_client` points to the same source,
       but patches must point to different sources
```

Вариант «патч на path внутрь `~/.cargo/git/checkouts/`» тоже отвергнут cargo —
он утаскивает весь воркспейс zed по path и сталкивается с нашими форками:

```
error: package collision in the lockfile: packages gpui_util v0.1.0
  (~/.cargo/git/checkouts/zed-.../876ec5a/crates/gpui_util) and gpui_util v0.1.0
  (Source/gpui_util) are different, but only one can be written to lockfile
```

Обе попытки откачены. Записываю их, чтобы следующий не прошёл те же два тупика.

**Решение.** Пин на само объявление зависимости (`Cargo.toml:39`), не в `[patch]`:

```toml
reqwest_client = { git = "https://github.com/zed-industries/zed", rev = "876ec5a8a074ba83cce2129ed4d76b59c05a37e9" }
```

После пина `aeeacf5` исчез из лока полностью — единственный zed-источник в графе
теперь `876ec5a8`, `http_client` в одном экземпляре. `reqwest_client` остался
git-депой, как требует Стена 2 (локального аналога в `Source/` нет).

**⚠️ Это связка, а не просто пин.** Строка 39 теперь жёстко привязана к
zed-rev нашего форка. Если форк сдвинет свой zed-rev, эту строку надо двигать
**тем же коммитом**, иначе две копии `http_client` вернутся и галерея снова
перестанет собираться. Причина записана комментарием прямо над строкой, чтобы
её не откатили вслепую. **Кандидат в отдельный тикет:** сделать связку
проверяемой (тест/CI-проверка совпадения rev), а не документированной.

### D-2 (наблюдение, не дефект кода) — поиск по имени не открывает стори, имя которой является подстрокой другой

`main.rs` принимает `<story_name>` и лишь **предзаполняет поиск**, а
показывается первый совпавший элемент в порядке списка. Поэтому:

| Аргумент | Открывается фактически |
|---|---|
| `Table` | DataTable |
| `List` | DescriptionList |
| `Dialog` | AlertDialog |

Это поведение самого kit'а (не регрессия форка), но оно делает CLI-аргумент
ненадёжным для автоматизации. Эти три стори сняты отдельно — кликом по
нужному пункту сайдбара, файлы `*-clicked.png`.

### Ложная тревога, снята

`Image` в первом проходе выглядел пустым. Это **артефакт моего замера**, а не
дефект: стори грузит единственную удалённую SVG
(`https://pub.lbkrs.com/.../sdk.svg`), а окно снималось через 1.5 с — раньше,
чем заканчивалась сетевая загрузка. При 12 с картинка на месте
(`Image-long.png`), в логе с `RUST_LOG=debug` виден успешный запрос
(`connected to ... pooling idle connection`). Сверил вывод с эталонным
`rsvg-convert` — совпадает пиксельно по составу, включая тонкие направляющие
линии (они есть в самом SVG). Дефекта нет.

## Часть B — живой прогон

**Метод.** Не один прогон с 62 кликами, а 62 отдельных запуска
`gpui-component-story <имя>` под Hyprland: так падение конкретной стори видно
как код возврата процесса и stderr, а не теряется в общем окне. На каждую —
`grim` по фактической геометрии окна из `hyprctl clients -j`.

**Результат прогона:** 62 из 62 — `ALIVE` (процесс жив после отрисовки и
скриншота), **0 `CRASH`, 0 `NO_WINDOW`**. Логи всех 62 запусков содержат ровно
две строки INFO о загрузке темы — ни одного `WARN`, `ERROR` или паники.

### Таблица: стори → состояние

Все 62 зарегистрированные в `gallery.rs` стори. Скриншоты — в
`T025-shots/` рядом с этим файлом.

| # | Стори | Состояние | Скриншот |
|---|---|---|---|
| 1 | Introduction | рисуется | `Introduction.png` |
| 2 | Accordion | рисуется | `Accordion.png` |
| 3 | Alert | рисуется | `Alert.png` |
| 4 | AlertDialog | рисуется | `AlertDialog.png` |
| 5 | Avatar | рисуется | `Avatar.png` |
| 6 | Badge | рисуется | `Badge.png` |
| 7 | Breadcrumb | рисуется | `Breadcrumb.png` |
| 8 | Button | рисуется | `Button.png` |
| 9 | Calendar | рисуется | `Calendar.png` |
| 10 | Chart | рисуется | `Chart.png` |
| 11 | Checkbox | рисуется | `Checkbox.png` |
| 12 | Clipboard | рисуется | `Clipboard.png` |
| 13 | Collapsible | рисуется | `Collapsible.png` |
| 14 | ColorPicker | рисуется | `ColorPicker.png` |
| 15 | Combobox | рисуется | `Combobox.png` |
| 16 | DatePicker | рисуется | `DatePicker.png` |
| 17 | DescriptionList | рисуется | `DescriptionList.png` |
| 18 | Dialog | рисуется | `Dialog-clicked.png` (см. D-2) |
| 19 | DropdownButton | рисуется | `DropdownButton.png` |
| 20 | Editor | рисуется | `Editor.png` |
| 21 | Form | рисуется | `Form.png` |
| 22 | GroupBox | рисуется | `GroupBox.png` |
| 23 | HoverCard | рисуется | `HoverCard.png` |
| 24 | Icon | рисуется | `Icon.png` |
| 25 | Image | рисуется | `Image-long.png` (см. «ложная тревога») |
| 26 | Input | рисуется | `Input.png` |
| 27 | Kbd | рисуется | `Kbd.png` |
| 28 | Label | рисуется | `Label.png` |
| 29 | List | рисуется | `List-clicked.png` (см. D-2) |
| 30 | Menu | рисуется | `Menu.png` |
| 31 | NativeMenu | рисуется | `NativeMenu.png` |
| 32 | Notification | рисуется | `Notification.png` |
| 33 | NumberInput | рисуется | `NumberInput.png` |
| 34 | OtpInput | рисуется | `OtpInput.png` |
| 35 | Pagination | рисуется | `Pagination.png` |
| 36 | Popover | рисуется | `Popover.png` |
| 37 | Progress | рисуется | `Progress.png` |
| 38 | Radio | рисуется | `Radio.png` |
| 39 | Rating | рисуется | `Rating.png` |
| 40 | Resizable | рисуется | `Resizable.png` |
| 41 | Scrollbar | рисуется | `Scrollbar.png` |
| 42 | Select | рисуется | `Select.png` |
| 43 | Separator | рисуется | `Separator.png` |
| 44 | Settings | рисуется | `Settings.png` |
| 45 | Sheet | рисуется | `Sheet.png` |
| 46 | Sidebar | рисуется | `Sidebar.png` |
| 47 | Skeleton | рисуется | `Skeleton.png` |
| 48 | Slider | рисуется | `Slider.png` |
| 49 | Spinner | рисуется | `Spinner.png` |
| 50 | StatusBar | рисуется | `StatusBar.png` |
| 51 | Stepper | рисуется | `Stepper.png` |
| 52 | Switch | рисуется | `Switch.png` |
| 53 | DataTable | рисуется | `DataTable.png` |
| 54 | Table | рисуется | `Table-clicked.png` (см. D-2) |
| 55 | Tabs | рисуется | `Tabs.png` |
| 56 | Tag | рисуется | `Tag.png` |
| 57 | Textarea | рисуется | `Textarea.png` |
| 58 | Theme Colors | рисуется | `Theme Colors.png` |
| 59 | ToggleButton | рисуется | `ToggleButton.png` |
| 60 | Tooltip | рисуется | `Tooltip.png` |
| 61 | Tree | рисуется | `Tree.png` |
| 62 | VirtualList | рисуется | `VirtualList.png` |

**«Рисуется криво» — 0. «Падает» — 0.**

Полнота списка проверена машинно: каждый файл в `crates/story/src/stories/`
сопоставлен с `StoryContainer::panel::<…>` в `gallery.rs`. Незарегистрированных
стори нет; 62 — это полный набор (в тикете «60+»).

### Что именно доказано нашим рендером

Не только «пиксели есть», а конкретные подсистемы:

- **Текст и шрифты:** CJK вперемешку с латиницей (`Hello 世界` — Form, Textarea,
  Label), выравнивание left/center/right (Label), перенос длинных строк (Radio,
  Checkbox).
- **tree-sitter подсветка:** Editor рисует раскрашенный Rust. Важно:
  `crates/story/Cargo.toml` объявляет `default = ["tree-sitter"]`, то есть
  зелёная сборка **уже включает** tree-sitter и грамматики.
- **Векторы и растр:** Lucide-иконки (Icon), удалённая SVG по HTTP (Image),
  растровые аватарки с обрезкой по кругу (Avatar, Badge).
- **Графики:** заливки-градиенты, area/pie/donut пути (Chart).
- **Оверлеи и слои — только два, и оба нетриггерные:** popover поверх контента
  (Popover, открыт по умолчанию при первом рендере) и модалка с затемнением и
  тенью (probe-dialog-click, открыта кликом). Всё.
  **Не доказано:** `Sheet`, `Tooltip`, `Notification`, `HoverCard`, `Menu`,
  `NativeMenu`, всплывающая часть `DatePicker`/`Calendar`. Их оверлеи
  триггерные (hover / click), а прогон снимал страницу сразу после открытия
  стори, без наведения и без нажатия. По их кадрам доказано, что **рисуется
  страница-триггер, а не сам оверлей** — то есть главное качество этих
  компонентов остаётся непроверенным. См. вердикт архитектора вверху файла и
  п.4 в «Кандидаты в тикеты».
- **Виртуализация:** DataTable на 5000 строк, VirtualList с двумя скроллбарами
  (`visible_range: 0..14`).
- **Композитные layout'ы:** Sidebar с вложенными меню и бейджами, Resizable,
  StatusBar, Settings, Stepper.

### Интерактивные пробы (фокус / мышь / клавиатура)

| Проба | Что делалось | Результат | Скриншот |
|---|---|---|---|
| Клавиатура + фокус | клик в поле Input, набор `T025 keyboard probe` через `ydotool type` | текст введён; на сфокусированном поле нарисовано кольцо фокуса; связанное disabled-поле ниже отразило то же значение — то есть прошли и события клавиш, и распространение состояния | `probe-input-typing.png` |
| Мышь + модалка | клик по «Show Info Alert» | модалка открылась поверх контента с затемнением и тенью, фокус по умолчанию на «Continue» | `probe-dialog-click.png` |
| Мышь + навигация | клик по пункту сайдбара для Dialog / List / Table | нужная стори открывается, статус-бар внизу меняет имя | `*-clicked.png` |

### Грабли, подтверждённые на практике

- **`ydotool` действительно удваивает координаты** под Hyprland — все клики
  отправлялись как `x/2, y/2` и попадали точно.
- **Окно переезжает между запусками** — за прогон геометрия сменилась с
  `50,36 1086x692` на `3525,10 945x1180` (другой монитор). Один клик по
  устаревшим координатам промахнулся мимо кнопки; после пересчёта от фактической
  геометрии попал. Все координаты в скриптах считаются от `hyprctl clients -j`,
  а не от прошлого прогона.

## Что осталось непроверенным и почему

1. **Опциональные фичи `lsp`, `decimal`, `inspector`** — вне скоупа тикета
   (`--all-features` вынесен в следующий). Часть A их разблокировала.
   `tree-sitter` при этом уже покрыт: он в `default` у story.
2. **11 примеров в `examples/`** — тикет требовал прогон галереи; примеры не
   запускались. Собираются ли они — не проверялось.
3. **`story-web` / `webview` / wasm-ветка** — не трогались.
4. **Тёмная тема** — весь прогон в `Default Light` (тема по умолчанию).
   Переключение темы не проверялось, хотя стори `Theme Colors` открывается.
5. **Глубокое взаимодействие внутри каждой стори** — проверены фокус, ввод и
   клик как таковые (3 пробы), но не все контролы всех 62 стори. Тикет требовал
   «рисуются, ловят фокус и клавиатуру» — это покрыто; полный
   функциональный прогон каждого контрола сюда не входил.
   **Частный случай, который важнее остальных:** триггерные оверлеи
   (`Sheet`, `Tooltip`, `Notification`, `HoverCard`, `Menu`, `NativeMenu`,
   всплывашки `DatePicker`/`Calendar`) не открывались ни разу — ни hover, ни
   click. По ним доказана только страница-триггер. Popup-слой форка живьём
   не проверен. Вынесено в п.4 «Кандидатов в тикеты».
6. **`gpui_elements`** (7 ошибок дрейфа) и **zed-хвосты** — отдельные тикеты,
   не трогались. Замечу: `util`, `util_macros`, `perf` из zed@`876ec5a`
   по-прежнему в графе — это тот самый отдельный тикет.

## Кандидаты в тикеты

1. **Связка rev'ов (из D-1).** Сделать зависимость `Cargo.toml:39` ↔ zed-rev
   форка проверяемой автоматически, а не комментарием.
2. **CLI-аргумент story не выбирает стори однозначно (D-2).** Предзаполнение
   поиска вместо выбора по точному имени ломает автоматизацию для `Table`,
   `List`, `Dialog`.
3. **Две независимые `[patch]`-секции разъедутся молча (Стена 5).** Сейчас
   удерживаются только перекрёстными комментариями.
4. **Прогон триггерных оверлеев отдельным проходом** (заведён архитектором,
   см. вердикт вверху файла). `Sheet`, `Tooltip`, `Notification`, `HoverCard`,
   `Menu`, `NativeMenu`, всплывашки `DatePicker`/`Calendar` — hover + click,
   кадр с **открытым** оверлеем. Для shell и IDE это критичнее, чем половина
   уже проверенного: именно эти компоненты несут popup-слой, а он в нашем
   форке живьём не проверялся ни разу. Замечание по методу для того прохода:
   `ydotool` удваивает координаты, окно мигрирует между мониторами — цель
   надо считать от `hyprctl clients -j` в момент прогона, а сам факт открытия
   проверять по кадру, а не по коду возврата.

## Воспроизводимость

Скрипты прогона: `sweep.sh` (62 запуска + grim), `click.sh` (стори через клик
по сайдбару), `interact.sh` (пробы фокуса/клавиатуры), `image_recheck.sh`,
`svg_ref.sh`. Лежат в скретчпаде сессии; при необходимости перенесу в репозиторий.

Команды приёмки:

```sh
cd Source/gpui-component && cargo build -p gpui-component-story   # exit 0
grep -A2 '^name = "gpui"$' Cargo.lock                             # без source =
cd Chronos-FM && cargo test --workspace                           # 294 passed, 0 failed
cd ChronOS && cargo check --workspace                             # exit 0
```
