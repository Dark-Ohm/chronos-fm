# Settings Tab v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `settings.rs` "to be implemented" placeholder with
a real form that edits the live-effective `config.toml` fields
(theme/ui/explorer) in place, preserving comments/formatting, with the
draft sections (keybindings/plugins/indexing/search/launcher) shown
disabled rather than hidden.

**Architecture:** A new `chronos-fm-core::config::patch` module edits
`config.toml` via `toml_edit::DocumentMut` (comment-preserving,
point-writes by key path) — pure, unit-testable with in-memory TOML
strings, no GPUI dependency. `SettingsPage` (the existing struct in
`settings.rs`) grows editable rows using `gpui_component::Switch` for
booleans and small button-groups (same idiom as a 2–4-way toggle) for
enums; each control's `on_click` calls the patch function and writes
the file. The existing config-watcher (`root.rs::start_config_watch`)
already hot-reloads on file change — no new propagation path needed.

**Tech Stack:** `toml_edit` (already in `Cargo.lock` transitively; add
as a direct `chronos-fm-core` dependency), `gpui_component::Switch`,
T002 patterns (`elevated_card`/`section_header`).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-06-settings-tab-live.md`.
- Point-edit `config.toml` via `toml_edit::DocumentMut` — never
  `toml::to_string(&Config)` full-document round-trip (destroys
  comments/formatting, spec explicitly forbids this).
- `unsafe_code = deny`, `clippy::unwrap_used`/`expect_used = warn`
  outside `#[cfg(test)]`.
- Every new `pub` item needs a doc comment (`missing_docs = "warn"`).
- Draft sections (Keybindings/Plugins/Indexing/Search/Launcher) render
  visibly but disabled — not omitted, per spec's explicit UX decision.
- No AI trailers in commits.

---

### Task 1: `config::patch` — comment-preserving point writes

**Files:**
- Create: `crates/chronos-fm-core/src/config/patch.rs`
- Modify: `crates/chronos-fm-core/src/config.rs` (add `pub mod patch;`)
- Modify: `crates/chronos-fm-core/Cargo.toml` (run `cargo add toml_edit -p chronos-fm-core` — let it resolve; do not hand-pin)

**Interfaces:**
- Produces:
  ```rust
  pub enum ConfigField {
      ThemeMode(chronos_fm_core::config::ThemeMode),
      ThemeAccent(String),
      UiDefaultSort(chronos_fm_core::config::SortOrder),
      UiShowHidden(bool),
      UiIconPack(String),
      ExplorerSplitDirection(chronos_fm_core::config::SplitDirection),
      ExplorerSyncedPanes(bool),
      ExplorerRestoreTabs(bool),
  }
  pub fn patch_config_text(source: &str, field: &ConfigField) -> anyhow::Result<String>;
  pub fn patch_config_file(path: &std::path::Path, field: &ConfigField) -> anyhow::Result<()>;
  ```
  `patch_config_text` is the pure, unit-testable core; `patch_config_file`
  is the thin read-write wrapper Task 3's UI calls.

- [ ] **Step 1: Write the failing test**

