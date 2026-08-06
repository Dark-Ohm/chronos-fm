# Batch Rename (T006) — Design Document

**Date:** 2026-08-06
**Ticket:** `docs/orchestration/tasks/active/T006-batch-rename.md`
**Project:** chronos-fm (`crates/chronos-fm-pages` explorer, `crates/chronos-fm-services`)
**Scope:** A batch-rename dialog for renaming multiple selected files with a
pattern, backed by a live "was → will be" preview before applying.

---

## 1. Problem & Goal

Single-file inline rename exists (b1 plan: `begin_rename`/`commit_rename`/
`cancel_rename`, backed by `ops::rename_in_place`). There is no way to rename
many files at once. This design adds:

- A modal **Batch Rename** dialog: pattern field (mini-DSL), find/replace
  fields, counter start, and a **live preview list** ("was → will be") shown
  *before* anything is applied — mass rename without a preview is dangerous
  (ticket requirement).
- Collision handling: targets that collide with another renamed file are
  auto-resolved via the already-tested `ops::unique_name`/`would_conflict`
  helpers, and the resolution is visible in the preview (user decision).
- Apply via the existing, already-tested `ops::rename_in_place`.

Non-goals for v1: renaming with regex backreferences, reordering/reindexing
via drag-drop, renaming across directories, undo.

## 2. Entry point

**Single selection** keeps today's inline rename (b1/b3/b4 behavior — no
change). **Multi-selection (2+ files)** makes the row context menu's
**Rename** item (arriving with b3/b4) open the Batch Rename dialog instead of
inline rename. This gives one unified "Rename" affordance: one file → inline,
many files → dialog.

The dialog is a self-contained modal entity (same pattern as
`PropertiesDialog` in `explorer/properties.rs`): stored on the pane as
`batch_rename: Option<Entity<BatchRenameDialog>>`, rendered as an
absolutely-positioned overlay with a scrim, closed by Escape / Cancel /
click-outside / Apply.

## 3. Pattern mini-DSL

