# T012 — Extensions Tab Landing Page: Implementation Plan

**Status:** Plan  
**Spec:** `docs/superpowers/specs/2026-08-06-extensions-tab-live.md`  
**Source:** `docs/orchestration/tasks/active/T012-extensions-tab-live.md`  
**Total estimated:** ~80 lines, 1 commit

## Task Breakdown

### Single Task — Rewrite ExtensionsPage + RootView wiring

**Crate:** `chronos-fm-pages`  
**Files:** `src/extensions.rs` (rewrite), `src/root.rs` (2 lines)  
**Lines:** ~80

1. **Rewire constructor:**
   - `ExtensionsPage::new(config: Config, window, cx) -> Self`
   - Store `config`, `focus_handle: cx.focus_handle()`
   - Add `set_config(&mut self, config: Config)` for hot reload
   - Implement `Focusable`

2. **Header:** `div().text_lg().font_weight(BOLD).child("🧩 Extensions")` +
   border bottom. Same pattern as S3Page.

3. **Architecture section** (`elevated_card` + `section_header("Plugin Architecture")`):
   - Static text: WASM Component Model, core vs community, doc links

4. **Configured Plugins section** (`elevated_card` +
   `section_header("Your Plugins (config.toml)")`):
   - Read `config.plugins.core` — render each as `🔌 <id>   [P4]` row
   - Read `config.plugins.community` — same pattern
   - Empty state when both lists empty: example TOML snippet

5. **Roadmap section** (`elevated_card` + `section_header("Roadmap")`):
   - Static: P4 (plugin host), P5 (marketplace)

6. **RootView wiring:**
   - Constructor: `ExtensionsPage::new(config.clone(), window, cx)`
   - Hot reload: `self.extensions.update(cx, |page, _cx| page.set_config(config.clone()))`
     in `apply_config`

### UI Patterns (reuse, no new components)

- `elevated_card(cx)` — card container
- `section_header(label)` — section title
- `theme::fg/accent/border/fg_secondary(cx)` — colors
- `FontWeight::BOLD`, `text_lg`, `text_base`, `text_sm` — typography

## Verification

| Step | Command | What |
|---|---|---|
| 1 | `cargo check -p chronos-fm-pages` | Compiles |
| 2 | `cargo test -p chronos-fm-pages` | 70/70 existing tests pass |
| 3 | `cargo clippy -p chronos-fm-pages` | No new warnings |

## Key Implementation Notes

- `section_header` returns `impl IntoElement` — use `.child(section_header("..."))` inside `elevated_card`.
- `config.plugins.core` is `Vec<String>`, `config.plugins.community` is `Vec<String>` — both from `chronos_fm_core::config::Plugins`.
- Hot reload already wired for S3Page and SettingsPage — add ExtensionsPage alongside them in `apply_config`.
- The empty-state TOML snippet uses `font_family("monospace")` and `bg(theme::bg_elevated(cx))` for code-block styling.
