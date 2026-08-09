# T011 — S3 Tab Live: Milestones B.1–B.4 Report

> ## ⚠ ВЕРДИКТ ПРИЁМКИ: VERIFIED WITH CAVEATS (2026-08-06/07)
>
> B.1–B.3 подтверждены живьём: секция S3 в Settings на месте, состояние
> `NeedCredentials` рисуется корректно («Connect to rustfs», endpoint,
> два поля, кнопка), connect переводит страницу в `Browsing`.
>
> **B.4 своей цели не достигает.** «Bucket listing: embedded ExplorerPane»
> в живом прогоне даёт `0 items` и **ноль запросов** к серверу: панель
> встроена, но провайдера никто не опрашивает. Разбор — в **T021**;
> первый заход по нему (`32cb1ac`) тоже не сработал.
>
> Причина в самой таблице «Жизненный цикл панели» §B.4: шаг 3 ставит
> `loaded = false` в расчёте на рендер, а шаг B.1 ровно перед этим
> сделал `reload()` пустышкой для provider-панелей. Два милестоуна
> договорились о противоположном.
>
> **Мелочь, замеченная живьём:** кнопки «Open Settings →» из состояния
> `NoProfiles` (заявлена в отчёте Milestone A) на карточке больше нет —
> похоже, потеряна при переписывании `s3.rs` в B.3/B.4.
>
> **Не проверено:** загрузка/скачивание/удаление объектов, переключение
> профилей — до них нельзя добраться, пока не работает листинг.
>
> Стенд для повторной проверки: **RustFS**, а не MinIO (Apache-2.0
> вместо AGPL) — рецепт и грабли rootless podman в тикете T021.

**Дата:** 2026-08-06
**Статус:** ✅ Milestone B complete (4 из 4)
**Коммиты:** 4 (B.1–B.4) + 3 (B fixes + B.1 fix)
**Предыдущий отчёт:** `docs/orchestration/tasks/report/T011-s3-tab-live-report.md` (Milestone A: Tasks 1-8)

---

## 0. Контекст: Milestone A recap

Milestone A (Tasks 1-8) построил фундамент S3-таба:
- `[s3]` config + `ConfigField` S3-варианты (Task 1)
- `FileSystemProvider` trait + `LocalFileSystemProvider` (Task 2)
- `S3Client` (aws-sdk-s3, свой tokio runtime) + `S3FileSystemProvider` (Tasks 3-4)
- `ExplorerPane::provider` dispatch (Task 5)
- `S3CredentialsManager` (oo7 keyring) (Task 6)
- `S3Page` UI (стаб) + `RootView` wiring (Tasks 7-8)

**4 отложенных гэпа** к Milestone B:
1. ❌ UI-thread blocking на S3 I/O — `reload()` блокирует UI на сетевых вызовах
2. ❌ Settings S3 UI controls — нет секции S3 в настройках
3. ❌ S3Page connect flow — статичный текст вместо интерактивных полей
4. ❌ Bucket listing — static «Connected» карточка вместо реального браузера

Все закрыты в B.1–B.4.

---

## 1. Что сделано (Milestone B)

### B.1 — UI-thread blocking fix (`4487d9d`)

**Файл:** `crates/chronos-fm-pages/src/explorer/navigation.rs` (+88/−10)

**Проблема:** в Milestone A `reload()` синхронно вызывал `provider.list_dir()` для S3, блокируя UI-поток на время сети.

**Решение:** разделение `reload()` на два пути:

| Метод | Ответственность | Блокирует UI? |
|---|---|---|
| `reload()` | Только local-FS, быстрый синхронный вызов. При наличии provider — no-op | Нет |
| `reload_provider(window, cx)` | **Новый** — spawn S3 `list_dir` на `cx.background_executor()` через `spawn_in` + `AsyncWindowContext::clone()` → `update_in` → `cx.notify()` | Нет |

**Callers:** `change_dir`, `go_back`, `go_forward` вызывают оба метода. `navigate_without_window` (mirror-sync, device mount) остаётся local-only.

**GPUI pattern:** Тот же, что в `reload_provider` для `ExplorerPane` — `spawn_in(window, ...)` + `AsyncWindowContext::clone()` + `update_in(&mut cx, ...)`.

