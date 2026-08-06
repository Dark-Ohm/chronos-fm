# T009 — Отчёт: живая вкладка Settings (theme/ui/explorer)

**Тикет:** T009 · **Приоритет:** P1 · **Статус:** готов к приёмке
**Спец:** `docs/superpowers/specs/2026-08-06-settings-tab-live.md`
**План:** `docs/superpowers/plans/2026-08-06-settings-tab-live.md`
(исполнен полностью, TDD — тесты до реализации, 3 коммита по таскам)

## 0. Контекст

Тикет T009 — единственный dispatch-ready из серии «live-вкладки»
(T010 Git / T011 S3 / T012 Extensions — трекинг, без spec/plan; T012 ещё
и заблокирован P4-роадмапом). Исполнен план 2026-08-06 целиком:
заглушка `SettingsPage` → живая форма, пишущая `config.toml` через
comment-preserving патч.

## 1. Что сделано

### Task 1 — `crates/chronos-fm-core/src/config/patch.rs` (новый модуль)

- **`ConfigField`** — enum из 8 вариантов, каждый маппится 1:1 на
  `[section].key` в `config.toml` (`theme.mode`/`theme.accent`,
  `ui.default_sort`/`ui.show_hidden`/`ui.icon_pack`,
  `explorer.split_direction`/`synced_panes`/`restore_tabs`).
- **`patch_config_text(source, field) -> Result<String>`** — точечный
  патч через `toml_edit::DocumentMut`: **никогда** не делает полный
  round-trip `Config`→TOML (уничтожил бы комментарии/форматирование
  пользователя), меняет ровно один ключ. Создаёт секцию, если её нет.
- **Важная деталь реализации:** в `toml_edit 0.25` decor (пробелы +
  inline-коммент) живёт на `Value`, а не на `Item` — чтобы сохранить
  inline-комментарий строки (`mode = "dark"   # ...`), decor старого
  значения копируется через `as_value()`/`as_value_mut()` перед заменой
  (тест `patches_theme_mode_preserving_comments` ассертит, что изменена
  ровно 1 строка, коммент цел).
- **`patch_config_file(path, field) -> Result<()>`** — read-patch-write
  обёртка. `#[allow(clippy::disallowed_methods)]` с комментарием —
  конвенция `loader.rs` (core — не-UI крейт, lint целится в UI-слой).
- Зависимости: `toml_edit 0.25.13` + `anyhow` (workspace) добавлены в
  `chronos-fm-core/Cargo.toml`.
- **3 теста** (из плана, TDD): comment-preserving patch (1 строка
  изменена, коммент цел), bool-патч, создание отсутствующей секции.

### Task 2 — `default_config_path()` в том же patch.rs

- Делегирует в **существующий** `paths::config_file()` (XDG-резолв уже
  реализован в `config/paths.rs`) — никакого дублирования path-логики.
- **1 тест** (`default_config_path_ends_with_expected_filename`):
  имя файла `config.toml`, содержит `chronos-fm`.

### Task 3 — `crates/chronos-fm-pages/src/settings.rs` (переписан)

- `SettingsPage { config: Config }` — держит текущий Config, получаемый
  при конструировании. `SettingsPage::new(config)`.
- **Секции-карточки** (`elevated_card` + `section_header`, T002):
  - **Theme** — три кнопки System/Light/Dark (активная — accent-заливка);
  - **UI** — Switch «Show hidden files» + 4 кнопки сортировки
    Name/Modified/Size/Kind;
  - **Explorer** — Switch «Synced panes» + Switch «Restore tabs on restart»;
  - **Draft** — видимая, приглушённая (`opacity 0.5`) строка про
    Keybindings/Plugins/Indexing/Search/Launcher (P3/P4), без контролов —
    спец-решение «show, don't hide».
- **Запись:** любой клик → `SettingsPage::write_field(ConfigField)` →
  `default_config_path()` → `patch_config_file()` → `tracing::error` при
  неудаче. Сама страница **не** мутирует `self.config` — файл пишется, а
  существующий watcher (`root.rs::start_config_watch`) перечитывает
  `config.toml` и применяет через `apply_config` (живой hot-reload,
  включая смену темы через `Theme::change`). Это и есть «live».
- **Call site:** `root.rs:95` — `SettingsPage::new(config.clone())`
  (config доступен как параметр до перемещения в `apply_config`).
- **Render-тест:** `settings_page_renders_without_panicking` —
  `gpui_component::init` + `add_empty_window().draw(...)` с entity-рендером
  через `into_any_element()`. Две ошибки по пути к зелёному (задокумен-
  тированы, чтобы не всплыли в T010-T012): (1) `cx.new` внутри draw
  требует трейт `AppContext` в scope (`App` реализует `AppContext`, но
  трейт надо импортировать); (2) рендерить entity надо через
  `Entity::into_any_element()` (как `root.rs::render_active_page`), а не
  прямым `Render::render` — прямое вызывает panic в
  `window::current_view()` (пустой rendered-entity-stack).

## 2. Верификация

