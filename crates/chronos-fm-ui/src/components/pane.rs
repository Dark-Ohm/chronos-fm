use crate::theme::theme;
use gpui::{App, IntoElement, div, prelude::*, px};

/// Non-functional tab bar (placeholder)
pub fn tab_bar(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .gap_2()
        .p_2()
        .border_1()
        .border_color(theme::accent(cx))
        .bg(theme::bg(cx))
        .text_color(theme::fg(cx))
        .child(
            div()
                .px_2()
                .py_1()
                .border_1()
                .border_color(theme::accent(cx))
                .child("Tab 1"),
        )
        .child(
            div()
                .px_2()
                .py_1()
                .border_1()
                .border_color(theme::accent(cx))
                .child("Tab 2"),
        )
}

/// Split container with a vertical resize bar (non-functional placeholder)
pub fn split_container<L: IntoElement, R: IntoElement>(left: L, right: R, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .gap_1()
        .child(
            div()
                .border_1()
                .border_color(theme::accent(cx))
                .child(left),
        )
        .child(
            // Resize bar placeholder
            div().w(px(4.0)).bg(theme::accent(cx)),
        )
        .child(
            div()
                .border_1()
                .border_color(theme::accent(cx))
                .child(right),
        )
}

#[cfg(test)]
mod tests {
    use super::{split_container, tab_bar};
    use gpui::{ParentElement, TestAppContext, div, point, px, size};

    #[gpui::test]
    async fn placeholders_lay_out_without_panicking(cx: &mut TestAppContext) {
        // Register gpui-component globals (incl. the `Theme` global the bridge
        // reads via `cx.theme()`). The real app does this in `app.rs::run`.
        cx.update(|app| gpui_component::init(app));
        let cx = cx.add_empty_window();
        // `draw` runs the element through layout and paint, so the tab-bar and
        // split-container builders are actually exercised (not just constructed).
        cx.draw(
            point(px(0.0), px(0.0)),
            size(px(800.0), px(600.0)),
            |_window, cx| {
                div()
                    .child(tab_bar(cx))
                    .child(split_container(div().child("left"), div().child("right"), cx))
            },
        );
    }
}
