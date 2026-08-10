# T037 — Explorer visual spec (design authority)

**Роль:** дизайнер. Этот документ — нормативная выжимка из мокапа
**`docs/design/mockups/chronos-file-manager.dc.html`** (768 строк, 1280×800,
`theme` default = `dark`), сверенная с фактическим кодом. Исполнителю T037
следует реализовывать **эти числа и токены**, а не читать HTML заново.

> **Ревизии мокапа.** Актуальная (v2, полировка архитектора 2026-08-09 19:55)
> **Ревизия 2026-08-10 (v3):** канон обновлён — `mockups/chronos-file-manager.dc.html`
> = `mockups/chronos-file-manager.dc.html` (identical). Previous v2 archived as
> `archive/chronos-file-manager.v2-2026-08-09.dc.html`. Density trend continues
> (e.g. list rows **32px** in v3 HTML). **If this spec conflicts with current
> HTML, HTML wins** until designer re-extracts § numbers. Executor T037: prefer
> live mockup HTML + §7 intent over stale v2 line numbers.

> всегда лежит по каноническому имени `mockups/chronos-file-manager.dc.html`;
> предыдущая — `mockups/archive/chronos-file-manager.v1.dc.html` (19:34).
> Правило: новая ревизия встаёт на каноническое имя, старая уезжает в
> `archive/` под номером — чтобы «очевидное» имя никогда не вело на устаревший
> файл. Отличий v1→v2 — 11, все — density-пасс
> на контенте explorer'а: строка списка и ячейка грида «дышат» шире, иконки
> на шаг крупнее, у активного пункта nav появилась тень. Токены, каркас и
> геометрия shell (36/64/48/32/28) **не менялись** — весь §0, §1 и §2.1–2.6
> действительны для обеих ревизий. Числа ниже даны по **v2**; там, где v1
> отличался, старое значение указано как `v1: N`.
>
> **Тренд между ревизиями важнее самих чисел:** заказчик двигает контент в
> сторону более свободной плотности при неизменном каркасе. Если придёт v3
> — ждать правок там же (row/grid/preview), а не в shell.

**Truth base каждой строки ниже:** mockup HTML (номера строк) + файлы
`crates/*` + `chronos.theme.json`. Ни одного утверждения из памяти/Zed.

## Область действия мокапа (уточнение архитектора, 2026-08-09)

Мокап — эталон **основного окна файлового менеджера**, не всего приложения.
На каждую страницу (Git / S3 / Extensions / Settings) будет **свой отдельный
мокап**. Отсюда два разных статуса у регионов одного и того же HTML:

| Часть мокапа | Статус | Что с ней делать в T037 |
|---|---|---|
| **Общий каркас**: title bar (§2.2), nav rail (§2.3), footer (§2.10), token map (§1) | **нормативно для всего приложения** — это shell, он один на все страницы | реализовать |
| **Explorer**: tab strip, address bar, Places, list/grid, preview (§2.4–2.9) | **нормативно** — это и есть предмет мокапа | реализовать |
| **Git / S3 / Extensions / Settings** (mockup:210-429) | **НЕ нормативно** — иллюстративная заглушка, чтобы каркас было видно в контексте | **не трогать**, ждать персональный мокап страницы |

Практическое следствие: из этой спеки на вторичные страницы переносится
**только §1 (токены) и общий каркас** — они обязаны совпадать, иначе
страницы разъедутся по палитре. Layout вторичных страниц из мокапа
**не копировать даже как «пока так»** — это создаст визуальный долг,
который придётся сносить, когда придёт их собственный мокап.

---

## 0. Главный вывод дизайнера

Плотность shell **уже почти совпадает** с мокапом — 36/64/48/32/24/28
константы в коде фактически есть. «Не читается как продукт» — это **не**
проблема layout. Это три вещи:

