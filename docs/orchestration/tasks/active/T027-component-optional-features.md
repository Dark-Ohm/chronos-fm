# T027 — Форк: опциональные фичи gpui-component (`decimal`, `inspector`)

**Приоритет:** P2. Разблокирован частью A тикета T025.
**Скоуп:** `Source/gpui-component`, только сборка и один запуск. **Кода
не трогать** — ни в `Source/`, ни в `Chronos-FM/`.

## Тикет меньше, чем я обещал — и вот почему

Я анонсировал этот тикет как «`lsp`, `decimal`, `inspector` не собирались
против форка ни разу». Проверил перед раздачей: **`lsp` уже собран.**

- `crates/ui/Cargo.toml:2` — `default = ["markdown", "html", "time",
  "chart", "lsp"]`.
- `crates/story/Cargo.toml:16` — `gpui-component = { workspace = true }`,
  то есть с дефолтными фичами.
- Значит зелёная сборка T025 уже прогнала `markdown`, `html`, `time`,
  `chart` и `lsp`. Плюс `tree-sitter` со всеми грамматиками — он в
  `default` у story (`Cargo.toml:10`).

Более того, модуль `inspector` в `crates/ui/src/lib.rs:12` закрыт
условием `#[cfg(all(any(feature = "inspector", debug_assertions),
feature = "lsp"))]`. В dev-профиле `debug_assertions` истинно, а `lsp`
включён по умолчанию — **сам модуль inspector в T025 тоже компилировался**.

Остаётся ровно две непроверенные вещи: **`decimal`** и **фича
`inspector`** (не модуль, а флаг).

## Что сделать

Из воркспейса `Source/gpui-component`:

1. `cargo check -p gpui-component --features decimal`
2. `cargo check -p gpui-component-story --features inspector`
3. `cargo check -p gpui-component --all-features` — контрольный прогон
   всего сразу, включая все 34 грамматики поимённо.

## Стены

**Стена 1 — `inspector` уезжает в наш gpui, а не остаётся в kit'е.**
Цепочка: `gpui-component/inspector` → `gpui/inspector` +
`gpui_macros/inspector` (`crates/ui/Cargo.toml:17`,
`Source/gpui/Cargo.toml:12`, `Source/gpui_macros` — там фича пустая).
То есть флаг компилирует **`Source/gpui` по коду, который никто ни разу
не собирал**. Это и есть настоящее содержание тикета.

Если упадёт — **это дефект форка, и чинить его здесь нельзя**:
`Source/gpui` держит на себе ChronOS и greeter Chronos-lm. Зафиксировать
ошибки дословно, завести отдельным тикетом, остановиться.

**Стена 2 — `inspector` без `lsp` бесполезен.** Из-за `cfg` выше модуль
не появится, даже если фича включена. Проверять на story (у него `lsp`
приезжает по дефолту через gpui-component), а не на голом ui с
`--no-default-features`.

**Стена 3 — `--all-features` может не пройти по причинам, не связанным с
форком.** В наборе 34 грамматики tree-sitter от разных авторов; конфликт
версий `tree-sitter` между ними — это дефект апстрима kit'а, а не нашего
форка. Различать в отчёте: «упало в нашем gpui» против «упало в чужой
грамматике». Первое — важно, второе — сноска.

**Стена 4 — `cargo check` не то же самое, что `build`.** Для `inspector`
после успешного `check` сделать `cargo build -p gpui-component-story
--features inspector`: проц-макросы и кодоген у `gpui_macros` до
линковки могут не проявиться.

## Необязательная часть, только если всё три команды зелёные

Запустить `cargo run -p gpui-component-story --features inspector` и
снять кадр: открывается ли инспектор вообще. Одна строка, один
скриншот. Если не открывается — это находка, не провал; не чинить.
Грабли те же, что в T026: `ydotool` удваивает координаты, геометрию
брать из `hyprctl clients -j` в момент прогона, «открылось» судить по
кадру, а не по `exit 0`.

## Приёмка

- Три команды с их фактическим выводом (exit code + хвост).
- Для каждого падения — дословный текст ошибки и вердикт «наш форк /
  чужая грамматика / kit».
- `git -C Source status` чист: ноль правок в коде, `Cargo.toml` и
  `Cargo.lock` тоже не трогаем — фичи передаются флагом командной
  строки, а не правкой манифеста.
- Если делалась необязательная часть — кадр, просмотренный глазами.

## Отчёт

`docs/orchestration/tasks/report/T027-component-optional-features-report.md`.
Приёмка — архитектор лично.
