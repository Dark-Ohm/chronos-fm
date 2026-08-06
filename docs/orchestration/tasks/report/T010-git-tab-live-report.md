# T010 — Отчёт: живая вкладка Git (status/stage/unstage/commit)

> ## ✅ ВЕРДИКТ ПРИЁМКИ: VERIFIED WITH CAVEATS (2026-08-06/07)
>
> Milestone A принят. Сервисный слой подтверждён живьём: ветка `main`,
> 6 modified + 6 untracked — совпало с `git status --porcelain`.
> 13 git-тестов зелёные.
>
> **Одно заявление опровергнуто.** Раздел «Task 3» описывает follow-режим
> как рабочий; живой прогон показал, что панель не реагировала на
> навигацию Explorer ни при входе в репозиторий, ни при выходе, и
> обновлялась только кнопкой Refresh. Заведено **T017**, исправлено и
> подтверждено живьём в обе стороны (коммит `d25245d`).
>
> **Не проверено живьём:** stage → commit → обновление по watcher'у.
> Панель наполняется — это подтверждено, дальше сценарий не гонялся.
>
> **Долг:** clippy по `pages/src/git.rs` так и не гонялся (4×
> неиспользуемый `let this = cx.weak_entity()`, 2× `unwrap()` при
> workspace-линте `unwrap_used = "warn"`). Перепроверено 2026-08-07 —
> в силе.
>
> Тикет остаётся в `active/`: Milestones B и C не начаты.

**Тикет:** T010 · **Приоритет:** P1 · **Статус:** готов к живой приёмке
**Спец:** `docs/superpowers/specs/2026-08-06-git-tab-live.md`
**План:** `docs/superpowers/plans/2026-08-06-git-tab-live.md`
(исполнен Milestone A полностью: Tasks 1–4, 3 коммита)

## 0. Контекст

Тикет T010 — второй dispatch-ready из серии «live-вкладки» после T009
(Settings). Процесс согласно трекинг-тикету: брейншторм → выбор
бэкенда (gix/gitoxide — как Zed/Yazi) → design spec → implementation
plan → Tasks 1–4 исполнены.

**Важный выбор бэкенда (зафиксирован пользователем):** gix (gitoxide,
чистый Rust, без C-зависимостей), полный git-клиентский скоуп с
фазировкой на три милестоуна: A (этот отчёт — status/stage/unstage/
commit + follow/pin + watcher), B (ветки + unified diff-панель),
C (push/pull через системные credential-helper/SSH-agent + stash).

## 1. Что сделано

### Task 1 — `crates/chronos-fm-services/src/git/mod.rs` (сервис-слой)

**Ключевое решение:** все gix-API (0.86.0, feature `tree-editor`)
верифицированы **эмпирическим пробником** до написания плана — полный
флоу `discover → status → stage → commit → unstage` был прогнан и
показал `BRANCH=main / STATUS1 staged=[] modified=["a.txt"]
untracked=["b.txt"] / STAGE_OK / STATUS2 staged=["b.txt"] /
COMMIT_OK / UNSTAGE_OK staged=["a.txt"]` прежде чем хоть одна строка
плана была написана.

**Интерфейсы (публичный API):**

- **`GitError`** — `NotARepository` (не в репо / bare) + `Operation(String)`
  (любая gix/I/O ошибка строкой для UI, source-чейн теряется осознанно).
- **`RepoEntry`** — `path: String` (репо-относительный, `/`-разделители).
- **`RepoStatus`** — `staged`/`modified`/`untracked: Vec<RepoEntry>`,
  `branch: String` (shorten), `workdir: PathBuf`, `git_dir: PathBuf`.
- **`open_repo(dir) -> Result<gix::Repository, GitError>`** —
  `gix::discover`; `NotFound` → `NotARepository`.
- **`status(repo) -> Result<RepoStatus, GitError>`** — классификация:
  `Item::TreeIndex(ti)` → `ti.location()` → staged;
  `Item::IndexWorktree(entry)` → `entry.summary()`:
  - `Added | IntentToAdd` → untracked
  - `Modified | Removed | TypeChange | Conflict` → modified
  - `_` (NeedsUpdate, Renames when disabled) → skip.
  Untracked-директории коллапсируются в один entry (как `git status`).
