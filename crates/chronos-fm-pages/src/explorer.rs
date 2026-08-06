mod entries;
mod list_setup;
mod navigation;
/// The split-view container that owns one or more panes (`docs/explorer-essentials.md` §3).
mod page;
mod preview;
pub mod properties;
mod search;
mod state;
mod types;
/// Rendering of a single explorer pane: header, sidebar, listing, and preview.
pub mod view;
/// Right-click context menu with "Open With" submenu (T007).
pub mod context_menu;

#[cfg(test)]
mod tests;

pub use page::ExplorerPage;
pub use state::ExplorerPane;

use gpui::{Context, IntoElement, Render, Window};

impl Render for ExplorerPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        view::render(self, window, cx)
    }
}