```rust
// crates/chronos-fm-core/src/config/patch.rs
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"#:schema https://chronos-fm.app/schema/config.schema.json
schema_version = 1

[theme]
mode = "system"   # "system" | "light" | "dark"
accent = "blue"   # named colour or hex (full customization in P5)

[ui]
default_sort = "name"   # "name" | "modified" | "size" | "kind"
show_hidden = false
icon_pack = "default"
"#;

    #[test]
    fn patches_theme_mode_preserving_comments() {
        let patched = patch_config_text(
            SAMPLE,
            &ConfigField::ThemeMode(crate::config::ThemeMode::Dark),
        )
        .unwrap();

        assert!(patched.contains(r#"mode = "dark""#));
        // The inline comment on that same line must survive.
        assert!(patched.contains(r#"mode = "dark"   # "system" | "light" | "dark""#));
        // An untouched line elsewhere must be byte-identical.
        assert!(patched.contains(r#"accent = "blue"   # named colour or hex (full customization in P5)"#));
        // Only one line changed.
        let before_lines: Vec<&str> = SAMPLE.lines().collect();
        let after_lines: Vec<&str> = patched.lines().collect();
        assert_eq!(before_lines.len(), after_lines.len());
        let diff_count = before_lines
            .iter()
            .zip(after_lines.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(diff_count, 1, "exactly one line should differ");
    }

    #[test]
    fn patches_ui_show_hidden_bool() {
        let patched =
            patch_config_text(SAMPLE, &ConfigField::UiShowHidden(true)).unwrap();
        assert!(patched.contains("show_hidden = true"));
    }

    #[test]
    fn patches_missing_section_by_creating_it() {
        // A config.toml with no [explorer] section at all — patching an
        // Explorer field must add the section, not error.
        let minimal = "schema_version = 1\n";
        let patched = patch_config_text(
            minimal,
            &ConfigField::ExplorerSyncedPanes(true),
        )
        .unwrap();
        assert!(patched.contains("[explorer]"));
        assert!(patched.contains("synced_panes = true"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-fm-core config::patch:: 2>&1`
Expected: FAIL — `patch_config_text`, `ConfigField` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/chronos-fm-core/src/config/patch.rs
//! Comment- and formatting-preserving point edits to `config.toml`,
//! used by the live Settings tab (spec:
//! docs/superpowers/specs/2026-08-06-settings-tab-live.md). Never does a
//! full `Config` -> TOML round-trip — that would discard the user's
//! comments and layout — only ever sets one key at a time via
//! `toml_edit::DocumentMut`.

use crate::config::{SortOrder, SplitDirection, ThemeMode};
use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::{DocumentMut, value};

/// One editable Settings-tab field and its new value. Each variant maps
/// to exactly one `config.toml` key path (`[section].key`).
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigField {
    ThemeMode(ThemeMode),
    ThemeAccent(String),
    UiDefaultSort(SortOrder),
    UiShowHidden(bool),
    UiIconPack(String),
    ExplorerSplitDirection(SplitDirection),
    ExplorerSyncedPanes(bool),
    ExplorerRestoreTabs(bool),
}

impl ConfigField {
    fn section(&self) -> &'static str {
        match self {
            Self::ThemeMode(_) | Self::ThemeAccent(_) => "theme",
            Self::UiDefaultSort(_) | Self::UiShowHidden(_) | Self::UiIconPack(_) => "ui",
            Self::ExplorerSplitDirection(_)
            | Self::ExplorerSyncedPanes(_)
            | Self::ExplorerRestoreTabs(_) => "explorer",
        }
    }

    fn key(&self) -> &'static str {
        match self {
            Self::ThemeMode(_) => "mode",
            Self::ThemeAccent(_) => "accent",
            Self::UiDefaultSort(_) => "default_sort",
            Self::UiShowHidden(_) => "show_hidden",
            Self::UiIconPack(_) => "icon_pack",
            Self::ExplorerSplitDirection(_) => "split_direction",
            Self::ExplorerSyncedPanes(_) => "synced_panes",
            Self::ExplorerRestoreTabs(_) => "restore_tabs",
        }
    }

    fn toml_value(&self) -> toml_edit::Item {
        match self {
            Self::ThemeMode(mode) => value(match mode {
                ThemeMode::System => "system",
                ThemeMode::Light => "light",
                ThemeMode::Dark => "dark",
            }),
            Self::ThemeAccent(s) | Self::UiIconPack(s) => value(s.as_str()),
            Self::UiDefaultSort(sort) => value(match sort {
                SortOrder::Name => "name",
                SortOrder::Modified => "modified",
                SortOrder::Size => "size",
                SortOrder::Kind => "kind",
            }),
            Self::UiShowHidden(b) | Self::ExplorerSyncedPanes(b) | Self::ExplorerRestoreTabs(b) => {
                value(*b)
            }
            Self::ExplorerSplitDirection(dir) => value(match dir {
                SplitDirection::Vertical => "vertical",
                SplitDirection::Horizontal => "horizontal",
            }),
        }
    }
}

