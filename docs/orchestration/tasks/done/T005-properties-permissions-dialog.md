# T005 — Properties / Permissions диалог

**Приоритет:** P2 — третий по критичности гэп до Dolphin-паритета.
**Статус:** ТРЕКИНГ-ТИКЕТ, не начинать код. Нет ни design spec, ни
implementation plan — backlog-пункт #2 из `docs/superpowers/specs/2026
-08-06-removable-media-mount.md` (низ файла), только приоритет одной
строкой, не проработано.

## Что нужно ПЕРЕД кодом

1. `brainstorming` skill: right-click → Properties — модальное окно/
   попап с: размер (рекурсивный для папок — нужен фоновый обход,
   `cx.background_spawn`, с индикатором «считаю…», не блокировать UI),
   владелец/группа (`std::os::unix::fs::MetadataExt`), права в
   человекочитаемом виде (`rwxr-xr-x`) + editable octal, дата
   изменения/создания. Решить UI-контейнер — новое GPUI popup-окно
   (как в ChronOS side_panel popups) или modal внутри текущего окна.
2. Design spec → `docs/superpowers/specs/`.
3. Implementation plan → `docs/superpowers/plans/`.
4. Только после этого — T-тикет на код.

## Зависимости

Использует T002-паттерны (`elevated_card`/`section_header`) для
вёрстки диалога, если решение — modal, а не отдельное окно. Не зависит
от T003/T004.

## Не раздавать как есть

Этот файл — не инструкция для исполнителя-кодера, сначала брейншторм.
