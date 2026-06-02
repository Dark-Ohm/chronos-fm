mod entries;
mod list_setup;
mod navigation;
mod preview;
mod search;
mod state;
mod types;
/// Rendering of the explorer page: header, sidebar, listing, and preview.
pub mod view;

#[cfg(test)]
mod tests;

pub use state::ExplorerPage;

use gpui::{AnyElement, Context, IntoElement, Render, Window};

impl Render for ExplorerPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        view::render(self, window, cx)
    }
}

impl crate::Page for ExplorerPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        <Self as Render>::render(self, window, cx).into_any_element()
    }
}
