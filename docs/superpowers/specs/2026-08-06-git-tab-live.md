# Git Tab v1 — Live Repository Status and Operations

**Дата:** 2026-08-06. **Автор:** Архитектор (брейншторм с пользователем).

## Контекст и цель

`crates/chronos-fm-pages/src/git.rs` — заглушка («📦 Git — Git integration
feature to be implemented»), как были settings/s3/extensions. Settings
оживлён (T009), git трекается как T010. Цель — живой git-статус
текущей директории эксплорера + базовые операции.

Пользовательские решения (брейншторм):
- **Бэкенд: gix (gitoxide)** — чистый Rust, без C-зависимостей; тот же
  выбор, что у Zed и Yazi (главные Rust-проекты этого класса). `git2`
  (libgit2-биндинги) отклонён: тяжёлая C-сборка. Шелл-обёртка
  отклонена: хрупкий парсинг `--porcelain`.
- **Скоуп: полный git-клиент** (status + stage/unstage/commit + ветки +
  diff + push/pull/stash), но **доставляется милестоунами** — один spec
  на всё, implementation plan на Milestone A.
- **Директория: оба режима** — follow за активной вкладкой эксплорера
  (по умолчанию) и ручной pin пути.
- **Diff: unified-панель** с подсветкой через уже существующий syntect
  (Milestone B).
- **Push/pull auth: системные креды** — gix использует system
  git credential-helper + SSH-agent, своих диалогов паролей в v1 нет
  (Milestone C).
- **Обновление: watcher на `.git`** — мгновенный отклик на изменения
  (обобщение watcher-паттерна T009).

## Милестоуны

- **A (первый implementation plan):** панель статуса
  (staged/modified/untracked), stage/unstage, commit с вводом сообщения,
  режимы follow/pin, watcher-обновление, empty-state вне репозитория.
- **B (второй план):** ветки (list/switch/create) + unified diff-панель
  с syntect-подсветкой + открытие файла из статуса.
- **C (третий план):** push/pull (system credential-helper / SSH-agent) +
  stash (create/apply/pop/list) + поверхностный показ ошибок в статус-баре.

Этот документ — полный скоуп; implementation plan заводится на Milestone
A. B и C получают планы после приёмки A (каждая фаза — работающий,
тестируемый результат).

## Архитектура

### Слой сервиса: `chronos-fm-services::git` (новый модуль)

Чистый слой поверх `gix`, без GPUI-зависимостей — юнит-тестируется на
temp-репозиториях.

```rust
/// Списки файлов статуса репозитория + счётчики.
pub struct RepoStatus {
    pub repo_path: PathBuf,          // корень найденного репозитория
    pub branch: Option<String>,      // текущая ветка (None — detached)
    pub staged: Vec<RepoEntry>,      // файлы в index
    pub modified: Vec<RepoEntry>,    // изменены в worktree, не staged
    pub untracked: Vec<RepoEntry>,   // не отслеживаются
}

/// Одна строка статуса: путь относительно корня репо + состояние.
pub struct RepoEntry {
    pub path: String,      // относительный путь, `/`-разделитель
    pub staged: bool,      // есть ли изменения в index
    pub status: char,      // 'M' | 'A' | 'D' | 'R' | '?' и т.п. (gix status)
}
```

Операции Milestone A (все — `Result<(), GitError>`; `GitError` — newtype
обёртка над `gix::Error` + контекст операции):

- `open_repo(dir: &Path) -> Result<RepoHandle, GitError>` — найти ближайший
  репозиторий от `dir` вверх (`gix::discover`). `RepoHandle` держит
  открытый `gix::Repository` + кэш корня.
- `status(handle) -> Result<RepoStatus, GitError>` — собрать три списка
  через `gix` status/index (направление: status → untracked/modified,
  index diff → staged). Сортировка списков: staged по path, остальные по
  path (детерминированный порядок для тестов).
- `stage(handle, paths: &[&str]) -> Result<(), GitError>` — добавить в
  index (`index.add_entry` для существующих/новых, `remove` для удалённых
  — через `git status` классификацию или `add_path`-эквивалент).
- `unstage(handle, paths: &[&str]) -> Result<(), GitError>` — убрать из
  index (reset index для путей).
- `commit(handle, message: &str) -> Result<(), GitError>` — создать коммит
  из staged (автор/коммиттер — из конфига репо; коммит пустого index —
  ошибка с понятным сообщением).

Файл: `crates/chronos-fm-services/src/git/mod.rs` (+ `git/status.rs`,
`git/ops.rs` по мере роста), ре-экспорт из `chronos_fm_services.rs`.

### Страница: `crates/chronos-fm-pages/src/git.rs` (переписывается)

```rust
pub struct GitPage {
    mode: DirMode,                    // Follow | Pinned(PathBuf)
    follow: WeakEntity<ExplorerPage>, // источник пути в Follow-режиме
    handle: Option<RepoHandle>,       // открытый репозиторий
    status: Option<RepoStatus>,       // последний собранный статус
    status_error: Option<String>,     // однострочная ошибка (шапка)
    commit_message: String,           // содержимое commit-инпута
    // Milestone B: selected_diff: Option<(path, DiffText)>; branches: Vec<Branch>
    _watcher: Option<GitWatcher>,     // watcher на .git текущего репо
}

pub enum DirMode {
    Follow,                 // путь = explorer.current_path(cx) при каждом refresh
    Pinned(std::path::PathBuf),
}
```

