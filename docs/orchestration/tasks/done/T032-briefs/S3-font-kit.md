# Agent S3 — font-kit (zed-font-kit)

**Тип:** реализация, шаг 3 (рендер)  
**Старт:** после `T032-notes/I2-font-kit.md` + merge S1  
**Worktree:** рекомендуется sibling of chronos-ecosystem

## Цель

Снять git `zed-industries/font-kit@94b0f28` по рекомендации I2:

- **A:** `zed-font-kit = "0.14.1-zed"` с crates.io (если rev/API совпадает)
- **B:** path-vendor в `Source/` + PATCHES.md

Точки dep:

- `Source/gpui/Cargo.toml`
- `Source/gpui_wgpu/Cargo.toml`
- macOS cfg blocks если есть дубли git-строк (Linux-only проект — не ломать parse)

## Стены (рендер)

1. **Grim до** изменения (зафиксировать путь скрина + commit HEAD Source).
2. Смена dep + check.
3. **Grim после** — тот же сценарий (тот же app: ChronOS bar/Chronos-FM).
4. Четыре baseline consumers.

```bash
# пример smoke (уточнить по HANDOFF / dev-cli)
# release binary + grim — unit green недостаточно
```

## Не трогать

- wgpu pin
- xim, scap (кроме косвенного lock churn)
- gpui_macos/windows policy

## Выход

- коммит(ы) в Source
- `T032-notes/S3-font-kit.md` + пути grim before/after
- NOTICE если path-vendor

## Приёмка

- [ ] нет git font-kit в gpui/gpui_wgpu manifests
- [ ] 4 baselines green
- [ ] grim before/after attached/paths in note
- [ ] решение A vs B названо