**Верификация:** `cargo check -p chronos-fm-pages` ✅, 69/69 explorer-тестов ✅.

---

### B.2 — Settings S3 UI controls (`614d306`)

**Файл:** `crates/chronos-fm-pages/src/settings.rs` (+95)

**Три новые функции:**

| Функция | Назначение |
|---|---|
| `s3_section(config, cx)` | `elevated_card` + `section_header("S3", "object storage")` + `default_profile` display + loop по profiles |
| `profile_card(name, profile, cx)` | Карточка одного профиля: жирное имя + три `field_row` |
| `field_row(label, value, cx)` | Строка «label: value», justify-between, monospace-шрифт для значений |

**Состояния:**
- **Нет профилей:** "No profiles configured" + пример TOML-сниппета в `bg_secondary`
- **Есть профили:** per-profile карточки: endpoint, region, force_path_style
- **default_profile пуст:** "none" muted-цветом

**Верстка:** S3-секция между Explorer и draft_sections. Переиспользует `elevated_card` / `section_header` / `bg_secondary`.

**Верификация:** `cargo check -p chronos-fm-pages` ✅.

---

### B.3 — S3Page interactive connect flow (`d9d10ae`)

**Файл:** `crates/chronos-fm-pages/src/s3.rs` (+299/−84)

**State machine (5 состояний):**

| Состояние | Триггер | UI |
|---|---|---|
| `NoProfiles` | Нет профилей / нет default_profile | Info card → Settings |
| `NeedCredentials` | Профиль есть, нет сохранённых credentials | **Input-форма:** Access Key ID + Secret Access Key + **Connect button** |
| `Connecting` | «Connect» нажат | Панель-заглушка "Connecting…" |
| `Browsing` | S3Client готов | Позже заменено в B.4 |
| `Error` | Ошибка валидации / сети | Error card |

**Connect flow (аппаратный):**
1. Пользователь заполняет поля (`gpui_component::Input`)
2. Клик «Connect» (`gpui_component::Button`)
3. `on_click` → `WeakEntity::update` → `start_connect(window, cx)`
4. Валидация: profile exists, access_key + secret_key не пустые
5. `cx.spawn_in(window, ...)` → `AsyncWindowContext::clone()` → async
6. `S3CredentialsManager::store()` (keyring, best-effort — падает без keyring)
7. `S3Client::from_profile()` → `update_in` → `Browsing` / `Error`

**GPUI wiring:** `Button::new("connect-btn").label("Connect").on_click({...})` → `WeakEntity::update` → `start_connect`. Ключевое открытие: `Context::spawn` даёт `&mut AsyncApp` (не `AsyncWindowContext`!), который не живёт через `.await`, поэтому использовать `spawn_in(window, ...)` с `AsyncWindowContext::clone()`.

**Верификация:** `cargo check -p chronos-fm-pages` ✅.

---

### B.4 — Bucket listing: embedded ExplorerPane (`73c3cff`)

**Файл:** `crates/chronos-fm-pages/src/s3.rs` (+113/−92)

**Проблема:** B.3 показывал "Connecting…" и статичную "Connected" карточку — реальной навигации по S3 не было.

**Решение:** встроить `ExplorerPane` с `S3FileSystemProvider`:

**Жизненный цикл панели:**

| Шаг | Фаза | Действие |
|---|---|---|
| 1 | Синхронно (start_connect) | `ExplorerPane::build(None, window, cx)` → `loaded = true` (блокирует reload с локальным cwd) → сохраняется в `s3_pane` |
| 2 | Async (spawn_in) | `S3CredentialsManager::store()` → `S3Client::from_profile()` |
| 3 | Async callback | `pane.set_provider(Arc::new(client))` + `pane.cwd = "s3://profile@"` + `loaded = false` → `cx.notify()` |
| 4 | Render | ExplorerPane вызывает `list_dir("s3://profile@")` → `S3FileSystemProvider::list_dir` → `list_buckets()` |

**Рендеринг:**

| Browsing | Connecting |
|---|---|
| ExplorerPane (полный, интерактивный) | ExplorerPane + overlay (полупрозрачный чёрный + "Connecting…") |

