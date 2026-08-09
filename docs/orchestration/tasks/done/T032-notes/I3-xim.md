# I3 — inventory: xim-rs (zed-xim)

**Задача:** сузить diff zed-форка xim-rs до IME-специфики; оценить vendor cost (монорепо `zed-xim` + `xim-ctext` + `xim-parser`).
**Тип:** read-only разведка. **Блокирует:** S4.
**Дата:** 2026-08-09 · git rev (Source) `a087521f`

---

## 1. Текущее состояние (Source) — production graph на Linux/X11

Объявление: `Source/gpui_linux/Cargo.toml:130-133`

```toml
xim = { git = "https://github.com/zed-industries/xim-rs.git", rev = "16f35a2c…",
        features = ["x11rb-xcb", "x11rb-client"],
        package = "zed-xim", version = "0.4.0-zed", optional = true }
```

Включается feature `x11` (строка 41: `"xim"`), который входит в `gpui default` через `gpui_linux/x11` → `gpui_platform/x11`.

Evidence `cargo tree` (x86_64-unknown-linux-gnu, offline):

```
$ cargo tree --offline -i zed-xim --target x86_64-unknown-linux-gnu -e features | head
zed-xim v0.4.0-zed (…/zed-industries/xim-rs.git?rev=16f35a2…)
├── zed-xim feature "client"
│   └── zed-xim feature "x11rb-client"
│       └── gpui_linux v0.1.0 … ← feature "x11" → `xim`
```

**Вывод:** zed-xim активен (Linux/X11 IME). `defer` неприменим — `x11` в default.

Usage: `Source/gpui_linux/src/linux/x11/xim_handler.rs` — `use xim::{AHashMap, AttributeName, Client, ClientHandler, InputStyle, ForwardEventFlag, PreeditDrawStatus, Feedback}`; проброс `XimPreeditEvent`/`XimCommitEvent` в `X11Window`.

## 2. Список zed/fix-коммитов (diff сужен до IME-специфики)

Checkout: `~/.cargo/git/checkouts/xim-rs-f2169dfc4c0688a9/16f35a2/` — форк от `Riey/xim-rs`, `zed-xim v0.4.0-zed`, `MIT`, `rust-version 1.64`.

`git log --oneline -20` — слой zed/IME-фиксов поверх базовой истории Riey:

```
16f35a2 Merge pull request #1 from mikayla-maki/main   <- HEAD (наш пин)
471c10c prep for crates.io
c0a70c1 throw no xim server error in filter_event, when error event with im_window received
d50d461 Reset Request and destroy ic reply implemented
62ccb42 fix: xim-ctext: Support all ctext encodings
42d10bf fix: Client: handle preedit_string encoding error
7695ee0 fix: client: Handle fcitx4's empty reply case
5550e50 fix: xim-parser: Handle Feedback as bitflag
f8bbbd8 fix: xim-ctext: Fix decode macro to track bytes
be24457 chore: example: Fix handler to handle all features
b997dac chore: Add client handle_forward_event example
3b75455 Bump version to 0.4.0
043dd6c Bump x11rb to v0.13.0
… (далее базовая история Riey)
```

IME-специфика zed-форка — это **robustness в протоколе IME/XIM для CJK/JP-ввода**:
- `xim-ctext` — поддержка всех compound-text кодировок + фикс `decode` macro (раскодировка preedit/commit строк из IME).
- `xim-parser` — `Feedback` как bitflag (иконки/обратная связь preedit).
- `Client` — обработка fcitx4 empty reply, `preedit_string` encoding error, no-xim-server error, `Reset Request`/destroy IC reply.

Вывод: дифф не про вендорный API, а про набор IME-фиксов на уровне парсера/ctext/client — при path-vendor их надо переносить целиком (см. §3).

## 3. Какие subcrates вендорить вместе + crates.io published?

**Монорепо checkout** (`[workspace] members`: `.`, `xim-ctext`, `xim-gen`, `xim-parser`):
- `zed-xim` (главный) → dep: `xim-parser` (path, v0.2.0), `xim-ctext` (path, v0.3.0).
- `xim-parser` → **build-dep** `xim-gen` (path, v0.1.0), опционально feature `bootstrap`.
- `xim-ctext` → `encoding_rs`.