/// Applies `field` to `source` (the raw text of a `config.toml`),
/// returning the patched text. Preserves every comment, blank line, and
/// key ordering except the single changed value. Creates the section
/// table if it doesn't exist yet (a config that never mentions
/// `[explorer]` still has explorer defaults applied at load time — this
/// lets the Settings tab add just that one section without disturbing
/// anything else).
pub fn patch_config_text(source: &str, field: &ConfigField) -> Result<String> {
    let mut doc: DocumentMut = source.parse().context("parsing config.toml")?;

    if doc.get(field.section()).is_none() {
        doc[field.section()] = toml_edit::table();
    }
    doc[field.section()][field.key()] = field.toml_value();

    Ok(doc.to_string())
}

/// Reads `path`, applies `field`, writes the result back. Thin
/// read-patch-write wrapper around [`patch_config_text`] — the pure
/// function is what's unit-tested; this is exercised live (Task 3's
/// UI wiring), not here.
pub fn patch_config_file(path: &Path, field: &ConfigField) -> Result<()> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let patched = patch_config_text(&source, field)?;
    std::fs::write(path, patched).with_context(|| format!("writing {}", path.display()))
}
```

**Note for the implementer:** confirm the exact variant names of
`ThemeMode`/`SortOrder`/`SplitDirection` against
`crates/chronos-fm-core/src/config/settings.rs` before writing the
`match` arms above — the plan's names are taken from that file as of
this writing, but re-check field/variant spelling case-sensitively
(e.g. `SortOrder::Kind` vs `SortOrder::FileType` — grep `pub enum
SortOrder` in that file first).

Add to `crates/chronos-fm-core/src/config.rs`:

```rust
pub mod patch;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-fm-core config::patch:: 2>&1`
Expected: PASS — `patches_theme_mode_preserving_comments`,
`patches_ui_show_hidden_bool`, `patches_missing_section_by_creating_it`.

- [ ] **Step 5: Commit**

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
git add crates/chronos-fm-core/src/config/patch.rs crates/chronos-fm-core/src/config.rs crates/chronos-fm-core/Cargo.toml Cargo.lock
git commit -m "core: comment-preserving config.toml point-edit (toml_edit)"
```

---

### Task 2: config path resolution helper (where does `config.toml` live?)

**Files:**
- Modify: `crates/chronos-fm-core/src/config/patch.rs` (add one function)

**Interfaces:**
- Consumes: nothing new
- Produces: `pub fn default_config_path() -> Result<std::path::PathBuf>`

This task exists because Task 1's `patch_config_file` needs a concrete
path, and the Settings UI (Task 4) needs to know where to write without
duplicating path-resolution logic that already exists somewhere in this
crate for the *reading* side.

- [ ] **Step 1: Find the existing read-side path resolution**

Run: `grep -rn "config.toml\|fn.*config_path\|xdg" crates/chronos-fm-core/src/config.rs crates/chronos-fm-core/src/config/*.rs 2>&1`

