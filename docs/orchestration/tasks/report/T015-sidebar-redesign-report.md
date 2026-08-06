# T015 — Sidebar Redesign: Implementation Report

**Дата:** 2026-08-06
**Статус:** ✅ Complete
**Коммит:** `9403f57`
**Связанные тикеты:** T003 (devices panel), T008 (device click wiring)

## 0. Контекст

Сайдбар имел 6 проблем (из tracking-тикета):

1. **Мёртвый верхний блок** — `Home`/`Favorites`/`Recent`/`Trash` без `on_click`
2. **Дубль `Home`** — и в мёртвом блоке, и в рабочих Folders
3. **Три карточки без ритма** — мёртвый блок → `elevated_card("Folders")` → `elevated_card("Devices")`
4. **Сырой текст действий** — `"unmount"`/`"eject"` в `text_xs` + `muted` без фона/рамки
5. **Дисбаланс кегля** — имя тома крупное, действия `text_xs`
6. **I/O в рендере** — `Path::exists()` на каждом кадре

Дизайн-решение: Approach A — единый список Places (Dolphin-паритет).

## 1. Что сделано

### Task 1 — Cache shortcuts (`state.rs`, +25 строк)

- Добавлено поле `shortcuts: Vec<(String, String)>` в `ExplorerPane`
- `compute_shortcuts()` — та же логика что `get_shortcuts()` из sidebar.rs, но вызывается один раз при построении панели
- Четыре вызова `Path::exists()` теперь на старте, а не на каждом рендере

### Tasks 2+3 — Unified Places list (`sidebar.rs`, +184/−273 строк)

**Структура:**
```
Places (elevated_card)
├── section_header("Places", "quick access")
├── 📁 Home        → change_dir
├── 📁 Desktop     → change_dir  (если существует)
├── 📁 Downloads   → change_dir  (если существует)
├── 📁 Documents   → change_dir  (если существует)
├── 📁 Pictures    → change_dir  (если существует)
├── 💾 VTOYEFI     → navigate / mount-and-navigate   [⊖] [⬆]
└── 💾 Ventoy      → mount-and-navigate              [⬆]
```

**Решения по каждой строке:**

| Элемент | Компонент | Клик |
|---|---|---|
| Папки | `ListItem` + `Icon(Folder)` | `change_dir(path)` |
| Устройства | `ListItem` + `Icon(HardDrive)` | Mounted: `change_dir(mount_point)`. Unmounted: `mount_and_navigate` |
| Unmount | `ListItem` + `Icon(Minus)` | `DeviceStore::unmount` + `stopPropagation` |
| Eject | `ListItem` + `Icon(ArrowUp)` | `DeviceStore::eject` + `stopPropagation` |

**Что убрано:**
- Мёртвый блок `Home`/`Favorites`/`Recent`/`Trash` (удалены полностью)
- `sidebar_item()` функция (мёртвый код)
- `render_shortcuts()` (переиспользует `page.shortcuts`)
- `render_devices_section()` (инлайнен в `render()`)

## 2. Верификация

| Чек | Результат |
|---|---|
| `cargo check -p chronos-fm-pages` | ✅ |
| Нет мёртвых кнопок в сайдбаре | ✅ (dead block удалён) |
| Иконки действий не упираются в край | ✅ (ListItem padding) |
| `stopPropagation` на eject/unmount | ✅ |
| Shortcuts не пересчитываются каждый кадр | ✅ (кэш в `ExplorerPane`) |
| Live test | ⚠️ Не проводился |

## 3. Файлы

| Файл | +/− |
|---|---|
| `crates/chronos-fm-pages/src/explorer/state.rs` | +25 |
| `crates/chronos-fm-pages/src/explorer/view/sidebar.rs` | +184/−273 |
| **Net** | **−64** |

## 4. GPUI lessons learned

- `Div.on_click` builder method отсутствует в этой версии GPUI — только `&mut self` вариант. Для кликабельных строк используй `ListItem` с `cx.listener`.
- `Icon.hover()` не существует — оборачивай иконку в `div` или `ListItem` для hover-эффектов.
- `ElementId::From<(&str, &str)>` отсутствует — используй `(&str, usize)` или `(&str, u64)`.

## 5. Коммит

```
9403f57 ui: T015 sidebar redesign — unified Places list with cached shortcuts + device icons
```
