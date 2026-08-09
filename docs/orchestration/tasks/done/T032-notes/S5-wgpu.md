# S5 — wgpu: path-vendor zed@357a0c5

**Статус:** DONE — коммиты `2044c98` (vendor) + `44544d5` (lockfile follow-up)
в `Source`, working tree clean.
**Основание:** `T032-notes/I4-wgpu.md` (Low-Medium cost, ровно 1 zed-commit
поверх `gfx-rs/wgpu` v29.0.3), разблокировано явным сообщением архитектора
2026-08-09.

## Что сделано

- Worktree `Source-wt-s5-wgpu` (ветка `t032-s5-wgpu-vendor`) создан от
  `dc5734e` (T032/S4).
- `wgpu/` — verbatim-копия cargo git checkout'а `zed-industries/wgpu@357a0c5`
  (`~/.cargo/git/checkouts/wgpu-423de87c978aca7f/357a0c5/`, `.git/` исключён,
  `rsync -a --exclude='.git'`). 39 MB на диске (checkout с `.git` — 116 MB).
  Никакие файлы внутри `wgpu/` не редактировались.
- `Source/Cargo.toml`:
  - `[workspace] exclude` → добавлен `"wgpu"` (по образцу существующего
    `"gpui-component"`) — внутренний 18-крейтовый workspace `wgpu/`
    остаётся самодостаточным, не мержится в members верхнего уровня.
  - `wgpu = { git = "https://github.com/zed-industries/wgpu.git", rev = "357a0c56..." }`
    → `wgpu = { path = "wgpu/wgpu" }`.
