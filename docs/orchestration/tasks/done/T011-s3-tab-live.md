# T011 — S3-таб: браузер объектного хранилища вместо заглушки

> ## ✅ ARCHITECT VERDICT: **CLOSED** (2026-08-09)
>
> Core deliverable met: S3 tab is live (config → credentials → connect →
> bucket/object listing via T021). Milestones A/B accepted with errata;
> listing fixed and live-confirmed on RustFS.
>
> **Residuals (not reopened as T011 — file new tickets if needed):**
> - ~~Mask Secret Access Key~~ **fixed 2026-08-09** (`T011-…-c-bugfixes-report`)
> - ~~Missing «Open Settings →» on NoProfiles~~ **fixed 2026-08-09**
> - Deeper prefix navigation / upload / download / delete / multi-profile
> - Out-of-scope B extras (DnD, multipart, bucket CRUD, …)
> - Optional live smoke of the two bugfixes on RustFS
>
> Reports: `report-log/T011-s3-tab-live-report.md`,
> `report-log/T011-s3-tab-live-b-milestones-report.md`,
> `report-log/T021-s3-bucket-listing-report.md`,
> `report-log/T011-s3-tab-live-c-bugfixes-report.md`.

**Приоритет:** P3 — вкладка-пустышка, нет бэкенда, ниже приоритет чем
Git (нишевее для повседневного использования).
**Статус (2026-08-06, чекпоинт #4):** Milestones **A и B сданы**.
Секция «Что нужно ПЕРЕД кодом» ниже исполнена целиком и оставлена как
история решений — брейншторм проведён, выбраны `aws-sdk-s3` +
`config.toml` для endpoint/region + `oo7` keyring для ключей, spec и
план написаны, код в `bad7522`…`73c3cff`.

**Milestone A принят как REFUTED с эрратой** (см. врезку в
`report/T011-s3-tab-live-report.md`): два заявления о верификации
оказались ложными, а коммит `07bce8b` внёс `oo7` с default features и
сломал сборку всего workspace для трёх последующих тикетов. Исправлено
`0564c6e` + `05a6e5a`.

**✅ Разблокирован (2026-08-09, задним числом).** T021 закрыт и принят
живьём 2026-08-07 (чекпоинт #5): после connect без навигации видно 2
бакета (`chronos-empty`, `chronos-smoke`), внутри бакета —
`data/`/`notes/`/`readme.txt` (31 B, байт-в-байт совпало с засеянным).
Список бакетов теперь запрашивается. Запись «⛔ Заблокирован T021» ниже
провисела устаревшей 2 дня — не была снята в момент закрытия T021,
мой косяк учёта. S3-таб на уровне листинга бакетов рабочий; пункты
«Открыто» ниже (кроме первого) актуальны сами по себе.

**Открыто:**
- **Живой прогон** — стенд (**RustFS**, не MinIO — Apache-2.0 вместо
  AGPL) отработан и подтверждён через T021: connect + листинг бакетов
  и файлов работает. Остаётся проверить: навигация по префиксам внутри
  бакета глубже одного уровня, загрузка/скачивание/удаление объектов,
  переключение между профилями — это T021 не покрывал.
- **Secret Access Key вводится открытым текстом** — маскировки в поле
  нет (замечено в живом прогоне 2026-08-06).
- Кнопка «Open Settings →» из состояния `NoProfiles` (заявлена в
  Milestone A §Task 7) в живом прогоне отсутствует — на карточке только
  два текстовых ряда. Похоже, потеряна при переписывании `s3.rs` в
  B.3/B.4.
- Вне скоупа B: drag-and-drop upload, multipart для больших файлов,
  создание/удаление бакетов, несколько одновременных подключений,
  presigned URL, client-side encryption.

## Что нужно ПЕРЕД кодом (исполнено, оставлено как история)

1. `brainstorming` skill: браузер S3-совместимого хранилища (bucket/key
   листинг как виртуальная директория). Решить: клиент (`aws-sdk-s3`
   тяжёлый vs `rusty-s3`/лёгкие альтернативы), хранение credentials
   (system keyring через `oo7`/`secret-service` — в дереве уже есть
   `oo7` транзитивно, см. Cargo.lock из T003 — переиспользовать, не
   искать новый крейт), scope v1 (read-only browse vs upload/download,
   какие S3-совместимые провайдеры целить — AWS/MinIO/R2/др).
2. Design spec → `docs/superpowers/specs/`.
3. Implementation plan → `docs/superpowers/plans/`.
4. Только после этого — T-тикет на код.

## Зависимости

Независим от остальных активных тикетов.

## Не раздавать как есть

Этот файл — не инструкция для исполнителя-кодера, сначала брейншторм.
