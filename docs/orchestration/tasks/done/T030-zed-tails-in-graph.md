# T030 — Форк: пять zed-крейтов в графе сборки

> ## ✅ ARCHITECT VERDICT: **CLOSED — DEFERRED** (2026-08-09)
>
> Not implemented. Closed as known dependency debt so the active queue
> stays honest. Partial related vendoring lives under **T032**
> (`reqwest_client` / `http_client_tls`); `http_client` itself still on
> zed git. Full hermetic removal → new ticket when scheduled.
>
> Report: `report-log/T030-zed-tails-in-graph-report.md`.

**Приоритет:** P2. Единственный оставшийся «хвост zed» после T025.
**Скоуп:** `Source/`. Правки в `Source/gpui*` **разрешены** — но см.
раздел про соседей, он здесь жёстче обычного.

## Что в графе

`cargo tree -e normal -p chronos-fm --target x86_64-unknown-linux-gnu`
показывает пять крейтов, тянущихся прямо из `zed-industries/zed@876ec5a`:

```
gpui → http_client (zed) → util (zed)
gpui → util_macros (zed) → perf (zed) → collections (zed)
```

Точки объявления — `Source/gpui/Cargo.toml:63` (`http_client`) и `:83`
(`util_macros`), обе **безусловные**, не под `cfg`. Следствие: сборка
форка не герметична — нужны сеть и доступность чужого репозитория.

Отдельно: `Source/gpui_zed_util` (lib name `util`) числится членом
воркспейса (`Source/Cargo.toml:25`), но **на него никто не ссылается** —
в графе живёт zed-овский `util`. Либо это заготовка под замену, либо
мусор; решить надо явно.

**Не в скоупе:** `wgpu`, `xim-rs`, `font-kit`, `reqwest`, `scap`. Они
zed-овские только по адресу репозитория — это форки сторонних библиотек
с зафиксированными rev. Замена = смена рендера и ввода.

## Порядок, по возрастанию риска

**Шаг 1 — `gpui_zed_util`.** Определить: подключить его вместо
zed-овского `util` или удалить из членов воркспейса. Дешёвый, обратимый,
делается первым, потому что от него зависит понимание шагов 2–3.

**Шаг 2 — `util_macros`.** Это proc-macro. Найти, что именно gpui из
него использует (`grep` по вызовам в `Source/gpui/src/`). Если один-два
макроса — вендорить и убить ветку `util_macros → perf → collections`,
минус три крейта.

**Шаг 3 — `http_client`.** Самый жирный узел, тянет `util`. **Сначала
измерить**, что gpui из него реально дёргает на линуксе, и только потом
решать: вендорить, обрезать фичей или оставить. Без замера не начинать.

**Шаг 4 — связка из T025.** `Source/gpui-component/Cargo.toml:39`
приколот к zed-rev `876ec5a` именно потому, что его тянет наш `gpui`.
Любое движение шагов 2–3 обязано двигать эту строку тем же коммитом.
Если после шагов zed-депы исчезнут совсем — пин снимается, и это надо
проверить отдельно, а не забыть.

## Стены

**Стена 1 — `Source/gpui` держит на себе четыре проекта.** ChronOS
(patch на path, 16 крейтов), greeter Chronos-lm
(`crates/greeter/Cargo.toml:15-16`), Chronos-FM, Chronos-IDE. Базовые
линии снять **до** первой правки и повторять после каждого шага:

```bash
cargo check --workspace --manifest-path ChronOS/Cargo.toml
cargo check --workspace --manifest-path Chronos-lm/Cargo.toml
cargo test  --workspace --manifest-path Chronos-FM/Cargo.toml   # эталон 294 passed
cargo build -p gpui-component-story                              # из Source/gpui-component
```

Красная линия **до** начала — фиксировать как есть и сравнивать с ней, не
чинить.

**Стена 2 — шаги независимы, коммиты раздельные.** Каждый шаг — свой
коммит со своим прогоном базовых линий. Если шаг 3 придётся откатить,
шаги 1–2 должны выжить.

**Стена 3 — вендоринг тянет лицензию.** Всё, что переносится из zed в
`Source/`, идёт с указанием происхождения: запись в `Source/NOTICE` и
`PATCHES.md` рядом с крейтом, как сделано для остальных вендоренных
(`Source/Cargo.toml:26-28` описывает эту конвенцию).

**Стена 4 — метрика одна и она проверяемая.** Успех измеряется числом:

```bash
cargo tree -e normal -p chronos-fm --target x86_64-unknown-linux-gnu \
  | grep -c 'zed-industries/zed?rev'
```

Сейчас пять крейтов. Цель шагов 1–2 — два. Полный ноль — только если
шаг 3 сойдётся, и это не обязательство.

**Стена 5 — остановиться можно на любом шаге.** Тикет не требует довести
до нуля. Три зелёных шага и честное «шаг 3 дороже выигрыша» — валидный
результат. Что не делалось — назвать.

## Приёмка

- Число zed-крейтов в графе «до/после» по команде из Стены 4.
- Базовые линии всех четырёх потребителей до и после каждого шага.
- Решение по `gpui_zed_util` названо явно: подключён или удалён, и почему.
- Записи в `NOTICE`/`PATCHES.md` для всего перенесённого.
- Состояние связки `gpui-component/Cargo.toml:39` после работы.

## Отчёт

`docs/orchestration/tasks/report/T030-zed-tails-in-graph-report.md`.
Приёмка — архитектор лично.
