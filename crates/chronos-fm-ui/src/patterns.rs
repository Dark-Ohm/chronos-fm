//! Structural UI patterns ported from ChronOS (T231).
//!
//! These are toolkit-level presentational helpers shared across the explorer
//! surfaces. They are built on the T001 theme bridge (`crate::theme::theme`) so
//! they follow the live Chronos palette in both dark and light modes.
//!
//! Reference (read-only, layout logic only — never copy verbatim):
//! ChronOS `crates/app/src/side_panel_right/tab/ui.rs`. We deliberately do NOT
//! reuse ChronOS's `elevation_apply_light_chrome` wrapper — gpui-component has no
//! such helper — and instead use gpui's built-in `.shadow_md()` for the drop
//! shadow. No `gpui-rsx` is used here (it is not part of our stack).

use gpui::prelude::*;
use gpui::{AnyElement, App, Div, FontWeight, div, px};
use gpui_component::ActiveTheme;

use crate::theme::theme;

/// A slightly-raised card surface.
///
/// Reads `cx.theme()` through the T001 bridge: an elevated (`bg_secondary`)
/// fill, a `border`-colored hairline, rounded corners, and gpui's built-in
/// medium drop shadow. Reads as lifted above the page background.
///
/// Port of ChronOS `elevated_card(theme)`. The ChronOS version derived its
/// radius/shadow from `theme.elevation_popup()`; we use fixed `px(12.)` corners
/// and `.shadow_md()` because that is the equivalent primitive available in
/// gpui/gpui-component.
pub fn elevated_card(cx: &App) -> Div {
    div()
        .relative()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(16.))
        .px(px(16.))
        .py(px(16.))
        .bg(theme::bg_secondary(cx))
        .border_1()
        .border_color(theme::border(cx))
        .rounded(px(12.))
        .shadow_md()
}

/// A section heading: a 3×12px accent bar, a semibold title, and a muted
/// monospace subtitle.
///
/// Port of ChronOS `section_header(theme, title, subtitle)`. The accent bar
/// uses `accent` at 0.85 opacity; the title is the primary foreground at
/// `SEMIBOLD`; the subtitle is muted and rendered in the theme's monospace
/// family. Returns `AnyElement` so it can be dropped into any parent.
pub fn section_header(cx: &App, title: &str, subtitle: &str) -> AnyElement {
    let title = title.to_string();
    let subtitle = subtitle.to_string();
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(3.))
                        .h(px(12.))
                        .rounded(px(1.5))
                        .bg(theme::accent(cx).opacity(0.85)),
                )
                .child(
                    div()
                        .text_color(theme::fg(cx))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                ),
        )
        .child(
            div()
                .text_color(theme::muted(cx))
                .text_xs()
                .font_family(cx.theme().mono_font_family.clone())
                .child(subtitle),
        )
        .into_any_element()
}
