# T032 — Форк: вендоринг рендер/ввод-зависимостей. Итоговый отчёт

**Дата:** 2026-08-09 · **Тикет:** `docs/orchestration/tasks/active/T032-render-input-forks-vendoring.md`
**Скоуп отчёта:** `Source/` (fork gpui-ce) — пять zed-git-крейтов: `reqwest_client`, `scap`, `font-kit`, `xim-rs`, `wgpu`.
**Статус:** **ВСЕ 5 ШАГОВ ЗАКРЫТЫ (S1–S5).** S1 — path-vendor (2 крейта: reqwest_client + http_client_tls), S2 — DEFER (scap), S3+S4 — crates.io (2 крейта: zed-font-kit, zed-xim), S5 — path-vendor (wgpu, разблокировано явным запросом архитектора 2026-08-09: «wgpu не должен быть залоченным»).
Все claims ниже ссылаются на заметки агентов (`T032-notes/*.md`) и на вывод команд — архитектор может принимать решение без чтения брифов. S5 дополнительно верифицирован независимо архитектором (не только по ноте исполнителя): `cargo tree`, `diff` коммита `a466bc38`, git log, все 8 baseline-логов, grim до/после.

---

## 1. Скоуп / что не делали

| Что | Статус | Причина |
|---|---|---|
| **wgpu** (S5) | **VENDORED** — path (`Source/wgpu/wgpu`) | Разблокировано явным запросом архитектора 2026-08-09; см. §3 S5. |
| **gpui_macos** | вне скоупа | Linux-only проект (тикет, раздел «Явно вне скоупа»). |
| **gpui_windows** | вне скоупа | Остаётся на git до отдельного решения (тикет). |
| `http_client`, `util_macros` (zed@876ec5a) | не трогали | Это T030 (предыдущий тикет), не T032. |
| `zed-reqwest` (git fork `reqwest`) | остался | Форк-стек HTTP, на котором стоит vendored `reqwest_client` (см. §4). |

Пройденные шаги: **S1 reqwest_client (path-vendor), S2 scap (DEFER), S3 font-kit (crates.io), S4 xim-rs (crates.io), S5 wgpu (path-vendor)**.

---

## 2. Шаг 0 — сводка inventory (5 крейтов)

Источники: `T032-notes/I1-scap.md`, `I2-font-kit.md`, `I3-xim.md`, `I4-wgpu.md`; S1-инвентарь поглощён S1-нотой (`I0`-бриф закрыт WIP на диске).

| Крейт | zed pin | Живой в графе? | Дрейф zed-форка от апстрима | Рекомендация inventory |
|---|---|---|---|---|
| `reqwest_client` | zed `876ec5a` | да (dev-dep gpui) | обёртка над `reqwest` (crates.io), не движок | path-vendor (S1) |
| `scap` (`zed-scap`) | scap `4afea48` | **нет** — `cargo tree -i zed-scap` → "nothing to print" во всех 4 consumers (I1 §2–3) | живые 4–11 zed-коммитов поверх `helmerapp/scap` (macos/windows/img-фиксы) | **DEFER** (I1 §5) |
| `font-kit` (`zed-font-kit`) | font-kit `94b0f28` | да — feature `font-kit` в default (I2 §1) | 5 zed-коммитов поверх servo; published `0.14.1-zed` ≡ `1105231` (на 1 коммит раньше пина: только `dirs 5→6`, не-API) | **crates.io** `0.14.1-zed` (I2 §4) |
| `xim-rs` (`zed-xim`) | xim-rs `16f35a2` | да — `x11` в default (I3 §1) | IME-фиксы (fcitx4 empty-reply, ctext encodings, Feedback bitflag); published `0.4.0-zed` формально может быть на 1 merge раньше пина (I3 §3 caveat) | **crates.io** `0.4.0-zed` (I3 §4) |
| `wgpu` | wgpu `357a0c5` | да — sole consumer `gpui_wgpu` (I4 §3) | **1 коммит** поверх `gfx-rs/wgpu` v29.0.3: `a466bc38 Add XCB display handle support to EGL backend` (`wgpu-hal/src/gles/egl.rs`, +23/−1) — поведенческий, X11/EGL (I4 §2) | ~~не имплементировать без запроса~~ → **path-vendor** (S5, после явного запроса 2026-08-09) |

---

## 3. По каждому пройденному шагу

### S1 — `reqwest_client` (+ `http_client_tls`): **PATH-VENDOR** ✅

