# T012 — Отчёт: Extensions landing page (Pre-P4)

**Тикет:** T012 · **Приоритет:** P4 (blocked by plugin-host) · **Статус:** готов
**Спец:** `docs/superpowers/specs/2026-08-06-extensions-tab-live.md`
**План:** `docs/superpowers/plans/2026-08-06-extensions-tab-live.md`

## 0. Контекст

Тикет T012 — четвёртый dispatch-ready из серии «live-вкладки». В отличие
от T009/T010/T011, плагинная инфраструктура физически не начата:
`chronos-fm-plugin-host` крейт закомментирован в `[workspace] members`,
WASM-рантайма нет. Брейншторм подтвердил: полноценный Extensions-таб
невозможен до P4.

**Решение (пользователь):** информационный лендинг «Coming in P4» —
три секции: архитектура плагинов, сконфигурированные плагины из
`config.toml` (живой список, hot reload), roadmap. Без marketplace,
без установки, без WASM.

## 1. Что сделано

### ExtensionsPage — лендинг (1 коммит, 2 файла, 194 строки)

**Файлы:** `crates/chronos-fm-pages/src/extensions.rs` (перезапись),
`crates/chronos-fm-pages/src/root.rs` (2 строки)

**Три секции:**

1. **Plugin Architecture** (`elevated_card` + `section_header`):
   - WASM Component Model + wasmtime-wasi — sandboxed execution
   - Core plugins (Rust native, bundled) vs Community (WASM, permissions)
   - Ссылки на документацию (текстовые, не кликабельные — нет doc viewer)

2. **Your Plugins (config.toml)** (`elevated_card` + `section_header`):
   - Читает `config.plugins.core` и `config.plugins.community`
   - Каждый плагин: `🔌 <id>   [P4]` (accent-цвет + muted-тег)
   - **Пустое состояние:** «No plugins configured» + пример TOML-сниппета
     в monospace-блоке (`font_family("monospace")`, `bg(theme::bg_secondary)`)
   - **Hot reload:** `RootView::apply_config` вызывает `page.set_config(config.clone())`
     — список обновляется при изменении `config.toml` без перезапуска

3. **Roadmap** (`elevated_card` + `section_header`):
   - P4: Plugin host (wasmtime-wasi) — install, permissions, activation
   - P5: Plugin marketplace — browse, one-click install, Go templates

**Технические детали:**
- `ExtensionsPage` хранит `config: Config`, получает через конструктор
  `new(config, window, cx)` (паттерн S3Page)
- `set_config(&mut self, config: Config)` для hot reload
- `Focusable` через `cx.focus_handle()`
- Все UI-компоненты переиспользованы: `elevated_card`, `section_header`,
  `theme::fg/accent/border/fg_secondary/bg_secondary`

### RootView wiring

- Конструктор: `ExtensionsPage::new(config.clone(), window, cx)` вместо `ExtensionsPage::new()`
- Hot reload: `self.extensions.update(cx, |page, _cx| page.set_config(config.clone()))`
  в `apply_config` (рядом с S3Page и SettingsPage)

## 2. Верификация

| Команда | Результат |
|---|---|
| `cargo check -p chronos-fm-pages` | чисто |
| `cargo test -p chronos-fm-pages` | 70/70 passed |

## 3. Что НЕ в этом заходе (P4+)

- Plugin marketplace / discovery
- Plugin install / uninstall
- Permission consent UI
- Plugin activation / deactivation
- WASM runtime interaction
- Кликабельные ссылки на документацию (нет system browser API)

## 4. Файлы

| Файл | Изменение |
|---|---|
| `crates/chronos-fm-pages/src/extensions.rs` | Перезапись: 3 секции, ~150 строк |
| `crates/chronos-fm-pages/src/root.rs` | Конструктор + hot reload: 2 строки |

## 5. Коммиты

```
cceb1df docs: T012 Extensions tab — design spec + implementation plan
570218e ui: ExtensionsPage landing page (T012) — architecture, config plugins, roadmap
```
