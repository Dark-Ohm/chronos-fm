# Fork → локальный `../Source`: path-зависимости для живой разработки, git для скачавших (REPORT)

**Тикет:** нет (прямое решение пользователя; инфраструктура сборки)
**Коммит:** `9790480` `build: path deps to ../Source for live fork dev; packaging+CI pin fork git`
**Форк:** запушен `57f582f..0f3a67e` (`Dark-Ohm/Chronos-GPUI`, main)
**Статус:** ⬜ inbox → на приёмку (архитектор). Принят → `report-log/`.
**Дата:** 2026-08-06

> Цель отчёта — дать архитектору всё для сверки с `git show 9790480` / `git status` форка.

---

## 1. Контекст

Постановка пользователя (две директивы):

1. «я разрешаю улучшать форк по мере надобности. локальные патчи не работают у
   пользователей которые скачали софт» — форковые фиксы должны доезжать до
   скачавших, а не жить локально.
2. «chronos fm должен искать source локально а не в гите» + «скачавшие софт
   будут линкованы к гиту, у нас живая разработка идет» — **dev-сборка резолвит
   форк из `../Source` (без rev-церемонии), release-пользователи — из git.**

Результат: Cargo.toml переведён на committed **path deps** (`../Source`) — это
dev-истина; release-путь (AUR/PKGBUILD, CI) переводит обратно на git-форк на
закреплённом коммите.

## 2. Что изменено (сверка по `git show 9790480`)

### Cargo.toml / Cargo.lock
- Все **17** зависимостей Chronos-GPUI переведены с
  `git = "https://github.com/Dark-Ohm/Chronos-GPUI", rev = "ee80b72"` на
  `path = "../Source/..."`:
  - `[workspace.dependencies]`: `gpui`, `gpui_platform`, `gpui-component`
    (путь до `../Source/gpui-component/crates/ui`);
  - `[patch."https://github.com/zed-industries/zed"]`: 14 записей
    (`gpui`, `gpui_platform`, `gpui_macros`, `gpui_web`, `gpui_shared_string`,
    `gpui_linux`, `gpui_wgpu`, `gpui_collections`, `gpui_scheduler`,
    `gpui_sum_tree`, `gpui_refineable`, `gpui_derive_refineable`, `gpui_media`,
    `gpui_util`) — без patch граф распадался бы на два gpui (наш + zed из
    gpui-component).
- Комментарий над депами переписан: dev = path, release = git через
  `script/gen-pkgbuild.sh`.
- `Cargo.lock` перегенерирован; **портируемый**: path-депы записаны без
  source-строк (`grep -c 'path+file' Cargo.lock` = 0), резолвятся от
  относительного `../Source` — один и тот же lock работает локально и в CI при
  наличии sibling-чекаута.

### Форк (Source) — запушен, не патчится локально
- `git push origin main` → `57f582f..0f3a67e` — уехали все 13 коммитов после
  старого пина: RTL-фиксы T152 (`86701db`, `de62111`, `4e6c3be`, `d8920c1`),
  feature-gates gpui-component (`6118382`), вендор `gpui-animation`/`gpui-rsx`
  (`66cd816`, `99cab5e`), T225 per-line quads (`0f3a67e`).
- **Отладочный локальный патч `gpui/src/text_system/line.rs` (RTL_TRACE
  `eprintln!`-спам на каждый RTL-рендер + дамп-тест) — откачен полностью**
  (`git checkout --`). Шипится как есть было нельзя (лог-мусор у юзеров).
  RTL-регрессионное покрытие в форке уже есть (`test_aligned_origin_x_rtl_*`).

### script/gen-pkgbuild.sh
- Fork-rev больше не извлекается из Cargo.lock (в path-lock нет git-ревизии) —
  резолвится из чекаута: `$CHRONOS_FORK_DIR` → `../Source` (локальная dev) →
  `Source/` внутри репо (CI). Без чекаута — fallback-коммит + предупреждение.
- PKGBUILD:
  - `source=` теперь качает **форк**: `gpui::git+https://github.com/Dark-Ohm/
    Chronos-GPUI.git#commit=${_GPUI_COMMIT}` (раньше — `zed-industries/zed` с
    fork-коммитом — это был баг: такого коммита в zed нет);
  - `prepare()`: `sed -i 's|path = "../Source/|path = "../gpui/|g' Cargo.toml`
    (17/17, проверено на копии) + `rm -f Cargo.lock` (path-lock невалиден для
    тарболла) + `cargo fetch`;
  - `build()`: без `--locked` (lock регенерируется в prepare);
  - `check()`: `test -d ../gpui/gpui` вместо `grep коммит Cargo.lock`.

