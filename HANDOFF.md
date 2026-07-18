# HANDOFF — контекст для новой сессии Архитектора (Chronos-FM)

**Обновлено: 2026-07-18. Свежий чистый клон (см. §0), единый тулкит с
ChronOS ЗАВЕДЁН и ПРИНЯТ (GROK №1, `8c2a7f4`). Лаунчер (P3) НЕ начат —
только конфиг-заглушка. Читать сверху вниз.**

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
физически)** → P4 Plugin Host (`0.3.0`, WASM Component Model, wit-bindgen
— пользователь рассматривает замену на Luau/`chronos_luau` из ChronOS,
т.к. plugin-host ещё не реализован, менять решение дёшево) → P5
Ecosystem → P6 Stabilization.

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

## 3. Лаунчер — сравнение с ChronOS (контекст решения "не строить пока")

Изначальный план сессии — сравнить лаунчер ChronOS (`SUPER+L`,
минималистичный, XDG toplevel, `crates/app/src/launcher/` в ChronOS) с
лаунчером Chronos-FM. Оказалось: в Chronos-FM лаунчера физически нет
(P3, только `[launcher]` в `settings.toml`: `hotkey` дефолт
`Cmd+Shift+Space` — macOS-наследие, `position_remember`). Сравнивать
было нечего — решение отложено, приоритет отдан переезду на единый
тулкит (см. §2) как более срочной и меньшей по риску задаче, пока
Chronos-FM маленький. Вопрос "сливается ли будущий лаунчер Chronos-FM с
`SUPER+L` ChronOS, или это два разных по назначению UX" — ОТКРЫТ, не
решён, спросить пользователя когда P3 подойдёт по факту.

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
2. Лаунчер (P3) vs ChronOS `SUPER+L` — открытый архитектурный вопрос,
   см. §3. Не начинать без явного решения пользователя.
3. Plugin-host (P4): WASM Component Model vs Luau — рассматривается
   пользователем, ничего не реализовано ни в одном варианте, решение
   не принято.
