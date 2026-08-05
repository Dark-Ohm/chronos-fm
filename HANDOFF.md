# HANDOFF — контекст для новой сессии Архитектора (Chronos-FM)

**Обновлено: 2026-08-05 (чекпоинт), HEAD `192c5b3`.** Цель сессии:
«перекрасить Chronos-FM в Chronos + перенести дизайн-паттерны +
доработать до состояния замены Thunar». Читать этот блок первым, §0-§5
ниже — история до 2026-07-18, остаётся верной.

## Чекпоинт 2026-08-05 — reskin-трек запущен, T001+T002 приняты

Заведена T-нумерованная оркестрация в этом репо (по образцу ChronOS,
но локальный счётчик — НЕ путать с T-ID в ChronOS, разные проекты):
`docs/orchestration/tasks/{active,report,report-log,rejected,done}`.
Design spec: `docs/superpowers/specs/2026-08-05-theme-reskin-design.md`.

**Параллельно существует независимый функциональный трек b1–b4**
(`docs/agents/active/` — clipboard/rename, file-ops, context-menu
row+grid, план `docs/superpowers/plans/2026-07-21-explorer-context
-menu.md`) — старая b-схема этого репо, НЕ T-нумерована, не трогать
файлы `row.rs`/`list.rs`/`grid.rs`/`listing.rs` пока они в работе.
Не раздана на момент чекпоинта (`docs/agents/report/` пуст).