- `new(explorer: WeakEntity<ExplorerPage>, cx) -> Self` — старт в Follow.
- `refresh(window, cx)` — пересобрать статус: определить целевой путь
  (Follow → `explorer.current_path(cx)`, Pinned → сохранённый), открыть
  репозиторий, собрать статус в `background_executor`, результат —
  `cx.update` + `notify`. Смена репозитория → пересоздать `GitWatcher`.
- `stage/unstage/commit` — те же операции, после успеха — `refresh`.
- `pin(path)` / `unpin()` — переключение режимов (кнопка в шапке:
  «Follow»/«Pinned: …», в Pinned — путь кликабелен → пикер/текстовый ввод).
- Рендер:
  - Шапка: путь репо, ветка (или «detached»), кнопка Refresh,
    `status_error` одной строкой (если есть).
  - Вне репозитория: empty-state «Не git-репозиторий — откройте папку в
    Explorer или закрепите путь» + кнопка Pin.
  - Три секции-карточки (паттерн `elevated_card`/`section_header` T002):
    **Staged** (заголовок + счётчик), **Modified**, **Untracked**.
    Строки: имя файла + кнопка действия (Staged: «Unstage»;
    Modified/Untracked: «Stage»). Одна строка — одна кнопка, без
    multi-select в v1 (multi-select-инфраструктура эксплорера
    не переиспользуется — свой скоуп).
  - Commit-бар: текстовый `Input` (gpui-component) для сообщения +
    кнопка «Commit» (disabled, если Staged пуст или сообщение пустое).
  - Секции с пустыми списками не рендерятся (или рендерятся с
    «нет изменений» — выбрать в плане одно поведение).
- Пока статус пересобирается (фоновый поток): секции остаются со старыми
  данными, в шапке тонкий индикатор «…» (без блокировки UI).

### Wiring: `crates/chronos-fm-pages/src/root.rs`

- `GitPage::new(explorer.clone().downgrade(), cx)` вместо `GitPage::new()`.
- Никакой новой глобальной инфраструктуры: `WeakEntity<ExplorerPage>` +
  `current_path(cx)` (уже публичен, `page.rs:525`).

### Watcher: `chronos-fm-services::git` (или `chronos-fm-core`)

`GitWatcher` — обобщение watcher-паттерна T009 (notify → канал →
foreground-poll через `cx.background_executor().timer`):

- Следит за `.git`-каталогом текущего репозитория (рекурсивно по путям
  `index`, `HEAD`, `refs/`, `packed-refs`, `config`, `objects/` — решить
  в плане: вся `.git` рекурсивно может быть шумной; v1 — подписка на
  ограниченный набор ключевых файлов).
- Класс-мёртвый триггер «что-то изменилось» → канал → GitPage-полл →
  `refresh`. Не пересоздаётся при каждом событии (дебаунс 400ms, как T009).
- Вне репозитория — watcher не создаётся (нет инфраструктуры).
- notify-крейт сверить с `Cargo.lock` (транзитивно уже может быть — как
  было с `toml_edit` в T009); если нет — добавить прямую зависимость.

## Данные и ошибки

- Вне git-репо: empty-state, не ошибка.
- `GitError` → однострочный `status_error` в шапке (цвет danger, паттерн
  T009 `config_status`) + `tracing::error` с полным контекстом.
- Коммит при пустом staged: кнопка disabled; если всё же вызван —
  понятная ошибка «nothing to commit».
- Пути в списках — относительные, `/`-разделитель, без префикса `./`.
- Submodule/rename-состояния: v1 показывает как есть (статус-символ из
  gix), без специальной обработки (задокументировать в плане).
- Конфликты merge (UU-статусы): показываются в Modified со статус-символом
  `U`, действие «Stage» в v1 не разрешает конфликт (это merge-работа).

## Тестирование

- **Сервис-слой** (тесты в `git/mod.rs`): на temp-репо через системный
  `git` CLI в `#[test]` (создание/коммиты/изменения — как fixtures,
  сам тест читает через наш слой):
  - `status_lists_staged_modified_untracked` — init+commit → modify →
    stage один → assert три списка (распределение по спискам + символы).
  - `stage_unstage_round_trip` — stage → unstage → списки вернулись.
  - `commit_creates_commit` — stage → commit → `status` пуст + HEAD
    изменился (проверить `log`/HEAD через gix).
  - `open_repo_finds_ancestor` — открытие репо из подпапки.
  - `open_repo_errors_outside_repo` — вне репо → `GitError`.
  - `commit_with_empty_index_errors`.
- **GitPage:** render-тест без паники (паттерн T009
  `settings_page_renders_without_panicking`); empty-state рендер.
- Работа с реальным репо в UI-тестах не делается (gix в фоне + watcher —
  не детерминированы в unit-окружении) — сервис-слой покрывает логику.

## Что НЕ в скоупе (милестоуны B/C + вне T010)

- Ветки, diff-панель, open-файла — Milestone B.
- push/pull, stash, credential-UI — Milestone C.
- Интерактивный rebase/merge-resolve, blame, история коммитов,
  подписи/подпись коммитов, git-LFS — не в T010 вообще.
- Вкладки S3/Extensions — отдельные тикеты T011/T012.

## Приёмка

Milestone A принимается после: `cargo build --workspace` + `cargo test
--workspace` чисто; живой прогон на реальном репо — открыть репо в
Explorer → вкладка Git показывает статус, stage/unstage/commit
работают и `git status` в терминале подтверждает; pin/follow
переключение работает; watcher подхватывает внешние изменения (второй
терминал `touch`/`git add`). Живой прогон — headless-сессия может не
позволить; тогда приёмка код-ревью архитектора + unit-покрытие.