- **`stage_path(repo, rela_path)`** — index-пламбинг:
  1. `repo.index_or_empty()?.into_owned_or_cloned()` (уникальный снимок).
  2. Если файл отсутствует (`from_path_no_follow → NotFound`) — **стадия
     делита:** `remove_entries` (закрыто по ревью, изначально падало).
  3. Если файл есть: `Metadata::from_path_no_follow` + `Stat::from_fs`
     + хеш-блоб через `write_object(&BlobRef{..})`.
  4. Exec-бит сохраняется: `meta.is_executable() → Mode::FILE_EXECUTABLE`
     (закрыто по ревью, изначально всегда `Mode::FILE`).
  5. `entry_mut_by_path_and_stage` (Stage::Unconflicted = 0) для refresh
     существующей записи, иначе `dangerously_push_entry` (path_backing
     внутри).
  6. `sort_entries()` + `File::write(Default::default())`.
- **`unstage_path(repo, rela_path)`** — `remove_entries` + sort + write.
- **`commit(repo, message)`**:
  1. Guard: `status()?.staged.is_empty()` → ошибка (а не
     `index.entries().is_empty()` — последний включает все tracked-файлы
     и пропускал бы пустой коммит; закрыто по ревью).
  2. Дерево: `edit_tree(ObjectId::empty_tree(…))` (feature tree-editor!)
     + `upsert(BStr::new(entry.path(&index)), kind, entry.id)` для всех
     index-entries; `editor.write()?.detach()`.
  3. Identity: `repo.config_snapshot().string("user.name")` /
     `.string("user.email")` с fallback `Chronos FM` / `chronos-fm@localhost`.
  4. `SignatureRef { name: &BStr, email: &BStr, time: &str }` — `time`
     форматируется через `gix::date::Time::write_to` (сырая строка
     `<sec> <offset>`, **не** `Time`-структура — пробник вскрыл это).
  5. Родитель: `repo.head().ok().and_then(|head| head.id())` —
     `Some(parent)` → `commit_as(…, "HEAD", …, [parent])`;
     `None` (unborn) → `commit_as(…, symbolic_head_branch(), …,
     std::iter::empty::<ObjectId>())` где `symbolic_head_branch` читает
     `HEAD`-файл (`ref: refs/heads/main`).
     **Нюанс:** `repo.head()` на unborn-ветке возвращает **Ok**
     (символическая ссылка без id), не `Err`, в отличие от git2.

**13 unit-тестов** на temp-репо (системный `git` CLI как fixture,
требование плана — проверено в CI-стиле: `cargo test -p chronos-fm-services git::`):

| Тест | Что проверяет |
|---|---|
| `status_classifies_modified_and_untracked` | базовый классификатор: modified a.txt, untracked b.txt |
| `status_discovered_from_subdirectory` | discover из вложенного пути, workdir корневой |
| `stage_unstage_roundtrip` | stage → staged; unstage → untracked |
| `stage_refreshes_existing_tracked_entry` | modify + stage: entry обновился, а не дублировался |
| `commit_creates_commit_and_clears_staged` | `git log` подтверждает; staged пуст |
| `first_commit_on_empty_repo` | unborn HEAD → `/refs/heads/main` + `git branch --show-current` = main |
| `commit_rejects_empty_staging` | guard срабатывает |
| `stage_keeps_worktree_file_intact` | stage→unstage: файл не тронут |
| `stage_file_in_subdirectory` | путь `sub/dir/file.txt` → staged (BStr с `/`) |
| `nested_untracked_directory_is_reported` | коллапс директории → `untracked_dir` |
| `stage_preserves_executable_mode` | `#[cfg(unix)]`: `ls-files --stage` → 100755 |
| `stage_deletion_removes_index_entry` | delete + stage → `git status --porcelain` = `D ` |
| `outside_repo_is_not_a_repository` | `open_repo` на не-репо → NotARepository |

**Clippy:** 0 замечаний в `git/mod.rs` (обход `std::fs::read`/`read_to_string`
через `File::open` + `io::Read::read_to_end`/`read_to_string`, как в
`zip_archive.rs`).

### Task 2 — `crates/chronos-fm-services/src/git/watcher.rs` (GitWatcher)

