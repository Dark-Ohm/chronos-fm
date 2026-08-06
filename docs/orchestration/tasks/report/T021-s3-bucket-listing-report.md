# T021 — Отчёт: после connect список бакетов не запрашивается ни разу

**Статус:** выполнено (код + юнит-тесты, корень установлен и закрыт).
**Приоритет:** P1. Приёмка — архитектор лично (живой прогон против RustFS).
**Корень:** установлен в тикете, подтверждён кодом и закрыт регрессионными
тестами (см. ниже).

## Корень (подтверждён кодом)

Взаимоисключающие контракты двух милестоунов T011:

- **B.4** (`s3.rs:151-156`) после успешного `S3Client::from_profile` делал
  `set_provider` + `cwd = s3_root` + `loaded = false` в расчёте на то, что
  рендер подхватит и загрузит.
- **B.1** (`navigation.rs`) превратил `reload()` в no-op при наличии
  provider'а (`loaded = true; return;`), а `reload_provider` вызывается
  только из навигационных методов (`change_dir`, `go_back`, `go_forward`).

Единственный, кто реагирует на `loaded = false`, — рендер
(`view.rs:21` → `ensure_loaded()` → `reload()` → ветка no-op). После
connect навигации не происходит → ни один вызов `reload_provider` не
срабатывает → провайдер подключён, но не опрошен ни разу. Отказ полностью
молчаливый: лог модуля `chronos_fm_services::s3` пуст, RustFS не получает
запросов, UI показывает «0 items» навсегда.

Проверено грепами из тикета: `reload_provider` — 3 вызова, все
навигационные (`navigation.rs:148/228/243`); `ensure_loaded` — 1 вызов из
рендера (`view.rs:21`).

## Что сделано

### 1. Загрузка сразу после `set_provider` (`s3.rs`)

В async-колбэке connect (внутри `this.update_in`, где доступен `window`)
вместо `pane.loaded = false` теперь вызывается `pane.reload_provider(window, cx)`
через `WeakEntity<ExplorerPane>::update_in`:

```rust
pane.downgrade()
    .update_in(cx, |pane, window, cx| {
        pane.set_provider(client.clone());
        pane.cwd = s3_root.clone();
        pane.reload_provider(window, cx);
        cx.notify();
    })
    .log_err();
```

Почему `update_in` (а не `pane.update`): `reload_provider` требует
`&mut Window` + `&mut Context<ExplorerPane>`, а `Entity::update_in` в этом
форке gpui требует `C: VisualContext`, которым `Context<S3Page>` не
является. `WeakEntity::update_in` принимает `C: AppContext` и сам
резолвит окно панели через `with_window(entity_id)` (панель создана в том
же окне, что и S3Page).

### 2. `ensure_loaded` научен уходить в provider-ветку (`navigation.rs`, `view.rs`)

Выбран осознанно (второй вариант из тикета) **в дополнение** к явному
вызову из п.1, а не вместо него:

- Явный вызов в колбэке — первичный механизм (не полагается на рендер и
  срабатывает даже если панель не отрисована в тот же кадр).
- Хардненинг `ensure_loaded(&mut self, window, cx)` закрывает **класс**
  дефекта: любой будущий `loaded = false` на провайдер-панели больше не
  может молча no-op'нуть. `reload_provider` синхронно ставит `loaded = true`
  до спавна, поэтому двойной загрузки нет (подтверждено тестом, п.5).

```rust
pub(crate) fn ensure_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.loaded {
        return;
    }
    if self.provider.is_some() {
        self.reload_provider(window, cx);
    } else {
        self.reload();
    }
}
```

Единственный вызывающий — `view.rs` (рендер), где `window` доступен.
Сигнатуру менять было допустимо: других вызывающих нет.

### 3. Молчаливый отказ убран (`view/listing.rs`)

Ошибка `list_dir` в provider-ветке и раньше выставляла
`set_status(StatusLevel::Error, …)`, но статус показывался только в
футере приложения, который отражает статус **главного** эксплорера, а не
встроенной панели S3. Теперь листинг рендерит инлайн-баннер
(⚠ + текст, цвет `theme::danger`), как в T016 (`search_bar.rs`):

- Баннер **гейтирован** на `page.provider.is_some()`: для локальной ФС
  ошибка и так видна в футере, поэтому в обычном эксплорере поведение не
  меняется. В S3-табе «сервер недоступен» теперь неотличим от «бакетов
  нет»: ошибка показывается, пустой список — нет.
- Баннер живёт внутри рендера самой панели, поэтому перерисовывается
  естественным образом при `cx.notify()` из `reload_provider` (не нужна
  подписка S3Page на панель).

### 4. Проверка остальных точек `loaded = false` (п.3 тикета)

- `explorer/page.rs:305` (`configure_tab`: session-restore, split,
  new-tab) — панели `ExplorerPage` провайдер никогда не получают
  (`set_provider` вызывается только из `s3.rs`), локальный `reload()`
  работает. Безопасно.
- `explorer/tests.rs` (тестовый хелпер) — безопасно.
- Плюс п.2: даже если в будущем провайдер появится на такой панели,
  контракт `loaded = false` уже безопасен.

## Тесты

`crates/chronos-fm-pages/src/explorer/tests.rs`, фейковый
`FileSystemProvider` (`CountingProvider`: in-memory, считает вызовы
`list_dir` через `AtomicUsize`, умеет возвращать фиксированный список и
ошибку):

| Тест | Что ловит |
|---|---|
| `provider_backed_pane_lists_immediately_after_set_provider` | путь connect: `set_provider` + `cwd` + `reload_provider` → `list_dir` вызван ровно **1** раз, entries применены, `loaded = true`; повторный `ensure_loaded` не вызывает повторный poll (нет двойной загрузки) |
| `ensure_loaded_routes_provider_pane_to_reload_provider` | класс дефекта: `loaded = false` + провайдер → `ensure_loaded` опрашивает провайдера (1 вызов), а не молча no-op'ит |
| `provider_listing_error_surfaces_in_status` | ошибка `list_dir` → `status_for_footer` = Error с текстом ошибки (это то, что баннер рендерит) |

## Верификация

- `cargo check -p chronos-fm-pages` — **чисто** (только предсуществующие
  `missing_docs`-варнинги на не-тронутых публичных предметах).
- `cargo test -p chronos-fm-pages` — **77/77 passed** (включая 3 новых).
- `cargo clippy -p chronos-fm-pages --lib` — новых замечаний нет
  (единственный в тронутом файле — предсуществующий
  `unnecessary_cast` на `navigation.rs:104`, вне зоны тикета).
- Живой прогон против RustFS (критерий приёмки из тикета: список бакетов
  виден сразу после connect, без навигации; `data/`, `notes/`,
  `readme.txt` внутри бакета; ошибка connect видна баннером) — **за
  архитектором** (headless-сессия, GUI не запускается).

## Зона файлов

- `crates/chronos-fm-pages/src/s3.rs` — connect-колбэк: явный
  `reload_provider` после `set_provider` (+ импорт `LogErr`).
- `crates/chronos-fm-pages/src/explorer/navigation.rs` — `ensure_loaded`
  диспетчеризует provider-панели в `reload_provider`.
- `crates/chronos-fm-pages/src/explorer/view.rs` — вызов `ensure_loaded(window, cx)`.
- `crates/chronos-fm-pages/src/explorer/view/listing.rs` — баннер ошибки
  (гейт на `provider.is_some()`).
- `crates/chronos-fm-pages/src/explorer/tests.rs` — `CountingProvider` + 3 теста.

Коммит не создавался (по соглашению сессии — только по явному запросу).
