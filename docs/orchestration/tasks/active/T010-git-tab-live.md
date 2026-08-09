# T010 — Git-таб: живой статус вместо заглушки

> ## Статус (2026-08-09, checkpoint #5)
>
> - **Milestone A** — accepted earlier (status/stage/commit + T017 follow).
> - **Milestone B** — **PARTIAL-ACCEPT (architect Path A)**: branches +
>   unified text diff. Service tests green; UI strings in binary. Live
>   harness could not switch page-nav rail (T034), so visual clickthrough
>   of Git tab deferred. Syntect residual still open.
> - **Milestone C** — open (push/pull/stash).
> - **T034 ACCEPT** — page-nav rail fixed (absolute + z-order). Live Git-tab
>   visual re-verify unblocked (human or harness with focused window).
>
> Report: `report-log/T010-git-tab-live-b-milestones-report.md`.

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
- ~~Milestone B — ветки + unified diff~~ **код 2026-08-09** (list/create/checkout
  branches; click file → unified text diff; syntect colouring deferred).
  Service tests green; live acceptance pending.
- Milestone B residual: syntect highlighting of the diff pane.
- Milestone C — push/pull через системный credential-helper и
  SSH-agent, stash.
- Живой прогон: сервисный слой подтверждён (ветка `main`, 6 modified +
  6 untracked совпали с `git status`), сценарии stage → commit →
  watcher-обновление не прогонялись.
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
