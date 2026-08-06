//! Skip-lists for the file indexer and watcher.
//!
//! `$HOME` (and `~/Documents`) of any real user eventually contains something
//! unreadable or pathologically heavy: podman/Docker volumes owned by another
//! uid, `~/.cache`, `node_modules`, `target/`, `.git` object stores, FUSE/gvfs
//! mounts. Walking into those is both slow and — for the watcher — fatal: a
//! single `PermissionDenied` aborts `notify`'s whole recursive add, which used
//! to take the entire search service down (T016). We skip them instead.

use std::path::{Path, PathBuf};

/// Directory-name fragments that are never indexed or watched, matched by path
/// component. `containers` catches `~/.local/share/containers/...` (the podman
/// volume from the T016 bug report) without excluding unrelated paths.
pub const DEFAULT_EXCLUDE_COMPONENTS: &[&str] = &[
    "containers",
    "node_modules",
    "target",
    ".git",
    ".cache",
    ".gvfs",
    ".cargo",
    ".npm",
    ".rustup",
    ".Trash",
    ".npm",
];

/// Resolved exclude set: user-configured `[indexing.exclude]` paths/globs.
/// The built-in [`DEFAULT_EXCLUDE_COMPONENTS`] are always applied on top.
#[derive(Clone, Debug, Default)]
pub struct Excludes {
    /// Absolute or indexed-root-relative literal paths to skip.
    pub paths: Vec<PathBuf>,
    /// Glob patterns (e.g. `**/target/**`) to skip.
    pub globs: Vec<String>,
}

impl Excludes {
    /// Build from config strings plus the built-in defaults.
    pub fn from_config(paths: Vec<String>, globs: Vec<String>) -> Self {
        Self {
            paths: paths.into_iter().map(PathBuf::from).collect(),
            globs,
        }
    }

    /// True if `path` should be skipped during indexing/watching.
    pub fn matches(&self, path: &Path) -> bool {
        // 1. Built-in component deny-list — catches the unreadable podman
        //    volume, node_modules, target, .git, .cache, FUSE mounts, ...
        for comp in path.components() {
            let name = comp.as_os_str().to_string_lossy();
            if DEFAULT_EXCLUDE_COMPONENTS.iter().any(|c| *c == name.as_ref()) {
                return true;
            }
        }

        // 2. User-configured literal paths: absolute => prefix match;
        //    relative => any component equal to the path's last component.
        for p in &self.paths {
            if p.is_absolute() {
                if path.starts_with(p) {
                    return true;
                }
            } else if let Some(last) = p.components().last() {
                let want = last.as_os_str();
                if path.components().any(|c| c.as_os_str() == want) {
                    return true;
                }
            }
        }

        // 3. User-configured globs.
        for g in &self.globs {
            if glob_matches(g, path) {
                return true;
            }
        }

        false
    }
}

/// Minimal glob matcher supporting `**` (match zero or more components) and
/// literal components. Enough for `**/target/**`, `node_modules`, `build/*`.
fn glob_matches(pattern: &str, path: &Path) -> bool {
    let pat: Vec<String> = pattern.split('/').map(|s| s.to_string()).collect();
    if pat.is_empty() {
        return false;
    }
    let pcs: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    glob_seq(&pat, &pcs)
}

/// Sequentially matches `pat` (glob segments) against `pcs` (path components),
/// treating `**` as a zero-or-more wildcard.
fn glob_seq(pat: &[String], pcs: &[String]) -> bool {
    let (mut pi, mut ci) = (0usize, 0usize);
    while pi < pat.len() {
        let seg = &pat[pi];
        if seg == "**" {
            let rest = &pat[pi + 1..];
            if rest.is_empty() {
                return true;
            }
            for k in ci..=pcs.len() {
                if glob_seq(rest, &pcs[k..]) {
                    return true;
                }
            }
            return false;
        } else if ci < pcs.len() && pcs[ci] == *seg {
            pi += 1;
            ci += 1;
        } else {
            return false;
        }
    }
    ci == pcs.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_components_skip_known_heavy_dirs() {
        let e = Excludes::default();
        assert!(e.matches(Path::new("/home/neo/.local/share/containers/storage/vol")));
        assert!(e.matches(Path::new("/home/neo/Documents/proj/node_modules/x")));
        assert!(e.matches(Path::new("/home/neo/Documents/proj/target/debug/app")));
        assert!(e.matches(Path::new("/home/neo/repo/.git/objects")));
        assert!(e.matches(Path::new("/home/neo/.cache/something")));
        assert!(!e.matches(Path::new("/home/neo/Documents/proj/src/main.rs")));
        assert!(!e.matches(Path::new("/home/neo/Pictures/photo.png")));
    }

    #[test]
    fn config_absolute_path_prefix_matches() {
        let e = Excludes::from_config(vec!["/home/neo/.secret".to_string()], vec![]);
        assert!(e.matches(Path::new("/home/neo/.secret/inner")));
        assert!(!e.matches(Path::new("/home/neo/.secretly/not")));
    }

    #[test]
    fn config_relative_path_component_matches() {
        let e = Excludes::from_config(vec!["build".to_string()], vec![]);
        assert!(e.matches(Path::new("/home/neo/proj/build/out")));
        assert!(!e.matches(Path::new("/home/neo/proj/building")));
    }

    #[test]
    fn config_glob_matches() {
        let e = Excludes::from_config(vec![], vec!["**/target/**".to_string()]);
        assert!(e.matches(Path::new("/home/neo/proj/target/debug/app")));
        assert!(!e.matches(Path::new("/home/neo/proj/targets/app")));
    }
}
