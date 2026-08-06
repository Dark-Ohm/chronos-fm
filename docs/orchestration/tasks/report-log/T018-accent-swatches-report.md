# T018 — Отчёт: свотчи accent-цвета рисуются неверным цветом (rgba вместо rgb)

**Статус:** выполнено (правка + юнит-тест, корень установлен и закрыт).
**Приоритет:** P2. Приёмка — архитектор лично (живой прогон, критерий ниже).
**Корень:** установлен в тикете, подтверждён кодом и закрыт регрессионным
тестом (см. ниже).

## Корень (подтверждён кодом)

`crates/chronos-fm-pages/src/settings.rs:100` (внесено в `f0ab77e`, follow-up
к T009):

```rust
let fill = gpui::Hsla::from(gpui::rgba(color.rgb));
```

`gpui::rgba(hex: u32)` читает `0xRRGGBBAA` (`../Source/gpui/src/color.rs:20`),
а `ACCENT_PALETTE` хранит 6-значные `0xRRGGBB`
(`crates/chronos-fm-core/src/config/settings.rs:122`). Значение сдвигается
на байт: красный канал всегда `0x00`, зелёный получает бывший красный,
синий — бывший зелёный, альфа — бывший синий. Совпадение с попиксельной
картиной из тикета воспроизведено тестом (см. «Тесты»: с `rgba` тест падает
с точной строкой `blue: green channel 0.000 != 0.478` — ровно колонка
«Что видно» для `blue 0x007acc`).

Грепом из тикета проверено: `rgba(0x` по всем UI-крейтам — 3 вхождения:

| Место | Литерал | Длина | Вердикт |
|---|---|---|---|
| `settings.rs:100` (fill свотча) | `color.rgb` (6-значный) | 6 hex | **баг — исправлено** |
| `settings.rs:102` (hover_border) | `0xffffff99` | 8 hex | корректно, не трогали |
| `s3.rs:223` (подложка модалки) | `0x00000044` | 8 hex | корректно, не трогали |

Проверен и сам конструктор: `gpui::rgb` — отдельная функция
(`color.rs:14`), даёт `Rgba { r, g, b, a: 1.0 }`.

## Что сделано

### 1. Правка + выделение `accent_fill` (`settings.rs`)

Конверсия цвета вынесена из `accent_swatch` в приватную функцию —
чтобы регрессионный тест гонял **ту же самую** production-конверсию,
а не её копию (копия в тесте молча пережила бы возврат к `rgba`):

```rust
/// ... `gpui::rgba` reads `0xRRGGBBAA` — feeding it a 6-digit value shifts
/// every channel left by a byte (red always 0, alpha = former blue), which
/// was the T018 swatch-colour bug. ...
fn accent_fill(color: &chronos_fm_core::config::AccentColor) -> gpui::Hsla {
    gpui::Hsla::from(gpui::rgb(color.rgb))
}
```

`accent_swatch` теперь вызывает `accent_fill(color)`. `hover_border`
(`0xffffff99`, 8-значный) не тронут — п.3 тикета подтверждён.

### 2. Проверка остальных вызовов `rgba(` (п.2 тикета)

Таблица выше: единственное 6-значное вхождение — исправленное. Оба
оставшихся — честные `0xRRGGBBAA`, менять не нужно.

## Тесты

`crates/chronos-fm-pages/src/settings.rs`, `#[test] accent_swatch_colors_match_palette`:

- Для каждого цвета `ACCENT_PALETTE` прогоняет production-путь
  `accent_fill(color)` → `Hsla` → обратно `Rgba` (`impl From<Hsla> for Rgba`,
  чистая математика, `AppContext` не нужен).
- Сравнивает `r/g/b` с байтами палитры (`/255.0`) и `a` с `1.0`
  (свотч должен быть непрозрачным — старый баг давал мусорную альфу),
  допуск `0.01` (погрешность f32-раундтрипа HSL ≪ 1e-3; байтовый сдвиг
  нарушает допуск минимум в ~5× — teal, красный канал).
- **Анти-проверка:** временный возврат к `gpui::rgba` в `accent_fill`
  роняет тест (`blue: green channel 0.000 != 0.478`, 0/1 passed);
  после восстановления — зелёный. Баг больше не вернётся молча.

## Верификация

- `cargo test -p chronos-fm-pages` — **84/84 passed** (77 из отчёта T021 + 3
  T021-теста + 3 прочих + 1 новый T018; 0 failed).
- `cargo test --workspace` — **зелёный**: main 4, core 62, pages 84,
  services 91, store 16, ui 30; 0 failed во всех бинарях.
- `cargo clippy -p chronos-fm-pages --lib` — новых замечаний в
  `settings.rs` нет (предсуществующие `missing_docs` и варнинги зависимостей
  не трогаем).
- Живой прогон (критерий приёмки из тикета: семь свотчей визуально
  blue/green/purple/orange/red/teal/pink, при `accent = "purple"` обведён
  фиолетовый) — **за архитектором** (headless-сессия, GUI не запускается).

## Зона файлов

- `crates/chronos-fm-pages/src/settings.rs` — `accent_fill` (`gpui::rgb`,
  6-значный), вызов из `accent_swatch`, регрессионный тест
  `accent_swatch_colors_match_palette`.

Коммит не создавался (по соглашению сессии — только по явному запросу).
