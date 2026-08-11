//! Paste/drop conflict dialog (T053).
//!
//! When a paste or drop would collide with an existing destination entry, the
//! transfer pauses behind this modal so the user can pick Skip / Rename /
//! Cancel / Overwrite — one conflict at a time, with an "apply to all"
//! checkbox (mockup `docs/explorer-essentials.md` §1.2, Rename is the default
//! `Enter` decision and Overwrite sits rightmost in the accent color).
//!
//! All decision state (the pure [`ConflictQueue`] from `conflict.rs`, the
//! current position, the apply-to-all flag) lives on the pane in
//! [`PendingTransfer`]; the dialog entity is a thin view over the current
//! item whose buttons call back into the pane. Decisions therefore never
//! re-enter the pane from inside its own update (which would panic the
//! entity map): keyboard decisions run directly on the pane, and button
//! decisions arrive from window-level events where the pane is not borrowed.

use std::path::PathBuf;
use std::rc::Rc;

use chronos_fm_services::fs::ops::ConflictResolution;
use chronos_fm_ui::patterns::elevated_card;
use chronos_fm_ui::theme::theme;
use gpui::prelude::*;
use gpui::*;
use gpui_component::Icon;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};

use super::conflict::{ConflictChoice, ConflictItem, ConflictQueue};
use super::properties::human_size;
use super::types::StatusLevel;
use super::view::listing::row::icon_path_for;
use super::ExplorerPane;

/// Runs the actual transfer once every conflict has been decided. Invoked
/// exactly once with the pane and its UI context (the pane is passed directly
/// so the transfer can finish against it without re-entering the entity,
/// which the pane's own update context would panic on) and the collected
/// resolutions.
pub(crate) type TransferStarter = Box<
    dyn FnOnce(
        &mut ExplorerPane,
        &mut Context<ExplorerPane>,
        Vec<Option<ConflictResolution>>,
    ),
>;

/// The paused paste/drop operation awaiting conflict decisions. Holds every
/// piece of decision state so the pane can drive the queue from keyboard
/// events without touching the dialog entity; `start` owns the transfer
/// itself (sources, destination, mode, executor).
pub(crate) struct PendingTransfer {
    pub queue: ConflictQueue,
    /// Conflict items in queue order (parallel to the queue's indices).
    pub items: Vec<ConflictItem>,
    /// Position of the item currently shown (in lockstep with the queue).
    pub position: usize,
    /// Whether the decision applies to every remaining conflict.
    pub apply_all: bool,
    pub start: TransferStarter,
}

/// Called when the user decides the current conflict.
type DecideCallback = Rc<dyn Fn(ConflictChoice, &mut App)>;
/// Called when the apply-to-all checkbox is toggled.
type ToggleApplyAllCallback = Rc<dyn Fn(&mut App)>;

/// One queued conflict presented modally (mockup §1.2). A view only: the
/// decision state lives on the pane's [`PendingTransfer`].
pub(crate) struct ConflictDialog {
    item: ConflictItem,
    /// Conflicts following the one shown (the "(N more)" counter).
    remaining: usize,
    apply_all: bool,
    on_decide: DecideCallback,
    on_toggle_apply_all: ToggleApplyAllCallback,
}

impl ConflictDialog {
    pub fn new(
        item: ConflictItem,
        remaining: usize,
        apply_all: bool,
        on_decide: DecideCallback,
        on_toggle_apply_all: ToggleApplyAllCallback,
    ) -> Self {
        Self {
            item,
            remaining,
            apply_all,
            on_decide,
            on_toggle_apply_all,
        }
    }

    /// Re-points the view at the next conflict of the same operation.
    pub fn set_item(&mut self, item: ConflictItem, remaining: usize, cx: &mut Context<Self>) {
        self.item = item;
        self.remaining = remaining;
        cx.notify();
    }

    /// Applies the checked apply-to-all flag to the view.
    pub fn set_apply_all(&mut self, apply_all: bool, cx: &mut Context<Self>) {
        self.apply_all = apply_all;
        cx.notify();
    }
}

