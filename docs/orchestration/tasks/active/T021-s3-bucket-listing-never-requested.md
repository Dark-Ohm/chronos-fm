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

## Что нужно

1. Вызывать загрузку сразу после `set_provider` — в том же
   async-колбэке `s3.rs` есть `update_in`, то есть доступен `window`:
   ```rust
   pane.update(cx, |pane, cx| {
       pane.set_provider(client.clone());
       pane.cwd = s3_root.clone();
       pane.reload_provider(window, cx);
   });
   ```
   Альтернатива — научить `ensure_loaded` уходить в provider-ветку, но
   у него нет `window`; тогда придётся менять сигнатуру. Выбрать
   осознанно, а не по пути наименьшего сопротивления.
2. **Убрать молчаливый отказ.** Сейчас пустой список неотличим от
   «бакетов нет» и от «сервер недоступен». Ошибка `list_dir` в
   provider-ветке должна попадать в UI, как это сделано для поиска в
   T016 (`⚠ Full-text search unavailable`).
3. Проверить остальные точки, где `loaded = false` выставляется в
   расчёте на рендер, — контракт B.1 ломает их все одинаково.

## Тесты

Юнит на `ExplorerPane` с фейковым `FileSystemProvider` (in-memory,
считает вызовы `list_dir`): после `set_provider` + `cwd` + пути,
которым идёт connect, счётчик `list_dir` должен стать `1`. Именно этот
тест ловит класс дефекта — «provider подключён, но не опрошен».

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

## Отчёт

`docs/orchestration/tasks/report/T021-s3-bucket-listing-report.md`.
Приёмка — архитектор лично. Принят → `report-log/`, тикет → `done/`.