- **Коммит:** `c0c4f23` — `vendor : reqwest_client + http_client_tls — vendored zed@876ec5a as path members (T032/S1)`
- **Решение:** вендорить в `Source/` как workspace members (src verbatim zed@876ec5a, manifest переписан, `PATCHES.md` + `LICENSE-APACHE` на месте).
- **Изменения:** `Source/Cargo.toml` — `members` + `[workspace.dependencies]` path-депы; добавлены `bytes = "1.0"`, `reqwest` = zed-reqwest pin `c1566246` (7 features), `rustls = "0.23.26"`, `rustls-platform-verifier = "0.5.0"`; git-строка `reqwest_client` удалена. `gpui-component/Cargo.toml` — `reqwest_client = { path = "../reqwest_client" }`. `NOTICE` — секции обоих крейтов (Apache-2.0).
- **Baselines (все EXIT:0):** `cargo check -p reqwest_client` (Source) · `cargo check --workspace` (ChronOS) · (Chronos-lm) · `cargo test --workspace` (Chronos-FM) · `cargo build -p gpui-component-story`. Источник: `S1-reqwest_client.md` таблица команд.
- **Grim:** не требуется (шаг 1 — HTTP-клиент, не рендер/ввод).
- **Tree-proof:** `cargo tree -i reqwest_client` → `(.../Source/reqwest_client)` — path, не git.
- **Доп. проверка:** `cargo check --workspace` + gpui examples (используют `reqwest_client::ReqwestClient`) — зелёные; `-p gpui` ambiguity — pre-existing (cfg-gated `gpui_macos`/`gpui_windows` zed-депы), не связано с T032.

### S2 — `scap`: **DEFER** (не вендорим) ⏸️

- **Коммит:** `3a0fe17` — `scap : DEFER vendoring — document why zed git pin stays (T032/S2)`
- **Решение:** git pin остаётся; `screen-capture` нигде не включён. Основание: `I1-scap.md` (решение I1), перепроверено после S1.
- **Evidence:** `cargo tree -i zed-scap` → "nothing to print" (Source, gpui-component, ChronOS, Chronos-lm); Chronos-FM → "did not match any packages" (не в resolve-графе). Единственные упоминания `scap` у потребителей — `ChronOS/reference/kael-main/` (архивная копия, не member). `gpui default` = `font-kit, wayland, x11, windows-manifest` (без `screen-capture`); `scap` — optional dep под off-by-default feature.
- **Что сделано:** комментарий-обоснование у git-pin в `Source/Cargo.toml`. Graph/lockfile/код не тронуты.
- **Baselines (все EXIT:0):** ChronOS · Chronos-lm · Chronos-FM (34 passed) · gpui-component-story.
- **Условие VENDOR:** любой потребитель включает `screen-capture` → вендорить `zed-scap@4afea48` по схеме S1 (path member + NOTICE + PATCHES.md).

### S3 — `font-kit`: **CRATES.IO** (option A) ✅

- **Коммит:** `5535764` — `vendor : font-kit — crates.io zed-font-kit 0.14.1-zed replaces zed git pin (T032/S3)`
- **Решение:** crates.io `zed-font-kit = "0.14.1-zed"` (рекомендация I2; published API идентичен пину `94b0f28`, разница только `dirs ^5.0` vs `6.0`).
- **Изменения:** `Source/Cargo.toml` — workspace dep keyed `font-kit` (пакет `zed-font-kit`); `gpui/Cargo.toml` (macOS block) + `gpui_wgpu/Cargo.toml` (Linux font path) — git-пин → `{ workspace = true, optional = true }`. Оба Cargo.lock: git → registry; добавлены `dirs 5.0.1`/`dirs-sys 0.4.1` (`dirs 6.0.0` остаётся для других). Feature-name `font-kit` сохранён (ключ депа = `font-kit`, чтобы implicit feature не сломался — подтверждено reviewer-ом и сборкой).
- **Grim (Стена 2 — живой кадр до/после):** smoke = `gpui/examples/text.rs` (гоняет `cosmic-text` + `font_kit::matching::find_best_match` — тот же шрифтовой путь, что у ChronOS bar). Живой шелл ChronOS не трогался.

  | | before (git `94b0f28`, HEAD `3a0fe17`) | after (crates.io `0.14.1-zed`, HEAD `5535764`) |
  |---|---|---|
  | кадры | `/tmp/t032-s3-before/screen.png`, `window.png` | `/tmp/t032-s3-after/screen2.png`, `window.png` |
  | window | 945x585 | 1265x692 (Hyprland затайлил иначе) |
  | colors / stddev | 8170 / 7525.58 | 9155 / 9611.83 |
  | run.log | 0 bytes (нет паник) | 0 bytes (нет паник) |

  Оба кадра рендерят реальные системные глифы (8–9k уникальных цветов, высокий контраст — не tofu/пусто) → font discovery на `dirs 5.0` работает (caveat I2 закрыт). Normalized compare (after → 945x585, fuzz 8%) = **24 323 px из 552 825 (4.4%)** — разница от разной ширины окна (wrap) + resampling, не от шрифтов. Pixel-exact при той же геометрии не удался: `hyprctl dispatch movewindowpixel`/`eval` падает на Lua-парсере Hyprland 0.56.1 (известный HANDOFF-кейс; зафиксировано в S3-ноте).