- `cargo test -p chronos-fm-core config::patch::` → **5 passed** (Task 1-2 + review-тест)
- `cargo test -p chronos-fm-core` → **54/54 passed**
- `cargo test -p chronos-fm-pages` → **70 passed, 0 failed** (69 + новый render-тест)
- `cargo build --workspace` → **EXIT=0**
- `cargo test --workspace` → **все suite ok** (223 passed суммарно, 0 failed)
- `cargo clippy -p chronos-fm-core -p chronos-fm-pages` → 0 замечаний в
  `patch.rs`/`settings.rs`/`root.rs` (оставшиеся warn — pre-existing в
  gpui-форке и других файлах)
- `toml_edit`-версия 0.25 сверена с API в кэше регистра (decor на Value,
  `as_value()`/`as_value_mut()` — а не по памяти.

## 2.1 Правки по ревью (code-reviewer)

Два реальных бага + мелочь, найдены ревью, исправлены (коммит `ab96ac1`):

1. **Stale config в SettingsPage.** Страница держала `Config`-снимок с
   момента конструирования, а `apply_config` (hot-reload путь, срабатывает
   на каждую запись файла — включая собственные клики страницы) обновлял
   theme/explorer, но не settings-entity. Симптом: клик «Dark» → приложение
   темнеет, а кнопка «System» остаётся подсвеченной. Фикс: `set_config` в
   SettingsPage + вызов из `apply_config` рядом с explorer-обновлением —
   активные состояния всегда следят за on-disk `config.toml`.
2. **Потенциальный panic в `patch_config_text`.** Guard был `is_none()`;
   при hand-edited `theme = "dark"` (строка, не таблица) `get()` возвращал
   `Some(non-table)`, guard пропускался, `doc["theme"]["mode"] = ...`
   паниковал на IndexMut. Фикс: `is_none_or(|item| !item.is_table())`;
   не-таблица удаляется (`as_table_mut().remove`) и вставляется заново —
   иначе decor старой строки протекал в заголовок (`[theme ]` вместо
   `[theme]`). +тест `patches_non_table_section_by_replacing_it`.
3. Мелочь: тест `default_config_path_ends_with_expected_filename` теперь
   берёт `env_lock()` — конвенция `paths.rs` (тесты мутируют XDG env).

После фиксов: `cargo test -p chronos-fm-core config::patch::` → 5/5,
`cargo test -p chronos-fm-pages` → 70/70, clippy → 0 в затронутых файлах.

## 3. Что НЕ в этом заходе (по спец-скоупу)

- Разрешение конфликтов config (спец — out of scope).
- P5-пикер accent-цвета — **частично сделан после отчёта** (коммит
  `f0ab77e`): 7 свотчей (blue/green/purple/orange/red/teal/pink) в
  Theme-секции; клик пишет `theme.accent` в config.toml, а
  `apply_config` ре-сидит accent-производные цвета ThemeConfig
  (accent/primary/caret/link/ring/progress_bar/selection, обе моды) и
  переприменяет через `Theme::change` — акцент меняется live через
  watcher. Палитра `ACCENT_PALETTE` + `accent_hex()` в
  `config/settings.rs` (3 unit-теста: имена, #/0x/hex-нормализация,
  fallback на синий). Свободный hex-инпут — остаётся P5. Правки по
  ревью (коммит `df4e203`): hover/active-оттенки
  (primary_hover/primary_active/link_hover/link_active/drop_target)
  очищаются в None, чтобы fallback'и gpui-component выводили их из
  нового primary/link, а не из хардкод-синего из JSON; selection
  сохраняет полупрозрачность (`hex+33`); guard `has_global::<Theme>`;
  ширина рамки свотча постоянна (border_1 в обоих состояниях — без
  сдвига ряда на 1px).
- `split_direction` — **добавлен после отчёта** (коммит `be49c0b`):
  две кнопки «Side by side»/«Stacked» в Explorer-секции (Vertical =
  side by side, Horizontal = stacked — по doc-комментарию enum);
  write-path покрыт тестом `patches_explorer_split_direction`
  (vertical/horizontal round-trip), settings-тесты 70/70, core patch 6/6.

## 4. Файлы

- Новые: `crates/chronos-fm-core/src/config/patch.rs`
- Изменены: `crates/chronos-fm-core/src/config.rs` (pub mod patch),
  `crates/chronos-fm-core/Cargo.toml` (+toml_edit, +anyhow),
  `crates/chronos-fm-pages/src/settings.rs` (переписан),
  `crates/chronos-fm-pages/src/root.rs` (call site с Config + live-sync),
  `Cargo.lock`

## 5. Коммиты

```
ebf3b6b core: config patch module (ConfigField + toml_edit write-back) (T009 Task 1)
3b80849 core: default_config_path helper for Settings tab writes (T009 Task 2)
d0a957c ui: live Settings tab (theme/ui/explorer editing, draft sections shown disabled) (T009 Task 3)
ab96ac1 ui+core: Settings page live-sync with config reload; guard non-table sections in patch (T009 review)
```

## 6. Живой прогон (приёмка архитектора)

НЕ проводился — headless-сессия. Клик «Dark» → живое применение темы,
`cat ~/.config/chronos-fm/config.toml` → ровно одна строка изменена,
toggle «Show hidden files» → показ dotfiles, активная кнопка следует за
конфигом после hot-reload — требуют живой Hyprland-сессии.
