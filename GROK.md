# GROK — задание №1: перевести Chronos-FM на наш форк gpui-ce (единый тулкит)

_Дата: 2026-07-18. Отчёт — `grok-report.md` в корне ЭТОГО репо (Chronos-FM),
формат SESSION_REPORT (см. `MEMORY.md`/`.rules` этого репо — тот же строгий
формат, что в ChronOS). **У тебя НОВАЯ СЕССИЯ** — предыдущего контекста нет,
всё нужное здесь. Это ДРУГОЙ репозиторий, не ChronOS — не путай пути._

## Контекст (полный, с нуля)

Пользователь держит два проекта на одном тулките GPUI:
- **ChronOS** (`/home/neo/projects/chronos-ecosystem/ChronOS`) — Wayland/Hyprland
  desktop shell, использует СВОЙ форк gpui-ce из
  `/home/neo/projects/chronos-ecosystem/Source` (path-deps).
- **Chronos-FM** (этот репо) — file manager + launcher (Raycast-style),
  порт форкнутого `nohrs` под Linux, СЕЙЧАС собирается на upstream
  `zed-industries/gpui` через crates.io (`gpui = "0.2"`, `gpui-component = "0.5"`).

Решение (брейнсторм-сессия, только что): **один тулкит для обоих проектов —
наш форк**, не upstream. Раз в Chronos-FM пока НЕТ лаунчера (P3, не начат —
только заглушка конфига), сравнивать нечего, но сам переезд на общий gpui-ce
логичен и его нужно сделать сейчас, пока Chronos-FM маленький (13k строк),
а не после того как он разрастётся.

## Установленные факты (проверено, не гадать заново)

- `Source/gpui/Cargo.toml`: `version = "0.2.2"`. Chronos-FM'овский
  `gpui = "0.2"` (crates.io) резолвится ТОЖЕ в `0.2.2` — та же базовая версия,
  наш форк накатывает патчи поверх (layer-shell, wayland sync-drop-fix,
  quit-hang fix, blur-порт из kael — см. `git -C ../Source log --oneline`).
- `Source/gpui-component/` — ПОЛНОСТЬЮ форкнутая копия `longbridge/gpui-component`
  (свой вложенный Cargo-воркспейс: `crates/ui`, `crates/story`, `crates/assets`
  и т.д., ~118k строк). Крейт `crates/ui` там называется `gpui-component`,
  версия `0.5.2`. Chronos-FM сейчас на `gpui-component = "0.5"` с crates.io —
  резолвится в `0.5.1`. Версии практически соседние — ожидай МАЛЫЙ дрейф API,
  но проверь эмпирически, не полагайся на это как на гарантию.
- Зависимость от `gpui`/`gpui-component` объявлена НАПРЯМУЮ (не через
  `[workspace.dependencies]`) в 4 местах:
  - `crates/chronos-fm-ui/Cargo.toml` (`gpui = "0.2"`, `gpui-component = "0.5"`,
    плюс dev-dependency `gpui = { version = "0.2", features = ["test-support"] }`)
  - `crates/chronos-fm-pages/Cargo.toml` (то же самое, plus dev-dep test-support)
  - `crates/chronos-fm/Cargo.toml` (bin: `gpui = "0.2"`, `gpui-component = "0.5"`)
  - `crates/chronos-fm-services/Cargo.toml` (`gpui = { version = "0.2", optional = true }`
    под фичей `gui = ["dep:gpui", "dep:syntect"]`)
- Использование `gpui_component::*` в коде — 18 файлов, 34 сайта
  (`grep -rl "gpui_component" crates/*/src --include="*.rs"`), сосредоточено
  в основном в `chronos-fm-pages` (6700 строк — самый жирный крейт, explorer UI)
  и `chronos-fm-ui` (1353 строки).
