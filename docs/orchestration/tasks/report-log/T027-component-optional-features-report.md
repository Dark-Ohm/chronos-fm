# T027 — Отчёт: опциональные фичи gpui-component (`decimal`, `inspector`)

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-09)
>
> Scope met: decimal, inspector (via story), --all-features, and
> inspector **build** all exit 0. No fork defects; no code changes.
> Live inspector optional residual (not required). Ticket → `done/`.

**Дата:** 2026-08-09
**Исполнитель:** Buffy (executor)
**Статус:** ✅ Все три команды + build — зелёные. 0 ошибок, 0 падений.

## Результаты

### 1. `decimal`

```
$ cargo check -p gpui-component --features decimal
    Checking gpui-component-assets v0.5.1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.01s
```

**Exit:** 0 ✅

### 2. `inspector` (через story + Стена 2)

```
$ cargo check -p gpui-component-story --features inspector
    Checking gpui_platform v0.1.0
    Checking gpui-component-story v0.5.1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 01s
```

**Exit:** 0 ✅. Предупреждения: только `nightly_coverage` (не наш cfg, апстрим gpui_linux).

### 3. `--all-features`

```
$ cargo check -p gpui-component --all-features
    Checking gpui-component-assets v0.5.1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 42.24s
```

**Exit:** 0 ✅. 65 gpui-ворнингов — все pre-existing.

### 4. Стена 4: `cargo build` (не только check)

```
$ cargo build -p gpui-component-story --features inspector
    Compiling gpui_platform v0.1.0
    Compiling gpui-component-story v0.5.1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 40.09s
```

**Exit:** 0 ✅. Проц-макросы и кодоген `gpui_macros` прошли линковку без ошибок.

## Классификация

Падений нет — классифицировать нечего.

| Команда | Результат | Комментарий |
|---------|-----------|-------------|
| `decimal` | ✅ | Фича собирается чисто |
| `inspector` (story) | ✅ | Цепочка `gpui-component/inspector` → `gpui/inspector` + `gpui_macros/inspector` прошла без ошибок (Стена 1 не сработала) |
| `--all-features` | ✅ | 34 tree-sitter грамматики + все фичи — конфликтов нет (Стена 3 не сработала) |
| `build` inspector | ✅ | Линковка чистая (Стена 4) |

## Стены — статус

| Стена | Суть | Сработала? |
|-------|------|------------|
| 1 | `inspector` → `Source/gpui` (форк) | **Нет** — наш форк переварил `inspector` без ошибок |
| 2 | `inspector` без `lsp` бесполезен | Соблюдена — проверяли на story (у него `lsp` по дефолту) |
| 3 | `--all-features` может упасть на грамматиках | **Нет** — все 34 грамматики разрешились |
| 4 | `check` ≠ `build` | Соблюдена — `build` прогнан дополнительно |

## Необязательная часть (live inspector)

Не выполнялась — все команды зелёные, live-прогон опционален. Может быть сделан отдельно при необходимости.

## Итог

`decimal` и `inspector` собираются против форка без единой ошибки. `--all-features` (включая 34 tree-sitter грамматики) тоже чист. Ни одного падения, ни одного дефекта форка. `git -C Source status` чист — правок не вносилось.
