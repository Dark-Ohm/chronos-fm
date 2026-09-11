# Chronos-FM — long-term project notes

## ⚠️ Pre-existing ashpd build blocker (gates ALL GUI-crate work)
`cargo check -p chronos-fm` / `-p chronos-fm-pages` fails:
`compile_error!("You can't enable both async-io & tokio features at once")`.
Cause: `gpui_linux`→`ashpd/async-io` AND `gpui_linux`(secret)→`oo7`→`ashpd/tokio`;
feature unification enables both. Reproducible at clean HEAD. Pre-existing, NOT
from our edits. **Effect:** cannot compile/build/run any crate pulling
`gpui_linux` (pages + binary). Pages-crate tests (e.g. properties.rs) and any
live GUI measurement are UNVERIFIABLE in this env — architect must do on a
buildable machine. `chronos-fm-services` compiles fine. Separate blocking issue
for architect; outside T013/T014/T016 scope.

## Conventions
- Task tickets + reports are written in **Russian** (match repo convention).
- Reports live in `docs/orchestration/tasks/report/` (inbox); acceptance moves
  them to `report-log/`. Tickets: `active/` → `done/`/`rejected/`.
- Grep tool's bundled ripgrep is sandbox-blocked (EACCES) → use `grep` via Bash
  with **quoted** globs (zsh expands `*.rs` otherwise).
- Fork is a flat-layout gpui-ce at `../Source` (`gpui/`, `gpui_wgpu/`,
  `gpui_linux/`, `gpui-component/`), NOT `crates/gpui/...`.
