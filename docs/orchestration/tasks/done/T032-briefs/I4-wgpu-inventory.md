# Agent I4 — inventory: wgpu (только разведка)

**Тип:** read-only разведка  
**Параллельно:** да  
**Имплементация:** **ЗАПРЕЩЕНА** (шаг 5 T032 — отдельный явный запрос)

## Задача

Оценить стоимость будущего вендоринга zed-wgpu; **не** планировать «сделать сейчас».

## Скоуп (только читать)

- `Source/Cargo.toml` ~88 — `wgpu` git `zed-industries/wgpu` rev `357a0c5`
- `Source/gpui_wgpu/`
- checkout: `~/.cargo/git/checkouts/wgpu-*/357a0c5`

## Команды

```bash
cd Source
cargo tree -i wgpu --depth 1 --target x86_64-unknown-linux-gnu
# checkout:
git log --oneline -15
# сравнить с gfx-rs/wgpu tag v29.0.3 если remote доступен — сколько zed-only commits
```

## Жёсткие запреты

- **Не** менять `Cargo.toml`, lock, `gpui_wgpu`, shaders
- **Не** vendor/path
- **Не** «顺便 поправить» renderer

## Выход

`Chronos-FM/docs/orchestration/tasks/active/T032-notes/I4-wgpu.md`

1. Размер checkout / число crates в workspace wgpu
2. zed-only delta vs upstream v29 (оценка)
3. Риски silent visual regression
4. Блок **«не имплементировать без отдельного запроса»**
5. Что нужно для будущего S5: grim baselines, checklist

## Definition of done

- note с честным cost estimate
- zero git changes