The loader (used by `root.rs`'s config watcher) already resolves
`~/.config/chronos-fm/config.toml` (or `$XDG_CONFIG_HOME` equivalent)
somewhere in this crate — find that exact function/constant.

- [ ] **Step 2: Reuse it, don't duplicate it**

If a `pub fn` already returns the path (even if named differently, e.g.
`config_file_path()` or similar), re-export or call it directly from
`patch.rs`:

```rust
/// The `config.toml` path the Settings tab writes to — the same path
/// the app's config loader/watcher already reads from and hot-reloads
/// (see `root.rs::start_config_watch`). Delegates to the existing
/// resolution logic rather than duplicating XDG-path handling.
pub fn default_config_path() -> anyhow::Result<std::path::PathBuf> {
    // Replace this body with a call to whatever function Step 1 found —
    // do NOT hand-roll a new `~/.config/chronos-fm/config.toml` join
    // here if one already exists.
    todo!("call the existing config-path resolver found in Step 1")
}
```

If Step 1 finds no existing reusable function (the path is inlined
ad-hoc at each call site instead of centralized), implement it fresh
using the same `dirs` crate pattern already used elsewhere in this
codebase (`crates/chronos-fm-services/src/search/indexer.rs` uses
`dirs::home_dir()` — follow that same convention, don't add a new path
crate).

- [ ] **Step 3: Add a test**

```rust
#[cfg(test)]
mod default_path_tests {
    use super::*;

    #[test]
    fn default_config_path_ends_with_expected_filename() {
        let path = default_config_path().unwrap();
        assert_eq!(path.file_name().unwrap(), "config.toml");
        assert!(path.to_string_lossy().contains("chronos-fm"));
    }
}
```

Run: `cargo test -p chronos-fm-core config::patch::default_path_tests:: 2>&1`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/chronos-fm-core/src/config/patch.rs
git commit -m "core: default_config_path helper for Settings tab writes"
```

---

### Task 3: `SettingsPage` — live theme/ui/explorer form

**Files:**
- Modify: `crates/chronos-fm-pages/src/settings.rs`
- Modify: `crates/chronos-fm-pages/Cargo.toml` (confirm `chronos-fm-core` and `gpui_component` are already dependencies — `grep -E "chronos-fm-core|gpui.component" crates/chronos-fm-pages/Cargo.toml`; add if missing)

**Interfaces:**
- Consumes: `chronos_fm_core::config::patch::{ConfigField, patch_config_file, default_config_path}` (Tasks 1–2); `gpui_component::Switch`; `chronos_fm_ui::patterns::{elevated_card, section_header}` (T002)
- Produces: nothing new consumed elsewhere — this is the leaf UI task.

This task has no automated test beyond "renders without panicking"
(same T002/T003 precedent — GPUI widget trees with live D-Bus/file I/O
side effects in click handlers aren't meaningfully unit-testable; the
*logic* they call, `patch_config_text`, already has full coverage from
Task 1).

- [ ] **Step 1: Write the failing test**

```rust
// crates/chronos-fm-pages/src/settings.rs — bottom of file
#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, point, px, size};

    #[gpui::test]
    async fn settings_page_renders_without_panicking(cx: &mut TestAppContext) {
        cx.update(|cx| gpui_component::init(cx));
        let cx = cx.add_empty_window();
        cx.draw(point(px(0.0), px(0.0)), size(px(600.0), px(900.0)), |window, cx| {
            let page = cx.new(|_cx| super::SettingsPage::new());
            page.update(cx, |page, cx| {
                use gpui::Render;
                page.render(window, cx).into_any_element()
            })
        });
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chronos-fm-pages settings::tests 2>&1`
Expected: FAIL (or panics) — `SettingsPage` currently has no config
loaded, so this establishes the baseline before the form is added; if
it already passes trivially because the placeholder renders fine,
that's expected too — the point of this step is confirming the render
path works *before* adding interactive controls, so any panic
introduced by Step 3 is attributable to your change.

- [ ] **Step 3: Write the live form**

Replace `crates/chronos-fm-pages/src/settings.rs`'s `Render for
SettingsPage` body. `SettingsPage` needs to hold the current `Config`
(read once at construction — the config-watcher elsewhere in the app
already re-triggers a full re-render on file change via
`root.rs::apply_config`, so this page doesn't need its own file-watch;
it reads whatever `Config` the app passes it):

```rust
use chronos_fm_core::config::patch::{ConfigField, default_config_path, patch_config_file};
use chronos_fm_core::config::{Config, ThemeMode, SortOrder, SplitDirection};
use gpui::{AnyElement, Context, Render, Window, div, prelude::*, px};
use gpui_component::Switch;
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;

/// The settings page: a live editor for the `config.toml` fields that
/// actually take effect (theme/ui/explorer), plus disabled previews of
/// the draft sections that don't yet (keybindings/plugins/indexing/
/// search/launcher — spec's explicit "show, don't hide" decision).
pub struct SettingsPage {
    config: Config,
}

impl SettingsPage {
    /// Creates a new settings page seeded with the app's current config.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Re-reads `config.toml`, patches one field, and writes it back.
    /// The app's existing config-watcher (`root.rs`) picks up the file
    /// change and re-applies it — this function does not mutate
    /// `self.config` directly; the next full config reload does that.
    fn write_field(field: ConfigField) {
        match default_config_path().and_then(|path| {
            patch_config_file(&path, &field)?;
            Ok(())
        }) {
            Ok(()) => {}
            Err(error) => {
                tracing::error!("Failed to write config.toml: {error}");
            }
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(16.))
            .p(px(24.))
            .bg(theme::bg(cx))
            .child(theme_section(&self.config, cx))
            .child(ui_section(&self.config, cx))
            .child(explorer_section(&self.config, cx))
            .child(draft_sections(cx))
    }
}

fn mode_button(
    label: &'static str,
    mode: ThemeMode,
    active: bool,
    cx: &gpui::App,
) -> impl IntoElement {
    div()
        .id(label)
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |d| d.bg(theme::accent(cx)).text_color(theme::bg(cx)))
        .when(!active, |d| d.text_color(theme::fg_secondary(cx)))
        .on_click(move |_event, _window, _cx| {
            SettingsPage::write_field(ConfigField::ThemeMode(mode));
        })
        .child(label)
}

fn theme_section(config: &Config, cx: &gpui::App) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(cx, "Theme", "appearance"))
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child(mode_button("System", ThemeMode::System, config.theme.mode == ThemeMode::System, cx))
                .child(mode_button("Light", ThemeMode::Light, config.theme.mode == ThemeMode::Light, cx))
                .child(mode_button("Dark", ThemeMode::Dark, config.theme.mode == ThemeMode::Dark, cx)),
        )
}

