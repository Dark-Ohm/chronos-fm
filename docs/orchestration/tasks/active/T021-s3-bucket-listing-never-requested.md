# T021 — После connect список бакетов не запрашивается ни разу

**Приоритет:** P1 — S3-таб доходит до состояния `Browsing` и показывает
пустой список навсегда. Milestone B.4 («Bucket listing») своей цели не
достигает.
**Статус:** баг с **установленным корнем**, воспроизведён живьём против
локального RustFS.
**Источник:** живой прогон T011 2026-08-06 (первый за всё время
существования тикета).

## Симптом

1. Профиль в `config.toml`, endpoint `http://127.0.0.1:9000`,
   `force_path_style = true`, на сервере два бакета и четыре объекта.
2. S3-таб → состояние `NeedCredentials` рисуется верно («Connect to
   rustfs», endpoint, два поля, кнопка).
3. Ввод ключей → «Connect» → состояние меняется на `Browsing`, path bar
   показывает `s3: > rustfs@`, встроенный ExplorerPane отрисован.
4. **`0 items`. Навсегда.**

Ключевое наблюдение — не в UI, а в двух логах:

- в логе приложения **ноль** записей модуля `chronos_fm_services::s3`
  (при `RUST_LOG=chronos_fm_services=debug`);
- контейнер RustFS **не получил ни одного запроса**.

То есть это не ошибка листинга и не проблема совместимости с сервером:
запрос не отправляется вообще. Ошибок в логе тоже нет — отказ полностью
молчаливый.

## Корень (по коду, не гипотеза)

Два милестоуна установили взаимоисключающие контракты.

**B.4** (`crates/chronos-fm-pages/src/s3.rs:151-156`) после успешного
`S3Client::from_profile` делает:

```rust
pane.set_provider(client.clone());
pane.cwd = s3_root.clone();
pane.loaded = false;   // ← расчёт: рендер подхватит и загрузит
cx.notify();
```

**B.1** (`crates/chronos-fm-pages/src/explorer/navigation.rs:31-38`)
ранее превратил `reload()` в no-op при наличии provider'а:

```rust
pub(crate) fn reload(&mut self) {
    if self.provider.is_some() {
        // S3/remote: don't block the UI thread. `reload_provider`
        // will be called from navigation methods (change_dir, go_back,
        // go_forward) which have window+cx for spawning.
        self.loaded = true;
        return;
    }
```

Путь рендера — единственный, кто реагирует на `loaded = false`:
`view.rs:21` → `ensure_loaded()` (`navigation.rs:22`) → `reload()` →
ветка выше → `loaded = true`, и на этом всё.

`reload_provider` имеет ровно три места вызова, все — навигационные:
`change_dir` (`navigation.rs:148`), `go_back` (`:228`),
`go_forward` (`:243`). После connect навигации не происходит, поэтому
**ни один из них не срабатывает**.

Проверяется двумя грепами: `grep -rn "reload_provider" crates/` и
`grep -rn "ensure_loaded" crates/`.

## Заход №1 (коммит `32cb1ac`) — не сработал, и почему

Первая правка сделана, закоммичена как `wip` и **проверена живьём: не
работает**. Разбор обязателен к прочтению — вторая попытка по тем же
граблям недопустима.

Что было сделано: `pane.loaded = false` заменён на

```rust
pane.downgrade()
    .update_in(cx, |pane, window, cx| { … })
    .log_err();
```

Что наблюдалось живьём против RustFS: `0 items`, **ноль запросов** к
серверу, ноль строк модуля `chronos_fm_services::s3`. В логе — ровно
одна строка, объясняющая всё:

```
WARN chronos_fm_core::telemetry: crates/chronos-fm-pages/src/s3.rs:165:
     entity has no current window
```

Причина: `WeakEntity::update_in` (`../Source/gpui/src/app/entity_map.rs:792`)
резолвит окно через `cx.with_window(entity_id)` →
`current_window_by_entity.get(&entity_id)?` (`app.rs:1717`). Эта карта
заполняется **только** из `ensure_window`, который зовётся из трёх мест —
`defer_in`, `observe_in`, `subscribe_in` (`app/context.rs:312/335/368`),
причём для собственного `entity_id` того контекста. Панель создаётся
через `cx.new(...)` в контексте `S3Page`; ни один из трёх методов на её
контексте не вызывается, значит её id в карте нет и `update_in`
возвращает `Err`.

**Стена, в которую упёрся исполнитель (мой пробел в первом брифе):**
`Entity::update_in` (`entity_map.rs:502`) требует `C: VisualContext`,
а `Context<S3Page>` им не является — прямой вызов не компилируется.
Отсюда и поиск обхода. Обход через `WeakEntity` компилируется, но
молча не работает.

