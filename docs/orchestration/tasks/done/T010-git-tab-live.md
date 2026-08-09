# T010 — Git-таб: живой статус вместо заглушки

> ## ✅ ARCHITECT VERDICT: **CLOSED** (2026-08-09)
>
> Full ticket closed. Milestones:
>
> | Milestone | Status |
> | --- | --- |
> | **A** status / stage / commit / follow (T017) | LIVE-ACCEPT |
> | **B** branches + unified text diff | LIVE-ACCEPT |
> | **C** push / pull / stash | ACCEPT (impl + 24 tests) |
>
> Residuals (optional, not blocking close):
> - syntect colouring of diff pane
> - live C smoke (push/pull/stash on a real remote)
> - P8 stash index parse polish; disable Push/Pull when no repo
>
> Reports: `report-log/T010-git-tab-live-b-milestones-report.md`,
> `report-log/T010-git-tab-live-c-milestones-report.md`,
> earlier A material under `report/T010-git-tab-live-report.md` (if present).
>
> Design/plan: `docs/superpowers/specs/2026-08-09-t010-c-…`, `plans/2026-08-09-t010-c-…`.

**Приоритет:** P2 — вкладка-пустышка, но нет бэкенда вообще (в отличие
от Settings, где конфиг уже есть).
**Статус (2026-08-06, чекпоинт #4):** Milestone A **сдан и принят с
оговоркой**. Секция «Что нужно ПЕРЕД кодом» ниже исполнена целиком и
оставлена как история решений — брейншторм проведён, бэкенд выбран
(`gix`/gitoxide), spec и план написаны, код в `17b731b` + `dac503b`,
отчёт в `report/`.

**T017 закрыт (2026-08-07)** — follow-режим починен и подтверждён живьём
в обе стороны: заход в репозиторий наполняет панель без Refresh, выход
возвращает пустое состояние. Блокер снят, тикет разблокирован.

**Открыто:**
- ~~Milestone B~~ **LIVE-ACCEPT**.
- ~~Milestone C~~ **ACCEPT** (impl 2026-08-09; live C optional).
- Residuals: syntect; optional live C smoke; Push/Pull disable when no repo.
- ~~Живой прогон A/B~~ **passed**.
- Deferred из отчёта: render-тест `git.rs` упирается в
  `recursion_limit = 128` у макроса `gpui::test`.
- ~~Из чекпоинта #3: clippy по `pages/src/git.rs` не гонялся — 4×
  неиспользуемый `let this = cx.weak_entity()`, 2× `unwrap()` под guard'ом,
  плюс missing docs.~~ **Закрыто 2026-08-07**: все 8 пунктов исправлены,
  `cargo clippy -p chronos-fm-pages --all-targets` по `git.rs` — 0 замечаний,
  тесты 87/87 зелёные (изменения не закоммичены).

**Приёмка живого прогона (уточнено после T017):** проверять сценарий
целиком — stage → ввод сообщения → Commit → `git log` подтверждает
коммит → изменение файла снаружи обновляет панель через watcher.
Наполнения панели при заходе в репозиторий **недостаточно**: это
покрыто T017 и уже подтверждено.

## Что нужно ПЕРЕД кодом (исполнено, оставлено как история)

1. `brainstorming` skill: показать git-статус текущей директории
   эксплорера (staged/modified/untracked файлы), базовые действия
   (stage/unstage/commit). Решить: `git2` крейт (libgit2-биндинг) vs
   шелл-обёртка вокруг системного `git` (`std::process::Command`) —
   `git2` чище API, но тяжелее собирается (нужен libgit2 системно или
   vendored-фича); шелл-обёртка проще, но парсинг вывода хрупкий.
   Сверить bleeding-edge деп-политику — какой подход реально
   используется в других Rust GPUI/файл-менеджер проектах сейчас.
2. Design spec → `docs/superpowers/specs/`.
3. Implementation plan → `docs/superpowers/plans/`.
4. Только после этого — T-тикет на код.

## Зависимости

Независим от T009 (Settings) и T003/T008 (Devices). Может
брейнштормиться параллельно с T011/T012.

## Не раздавать как есть

Этот файл — не инструкция для исполнителя-кодера, сначала брейншторм.