fn ui_section(config: &Config, cx: &gpui::App) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(cx, "UI", "listing behavior"))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("Show hidden files"))
                .child(
                    Switch::new("show-hidden")
                        .checked(config.ui.show_hidden)
                        .on_click(|checked, _window, _cx| {
                            SettingsPage::write_field(ConfigField::UiShowHidden(*checked));
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child(sort_button("Name", SortOrder::Name, config.ui.default_sort, cx))
                .child(sort_button("Modified", SortOrder::Modified, config.ui.default_sort, cx))
                .child(sort_button("Size", SortOrder::Size, config.ui.default_sort, cx))
                .child(sort_button("Kind", SortOrder::Kind, config.ui.default_sort, cx)),
        )
}

fn sort_button(
    label: &'static str,
    sort: SortOrder,
    current: SortOrder,
    cx: &gpui::App,
) -> impl IntoElement {
    let active = sort == current;
    div()
        .id(label)
        .px(px(10.))
        .py(px(4.))
        .rounded(px(6.))
        .cursor_pointer()
        .when(active, |d| d.bg(theme::accent(cx)).text_color(theme::bg(cx)))
        .when(!active, |d| d.text_color(theme::fg_secondary(cx)))
        .on_click(move |_event, _window, _cx| {
            SettingsPage::write_field(ConfigField::UiDefaultSort(sort));
        })
        .child(label)
}

fn explorer_section(config: &Config, cx: &gpui::App) -> impl IntoElement {
    elevated_card(cx)
        .child(section_header(cx, "Explorer", "split view"))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("Synced panes"))
                .child(
                    Switch::new("synced-panes")
                        .checked(config.explorer.synced_panes)
                        .on_click(|checked, _window, _cx| {
                            SettingsPage::write_field(ConfigField::ExplorerSyncedPanes(*checked));
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_color(theme::fg(cx)).child("Restore tabs on restart"))
                .child(
                    Switch::new("restore-tabs")
                        .checked(config.explorer.restore_tabs)
                        .on_click(|checked, _window, _cx| {
                            SettingsPage::write_field(ConfigField::ExplorerRestoreTabs(*checked));
                        }),
                ),
        )
}

/// Draft sections not yet wired to any subsystem (P3/P4) — shown, not
/// hidden, per spec's decision. Visually muted, no interactive controls.
fn draft_sections(cx: &gpui::App) -> impl IntoElement {
    div()
        .opacity(0.5)
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_color(theme::fg_secondary(cx))
                .text_sm()
                .child("Keybindings, Plugins, Indexing, Search, Launcher — coming in P3/P4"),
        )
}

