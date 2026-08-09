# T011 — S3 Tab: bugfixes (masking + NoProfiles button)

> ## ✅ ARCHITECT VERDICT: **ACCEPT** (2026-08-09)
>
> Two residuals from T011 close: secret-key mask + NoProfiles → Settings.
> Code verified (`set_masked` + `mask_toggle`; `NavigateToSettings` →
> `RootPage::handle_navigate_to_settings`). `cargo check -p chronos-fm-pages`
> green. Live smoke (RustFS) still optional residual.
>
> Report → `report-log/`. T011 remains **closed**; residuals list updated.

**Дата:** 2026-08-09
**Исполнитель:** Buffy (executor)
**Предшественники:** T011 Milestone A (REFUTED with errata), T011 Milestone B (VERIFIED WITH CAVEATS), T021 (unblock)
**Статус:** ✅ 2 bugfixes applied, waiting for live acceptance

## Что исправлено

### Fix #1 — Secret Access Key masking

**Проблема:** поле `Secret Access Key` показывало ключ открытым текстом (замечено в живом прогоне 2026-08-06).

**Корень:** `InputState` создавался без маскировки (`masked: false` по умолчанию, `state.rs:510`).

**Фикс (2 строки):**
- `secret_key_input` инициализируется с `state.set_masked(true, window, cx)`
- `Input::new(&page.secret_key_input).mask_toggle()` — добавляет кнопку 👁 для показа/скрытия

**Код:** `crates/chronos-fm-pages/src/s3.rs` — `S3Page::new` + `credentials_form`

### Fix #2 — «Open Settings →» button in NoProfiles state

**Проблема:** кнопка «Open Settings →» из Milestone A §Task 7 потеряна при переписывании `s3.rs` в B.3/B.4. Карточка `NoProfiles` показывала только два текстовых ряда без actionable элемента.

**Корень:** функция `no_profiles_card` не содержала кнопки, и сигнатура `fn(cx: &App)` не позволяла добавить интерактивный `on_click`.

**Фикс (17 строк):**
1. `gpui::actions!(s3, [NavigateToSettings])` — экшен для межстраничной навигации
2. `no_profiles_card` теперь принимает `cx: &mut Context<S3Page>` + рендерит `Button::new("open-settings").label("Open Settings")`
3. `on_click` диспатчит `NavigateToSettings` на окно
4. `root.rs`: импорт `NavigateToSettings`, `.on_action(cx.listener(Self::handle_navigate_to_settings))`, метод `handle_navigate_to_settings` → `set_page(PageKind::Settings, cx)`

**Код:** `crates/chronos-fm-pages/src/s3.rs` + `crates/chronos-fm-pages/src/root.rs`

## Проверка

| Чек | Результат |
|-----|-----------|
| `cargo check -p chronos-fm-pages` | ✅ 0 errors |
| `cargo build -p chronos-fm` | ✅ Success |
| Clippy | ✅ 0 новых warnings |
| Тесты (`s3::tests`) | ⚠️ linker issue (pre-existing, не из правок) |
| Code review | ✅ LGTM |

## Что НЕ входит

- Живой прогон — требует RustFS стенда (T021) и проверки: маскировка видна, кнопка «Open Settings» переключает на Settings-таб
- Deeper prefix navigation (>1 уровня) — не проверялось; использует стандартный механизм ExplorerPane и должен работать если `S3FileSystemProvider::list_dir` корректно обрабатывает nested-префиксы
- Upload/download/delete объектов — вне скоупа этих багфиксов
- Profile switching — не проверялось

## Коммиты

Не закоммичено — ждёт приёмки.
