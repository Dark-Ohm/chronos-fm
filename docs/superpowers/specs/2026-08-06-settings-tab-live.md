# Settings Tab v1 — Live Config Editing

**Дата:** 2026-08-06. **Автор:** Архитектор.

## Контекст и цель

`crates/chronos-fm-pages/src/settings.rs` — 61-строчная заглушка:
карточка с текстом «⚙️ Settings — Application settings to be
implemented». То же самое (даже без T002-карточки) в `git.rs`, `s3.rs`,
`extensions.rs`. Пользователь явно попросил оживить пустые вкладки,
Settings — в приоритете (git/s3/extensions трекаются отдельно, T009–T011,
внизу этого документа коротко).

Цель — реальная форма редактирования `config.toml`, вместо текста
«to be implemented».

## Scope v1

**Живые секции (пишут в `config.toml`, эффект применяется через уже
существующий hot-reload — `root.rs` уже слушает файл и вызывает
`apply_config` при изменении, ничего нового в этой части не нужно):**

- **Theme**: `mode` (Light/Dark/System — три кнопки-переключателя,
  визуально сразу видно активный по T001-палитре), `accent` (текстовое
  поле, именованный цвет или hex — P1 ограничение, полная кастомизация
  в P5, см. комментарий в `settings.rs` конфига).
- **UI**: `default_sort` (dropdown: Name/Modified/Size/Kind — сверить
  точные варианты `SortOrder` в `config/settings.rs`), `show_hidden`
  (toggle), `icon_pack` (текстовое поле — P1 нет пикера паков, только
  строка).
- **Explorer**: `split_direction` (Vertical/Horizontal toggle),
  `synced_panes` (toggle), `restore_tabs` (toggle).

**Черновые секции — показаны, задизейблены, с подписью «coming in
P3/P4»:** Keybindings, Plugins, Indexing, Search, Launcher. Рендерятся
серым, поля недоступны для ввода — честно показывают, что есть в схеме,
не редактируют то, что ни на что не влияет (решение пользователя: лучше
показать заглушку, чем ничего).

## Архитектура записи (без потери ручных правок)

`config.toml` уже может быть вручную отредактирован с inline-
комментариями (см. живой файл — `mode = "system"   # "system" |
"light" | "dark"`). Полный `toml::to_string(&Config)` round-trip убивает
комментарии и форматирование. Вместо этого — **точечная правка через
`toml_edit::DocumentMut`** (уже в дереве зависимостей транзитивно —
`Cargo.lock` содержит `toml_edit`, добавить прямую зависимость в
`chronos-fm-core`, не тянуть новый крейт с нуля):

1. Прочитать сырой текст `config.toml` в `DocumentMut`.
2. Для каждого изменённого UI-поля — точечно установить значение по
   пути (`doc["theme"]["mode"] = value.into()`), не трогая остальные
   ключи/комментарии/whitespace.
3. Записать документ обратно (`doc.to_string()` → `fs::write`).
4. Существующий config-watcher (`root.rs::start_config_watch`) подхватит
   изменение и вызовет `apply_config` — эффект применяется тем же
   путём, что и ручное редактирование файла руками, ничего не дублируем.

Если пользователь одновременно правит файл руками, пока открыт Settings
tab — v1 принимает risk последнего-записавшего (last write wins), не
делает конфликт-мёрж. Не тикет на файловую синхронизацию.

## UI

Т002-паттерны (`elevated_card`+`section_header`) на каждую секцию —
тот же визуальный язык, что уже применён к самой карточке-заглушке.
Toggle/dropdown — сверить, есть ли уже готовые gpui-component виджеты
(`Switch`, `Select`/`Dropdown` — Longbridge toolkit почти наверняка их
имеет, не писать свои с нуля, это будет частью implementation plan).

## Вне скоупа v1

- Живое редактирование Keybindings/Plugins/Indexing/Search/Launcher —
  их подсистемы не подключены (P3/P4), редактировать нечего.
- Конфликт-резолюция при одновременном ручном+UI редактировании файла.
- Import/export пресетов темы.
- Полная кастомизация акцента (color picker) — P5 по роадмапу.

## Верификация

- `cargo build --workspace` + `cargo test --workspace` чисто.
- Живой прогон: открыть Settings tab, переключить `theme.mode` на
  Dark → окно перекрашивается (T001 живой toggle), `config.toml`
  на диске показывает изменённую строку, ОСТАЛЬНЫЕ строки/комментарии
  файла байт-в-байт не тронуты (diff перед/после — только одна строка).
  Toggle `show_hidden` → explorer действительно показывает/прячет
  dotfiles. Черновые секции визуально задизейблены, клик по ним ничего
  не делает.

## Коммит

`ui : live Settings tab — theme/ui/explorer editing via config.toml
(toml_edit, comment-preserving)` — после реализации по плану.

---

## T009–T011 — остальные вкладки-пустышки (трекинг, не в этом spec)

Все три требуют реального бэкенда, которого сейчас физически нет в
`chronos-fm-services` (ни git-, ни s3-, ни plugin-host-модуля) —
несопоставимо больший объём, чем Settings (конфиг-форма). Не спекать
блиндом в один заход, каждый — отдельный brainstorm, когда дойдёт
очередь:

- **T009 — Git tab**: показывать git-статус текущей директории
  (staged/modified/untracked), базовые действия (stage/commit) —
  нужен `git2`/шелл-обёртка вокруг `git`, решить какой подход.
- **T010 — S3 tab**: браузер S3-совместимого хранилища — нужны
  credentials-хранилище (keyring?), клиент (`aws-sdk-s3`/`rusty-s3`),
  решить scope (read-only browse vs upload/download).
- **T011 — Extensions tab**: витрина плагинов — жёстко зависит от P4
  (`chronos_luau` plugin-host, ADR 0009) — не начинать раньше, чем
  plugin-host физически существует, иначе показывать нечего.
