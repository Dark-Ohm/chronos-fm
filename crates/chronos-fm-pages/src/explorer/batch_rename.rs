//! Batch Rename dialog (T006): pattern / find-replace / counter inputs with a
//! live "was → will be" preview shown before anything is applied.
//!
//! The dialog owns four `InputState`s and recomputes the preview on every
//! `InputEvent::Change` via `cx.subscribe`, so the preview always reflects what
//! Apply would do. Apply renames each file in preview order through
//! `ops::rename_in_place` and reports per-file failures through the
//! `on_committed` callback (which the pane uses to reload and surface errors).

use std::rc::Rc;

use chronos_fm_services::fs::batch_rename::{build_preview, RenamePreview, RenameStatus, Template};
use chronos_fm_services::fs::listing::FileEntryDto;
use chronos_fm_ui::patterns::{elevated_card, section_header};
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputEvent, InputState};

/// Called on Apply with the per-file errors (empty `Vec` = success).
type CommitCallback = Rc<dyn Fn(&mut App, Vec<String>)>;

/// Called on Cancel to close the dialog through the pane.
type CancelCallback = Rc<dyn Fn(&mut App)>;

/// Modal state for renaming a multi-file selection with a pattern (T006).
pub struct BatchRenameDialog {
    /// The selected entries, in row order (their `name`s are the "was" side).
    pub entries: Vec<FileEntryDto>,
    /// Pattern input (`{name}_{n}.{ext}` …).
    pattern: Entity<InputState>,
    /// Find/replace inputs applied to the stem before `{name}`.
    find: Entity<InputState>,
    replace: Entity<InputState>,
    /// Counter start (defaults to 1 when empty; invalid text is an error).
    start: Entity<InputState>,
    /// Live preview, recomputed on every keystroke.
    pub preview: Vec<RenamePreview>,
    /// Pattern / start parse error shown as a banner; Apply is disabled while set.
    pub last_error: Option<String>,
    /// Subscriptions to the four inputs' `InputEvent::Change`, kept alive for
    /// the dialog's lifetime so typing always refreshes the preview.
    subs: Vec<Subscription>,
    /// Called on Apply with the per-file errors (empty `Vec` = success).
    on_committed: CommitCallback,
    /// Called when Cancel is pressed — closes the dialog through the pane.
    on_cancel: CancelCallback,
}

impl BatchRenameDialog {
    /// Builds the dialog for `entries`, wiring the four inputs so every change
    /// recomputes the preview. `on_committed` fires on Apply (the pane reloads
    /// and reports errors); `on_cancel` fires on Cancel (the pane closes it).
    pub fn new(
        entries: Vec<FileEntryDto>,
        window: &mut Window,
        cx: &mut Context<Self>,
        on_committed: CommitCallback,
        on_cancel: CancelCallback,
    ) -> Self {
        let pattern = cx.new(|cx| InputState::new(window, cx));
        let find = cx.new(|cx| InputState::new(window, cx));
        let replace = cx.new(|cx| InputState::new(window, cx));
        let start = cx.new(|cx| InputState::new(window, cx));

        let mut dialog = Self {
            entries,
            pattern,
            find,
            replace,
            start,
            preview: Vec::new(),
            last_error: None,
            subs: Vec::new(),
            on_committed,
            on_cancel,
        };

        for input in [
            dialog.pattern.clone(),
            dialog.find.clone(),
            dialog.replace.clone(),
            dialog.start.clone(),
        ] {
            let sub = cx.subscribe(&input, |this, _entity, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.refresh_preview(cx);
                }
            });
            dialog.subs.push(sub);
        }

        dialog.refresh_preview(cx);
        dialog
    }

    /// The pattern input, exposed for tests to set the pattern text.
    #[cfg(test)]
    pub(crate) fn pattern_input(&self) -> &Entity<InputState> {
        &self.pattern
    }

    /// Re-reads the four inputs and recomputes the preview (+ parse errors).
    pub fn refresh_preview(&mut self, cx: &mut Context<Self>) {
        let pattern = self.pattern.read(cx).text().to_string();
        let find = self.find.read(cx).text().to_string();
        let replace = self.replace.read(cx).text().to_string();
        let start_text = self.start.read(cx).text().to_string();
        let start: u64 = if start_text.is_empty() {
            1
        } else {
            match start_text.parse() {
                Ok(n) => n,
                Err(_) => {
                    self.preview.clear();
                    self.last_error = Some("Counter start must be a number".into());
                    return;
                }
            }
        };
        self.last_error = match Template::parse(&pattern) {
            Ok(template) => {
                self.preview = build_preview(&self.entries, &template, &find, &replace, start);
                None
            }
            Err(e) => {
                self.preview.clear();
                Some(format!("Pattern error: {e:?}"))
            }
        };
    }

    /// Whether Apply is safe: a non-empty preview, no parse error, and at least
    /// one row that actually changes.
    pub fn can_apply(&self) -> bool {
        !self.preview.is_empty()
            && self.last_error.is_none()
            && self.preview.iter().any(|p| p.new_name != p.old_name)
    }

    /// Applies every previewed rename via `ops::rename_in_place` (resolved names
    /// in preview order), collects per-file errors, then fires `on_committed`.
    pub fn apply(&mut self, cx: &mut Context<Self>) {
        if !self.can_apply() {
            return;
        }
        let mut errors: Vec<String> = Vec::new();
        // `build_preview` iterates `entries` in order, so the previews align
        // one-to-one with the entries (names are unique within one directory).
        for (entry, preview) in self.entries.iter().zip(self.preview.iter()) {
            let src = std::path::Path::new(&entry.path);
            match chronos_fm_services::fs::ops::rename_in_place(src, &preview.new_name) {
                Ok(_) => {}
                Err(error) => errors.push(format!("{}: {error}", preview.old_name)),
            }
        }
        let on_committed = std::mem::replace(
            &mut self.on_committed,
            Rc::new(|_cx: &mut App, _errors: Vec<String>| {}) as CommitCallback,
        );
        on_committed(cx, errors);
    }

    /// Cancels without applying anything, via the pane.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        let on_cancel = std::mem::replace(
            &mut self.on_cancel,
            Rc::new(|_cx: &mut App| {}) as CancelCallback,
        );
        on_cancel(cx);
    }
}