A tiny own parser (~60 lines, **no new dependency** — user decision, matches
the ticket's "свой мини-DSL vs готовый крейт" research item). Placeholders:

| Placeholder | Meaning |
|---|---|
| `{name}` | Original stem (filename without the last extension) after find/replace |
| `{ext}` | Original last extension, without the dot (empty if the file has none) |
| `{n}` | Counter, starting at the user-set start value (default `1`) |
| `{n:W}` | Counter zero-padded to width `W`, e.g. `{n:3}` → `001`, `002`, … |

- All other characters are literal.
- A literal `{`/`}` requires `{{`/`}}` escaping.
- An unknown placeholder (e.g. `{q}`) is treated as an error: the preview
  shows an error banner and Apply is disabled — never silently mis-renames.
- The ticket's example `{name}_{n}.{ext}` works as-is; `{n}` and `{n:3}`
  cover counter start/width numbering (the ticket's "нумерация с заданным
  стартом/шириной").

**Find/replace** applies to the original stem *before* `{name}` substitution:
find `foo` → replace `bar` turns `foobar.txt` → stem `barbar`. An empty find
field means no replacement.

**Stem/extraction rule (decided):** last extension only. `archive.tar.gz` →
stem `archive`, ext `gz`. Matches most file managers' "extension" semantics.

## 4. Architecture

### 4.1 Pure logic — `chronos-fm-services/src/fs/batch_rename.rs` (new)

Everything that can be tested without a window lives in services as pure
functions:

- `Template::parse(pattern: &str) -> Result<Template, PatternError>` — tokenize
  the mini-DSL once (not per-file).
- `Template::render(&self, stem: &str, ext: &str, counter: u64) -> String` —
  substitute placeholders.
- `apply_find_replace(stem: &str, find: &str, replace: &str) -> String`.
- `split_stem_ext(name: &str) -> (String, String)` — last-extension rule.
- `build_preview(entries: &[FileEntryDto], template: &Template, find: &str,
  replace: &str, start: u64) -> Vec<RenamePreview>` — pure, no I/O: for each
  entry in selection order compute old → new; resolve collisions with the
  existing `ops::unique_name`/`would_conflict` over the *batch's own* target
  set (and mark the entry as `ResolvedCollision` so the preview shows it).
- `RenamePreview { old_name, new_name, status: Ok | ResolvedCollision }`.

Why a new module rather than extending `ops.rs`: `ops` is single-file
filesystem operations with disk I/O; batch preview logic is pure name
machinery — same split the repo already makes between `listing` and `ops`.

### 4.2 Dialog — `crates/chronos-fm-pages/src/explorer/batch_rename.rs` (new)

`BatchRenameDialog` — modal entity modeled on `PropertiesDialog`:

- State: `entries: Vec<FileEntryDto>` (cloned selection, in row order),
  `pattern_input`, `find_input`, `replace_input`, `start_input`
  (`gpui_component::input::InputState` each, like the search bar), a
  recomputed `preview: Vec<RenamePreview>`, and `last_error: Option<String>`.
- Recomputes the preview on every keystroke (pure `build_preview` — cheap:
  hundreds of entries max, per config `DIR_LISTING_LIMIT`).
- **Apply** button: disabled when the preview is empty, contains an error, or
  produces no actual changes. On apply, calls `ops::rename_in_place` per entry
  (from the *resolved* new names in preview order), then the pane reloads and
  reports failures via the existing `set_status(StatusLevel::Error, …)`
  pattern — matching every other explorer error path.
- **Rendering:** `elevated_card` + `section_header` patterns (like
  `properties.rs`), a preview list with "old → new" rows (collision-resolved
  rows tinted/dimmed with a note), footer with Apply/Cancel buttons.

### 4.3 Pane wiring

- `ExplorerPane.batch_rename: Option<Entity<BatchRenameDialog>>` (state.rs).
- `open_batch_rename(entries, window, cx)` / `close_batch_rename(cx)` /
  `commit_batch_rename(cx)` on the pane (new `batch_rename.rs` impl module in
  pages, next to `rename.rs` from b1).
- Called from the b3/b4 context-menu "Rename" handler when
  `selected_paths().len() > 1` (single stays inline). Since b3/b4 are
  still active tasks, this call site is stubbed in the plan and wired when
  b3/b4 land.
- View wiring: overlay + scrim in `view.rs`, same shape as the existing
  `render_properties_dialog` / `render_context_menu`.

## 5. Error handling

- **Pattern parse error** → inline error banner in the dialog, Apply disabled.
- **Apply failure per file** (permission, disappeared, cross-device) →
  collected and shown via `set_status(StatusLevel::Error, …)` after `reload()`
  (the repo rule: report the error *after* `reload()`, or the success path
  clears it — see the 2026-07-21 context-menu plan, Global Constraints).
- **Collision** → not an error: auto-resolved in preview (user decision),
  visibly marked, no silent surprise.
- **No-op** (pattern changes nothing) → Apply disabled with a hint.

## 6. Testing

- **Unit tests (services, `batch_rename.rs`)** — the bulk of coverage, all
  pure functions, no window needed:
  - `Template::parse`: all placeholders, `{{`/`}}` escape, unknown
    placeholder → error, empty pattern.
  - `Template::render`: `{name}`, `{ext}`, `{n}`, `{n:3}`, literal text,
    dotted filenames (`archive.tar.gz` → stem `archive`, ext `gz`), no-extension.
  - `apply_find_replace`: empty find, find with no match, replace in middle.
  - `build_preview`: numbering from custom start, width padding, collision
    resolution (two files → same target: second gets `unique_name` treatment,
    status `ResolvedCollision`), ordering in selection order.
- **Dialog tests (pages)** only if the window-test harness allows (the
  `new_explorer_for_tests` helper from b1); otherwise dialog behavior is
  verified via the pure preview functions + manual smoke, disclosed in the
  report as unverified-by-screenshot (repo `verification-before-completion`
  convention).
- No GUI screenshot testing exists for this crate; called out explicitly in
  the report.

## 7. Dependency / sequencing

**Hard gate (ticket rule):** no implementation plan for T006 until **b1**
(`docs/agents/active/b1-clipboard-rename-foundation.md`) is in
`report-log/`/`done/` — the dialog and its tests reuse b1's
`begin_rename`/`commit_rename`/`cancel_rename` infra and the
`new_explorer_for_tests` harness. b1 is still **active** as of this spec
(`docs/agents/report/` empty; `clipboard.rs`/`rename.rs` absent from
`explorer/`). The spec is deliberately written against only the *existing*
services API (`ops::rename_in_place`, `ops::unique_name`, `ops::would_conflict`
— all present and tested) so it stays valid when b1 lands.

## 8. Non-Goals (this pass)

- Regex backreferences / full regex find-replace.
- Per-file manual override inside the preview.
- Rename with directory components or cross-directory moves.
- Undo / recycle-bin for renames.
- Async preview computation (selection is bounded by `DIR_LISTING_LIMIT`;
  pure function, recomputed per keystroke is fine).
