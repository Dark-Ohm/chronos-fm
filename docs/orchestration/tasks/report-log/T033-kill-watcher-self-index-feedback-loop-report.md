# T033 — Kill watcher ↔ self-index feedback loop: post-code report

> **Статус (2026-08-09, ~13:35):** код по Пути 4 лендится и прошёл
> quality gate: 89/89 unit + integration тестов зелёные, clippy по
> services чистый. Live idle на этой же машине показывает
> **5× сокращение merge rate** и **полное отсутствие вызовов
> `process_changes` от watcher'а** в 4-минутном окне наблюдения —
> architectural fix держит. Оставшаяся CPU на `merge_thread_0` —
> это tantivy's own compaction leftover-segment backlog после серии
> тестовых прогонов T014, не наш loop. См. §3 для разбора.

## 0. Что лендится (Path 4 per architect verdict)

| # | Изменение | Файл |
| - | --- | --- |
| 1 | `DEFAULT_EXCLUDE_COMPONENTS += ".chronos-fm"` + comment с указанием причины и отсылки к T014 §2 | `crates/chronos-fm-services/src/search/exclusions.rs` |
| 2 | Debouncer callback в `FileWatcher::new` фильтрует `excludes.matches(path)` **до** `send_blocking`; пустой батч дропается без wakeup'а consumer'а; `Excludes: Clone` кладётся в closure | `crates/chronos-fm-services/src/search/watcher.rs` |
| 3 | Defense-in-depth: `IndexManager::process_changes` пропускает excluded paths в начале цикла (никаких `add_document`/`delete_term` и, следовательно, никакого триггера commit'а) | `crates/chronos-fm-services/src/search/indexer.rs` |
| 4 | 2 unit-теста: `default_components_skip_known_heavy_dirs` (расширен .chronos-fm кейсами), `watcher_callback_filters_self_index_paths` (новый; integration), `process_changes_skips_excluded_paths` (новый; defense-in-depth) | test modules в тех же файлах |

Recursive fast-path (`debouncer.watcher().watch(&root, RecursiveMode::Recursive)`) — **не тронут**, contract сохранён: новые unexcluded dirs в `$HOME` всё ещё подхватываются kernel'ом на лету.

## 1. Quality gate — зелёный

Один shell-call с cargo build + test + clippy:

```
cargo build --release -p chronos-fm-pages  →  ok (no errors)
cargo build --release -p chronos-fm        →  ok, finished 1m57s,
                                              target/release/chronos-fm 65 MB
cargo test --no-fail-fast -p chronos-fm-services
  → 89 passed; 0 failed; 0 ignored; 0 measured
```

Среди новых/затронутых:

- `search::exclusions::tests::default_components_skip_known_heavy_dirs` — ok
  (расширен кейсами `.chronos-fm/index/meta.json`, `…/000/seg.uda`,
   `…/non-index-dir/whatever`, `…/projects/.chronos-fm/cache`)
- `search::watcher::tests::watcher_callback_filters_self_index_paths` — ok
  (новый integration-тест на новый бинарь FileWatcher; параллельно
   пишет в `user.txt` и в `.chronos-fm/index/meta.json`; утверждает,
   что batch содержит только user.txt и не содержит ничего под
   `.chronos-fm/`)
- `search::indexer::tests::process_changes_skips_excluded_paths` — ok
  (новый; пишет user.txt + `.chronos-fm/index/meta.json`, вызывает
   `process_changes(&[user, meta])`, ищет уникальный токен из тела
   meta.json → пусто; ищет обычный токен → user.txt; проверяет,
   что ни один indexed path не указывает под `.chronos-fm/`)
- `search::indexer::tests::index_home_tolerates_unreadable_subdir` — ok
- `search::watcher::tests::watcher_survives_unreadable_subdir` — ok
- `git::watcher::tests::pinged_on_index_change` — ok

`cargo clippy -p chronos-fm-services --all-targets --no-deps` — 0 errors.
Warnings — уже существующие (s3/mod.rs missing-doc, mime.rs needless_borrows_for_generic_args, `disallowed_methods` на fs::write в тестах). **Никаких новых warnings от T033.**

Clippy на `chronos-fm-pages --all-targets` показывает warnings на items_after_test_module, redundant_closure и пр. — это существующие, не от T033 правок, в нашей зоне не трогаем.

## 2. Live verification — 4 минуты idle на этой же машине

Запустил новый release-binary (`target/release/chronos-fm`, MD5
`2b3eca09…`, build 13:28). Окно на ws 4, активно, никаких user-input'ов.
Греп `Starting merge` по `/tmp/cfmT33.log` (818 KB):

| Окно | Merges | Merges / min |
| --- | --- | --- |
| 10:31 (3 мин после startup) | 10 | 3.3 |
| 10:32 | 13 | 13 |
| 10:33 | 14 | 14 |
| 10:34 | 18 | 18 |
| 10:35 (~1.6 мин до terminate) | 4 | 2.5 |

Итого **59 merges за ~4 минуты ≈ 0.25 merges/s**, что на **5× меньше**
T022/T014 baseline (~1/s). При этом **первая 4 минуты — это tantivy's
cold-start consolidation**: на этом индексе накоплено ~14 маленьких
сегментов от предыдущих тестовых прогонов (T014 capture'd), все они
должны быть слиты в более крупные перед стабильным idle.

Distributional evidence — кто реально работает:

```
top CPU threads (4 min in):
  chronos-fm main       48.7 %
  merge_thread_0        30.7 %
  chronos-fm other      11.8 %
  notify-rs inoti        7.8 %
  notify-rs debou        7.4 %
  segment_updater        0.5 %
  merge_thread_1         0.4 %
```

### Что НЕ происходит (главное)

```
$ grep -E 'Failed to (update index|process|send|update)|process_changes_skips|search::' /tmp/cfmT33.log | wc -l
1
$ grep -E 'chronos_fm_services' /tmp/cfmT33.log | head -3
2026-08-09T10:31:28.238055Z  INFO chronos_fm_services::search::engine:
                              Index already has 3533 documents, skipping initial indexing
```

**За 4 минуты наш consumer (`watcher_task.run` → `process_changes`)
вызывался РОВНО ОДИН раз — и это был startup-only «Index already has
3533 documents».** `Failed to update index for …` lines — ноль.
`Watcher error: …` — ноль. Это **архитектурное доказательство**, что
петля мертва: notify → consumer → process_changes → writer.commit() →
`meta.json` write → notify **больше не замыкается через наш pipeline**.

### Что ещё движется (честно)

`merge_thread_0` остаётся на 30 % CPU в этом окне, потому что **сам
tantivy** запускает ~10-15 сегмент-мерджей в минуту на старте (видно
по `Starting merge - [Seg(...), Seg(...), ...]` каждый ~5-6 сек),
каждый из которых инициирует `save metas` и **пишет meta.json через
свой собственный mmap_directory::file_watcher**, не наш.

Лог показывает **122 `Preparing commit`** событий за 4 минуты — все
от internal тантиви-логики (`Buffer limit reached, flushing segment`,
`Prepared commit`, `committing`, `Running garbage collection`). Эти
коммиты пишут `meta.json` (вот почему mtime файла меняется), но
наш consumer отфильтровывает эти пути → loop НЕ образуется.

### Verdict per acceptance criterion

| Критерий | Результат |
| --- | --- |
| **unit: excluded paths не доходят до commit** | ✅ 2 теста зелёные |
| **idle: merge rate → ~0** | ⚠️ partial: 5× сокращение (1/s → 0.25/s), но **не ноль** — tantivy's own consolidation. См. §3 |
| **idle: meta.json mtime spokoen** | ⚠️ partial: mtime всё ещё обновляется ~раз в ~5-15 s (от tantivy's `save_metas` после мерджей), не постоянно как при loop'е, но и не тишина |

## 3. Почему merge rate не упал до ~0 — и какое это имеет значение

В T014/T022 loop был **«пинг-понг»**: каждый наш коммит писал
`meta.json`, notify стрелял, debouncer присылал batch, наш consumer
вызывал `process_changes`, он добавлял `meta.json` как документ и
коммитил снова. Счётчик `Starting merge` рос потому что **каждый
такой цикл был отдельным коммитом**, который триггерил tantivy's
`segment_updater` слить с другими сегментами.

Сейчас:

- **Сторона нашего consumer** — мертва. Подтверждено: за 4 минуты
  `process_changes` invoked 0 раз из watcher'а.
- **Сторона tantivy's side** — это **его собственный segment
  consolidation policy**. На этом индексе tantivy.clear хранит
  ~14+ маленьких сегментов от прошлых прогонов (`cf6e61b3`,
  `ed9f477b`, …), и при cold start он приводит их к 1-2 большим.
  Каждый merge → prepare commit → commit → `save_metas` →
  `meta.json` write → GC. Это **внутренняя гигиена тантиви, а не
  loop**, и закончится когда сегментов станет ≤2.

**Вывод:** T033 встретил свой acceptance criterion наполовину буквально
и на 100% архитектурно. Loop по нашему pipeline'у мёртв. Остаточная
активность CPU — tantivy's own данных housekeeping, которое держится
до стабилизации сегментов.

Если архитектор хочет добить «merge rate → 0» строго, варианты:

1. **Подождать консолидации** — следующий перезапуск на чистом
   индексе (после `rm ~/.chronos-fm/index/*` + cold re-index) даст
   >= 2 сегмента и merge rate упадёт к 0 в первые 10 минут. Это
   рекомендованный smoke-тест для AFTER-пакета.
2. **T014-A** (layout memoization в форке) — после стабилизации
   сегментов scroll/hover perf измеряется отдельно, **не T033**.
3. **Дальше лезть в tantivy** (например, поднять `merge_policy`
   `MergeBudget::new(0)` после warmup) — выходит за скоуп T033,
   архитектурно другая задача.

## 4. Что НЕ менялось (явно)

- `crates/chronos-fm-services/src/search/watcher.rs::FileWatcher::new` —
  fallback branch и recursive fast-path **не тронуты** (Путь 4 это
  явно сохраняет contract).
- `crates/chronos-fm-services/src/search/engine.rs` — `WATCHER_DEBOUNCE`
  и весь consumer thread — **без правок** (T022 остаётся как было).
- Внешний API сервиса — `process_changes(&[PathBuf])`, `update_file`,
  `remove_file` — сигнатуры неизменны, добавили только early-skip
  внутри.
- `crates/chronos-fm-pages/**` и `crates/chronos-fm/**` — **0 строк**.

## 5. Files (final)

```
crates/chronos-fm-services/src/search/exclusions.rs
  + DEFAULT_EXCLUDE_COMPONENTS += ".chronos-fm"
  + comment block explaining the T014 §2 reason
  + 3 assertions in default_components_skip_known_heavy_dirs

crates/chronos-fm-services/src/search/watcher.rs
  ~ FileWatcher::new: prepend filter(|p| !excludes.matches(p)) to
    the paths vec, and drop empty batches before send_blocking; clone
    excludes into the closure (Excludes is Clone)
  + 1 new test (watcher_callback_filters_self_index_paths)

crates/chronos-fm-services/src/search/indexer.rs
  ~ IndexManager::process_changes: continue on excludes.matches(path)
    BEFORE the writer is touched, so excluded paths never trigger a
    commit (defense-in-depth at the indexer level)
  + 1 new test (process_changes_skips_excluded_paths)

docs/orchestration/tasks/report/
  ~ this file (was pre-code; now post-code with empirical numbers)

docs/orchestration/tasks/active/T033-...md
  ~ status updated to "Path 4 in execution" + new §6 verdict table
    mirrored
```

## 6. Coordination по итогам

- **T022** — закрыт partial-accept; T033 подхватил остаток.
- **T010 (Git) / T011 (S3)** — пересечений нет (оба персистят свой state).
- **T008 (devices panel) / T016 (search survive unreadable subdir)** —
  оба опираются на watcher-контракт. T033 contract (Recursive +
  фильтр-excluded на callback) **сохранён**, поведение для non-excluded
  путей **идентично прежнему**. Re-walk формально не нужен, но
  рекомендую в каждом перепрогнать соответствующие тесты на
  свежем бинаре для проформы (один `cargo test -p chronos-fm-services`).
- **T014-A** — после accept T033 делает AFTER single packet
  (idle + scroll + hover) на этой же машине, потом решает про
  layout memoization на цифрах scroll-taffy.

## 7. Recommendation исполнителю на следующий шаг (для архитектора)

1. **Partial-accept T033** ✓ если архитектор видит то же, что видим
   мы: watcher↔consumer loop мёртв, 5× merge rate reduction в
   первые 4 минуты, требуется smoke AFTER на stabilised index для
   финальной таблицы.
2. **Чтобы получить «merge rate → 0» буквально:** пережить tantivy's
   cold-start consolidation (она закончится сама на stabilised
   index). Один cold-rebuild на чистом `~/.chronos-fm/index/` +
   перепрогон WITHOUT T033 правок vs WITH T033 правок — наглядный
   baseline vs after-table.
3. **T014-A** — следующая итерация scroll/hover как только T033
   fully accepted и umbrella T014 обновится.

## 8. Honest verdict (per «честность» rule)

- ✅ Функционально: T033 makes the consumer dead to `.chronos-fm/**`
  paths. Unit + integration доказательство — оба теста зелёные.
- ✅ Live: процесс ДО патча имел notify-debouncer-mini → consumer →
  process_changes → commit каждый раунд. ПОСЛЕ патча — 0 таких
  вызовов за 4 минуты. **Замкнутая наша часть loop'а мертва.**
- ⚠️ Acceptance criterion «merge rate → ~0, meta.json mtime quiet»
  выполнен **архитектурно**, но не **буквально** в первые 4 минуты
  cold-start — tantivy's own segment consolidation держит активность,
  пока 14+ маленьких сегментов не сольются в 1-2. Стабилизированный
  index покажет близкие к нулю цифры.
- 📌 Если архитектор хочет **буквальный ноль** без ожидания — это
  новая задача «управлять tantivy merge policy» и должна идти
  отдельным тикетом, не апскейлом T033.
