use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

// Crate-local assets so `chronos-fm-ui` stays self-contained and publishable: the
// folder is resolved relative to this crate's manifest dir and is included in
// the packaged crate. Workspace-root `assets/doc/` (README images) is separate.
/// GPUI `AssetSource` backed by the crate-local `assets/` directory embedded at compile time.
#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

/// Paths already reported missing, so a broken icon logs exactly once instead
/// of spamming `ERROR` on every painted frame (T019).
///
/// gpui's SVG element re-attempts the asset load every frame it is painted
/// (failed loads are never cached in the sprite atlas), so an `Err` here
/// becomes one ERROR line per frame — a real perf cost on top of the noise
/// (T014). The first miss for a path still returns `Err` so the caller's
/// `log_err` surfaces it once; every later miss returns `Ok(None)`, which
/// gpui treats as "no asset, paint nothing" and keeps quiet.
static MISSING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if let Some(f) = Self::get(path) {
            return Ok(Some(f.data));
        }
        let first_miss = match MISSING.get_or_init(|| Mutex::new(HashSet::new())).lock() {
            Ok(mut set) => set.insert(path.to_owned()),
            // A poisoned lock can only mean a panic inside the (infallible)
            // `insert`; recovering keeps the app rendering either way.
            Err(poisoned) => poisoned.into_inner().insert(path.to_owned()),
        };
        if first_miss {
            Err(anyhow::anyhow!("could not find asset at path \"{path}\""))
        } else {
            Ok(None)
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| {
                if p.starts_with(path) {
                    Some(gpui::SharedString::from(p.to_string()))
                } else {
                    None
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::Assets;
    use gpui::AssetSource;
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};

    /// Strips `//` line comments and `/* … */` block comments so prose like
    /// `IconName::Sync?` inside a doc comment can't create a phantom icon
    /// requirement in the audit test below.
    fn strip_comments(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let mut in_block = false;
        let mut chars = src.chars().peekable();
        while let Some(c) = chars.next() {
            if in_block {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    in_block = false;
                }
                continue;
            }
            match c {
                '/' if chars.peek() == Some(&'/') => {
                    // Line comment: drop through end of line.
                    while let Some(&next) = chars.peek() {
                        if next == '\n' {
                            break;
                        }
                        chars.next();
                    }
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    in_block = true;
                }
                _ => out.push(c),
            }
        }
        out
    }

    /// Every `IconName::Variant` occurrence, in source order.
    fn icon_names_in(code: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut rest = code;
        while let Some(pos) = rest.find("IconName::") {
            rest = &rest[pos + "IconName::".len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.push(name);
            }
        }
        names
    }

    /// Every literal `icons/<name>.svg` path, in source order.
    fn icon_literals_in(code: &str) -> Vec<String> {
        let mut literals = Vec::new();
        let mut rest = code;
        while let Some(pos) = rest.find("\"icons/") {
            rest = &rest[pos + 1..];
            let lit: String = rest.chars().take_while(|c| *c != '"').collect();
            if lit.ends_with(".svg") {
                literals.push(lit);
            }
        }
        literals
    }

    /// `HardDrive` → `hard-drive`, `GalleryVerticalEnd` → `gallery-vertical-end`.
    fn kebab(name: &str) -> String {
        let mut out = String::new();
        for c in name.chars() {
            if c.is_ascii_uppercase() {
                if !out.is_empty() {
                    out.push('-');
                }
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Collect every icon the workspace UI references — `IconName::Variant`
    /// usages and literal `icons/x.svg` paths — from all `crates/*/src`.
    ///
    /// This file is skipped: it holds the audit itself plus test scaffolding
    /// (dummy paths, `format!` templates) that is not shipped UI — the phantom
    /// references would fail the audit without meaning anything.
    fn used_icon_paths() -> Vec<String> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let audit_file = Path::new(file!());
        let mut found = HashSet::new();
        let mut stack: Vec<PathBuf> = vec![workspace_root.join("crates")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.ends_with(audit_file) {
                    continue;
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let src = std::fs::read_to_string(&path).unwrap();
                    let code = strip_comments(&src);
                    for name in icon_names_in(&code) {
                        found.insert(format!("icons/{}.svg", kebab(&name)));
                    }
                    for lit in icon_literals_in(&code) {
                        found.insert(lit);
                    }
                }
            }
        }
        let mut paths: Vec<String> = found.into_iter().collect();
        paths.sort();
        paths
    }

    /// T019 regression guard: every icon name the UI actually uses must have
    /// a file in `assets/icons/`. This catches the whole defect class (the
    /// missing `hard-drive.svg`/`arrow-up.svg` from T015, plus the `info.svg`
    /// the footer uses) instead of just the two known files.
    #[test]
    fn every_used_icon_has_an_asset() {
        let missing: Vec<String> = used_icon_paths()
            .into_iter()
            .filter(|path| Assets::get(path).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "icons used by the UI but missing from assets/icons/: {missing:?}"
        );
    }

    /// The audit scanner's own pieces: comment stripping, identifier capture,
    /// kebab-casing, and literal-path capture.
    #[test]
    fn scanner_finds_icon_names_and_strips_comments() {
        let code = r#"
            // IconName::Sync is prose in a comment — must be ignored
            let a = IconName::HardDrive;
            let b = IconName::GalleryVerticalEnd;
            /* IconName::Star inside a block comment — ignored */
            let c = IconName::ArrowUp;
            let d = "icons/folder.svg";
        "#;
        let stripped = strip_comments(code);
        assert!(!stripped.contains("Sync"));
        assert!(!stripped.contains("Star"));
        assert_eq!(icon_names_in(&stripped), ["HardDrive", "GalleryVerticalEnd", "ArrowUp"]);
        assert_eq!(icon_literals_in(&stripped), ["icons/folder.svg"]);
        assert_eq!(kebab("HardDrive"), "hard-drive");
        assert_eq!(kebab("GalleryVerticalEnd"), "gallery-vertical-end");
        assert_eq!(kebab("PanelBottomOpen"), "panel-bottom-open");
    }

    /// T019 fail-fast: a missing asset errors on the first load (so the caller
    /// logs it once) and returns `Ok(None)` afterwards — no per-frame ERROR
    /// spam. Uses a path that can never ship so the shared `MISSING` memo stays
    /// isolated from the real-icon tests.
    #[test]
    fn missing_asset_errors_once_then_stays_silent() {
        let path = "icons/definitely-not-a-real-icon-t019.svg";
        assert!(Assets.load(path).is_err(), "first miss must surface an error");
        assert!(matches!(Assets.load(path), Ok(None)), "later misses must be silent");
        assert!(matches!(Assets.load(path), Ok(None)), "later misses must be silent");
    }

    #[test]
    fn present_asset_loads_ok() {
        let data = Assets.load("icons/folder.svg").unwrap();
        let Some(data) = data else {
            panic!("folder.svg should be embedded in the asset bundle");
        };
        assert!(!data.is_empty());
    }
}
