# T033 — Kill watcher ↔ self-index feedback loop

> ## ✅ ARCHITECT VERDICT: **PARTIAL-ACCEPT** (2026-08-09)
>
> Architect stamp. Path 4 accepted. See
> `report-log/T033-kill-watcher-self-index-feedback-loop-report.md`.
> Ticket → `done/`. Next umbrella: AFTER packet → T014-A gate.

**Статус (sync, 2026-08-09):** исполнение по Пути 4. Архитектурное
решение архитектора (см. § «Решение архитектора» ниже): фильтрация
excluded путей в debouncer callback + `.chronos-fm` в
`DEFAULT_EXCLUDE_COMPONENTS`, defense-in-depth в `process_changes`,
Recursive fast-path нетронут. Диагностика §4 — не gate. Корневая
причина и кандидат-фикс уже даны в T022 §4 (отчёт
`T022-perf-quick-wins-report.md`).

**Родитель:** T014 (umbrella 144 fps). После T022 — это первый
инженерный тикет в цепочке до T014-A. **Rejected:** ничего не
трогать «на всякий случай», пока не выбран путь.

## Корневая причина — одна фраза

Файл-вотчер (`FileWatcher::new` в
`crates/chronos-fm-services/src/search/watcher.rs`) рекурсивно
следит за `$HOME`, а сам поисковый индекс живёт в
`~/.chronos-fm/index/**` → каждый tantivy-коммит пишет `meta.json`
/ сегментные файлы в наблюдаемое поддерево → `notify-debouncer-mini`
стреляет → `process_changes` пытается переиндексировать эти
`meta.json` → следующий коммит → write → loop. Никакой внешней
активности не требуется.

**Эмпирически:** на этой же машине (RTX 3070, Samsung 144 Hz DP-1,
warm index 3499 docs) счётчик `grep -c 'Starting merge' /tmp/cfmR.log`
растёт ~1 раз/сек в покое, без единого пользовательского жеста.
Thread snapshot в покое: `merge_thread_0` ~50%, `index_writer` ~27%,
`notify-rs debou` ~14%, `notify-rs inoti` ~16%.

**Почему T022 этого не поймал:** Part D (debounce 5 s) снижает
частоту батча в ~2.5×, но петля остаётся — meta.json всё равно
переиндексируется, фьюзы на сегмент всё равно случаются, всё
ровно с тем же качественным паттерном, только медленнее.

## Что нужно ПЕРЕД кодом (все три пункта — диагностика)

> **Вердикт архитектора (2026-08-09):** § «Решение архитектора» ниже
> явно указывает, что эта диагностика — **не gate**. T022 уже дал
> merge rate ~1/s idle + meta.json mtime каждые ~2 s, и эмпирика этой
> машины достаточна для commit'а Пути 4. Пункты могут быть исполнены
> как **опциональные smoke-проверки** уже **после** кода (лог «dropped
> N excluded paths» + per-batch prefix histogram).

1. **`process_changes` действительно получает пути под
   `~/.chronos-fm/index/`?** Проверка через `tracing::debug!` (или
   короткое добавление в логе уровня info) на каждом батче — один
   прогон в 10 секунд покажет, какие префиксы приходят.
   `index_single_file` (в `search/indexer.rs`) фильтрует binary
   (null-byte) и > 10 MB, но `meta.json` — текстовый, проходит, и
   коммит на него переписывает файл → следующая итерация цикла.
2. **Сколько энтропии в нагрузке watcher'а именно от нашего
   индекса?** Простое измерение: под нагрузкой-off (выключить
   watcher-нить, оставить только index writer) — растёт ли
   счётчик `Starting merge` так же? Если off → ноль, значит петля
   идёт ровно через watcher.
3. **На fast-path'е watcher'а (`Recursive` notify add) можно
   эффективно удалить watches под `~/.chronos-fm`?** Проверить
   Linux inotify limits + поведение `notify::RecommendedWatcher`:
   после `watch(&home, Recursive)` доступен ли `unwatch(handle)`
   для watches, не имеющих ещё handle'а через публичный API.
   **Closed as moot** — Путь 4 не использует unwatch.

## Что сделать — два пути к выбору

### Путь 1 (рекомендуется технически, но меняет контракт fast-path)

**Заменить fast-path `Recursive` на filtered fallback всегда**, то
есть watcher всегда идёт по
`crates/chronos-fm-services/src/search/watcher.rs:53` (fallback
walk) с исключениями. Это даёт:

* ✅ Полная фильтрация: `.chronos-fm` в `DEFAULT_EXCLUDE_COMPONENTS`
  сразу работает — нужное поведение из коробки.
* ❌ Теряется **покрытие новых директорий**: если юзер создаёт
  `~/newproject/`, fallback watch не подхватит его до перезапуска
  watcher'а. (Fast-path это даёт, потому что inotify рекурсивен
  ядром.)

**Контрактное изменение:** watcher больше не «реактивен» на новые
subdirs в `$HOME`. Документировать в `crates/chronos-fm-services`
README и `search/watcher.rs` doc-comment.

### Путь 2 (минимально инвазивный, но неполный)

