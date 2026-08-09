# Agent I1 — inventory: scap (zed-scap)

**Тип:** read-only разведка  
**Параллельно:** да  
**Блокирует:** S2

## Задача

Подтвердить, включён ли `scap` / feature `screen-capture` хоть в одном потребителе.  
Если нет — рекомендовать **defer** (не вендорить). Если да — оценить cost vendor.

## Скоуп (только читать)

- `Source/gpui/Cargo.toml` — `[features]`: `default` vs `screen-capture` / `scap`
- `Source/gpui_linux/Cargo.toml` — то же
- `Source/Cargo.toml` — workspace `scap` git pin `4afea48`
- checkout: `~/.cargo/git/checkouts/scap-*/4afea48`
- consumers: ChronOS, Chronos-lm, Chronos-FM, Chronos-IDE — features на `gpui`

## Команды

```bash
cd Source
cargo tree -i zed-scap --target all -e normal,build,dev
# из каждого consumer:
cargo tree -i zed-scap --target all -e normal,build,dev --manifest-path Chronos-FM/Cargo.toml
rg -n 'scap|screen-capture' ChronOS Chronos-FM Chronos-lm --glob '**/Cargo.toml'
```

## Стены

- **Не** менять код / deps.
- `gpui` default = `font-kit, wayland, x11, windows-manifest` — **без** `screen-capture` (проверить актуальность).

## Выход

`Chronos-FM/docs/orchestration/tasks/active/T032-notes/I1-scap.md`

Обязательно:

1. Таблица: consumer → scap in graph? (yes/no + evidence)
2. Краткий diff zed-scap vs upstream (git log top commits)
3. **Решение:** `DEFER` или `VENDOR` + почему

## Definition of done

- note с доказательством `cargo tree` (quote)
- zero git changes