impl Render for ConflictDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog = cx.entity();
        let item = self.item.clone();
        let remaining = self.remaining;
        let apply_all = self.apply_all;
        let icon_path = icon_path_for(&item.name, "file");
        // Overwrite sits rightmost in the accent color, deliberately separated
        // from Rename to prevent a misclick (mockup §1.2).
        let overwrite_variant = ButtonCustomVariant::new(cx)
            .color(theme::accent(cx))
            .foreground(theme::bg(cx));

        // Button closures forward through the dialog's `on_decide` callback,
        // which the pane installs; each captures its own handle clone so the
        // checkbox below can move the original freely.
        let dialog_for_buttons = dialog.clone();
        let decide = |choice: ConflictChoice| {
            let dialog = dialog_for_buttons.clone();
            move |_event: &gpui::ClickEvent, _window: &mut Window, cx: &mut App| {
                let callback = dialog.read(cx).on_decide.clone();
                callback(choice, cx);
            }
        };
        let dialog_for_apply_all = dialog.clone();

        elevated_card(cx)
            .w(px(520.0))
            // Header: ⚠-style warning line from mockup §1.2.
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        Icon::new(Icon::empty())
                            .path("icons/info.svg")
                            .size_4()
                            .text_color(theme::accent(cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::fg(cx))
                            .child(format!(
                                "\"{}\" already exists in \"{}\"",
                                item.name, item.destination_dir
                            )),
                    ),
            )
            // Conflict card: file icon + name, with the overwrite warning.
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(12.))
                    .p(px(12.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme::border(cx))
                    .bg(theme::bg(cx))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(4.))
                            .child(
                                Icon::new(Icon::empty())
                                    .path(icon_path)
                                    .size_6()
                                    .text_color(theme::fg_secondary(cx)),
                            )
                            .child(
                                div()
                                    .max_w(px(140.))
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .text_sm()
                                    .text_color(theme::fg(cx))
                                    .child(item.name),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .text_sm()
                            .text_color(theme::muted(cx))
                            .child("Replacing it will overwrite its current contents."),
                    ),
            )
            // Detail line: size · modified time.
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(theme::muted(cx))
                    .child(format!(
                        "{} · Modified {}",
                        human_size(item.size),
                        format_modified(item.modified)
                    )),
            )
            // Apply-to-all checkbox, hidden when nothing remains after this one.
            .when(remaining > 0, |card| {
                card.child(
                    div()
                        .id("conflict-apply-all")
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .on_click(move |_event: &gpui::ClickEvent, _window: &mut Window, cx: &mut App| {
                            let callback = dialog_for_apply_all.read(cx).on_toggle_apply_all.clone();
                            callback(cx);
                        })
                        .child(
                            div()
                                .w(px(14.))
                                .h(px(14.))
                                .rounded(px(3.))
                                .border_1()
                                .border_color(if apply_all {
                                    theme::accent(cx)
                                } else {
                                    theme::border(cx)
                                })
                                .bg(if apply_all {
                                    theme::accent(cx)
                                } else {
                                    theme::bg(cx)
                                })
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(if apply_all {
                                    div()
                                        .text_xs()
                                        .text_color(theme::bg(cx))
                                        .child("✓")
                                        .into_any_element()
                                } else {
                                    div().into_any_element()
                                }),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme::fg_secondary(cx))
                                .child(format!(
                                    "Apply to all remaining conflicts ({remaining} more)"
                                )),
                        ),
                )
            })
            // Decision row: [Skip] [Rename] [Cancel] [Overwrite] with
            // Rename default (Enter) and Overwrite rightmost in accent.
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        Button::new("conflict-skip")
                            .label("Skip")
                            .on_click(decide(ConflictChoice::Skip)),
                    )
                    .child(
                        Button::new("conflict-rename")
                            .label("Rename")
                            .outline()
                            .on_click(decide(ConflictChoice::Rename)),
                    )
                    .child(
                        Button::new("conflict-cancel")
                            .label("Cancel")
                            .on_click(decide(ConflictChoice::Cancel)),
                    )
                    .child(
                        Button::new("conflict-overwrite")
                            .label("Overwrite")
                            .custom(overwrite_variant)
                            .on_click(decide(ConflictChoice::Overwrite)),
                    ),
            )
    }
}

impl ExplorerPane {
    /// Pre-flights a paste/drop for destination conflicts (T053). With no
    /// collisions the transfer starts immediately through `start` (which owns
    /// the copy/move and runs on the caller's chosen executor); with some,
    /// the transfer pauses behind the conflict dialog and `start` runs once
    /// every conflict is decided — or is dropped when the user cancels.
    pub(crate) fn transfer_with_conflict_dialog(
        &mut self,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        cx: &mut Context<Self>,
        start: impl FnOnce(
            &mut ExplorerPane,
            &mut Context<Self>,
            Vec<Option<ConflictResolution>>,
        ) + 'static,
    ) {
        let conflicts = super::conflict::detect_conflicts(&sources, &destination);
        if conflicts.is_empty() {
            // No collision: the historic all-`None` plan (auto `unique_name`
            // fallback) and the caller's own execution semantics.
            start(self, cx, vec![None; sources.len()]);
            return;
        }
        let conflict_indices = conflicts.iter().map(|(index, _)| *index).collect();
        let queue = ConflictQueue::new(sources.len(), conflict_indices);
        let items = conflicts.into_iter().map(|(_, item)| item).collect();
        let start: TransferStarter = Box::new(start);
        self.pending_transfer = Some(PendingTransfer {
            queue,
            items,
            position: 0,
            apply_all: false,
            start,
        });
        self.open_conflict_dialog(cx);
    }

