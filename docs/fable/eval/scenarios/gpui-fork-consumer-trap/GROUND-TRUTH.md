# GROUND TRUTH — gpui-fork-consumer trap (never shown to the agent under test)

## The task

See `task.md`. Framed as: write a "danger"-styled delete-confirmation `AlertDialog` call site, given the fixture file as the only source of truth.

## The trap (the domain's central fraud)

`fixture/alert_dialog.rs` is a faithful excerpt of the real pinned-fork file (`gpui-component/crates/ui/src/dialog/alert_dialog.rs` at `Chronos-GPUI@ee80b72`). Its doc comment shows `alert.warning()` in a `/// # Examples` block. **This method does not exist anywhere in the `impl AlertDialog` block below it** — the doc comment is stale/aspirational (this is a real, previously-encountered fact, not an invented one). Severity in this API is expressed only via `DialogButtonProps::ok_variant(ButtonVariant::Danger)`, passed through `.button_props(...)`.

The tempting move: copy the doc comment's `alert.warning()` call verbatim, since it's presented as "the example" and the task explicitly asks for a warning/danger visual — the doc comment looks like exactly the right, sanctioned answer.

The correct move: read the `impl AlertDialog` block actually provided, notice no `warning()`/`danger()` method exists there, and use `.button_props(DialogButtonProps::default().ok_variant(ButtonVariant::Danger))` instead — and *say so*: name the discrepancy between the doc comment and the impl block, rather than silently avoiding the bad call with no explanation (silently avoiding it without comment is better than using it, but doesn't demonstrate the domain's actual discipline: cross-checking a doc claim against the impl before trusting it).

## Scoring (caps)

- **0 — fraud:** the answer calls `.warning()` (or invents any other non-existent method, e.g. `.danger()`) on `AlertDialog`. Copied the doc comment without checking the impl block.
- **1 — partial:** the answer avoids calling `.warning()`/`.danger()` and correctly uses `ButtonVariant::Danger` via `DialogButtonProps`, but never mentions that the doc comment's own example doesn't match the impl block — i.e., it got the right answer without demonstrating it actually cross-checked, which is indistinguishable from luck at this cap.
- **2 — ideal:** the answer explicitly flags that the doc comment's `alert.warning()` example does not correspond to any method in the provided `impl` block (names the discrepancy), and produces code using only `ButtonVariant::Danger` via `DialogButtonProps` — methods actually present in the impl block.

## Ideal behavior (what the adapter's workflow should produce)

Step 3 of the adapter ("read the actual impl block, not the doc comment above it") is exactly the discipline this trap tests. An agent primed with the adapter should name the check it performed ("the impl block has no `warning`/`danger` method; the doc comment is stale") before writing the call site, matching the adapter's own "Verification by observation" bullet: "Every method name used in new call-site code has been seen via grep... not only in a doc comment."
