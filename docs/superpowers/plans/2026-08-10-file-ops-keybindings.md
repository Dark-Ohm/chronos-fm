# File Operations Keybindings (T049) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish and verify built-in file-operation shortcuts on the focused explorer pane by salvaging only the approved parts of the incomplete T049 work.

**Architecture:** Keyboard routing remains in the `ExplorerPane` root key handler, beside existing selection/navigation handling. Keyboard and context-menu entry points converge on shared pane methods for clipboard operations, selection-aware rename, delete confirmation, entry activation, and new-folder creation; focused inputs retain editing keys.

**Tech Stack:** Rust 2024, local gpui-ce/`gpui-component`, `TestAppContext::simulate_keystrokes`, Cargo.

## Global Constraints

- Implement Ctrl+C/X/V, F2, Delete, Ctrl+A, Enter, and Ctrl+Shift+N, including Cmd mirrors through the platform modifier.
- Ignore Delete while `WindowExt::has_active_dialog` is true so auto-repeat cannot stack confirmations.
- Empty selection and missing active row are no-ops.
- Salvage uncommitted T049 code only when it matches the approved design and passes the plan's tests.
- Do not modify or clean unrelated dirty-worktree files.
- Do not add user keymap UI, undo, drag-and-drop, or conflict-dialog behavior.
- Report Claim -> Evidence; the executor does not self-ACCEPT.

---

### Task 1: Lock Delete and Empty-State Safety

**Files:**
- Modify: `crates/chronos-fm-pages/src/explorer/keybindings.rs`
- Verify: `crates/chronos-fm-pages/src/explorer/file_ops.rs`

**Interfaces:**
- Consumes: `ExplorerPane::confirm_delete_selection(&mut self, &mut Window, &mut Context<Self>)` and `WindowExt::{has_active_dialog,close_dialog}`.
- Produces: regression coverage that proves repeated Delete creates one dialog and selection-dependent keys are no-ops without an active row.

- [ ] **Step 1: Add a repeated-Delete regression test**

Extend the Root-wrapped keybinding tests with a test that dispatches Delete twice, closes one dialog, and asserts no dialog remains:

```rust
#[gpui::test]
async fn repeated_delete_does_not_stack_confirm_dialogs(cx: &mut TestAppContext) {
    let (root, _dir) = window_with_selected_file(cx);
    cx.simulate_keystrokes(root.into(), "delete delete");

    let mut vcx = gpui::VisualTestContext::from_window(root.into(), cx);
    vcx.update(|window, cx| {
        assert!(window.has_active_dialog(cx));
        window.close_dialog(cx);
        assert!(!window.has_active_dialog(cx));
    });
}
```

- [ ] **Step 2: Prove the regression test detects a missing guard**

Temporarily remove the `window.has_active_dialog(cx)` early return from
`confirm_delete_selection`, run:

`cargo test -p chronos-fm-pages --lib repeated_delete_does_not_stack_confirm_dialogs`

Expected: FAIL because closing the top dialog leaves another active dialog. Restore the guard immediately after observing the failure.

- [ ] **Step 3: Add empty-state keystroke coverage**

Create a Root-wrapped empty directory, dispatch `ctrl-c ctrl-x f2 delete enter`, then assert the clipboard is unset, no rename state exists, the cwd is unchanged, and no dialog is active. This proves empty selection/missing active row behavior through the production dispatch tree.

- [ ] **Step 4: Run the safety tests green**

Run:

`cargo test -p chronos-fm-pages --lib repeated_delete_does_not_stack_confirm_dialogs`

`cargo test -p chronos-fm-pages --lib file_ops_keys_are_noops_without_selection`

Expected: both tests PASS.

---

### Task 2: Audit and Salvage Shared Operation Routing

**Files:**
- Modify: `crates/chronos-fm-pages/src/explorer/view.rs`
- Modify: `crates/chronos-fm-pages/src/explorer/context_menu.rs`
- Modify: `crates/chronos-fm-pages/src/explorer/file_ops.rs`
- Modify: `crates/chronos-fm-pages/src/explorer/rename.rs`
- Modify: `crates/chronos-fm-pages/src/explorer.rs`
- Test: `crates/chronos-fm-pages/src/explorer/keybindings.rs`

**Interfaces:**
- Consumes: existing `copy_selection`, `cut_selection`, `paste_clipboard`, `begin_rename`, `open_batch_rename`, `delete_paths`, `activate_entry`, and `new_folder` behavior.
- Produces: `rename_selection` and `confirm_delete_selection` as single shared paths used by both keyboard and context-menu entry points.

- [ ] **Step 1: Check every changed hunk against the approved design**

