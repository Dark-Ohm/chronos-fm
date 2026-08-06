# T007 — «Open with» / service menu

**Приоритет:** P3 — нужно для полного Dolphin-паритета, не блокер
daily-driver (`xdg-open`-эквивалент дефолтного открытия, если уже есть,
покрывает базовый случай).
**Статус:** ТРЕКИНГ-ТИКЕТ, не начинать код. Нет ни design spec, ни
implementation plan — backlog-пункт #4 из `docs/superpowers/specs/2026
-08-06-removable-media-mount.md` (низ файла), только приоритет одной
строкой, не проработано.

## Что нужно ПЕРЕД кодом

1. `brainstorming` skill: right-click → «Open With» — подменю со
   списком приложений из `.desktop`-реестра, умеющих открыть файл по
   его MIME-типу (freedesktop.org Shared MIME database + `xdg-mime
   query filetype`/`query default`). Решить: парсить `.desktop`-файлы
   самим (`~/.local/share/applications`, `/usr/share/applications`) или
   есть готовый крейт под это в экосистеме (проверить перед тем как
   писать свой парсер — bleeding-edge деп-политика, не изобретать
   велосипед если есть живой крейт). Плюс «Set as default» действие.
2. Design spec → `docs/superpowers/specs/`.
3. Implementation plan → `docs/superpowers/plans/`.
4. Только после этого — T-тикет на код.

## Зависимости

Не зависит от T003–T006. Полностью независимая зона (новый модуль,
скорее всего `chronos-fm-services/src/mime/` или аналог).

## Не раздавать как есть

Этот файл — не инструкция для исполнителя-кодера, сначала брейншторм.
