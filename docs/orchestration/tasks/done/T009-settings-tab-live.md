# T009 — Settings-таб: живое редактирование config.toml

**Приоритет:** P1 — приоритет пользователя из четырёх вкладок-пустышек.
**Роль:** реализация по готовому implementation plan, TDD.
**Источник:** design spec `docs/superpowers/specs/2026-08-06-settings-
tab-live.md` + implementation plan `docs/superpowers/plans/2026-08-06-
settings-tab-live.md` (3 таска, TDD, полный код в плане — читать план
целиком перед началом).

## Контекст

`crates/chronos-fm-pages/src/settings.rs` — заглушка «⚙️ Settings —
Application settings to be implemented». Цель — реальная форма:
theme.mode/accent, ui.show_hidden/default_sort/icon_pack,
explorer.split_direction/synced_panes/restore_tabs редактируются в UI,
пишутся точечно в `config.toml` через `toml_edit` (комментарии/
форматирование НЕ теряются — жёсткое требование spec), эффект
подхватывается уже существующим config-watcher в `root.rs` без
дополнительного кода. Черновые секции (keybindings/plugins/indexing/
search/launcher) — показаны, задизейблены, не редактируются (решение
пользователя).

## Что нужно

Три таска по порядку:
1. `config::patch` модуль — чистая функция `patch_config_text` (TDD,
   готовые тесты в плане) + `patch_config_file`.
2. `default_config_path()` — переиспользовать существующую логику
   резолва пути (план явно требует НЕ дублировать, сначала грепнуть).
3. `SettingsPage` — живая форма на `gpui_component::Switch` +
   T002-паттернах, точечные кнопки-переключатели для enum-полей.

## Зона файлов

`crates/chronos-fm-core/src/config/patch.rs` (новый),
`crates/chronos-fm-core/src/config.rs` (точечно),
`crates/chronos-fm-pages/src/settings.rs` (переписывается),
`crates/chronos-fm-pages/Cargo.toml`/`crates/chronos-fm-core/Cargo.toml`
(deps). `SettingsPage::new()` меняет сигнатуру (принимает `Config`) —
план явно требует найти ВСЕ call sites конструктора перед правкой, не
трогать остальные вкладки/файлы.

## Верификация

Полный список — в конце implementation plan, "Final Live Verification":
`cargo build`+`test` чисто, живой клик Dark в Settings → окно
перекрашивается (T001 живой toggle), `config.toml` на диске меняет
РОВНО одну строку (сверить diff до/после, остальные комментарии/
форматирование нетронуты), show_hidden реально переключает видимость
dotfiles в эксплорере, черновые секции визуально задизейблены без
реакции на клик.

## Отчёт

`docs/orchestration/tasks/report/T009-settings-tab-live-report.md`
(inbox). Приёмка — архитектор лично, та же дисциплина, что T001-T003
(грепы/дифф/build/test + живой grim + сверка diff config.toml, отчёту
на слово не верить). Принят → `report-log/`, тикет → `done/`.
Отклонён → `rejected/`.

## Коммит

По одному коммиту на таск, сообщения — в самом плане.
