# I1 — inventory: scap (zed-scap)

**Задача:** подтвердить, включён ли `scap` / feature `screen-capture` хоть в одном потребителе.
**Тип:** read-only разведка. **Блокирует:** S2.
**Дата:** 2026-08-09 · git rev (Source) `a087521f`

---

## 1. Гейт по умолчанию (Source)

`Source/gpui/Cargo.toml` (`gpui` v0.2.2) — `[features]`:

```toml
default = ["font-kit", "wayland", "x11", "windows-manifest"]   # line 20
screen-capture = ["scap"]                                      # lines 36-38
```

`scap` объявлен **optional** и активен только через `screen-capture`:

```toml
[target.'cfg(any(target_os = "linux", target_os = "freebsd", target_os = "windows"))'.dependencies]
scap = { workspace = true, optional = true }                   # lines 137-138
```

`Source/gpui_linux/Cargo.toml`: `default = ["wayland","x11"]`; `screen-capture = ["gpui/screen-capture", "scap"]`.
Workspace pin (`Source/Cargo.toml:150`): `scap = { git = "https://github.com/zed-industries/scap", rev = "4afea48…", default-features = false, package = "zed-scap" }`.

**Вывод стены подтверждён:** default **не** включает `screen-capture`. Захват экрана — за отдельным обратным feature, включённым по умолчанию у **ни одного** потребителя (см. §3).

## 2. zed-scap в графе сборки — доказательство `cargo tree`

Из `Source/` (offline, без модификаций):

```
$ cargo tree --offline -i zed-scap -e normal,build,dev
[stderr] warning: nothing to print.     # host target

$ cargo tree --offline -i zed-scap --target all -e normal,build,dev
[stderr] warning: nothing to print.     # все target
```

zed-scap **не входит** в разрешённый граф сборки — ни на одном целевом таргете.
Feature `screen-capture` — это и есть единственный «включатель»:

```
$ cargo tree --offline -p gpui --features screen-capture -i zed-scap -e normal,build,dev
zed-scap v0.0.8-zed (…/scap?rev=4afea48…)
└── gpui v0.2.2 (…/Source/gpui)
    ├── gpui_linux v0.1.0 …
    │   └── gpui_platform v0.1.0 …
```

При этом `Source/Cargo.lock` **содержит** `zed-scap` (записи `[[package]] name = "zed-scap"`, `source = "git+…scap?rev=4afea48…"`, и в `[[package]] name="gpui"` dependency-списках `zed-scap`). Это штатное поведение lock-файла (resolver фиксирует потенциальную опциональную зависимость), **но** реальная сборка её не тянет — `cargo tree -i` это доказывает.

## 3. Таблица: consumer → scap in graph?

Источник evidence: `rg -n -i 'scap|screen-capture' <consumer> --glob '**/Cargo.toml'` + объявление `gpui`.

| Consumer | scap в графе? | Evidence |
|---|---|---|
| `ChronOS/Cargo.toml` | **Нет** | `rg` → 0 совпадений (exit 1). `gpui = { path = "../Source/gpui" }` — без `features`, берёт default (без `screen-capture`). |
| `Chronos-FM/Cargo.toml` | **Нет** | `rg` → 0 совпадений. `gpui = { path = "../Source/gpui" }` (default). |
| `Chronos-lm/Cargo.toml` | **Нет** | Зависимость от `gpui` **отсутствует вовсе** (workspace = protocol/sessions/helper/lmd/greeter/session, только `tokio`/`serde`). `rg` → 0 совпадений. |
| `Chronos-IDE` (workspace + members) | **Нет** | `rg -i 'scap\|screen-capture' … --glob '**/Cargo.toml'` по всему дереву → exit 1. `gpui = { git = "…/Chronos-GPUI", rev = "ee80b72" }` — без `features`, default. |

**Итого: `scap`/`screen-capture` не включён ни у одного потребителя.**

> Note: ChronOS и Chronos-FM нацелены на локальный `Source/gpui` (default). Chronos-IDE смотрит на git-снапшот `ee80b72` Chronos-GPUI — но там тоже default (без `screen-capture`), иначе бы фигурировал в rg.

## 4. Краткий diff zed-scap vs upstream

Checkout: `~/.cargo/git/checkouts/scap-40ad33e1dd47aaea/4afea48/`
Пакет: `zed-scap v0.0.8-zed` — форк с crates.io-происхождением `repository = "https://github.com/helmerapp/scap"` (lib: wezterm-derived scap; `license = MIT`), отформачен zed-industries.

`git log --oneline -12`:

```
4afea48 Get macos version compiling      <- HEAD (наш пин)
35cd0b3 cargo.lock
c2a5800 fix-typo
a6e9afa prep for crates.io
808aa5c Revert "windows: Bump windows-capture dep version"
7a357a0 windows: Bump windows-capture dep version
e01534d x11: Fix potential underflow from TOCTOU.
e7c4361 fixup! fixup! windows: Use human-friendly title as a window title
9d05381 fixup! windows: Use human-friendly title as a window title
3c545fe windows: Use human-friendly title as a window title
18024ed Add accessor for target to capturer traits
270538d Merge pull request #1 from scoudreau/wayland-only
```

Слой zed-форка = верхние 4–11 коммитов (macos-компиляция, cargo.lock, fix-typo, crates.io prep, windows/img-фиксы) поверх upstream `helmerapp/scap`. Дрейф небольшой, но это **живой форк** с собственными коммитами zed — «чистого» upstream-кола тянет за собой, а вендор означал бы перенос именно zed-форка.

## 5. Решение: **DEFER** (не вендорить)

**Почему:**
1. У потребителей (ChronOS, Chronos-FM, Chronos-lm, Chronos-IDE) `screen-capture` не включён — `cargo tree -i zed-scap` печатает «nothing to print» даже при `--target all`.
2. `scap` в `Source` — **optional**-зависимость под обратным feature (`screen-capture`), default его не содержит.
3. Сейчас это непроизведённый «мёртвый» узел: он в `Cargo.lock`, но в фактический граф сборки не попадает — поэтому не имеет влияния на воспроизводимость/лицензии.
4. Вендор zed-форка (живой, `4afea48`) сейчас = лишняя стоимость поддержки без единого потребителя.

**Condition для VENDOR:** как только любой потребитель включит `screen-capture` (например, ChronOS записи экрана) — тогда вендорить `zed-scap@4afea48` (или обновлённый zed-пин) в `Source/`. До этого — DEFER.

## Definition of done

- ✅ evidence `cargo tree` (цитаты в §2)
- ✅ таблица consumer → scap in graph (0/4 да) в §3
- ✅ краткий diff zed-scap vs upstream в §4
- ✅ решение DEFER + условие перехода в §5
- ✅ zero git changes от агента I1: `Source/` — чтение только (существующие перед T032 локальные правки `Cargo.toml`/`Cargo.lock`/`NOTICE`/`gpui-component`/`reqwest_client`/`http_client_tls` — НЕ мои, не тронуты).