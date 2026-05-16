use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{InputState, TextInput};
use std::sync::Once;

actions!(preview, [SafeSearch]);

static BIND_SEARCH_ONCE: Once = Once::new();

pub struct PreviewEditor {
    editor_state: Entity<InputState>,
}

impl PreviewEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        BIND_SEARCH_ONCE.call_once(|| {
            // Override the default search keybinding to prevent panic in gpui-component
            // caused by double borrowing InputState during search panel initialization.
            // We bind "cmd-f"/"ctrl-f" to our SafeSearch action in the "Input" context.
            cx.bind_keys(vec![
                KeyBinding::new("cmd-f", SafeSearch, Some("Input")),
                KeyBinding::new("ctrl-f", SafeSearch, Some("Input")),
            ]);
        });

        let editor_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("plain")
                .searchable(true)
                .line_number(true)
                .soft_wrap(false) // Enable horizontal scrolling
        });
        Self { editor_state }
    }

    pub fn set_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.editor_state.update(cx, |state, cx| {
            state.set_value(text, window, cx);
        });
    }

    pub fn set_language(&mut self, language: String, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor_state.update(cx, |state, cx| {
            state.set_highlighter(language, cx);
        });
    }

    pub fn set_highlights(
        &mut self,
        _highlights: Vec<(std::ops::Range<usize>, HighlightStyle)>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // TODO: Implement custom highlighting matching ranges to InputState's highlighting mechanism
        // InputState uses tree-sitter based highlighting usually, or DiagnosticSet for errors.
        // For search results, we might need a different approach or see if `search` functionality covers it.
    }

    pub fn scroll_to(&mut self, offset: usize, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor_state.update(cx, |state, cx| {
            state.scroll_to(offset, cx);
        });
    }

    pub fn set_search_query(&mut self, query: String, window: &mut Window, cx: &mut Context<Self>) {
        self.editor_state.update(cx, |state, cx| {
            state.set_search_query(query, window, cx);
        });
    }

    fn on_safe_search(&mut self, _: &SafeSearch, window: &mut Window, cx: &mut Context<Self>) {
        // Trigger search using set_search_query which defers the panel update, avoiding panic.
        self.editor_state.update(cx, |state, cx| {
            state.set_search_query("", window, cx);
        });
    }
}

impl Render for PreviewEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                TextInput::new(&self.editor_state)
                    .size_full()
                    .h_full()
                    .focus_bordered(false) // Remove black focus border
                    .appearance(false), // Remove default border/background for cleaner look
            )
            .on_action(cx.listener(Self::on_safe_search))
    }
}
