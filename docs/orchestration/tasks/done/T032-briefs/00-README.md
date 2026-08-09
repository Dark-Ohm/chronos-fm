# T032 — индекс брифов (один файл = один агент)

**Тикет:** `../T032-render-input-forks-vendoring.md`  
**Каталог заметок агентов:** `../T032-notes/`  
**Итоговый отчёт:** `../../report/T032-render-input-forks-vendoring-report.md`

## Раздать сейчас

| Файл | Роль | Параллельно |
|---|---|---|
| `S1-reqwest_client-vendor.md` | дописать вендор (WIP на диске) | **один** writer на main Source |
| `I1-scap-inventory.md` | разведка scap | да, read-only |
| `I2-font-kit-inventory.md` | разведка font-kit | да, read-only |
| `I3-xim-inventory.md` | разведка xim-rs | да, read-only |
| `I4-wgpu-inventory.md` | разведка wgpu (без кода) | да, read-only |
| `I0-reqwest_client-inventory.md` | разведка reqwest (опц.) | да; почти закрыт WIP S1 |

## Потом

| Файл | Когда |
|---|---|
| `S2-scap.md` | после I1; merge после S1 |
| `S3-font-kit.md` | после I2 + S1; grim |
| `S4-xim.md` | после I3 + S1; grim/X11 |
| `S5-wgpu-blocked.md` | **не назначать** без явного запроса |
| `R-report.md` | когда закрыты сделанные S* |

## Промпт агенту

```
Тикет T032. Открой только свой бриф и тикет:
Chronos-FM/docs/orchestration/tasks/active/T032-briefs/<THIS_FILE>
Chronos-FM/docs/orchestration/tasks/active/T032-render-input-forks-vendoring.md

Скоуп — только бриф. Source/ — fork. Worktree = sibling of
chronos-ecosystem, не /tmp. Не трогай wgpu (кроме I4 read-only).
```

Устаревший монолит: `../T032-agent-briefs.md` → смотри этот каталог.
