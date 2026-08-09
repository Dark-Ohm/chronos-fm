# I2 — inventory: font-kit (zed-font-kit)

**Задача:** оценить расхождение zed-форка от servo/font-kit и выбрать cheapest: `crates.io zed-font-kit 0.14.1-zed` vs path-vendor.
**Тип:** read-only разведка. **Блокирует:** S3.
**Дата:** 2026-08-09 · git rev (Source) `a087521f`

---

## 1. Текущее состояние (Source) — font-kit **в production graph**

Объявления:
- `Source/gpui/Cargo.toml:20` → `default = ["font-kit","wayland","x11","windows-manifest"]` — feature **включена в default**.
- `Source/gpui/Cargo.toml:123` → `font-kit = { git="…/zed-industries/font-kit", rev="94b0f281…", package="zed-font-kit", version="0.14.1-zed", optional=true }`.
- `Source/gpui_wgpu/Cargo.toml:38` → то же (optional), `:16` → `font-kit = ["dep:font-kit"]`.
- `Source/gpui_platform/Cargo.toml:16` → `font-kit = ["gpui_macos/font-kit"]` (только macOS).
- Usage: `gpui_wgpu/src/cosmic_text_system.rs` под `#[cfg(feature="font-kit")]` (стр. 622, 642, 823–855) вызывает `font_kit::matching::find_best_match(...)` и `font_kit::properties::Properties { Style::…, Weight, Stretch }` — это источник шрифтов для `cosmic-text`.

Evidence `cargo tree` (x86_64-unknown-linux-gnu, offline):

```
$ cargo tree --offline -i zed-font-kit --target x86_64-unknown-linux-gnu -e features
zed-font-kit v0.14.1-zed (…/zed-industries/font-kit?rev=94b0f281…)
├── zed-font-kit feature "default"
│   └── gpui_wgpu v0.1.0 (…/Source/gpui_wgpu)
│       └── gpui_wgpu feature "font-kit" ← gpui_linux → (wayland/x11) → gpui default
```

**Вывод:** в отличие от scap (I1), font-kit **активен** — это загрузка системных шрифтов для текстового рендера на Linux. `defer` неприменим.

## 2. zed-коммиты поверх servo (`git log -oneline` checkout `94b0f28`)

Checkout: `~/.cargo/git/checkouts/font-kit-2de8721bb3347bf4/94b0f28/`

```
94b0f28 Upgrade dirs to 6.0              <- HEAD (наш пин; НЕ опубликован на crates.io)
1105231 Prep for crates.io launch        <- ~ база published 0.14.1-zed
5474cfa Bump core foundation
40391b7 Merge remote-tracking branch 'servo/main'
01791a8 Fix warnings
d639dfb Merge remote-tracking branch 'servo/master'
────────────────── грань форка ──────────
1f992b0 … # servo/font-kit history
38cd8a2 … # (dirs-next→dirs, font-config 6.0, freetype-sys, …)
…
```

**Table zed-only commits поверх servo:**

| commit | что делает | на crates.io? |
|---|---|---|
| `94b0f28` Upgrade dirs to 6.0 | dirs ^5.0 → 6.0 | ❌ нет |
| `1105231` Prep for crates.io launch | манифест/readme для публикации | ✅ да (якобы 0.14.1-zed) |
| `5474cfa` Bump core foundation | core-foundation 0.9→0.10 (macOS) | ✅ да |
| `40391b7` Merge remote-tracking branch 'servo/main' | sync с servo main | ✅ да |
| `01791a8` Fix warnings | чистка | ✅ да |
| `d639dfb` Merge 'servo/master' | sync | ✅ да |

Итог: форк = servo/font-kit + **5 zed-коммитов**; дрейф небольшой и стабильный.

## 3. crates.io pin viable? (сравнение с rev 94b0f28)

`cargo info zed-font-kit` + crates.io API:
- На crates.io `zed-font-kit` — **единственная** версия **`0.14.1-zed`**, опубликована **2025-10-05** (автор `mikayla-maki`, zed), **не yanked**, `rust-version 1.77`, `edition 2018`, `MIT OR Apache-2.0`.
- Feature-set published (default=`["source"]`, loader-freetype*, source-fontconfig*, source) **совпадает** с checkout rev `94b0f28`.
- **Расхождение:** published `dirs = "^5.0"`, а rev `94b0f28` — `dirs = "6.0"` (это коммит `94b0f28 Upgrade dirs to 6.0`, который на crates.io **не** опубликован). Значит published `0.14.1-zed` ≡ коммит `1105231` (crates.io launch) — **на 1 коммит раньше пина `94b0f28`**.

**API-вывод:** API у published `0.14.1-zed` и у git rev `94b0f28` **идентичен** (переход `dirs 5→6` — только transitive-зависимость, не поменял ни строки в `src/`). «Same rev» формально — нет (1 коммит), «same API» — да.

## 4. Рекомендация: **`crates.io`** (`zed-font-kit = "0.14.1-zed"`)

Перейти с git-пина на published: заменить объявление в `gpui/Cargo.toml:123` и `gpui_wgpu/Cargo.toml:38` на `zed-font-kit = "0.14.1-zed"` (добавить в `[workspace.dependencies]`).

Почему:
1. zed сам публикует форк на crates.io (`mikayla-maki`, 179k+ downloads) — это де-факто «артефакт-исходник», не требует вендора.
2. API совпадает с ревом `94b0f28`; разница — только `dirs ^5.0` вместо `6.0` (не-API).
3. `path-vendor` избыточен: нет ни уникальных правок `src/`, ни svn-ости ветвления, которые требовали бы переноса в `Source/`.
4. `defer` неприменим — feature в default.

**Caveat для S3:** после перехода на crates.io у `zed-font-kit` будет `dirs ^5.0` (не `6.0`) — проверить, что системный «font-dir» discovery всё ещё работает. Если `dirs 6.0` окажется критичен → fallback `path-vendor` рева `94b0f28` в `Source/` (тогда вендорить).

## 5. Grim plan для S3 (smoke-сценарий)

1. **App/Smoke:** собрать и запустить минимальный gpui-пример **`gpui/examples/text.rs`** (или `text_wrapper`) c `--features font-kit` (defaults) на Linux — он гоняет `cosmic-text` + `zed-font-kit` path (`find_best_match` выбор шрифта).
2. **На кадре:** визуально/шотом убедиться, что заголовок и тело рендерятся **реальными системными глифами** (fontconfig source), а не tofu/заглушкой; проверить кириллицу и несколько ligatures — отсутствие missing-glyph «■»; окно текста строится без font-load panic.
3. **Критерий pass:** `cargo build --features font-kit` зелёный **и** на скриншоте кадра присутствуют корректные глифы системного шрифта (не fallback-box). Это подтвердит, что crates.io `0.14.1-zed` (dirs 5.0) полностью заменяет пин `94b0f28`.

## Definition of done

- ✅ evidence: `cargo tree -i zed-font-kit` (production graph) + crates.io API (`0.14.1-zed`, deps `dirs=^5.0`)
- ✅ таблица zed-only commits поверх servo (p.2)
- ✅ crates.io pin viable: Да (API), formal rev — нет (dirs 6.0 не опубликован)
- ✅ рекомендация `crates.io` + fallback path-vendor + grim plan (p.4–5)
- ✅ zero git changes от агента I2 (`Source/` — чтение только; существующие локальные правки не тронуты).