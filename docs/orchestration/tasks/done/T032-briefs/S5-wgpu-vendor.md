# Agent S5 — wgpu — VENDOR (разблокирован 2026-08-09)

**Статус:** разблокирован явным сообщением архитектора 2026-08-09
(«wgpu не должен быть залоченным») — снимает условие #3 из
`S5-wgpu-blocked.md` («explicit user message to start wgpu vendoring»).
**Тип:** реализация, шаг 5 (рендер, ядро) — **самый рискованный шаг T032**.
**Старт:** после merge S1–S4 (уже в `Source` на HEAD `dc5734e`).
**Worktree:** обязателен (не мейн-дерево `Source/`) — это ядро рендера.

## Источник решения

`T032-notes/I4-wgpu.md` — read-only inventory, cost **Low-Medium**:
- checkout 116 MB, 18 крейтов workspace (naga + wgpu-core/hal/types/…)
- **ровно 1 zed-commit** поверх `gfx-rs/wgpu` v29.0.3: `a466bc38`
  «Add XCB display handle support to EGL backend» (`wgpu-hal/src/gles/egl.rs`, +23/-1)
- sole consumer: `gpui_wgpu` (path crate, 6 .rs + 2 .wgsl)

## Цель

Снять `git = "https://github.com/zed-industries/wgpu.git", rev = "357a0c5"`
(`Source/Cargo.toml:88`) → **path-vendor** по схеме S1 (нет published
`zed-wgpu` на crates.io — crates.io-опция, которую использовали S3/S4,
здесь не существует).

## Стены (называем явно — это ядро рендера, не HTTP-клиент)

1. **XCB/EGL-коммит `a466bc38` обязан выжить byte-for-byte.** Это не
   косметика — поведенческий патч (X11/EGL display handle). Потерять его =
   молчаливый регресс на Hyprland/X11+nvidia-EGL путях. Diff коммита —
   в `I4-wgpu.md` §2, сверить построчно после копирования src.
2. **Grim до/после — обязателен, не опционален.** `cargo check` зелёный
   ничего не доказывает для рендер-ядра (см. Стену 2 T032, уже
   применённую в S3). Сценарий: тот же smoke, что у S3
   (`gpui/examples/text.rs` или живой ChronOS bar) — реальный GPU-путь,
   не headless/swiftshader заглушка, если есть выбор.
3. **Четыре потребителя** (ChronOS, Chronos-lm, Chronos-FM, gpui-component-story)
   зелёные до и после — baseline `/tmp/t032-baselines/*-before.log` от
   2026-08-08 уже есть для «до»; снять «после» тем же набором команд.
4. **NOTICE + PATCHES.md обязательны** (path-vendor, как S1): PATCHES.md
   документирует коммит `a466bc38` как единственное отклонение от
   `gfx-rs/wgpu` v29.0.3 — откуда взят, зачем, что будет при апстрим-мерже.
5. **Один коммит в `Source/`**, как S1–S4. Hot files: `Source/Cargo.toml`,
   `Cargo.lock`, `NOTICE` — один writer.
6. **Не трогать**: `gpui_wgpu` рендер-логику (`wgpu_renderer.rs`,
   `wgpu_context.rs`, шейдеры) — только манифест-переключение git→path.
   Если сборка требует правок в потребителе — стоп, отдельная заметка,
   не «заодно поправить».
7. **Lockfile regen без сетевого апстрима** — `cargo generate-lockfile`
   должен резолвить path-члены, не бить по сети за git rev.

## Не крысить (вне скоупа даже сейчас)

- `zed-reqwest` git fork (остаётся, §4 отчёта T032)
- `gpui_macos`/`gpui_windows` git-хвосты
- `zed-scap` (DEFER, S2)

## Шаги

1. Скопировать checkout `zed-industries/wgpu@357a0c56` в `Source/wgpu*`
   (или единый `Source/wgpu` workspace, если 18 членов того требуют —
   решение по структуре за исполнителем, задокументировать в ноте).
2. `Source/Cargo.toml:88` — git-строка → path/workspace-member.
3. `gpui_wgpu/Cargo.toml` — сверить, что зависимость идёт через workspace,
   не дублирует git-пин.
4. Grim «до» (если ещё нет свежего — S3 baseline устарел, снять новый
   на HEAD `dc5734e` перед сменой).
5. Внести изменение, собрать.
6. Grim «после» — тот же сценарий, что «до».
7. Diff-сверка `a466bc38` byte-for-byte (git diff чекаута vs скопированный src).
8. Четыре baseline-потребителя.
9. `NOTICE` + `PATCHES.md`.
10. Коммит: `vendor : wgpu — path-vendor zed@357a0c5 (XCB/EGL patch preserved) (T032/S5)`.
11. `T032-notes/S5-wgpu.md` — то же по форме, что S1/S3-ноты.

## Выход

- коммит в `Source/`
- `T032-notes/S5-wgpu.md` (пути grim before/after, diff-сверка коммита,
  4 baseline-логи, решение по структуре workspace)
- `NOTICE`/`PATCHES.md`

## Приёмка

- [ ] нет `git = "...zed-industries/wgpu..."` в `Source/Cargo.toml`
- [ ] `cargo tree -i wgpu` → path, не git
- [ ] коммит `a466bc38` присутствует byte-for-byte в скопированном src (diff приложен)
- [ ] grim до/после того же сценария, честная разница объяснена (не молчаливая)
- [ ] 4 baseline zелёные (до и после)
- [ ] NOTICE + PATCHES.md на месте
- [ ] отчёт (не обязательно отдельный R-report — можно дополнить существующий T032-отчёт §по S5)