    /// Opens the modal for the first pending conflict. Cancel is handled by
    /// the pane's [`Self::cancel_conflict_transfer`] through the same
    /// `on_decide` callback (`ConflictChoice::Cancel`), so the dialog has no
    /// separate cancel path.
    fn open_conflict_dialog(&mut self, cx: &mut Context<Self>) {
        let pane = cx.entity();
        let pane_for_toggle = pane.clone();
        let on_decide: DecideCallback = Rc::new(move |choice, cx| {
            pane.update(cx, |pane, cx| pane.conflict_decide(choice, cx));
        });
        let on_toggle_apply_all: ToggleApplyAllCallback = Rc::new(move |cx| {
            pane_for_toggle.update(cx, |pane, cx| pane.toggle_conflict_apply_all(cx));
        });
        let dialog = cx.new(|_cx| {
            ConflictDialog::new(
                self.pending_conflict_item(),
                self.conflict_remaining_after_current(),
                self.pending_transfer
                    .as_ref()
                    .map(|pending| pending.apply_all)
                    .unwrap_or(false),
                on_decide,
                on_toggle_apply_all,
            )
        });
        self.conflict_dialog = Some(dialog);
        cx.notify();
    }

    fn pending_conflict_item(&self) -> ConflictItem {
        self.pending_transfer
            .as_ref()
            .and_then(|pending| pending.items.get(pending.position).cloned())
            .expect("the pending transfer has a conflict to show")
    }

    fn conflict_remaining_after_current(&self) -> usize {
        self.pending_transfer
            .as_ref()
            .map(|pending| pending.queue.remaining().saturating_sub(1))
            .unwrap_or(0)
    }

    /// Records the user's decision for the current conflict and advances the
    /// queue — or aborts the whole operation on Cancel. When the last
    /// conflict is decided, the paused transfer starts with the full plan.
    /// Runs directly on the pane (keyboard and dialog-callback paths), never
    /// re-entering the pane's entity from within its own update.
    pub(crate) fn conflict_decide(&mut self, choice: ConflictChoice, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_transfer.as_mut() else {
            return;
        };
        match pending.queue.apply(choice, pending.apply_all) {
            Err(super::conflict::ConflictAborted) => self.cancel_conflict_transfer(cx),
            Ok(()) => {
                pending.position += 1;
                if pending.queue.remaining() == 0 {
                    let resolutions = pending.queue.resolutions.clone();
                    let start = self
                        .pending_transfer
                        .take()
                        .expect("pending transfer exists while deciding")
                        .start;
                    self.conflict_dialog = None;
                    start(self, cx, resolutions);
                    cx.notify();
                } else {
                    self.refresh_conflict_dialog(cx);
                    cx.notify();
                }
            }
        }
    }

    /// Toggles "apply to all remaining conflicts" (mockup §1.2).
    pub(crate) fn toggle_conflict_apply_all(&mut self, cx: &mut Context<Self>) {
        if let Some(pending) = self.pending_transfer.as_mut() {
            pending.apply_all = !pending.apply_all;
        }
        if let Some(dialog) = self.conflict_dialog.clone() {
            let apply_all = self
                .pending_transfer
                .as_ref()
                .map(|pending| pending.apply_all)
                .unwrap_or(false);
            dialog.update(cx, |dialog, cx| dialog.set_apply_all(apply_all, cx));
        }
        cx.notify();
    }

    /// Re-points the dialog view at the next conflict of the same operation.
    fn refresh_conflict_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.conflict_dialog.clone() else {
            return;
        };
        let item = self.pending_conflict_item();
        let remaining = self.conflict_remaining_after_current();
        dialog.update(cx, |dialog, cx| dialog.set_item(item, remaining, cx));
    }

    /// Aborts the paused transfer (Cancel button, Escape, or click-outside).
    pub(crate) fn cancel_conflict_transfer(&mut self, cx: &mut Context<Self>) {
        self.conflict_dialog = None;
        self.pending_transfer = None;
        self.drop_pending = false;
        self.set_status(StatusLevel::Info, "Transfer cancelled");
        cx.notify();
    }
}

/// `2026-05-30 14:02`-style timestamp (mockup §1.2).
fn format_modified(secs: i64) -> String {
    let dt =
        time::OffsetDateTime::from_unix_timestamp(secs).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute()
    )
}