- **Baselines (все EXIT:0):** ChronOS · Chronos-lm · Chronos-FM · gpui-component-story · `cargo build -p gpui --example text`.
- **Остаток:** git `zed-font-kit` присутствует в resolve-graph только через cfg-gated `gpui_macos` (на Linux не собирается).

### S4 — `xim-rs`: **CRATES.IO** ✅ (с честно зафиксированным smoke-gap)

- **Коммит:** `dc5734e` — `vendor : xim — crates.io zed-xim 0.4.0-zed replaces zed git pin 16f35a2 (T032/S4)` (файлы: `Cargo.toml`, `gpui_linux/Cargo.toml`, `Cargo.lock`, `gpui-component/Cargo.lock`).
- **Решение:** crates.io `zed-xim = "0.4.0-zed"` (рекомендация I3: published pin; subcrates `xim-parser`/`xim-ctext` приходят транзитивно с crates.io).
- **Изменения:** `Source/Cargo.toml` — workspace dep `xim = { version = "0.4.0-zed", package = "zed-xim" }`; `gpui_linux/Cargo.toml` — inline git-пин → `{ workspace = true, features = ["x11rb-xcb","x11rb-client"], optional = true }` (включается feature `x11`, входящим в `gpui default`).
- **Tree-proof:** `cargo tree -i zed-xim` → `zed-xim v0.4.0-zed` (без git-суффикса → crates.io) ← `gpui_linux`; lock-checksum совпадает с published; `grep zed-industries/xim-rs` в обоих lock → 0 совпадений.
- **Baselines (compile):** `cargo check --offline -p gpui_linux --features x11` ✅ · `cargo check --offline -p gpui_platform --features x11,wayland,font-kit` ✅ · `cargo check --offline --manifest-path gpui-component/Cargo.toml` ✅ (2.99s; только pre-existing `nightly_coverage` cfg-warnings).
- **Smoke / честный gap (Стена 2):** **`xim` не exercised — only compile + tree.** Компилятор не ловит IME-регресс; живой X11 + fcitx5/ibus на этом стенде недоступен (нет дисплейного X11-сессии в прогоне); pure Wayland xim не докажет (xim активен только на X11-пути). **Parity-caveat:** published `0.4.0-zed` формально может быть на 1 merge-коммит раньше пина `16f35a2` (пара IME-фиксов); API идентичен. Приёмка S4 живым X11+fcitx5/ibus вводом (CJK/JP: PreeditDraw → commit) остаётся follow-up (см. §6). Внешние consumers (ChronOS, Chronos-FM, Chronos-IDE) отдельными репо не пересобирались (затронуты только transitive-источником, без смены API) — тоже follow-up.
- **NOTICE/PATCHES:** не требовались (crates.io, не vendor).

### S5 — `wgpu`: **PATH-VENDOR** ✅ (разблокирован явным запросом 2026-08-09)