**T001 (theme reskin infra) — ПРИНЯТ, `done/`.** Единый источник цвета:
`gpui_component::Theme` (у gpui-component уже был готовый light/dark
toggle-движок, `Theme::change()` был наполовину подключён в `root.rs`,
просто красил дефолтную палитру) + новый `crates/chronos-fm/assets/
themes/chronos.theme.json` (140 ключей/режим, сгенерирован
`script/dev/gen_chronos_theme.py` из Base16-сида — тот же сид, что
`../ChronOS/crates/ui/src/theme/schemes.rs::DEFAULT_BASE16` (Mocha dark)
и `light_scheme()` ("Light C"). Мост `crates/chronos-fm-ui/src/theme.rs`
— 20 бывших `pub const` → `pub fn name(cx: &App) -> Hsla`, читают
`cx.theme()`. Живая приёмка архитектором (не только компиляция):
grim+пипетка обоих режимов, `bg` dark `#1e1e2e` и light `#dde0f2` —
точное совпадение с seed. Найдена и исправлена ошибка: dark-акцент
случайно был Mocha-mauve вместо `#007acc` (правило «акцент не красится
темой» нарушалось) — поймано архитектором до приёмки, не пользователем.

**T002 (elevated_card + section_header) — ПРИНЯТ, `done/`.** Новый
`crates/chronos-fm-ui/src/patterns.rs`, порт ChronOS T231-паттерна (без
`elevation_apply_light_chrome` — гпуи-компонентный `.shadow_md()`
эквивалент). Применён в `sidebar.rs` (секция Folders) и `settings.rs`.
Живой кадр подтвердил `sidebar.rs` (dark); `settings.rs` принят по
код-ревью — попытка синтетического клика (`ydotool`) на живом столе
промахнулась мимо gear-иконки в чужое окно (на столе шла не связанная
с сессией активность), решил не долбить вслепую повторными кликами.

**Empty-state паттерн — НЕ портирован, сознательно.** Источник в ChronOS
— T252 (`docs/orchestration/tasks/active/T252-*.md` в ChronOS), сам ещё
не закрыт на момент этого чекпоинта (ждёт отчёта от GPT 5.6 sol). Заводить
T003 в Chronos-FM только после того как T252 закроется в ChronOS.

**Практическая находка обеих приёмок:** `cargo build ... | tail -N` в
отчётах глушит реальный exit-код cargo (bash без `pipefail` — экзит-код
принадлежит `tail`, не билду). Проверять `cargo build ... 2>&1; echo
EXIT=$?` или без пайпа вовсе — T001-исполнитель наступил, T002-исполнитель
уже учёл это в своём отчёте.

**Очередь:** T003 (empty-state, после ChronOS T252) — не заведён. b1–b4
можно раздавать независимо в любой момент (файловые зоны не пересекаются
с reskin-треком). Хвосты §2 старого HANDOFF (app_id, doc-sync,
search-permission) — не в очереди, ждут запроса.

## 0. Как этот репо вообще выглядит (важно, было недоразумение)

Это НЕ тот же самый рабочий каталог, что был до 2026-07-18 — старый
каталог содержал документы, наплетённые локальной 3B-моделью
(галлюцинации: корневые `ARCHITECTURE.md`/`MEMORY.md` оказались
дословными копиями РАННИХ доков ChronOS-шелла, плюс выдуманный
release-notes на несуществующий релиз `v0.1.0-cachy1`). По запросу
пользователя старый каталог снесён (перемещён в
`../Chronos-FM.bak-3b-<таймстамп>`, не удалён физически) и репо
переклонировано начисто:

```
gh repo view Dark-Ohm/chronos-fm  # публичный, не приватный (пользователь
                                   # ошибочно помнил его приватным)
git clone git@github.com:Dark-Ohm/chronos-fm.git Chronos-FM
```

**Канон документов теперь — только то, что реально в git-истории этого
клона**: `docs/architecture.md` (актуальная архитектура, японский),
`docs/ROADMAP.md` (P1–P6 фазы), `CHANGELOG.md`, `README.md`. Корневых
`ARCHITECTURE.md`/`MEMORY.md`/`SESSION_REPORT.md`/`SOUL.md` в чистом
клоне НЕТ — если они появятся снова (например, кто-то скопирует их из
бэкапа), это НЕ канон, а рецидив 3B-мусора — не доверять, сверять с
`docs/`. `.rules` в корне — это конфиг агента (Factory Droid, generic
Rust/GPUI гайдлайны), не архитектура проекта; `AGENTS.md`/`CLAUDE.md` —
однострочные указатели на него (`.rules`), это нормально, не баг.

Git identity в этом репо — локальный (не global) `dark-ohm` /
`dohm.labs@proton.me`, тот же что в ChronOS. БЕЗ AI-трейлеров в коммитах
(та же политика, что в ChronOS — глобальное правило пользователя, не
специфичное для одного репо).

## 1. Что такое Chronos-FM

"Launcher × Explorer" — Raycast-style лаунчер + keyboard-driven файловый
эксплорер в одном приложении, форк `nohrs` (см. `dce6124` — "renamed
from nohrs"), портируется под Linux/Hyprland лично пользователем.
MIT, `github.com/Dark-Ohm/chronos-fm` (публичный).

Воркспейс (7 активных крейтов, `Cargo.toml`):
`chronos-fm` (bin) / `chronos-fm-core` / `chronos-fm-models` /
`chronos-fm-services` / `chronos-fm-store` / `chronos-fm-ui` /
`chronos-fm-pages` (самый жирный, ~6700 строк — эксплорер).
`chronos-fm-launcher` (P3) и `chronos-fm-plugin-host` (P4) — ЕЩЁ НЕ
СОЗДАНЫ, только закомментированы в `[workspace] members`.

Фазы (`docs/ROADMAP.md`): P1 Foundation (сейчас, `0.0.x`) → P2 Explorer
Essentials (`0.1.0`) → **P3 Launcher & Search (`0.2.0`, лаунчера ещё нет
физически)** → **P4 Plugin Host (`0.3.0`) — РЕШЕНО 2026-07-18: Luau
(`chronos_luau`), НЕ WASM.** WASM Component Model/wit-bindgen/wasmtime
(ADR 0005) отменено целиком — см. [ADR 0009](docs/adr/0009-luau-plugin-host-supersedes-wasm.md),
ADR 0005 помечен Superseded. Rust/TS/Python плагин-темплейты отменены.
Не реализовано ни в одном варианте — P4 не начат, менять было дёшево.
Открытый хвост: расшарить `chronos_luau` из ChronOS path-депом (по
аналогии с GPUI-тулкитом) или завести отдельную копию — не решено, не
начинать `chronos-fm-plugin-host` без этого решения. → P5 Ecosystem
→ P6 Stabilization.

## 2. Единый тулкит с ChronOS (сделано 2026-07-18)

Решение брейнсторм-сессии: один GPUI-форк на оба проекта — наш
`../Source` (gpui-ce chronos edition), НЕ upstream zed/crates.io.
Причина: у обоих проектов свой лаунчер по плану — по-другому
несопоставимо и глупо плодить два форка одного тулкита.

**Выполнено GROK'ом (задание в `GROK.md`, отчёт в
`report-log/grok-report-1.md`), ПРИНЯТО мной лично** (`8c2a7f4`,
живой смок + `cargo tree` + тесты перепроверены, не поверил отчёту на
слово):
- `gpui`/`gpui_platform`/`gpui-component` теперь path-депы на
  `../Source/gpui`, `../Source/gpui_platform`,
  `../Source/gpui-component/crates/ui` (были crates.io: `gpui="0.2"`,
  `gpui-component="0.5"`).
- `[patch."https://github.com/zed-industries/zed"]` в корневом
  `Cargo.toml` — `gpui-component`'овский вложенный воркспейс всё ещё
  объявляет zed-siblings как git-депы; патч унифицирует граф на один
  path-`gpui` (иначе Cargo резолвит ДВА разных `gpui` и падает).
  Подтверждено `cargo tree --workspace -i gpui` — один узел.
- Bootstrap: `Application::new()` (upstream-паттерн) не существует в
  нашем форке публично → заменено на `gpui_platform::application()`
  (`crates/chronos-fm/src/app.rs`) — тот же паттерн, что ChronOS
  `main.rs`.
- Дрейф API — маленький, 4 места: `flex_grow()` →
  `flex_grow_1()`/`flex_grow(f32)` (`explorer/view.rs`,
  `pane_group.rs`), `FocusHandle::focus(window)` →
  `focus(window, cx)` (`pane_group.rs`).
- `Source/` — 0 правок (было условие задания, read-only).
- Живой смок (мой, не только Grok'а): `cargo build --workspace` +
  `cargo test --workspace` (171/171) чисто; бинарь реально открывает
  окно на этой Hyprland-сессии, `gpui_wgpu` реально выбирает
  "NVIDIA GeForce RTX 3070 (Vulkan)" — подтверждает, что это НАШ
  форк-рендерер работает, а не заглушка.

**Хвосты (не блокеры, не трогать без запроса пользователя):**
- Окно `chronos-fm` имеет пустые `class`/`title` в `hyprctl clients -j`
  (нужен `app_id` — мешает window rules/поиску по имени).
- `docs/architecture.md`/README не обновлены под новую path-deps
  конвенцию (Grok сам это отметил как долг).
- Search-сервис падает с `Permission denied` на
  `~/.local/share/containers/storage/volumes/hindsight_pg_data/_data`
  (индексатор пытается сканировать podman-volume, куда нет доступа) —
  **не связано с миграцией тулкита**, pre-existing, некритично (app не
  падает, просто ищет без индекса). Отдельная задача, если пользователь
  захочет её закрыть.

## 3. Лаунчер — РЕШЕНО 2026-07-18: разные по назначению, не сливать

Изначальный план сессии — сравнить лаунчер ChronOS (`SUPER+L`,
минималистичный, XDG toplevel, `crates/app/src/launcher/` в ChronOS) с
лаунчером Chronos-FM. Оказалось: в Chronos-FM лаунчера физически нет
(P3, только `[launcher]` в `settings.toml`: `hotkey` дефолт
`Cmd+Shift+Space` — macOS-наследие, `position_remember`). Сравнивать
было нечего.

**Решение пользователя:** лаунчеры остаются РАЗНЫМИ по задаче, не
сливаются в один UX/хоткей.
- ChronOS `SUPER+L` — быстрый системный app-launcher, часть шелла,
  работает всегда независимо от того, запущен ли Chronos-FM.
- Chronos-FM (P3, будущий) — "глубокий поиск" по файлам/командам/
  git-статусу, привязанный к данным эксплорера (SQLite+Tantivy индекс) —
  другая задача, даже если стилистически похож (Raycast-style).

Не пересматривать это без явного нового запроса пользователя — компромисс
(единый UX против меньшей связанности) был явно взвешен и решён в пользу
разделения при текущем масштабе проекта.

## 4. Линты и code style, принесённые В ChronOS ИЗ этого проекта

При сравнении кода (см. `docs/architecture.md` §5 здесь) выяснилось,
что дисциплина линтов в Chronos-FM строже: `unsafe_code = deny`,
`clippy::unwrap_used`/`expect_used = warn`, запрет глушить ошибки
`let _ =`. Эти три вещи перенесены В ChronOS (`ChronOS/CLAUDE.md` §"Код
— правила", commit `93f917c` в ChronOS) — НЕ в этот репо, они тут уже
были. Обратного переноса (мод.rs-запрет, cargo-deny tokio-бан,
missing_docs, ADR-файлы) в ChronOS НЕ произошло — рассмотрено и
осознанно отклонено, см. ChronOS `CLAUDE.md` для причин каждого пункта.

## 5. Минион-конвенция для этого репо

Та же схема, что в ChronOS: задание → `<ИМЯ>.md` в корне этого репо
(не ChronOS!) → пользователь скармливает агенту → отчёт в
`<имя>-report.md` → Архитектор принимает лично (грепы/сборка/живой
смок), после приёмки переносит отчёт в `report-log/<имя>-report-N.md`
git-мувом (не просто `rm`, иначе может "воскреснуть" при неосторожном
git-мусоре других сессий/агентов — см. инцидент в ChronOS MEMORY.md).
Пока в этом репо был только Grok (задание №1, принято).

## Очередь

1. Хвосты §2 (app_id, README/architecture.md doc-sync, search
   permission) — не назначены, ждут запроса пользователя.
2. Лаунчер (P3) — решение принято (§3, разделены), реализация не
   начата, не в очереди пока пользователь не попросит.
3. Plugin-host (P4) — решение принято (Luau, §2, ADR 0009), но есть
   ОТКРЫТЫЙ хвост перед стартом реализации: расшарить `chronos_luau` из
   ChronOS path-депом или завести отдельную копию в Chronos-FM — не
   решено, спросить пользователя перед началом `chronos-fm-plugin-host`.
4. `docs/ROADMAP.md`/`docs/plugin-*.md` — P4-секции всё ещё описывают
   WASM построчно (см. баннер вверху ROADMAP.md) — переписать при
   реальном старте P4, не заранее.
