# T025 — Форк: привязать gpui-component к нашему gpui и прогнать галерею живьём

**Приоритет:** P0 для экосистемы. Chronos-FM использует 10 модулей kit'а
из 55; остальные 45 нужны shell / IDE / login-manager / text-editor и
против нашего форка **никогда не проверялись в рантайме**.
**Скоуп:** каталог `/home/neo/projects/chronos-ecosystem/Source/`.
Репозиторий форка — `git@github.com:Dark-Ohm/Chronos-GPUI.git`.
**Chronos-FM в этом тикете не правится вообще.**

## Зачем, если всё компилируется

Компилируется — да. `cargo check -p gpui-component` из воркспейса
Chronos-FM зелёный за 7.5 с, все 55 модулей собираются против форка.
Но это единственное, что мы знаем, и знание это узкое:

1. **`Source/gpui-component` — отдельный воркспейс.** Он в `exclude`
   корневого `Source/Cargo.toml:35`, у него свой `Cargo.lock`, и в нём
   `gpui` резолвится в **upstream zed**:

   ```
   Source/gpui-component/Cargo.lock:2876
   source = "git+https://github.com/zed-industries/zed#aeeacf5439b2d30d01e38d65d767e6f31b255ecc"
   ```

   Секция `[patch]` живёт только в `Chronos-FM/Cargo.toml:65`. Значит
   story-галерея (60+ стори: `table`, `tree`, `sidebar`, `tabs`,
   `menu`, `notification`, `dock`, `popover`, `tooltip`, `chart`,
   `editor`, `calendar` …) и 11 примеров в `examples/` собираются
   против **чужого** gpui и про наш форк не доказывают ничего.

2. **Опциональные фичи включить неоткуда.** Из воркспейса FM:
   `cargo check -p gpui-component --features tree-sitter` →
   `error: cannot specify features for packages outside of workspace`.
   Речь про `tree-sitter` + 34 грамматики, `lsp`, `decimal`,
   `inspector` — фундамент IDE и текст-редактора. Ни разу не собирались
   против форка.

3. **Дрифт — не гипотеза.** `gpui_elements` уже исключён из
   `Source/Cargo.toml:31-33` с семью ошибками API-дрифта
   (`ActionBindingCollection`, методы `TextLayout`, отсутствующий
   `is_nearly_eq`).

## Часть A — патч в собственном воркспейсе gpui-component

**Где:** `Source/gpui-component/Cargo.toml`

Добавить секцию `[patch."https://github.com/zed-industries/zed"]`,
перенаправляющую zed-овские git-депы на локальные пути форка. Пути
относительно этого файла: `Source/gpui-component/Cargo.toml` → `../gpui`
указывает на `Source/gpui`.

### Стены, о которые бьются с первого захода

**Стена 1 — список патчей нельзя скопировать из Chronos-FM.** В
`Chronos-FM/Cargo.toml:65-79` перечислено 14 крейтов, и там же в
комментарии сказано почему именно эти: *«Only packages that appear under
that git source in our lock graph are listed (extra entries produce
"patch was not used" warnings)»*. У воркспейса gpui-component **другой**
граф — в нём есть `story`, `story-web`, `webview` и 11 примеров, которых
в графе FM нет. Список выводить из **его** `Cargo.lock`, а не переносить
готовый. Лишняя запись → warning `patch was not used`, недостающая →
две копии `gpui` и ошибка унификации.

**Стена 2 — `reqwest_client` не патчить.** `gpui-component/Cargo.toml:39`
объявляет `reqwest_client = { git = zed-industries/zed }`, и **локального
аналога в `Source/` нет** (сверь `ls Source/` — там 20 `gpui_*` крейтов,
`reqwest_client` среди них отсутствует). Он должен остаться git-депом.
То же касается `reqwest` (`Cargo.toml:42`, форк zed) — не трогать.

**Стена 3 — секция `[patch.crates-io]` уже существует**
(`gpui-component/Cargo.toml:128`, `psm` с git). Новая секция — **другой**
ключ (`[patch."https://github.com/zed-industries/zed"]`), добавляется
отдельным блоком. Не сливать их и не переписывать существующий.

**Стена 4 — фичи `gpui_platform`.** `gpui-component/Cargo.toml:36`
запрашивает `features = ["font-kit", "x11", "wayland", "runtime_shaders"]`.
Все четыре в локальном `Source/gpui_platform/Cargo.toml` есть — но
`font-kit` и `runtime_shaders` там разворачиваются в `gpui_macos/*`,
которого на линуксе нет. Если сборка упадёт именно на этом — это
находка, фиксируй как дефект форка, **не глуши правкой фич в
gpui-component**.

**Стена 5 — два независимых патча теперь надо держать синхронными.**
После этого тикета `[patch]` существует в двух местах: `Chronos-FM/Cargo.toml`
и `Source/gpui-component/Cargo.toml`. Они не связаны и разъедутся молча.
В обе секции добавить комментарий-перекрёстную ссылку на второй файл.

