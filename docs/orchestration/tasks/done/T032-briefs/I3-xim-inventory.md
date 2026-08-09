# Agent I3 — inventory: xim-rs (zed-xim)

**Тип:** read-only разведка  
**Параллельно:** да  
**Блокирует:** S4

## Задача

Сузить diff zed-форка xim-rs до IME-специфики; оценить vendor cost  
(монорепо: `zed-xim` + `xim-ctext` + `xim-parser`).

## Скоуп (только читать)

- `Source/gpui_linux/Cargo.toml` ~129–133 (`package = "zed-xim"`, rev `16f35a2`)
- usage: `rg xim` в `Source/gpui_linux/src`
- checkout: `~/.cargo/git/checkouts/xim-rs-*/16f35a2`

## Команды

```bash
cd Source
cargo tree -i zed-xim --target x86_64-unknown-linux-gnu -e features | head -50
rg -n 'xim|Xim|XIM' gpui_linux/src --glob '*.rs' | head -40
# checkout:
git log --oneline -20
ls  # workspace members xim-ctext xim-parser
```

## Стены

- Только Linux/X11 path (`gpui_linux` feature `x11` включает `xim`).
- Pure Wayland smoke **не** докажет IME xim — в note честно ограничение.
- S4: живой кадр/ввод где возможно; не только `cargo check`.
- Не менять deps.

## Выход

`Chronos-FM/docs/orchestration/tasks/active/T032-notes/I3-xim.md`

1. Список zed/fix commits (кратко)
2. Какие subcrates надо вендорить вместе
3. crates.io `zed-xim` published?
4. Рекомендация + smoke plan (X11 vs Wayland)

## Definition of done

- note с evidence
- zero git changes
