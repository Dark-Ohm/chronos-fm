/// Global clipboard for copy/cut/paste, shared across panes and tabs.
pub mod clipboard;
pub(crate) mod marquee;
mod entries;
mod file_ops;
mod list_setup;
mod navigation;
mod rename;
/// The split-view container that owns one or more panes (`docs/explorer-essentials.md` §3).
mod page;
mod preview;
pub mod properties;
mod search;
/// Batch Rename dialog (T006) — modal multi-file rename with a live preview.
pub mod batch_rename;
mod state;
mod types;
/// Rendering of a single explorer pane: header, sidebar, listing, and preview.
pub mod view;
/// Unified right-click context menu: file ops (Rename/Copy/Cut/Copy Path/
/// Delete) + "Open With" (T007) on rows, New Folder/Paste/Refresh on empty
/// space (b3/b4).
pub mod context_menu;

#[cfg(test)]
mod keybindings;
#[cfg(test)]
pub(crate) mod tests;

pub use page::ExplorerPage;
pub use state::ExplorerPane;

use gpui::{Context, IntoElement, Render, Window};

impl Render for ExplorerPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        view::render(self, window, cx)
    }
}
