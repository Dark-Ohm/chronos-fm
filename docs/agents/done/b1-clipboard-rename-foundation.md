# Task b1-clipboard-rename-foundation

**Status:** active
**Plan ref:** `docs/superpowers/plans/2026-07-21-explorer-context-menu.md`, Tasks 1, 2, 3
**Repo root:** `/home/neo/projects/chronos-ecosystem/Chronos-FM`
**Depends on:** none. **Parallel:** none — this is the foundation task; b2 depends on it, b3/b4 depend on b1+b2.

## Report (MANDATORY)

Write to the **inbox** (not archive):

- **Inbox:** `docs/agents/report/b1-clipboard-rename-foundation-report.md`
- **Accepted report (archive):** `docs/agents/report-log/b1-clipboard-rename-foundation-report.md`

Report format:

```markdown
# b1-clipboard-rename-foundation — report

## Outcome
PASS | FAIL | BLOCKED

## What changed
- paths created/modified

## Verification (commands + observed output)
```
paste cargo test / cargo build output
```

## Risks / follow-ups
```

After orchestrator accept: report → `docs/agents/report-log/`, this brief → `docs/agents/done/` (or `docs/agents/rejected/` if declined).

## Goal

Add the `FileClipboard` global (copy/cut/paste state, shared across panes), register it at app startup, and add inline-rename state + pane methods to `ExplorerPane`. This is pure state/logic — no UI rendering changes in this task.

Read the plan file's Tasks 1, 2, 3 in full before starting — they contain the exact code for every step, including the two new test-harness helpers (`new_explorer_for_tests`, `pub(crate) mod tests;`) that later tasks (and your own tests) depend on.

## Deliverables (touch only these)

| Path | Role |
|---|---|
| `crates/chronos-fm-pages/src/explorer/clipboard.rs` | NEW — `FileClipboard` global, `ClipboardMode`, `init`/`set_copy`/`set_cut`/`clear`/`current` |
| `crates/chronos-fm-pages/src/explorer/rename.rs` | NEW — `begin_rename`/`commit_rename`/`cancel_rename` on `ExplorerPane` |
| `crates/chronos-fm-pages/src/explorer/state.rs` | add `renaming: Option<(usize, Entity<InputState>)>` field only |
| `crates/chronos-fm-pages/src/explorer/tests.rs` | add `new_explorer_for_tests` helper; change `mod tests;` visibility in `explorer.rs` |
| `crates/chronos-fm-pages/src/explorer.rs` | add `pub mod clipboard;`, `mod rename;`, `pub(crate) mod tests;` |
| `crates/chronos-fm/src/app.rs` | one line: call `chronos_fm_pages::explorer::clipboard::init(app);` |

Do not touch `file_ops.rs`, `row.rs`, `list.rs`, `grid.rs`, `listing.rs` — those belong to b2/b3/b4.

## Verification (required)

```bash
cd /home/neo/projects/chronos-ecosystem/Chronos-FM
cargo test -p chronos-fm-pages clipboard::
cargo test -p chronos-fm-pages rename::
cargo build -p chronos-fm
```

All must PASS. Paste full output in the report.

## Constraints

- No `std::fs::*` calls directly — filesystem mutation goes through `chronos_fm_services::fs::ops` only (already used in `rename.rs` per the plan).
- Every new `pub` item needs a doc comment (`missing_docs = "warn"` at workspace level).
- `clippy::unwrap_used`/`expect_used` are warn-level outside `#[cfg(test)]` — fine in test code, avoid elsewhere.
- No commit required; if you commit, no AI trailers unless the repo's own commit convention already uses them (check `git log` first).

## Out of scope

Copy/cut/paste/new-folder/delete pane methods (b2), any context-menu UI wiring in row/list/grid (b3, b4).