impl Render for BatchRenameDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog = cx.entity();
        let dialog_for_apply = dialog.clone();
        let can_apply = self.can_apply();

        elevated_card(cx)
            .w(px(560.))
            .child(section_header(
                cx,
                "Batch Rename",
                &format!("{} files", self.entries.len()),
            ))
            .child(input_row(cx, "Pattern", &self.pattern, "e.g. {name}_{n:3}.{ext}"))
            .child(input_row(cx, "Find", &self.find, "text to replace in the stem"))
            .child(input_row(cx, "Replace", &self.replace, "replacement text"))
            .child(input_row(cx, "Start", &self.start, "counter start (default 1)"))
            .when_some(self.last_error.clone(), |card, err| {
                card.child(
                    div()
                        .w_full()
                        .text_xs()
                        .text_color(theme::danger(cx))
                        .child(err),
                )
            })
            .child(preview_list(cx, &self.preview))
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("batch-rename-cancel")
                            .label("Cancel")
                            .on_click(move |_event, _window, cx| {
                                dialog.update(cx, |d, cx| d.cancel(cx));
                            }),
                    )
                    // Apply is always visible but inert (dimmed) until the
                    // preview is valid and changes something.
                    .child(
                        Button::new("batch-rename-apply")
                            .label("Apply")
                            .when(!can_apply, |button| button.opacity(0.5))
                            .when(can_apply, |button| {
                                button.on_click(move |_event, _window, cx| {
                                    dialog_for_apply.update(cx, |d, cx| d.apply(cx));
                                })
                            }),
                    ),
            )
    }
}

impl super::ExplorerPane {
    /// Opens the Batch Rename dialog for `entries` (the multi-file selection).
    /// On Apply the dialog renames every file and then reloads this pane,
    /// reporting per-file failures through the footer status (error after
    /// `reload()`, per the repo rule).
    pub(crate) fn open_batch_rename(
        &mut self,
        entries: Vec<FileEntryDto>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = cx.entity();
        let pane_for_cancel = pane.clone();
        let on_committed: CommitCallback = Rc::new(move |cx, errors| {
            pane.update(cx, |pane, cx| {
                pane.batch_rename = None;
                pane.reload();
                if !errors.is_empty() {
                    pane.set_status(
                        super::types::StatusLevel::Error,
                        format!("Batch rename failed for {}", errors.join(", ")),
                    );
                }
                cx.notify();
            });
        });
        let on_cancel: CancelCallback = Rc::new(move |cx| {
            pane_for_cancel.update(cx, |pane, cx| pane.close_batch_rename(cx));
        });
        let dialog =
            cx.new(|cx| BatchRenameDialog::new(entries, window, cx, on_committed, on_cancel));
        dialog.update(cx, |dialog, cx| {
            dialog.pattern.update(cx, |input, cx| input.focus(window, cx));
        });
        self.batch_rename = Some(dialog);
        cx.notify();
    }