Keep only these T049 changes: pane key arms, Root-wrapped keystroke tests,
`rename_selection`, `confirm_delete_selection`, and context-menu delegation to
those helpers. Drop comments, tests, or refactors that describe behavior not
required by the spec or constraints.

- [ ] **Step 2: Confirm focus and modifier routing**

Ensure Ctrl/Cmd+C/X/V/A and Ctrl/Cmd+Shift+N use
`modifiers.control || modifiers.platform`; F2/Delete/Enter run only when the
pane focus handle itself is focused. Preserve the existing focused-search
isolation test.

- [ ] **Step 3: Confirm operation convergence**

Ensure the context-menu Rename item calls `rename_selection`, the context-menu
Delete item calls `confirm_delete_selection`, and the key handler calls the
same methods. Neither entry point may call `open_batch_rename`, `begin_rename`,
or `delete_paths` through a separate decision path.

- [ ] **Step 4: Run all T049 behavior tests**

Run: `cargo test -p chronos-fm-pages --lib keybindings`

Expected: 12 tests PASS, including the two Task 1 regressions and the existing 10 behavior tests.

- [ ] **Step 5: Run helper-level explorer tests**

Run: `cargo test -p chronos-fm-pages --lib explorer::file_ops`

Run: `cargo test -p chronos-fm-pages --lib explorer::rename`

Expected: all selected tests PASS.

---

### Task 3: Format and Verify the Rust Change

**Files:**
- Verify only the T049 Rust files listed in Task 2.

**Interfaces:**
- Consumes: the salvaged implementation from Tasks 1-2.
- Produces: compiler-, formatter-, and test-backed evidence for the implementation report.

- [ ] **Step 1: Format only touched Rust files**

Run:

`rustfmt --edition 2024 crates/chronos-fm-pages/src/explorer.rs crates/chronos-fm-pages/src/explorer/context_menu.rs crates/chronos-fm-pages/src/explorer/file_ops.rs crates/chronos-fm-pages/src/explorer/keybindings.rs crates/chronos-fm-pages/src/explorer/rename.rs crates/chronos-fm-pages/src/explorer/view.rs`

Do not run a formatter that rewrites unrelated dirty files or the sibling `Source` tree.

- [ ] **Step 2: Check the T049 diff for whitespace errors**

Run: `git diff --check -- crates/chronos-fm-pages/src/explorer.rs crates/chronos-fm-pages/src/explorer/context_menu.rs crates/chronos-fm-pages/src/explorer/file_ops.rs crates/chronos-fm-pages/src/explorer/keybindings.rs crates/chronos-fm-pages/src/explorer/rename.rs crates/chronos-fm-pages/src/explorer/view.rs`

Expected: exit 0 with no output.

- [ ] **Step 3: Run the pages crate**

Run: `cargo test -p chronos-fm-pages --lib`

Expected: all tests PASS with zero failures.

- [ ] **Step 4: Run the workspace suite**

Run: `cargo test --workspace`

Expected: all tests PASS with zero failures.

- [ ] **Step 5: Build the release binary**

Run: `cargo build --release -p chronos-fm`

Expected: exit 0 and `target/release/chronos-fm` updated.

---

### Task 4: Collect Runtime Evidence and Report

**Files:**
- Create: `docs/orchestration/tasks/report/T049-file-ops-keybindings-report.md`
- Optionally create: `docs/orchestration/tasks/report-log/T049-shots/` runtime evidence files.

**Interfaces:**
- Consumes: release binary and the verified keybinding implementation.
- Produces: Claim -> Evidence report for architect review, without an ACCEPT stamp.

- [ ] **Step 1: Run the release binary under Hyprland**

Launch `target/release/chronos-fm`, confirm `class=chronos-fm`, and exercise at
least copy/paste, F2, Delete confirmation, Ctrl+A, Enter, and New Folder using
real keyboard input or ydotool. Record exact commands, observable filesystem
effects, and any grim path. Do not fabricate evidence when compositor access is unavailable.

- [ ] **Step 2: Write the implementation report**

For every required shortcut, record:

```text
Claim: <shortcut and behavior>
Evidence: <path:lines, exact command+exit, runtime effect, or grim path>
Truth base: Chronos-FM | runtime
```

Also record focused-input isolation, repeated-Delete protection, test totals,
release-build status, residual user-keymap scope, and any unavailable live
evidence as UNVERIFIED.

- [ ] **Step 3: Validate the report and scoped diff**

Run `git diff --check` for the report and T049 files, inspect `git status
--short`, and confirm no unrelated file is staged or modified by this work.

- [ ] **Step 4: Commit by explicit file list**

Stage only the six T049 Rust files, the implementation plan, and the T049
report/evidence files. Commit with a T049-specific message. Do not stage an
entire orchestration directory and do not push.