- **Структурное расхождение бутстрапа приложения (ОТКРЫТЫЙ ВОПРОС, не решён
  заранее — разберись сам):** ChronOS's `main.rs` стартует через
  `gpui_platform::application()` (`Source/gpui_platform/src/gpui_platform.rs:13`,
  `pub fn application() -> gpui::Application`) — наш форк вынес
  platform-инициализацию (backend selection: wayland/x11, шрифты) в отдельный
  крейт `gpui_platform`, а не только через `gpui::Application::new()` напрямую.
  Chronos-FM сейчас бутстрапится классическим upstream-паттерном
  `Application::new().with_assets(Assets).run(...)` (`crates/chronos-fm/src/app.rs:48`,
  `Application` — реэкспорт из `gpui`, `pub struct Application` живёт в
  `Source/gpui/src/app.rs:144` — САМ тип в нашем форке присутствует и структурно
  не менялся). Открытый вопрос: работает ли `gpui::Application::new()` напрямую
  на нашем форке "из коробки" (без прохода через `gpui_platform::application()`),
  или наш форк тихо полагается на что-то, что `gpui_platform::application()`
  настраивает (feature-флаги `wayland`/`x11`/`font-kit` уже включены в
  `gpui_platform.workspace = true` дефолтами ChronOS, а не в самом `gpui`).
  **Проверь сборкой и живым запуском — не гадай по коду.** Если
  `Application::new()` не взлетает, добавь `gpui_platform` как прямую
  зависимость `chronos-fm` (bin) и замени бутстрап на `gpui_platform::application()`,
  по аналогии с ChronOS `main.rs:14`.

## Задача

1. В 4 файлах выше замени `gpui = "0.2"` / `gpui-component = "0.5"` (и
   dev-dependency вариант с `test-support`) на **path-зависимости** относительно
   каждого крейта на `../../Source/gpui` и `../../Source/gpui-component/crates/ui`
   (посчитай реальную глубину пути от `crates/<name>/Cargo.toml` до
   `Source/` — она НЕ `../Source`, а на уровень глубже, т.к. крейты лежат в
   `crates/<name>/`, а `Source/` — сосед `Chronos-FM/`, не `crates/`).
   Лучше сначала добавь `gpui`/`gpui-component` в корневой
   `[workspace.dependencies]` как path-зависимости, а в каждом крейте перейди
   на `gpui.workspace = true` / `gpui-component.workspace = true` — это
   соответствует тому, как остальные зависимости (`serde`, `anyhow`, `time`)
   УЖЕ объявлены в этом воркспейсе (см. `docs/architecture.md` §1), так что
   это не отсебятина, а следование существующей конвенции репо.
2. `cargo build --workspace` (тулкит-зависимые крейты не в `default-members`,
   так что голый `cargo build` их не тронет — используй `--workspace` явно,
   как я проверил вживую перед тем как отдать тебе задание).
3. Почини ВСЁ, что вылезет при сборке — версийный дрейф API между
   `gpui-component 0.5.1` (crates.io, что было) и `0.5.2` (наш форк, что
   будет) в 18 файлах, плюс возможный бутстрап-вопрос из пункта выше.
4. `cargo test --workspace` (или default-members, если GUI-тесты требуют
   дисплея — проверь, есть ли headless-режим, `docs/agent-ui-verification.md`
   уже должен объяснять, как гонять UI на Linux).
5. Живой смок: собери `--release` или хотя бы `debug`, запусти бинарь
   (`./target/debug/chronos-fm`), убедись что окно реально открывается на
   этой Hyprland-сессии (я лично проверил ДО тебя на upstream-версии — окно
   открывается, Vulkan/Wayland surface живой, RTX 3070 определяется). После
   твоего перевода на наш форк — то же самое ДОЛЖНО продолжать работать,
   не хуже. Скриншот необязателен, но лог реального успешного запуска —
   обязателен в отчёте (не «должно работать»).

## Зоны (ЖЁСТКО)

- Твои: весь `Chronos-FM/` (это отдельный проект, сейчас там нет других
  агентов).