**Явно «забыть» subtree `~/.chronos-fm`** после успешного
recursive watch — через `debouncer.watcher()` собрать handles для
всех watch'ей и unwatch'нуть те, что под `~/.chronos-fm`.

* ✅ Минимальное изменение: всё семейство Recursive покрытие
  сохраняется.
* ❌ Корявое API: `notify::RecommendedWatcher` на Linux (inotify)
  не имеет стандартного «unwatch subtree», придётся либо хранить
  per-dir handles, либо рекурсивно walk и unwatch каждый.
  Объём — заметный.
* ❌ Если юзер переустановит конфиг и путь к индексу сменится на
  что-то другое — придётся обновлять путь.

### Путь 3 (гибрид — то, что я бы попробовал первым)

В `FileWatcher::new` **после successful recursive watch добавить
в DEFAULT_EXCLUDE_COMPONENTS путь `index` через конфигурируемое
поле** — и переписать `FastPath`-ветку так, чтобы она для каждого
наблюдаемого subdir вызывала `Watch::new` и затем сразу
`debouncer.watcher().unwatch(handle)` если subdir матчит excluded.
Это то же, что Путь 2, но реализованное через нормальный API
`notify_debouncer_mini::Debouncer` (по одному `watcher().watch()` с
`Recursive=false` для каждого readable dir'а из fallback-walk).

* ✅ Полная фильтрация.
* ✅ Покрытие новых subdir'ов сохраняется через пере-existing
  watches (любая новая директория потребует отдельного рестарта,
  если она НЕ существует на момент построения walker'а).
* ❌ Не покрывает runtime-созданные директории — для них нужен
  watcher-канал `notify::Watcher::event` с фильтром «event в
  watched dir → пересоздать watch на новый subdir».

### Решение архитектора (2026-08-09) — **Путь 4 ACCEPT**

**Не 1 / 2 / 3.** Ложная развилка: петлю рвёт отсутствие
`process_changes`→commit на index-файлы, а не снятие watch.

**Путь 4 — filter-at-callback + exclude-component; Recursive fast-path
оставить:**

1. `DEFAULT_EXCLUDE_COMPONENTS` += `".chronos-fm"`.
2. Debouncer callback: `filter(|p| !excludes.matches(p))`; empty
   batch не слать.
3. `process_changes`: skip excluded paths (defense-in-depth).
4. Без always-fallback, без unwatch-subtree.

| Путь | Вердикт |
| --- | --- |
| 1 always-fallback | reject — теряет runtime new-dir |
| 2 unwatch-subtree | reject — API/Linux mess |
| 3 hybrid | reject — contract loss + лишний код |
| **4 filter + exclude** | **accept** |

Диагностика «ПЕРЕД кодом» § выше — **не gate**. T022 evidence
достаточно; optional smoke-log после фикса. Код **можно писать**.

## Зона файлов

* `crates/chronos-fm-services/src/search/exclusions.rs:13` — добавление `.chronos-fm`
  в `DEFAULT_EXCLUDE_COMPONENTS` (нужно для Пути 1 и Пути 3; для
  Пути 2 — формально не нужно, потому что fast-path exclude'ы
  scoped'ятся иначе).
* `crates/chronos-fm-services/src/search/watcher.rs` — основной
  change (выбор пути определяет структуру).
* Возможно: `crates/chronos-fm-core/src/config.rs` — если выносим
  `index_root` в пользовательский config.

## Координация

* **T022** — closed/partial; передаёт эстафету этой задаче.
* **T010/T011** (Git/S3) — пересечения нет, оба персистят свой state,
  но не трогают watcher / search service.
* **T014-A** — после завершения T033 берётся решение по
  memoization на основе post-T033 цифры. T033 сам по себе не
  трогает layout-код — только watcher.
* **T008/T016** (devices panel / search survive unreadable subdir) — оба
  опираются на watcher-контракт; Путь 1 меняет контракт, поэтому
  эти тикеты нуждаются в re-walk против контракта после T033.

## Верификация

* `perf record -F 1000 -g -p $(pidof chronos-fm) -- sleep 10` в
  покое, та же машина и папка (~114 items), что в T014 — ДО и
  ПОСЛЕ. **Ожидание:** доля `notify-rs debouncer` падает с ~14 %
  до фонового шума (< 5 %); доля `merge_thread_0` падает с ~50 %
  до < 5 %. Это закроет основной кусок T014 «~30 % CPU в покое».
* `cargo test -p chronos-fm-services`: существующие кейсы
  watcher-выживания должны проходить. Если выбран Путь 1
  или 3 — добавить тест «watcher не подписывается на
  `$HOME/.chronos-fm/index`».
* `grim` screenshot до/после: внешний вид списка не меняется.
* **Не требует замеров scroll/hover**: цикл жил в idle-side CPU,
  layout-cost taffy в scroll/hover не зависит от watcher'а.

## Отчёт

`docs/orchestration/tasks/report/T033-kill-watcher-self-index-feedback-loop-report.md`
(inbox). **Приёмка — архитектор:** принят → тикет и отчёт →
соответственно `done/` и `report-log/`. До приёмки всё лежит в
inbox; я **не двигаю** файлы сам.
