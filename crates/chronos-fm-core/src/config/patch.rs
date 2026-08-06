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
    /// `[theme].mode`.
    ThemeMode(ThemeMode),
    /// `[theme].accent`.
    ThemeAccent(String),
    /// `[ui].default_sort`.
    UiDefaultSort(SortOrder),
    /// `[ui].show_hidden`.
    UiShowHidden(bool),
    /// `[ui].icon_pack`.
    UiIconPack(String),
    /// `[explorer].split_direction`.
    ExplorerSplitDirection(SplitDirection),
    /// `[explorer].synced_panes`.
    ExplorerSyncedPanes(bool),
    /// `[explorer].restore_tabs`.
    ExplorerRestoreTabs(bool),
}

impl ConfigField {
    /// The `config.toml` section table this field lives under.
    fn section(&self) -> &'static str {
        match self {
            Self::ThemeMode(_) | Self::ThemeAccent(_) => "theme",
            Self::UiDefaultSort(_) | Self::UiShowHidden(_) | Self::UiIconPack(_) => "ui",
            Self::ExplorerSplitDirection(_)
            | Self::ExplorerSyncedPanes(_)
            | Self::ExplorerRestoreTabs(_) => "explorer",
        }
    }

    /// The `config.toml` key within the section.
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

    /// The `toml_edit` item carrying the new value, in `config.toml` spelling.
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
    let section = field.section();
    let key = field.key();
    // Preserve the existing value's decor (space after `=` and any trailing
    // inline comment) so a point-edit keeps the user's annotation on that
    // line — a fresh `value()` item would silently drop it.
    let decor = doc[section]
        .get(key)
        .and_then(|item| item.as_value().map(|value| value.decor().clone()));
    doc[section][key] = field.toml_value();
    if let Some(decor) = decor {
        if let Some(value) = doc[section][key].as_value_mut() {
            *value.decor_mut() = decor;
        }
    }

    Ok(doc.to_string())
}

/// Reads `path`, applies `field`, writes the result back. Thin
/// read-patch-write wrapper around [`patch_config_text`] — the pure
/// function is what's unit-tested; this is exercised live (Task 3's
/// UI wiring), not here.
/// `std::fs` here is fine — chronos-fm-core is a non-UI crate (same
/// carve-out as `loader.rs`); the lint targets UI-layer call sites.
#[allow(clippy::disallowed_methods)]
pub fn patch_config_file(path: &Path, field: &ConfigField) -> Result<()> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let patched = patch_config_text(&source, field)?;
    std::fs::write(path, patched).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