**Правильный выход — не искать окно, а не терять его.** Внешний колбэк
уже получает окно из `spawn_in` и выбрасывает его подчёркиванием:

```rust
let _ = this.update_in(&mut cx, |this, _window, cx| {   // ← вот здесь
```

## Что нужно

1. Принять `window` во внешнем колбэке (`|this, window, cx|`) и
   прокинуть его в обычный `pane.update(cx, …)`:
   ```rust
   let _ = this.update_in(&mut cx, |this, window, cx| {
       pane.update(cx, |pane, cx| {
           pane.set_provider(client.clone());
           pane.cwd = s3_root.clone();
           pane.reload_provider(window, cx);
       });
       this.state = S3State::Browsing;
       cx.notify();
   });
   ```
   Если компилятор возразит по заимствованиям — **решать по месту, не
   подменяя механизм**. Возврат к `WeakEntity::update_in` в любом виде
   означает повтор `32cb1ac`.
2. **Не полагаться на `.log_err()` как на сигнал.** Он пишет через
   `log::Level::Error`, мост `log → tracing` есть (`try_init` +
   `tracing-log`), но выходит это на уровне **WARN** с таргетом
   `chronos_fm_core::telemetry`. Греп по `ERROR` его не находит — на
   этом потерял время и я. Ошибка соединения S3Page ↔ панель должна
   идти через `tracing::error!` с внятным текстом.
3. Уже сделано в `32cb1ac` и **сохранить**: `ensure_loaded` уходит в
   provider-ветку, инлайн-баннер ошибки листинга, `CountingProvider`.
   Переделывать не нужно.

## Тесты

Три теста из `32cb1ac` зелёные и дефект не поймали — они дёргают панель
напрямую, а сломано звено `S3Page → панель`. Нужен тест **через
S3Page**, а не через панель:

- построить `S3Page`, довести до состояния connect с фейковым клиентом
  (или замокать `S3Client::from_profile`), выполнить тот же колбэк и
  проверить, что `CountingProvider::list_dir` вызван **1 раз** и
  `pane.cwd` стал `s3://<profile>@`.
- если поднять `S3Page` в тесте дорого — минимум негативный тест:
  панель, созданная как в `start_connect` (через `cx.new` без
  `defer_in`/`observe_in`/`subscribe_in`), и ассерт, что
  `WeakEntity::update_in` на ней возвращает `Err`. Это фиксирует
  ловушку в коде, а не в отчёте.

## Зона файлов

`crates/chronos-fm-pages/src/s3.rs` (async-колбэк connect),
`crates/chronos-fm-pages/src/explorer/navigation.rs`
(`reload` / `ensure_loaded` / `reload_provider`).

## Верификация

Локальный RustFS вместо MinIO (Apache-2.0, не AGPL):

```bash
podman run -d --name chronos-rustfs -p 9000:9000 -p 9001:9001 \
  -v chronos-rustfs-data:/data -v chronos-rustfs-logs:/logs \
  docker.io/rustfs/rustfs:latest
```
Ключи по умолчанию `rustfsadmin` / `rustfsadmin`, API на `:9000`.
**Bind-маунт хостовых каталогов под rootless podman не работает**
(`PermissionDenied` на `/data`) — использовать именованные тома.

Скрипт наполнения (boto3, `force_path_style`) —
`seed_s3.py` из прогона 2026-08-06: два бакета, объект в корне и два
общих префикса.

Критерий: после connect список бакетов виден **без** навигации; заход в
бакет показывает префиксы `data/` и `notes/` и ключ `readme.txt`.

**Приёмка считается пройденной только при всех трёх подтверждениях —
скриншот сам по себе не доказательство, `32cb1ac` тоже «показывал
панель»:**

1. Бакеты видны на экране (`grim`).
2. `podman logs chronos-rustfs` содержит запросы от приложения — сервер
   реально опрошен.
3. В логе приложения при `RUST_LOG=chronos_fm_services=debug` есть
   строки модуля `chronos_fm_services::s3`, и **нет** ни одной строки
   `entity has no current window` (грепать по `WARN`, не только `ERROR`).

Пункты 2 и 3 обязательны: именно они, а не картинка, отличают
работающий листинг от `32cb1ac`, где UI выглядел правдоподобно при
нулевой сетевой активности.

**Сборка:** голый `cargo build --release` бинарь не собирает — GUI-крейты
вне `default-members`. Нужен `cargo build --release -p chronos-fm`, и
перед прогоном сверить `stat -c '%y' target/release/chronos-fm` с mtime
правленых исходников.

## Отчёт

`docs/orchestration/tasks/report/T021-s3-bucket-listing-report.md`.
Приёмка — архитектор лично. Принят → `report-log/`, тикет → `done/`.