| # | Причина | Evidence | Truth base |
|---|---------|----------|------------|
| 1 | `mode = "light"` + `accent = "purple"` в live-конфиге → лавандовая заливка вместо Catppuccin-dark | `~/.config/chronos-fm/config.toml` `[theme] mode="light" accent="purple"`; аккент переоркашивает `accent/primary/selection/caret` в `root.rs:146-177` | config + Chronos-FM |
| 2 | **Плоскость поверхностей**: мокап строит иерархию тремя фонами (`bg` / `bgSecondary` / `bgTertiary`), код местами берёт неверный токен (например footer `theme::gray_200` = `border`, а в мокапе footer = `gray200` = `#25253b` = `secondary`) | mockup:434 `background:{{ c.gray200 }}` + mockup:616 `gray200:'#25253b'`; `footer.rs:57` `.bg(theme::gray_200(cx))`; `theme.rs` `gray_200 → cx.theme().border` (#313244) | mockup + Chronos-FM |
| 3 | **Иконки**: в мокапе 8 типов файлов + 5 nav + 7 places; в коде — `Folder`/`File` и всё | `row.rs:23-24`, `grid.rs:58-59`, `sidebar.rs:185-186`; mockup `ICONS` (mockup:451-465) | mockup + Chronos-FM |

Палитра темы при этом **уже точна**: `chronos.theme.json` «Chronos Dark»
= мокап dark один в один (`background #1e1e2e`, `foreground #cdd6f4`,
`border #313244`, `primary #007acc`, `secondary #25253b`, `sidebar #181825`,
`muted_foreground #6c7086`). Перекрашивать тему не нужно.

---

## 1. Token map (мокап → theme.json → `theme::*`)

Мокап оперирует 20 токенами. Отображение на наш bridge:

| mockup token | dark hex | theme.json key | accessor `chronos_fm_ui::theme::theme::` | статус |
|---|---|---|---|---|
| `bg` | `#1e1e2e` | `background` | `bg` | ✅ |
| `bgSecondary` | `#25253b` | `secondary` | `bg_secondary` | ✅ |
| `bgTertiary` | `#181825` | `sidebar` / `muted` | `toolbar_bg` (= `sidebar`) | ⚠️ имя врёт — см. §1.1 |
| `bgHover` | `#313244` | `list_hover` `#28283D` | `bg_hover` | ⚠️ дельта 2 шага яркости, допустимо |
| `border` | `#313244` | `border` | `border` | ✅ |
| `fg` | `#cdd6f4` | `foreground` | `fg` | ✅ |
| `fgSecondary` | `#a6adc8` | `secondary_foreground` | `fg_secondary` | ✅ |
| `muted` | `#6c7086` | `muted_foreground` | `muted` | ✅ |
| `disabled` | `#45475a` | — | **отсутствует** | ❌ добавить `theme::disabled` |
| `accent` | `#007acc` | `primary` | `accent` | ✅ |
| `onAccent` | `#ffffff` | `primary_foreground` | — | ❌ добавить `theme::on_accent` |
| `toolbarBg` | `#181825` | `sidebar` | `toolbar_bg` | ✅ |
| `toolbarBorder` | `#313244` | `sidebar_border` | `toolbar_border` | ✅ |
| `toolbarHoverBg` | `rgba(0,122,204,.18)` | `sidebar_accent` `#007acc2E` | `toolbar_hover` | ✅ (2E≈.18) |
| `toolbarActiveBg` | `rgba(0,122,204,.18)` | `list_active` `#007acc2E` | `toolbar_active_bg` | ✅ |
| `toolbarActiveText` | `#cdd6f4` | `sidebar_primary_foreground` `#FFFFFF` | `toolbar_active_text` | ❌ мокап = `fg`, не белый |
| `gray200` (footer bg) | `#25253b` | `secondary` | `gray_200` → **`border`** | ❌ баг маппинга |
| `gray700` (footer text) | `#cdd6f4` | `foreground` | `gray_700` | ✅ |
| `green` | `#a6e3a1` | `green`/`success` | — | ✅ через `cx.theme()` |
| `danger` | `#f38ba8` | `danger` | `danger` | ✅ |

### 1.1 Правки, которые дизайнер требует в `theme.rs`

Три точечных изменения, без рефакторинга:

1. `gray_200` → `cx.theme().secondary` (сейчас `border`). Мокап использует
   `gray200` как **поверхность** (footer), а не как линию. Все линии в
   мокапе идут через `c.border`, и код для них уже зовёт `theme::border`.
   *Риск:* `gray_200` может использоваться как рамка где-то ещё —
   исполнителю **сверить `rg 'gray_200'`** перед правкой; если есть
   consumer-рамка, не менять `gray_200`, а в `footer.rs:57` поставить
   `theme::bg_secondary(cx)` (минимально-инвазивный вариант, предпочтителен).
2. Добавить `pub fn disabled(cx) -> Hsla` → ближайший `#45475a`. В нашей
   dark-палитре это `secondary_active`/`muted` не подходят; корректный
   источник — новый ключ в `chronos.theme.json` **не заводить**, взять
   `cx.theme().muted_foreground` с `opacity 0.7` на call-site, либо
   добавить ключ, если генератор темы (`script/dev/gen_chronos_theme.py`)
   это поддерживает. Решение дизайнера: **call-site opacity**, тему не трогаем.
3. `toolbar_active_text` → `cx.theme().sidebar_accent_foreground`
   (`#cdd6f4`), а не `sidebar_primary_foreground` (`#FFFFFF`). В мокапе
   активная nav-иконка **не белая** — она `c.toolbarActiveText = fg`, а
   выделение несёт фон-заливка (mockup:615, mockup:640).

Ничего из этого не требует патча в `Source`.

---

## 2. Геометрия по регионам (нормативно)

Все значения — из мокапа, номер строки в скобках. Колонка «в коде» —
факт на 2026-08-09.

### 2.1 Оболочка окна (mockup:23)
- 1280×800, `border-radius 10`, `border 1px c.border`, `overflow hidden`,
  фон `c.bg`, `flex-column`.

### 2.2 Title bar — h **36** (mockup:25-30)
| Свойство | Значение | В коде |
|---|---|---|
| height | 36 | `UNIFIED_TOOLBAR_HEIGHT = px(36.0)` ✅ |
| bg / border-bottom | `c.bg` / 1px `c.border` | сверить |
| padding | `0 14` | сверить |
| слева | `chronos-fm`, JetBrains Mono **12**, `c.fgSecondary` | — |
| справа | 26×26, radius 8, иконка 16 (`circle-user`), hover `c.toolbarHover` | account button есть (`unified_toolbar.rs`) |

**Дизайн-решение:** mono-шрифт заголовка обязателен — это опознавательный
знак продукта в мокапе. Если mono-фэмили не зарегистрирована, ставить
единственную задачу «зарегистрировать JetBrains Mono», не заменять на sans.

### 2.3 Nav rail — w **64** (mockup:34-40)
| Свойство | Значение | В коде |
|---|---|---|
| width / bg / border-right | 64 / `c.toolbarBg` / `c.toolbarBorder` | `root.rs:434` w64 ✅ |
| padding / gap | `16 0` / 8 | сверить |
| item | 48×48, radius 8 | `root.rs:461-462` ✅ |
| icon | **19px** | сверить |
| active | bg `toolbarActiveBg`, icon color `toolbarActiveText` (= `fg`), **+ `box-shadow 0 1px 2px rgba(0,0,0,.25)`** (v2, mockup:36,640) | см. §1.1 п.3 |
| hover | bg `toolbarHoverBg` | сверить |

**Дизайн-решение по тени (v2).** Тень у активного пункта — единственное
место в мокапе, где выделение несёт не только цвет. Если в gpui нет
дешёвого `box_shadow` на этом элементе — **не** имитировать рамкой:
пропустить и записать residual. Приоритет тени низкий, заливка несёт 90%
читаемости состояния.

Порядок пунктов (mockup:628-634): Explorer · Git · S3 · Extensions · Settings.

### 2.4 Tab strip — h **32** (mockup:51-85)
| Элемент | Значение | В коде |
|---|---|---|
| полоса | h32, bg `c.toolbarBg`, border-bottom `c.border`, padding `0 6`, gap 4 | `pane_group.rs:574` h32 ✅ |
| таб | h24, padding `0 10`, radius 6, font 12 | `pane_group.rs:658` h24 ✅ |
| close в табе | 14×14, radius 4, глиф 9; **только если табов > 1** | mockup:55-59 |
| `+` | 20×20, radius 5, глиф 11, цвет `c.muted` | `pane_group.rs:601` |
| spacer | `flex:1` | |
| Split chip | padding `3 7`, radius 5, icon 12 + подпись **11px** «Split»; только у панели idx 0 | mockup:66-71, 694 |
| list/grid | 2×(20×20, radius 5), gap **1**, глиф 11, активный — bg `c.bgHover` | mockup:72-79 |
| close pane | 22×22, radius 5, глиф 11; только в split | mockup:80-84 |

**Дизайн-решение (порядок справа налево):** Split → list/grid → close-pane.
Не менять местами: list/grid всегда прижат к правому краю панели, кроме
close-pane.

### 2.5 Address bar (mockup:87-98) — **главная визуальная дельта**
| Свойство | Значение | В коде (`explorer/view/header.rs`) |
|---|---|---|
| контейнер | padding `6 14`, gap 8, bg **`c.bgTertiary`**, border-bottom `c.border` | `px(24) py(12)`, bg `theme::bg` ❌ |
| высота итог | ≈34 | ≈44 ❌ |
| back / forward | 22×22, radius 6, **SVG chevron 13px**; disabled → цвет `c.disabled`, `cursor:not-allowed` | `ListItem` с текстовыми «←»/«→», `opacity(0.3)` ❌ |
| folder-иконка | 12px, `c.muted`, `flex:none` | отсутствует ❌ |
| путь | `flex:1`, **JetBrains Mono 11.5**, `c.fgSecondary`, bg `c.bg`, border 1px `c.border`, radius 5, padding `3 8`, ellipsis | `Breadcrumb` без рамки/фона ❌ |

**Дизайн-решение по конфликту breadcrumb vs path field.** Мокап рендерит
mono-строку пути (mockup:97), но в JS-модели мокапа breadcrumb-объект тоже
посчитан и не использован (mockup:670-676) — то есть автор мокапа оставил
оба. Продуктовое поведение (кликабельные сегменты) — наше преимущество,
терять его нельзя.

> **Норма:** сохранить `Breadcrumb`, но поместить его **внутрь chrome
> адресного поля** — bg `c.bg`, border 1px `c.border`, radius 5,
> padding `3 8`, mono 11.5, сегменты `c.fgSecondary`, последний сегмент
> `c.fg` weight 500 (mockup:673-674), ellipsis слева при переполнении.

Тогда side-by-side читается как мокап, а функциональность растёт, а не падает.

### 2.6 Places sidebar — w **212** (mockup:102-150)
| Элемент | Значение | В коде |
|---|---|---|
| панель | w **212**, bg `c.bgTertiary`, border-right, padding `14 10`, gap 16, скролл | `view.rs:116-117` size 180, range 180..360 ❌ |
| section header | 3×12 бар radius 1.5 `c.accent` opacity .85 + текст 12.5/600 `c.fg`; ниже мono **10** `c.muted` «quick access» | `patterns.rs:67-68` бар 3×12 ✅ |
| row | padding 6, radius 6, gap 9, icon **14**, label 12.5 | сверить |
| row active | bg `c.bgHover`, текст `c.fg` weight 500, icon `c.accent`, **+ 2px accent-бар слева**, inset top/bottom 6, radius 2 | mockup:116-118 |
| row idle | текст `c.fgSecondary` weight 400, icon `c.muted` | mockup:663-665 |
| divider | 1px `c.border`, margin `0 6` | mockup:125 |
| device | icon 15 `c.muted` + label 12.5 `c.fgSecondary`; полоса 3px radius 2 bg `c.border`, заливка `c.accent` на `pct%`, `margin-left 24` | `sidebar.rs:110` HardDrive есть; полосы заполнения нет ❌ |
| Trash | row + счётчик mono **10** `c.disabled`, `margin-left:auto` | сверить |

**Дизайн-решение:** дефолт ширины panel → **212** (range 180..360 оставить).
Полоса заполнения диска — обязательный элемент мокапа, но она требует
данных `devices_store`; если объёма нет в модели — рисовать **не** фейковую
полосу, а опустить её (T037 запрещает fake data). Зафиксировать как residual.

### 2.7 File list (mockup:152-170)
| Элемент | Значение | В коде |
|---|---|---|
| header row | padding `6 16`, border-bottom, font **10**/600, uppercase, letter-spacing .03em, `c.muted` | `list.rs:119` h **48** ❌ (мокап ≈25) |
| колонки | Name `flex:1` · Size **w60** right · Modified **w80** right | + Type ⚠️ |
| body padding | `4 8` | сверить |
| row | padding **`6 10`** (v1: `5 8`), radius 6, gap **10** (v1: 9) → **h≈28** (v1: ≈26) | `row.rs:145` h **32** ❌ |
| icon | **16px** (v1: 15); выделен → `c.accent`, иначе `c.fgSecondary` | Folder/File only ❌ |
| name | **12.5px** (v1: 12); выделен → `c.fg`, иначе `c.fgSecondary` | сверить |
| size / modified | mono **10.5**, `c.muted`, right | сверить |
| selected | bg `rgba(0,122,204,.18)` (= `list_active`) | сверить |
| hover | `c.bgHover` | сверить |

**Дизайн-решение по колонке Type.** Мокап её не имеет; в коде она
resizable и уже живёт (`row.rs:255`). Удалять функциональность ради
картинки — нельзя. Норма: Type **остаётся**, но выравнивается по
мокап-типографике (mono 10.5, `c.muted`, right) и ставится **между Name и
Size**, ширина по умолчанию 90. Порядок: Name (flex) · Type 90 · Size 60 ·
Modified 80. Разница с мокапом на одну колонку — принимается дизайнером,
это не блокер ACCEPT.

### 2.8 Grid view (mockup:171-180)
- контейнер: padding 16, `grid-template-columns: repeat(auto-fill, minmax(**88**, 1fr))` (v1: 84), gap **8** (v1: 6), `align-content:start`.
- ячейка: padding **`12 6`** (v1: `10 4`), radius 8, gap **7** (v1: 6), иконка **32** (v1: 30), подпись 10.5, line-height 1.25, **клэмп 2 строки**, центр.
- В коде `grid.rs:91` ширина ячейки **180** — вдвое крупнее мокапа ❌.

### 2.9 Preview — w **220** (mockup:182-200)
| Элемент | Значение | В коде |
|---|---|---|
| панель | w **220**, border-left `c.border`, bg `c.bg` | `view.rs:143-144` size 240, range 240..2000 ❌ |
| заголовок | padding **`10 16`** (v1: `10 14`), border-bottom, 12.5/600 `c.fg` | сверить |
| тело | центр, padding **20** (v1: 16), gap **14** (v1: 12) | сверить |
| пусто | «No file selected», 12px, `c.muted` | T023 ✅ — перекрасить в `c.muted`/12 |
| выбран | иконка **44** (v1: 42), имя 12/500 центр `c.fg` (`word-break`), мета mono 10.5 `c.muted` | сверить |

Мокап показывает preview **только когда `!split`** (mockup:696) — то же
правило для sidebar (mockup:695). Это ключевая часть композиции: в split
обе панели идут «голыми», иначе 1280px не хватает.

### 2.10 Footer — h **28** (mockup:434-445)
| Свойство | Значение | В коде |
|---|---|---|
| высота | 28 | `footer.rs:51` ✅ |
| bg | **`c.gray200` = `#25253b` = `secondary`** | `theme::gray_200` → `border` ❌ |
| border-top | 1px `c.border` | ✅ |
| padding | `0 8` | сверить |
| слева | «N items», **10.5px**, `c.gray700` (= `fg`) | `gray_700` ✅ |
| справа | путь, 10.5px, `c.gray700` | сверить |
| внутренние ячейки | h24, padding `0 8`, radius 4, gap 5 | `footer.rs:122` h24 ✅ |

---

## 3. Иконки — карта и дефицит

**Правило T037:** только `crates/chronos-fm-ui/assets/icons/*.svg`.
Path-строки из мокапа (`ICONS`, mockup:451-465) — Phosphor, **не копировать**.

### 3.1 Nav rail (5)
| Мокап | Наш файл | Статус |
|---|---|---|
| folder | `icons/folder.svg` | ✅ (`chronos_fm_pages.rs:56`) |
| git | `icons/github.svg` | ⚠️ логотип GitHub ≠ git-граф. **Нужен `git-branch.svg`** |
| cloud | `icons/database.svg` | ⚠️ семантика «БД» ≠ «облако». **Нужен `cloud.svg`** |
| puzzle | `icons/layout-dashboard.svg` | ⚠️ **Нужен `puzzle.svg`** |
| gear | `icons/settings.svg` | ✅ |

### 3.2 Places (7, mockup `PLACES_DEF`)
Есть: `home.svg`/`house.svg`, `trash-2.svg`, `hard-drive.svg`, `folder.svg`.
**Нужны:** `desktop.svg` (монитор), `download.svg` (стрелка-вниз в лоток),
`image.svg`, `file-text.svg` (Documents).

### 3.3 Типы файлов (8, mockup `iconFor`)
Сейчас в коде ровно два состояния — `Folder` / `File`
(`row.rs:23-24`, `grid.rs:58-59`). **Нужны:** `file-code.svg`,
`file-archive.svg`, `file-image.svg`, `file-text.svg`.

> **Дизайн-решение:** добавляем ровно **8 SVG** (`git-branch`, `cloud`,
> `puzzle`, `desktop`, `download`, `file-code`, `file-archive`,
> `file-image`) — рисуем в стиле существующего пака (stroke-based, 24×24
> viewBox, `currentColor`), а не конвертируем Phosphor-филлы. Пак должен
> остаться однородным: смешивать stroke-иконки с fill-глифами нельзя,
> это читается как «собрано из двух проектов» — ровно та жалоба клиента.

Приоритет реализации: nav (5) → типы файлов (4) → places (3). Если время
кончилось — маппить недостающее на ближайший существующий глиф пака,
**не** тянуть внешний набор.

---

## 4. Правила темы и конфига

1. Продуктовый дефолт — **dark**. Менять дефолт в схеме/`Config::default`,
   а **не** переписывать пользовательский `~/.config/chronos-fm/config.toml`.
2. Для приёмочного grim клиенту документировать однострочник:
   `[theme] mode = "dark"`, `accent = "blue"` (или удалить `accent`, чтобы
   взялся `#007acc` из темы). Purple-акцент перекрашивает
   `accent/primary/selection/caret` (`root.rs:146-177`) и ломает
   side-by-side с мокапом, где акцент — `#007acc` **в обоих режимах**.
3. Light-режим не удалять: мокап light-палитра (mockup:618-625) один в один
   совпадает с «Chronos Light» в `chronos.theme.json` (`#dde0f2` / `#2c2e4a`
   / `#c4c8e6` / `#eceefa`). Оба режима валидны, dark — дефолт.

---

## 5. Порядок работ (уточняет §«Suggested implementation order» в T037)

1. **Конфиг/дефолт dark + аккент `#007acc`** — самая большая дельта
   картинки на единицу работы. Grim сразу после.
2. **Поверхности:** footer bg (§1.1 п.1), address bar bg → `bgTertiary`,
   tab strip bg → `toolbar_bg`, sidebar bg → `bgTertiary`. Это даёт
   трёхуровневую глубину мокапа.
3. **Address bar** (§2.5) — chrome поля + chevron-иконки + mono. Самый
   заметный «пропавший chrome» на клиентском шоте.
4. **Плотность списка:** header 48→≈25, row 32→28, name 12.5, icon 16,
   mono 10.5 для Size/Modified/Type.
5. **Ширины панелей:** sidebar 180→212, preview 240→220, grid-ячейка 180→88.
6. **Иконки** (§3), 8 штук, партиями по приоритету.
7. **Grim vs мокап**, тёмный, 1280×800, тот же каталог.

Пункты 1–3 — accept-critical. 4–6 — parity. 7 — доказательство.

---

## 6. Что дизайнер выносит из скоупа T037

- Полоса заполнения дисков в Places без реальных данных (§2.6) — residual.
- Git/S3/Extensions/Settings-страницы (mockup:210-429) — **вне скоупа и вне
  нормы**. Мокап посвящён основному окну файлового менеджера; эти регионы в
  нём — заглушка «чтобы каркас было видно в контексте», у каждой страницы
  будет **свой мокап** (см. «Область действия мокапа» выше). Не переносить
  их layout, не считать карточную систему из mockup:213/230/269/283/416
  утверждённой. Что переносится безусловно — §1 token map и общий каркас
  (title bar / nav rail / footer): палитра и shell обязаны быть общими,
  иначе страницы разъедутся между собой.
- Геометрия окна под Hypr (T037 §Decision п.5) — не визуальная задача.

---

## 7. Чек-лист приёмки визуала (для vision-модели)

Сравнивать grim и мокап **по этим семи признакам**, каждый — бинарно:

1. Фон окна `#1e1e2e`, не лавандовый; акцент синий, не фиолетовый.
2. Видны **три** уровня поверхности: страница (`#1e1e2e`), rail/address/
   sidebar (`#181825`), footer (`#25253b`).
3. Адресная строка — обведённое поле с mono-путём и chevron-кнопками,
   а не «голый» текст.
4. Places слева шириной ≈212 с accent-баром у активного пункта.
5. Строка списка ≈28px (v2), Size/Modified — mono и прижаты вправо.
6. Иконки типов файлов различаются (не все одинаковые «файл»).
7. Footer 28px, тёмно-серый, «N items» слева и путь справа.

7/7 → ACCEPT по визуалу. Любой «нет» → claim + evidence, что мешает.
