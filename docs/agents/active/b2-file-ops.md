# Task b2-file-ops

**Status:** active
**Plan ref:** `docs/superpowers/plans/2026-07-21-explorer-context-menu.md`, Task 4
**Repo root:** `/home/neo/projects/chronos-ecosystem/Chronos-FM`
**Depends on:** b1-clipboard-rename-foundation (needs `FileClipboard` global + `begin_rename`; must be accepted/merged first — do not start until b1's `clipboard.rs` and `rename.rs` exist on your branch). **Parallel:** none against b1; b3/b4 depend on this task too, so treat it as blocking for them.

## Report (MANDATORY)

Write to the **inbox** (not archive):

- **Inbox:** `docs/agents/report/b2-file-ops-report.md`
- **Accepted report (archive):** `docs/agents/report-log/b2-file-ops-report.md`

Report format:

```markdown
# b2-file-ops — report

## Outcome
PASS | FAIL | BLOCKED

## What changed
- paths created/modified

## Verification (commands + observed output)
```
paste cargo test output
```

## Risks / follow-ups
```

After orchestrator accept: report → `docs/agents/report-log/`, this brief → `docs/agents/done/` (or `docs/agents/rejected/` if declined).

## Goal

Add the five filesystem-mutating pane methods (`copy_selection`, `cut_selection`, `paste_clipboard`, `new_folder`, `delete_paths`), each backed by the existing, already-tested `chronos_fm_services::fs::ops` functions. Read the plan file's Task 4 in full — it has the exact code for every method and all four tests, including the "reload() before set_status on error" ordering rule explained in the plan's Global Constraints (reload() unconditionally clears the status bar on success, so an error set before reload() gets silently erased — always call reload() first, then set_status only on error).

## Deliverables (touch only these)

| Path | Role |
|---|---|
| `crates/chronos-fm-pages/src/explorer/file_ops.rs` | NEW — the five pane methods + their tests |
| `crates/chronos-fm-pages/src/explorer.rs` | one line: `mod file_ops;` |
| `crates/chronos-fm-pages/src/explorer/navigation.rs` | add `#[cfg(test)] pub(crate) fn change_dir_for_test` only — do not touch `change_dir`/`go_back`/`go_forward`/`activate_entry` |

Do not touch `clipboard.rs`, `rename.rs`, `state.rs`, `row.rs`, `list.rs`, `grid.rs`, `listing.rs`.

## Verification (required)

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
cargo test -p chronos-fm-pages file_ops::
cargo test -p chronos-fm-pages
```

All must PASS (the second command is the full crate suite — confirms b1's tests still pass alongside yours). Paste full output in the report.

## Constraints

- No `std::fs::*` calls directly — use `chronos_fm_services::fs::ops` (`copy_path`, `move_path`, `create_dir`, `trash_path`, `unique_name`) exactly as the plan shows.
- Do not unit-test `trash_path`'s success path against a real file — it depends on a desktop trash service that may not exist in a headless test runner. Only test its error path (missing source path), matching the plan's own rationale (`ops.rs`'s own test suite does the same: it tests `delete_permanent` but not `trash_path`).
- Every new `pub`/`pub(crate)` item needs a doc comment.

## Out of scope

Clipboard global itself, rename state/methods (both b1), any context-menu UI wiring (b3, b4).