### Приёмка части A

- В `Source/gpui-component/Cargo.lock` у пакета `gpui` источник —
  локальный path, а не `git+https://github.com/zed-industries/zed`.
  Показать строку в отчёте.
- `cargo build -p gpui-component-story` из `Source/gpui-component/`
  завершается успешно. Если нет — см. часть B, это не провал.
- `cargo build` не печатает ни одного `warning: Patch ... was not used`.
- **`Chronos-FM/Cargo.lock` не изменился** (`git -C Chronos-FM status
  --porcelain` пуст по этому файлу) и `cargo test --workspace` в
  Chronos-FM даёт прежние **294 passed, 0 failed** (базовая линия
  перемерена 2026-08-07; цифра 293 в моих старых записях устарела —
  сверяйся с 294). Регрессия в FM = провал тикета.
- `cargo check --workspace` по ChronOS не сдвинулся — см. «Соседи по
  `Source/`» ниже.

## Часть B — живой прогон галереи

`cargo run -p gpui-component-story` под Hyprland. Пройти по всем стори,
на каждую — `grim`.

Это и есть «подключить»: не аудит исходников, а экран, доказывающий,
что `table`, `tree`, `sidebar`, `dock`, `tabs`, `notification`,
`popover`, `tooltip`, `chart`, `editor` рисуются нашим рендером, ловят
фокус и клавиатуру.

**Что НЕ делать:** не чинить найденное на ходу. Сломанное — находка
этого тикета, а не его работа. Каждый дефект — отдельная строка отчёта
с именем стори и скриншотом. Иначе тикет расползётся и не закроется.

### Грабли живого прогона (проверены 2026-08-07)

- `ydotool` под Hyprland **удваивает координаты** — целься в половину.
- Окно может переехать между запусками; координаты пересчитывать от
  фактической геометрии, а не от прошлого прогона.
- Доказательство — файл `grim`, а не фраза «выглядит нормально».
- Если понадобится release-сборка: голый `cargo build --release`
  GUI-бинарь может не собрать (в Chronos-FM это стоило захода —
  GUI-крейты вне `default-members`). Всегда `-p <имя пакета>` и сверка
  `stat -c '%y'` артефакта.

### Приёмка части B

Выполнимый критерий — **не** «все 60 стори работают». Требуется:

- Таблица «стори → рисуется / рисуется криво / падает» по **каждой**
  стори из `crates/story/src/stories/` (список полный, пропуски
  недопустимы; если стори не открылась — так и писать).
- Скриншот-доказательство на каждую строку таблицы.
- Отдельным списком — дефекты, пригодные к заведению тикетами.
- Явно назвать, что осталось непроверенным и почему.

## Соседи по `Source/`

`Source/gpui-component/crates/ui/` собирает не только FM, но и ChronOS
(по path, с `default-features = false`). Отсюда два правила:

1. **Правим ровно один файл** — `Source/gpui-component/Cargo.toml`
   (плюс его `Cargo.lock` как следствие). `Source/gpui*` не трогаем:
   на них висят ChronOS и greeter Chronos-lm.
2. **Дефолтный набор фич `crates/ui/Cargo.toml:2` не менять** — ChronOS
   берёт kit с `default-features = false` и добирает `markdown` своей
   фичей. Не хватает фичи галерее — включать её в манифесте story.

Секция `[patch]` в неглавном манифесте Cargo'й игнорируется, так что на
сборки FM и ChronOS часть A не влияет — она работает только при сборке
воркспейса gpui-component отдельно. Риск не в ней, а в части B: правка
в `crates/ui/src/`, сделанная по дороге, прилетает в ChronOS сразу.

Проверка после работы — `cargo check --workspace` по ChronOS. Красный
результат до начала работы фиксируем как есть и не чиним: тикет про
форк.

## Не в скоупе

- `cargo check -p gpui-component --all-features` (tree-sitter + 34
  грамматики, lsp, decimal, inspector) — **следующий тикет**, он
  разблокируется частью A. Жду дрифта именно там: `highlighter` +
  `editor` — самая тяжёлая часть kit'а, FM её не касается вообще.
- Расконсервация `gpui_elements` (7 ошибок дрифта) — отдельный тикет.
- Zed-хвосты в графе (`util`, `util_macros`, `perf`, `collections`,
  `http_client` из `zed-industries/zed@876ec5a`) — отдельный тикет,
  этот их не трогает.
- Любые правки в `Chronos-FM/`.

## Гигиена коммитов

Стейджить **пофайлово**, не по каталогу. 2026-08-07 `git add` по
каталогу утащил в чужой коммит незакоммиченный каталог `hold/`; откат
стоил отдельного коммита. В `Source/` рядом могут работать другие
агенты.

## Отчёт

`docs/orchestration/tasks/report/T025-fork-component-wire-gallery-report.md`
(в репозитории Chronos-FM, рядом с остальными). Приёмка — архитектор
лично, включая личный просмотр скриншотов. Принят → `report-log/`,
тикет → `done/`.
