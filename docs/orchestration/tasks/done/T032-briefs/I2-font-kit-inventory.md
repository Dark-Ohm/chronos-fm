# Agent I2 — inventory: font-kit (zed-font-kit)

**Тип:** read-only разведка  
**Параллельно:** да  
**Блокирует:** S3

## Задача

Оценить, насколько zed-форк разъехался от servo/font-kit, и что дешевле:  
**crates.io `zed-font-kit = 0.14.1-zed`** vs path-vendor.

## Скоуп (только читать)

- `Source/gpui/Cargo.toml` ~122–123 (git font-kit, package `zed-font-kit`)
- `Source/gpui_wgpu/Cargo.toml` ~38
- checkout: `~/.cargo/git/checkouts/font-kit-*/94b0f28`
- usage: `gpui_wgpu/src/cosmic_text_system.rs` (`#[cfg(feature = "font-kit")]`)

## Команды

```bash
cd Source
cargo tree -i zed-font-kit --target x86_64-unknown-linux-gnu -e features | head -40
cargo info zed-font-kit
# в checkout font-kit:
git log --oneline -20
# сравнить published crates.io version с rev 94b0f28 если возможно
```

## Стены

- Feature `font-kit` **в default** gpui → в production graph.
- S3 потребует **grim** — в note заложить smoke-сценарий (какой app, что смотреть на кадре).
- Не менять deps.

## Выход

`Chronos-FM/docs/orchestration/tasks/active/T032-notes/I2-font-kit.md`

1. Таблица zed-only commits поверх servo
2. crates.io pin viable? (same rev / API)
3. Рекомендация: `crates.io` | `path-vendor` | `defer`
4. Grim plan для S3 (1–3 bullets)

## Definition of done

- note с evidence
- zero git changes
