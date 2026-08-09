# T032 — общие стены (справочник)

Полный текст тикета: `../T032-render-input-forks-vendoring.md`.  
В брифах агентов стены **продублированы** — этот файл не обязателен к открытию.

1. **Четыре потребителя** после каждого шага реализации:
   ```bash
   cargo check --workspace --manifest-path ChronOS/Cargo.toml
   cargo check --workspace --manifest-path Chronos-lm/Cargo.toml
   cargo test  --workspace --manifest-path Chronos-FM/Cargo.toml
   cargo build -p gpui-component-story --manifest-path Source/gpui-component/Cargo.toml
   ```
2. Один шаг = один коммит в `Source/`.
3. Вендор → `NOTICE` + `PATCHES.md` (`Source/skills/workspace-vendoring`).
4. Рендер/ввод (font-kit, xim, wgpu): grim до/после, не только `cargo check`.
5. Вне скоупа: `gpui_macos`. `gpui_windows` — не трогать (остаётся git).
6. **wgpu — не имплементировать** без отдельного явного запроса.
7. Hot files: `Source/Cargo.toml`, `Cargo.lock`, `NOTICE` — один writer или worktree.

Базовая линия 2026-08-08: все 4 → EXIT:0, логи `/tmp/t032-baselines/*-before.log`.
