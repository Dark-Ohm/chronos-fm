//! Top-level page views for the chronos-fm application, including the explorer,
//! git, S3, extensions, and settings pages, along with the root view that
//! hosts them.

use gpui::AnyElement;

/// The file explorer page and its supporting state and views.
pub mod explorer;
/// The extensions management page.
pub mod extensions;
/// The git page.
pub mod git;
/// A reusable 2-way split/tab container shared across pages.
pub mod pane_group;
/// The application root view that hosts the sidebar and active page.
pub mod root;
/// The S3 page.
pub mod s3;
// removed search
/// The settings page.
pub mod settings;

pub use root::RootView;

/// Identifies which top-level page is currently selected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageKind {
    /// The file explorer page.
    Explorer,

    /// The git page.
    Git,
    /// The S3 page.
    S3,
    /// The extensions page.
    Extensions,
    /// The settings page.
    Settings,
}

impl PageKind {
    /// Parses a page name from the `--page` debug CLI flag (T046 residual):
    /// case-insensitive, matches the same names `label()` returns plus a
    /// couple of obvious short aliases (`fs`, `files`). Returns `None` for
    /// anything else so the caller can warn-and-ignore rather than silently
    /// falling back to a page the user didn't ask for.
    pub fn from_cli_name(name: &str) -> Option<PageKind> {
        match name.trim().to_lowercase().as_str() {
            "explorer" | "fs" | "files" => Some(PageKind::Explorer),
            "git" => Some(PageKind::Git),
            "s3" => Some(PageKind::S3),
            "extensions" | "plugins" => Some(PageKind::Extensions),
            "settings" => Some(PageKind::Settings),
            _ => None,
        }
    }

    /// Returns the human-readable label for this page.
    pub fn label(&self) -> &'static str {
        match self {
            PageKind::Explorer => "Explorer",
            PageKind::Git => "Git",
            PageKind::S3 => "S3",
            PageKind::Extensions => "Extensions",
            PageKind::Settings => "Settings",
        }
    }

    /// Returns the asset path of the icon representing this page.
    pub fn icon_path(&self) -> &'static str {
        match self {
            PageKind::Explorer => "icons/folder.svg",

            PageKind::Git => "icons/git-branch.svg",
            PageKind::S3 => "icons/cloud.svg",
            PageKind::Extensions => "icons/puzzle.svg",
            PageKind::Settings => "icons/settings.svg",
        }
    }

    /// Returns all page kinds in display order.
    pub fn all() -> Vec<PageKind> {
        vec![
            PageKind::Explorer,
            PageKind::Git,
            PageKind::S3,
            PageKind::Extensions,
            PageKind::Settings,
        ]
    }
}

/// Trait for page rendering
pub trait Page {
    /// Renders the page into an element tree.
    fn render(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> AnyElement
    where
        Self: Sized;
}

#[cfg(test)]
mod tests {
    use super::PageKind;

    #[test]
    fn from_cli_name_matches_every_label_case_insensitively() {
        for kind in PageKind::all() {
            let lower = kind.label().to_lowercase();
            assert_eq!(PageKind::from_cli_name(&lower), Some(kind));
            assert_eq!(PageKind::from_cli_name(&kind.label().to_uppercase()), Some(kind));
        }
    }

    #[test]
    fn from_cli_name_accepts_short_aliases() {
        assert_eq!(PageKind::from_cli_name("fs"), Some(PageKind::Explorer));
        assert_eq!(PageKind::from_cli_name("files"), Some(PageKind::Explorer));
        assert_eq!(PageKind::from_cli_name("plugins"), Some(PageKind::Extensions));
    }

    #[test]
    fn from_cli_name_trims_whitespace() {
        assert_eq!(PageKind::from_cli_name("  git  "), Some(PageKind::Git));
    }

    #[test]
    fn from_cli_name_rejects_unknown() {
        assert_eq!(PageKind::from_cli_name("nonexistent"), None);
        assert_eq!(PageKind::from_cli_name(""), None);
    }
}