    /// Closes the Batch Rename dialog without applying anything.
    pub(crate) fn close_batch_rename(&mut self, cx: &mut Context<Self>) {
        self.batch_rename = None;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    // Test fixtures write files directly; the synchronous-fs ban targets app code.
    #![allow(clippy::disallowed_methods)]

    use super::super::tests::new_explorer_for_tests;
    use chronos_fm_services::fs::batch_rename::RenameStatus;
    use gpui::TestAppContext;
    use std::fs;
    use tempfile::tempdir;

    /// Opens the dialog for two files, sets the pattern, and asserts the live
    /// preview — then closes it. The whole flow runs inside a single
    /// `window.update` so no live `Input` is painted across an update boundary
    /// (the pane-rooted test harness has no `gpui_component::Root`; see the
    /// `new_explorer_for_tests` docs).
    #[gpui::test]
    async fn batch_rename_dialog_previews_pattern_and_closes(cx: &mut TestAppContext) {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "x").unwrap();
        fs::write(dir.path().join("b.txt"), "y").unwrap();
        let window = new_explorer_for_tests(cx, dir.path());
        window.update(cx, |page, _window, _cx| page.reload()).unwrap();

        window
            .update(cx, |page, window, cx| {
                // Select both rows, then open the dialog for the selection
                // (exactly what the context-menu Rename item does).
                page.select_single(0);
                page.select_range_to(1);
                let entries = page.filtered_entries_for_selection();
                assert_eq!(entries.len(), 2);
                page.open_batch_rename(entries, window, cx);
                assert!(page.batch_rename.is_some());

                let dialog = page.batch_rename.clone().unwrap();
                dialog.update(cx, |dialog, cx| {
                    // Set the pattern and refresh: `{name}_{n:3}.{ext}` over
                    // a.txt/b.txt with default start 1.
                    let pattern = dialog.pattern_input().clone();
                    pattern.update(cx, |state, cx| {
                        state.set_value("{name}_{n:3}.{ext}", window, cx)
                    });
                    dialog.refresh_preview(cx);

                    assert!(dialog.last_error.is_none());
                    assert_eq!(dialog.preview.len(), 2);
                    let names: Vec<&str> =
                        dialog.preview.iter().map(|p| p.new_name.as_str()).collect();
                    assert_eq!(names, vec!["a_001.txt", "b_002.txt"]);
                    assert!(dialog
                        .preview
                        .iter()
                        .all(|p| p.status == RenameStatus::Ok));
                    assert!(dialog.can_apply());
                });

                page.close_batch_rename(cx);
                assert!(page.batch_rename.is_none());
            })
            .unwrap();

        // Nothing was applied — the files keep their original names.
        assert!(dir.path().join("a.txt").exists());
        assert!(dir.path().join("b.txt").exists());
        assert!(!dir.path().join("a_001.txt").exists());
    }
}

fn input_row(
    cx: &App,
    label: &str,
    input: &Entity<InputState>,
    hint: &str,
) -> impl IntoElement {
    let label = label.to_string();
    let hint = hint.to_string();
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .w_full()
                .flex()
                .justify_between()
                .child(div().text_xs().text_color(theme::fg_secondary(cx)).child(label))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted(cx))
                        .child(hint),
                ),
        )
        .child(Input::new(input))
}

fn preview_list(cx: &App, preview: &[RenamePreview]) -> impl IntoElement {
    let mut list = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(2.))
        .max_h(px(220.))
        .overflow_hidden();
    if preview.is_empty() {
        return list
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(theme::muted(cx))
                    .child("Type a pattern to preview the renames."),
            )
            .into_any_element();
    }
    for row in preview {
        let resolved = row.status == RenameStatus::ResolvedCollision;
        list = list.child(
            div()
                .w_full()
                .flex()
                .gap(px(8.))
                .when(resolved, |d| d.opacity(0.6))
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_sm()
                        .text_color(theme::muted(cx))
                        .child(row.old_name.clone()),
                )
                .child(div().text_sm().text_color(theme::fg_secondary(cx)).child("→"))
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_sm()
                        .text_color(theme::fg(cx))
                        .child(row.new_name.clone()),
                )
                .when(resolved, |d| {
                    d.child(
                        div()
                            .text_xs()
                            .text_color(theme::muted(cx))
                            .child("(auto-resolved)"),
                    )
                }),
        );
    }
    list.into_any_element()
}