**crates.io — всё опубликовано отдельно:**
- `zed-xim` **`0.4.0-zed`** — ✅ published (2025-10-05, `mikayla-maki`/zed, не yanked). Features совпадают с нужными gpui: `client`, `x11rb-client`, `x11rb-xcb` (+ `std`). Dep low-level на **published** `xim-parser ^0.2.0`, `xim-ctext ^0.3.0`, `x11rb ^0.13`, `x11-dl ^2.18.5` (opt), `ahash ^0.8`, `hashbrown ^0.14.0`.
- `xim-parser` — ✅ published (v0.2.x, latest 0.2.2; 0.2.1 присутствует).
- `xim-ctext` — ✅ published (v0.3.x имеется; latest 0.4.1).

**Значит:**
- Вариант **crates.io**: достаточно только `zed-xim = "0.4.0-zed"` с features — subcrates не вендорить (подтянутся published `xim-parser`/`xim-ctext`).
- Вариант **path-vendor**: вендорить **всё монорепо** — `zed-xim` + `xim-parser` + `xim-ctext` (+ `xim-gen`, т.к. build-dep `xim-parser`, хотя gpui не использует feature `bootstrap`, т.е. `xim-gen` не скомпилируется; но для «честно полного» vendor включён).


**Caveat по parity (важно для IME):** пин `16f35a2` — merge-коммит `mikayla-maki/main` (см. §2), который несёт IME-фиксы. Published `0.4.0-zed` датирован той же эпохой и пред-паб-коммит `471c10c` в истории — формально published `0.4.0-zed` может быть **на 1 коммит/merge раньше** пина `16f35a2`. Манифест и deps совпадают (x11rb ^0.13 и т.д.), но точную равноценность IME-фиксов по исходникам проверить read-only нельзя — закладывается в smoke S4 (см. §5).

## 4. Рекомендация: **`crates.io`** (`zed-xim = "0.4.0-zed"`)

Заменить git-пин `16f35a2…` на published `zed-xim = "0.4.0-zed"` с теми же features `["x11rb-xcb","x11rb-client"]` (добавить в `[workspace.dependencies]`). Subcrates (`xim-parser`, `xim-ctext`) не вендорить — они published и придут транзитивно.

Почему:
1. zed публикует `zed-xim` + у него весь низ (parser/ctext) уже published — артефакт-исходник существует, path-vendor избыточен.
2. Feature-set/декларация совпадают с нужным gpui (`x11rb-xcb`, `x11rb-client`).
3. Вендор монорепо (3+1 крейта) — лишняя поддержка без выигрыша по IME, т.к. фиксы те же.
4. `defer` неприменим — `x11` в default.

**Fallback:** если S4 докажет, что published `0.4.0-zed` теряет какой-то IME-фикс из рева `16f35a2` — тогда `path-vendor` рева `16f35a2` целиком (`zed-xim`+`xim-parser`+`xim-ctext`(+`xim-gen`)) в `Source/`.

## 5. Grim/Smoke plan для S4 (X11 vs Wayland)

1. **X11 (обязательно для xim-доказательства):** запустить минимальный gpui-апп под **real X11 или XWayland** (НЕ чистый Wayland), с feature `x11`. Включить реальный IME с XIM-фронтендом — **fcitx5 (xim)** или **ibus**.
2. **Живой ввод, не только `cargo check`:** в текстовом поле набрать CJK/Japanese (e.g. китайские иероглифы / японскую ромадзи→кандзи): на кадре должны появиться предредактируемая строка (PreeditDraw → `XimPreeditEvent`) и итоговый коммит (`XimCommitEvent`), текст ложится в editor; отследить отсутствие «пропавшего» ввода (проверка фиксов fcitx4 empty reply / encoding).
3. **Ограничение:** **pure Wayland smoke НЕ докажет IME xim** (xim — только X11/`gpui_linux/x11`); Wayland-прогон годится только как baseline, что x11 сборка не крашит при запуске под XWayland. Критерий pass для реко crates.io: IME-ввод (preedit+commit) работает идентично current pin `16f35a2`.

## Definition of done

- ✅ evidence `cargo tree -i zed-xim` + usage `xim_handler.rs`
- ✅ список zed/fix-коммитов (сужен до IME-специфики, p.2)
- ✅ какие subcrates вендорить: при path-vendor — `zed-xim`+`xim-parser`+`xim-ctext`(+`xim-gen`); при crates.io — ни одного (p.3)
- ✅ crates.io `zed-xim` published: Да, `0.4.0-zed` (+ `xim-parser`/`xim-ctext` published) (p.3)
- ✅ рекомендация crates.io + fallback path-vendor + smoke plan X11 vs Wayland (p.4–5)
- ✅ zero git changes от агента I3 (`Source/` — чтение только; существующие локальные правки не тронуты).