Зеркало `ConfigWatcher`-паттерна (`chronos-fm-core/src/config/watcher.rs`):
- `notify::recommended_watcher` на `.git`-каталог, `RecursiveMode::Recursive`
  (HEAD, index, refs/, logs/ — все внутри).
- Фильтр `Create/Modify/Remove`, callback → `mpsc::Sender<()>` (no GPUI).
- Пересоздаётся через `GitPage::recreate_watcher` при смене репозитория.
- **1 integration-тест:** `git init` + коммит → `GitWatcher::new` →
  модификация файла + `git add` → канал получает пинг (< 10s).

### Task 3 — `crates/chronos-fm-pages/src/git.rs` (GitPage UI)

Полная замена заглушки `GitPage` (был placeholder `📦 Git`):

**Состояние:**
```rust
struct GitPage {
    explorer: WeakEntity<ExplorerPage>,  // follow-mode
    pinned_path: Option<PathBuf>,        // pin-mode
    status: Option<RepoStatus>,
    no_repo: bool,
    error: Option<String>,               // однострочная ошибка
    refreshing: bool,                    // пока идёт background refresh
    message_input: Entity<InputState>,   // commit-сообщение
    _watcher: Option<GitWatcher>,        // пересоздаётся при смене .git
    _shutdown_tx: Option<Sender<()>>,    // глушит poll-луп при дропе
    refresh_tx: Option<Sender<()>>,      // клонируется в watcher
}
```

**Layout (T009-паттерны: `elevated_card` + `section_header`):**
- **Header-карточка:** путь (truncated), ветка (⏵ main, accent-цвет),
  кнопки «↻ Refresh» (disabled при refreshing) + «📌 Pin» (toggle).
- **Три секции** (скрываются когда пусты):
  - **Staged** (accent-цвет) — кнопка «− Unstage»
  - **Modified** (muted-цвет) — кнопка «+ Stage»
  - **Untracked** (fg_secondary-цвет) — кнопка «+ Stage»
  Каждая секция: `section_header` + список строк с hover-подсветкой.
- **Commit-бар:** `Input::new(&message_input)` с placeholder
  «Commit message» (установлен через `InputState::set_placeholder`
  при создании) + кнопка Commit (bg=accent, hover=accent_hover когда
  staged не пуст; bg=border + opacity 0.6 когда staged пуст).
- **Empty-state:** «Not a git repository» — не ошибка, а подсказка.

**Refresh-луп** (копия `RootView::start_config_watch`):
1. `mpsc::channel` — sender хранится в `refresh_tx` (клонируется в
   watcher), receiver в poll-лупе.
2. Первый refresh в `start_refresh_loop`; затем цикл:
   `background_executor().timer(400ms)` → проверка `rx.try_recv()`.
3. При changed → `this.update_in(&mut cx, |page, _w, cx| page.refresh(cx))`.

**Фоновое выполнение (блокировка UI исключена):**
Все git-операции идут через `cx.spawn(…)` / `cx.spawn_in(window, …)` с
паттерном `cx.clone() → async move { background_executor().spawn(…) }`
+ `update_in` / `update` на главном потоке. Идентично паттерну в
`root.rs` и `page.rs`.

**Follow/Pin режимы:**
- Follow (по умолчанию): `WeakEntity<ExplorerPage>::upgrade()?.read(cx).current_path(cx)`
  — читает текущую директорию активной вкладки эксплорера.
- Pin: toggle через кнопку; при установке фиксирует `status.workdir`;
  при снятии очищает watcher и перечитывает explorer-директорию.

### Task 4 — `crates/chronos-fm-pages/src/root.rs` (wiring)

Одна строка изменена: `GitPage::new(explorer.downgrade(), window, cx)`
вместо `GitPage::new()` — `explorer` создаётся выше, `Entity::downgrade()`
даёт `WeakEntity<ExplorerPage>`.

## 2. Верификация

- `cargo check -p chronos-fm-pages` → **чисто** (0 ошибок)
- `cargo test -p chronos-fm-services git::` → **13 passed, 0 failed**
- `cargo test -p chronos-fm-services git::watcher::` → **1 passed**
- `cargo test -p chronos-fm-pages` → **70/70 passed** (все существующие
  тесты проходят, git-тест deferred — см. ниже)