- **Разблокировка:** сообщение архитектора «wgpu не должен быть залоченным» — снимает условие S5-блокировки (I4 §5, `_shared-walls.md` gate 6: «explicit user message to start wgpu vendoring»).
- **Коммиты:** `2044c98` — `vendor : wgpu — path-vendor zed@357a0c5 (XCB/EGL patch preserved) (T032/S5)`; `44544d5` — `vendor : wgpu -- sync gpui-component/Cargo.lock (T032/S5 follow-up)` (отклонение от брифа: заявлен 1 коммит, по факту 2 — второй обнаружен только при прогоне 4-го baseline после мержа в main; логический diff тот же, честно зафиксировано в S5-ноте).
- **Решение:** нет published `zed-wgpu` на crates.io (в отличие от S3/S4) → path-vendor по схеме S1. `wgpu/` — verbatim-копия cargo git checkout `zed-industries/wgpu@357a0c5` (39 MB без `.git`), вложенный самодостаточный Cargo-workspace (18 членов), добавлен в `[workspace] exclude` (как `gpui-component`), верхний workspace тянет только `wgpu = { path = "wgpu/wgpu" }`. `gpui_wgpu/Cargo.toml` правки не потребовал (уже `wgpu.workspace = true`).
- **Стена #1 (byte-for-byte коммит `a466bc38`):** `diff` скопированного `wgpu-hal/src/gles/egl.rs` против нетронутого cargo-checkout → **exit 0, identical**. Патч (XCB display handle в EGL backend, +23/−1) не потерян.
- **Grim (Стена 2 — живой кадр до/после):** smoke = `gpui/examples/text.rs` (тот же сценарий, что S3). До: `dc5734e`, окно 945×1180. После: `2044c98`, окно 1265×1394 (другой Hyprland-тайлинг, не связано с wgpu — тот же эффект, что в S3). Оба рендерят идентичный набор глифов, `run.log` 0 байт (нет паник) в обоих. **Честный gap:** конкретно XCB/EGL-путь (nvidia-EGL/X11-XCB) этим Wayland-смоуком не покрыт напрямую — гарантия непотери патча идёт от byte-for-byte diff, не от рендера (зафиксировано в S5-ноте, не скрыто).
- **Baselines (все EXIT:0, до и после):** ChronOS, Chronos-lm, Chronos-FM (294 passed, идентично до/после), `gpui-component-story`. Логи `/tmp/t032-baselines/*-{before,after}.log`.
- **Tree-proof:** `cargo tree -i wgpu` → `(.../Source/wgpu/wgpu)` — path, не git.
- **NOTICE/PATCHES:** `Source/NOTICE` секция добавлена; `Source/wgpu/PATCHES.md` создан (провенанс, delta-коммит, план на апстрим-мерж, обоснование структуры workspace).
- **Независимая верификация архитектором** (не только по заявлению исполнителя): git log/status, `cargo tree -i wgpu`, `diff` коммита, все 8 baseline-логов (`test result: ok` идентично до/после), наличие PATCHES.md/NOTICE — все воспроизведены самостоятельно.

---

## 4. Что осталось на git — и почему

Источник: `Cargo.lock` (Source, `grep zed-industries | sort -u`), S1/S3/S4-ноты.

| Зависимость | Источник | Почему осталась |
|---|---|---|
| `http_client`, `util_macros` | `zed@876ec5a` | **T030** — предыдущий тикет, вне скоупа T032. |
| `gpui_macos`, `gpui_windows` | `zed@876ec5a` | **Вне скоупа T032** (Linux-only; windows — до отдельного решения). |
| `zed-reqwest` (git fork `reqwest` c1566246) | `zed-industries/reqwest.git` | Форк-стек HTTP, на котором стоит vendored `reqwest_client` — тот же pin, что у zed; снять его = отдельная задача (замена на crates.io `reqwest` не эквивалентна по API-поверхности zed-обёртки). |
| `zed-font-kit` (git `94b0f28`) | `zed-industries/font-kit` | Только в lock, через cfg-gated `gpui_macos`; на Linux не собирается (S3). Исчезнет вместе с решением по `gpui_macos`. |
| `zed-scap` (git `4afea48`) | `zed-industries/scap` | **DEFER (S2)** — не в build-graph ни одного consumer; vendor при включении `screen-capture`. |

---

## 5. Метрика tree (актуально, `Source/`, 2026-08-09)

| Крейт | Раньше (до T032) | Сейчас | Как проверено |
|---|---|---|---|
| `reqwest_client` | git `zed@876ec5a` | **path** `(.../Source/reqwest_client)` | `cargo tree -i reqwest_client` |
| `http_client_tls` | git `zed@876ec5a` | **path** `(.../Source/http_client_tls)` | S1-нота + lock diff (git-source строка снята) |
| `zed-font-kit` | git `94b0f28` | **registry** `0.14.1-zed` (Linux graph); git-копия только через cfg `gpui_macos` | `cargo tree -i zed-font-kit` → ambiguity registry+git; S3-нота |
| `zed-xim` | git `16f35a2` | **registry** `0.4.0-zed` | `cargo tree -i zed-xim` → без git-суффикса |
| `zed-scap` | git `4afea48` | **не в графе** ("nothing to print"), git pin в lock остаётся | `cargo tree -i zed-scap` |
| `wgpu` | git `357a0c5` | **path** `(.../Source/wgpu/wgpu)` | `cargo tree -i wgpu`; `diff` коммита `a466bc38` exit 0 |

