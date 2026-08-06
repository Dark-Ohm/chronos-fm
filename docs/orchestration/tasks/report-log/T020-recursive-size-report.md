# T020 — Отчёт: рекурсивный размер папки — обход вынесен из GPUI и покрыт тестами

**Статус:** выполнено (вынос + 2 юнит-теста, поведение зафиксировано).
**Приоритет:** P3 (последний неисполненный пункт T013). Приёмка — архитектор
лично (живой прогон, критерий ниже).
**Корень (исходная проблема):** логика обхода была вшита в
`PropertiesDialog::new(item, cx)` — внутрь `cx.background_spawn`, которому
нужен GPUI-`Context` — и потому была непокрыта тестами (причина, принятая в
отчёте T013).

## Что сделано

### 1. Вынос обхода в свободную функцию (`properties.rs`)

Обход вынесен из замыкания `background_spawn` в GPUI-независимую функцию
(рекомендация отчёта T013, сигнатура — из тикета):

```rust
fn compute_recursive_size(path: &Path) -> SizeStatus
```

Логика перенесена **байт в байт**: `WalkDir::new(path)` без `follow_links`,
`dirs`/`files`/`bytes`/`errors`, `Err(_) => errors += 1` (толерантность к
нечитаемым веткам сохранена), `bytes += e.metadata().map(|m| m.len()).unwrap_or(0)`.
`PropertiesDialog::new` теперь:

```rust
if item.kind == "dir" {
    let status = size_status.clone();
    let path: PathBuf = item.path.clone().into();
    cx.background_spawn(async move {
        *status.lock().unwrap() = SizeStatus::Counting { files: 0, bytes: 0 };
        *status.lock().unwrap() = compute_recursive_size(&path);
    })
    .detach();
}
```

**Осознанное изменение поведения (рецензент зафиксировал, фиксирую и здесь):**
промежуточные тики прогресса `Counting { files, bytes }` каждые 100 файлов
(бывший `if files % 100 == 0`) выпали — при выбранной сигнатуре
`path -> SizeStatus` (без колбэка/наблюдателя) промежуточные состояния из
функции не вернуть. Диалог по-прежнему мгновенно показывает «Counting…»
(ставим `Counting{0,0}` до вызова), затем сразу `Done`. Для больших деревьев
живого счёта в процессе обхода больше нет — это плата за тестируемость,
принята осознанно; при необходимости прогресс вернётся через инжектированный
наблюдатель. Не изменилось: симлинки не считаются ни файлом, ни директорией
(`WalkDir` без `follow_links` даёт `is_symlink()`, не попадающую ни в одну
ветку) — ровно как было до рефакторинга.

### 2. Тест: сумма дерева (`compute_recursive_size_sums_tree`)

`tempfile::TempDir` + файлы известного размера + две вложенные директории:

```
tmp/           a.bin 1000 B, b.bin 500 B
tmp/sub/       c.bin 250 B
tmp/sub/deep/  d.bin 100 B
```

Ассерты: `bytes = 1850`, `files = 4`, `dirs = 3` (корень + sub + deep —
`WalkDir` отдаёт корень первым, как и раньше), `errors = 0`.

### 3. Тест: толерантность к нечитаемой ветке (`compute_recursive_size_tolerates_unreadable_subdir`)

`#[cfg(unix)]`, скип под root и на ФС, где chmod не действует, — тем же
пробником, что в T016 (`chmod 000` на probe-дир → `read_dir` всё ещё ок →
return):

- читаемый файл `a.bin` (100 B);
- поддиректория `locked/` (chmod 000) с `secret.bin` (900 B).

Ассерты: `bytes = 100` (только читаемая часть), `files = 1`, `dirs = 2`
(корень + сама запись `locked` — её stat проходит; не проходит только чтение
содержимого), `errors = 1` — обход **не** прерывается. Пермишены
восстанавливаются перед очисткой `TempDir`.

## Верификация

- `cargo test -p chronos-fm-pages properties` — 6/6, включая оба новых
  (tolerance-тест **реально отработал** на этой машине, не скипнулся).
- `cargo test -p chronos-fm-pages` — **86/86 passed** (84 из T018 + 2 новых).
- `cargo clippy -p chronos-fm-pages --lib` — новых замечаний в
  `properties.rs` нет: единственный флаг на новых строках —
  предсуществующий warn-only `unwrap_used` на `Mutex::lock()`, тот же
  паттерн, что уже был на строках 67/223 до рефакторинга.
- Живой прогон (критерий приёмки из тикета: `Ctrl+I` на папке → размер
  сходится с `du -sb`) — **за архитектором** (headless-сессия, GUI не
  запускается).

## Зона файлов

- `crates/chronos-fm-pages/src/explorer/properties.rs` — свободная
  `compute_recursive_size(path: &Path) -> SizeStatus`; `PropertiesDialog::new`
  зовёт её из фоновой задачи (путь как `PathBuf`); +2 юнит-теста.

Коммит не создавался (по соглашению сессии — только по явному запросу).