### CI (.github/workflows)
- `aur-publish.yml`: перед генерацией PKGBUILD — `actions/checkout` форка в
  `path: Source` (fallback-ветка скрипта).
- `cachy.yml`:
  - после checkout — клон форка в sibling: `git clone --depth 1
    https://github.com/Dark-Ohm/Chronos-GPUI "$GITHUB_WORKSPACE/../Source"`
    (path-депы `../Source/...` резолвятся именно от sibling);
  - шаг `Verify GPUI commit` (жёсткий хардкод `69e21302...` + grep по lock —
    сломался бы) заменён на `Verify fork link` (`test -d ../Source/gpui` +
    `grep 'path = "../Source/' Cargo.toml`).

## 3. Отвергнутые механизмы (эмпирически проверены, чтобы не гадали)

| Механизм | Результат |
|---|---|
| `.cargo/config.toml` `paths = ["../Source"]` | `error: found patches and a path override` — конфликт с `[patch."zed"]` в Cargo.toml (нужен для унификации графа) |
| `[source."git+…"]` + `directory`-замена | Не применяется: source replacement — только для **идентичных** зеркал (Cargo Book); у нас форк на 13 коммитов новее пина + directory требует `.cargo-checksum.json`. `manifest_path` остаётся на git-кэше |
| `[patch]` в `.cargo/config.toml` | Применяется (манифесты идут из Source), но ломает резолв: два `gpui 0.2.2` из разных источников (git-патч в Cargo.toml + path-патч в config) + переписал бы lock на path+file (сломало бы юзеров) |
| **Committed path deps в Cargo.toml** | ✅ Резолв чистый, все gpui\* → `…/Source/…`; lock портируемый; локально — живой dev без rev-церемонии |

## 4. Верификация (команды + результаты)

```
# локальная сборка против нового форк-HEAD (13 коммитов, feature-gates)
cargo check --workspace          → 0 ошибок
cargo test --workspace           → 214 passed, 0 failed (51+69+48+16+30, EXIT=0)
cargo build -p chronos-fm        → EXIT=0 (линкуется)

# скрипт/шаблон
bash -n script/gen-pkgbuild.sh   → OK
python yaml.safe_load (оба workflow) → OK
генерация PKGBUILD (мок-шесумма, CHRONOS_FORK_DIR=../Source)
  → _GPUI_COMMIT=0f3a67eef963c1d7d582f350c93a7ad1cb205ed3 (реальный HEAD форка)
  → source= качает Dark-Ohm/Chronos-GPUI.git, prepare() с sed-конвертацией
sed dry-run на копии Cargo.toml  → 17 конвертировано, 0 остатков `../Source/`
```

Форк-пуш сделан ДО локальной верификации? Нет: сначала `cargo check`/`test`/
`build` против форк-HEAD, потом пуш (не пушили непроверенное состояние).

## 5. Код-ревью / решения

Отдельного code-review не было (изменения — манифест/скрипт/CI, не Rust-код).
Вместо этого — эмпирическая проверка всех трёх альтернативных механизмов cargo
(§3) и рендер-тест шаблона. Ревьюверский глаз нужен на: семантику `--locked` в
cachy при расхождении форк-HEAD и lock (см. оговорку 3).

## 6. Оговорки (честно)

1. **Dev-истина требует `../Source` рядом** — любой `cargo build` вне этого
   layout сломается. Осознанно: официальный путь для скачавших — AUR/PKGBUILD
   (prepare конвертит path→`../gpui` + регенерит lock), CI клонирует форк сам.
2. **Cargo.lock** — большой diff (1609 строк): перегенерация с нуля
   (git-источники → path, плюс новые registry-крейты из более свежего графа
   форка). В release-тарболле не используется (rm + регенерация).
3. **cachy.yml `--locked` + клон форка на HEAD**: пока форк-HEAD и lock
   согласованы — работает; если форк уедет вперёд без обновления lock — CI
   упадёт с `--locked` (это сигнал обновить lock/форк, а не поломка).
4. **Отладочный RTL-тест не перенесён в форк** — откачен вместе с eprintln.
   Чистый smoke-тест mixed-script шейпинга (через `make_shaped_line`, без
   WindowTextSystem/шрифтов) — отдельная задача, если нужен.
5. **Пуш chronos-fm не выполнялся** — только локальный коммит `9790480`;
   ветка main на 42 коммита впереди origin (пуш за пользователем).

## 7. Готовность

Infra-задача закрыта: локальная сборка живая против `../Source` (без rev-bump),
скачавшие линкуются к git (PKGBUILD/CI на закреплённом fork-коммите),
отладочный патч форка не шипится. Приёмка архитектора → `report-log/`.
