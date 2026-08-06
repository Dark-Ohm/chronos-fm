//! Global clipboard for copy/cut/paste in the explorer, shared across all
//! panes and tabs so a copy in one pane can be pasted in another.

use gpui::{App, Global};

/// Whether a clipboard entry was set via Copy (kept at the source) or Cut
/// (removed from the source once pasted).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardMode {
    /// Paste duplicates the source; the source is left in place.
    Copy,
    /// Paste moves the source; the source is removed once the paste succeeds.
    Cut,
}

/// The current explorer clipboard contents: zero or more absolute paths and
/// whether they were copied or cut.
#[derive(Clone, Default)]
pub struct FileClipboard {
    /// Absolute paths of the files/directories on the clipboard.
    pub paths: Vec<String>,
    /// How the paths were placed on the clipboard, or `None` when empty.
    pub mode: Option<ClipboardMode>,
}

impl Global for FileClipboard {}

/// Registers the clipboard global with its empty default. Must be called once
/// before any window opens (see `chronos-fm/src/app.rs`).
pub fn init(cx: &mut App) {
    cx.set_global(FileClipboard::default());
}

/// Replaces the clipboard with `paths` in Copy mode.
pub fn set_copy(paths: Vec<String>, cx: &mut App) {
    cx.set_global(FileClipboard {
        paths,
        mode: Some(ClipboardMode::Copy),
    });
}

/// Replaces the clipboard with `paths` in Cut mode.
pub fn set_cut(paths: Vec<String>, cx: &mut App) {
    cx.set_global(FileClipboard {
        paths,
        mode: Some(ClipboardMode::Cut),
    });
}

/// Empties the clipboard.
pub fn clear(cx: &mut App) {
    cx.set_global(FileClipboard::default());
}

/// Returns a clone of the current clipboard contents.
pub fn current(cx: &App) -> FileClipboard {
    cx.global::<FileClipboard>().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn set_copy_then_clear_round_trips(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            set_copy(vec!["/a".to_string(), "/b".to_string()], cx);
            let state = current(cx);
            assert_eq!(state.mode, Some(ClipboardMode::Copy));
            assert_eq!(state.paths, vec!["/a".to_string(), "/b".to_string()]);

            clear(cx);
            assert!(current(cx).mode.is_none());
            assert!(current(cx).paths.is_empty());
        });
    }

    #[gpui::test]
    async fn set_cut_replaces_a_prior_copy(cx: &mut TestAppContext) {
        cx.update(|cx| {
            init(cx);
            set_copy(vec!["/a".to_string()], cx);
            set_cut(vec!["/b".to_string()], cx);
            let state = current(cx);
            assert_eq!(state.mode, Some(ClipboardMode::Cut));
            assert_eq!(state.paths, vec!["/b".to_string()]);
        });
    }
}