impl crate::Page for SettingsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}
```

**Note for the implementer:**
1. `SettingsPage::new` now takes a `Config` — find every call site that
   constructs `SettingsPage` (likely `root.rs` or wherever `Page` trait
   objects are built) and update it to pass the current config;
   `SettingsPage::default()` (the old zero-arg constructor) can no
   longer exist unchanged — either remove the `Default` impl or make it
   build with `Config::default()` explicitly, whichever the call sites
   need. Grep `SettingsPage::new()\|SettingsPage::default()` before
   this step to see every place this ripples to.
2. `gpui_component::Switch`'s exact `on_click` closure signature and
   `elevated_card`'s exact chaining (does it need `.id()` applied by
   the caller, per its doc comment ported in T002?) — verify against
   the actual `Switch`/`elevated_card` source before trusting the code
   above verbatim; this plan's code was written from reading
   `Source/gpui-component/crates/ui/src/switch.rs` and
   `crates/chronos-fm-ui/src/patterns.rs` (T002) directly, but re-check
   for drift.
3. `SortOrder`/`ThemeMode` variant names: confirm exact spelling in
   `crates/chronos-fm-core/src/config/settings.rs` (`pub enum
   SortOrder`) before compiling — this plan assumed `Name`/`Modified`/
   `Size`/`Kind`, adjust the `match` arms in Task 1 and the button list
   here together if they differ.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p chronos-fm-pages settings:: 2>&1`
Expected: PASS — `settings_page_renders_without_panicking`.

- [ ] **Step 5: Full workspace check**

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
cargo build --workspace 2>&1; echo EXIT=$?
cargo test --workspace 2>&1 | grep -E "^test result"
```

Expected: `EXIT=0`, all `test result: ok`, no `FAILED`.

- [ ] **Step 6: Commit**

```bash
git add crates/chronos-fm-pages/src/settings.rs crates/chronos-fm-pages/Cargo.toml
git commit -m "ui: live Settings tab (theme/ui/explorer editing, draft sections shown disabled)"
```

---

## Final Live Verification (do this before declaring the plan complete)

1. `cargo build --release --bin chronos-fm 2>&1; echo EXIT=$?` — expect 0.
2. Launch, open the Settings tab.
3. Click "Dark" in the Theme section → whole app repaints dark (T001
   palette) immediately (via the existing config-watcher hot-reload —
   confirm this actually round-trips: click writes file → watcher
   reads it back → `Theme::change` fires).
4. `cat ~/.config/chronos-fm/config.toml` (or wherever `default_config_path`
   resolved) — confirm exactly one line changed vs. a `git diff`/backup
   of the file taken before the click; all comments intact.
5. Toggle "Show hidden files" → explorer listing actually shows/hides
   dotfiles.
6. Confirm the draft-sections text renders visibly muted and has no
   clickable controls.
7. `grim` screenshots of the Settings tab, both themes.

## Self-Review Notes

- **Spec coverage:** live theme/ui/explorer editing (Tasks 1–3),
  comment-preserving write-back (Task 1), draft sections shown-disabled
  (Task 3) — all of spec's scope covered. Config-conflict resolution and
  P5 color picker are explicitly out of scope per spec, not tasked.
- **Type consistency:** `ConfigField` (Task 1) variants match every
  `SettingsPage` call site (Task 3) 1:1; `default_config_path` (Task 2)
  is the single path source both `patch_config_file` and the UI use —
  no duplicated path logic.
- **Known soft spots, named explicitly:** (1) exact `SortOrder`/
  `ThemeMode` variant spelling — plan assumed names from reading the
  config source but flags re-verification; (2) `Switch`/`elevated_card`
  exact call signature — same treatment as T003's zbus-API-drift
  disclosure, not hidden as a false certainty.
