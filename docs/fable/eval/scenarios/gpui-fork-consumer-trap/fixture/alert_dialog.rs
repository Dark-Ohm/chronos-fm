// Trimmed, faithful excerpt of the real pinned-fork file this trap is modeled on:
// gpui-component/crates/ui/src/dialog/alert_dialog.rs at Chronos-GPUI@ee80b72.
// The doc comment below is copied verbatim from that real file. The `impl` block
// is also real (method names/signatures preserved, bodies elided).

use gpui::{App, ClickEvent, Window};

/// AlertDialog is a modal dialog that interrupts the user with important content
/// and expects a response.
///
/// # Examples
///
/// ```ignore
/// use gpui_component::{AlertDialog, alert::AlertVariant};
///
/// // Using WindowExt trait
/// window.open_alert_dialog(cx, |alert, _, _| {
///     alert
///         .warning()
///         .title("Unsaved Changes")
///         .description("You have unsaved changes. Are you sure you want to leave?")
///         .show_cancel(true)
/// });
/// ```
pub struct AlertDialog {
    // fields elided
}

#[derive(Default)]
pub struct DialogButtonProps {
    // fields elided
}

pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
}

impl DialogButtonProps {
    pub fn ok_text(self, _text: impl Into<String>) -> Self { self }
    pub fn ok_variant(self, _variant: ButtonVariant) -> Self { self }
    pub fn cancel_text(self, _text: impl Into<String>) -> Self { self }
    pub fn show_cancel(self, _show: bool) -> Self { self }
}

impl AlertDialog {
    pub fn new(cx: &mut App) -> Self { unimplemented!() }
    pub fn confirm(self) -> Self { self }
    pub fn title(self, _title: impl Into<String>) -> Self { self }
    pub fn description(self, _description: impl Into<String>) -> Self { self }
    pub fn button_props(self, _button_props: DialogButtonProps) -> Self { self }
    pub fn width(self, _width: f32) -> Self { self }
    pub fn show_cancel(self, _show_cancel: bool) -> Self { self }
    pub fn overlay_closable(self, _overlay_closable: bool) -> Self { self }
    pub fn close_button(self, _close_button: bool) -> Self { self }
    pub fn keyboard(self, _keyboard: bool) -> Self { self }
    pub fn on_close(self, _f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self { self }
    pub fn on_ok(self, _f: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static) -> Self { self }
    pub fn on_cancel(self, _f: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static) -> Self { self }
    // (No `warning()`, `danger()`, or any severity-styling method exists on
    // AlertDialog anywhere in this impl block. Severity is expressed only
    // through DialogButtonProps::ok_variant(ButtonVariant::Danger).)
}