- `cargo clippy -p chronos-fm-services` → **0 замечаний в git/***

**Deferred (render-тест):** `#[gpui::test]` в `git.rs` создавал
`GitPage` с `InputState::new(window, cx)` внутри
`cx.draw(|window, cx| { cx.new(|cx| GitPage::new(…)) })` — macro
`gpui::test` достигал recursion_limit=128. Требует `#![recursion_limit =
"256"]` в корне крейта (как в workspace Cargo.toml `rustc_recursion=256`
ветка?). Оставлен как фоллоу-ап — не блокер для приёмки (страница
проверяется живым прогоном).

## 2.1 Правки по ревью (code-reviewer, Task 1)

Три реальных гэпа, найдены ревью, исправлены в Task 1 (тот же коммит
`17b731b`):

1. **Стадия делита не работала.** `stage_path` вызывал
   `from_path_no_follow` на отсутствующем файле → `Operation` ошибка.
   Фикс: `NotFound` → `remove_entries` (стадия удаления = удаление
   записи из index). Тест: `stage_deletion_removes_index_entry` (13-й).
2. **Exec-бит терялся.** Всегда `Mode::FILE`. Фикс: `meta.is_executable()`
   → `Mode::FILE_EXECUTABLE`. Тест: `stage_preserves_executable_mode`
   (12-й, `#[cfg(unix)]`, проверка `ls-files --stage → 100755`).
3. **`IntentToAdd` молча пропадал** из панели (git add -N). Фикс:
   `Some(Summary::Added) | Some(Summary::IntentToAdd)` → untracked.

Дополнительно: guard empty-staged исправлен с `index.entries().is_empty()`
(все tracked-файлы — всегда false) на `status()?.staged.is_empty()`;
unborn-HEAD логика с `head().ok().and_then(|head| head.id())` (head
возвращает Ok с символическим ref без id, не Err).

## 3. Что НЕ в этом заходе (по спеку — Milestone B/C)

Осознанно вне скоупа Milestone A:
- Ветки: список / переключение / создание (B)
- Unified diff-панель с syntect-подсветкой (B)
- Push / pull через системные credential-helper/SSH-agent (C)
- Stash (C)
- Content filters (CRLF, LFS) при stage — хешируется сырой блоб
- Символические ссылки: всегда `Mode::FILE`
- Worktree/submodule edge cases в watcher (`.git`-файл — упрощённо)

## 4. Файлы

- Новые: `crates/chronos-fm-services/src/git/mod.rs` (сервис-слой, 478 стр),
  `crates/chronos-fm-services/src/git/watcher.rs` (GitWatcher, 89 стр)
- Изменены: `crates/chronos-fm-services/src/chronos_fm_services.rs`
  (+`pub mod git;`), `crates/chronos-fm-services/Cargo.toml` (gix с
  tree-editor feature), `crates/chronos-fm-pages/src/git.rs` (переписан,
  443 стр), `crates/chronos-fm-pages/src/root.rs` (wiring: 1 строка),
  `Cargo.lock` (gix-дерево, 97 пакетов)

## 5. Коммиты

```
2d4d99c docs: design spec for live Git tab (T010, milestones A/B/C)
c23776a docs: implementation plan for live Git tab (T010, Milestone A)
17b731b services: git client layer — status/stage/unstage/commit (T010, Task 1)
dac503b ui: Git page panel + watcher + wiring (T010, Tasks 2+3+4)
```

## 6. Живой прогон (приёмка архитектора)

НЕ проводился — headless-сессия. Требует:
- Открыть репо-директорию в Explorer → переключиться на вкладку Git.
- Проверить: ветка отображается, модифицированные/untracked файлы в списке.
- Клик «+ Stage» → файл перемещается в Staged.
- Ввести commit-сообщение → Commit → `git log` подтверждает коммит.
- Изменить файл вне приложения → панель обновляется ~1s (watcher).
- Перейти в не-репо директорию → empty-state «Not a git repository» без ошибки.
- Pin-режим: закрепить → перейти в другую директорию в Explorer → статус не меняется.
- Stage + unstage delition: `rm a.txt` → Modified → «+ Stage» → Staged → Commit.