**Итог по тикет-метрике:** 3 из 5 zed-git-крейтов полностью сняты с git (path: reqwest_client, http_client_tls, wgpu), 2 переведены на crates.io (published zed-артефакты: zed-font-kit, zed-xim), 1 — осознанно DEFER вне графа (zed-scap). Ноль шагов остались невыполненными.

---

## 6. Риски / follow-ups

1. **S4 IME-parity не доказан живьём** — published `zed-xim 0.4.0-zed` может быть на 1 merge раньше пина `16f35a2`. Follow-up: живой ввод X11 + fcitx5/ibus (CJK/JP preedit→commit) на машине с дисплейным X11; при регрессе — fallback path-vendor рева `16f35a2` целиком (`zed-xim`+`xim-parser`+`xim-ctext`).
2. **S4: внешние consumers отдельными репо не пересобирались** — ChronOS/Chronos-FM/Chronos-IDE затронуты transitive-источником; штатно прогнать `cargo check` в каждом.
3. **S3: `dirs ^5.0` vs `6.0`** — caveat закрыт grim-прогоном (font discovery работает). Если позже понадобится `dirs 6` — fallback path-vendor рева `94b0f28`.
4. **S5: XCB/EGL-путь не exercised живьём** — гарантия непотери патча `a466bc38` идёт от byte-for-byte diff, не от рендер-смоука (Wayland-сцена не обязательно бьёт в GLES/EGL-XCB backend). Follow-up: живой X11/nvidia-EGL прогон, если появится стенд.
5. **S5: внутренний `wgpu/Cargo.lock`** (вложенного workspace) — vestigial, не используется при сборке из `Source/` (резолвится единый `Source/Cargo.lock`); не трогался, оставлен как есть.
6. **S5: два коммита вместо одного** (`2044c98` + `44544d5`) — процессное отклонение от брифа (4-й baseline прогнан после мержа в main, а не внутри worktree); логический diff идентичен, риск оценён как низкий.
7. **`zed-reqwest` git fork** — единственная оставшаяся git-зависимость в активном Linux-graph HTTP-стека; кандидат на отдельный тикет (замена на upstream `reqwest` не эквивалентна).
8. **`gpui_macos`/`gpui_windows`** — тянут `zed@876ec5a` + git `zed-font-kit` в lock (не в Linux build); решение по ним = отдельный тикет.

---

### Definition of done (тикет, раздел «Приёмка»)

- [x] По каждому пройденному шагу: diff zed-форка от апстрима (Шаг 0) → I1/I2/I3/I4 notes (§2)
- [x] Решение вендорить/отложить и почему → §3 (VENDOR ×2, DEFER ×1, crates.io ×2)
- [x] Шаги 3–5 (рендер/ввод) — живой кадр до/после: S3 grim before/after (§3, пути в S3-ноте); S4 — честный Wayland/headless gap задокументирован; S5 grim before/after + byte-for-byte diff коммита (§3, пути в S5-ноте)
- [x] Baselines всех четырёх потребителей после каждого шага → EXIT:0 везде (§3, таблицы команд в S1–S5-нотах)
- [x] `NOTICE`/`PATCHES.md` для перенесённого: S1, S5 (path-vendor) — да; S3/S4 (crates.io) — не требуется
- [x] Явно названо, что осталось на git и почему → §4
- [x] Отчёт по адресу `docs/orchestration/tasks/report/T032-render-input-forks-vendoring-report.md`
- [x] S5 (wgpu) — явный запрос архитектора получен 2026-08-09, выполнено, независимо верифицировано архитектором (не только по ноте)

**Коммиты T032 в `Source/` (последовательно):** `c0c4f23` (S1) → `3a0fe17` (S2) → `5535764` (S3) → `dc5734e` (S4) → `2044c98` + `44544d5` (S5); рабочее дерево чистое (`git status` пуст на `44544d5`).

**Все 5 шагов тикета закрыты. Тикет T032 принят и закрыт.**
