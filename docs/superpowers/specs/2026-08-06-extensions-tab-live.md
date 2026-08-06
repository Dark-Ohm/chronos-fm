# T012 — Extensions Tab: Landing Page (Pre-P4)

**Status:** Spec  
**Source:** `docs/orchestration/tasks/active/T012-extensions-tab-live.md`  
**Priority:** P4 (blocked by plugin-host, but UI shell is P3-ready)

## 1. Overview

Replace the `ExtensionsPage` placeholder ("Extension store to be
implemented") with an informative landing page that:

1. **Explains the plugin architecture** (WASM Component Model,
   core vs community) to users visiting the tab.
2. **Shows configured plugins** from `config.toml`'s `[plugins]`
   section (`core` and `community` lists), live and hot-reloaded.
3. **Shows the roadmap** so users know when functional plugins arrive.

This is a **read-only information page** — no plugin installation,
discovery, or execution. Those land in P4 (plugin-host) and P5
(marketplace). The page bridges the gap between "the config schema
already accepts plugins" and "nothing renders them yet."

## 2. Layout

```
┌──────────────────────────────────────────────┐
│ 🧩 Extensions                                │  ← header
├──────────────────────────────────────────────┤
│ ┌──────────────────────────────────────────┐ │
│ │ Plugin Architecture                      │ │  ← section_header
│ │ WASM Component Model + wasmtime-wasi     │ │
│ │ Core plugins: Rust native, bundled       │ │
│ │ Community: WASM, sandboxed permissions   │ │
│ │ 📖 Plugin Overview · API · Permissions   │ │  ← doc links
│ └──────────────────────────────────────────┘ │
│                                              │
│ ┌──────────────────────────────────────────┐ │
│ │ Your Plugins (config.toml)               │ │  ← section_header
│ │ core:                                    │ │
│ │   🔌 git          [P4]                   │ │  ← accent icon + tag
│ │   🔌 calculator   [P4]                   │ │
│ │ community:                               │ │
│ │   🔌 user/repo    [P4]                   │ │
│ │ (No plugins configured.)                 │ │  ← empty state
│ └──────────────────────────────────────────┘ │
│                                              │
│ ┌──────────────────────────────────────────┐ │
│ │ Roadmap                                  │ │  ← section_header
│ │ P4: Plugin host (wasmtime-wasi)          │ │
│ │     Install, permissions, activation     │ │
│ │ P5: Plugin marketplace                   │ │
│ │     Browse, one-click install, Go tmpl   │ │
│ └──────────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
```

## 3. Components

### 3.1 ExtensionsPage struct

```rust
pub struct ExtensionsPage {
    config: Config,          // live, updated on hot reload
    focus_handle: FocusHandle,
}
```

Constructor: `ExtensionsPage::new(config, window, cx)` — same pattern
as `S3Page` and `GitPage`.

Public method: `set_config(&mut self, config: Config)` — called from
`RootView::apply_config` on hot reload.

### 3.2 Header

Plain text: "🧩 Extensions". `text_lg` + `FontWeight::BOLD`, border
bottom. Same pattern as the S3Page header.

### 3.3 Architecture Section

`section_header("Plugin Architecture")` from `chronos_fm_ui::patterns`.

Content (static text):
- "WASM Component Model + wasmtime-wasi"
- "Core plugins: Rust native, bundled with chronos-fm"
- "Community plugins: WASM, sandboxed, user-granted permissions"

Doc links (plain text, non-clickable for v1):
- "📖 Plugin Overview · API Reference · Permissions"

Design decision: links are informational text only (no `on_click`
navigation) — the app has no built-in markdown/doc viewer. A future
iteration could open the system browser.

### 3.4 Configured Plugins Section

`section_header("Your Plugins (config.toml)")`.

Reads `self.config.plugins.core` and `self.config.plugins.community`.

Each list rendered as:
- Sub-header: "core:" / "community:" (or hidden when empty)
- Per-plugin row: `🔌 <plugin-id>   [P4]` with muted tag

Empty state: "No plugins configured. Add them in `config.toml`:"
followed by example TOML snippet in a code-style div (`font_family("monospace")`,
`bg(theme::bg_elevated(cx))`).

Hot reload: `RootView::apply_config` calls `self.extensions.update(cx, |page, _cx| page.set_config(config.clone()))` — identical to the S3Page wiring already present.

### 3.5 Roadmap Section

`section_header("Roadmap")`.

Static text:
- "**P4:** Plugin host (wasmtime-wasi) — install, permissions, activation"
- "**P5:** Plugin marketplace — browse, one-click install, Go templates"

### 3.6 UI Patterns

Reuses existing components:
- `elevated_card(cx)` from `chronos_fm_ui::patterns` — card container
- `section_header(label)` from `chronos_fm_ui::patterns` — section title
- `theme::fg(cx)`, `theme::fg_secondary(cx)`, `theme::accent(cx)` — colors
- `FontWeight::BOLD`, `text_lg`, `text_base`, `text_sm` — typography

No new UI components. No imports beyond existing `gpui`, `gpui_component`,
`chronos_fm_ui`.

## 4. RootView Wiring

Already partially done (ExtensionsPage is constructed and rendered in
`render_active_page`). Changes:

1. Constructor: `ExtensionsPage::new(config.clone(), window, cx)`
   instead of `ExtensionsPage::new()`.
2. Hot reload: `self.extensions.update(cx, |page, _cx| page.set_config(config.clone()))`
   in `apply_config` (alongside existing S3Page and SettingsPage updates).

## 5. Out of Scope

- Plugin discovery / marketplace
- Plugin install / uninstall
- Permission consent UI
- Plugin activation / deactivation
- Any WASM runtime interaction
- Clickable documentation links (system browser)

## 6. Verification

- `cargo check -p chronos-fm-pages` — compiles
- `cargo test -p chronos-fm-pages` — 70/70 existing tests pass
- Live: Extensions tab shows architecture + roadmap + configured plugins
- Live: edit `config.toml` → add `core = ["git"]` → tab updates (hot reload)

## 7. Files

| File | Change |
|---|---|
| `crates/chronos-fm-pages/src/extensions.rs` | Rewrite (~80 lines) |
| `crates/chronos-fm-pages/src/root.rs` | 2 lines (constructor + hot reload) |