- `gpui_wgpu/Cargo.toml` — уже использовал `wgpu.workspace = true` (нет
  дублирующего git-пина), правка не потребовалась (стена #6 брифа).
- `Cargo.lock` (Source) — `cargo generate-lockfile --offline` прошёл без
  сети (стена #7); затем `cargo fetch` подтянул обычные registry-крейты.
- `gpui-component/Cargo.lock` — обновлён отдельным коммитом `44544d5`
  (см. «Отклонение от брифа» ниже): 9 строк `source = "git+...zed/wgpu..."`
  убраны у `naga`/`wgpu`/`wgpu-core`/`wgpu-hal`/`wgpu-types`/
  `wgpu-naga-bridge`/`wgpu-core-deps-*` — граф зависимостей не изменился,
  только источник.
- `Source/NOTICE` — секция «wgpu — path-vendored zed fork (T032/S5)»
  добавлена (Apache-2.0 OR MIT, upstream rev, delta-коммит, ссылка на
  `wgpu/PATCHES.md`).
- `wgpu/PATCHES.md` — создан: источник, delta-коммит `a466bc38` (текст,
  почему важен, verified-byte-for-byte), план на апстрим-мерж, обоснование
  структуры workspace (nested/excluded, не flatten).

## Diff-сверка коммита `a466bc38` (byte-for-byte)

```
$ diff ~/.cargo/git/checkouts/wgpu-423de87c978aca7f/357a0c5/wgpu-hal/src/gles/egl.rs \
       Source/wgpu/wgpu-hal/src/gles/egl.rs
$ echo "diff exit: $?"
diff exit: 0
```

Коммит в исходном checkout'е:

```
commit a466bc382ea747f8e1ac810efdb6dcd49a514575
Author: John Tur <john-tur@outlook.com>
Date:   Fri Mar 20 01:44:55 2026 -0400

    Add XCB display handle support to EGL backend

 wgpu-hal/src/gles/egl.rs | 24 +++++++++++++++++++++++-
 1 file changed, 23 insertions(+), 1 deletion(-)
```

Совпадает с I4-note §2 (+23/-1) и с брифом. Патч заменяет `todo!("xcb")` на
`EGL_EXT_platform_xcb`-путь. Файл после копирования — идентичен исходнику
(diff exit 0 выше), это единственная проверка, которая имеет значение для
стены #1.

## Решение по структуре workspace

`wgpu/` — нетронутый вложенный Cargo-workspace (18 членов: `naga`,
`wgpu-core`, `wgpu-hal`, `wgpu-types`, `wgpu-macros`, `wgpu-naga-bridge`,
`wgpu`, плюс bench/test/tooling-крейты, не используемые здесь).
`Source/Cargo.toml` **не** перечисляет `wgpu/*` в `members`; вместо этого
`wgpu` добавлен в `exclude` (как уже сделано для `gpui-component`), а
верхний workspace тянет только `wgpu = { path = "wgpu/wgpu" }` — все
внутренние pass-through зависимости (`wgpu-core`, `wgpu-hal`, ...) у
`wgpu/wgpu` уже объявлены как `{ version = "...", path = "./wgpu-core" }`
и резолвятся сами, независимо от того, кто зависит от верхнего `wgpu`.
Альтернатива (flatten всех 18 крейтов в `Source`'s members) была отвергнута:
потребовала бы правок манифестов внутри `wgpu/` (риск для стены #1 —
byte-for-byte) без выигрыша. Компромисс: `wgpu/Cargo.lock` (собственный
лок вложенного workspace) не используется при сборке из `Source/` —
резолвится единый `Source/Cargo.lock`; внутренний лок остался как есть
(не регенерировался, не удалялся) — vestigial-артефакт апстрима.
Подробности — `wgpu/PATCHES.md` §«Workspace structure decision».

## Отклонение от брифа (честно зафиксировано)

Брифом заявлен **один** коммит на выйгранное дерево worktree. По факту —
**два** коммита в `Source`:

1. `2044c98` — сам vendor-коммит (worktree → fast-forward merge).
2. `44544d5` — синхронизация `gpui-component/Cargo.lock`, обнаруженная
   только при прогоне 4-го baseline-потребителя (`gpui-component-story`)
   **после** мержа в основной `Source`, а не внутри worktree. Если бы
   `cargo build -p gpui-component-story` был прогнан внутри worktree до
   коммита, лок обновился бы в том же дереве и вошёл бы в `2044c98`.
   Дублирующей git-строки `wgpu` в этом локе не было — только `source =`
   аннотация на 9 уже существующих пакетов (`naga`, `wgpu`, `wgpu-core`,
   `wgpu-hal`, `wgpu-types`, `wgpu-naga-bridge`, 3× `wgpu-core-deps-*`),
   граф зависимостей не менялся. Риск оцениваю как низкий (тот же logical
   diff, что был бы в едином коммите, просто разбит на два), но фиксирую
   как процессное отклонение, а не как «всё по плану».

## Baseline (реальный gap: заявленных `/tmp/t032-baselines/*-before.log` от
2026-08-08 в среде не оказалось)

В брифе указано, что baseline «до» уже снят 2026-08-08. На момент
исполнения `/tmp/t032-baselines/` была пуста — файлов не было. Честно
зафиксировано как gap среды (не молчаливо подменено): baseline «до»
пересобран заново на чистом дереве `dc5734e` (тот же HEAD, что указан в
задаче) непосредственно перед началом изменений, тем же набором из 4
команд, что и «после».

## Grim до/после (smoke: `gpui/examples/text.rs`, тот же сценарий, что S3)

Реальный GPU-путь: Hyprland + `WAYLAND_DISPLAY=wayland-1`, `grim`
(не headless/swiftshader).

| | до (git 357a0c56, коммит `dc5734e`) | после (path-vendor, коммит `2044c98`) |
|---|---|---|
| build | `cargo build -p 'path+file://.../gpui#0.2.2' --example text` → exit 0, лог `/tmp/t032-s5-before/build.log` | то же, exit 0, лог `/tmp/t032-s5-after/build.log` |
| run.log | 0 bytes (нет паник) | 0 bytes (нет паник) |
| окно | «GPUI Typography», 945×1180 @ [3525,10] (Hyprland-тайлинг) | «GPUI Typography», 1265×1394 @ [10,36] (другой тайлинг — другая рабочая область, не связано с wgpu) |
| скрин | `/tmp/t032-s5-before/window.png` (+ `screen.png`) | `/tmp/t032-s5-after/window.png` (+ `screen.png`) |

Оба окна рендерят идентичный набор глифов/тестовых строк (ZedMono
тестовая таблица + "The quick brown fox…" каскад размеров/инверсий) —
визуально неотличимы помимо геометрии окна (тайлинг Hyprland решает
размер независимо от рендер-бэкенда, тот же эффект отмечен в S3-ноте).
Разница объяснена, не молчаливая. Честный pixel-diff (как в S3) не снимал —
геометрия окон разная по той же причине (`hyprctl movewindowpixel`
недоступен, HANDOFF-кейс из S3), визуальное сравнение через оба скрина
достаточно для вывода «нет регресса» на этом смоуке; XCB/EGL-путь конкретно
(nvidia-EGL/X11-XCB) этим смоуком не покрыт напрямую (сцена — Wayland через
Hyprland, GLES/EGL backend с Xcb display handle не обязательно является
активным путём в этой конкретной GPU/драйвер-конфигурации) — гарантия
непотери патча идёт от byte-for-byte diff кода (выше), не от этого рендера.

## Baseline-потребители (4, до и после)

| команда | до | после |
|---|---|---|
| `cargo check --workspace` (ChronOS) | exit 0, `/tmp/t032-baselines/chronos-before.log` | exit 0, `/tmp/t032-baselines/chronos-after.log` |
| `cargo check --workspace` (Chronos-lm) | exit 0, `/tmp/t032-baselines/chronoslm-before.log` | exit 0, `/tmp/t032-baselines/chronoslm-after.log` |
| `cargo test --workspace` (Chronos-FM) | exit 0, `/tmp/t032-baselines/chronosfm-before.log` | exit 0, `/tmp/t032-baselines/chronosfm-after.log` |
| `cargo build -p gpui-component-story` (Source/gpui-component) | exit 0, `/tmp/t032-baselines/gpuicomponent-before.log` | exit 0, `/tmp/t032-baselines/gpuicomponent-after.log` (регенерировал `gpui-component/Cargo.lock`, см. «Отклонение от брифа») |

«После» снят **после** fast-forward мержа `t032-s5-wgpu-vendor` → `main`
(baseline-потребители ссылаются на `../Source` по фиксированному пути, не
на worktree — прогон против worktree до мержа проверял бы неизменённый
main и дал бы ложно-зелёный результат по старому коду; это учтено).

## `cargo tree -i wgpu` (после)

```
wgpu v29.0.3 (/home/neo/projects/chronos-ecosystem/Source/wgpu/wgpu)
└── gpui_wgpu v0.1.0 (/home/neo/projects/chronos-ecosystem/Source/gpui_wgpu)
    └── gpui_linux v0.1.0 (/home/neo/projects/chronos-ecosystem/Source/gpui_linux)
        └── gpui_platform v0.1.0 (/home/neo/projects/chronos-ecosystem/Source/gpui_platform)
            [dev-dependencies]
            └── gpui v0.2.2 (/home/neo/projects/chronos-ecosystem/Source/gpui)
                └── ... (gpui_tokio, gpui_wgpu, gpui-animation, gpui_macros)
```

Path, не git — приёмочный критерий закрыт.

## Слияние worktree → main

`git worktree add ../Source-wt-s5-wgpu -b t032-s5-wgpu-vendor` от `dc5734e`
→ один коммит `2044c98` в worktree → `git merge --ff-only
t032-s5-wgpu-vendor` в основной `Source` (fast-forward, т.к. main не
продвигался за время работы) → `git worktree remove` + `git branch -d`.
Второй коммит `44544d5` сделан прямо в `main` (см. «Отклонение от брифа»).

## Осталось на git / вне скоупа

- `util_macros` / `http_client` (T030).
- `gpui_macos` / `gpui_windows` (cfg-gated, не собираются на Linux).
- `reqwest` / `zed-reqwest` fork (пин `c1566246`).
- `zed-scap` (DEFER, S2).
- Внутренний `wgpu/Cargo.lock` (vestigial, не используется при сборке из
  `Source/`; не трогался).
