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
    /// `[s3].default_profile`.
    S3DefaultProfile(String),
    /// `[s3.profiles.<profile>].endpoint`.
    S3ProfileEndpoint { profile: String, endpoint: String },
    /// `[s3.profiles.<profile>].region`.
    S3ProfileRegion { profile: String, region: String },
    /// `[s3.profiles.<profile>].force_path_style`.
    S3ProfileForcePathStyle { profile: String, value: bool },
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
            Self::S3DefaultProfile(_) => "s3",
            Self::S3ProfileEndpoint { .. }
            | Self::S3ProfileRegion { .. }
            | Self::S3ProfileForcePathStyle { .. } => "s3",
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
            Self::S3DefaultProfile(_) => "default_profile",
            Self::S3ProfileEndpoint { .. }
            | Self::S3ProfileRegion { .. }
            | Self::S3ProfileForcePathStyle { .. } => "profiles",
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
            Self::ThemeAccent(s) | Self::UiIconPack(s) | Self::S3DefaultProfile(s) => {
                value(s.as_str())
            }
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
            // S3 profile fields: the section is `s3` but the key path is
            // `profiles.<profile>.<field>`. These are handled specially in
            // `patch_config_text` — here we just provide the value.
            Self::S3ProfileEndpoint { endpoint, .. } => value(endpoint.as_str()),
            Self::S3ProfileRegion { region, .. } => value(region.as_str()),
            Self::S3ProfileForcePathStyle { value: v, .. } => value(*v),
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

    // S3 profile fields write to `[s3.profiles.<profile>]`, a sub-table.
    // They are routed through a separate path that ensures the profile
    // table exists.
    if matches!(
        field,
        ConfigField::S3ProfileEndpoint { .. }
            | ConfigField::S3ProfileRegion { .. }
            | ConfigField::S3ProfileForcePathStyle { .. }
    ) {
        return patch_s3_profile(&mut doc, field);
    }

    // Missing section, or a hand-edited non-table at that key (e.g. a
    // top-level `theme = "dark"` string) — replace it with a table
    // before indexing, or toml_edit would panic on the non-table item.
    // The old item is removed first: replacing it in place would leak its
    // decor into the header (rendering `[theme ]` instead of `[theme]`).
    if doc.get(field.section()).is_none_or(|item| !item.is_table()) {
        doc.as_table_mut().remove(field.section());
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

/// Patch a key inside `[s3.profiles.<profile>]`, creating the sub-table
/// hierarchy if needed.
fn patch_s3_profile(doc: &mut DocumentMut, field: &ConfigField) -> Result<String> {
    let (profile, profile_key, toml_item) = match field {
        ConfigField::S3ProfileEndpoint { profile, endpoint } => {
            (profile, "endpoint", value(endpoint.as_str()))
        }
        ConfigField::S3ProfileRegion { profile, region } => {
            (profile, "region", value(region.as_str()))
        }
        ConfigField::S3ProfileForcePathStyle { profile, value: v } => {
            (profile, "force_path_style", value(*v))
        }
        _ => unreachable!("patch_s3_profile called for non-S3 field"),
    };

    // Ensure `[s3]` exists.
    if doc.get("s3").is_none_or(|item| !item.is_table()) {
        doc.as_table_mut().remove("s3");
        doc["s3"] = toml_edit::table();
    }
    // Ensure `[s3.profiles.<profile>]` exists.
    let profiles_key = format!("profiles.{profile}");
    if doc["s3"].get(&profiles_key).is_none_or(|item| !item.is_table()) {
        doc["s3"][&profiles_key] = toml_edit::table();
    }
    // Preserve decor on the existing value.
    let decor = doc["s3"][&profiles_key]
        .get(profile_key)
        .and_then(|item| item.as_value().map(|value| value.decor().clone()));
    doc["s3"][&profiles_key][profile_key] = toml_item;
    if let Some(decor) = decor {
        if let Some(value) = doc["s3"][&profiles_key][profile_key].as_value_mut() {
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

/// The `config.toml` path the Settings tab writes to — the same path
/// the app's config loader/watcher already reads from and hot-reloads
/// (see `root.rs::start_config_watch`). Delegates to [`super::paths::config_file`]
/// rather than duplicating XDG-path handling.
pub fn default_config_path() -> Result<std::path::PathBuf> {
    Ok(super::paths::config_file())
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
    fn default_config_path_ends_with_expected_filename() {
        // Reads `XDG_CONFIG_HOME`, so it must serialize with the mutating
        // XDG tests in `paths.rs` (same convention as `paths::tests`).
        let _guard = crate::config::test_env::env_lock();
        let path = default_config_path().unwrap();
        assert_eq!(path.file_name().unwrap(), "config.toml");
        assert!(path.to_string_lossy().contains("chronos-fm"));
    }

    #[test]
    fn patches_non_table_section_by_replacing_it() {
        // A hand-edited top-level `theme = "dark"` string (malformed but
        // loadable) must not panic: the section is replaced with a table.
        let malformed = "schema_version = 1\ntheme = \"dark\"\n";
        let patched = patch_config_text(
            malformed,
            &ConfigField::ThemeMode(crate::config::ThemeMode::Light),
        )
        .unwrap();
        assert!(patched.contains("[theme]"));
        assert!(patched.contains("mode = \"light\""));
    }

    #[test]
    fn patches_explorer_split_direction() {
        // The Settings tab's split-direction buttons write through this
        // variant — both spellings must round-trip to config.toml keys.
        let base = "schema_version = 1\n[explorer]\n";
        let vertical = patch_config_text(
            base,
            &ConfigField::ExplorerSplitDirection(crate::config::SplitDirection::Vertical),
        )
        .unwrap();
        assert!(vertical.contains("split_direction = \"vertical\""));

        let horizontal = patch_config_text(
            base,
            &ConfigField::ExplorerSplitDirection(crate::config::SplitDirection::Horizontal),
        )
        .unwrap();
        assert!(horizontal.contains("split_direction = \"horizontal\""));
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