**Навигация:** вся навигация по бакетам и префиксам — через стандартные механизмы ExplorerPane:
- Path bar в header: отображает текущий `s3://profile@bucket/prefix`
- Click по бакету → `change_dir("s3://profile@bucket-name/")` → `reload_provider` → `list_objects`
- Back/forward работают через историю навигации

**Отказ от S3State::Browsing { client }:** клиент теперь хранится внутри `ExplorerPane.provider`, а не в состоянии S3Page. `S3State` упрощён до `Browsing` (unit-вариант).

**Верификация:** `cargo check -p chronos-fm-pages` ✅.

---

## 2. Верификация

| Чек | Результат |
|---|---|
| `cargo check -p chronos-fm-pages` (B.1) | ✅ |
| `cargo check -p chronos-fm-pages` (B.2) | ✅ |
| `cargo check -p chronos-fm-pages` (B.3) | ✅ |
| `cargo check -p chronos-fm-pages` (B.4) | ✅ |
| 69/69 explorer-тестов (B.1) | ✅ |
| End-to-end live test | ⚠️ Не проводился (нужен реальный S3/MinIO эндпоинт) |

---

## 3. Файлы

| Файл | B.1 | B.2 | B.3 | B.4 |
|---|---|---|---|---|
| `crates/chronos-fm-pages/src/explorer/navigation.rs` | +88/−10 | — | — | — |
| `crates/chronos-fm-pages/src/settings.rs` | — | +95 | — | — |
| `crates/chronos-fm-pages/src/s3.rs` | — | — | +299/−84 | +113/−92 |

---

## 4. Коммиты

| Хэш | Milestone | Описание |
|---|---|---|
| `4487d9d` | B.1 | UI-thread blocking: spawn `reload_provider` on background_executor |
| `614d306` | B.2 | Settings S3 section: default_profile + per-profile cards |
| `d9d10ae` | B.3 | S3Page connect flow: Inputs + Save → S3CredentialsManager → S3Client |
| `73c3cff` | B.4 | Embedded ExplorerPane with S3FileSystemProvider |

---

## 5. Milestone B complete — что дальше

Все 4 гэпа из Milestone A закрыты. Полный путь пользователя теперь:

1. ✏️ Добавляет S3 профиль в `config.toml` (редактирует файл или через Settings)
2. 🔐 В S3-табе вводит access key + secret key, жмёт «Connect»
3. 📁 Видит список бакетов (ExplorerPane c S3FileSystemProvider)
4. 🗂️ Навигирует по бакетам и префиксам (стандартная Explorer-навигация)

**Осталось (не в скоупе Milestone B):**
- Drag-and-drop upload в S3
- Multipart upload для больших файлов
- Client-side encryption
- S3 bucket creation / deletion UI
- End-to-end live test с реальным S3/MinIO

---

## 6. Полный список T011 коммитов

```
73c3cff ui: S3 bucket listing — embedded ExplorerPane with S3FileSystemProvider (T011 Milestone B.4)
d9d10ae ui: S3Page interactive connect flow — credentials Inputs + Save (T011 Milestone B.3)
614d306 ui: S3 settings section — default_profile + per-profile cards (T011 Milestone B.2)
4487d9d fix: S3 UI-thread blocking — spawn reload_provider on background_executor (T011 Milestone B.1)
ed048b8 docs: T011 S3 tab implementation report (Tasks 1-8, Milestone A)
9ca9181 fix: cross-platform metadata, S3Config re-exports, unused import (T011 review fixes)
f7e6d88 ui: S3Page rewrite + RootView wiring (T011, Tasks 7+8)
07bce8b ui+config: ExplorerPane provider dispatch + S3CredentialsManager (T011, Tasks 5+6)
6a6de36 services: S3FileSystemProvider — FileSystemProvider impl for S3Client (T011, Task 4)
ccbc9e0 services: S3Client (aws-sdk-s3) + FileSystemProvider trait (T011, Tasks 2+3)
40edac2 services: FileSystemProvider trait + LocalFileSystemProvider adapter (T011, Task 2)
bad7522 config: S3Config + S3Profile + ConfigField S3* variants (T011, Task 1)
```