- **`Source/` — ТОЛЬКО ЧТЕНИЕ.** Это общая инфраструктура, которую
  использует и ChronOS. Если для сборки Chronos-FM тебе кажется, что
  что-то нужно ПОПРАВИТЬ в `Source/` (например, экспортировать что-то,
  чего не хватает) — НЕ трогай, останови работу и опиши точно, что именно
  нужно и зачем, в отчёте. Правки в `Source/` — отдельное решение, не
  твоё сейчас (это может сломать ChronOS).
- Не трогай `/home/neo/projects/chronos-ecosystem/ChronOS` вообще.

## Условие эскалации (важно, читай внимательно)

Я (Architect) ожидаю, что это ОДНОЗАДАЧНАЯ работа — 4 Cargo.toml + починка
дрейфа в разумных пределах. **Если по факту сборки окажется, что API
разъехалось СИЛЬНО** (не пара переименований, а десятки сломанных мест,
структурные изменения виджетов gpui-component, которые требуют переписывания
логики, а не просто адаптации сигнатур) — **СТОП**, не пытайся героически
дожать в одиночку. Вместо этого в отчёте дай честную разбивку: что именно
сломано, по какому крейту сколько строк/мест, и я разрежу оставшееся на
параллельных агентов (Cline/Hermes/OMP) по зонам. Это не провал — это
корректная эскалация, я жду её при первом же признаке, что объём больше
одного захода.

## Git

Коммить в этом репо (Chronos-FM), НЕ в ChronOS. Идентичность git —
dark-ohm / dohm.labs@proton.me (та же, что везде). БЕЗ AI-трейлеров
(Co-Authored-By и т.п.) — жёсткое правило пользователя, действует
одинаково в обоих репо. Сообщение коммита: кратко, по-английски или
по-русски (в этом репо `.rules` не требует конкретного языка коммитов —
ориентируйся на существующий `git log` этого репо для стиля, если
сомневаешься).

Отчёт — `grok-report.md` В КОРНЕ Chronos-FM, формат SESSION_REPORT (см.
`MEMORY.md` этого репо: Сделано / Расхождения / Не реализовано / Проверено
фактом / Новые риски / Статус доков).

## Приёмка (2026-07-18, Архитектор) — ✅ ПРИНЯТО

Каждое заявление отчёта сверено с деревом лично, все подтвердились:
- `git show 9e73c08` — диф ровно такой, как описан (path-депы + `[patch]`
  на zed-git siblings + bootstrap-свитч в `app.rs` + 4 сигнатурных фикса).
  Комментарии в коде объясняют WHY, не просто WHAT — хороший стиль.
- `cargo tree --workspace -i gpui` → единственный `gpui v0.2.2
  (Source/gpui)`, ни одного дубля с zed-git. Граф реально унифицирован,
  `[patch]`-обход не косметический.
- `cargo build --workspace` — чисто. `cargo test --workspace` — 171/171
  (4+51+56+23+16+21), совпадает с отчётом до цифры.
- Живой смок лично (не по логам отчёта — свой прогон): окно маппится на
  этой Hyprland-сессии, `gpui_wgpu` реально выбирает RTX 3070 (Vulkan) —
  значит это наш форк-рендерер, а не стоковый. Та же ошибка
  `Permission denied … hindsight_pg_data` на поиске, что в отчёте —
  подтверждена, не выдумана, некритична (pre-existing, не блокер).
- `Source/` — 0 правок, подтверждено `git status` там.

Эскалация не потребовалась и была не нужна — дрейф API оказался
маленьким (4 сигнатуры + один bootstrap), ровно как я и предполагал в
задании. Хорошая работа: точный отчёт, никакого вранья, честно отметил
второстепенные хвосты (пустой app_id/class, `docs/architecture.md` не
обновлён под path-деп конвенцию, README не отражает sibling-`Source/`
требование) вместо того чтобы промолчать.

**Хвосты на будущее (не блокеры, не делать сейчас без запроса):**
пустой `app_id`/class у окна (мешает `hyprctl`/window rules по имени),
`docs/architecture.md`/README не обновлены под новую path-deps
конвенцию, search-service permission error на `hindsight_pg_data` (не
связано с миграцией, отдельная проблема).
