# Batch Rename (T006) Implementation Plan

> **For agentic workers:** implement this plan task-by-task; checkboxes track progress.
> **Spec:** `docs/superpowers/specs/2026-08-06-batch-rename-design.md` (read first — this plan is the executable form of it).

**Goal:** A modal **Batch Rename** dialog for renaming multiple selected files via a
pattern mini-DSL, with a live "was → will be" preview before anything is applied.
Pure name machinery lives in `chronos-fm-services::fs::batch_rename` (fully
unit-testable); the dialog + pane wiring live in `chronos-fm-pages`.

**Gate status:** b1 (`clipboard-rename-foundation`) has **landed** (report in
`docs/agents/report/`, its `new_explorer_for_tests` harness and
`renaming` field are in the tree) — the b1 gate on this plan is closed.
The **entry-point call site** (multi-selection → Rename opens this dialog) belongs
to b3/b4's context menu, which is still active; Task 5 stubs it.

## Global Constraints

- `unsafe_code = "deny"`, `missing_docs = "warn"` (every `pub` item needs a doc comment),
  `clippy::unwrap_used`/`expect_used = "warn"` (allowed in `#[cfg(test)]` only).
- `disallowed-methods` bans `std::fs::read`/`write`/`read_to_string` in app code — renames go
  through `chronos_fm_services::fs::ops::rename_in_place`; test modules carry
  `#![allow(clippy::disallowed_methods)]` (repo convention, see `clippy.toml`).
- Error-report ordering rule: any method that calls `self.reload()` must `set_status` a failure
  *after* `reload()`, or the success path silently erases the error.
- Dialog overlay pattern: mirror `render_properties_dialog`
  (`crates/chronos-fm-pages/src/explorer/view.rs:175`) — absolutely-positioned scrim +
  centered card, Escape / click-outside to close.
- `InputState` usage: mirror `search_bar.rs` (it already builds an `InputState`, binds it to an
  `Input` widget, and reads `.text()`); gpui-component event-hook names must be verified against
  the pinned `Chronos-GPUI@ee80b72` checkout before writing them (same rule the context-menu plan
  used — don't assume from docs).
- Test harness: `new_explorer_for_tests(cx, cwd)` from b1 (`explorer/tests.rs`) — roots a pane at
  a tempdir and registers the clipboard global (note: it resets the clipboard).

---

## Task 1: Pure batch-rename logic — `chronos-fm-services/src/fs/batch_rename.rs`

**Files:**
- Create: `crates/chronos-fm-services/src/fs/batch_rename.rs`
- Modify: `crates/chronos-fm-services/src/chronos_fm_services.rs` (add `pub mod batch_rename;`
  next to the other `fs` submodule declarations)

**Interfaces:** consumes `ops::would_conflict(&Path) -> bool`, `ops::unique_name(dir, name) ->
String`, `listing::FileEntryDto` (all existing, tested). Produces the pure preview API the dialog
calls; nothing here touches the window or does I/O beyond `ops`'s stat-only `would_conflict`.

- [ ] **Step 1: Write the module**

```rust
//! Batch-rename pattern machinery: a tiny mini-DSL for rendering new file names
//! from placeholders, plus pure preview computation with collision resolution.
//! No I/O here — disk mutation lives in `ops`; this module only *names* things.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::listing::FileEntryDto;
use super::ops;

/// Status of one previewed rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameStatus {
    /// The target is free and will be applied as computed.
    Ok,
    /// The computed target collided (with another batch target or an existing
    /// file on disk) and was auto-resolved to a unique name.
    ResolvedCollision,
}

/// One row of the live preview: what a file is now and what it will become.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePreview {
    /// The current file name.
    pub old_name: String,
    /// The name it will be renamed to (already collision-resolved).
    pub new_name: String,
    /// Whether the target needed collision resolution.
    pub status: RenameStatus,
}

/// Parse errors for the pattern mini-DSL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// An unknown placeholder such as `{q}`.
    UnknownPlaceholder(String),
    /// A `{` without a matching `}`, or a stray `}`.
    UnclosedBrace,
    /// `{n:W}` with a non-numeric width.
    InvalidWidth(String),
}

/// A parsed batch-rename pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Literal(String),
    Name,
    Ext,
    Counter { width: Option<usize> },
}

/// Splits `name` into (stem, extension) by the last-extension rule:
/// `archive.tar.gz` → (`archive`, `gz`); names with no extension (or a leading
/// dot, e.g. `.hidden`) yield an empty extension.
pub fn split_stem_ext(name: &str) -> (String, String) {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => {
            (stem.to_string(), ext.to_string())
        }
        _ => (name.to_string(), String::new()),
    }
}

/// Applies a find/replace to `stem`; an empty `find` is a no-op.
pub fn apply_find_replace(stem: &str, find: &str, replace: &str) -> String {
    if find.is_empty() {
        stem.to_string()
    } else {
        stem.replace(find, replace)
    }
}

impl Template {
    /// Parses the pattern mini-DSL. Placeholders: `{name}`, `{ext}`, `{n}`,
    /// `{n:W}` (zero-padded counter to width `W`). Literal braces are
    /// `{{`/`}}`. Unknown placeholders and malformed braces are errors — never
    /// silently mis-rename (the dialog disables Apply on `Err`).
    pub fn parse(pattern: &str) -> Result<Template, PatternError> {
        let mut tokens = Vec::new();
        let mut literal = String::new();
        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '{' if chars.get(i + 1) == Some(&'{') => {
                    literal.push('{');
                    i += 2;
                }
                '}' if chars.get(i + 1) == Some(&'}') => {
                    literal.push('}');
                    i += 2;
                }
                '{' => {
                    let Some(rel) = chars[i + 1..].iter().position(|&c| c == '}') else {
                        return Err(PatternError::UnclosedBrace);
                    };
                    let close = i + 1 + rel;
                    let body: String = chars[i + 1..close].iter().collect();
                    if !literal.is_empty() {
                        tokens.push(Token::Literal(std::mem::take(&mut literal)));
                    }
                    tokens.push(match body.as_str() {
                        "name" => Token::Name,
                        "ext" => Token::Ext,
                        "n" => Token::Counter { width: None },
                        _ if body.starts_with("n:") => {
                            let width: usize = body[2..]
                                .parse()
                                .map_err(|_| PatternError::InvalidWidth(body[2..].to_string()))?;
                            Token::Counter { width: Some(width) }
                        }
                        _ => return Err(PatternError::UnknownPlaceholder(body)),
                    });
                    i = close + 1;
                }
                '}' => return Err(PatternError::UnclosedBrace),
                c => {
                    literal.push(c);
                    i += 1;
                }
            }
        }
        if !literal.is_empty() {
            tokens.push(Token::Literal(literal));
        }
        Ok(Template { tokens })
    }

    /// Renders one new file name from the parsed tokens.
    pub fn render(&self, stem: &str, ext: &str, counter: u64) -> String {
        let mut out = String::new();
        for token in &self.tokens {
            match token {
                Token::Literal(s) => out.push_str(s),
                Token::Name => out.push_str(stem),
                Token::Ext => out.push_str(ext),
                Token::Counter { width } => match width {
                    Some(w) => out.push_str(&format!("{counter:0w$}")),
                    None => out.push_str(&counter.to_string()),
                },
            }
        }
        out
    }
}

/// Returns a name not present in `taken`, bumping `name (2).ext`, `name (3).ext`,
/// … from the original name (mirrors `ops::unique_name`'s naming scheme).
fn unique_within(name: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(name) {
        return name.to_string();
    }
    let (stem, ext) = split_stem_ext(name);
    let mut n = 2;
    loop {
        let candidate = if ext.is_empty() {
            format!("{stem} ({n})")
        } else {
            format!("{stem} ({n}).{ext}")
        };
        if !taken.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Resolves `rendered` against the batch's already-taken targets and (only for
/// names that actually change) against existing files in the entry's directory.
/// Returns `(final_name, needed_resolution)`.
fn resolve_target(
    rendered: &str,
    entry_path: &str,
    taken: &HashSet<String>,
) -> (String, bool) {
    let parent: PathBuf = Path::new(entry_path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut n = 2u64;
    loop {
        let occupied = taken.contains(rendered)
            || (rendered != Path::new(entry_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                && ops::would_conflict(&parent.join(rendered)));
        if !occupied {
            return (rendered.to_string(), n > 2);
        }
        let (stem, ext) = split_stem_ext(rendered);
        rendered = if ext.is_empty() {
            format!("{stem} ({n})")
        } else {
            format!("{stem} ({n}).{ext}")
        };
        n += 1;
    }
}
```

> Note: `resolve_target`'s `rendered` rebinding inside the loop is a plan-level
> sketch; implement with an owned `String` accumulator (`let mut candidate = rendered.to_string();`
> then `candidate = bumped`), keeping the bump derived from the *original* `rendered`
> so the sequence is `name (2).ext`, `name (3).ext`, …. Write the loop with a
> `loop { let occupied = …candidate…; if !occupied { return (candidate, resolved); } … }`
> shape and a `resolved: bool` flag. The unit tests below pin the exact semantics.

- [ ] **Step 2: `build_preview` (the API the dialog calls)**

```rust
/// Computes the full preview for a batch in selection order. Targets are
/// resolved against the batch's own earlier targets and against existing files
/// on disk (only for names that actually change), so two files can never land
/// on the same name; any resolution is surfaced via `RenameStatus`.
pub fn build_preview(
    entries: &[FileEntryDto],
    template: &Template,
    find: &str,
    replace: &str,
    start: u64,
) -> Vec<RenamePreview> {
    let mut previews = Vec::with_capacity(entries.len());
    let mut taken: HashSet<String> = HashSet::new();

    for (ix, entry) in entries.iter().enumerate() {
        let (stem, ext) = split_stem_ext(&entry.name);
        let stem = apply_find_replace(&stem, find, replace);
        let mut candidate = template.render(&stem, &ext, start + ix as u64);
        let mut status = RenameStatus::Ok;

        if taken.contains(&candidate) {
            candidate = unique_within(&candidate, &taken);
            status = RenameStatus::ResolvedCollision;
        } else if candidate != entry.name {
            let parent = Path::new(&entry.path)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            if ops::would_conflict(&parent.join(&candidate)) {
                candidate = unique_within(&candidate, &taken);
                status = RenameStatus::ResolvedCollision;
            }
        }

        taken.insert(candidate.clone());
        previews.push(RenamePreview {
            old_name: entry.name.clone(),
            new_name: candidate,
            status,
        });
    }
    previews
}
```

- [ ] **Step 3: Wire the module** — in `crates/chronos-fm-services/src/chronos_fm_services.rs`, next
to the other `fs` submodules:

```rust
pub mod fs {
    pub mod batch_rename;
    pub mod listing;
    pub mod ops;
}
```
(Adjust to the file's actual `mod fs { ... }` shape — it already declares `listing`/`ops`.)

- [ ] **Step 4: Unit tests** (`#[cfg(test)] mod tests` with `#![allow(clippy::disallowed_methods)]`;
fixtures written via `std::fs` in a `tempfile::tempdir`, exactly like `ops.rs`'s tests):

  - `parse_accepts_all_placeholders_and_literals` — `{name}_{n:3}.{ext}` parses; render of
    (`stem="photo"`, `ext="jpg"`, `counter=7`) → `photo_007.jpg`.
  - `parse_handles_escaped_braces` — `{{lit}}` → literal `{lit}` (render has no placeholders).
  - `parse_rejects_unknown_placeholder` — `{q}` → `Err(PatternError::UnknownPlaceholder("q"))`.
  - `parse_rejects_unclosed_and_stray_braces` — `a{name` and `a}` → `Err(UnclosedBrace)`.
  - `parse_rejects_bad_width` — `{n:x}` → `Err(InvalidWidth("x"))`.
  - `split_stem_ext_last_extension_rule` — `archive.tar.gz` → (`archive`, `gz`);
    `noext` → (`noext`, ``); `.hidden` → (`.hidden`, ``).
  - `apply_find_replace_basic` — empty find = no-op; `foobar`/`foo`→`bar` = `barbar`;
    no match = unchanged.
  - `build_preview_numbers_from_custom_start_with_padding` — 3 entries `a.txt,b.txt,c.txt`,
    `{n:3}` start `5` → `005.txt, 006.txt, 007.txt`, all `Ok`, selection order preserved.
  - `build_preview_resolves_batch_internal_collision` — two entries both rendering `x.txt`
    (e.g. pattern `x.{ext}` on `a.txt`+`b.txt`): first `x.txt Ok`, second `x (2).txt
    ResolvedCollision`.
  - `build_preview_resolves_disk_collision` — pre-create `b.txt` in the tempdir; pattern
    `b.{ext}` over `a.txt` → preview `b (2).txt ResolvedCollision` (would_conflict sees the real
    file), and a same-name no-op (`b.txt` → `b.txt`) stays `Ok`.

- [ ] **Step 5: Verify** — `cargo test -p chronos-fm-services batch_rename::`

---

## Task 2: Dialog entity — `crates/chronos-fm-pages/src/explorer/batch_rename.rs`

**Files:**
- Create: `crates/chronos-fm-pages/src/explorer/batch_rename.rs`
- Modify: `crates/chronos-fm-pages/src/explorer.rs` (add `mod batch_rename;`)

**Interfaces:** consumes Task 1's `build_preview`/`Template`/`RenamePreview`,
`ops::rename_in_place(src, new_name)`, `listing::FileEntryDto`, gpui-component `Input`/`InputState`
(via the `search_bar.rs` pattern), `patterns::{elevated_card, section_header}`.

- [ ] **Step 1: Struct + construction**

```rust
//! Batch Rename dialog (T006): pattern / find-replace / counter inputs with a
//! live "was → will be" preview shown before anything is applied.

use std::rc::Rc;

use chronos_fm_services::fs::batch_rename::{build_preview, RenamePreview, Template};
use chronos_fm_services::fs::listing::FileEntryDto;
use gpui::*;
use gpui_component::input::{Input, InputState};

/// Modal state for renaming a multi-file selection with a pattern (T006).
pub struct BatchRenameDialog {
    /// The selected entries, in row order (their `name`s are the "was" side).
    pub entries: Vec<FileEntryDto>,
    /// Pattern input (`{name}_{n}.{ext}` …).
    pattern: Entity<InputState>,
    /// Find/replace inputs applied to the stem before `{name}`.
    find: Entity<InputState>,
    replace: Entity<InputState>,
    /// Counter start (defaults to 1 when empty/invalid is an error).
    start: Entity<InputState>,
    /// Live preview, recomputed on every keystroke.
    pub preview: Vec<RenamePreview>,
    /// Pattern / start parse error shown as a banner; Apply is disabled while set.
    pub last_error: Option<String>,
    /// Called on Apply with the per-file errors (empty Vec = success).
    on_committed: Rc<dyn Fn(&mut App, Vec<String>)>,
}

impl BatchRenameDialog {
    /// Builds the dialog for `entries`, with `on_committed` invoked on Apply.
    /// The pane passes a callback that reloads the listing and reports failures.
    pub fn new(
        entries: Vec<FileEntryDto>,
        window: &mut Window,
        cx: &mut Context<Self>,
        on_committed: Rc<dyn Fn(&mut App, Vec<String>)>,
    ) -> Self {
        let input = |cx: &mut Context<Self>| InputState::new(window, cx);
        let pattern = cx.new(input);
        let find = cx.new(input);
        let replace = cx.new(input);
        let start = cx.new(input);
        let mut dialog = Self {
            entries,
            pattern,
            find,
            replace,
            start,
            preview: Vec::new(),
            last_error: None,
            on_committed,
        };
        dialog.refresh_preview(cx);
        dialog
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
                    cx.notify();
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
        cx.notify();
    }
}
```

- [ ] **Step 2: Apply + close**

```rust
impl BatchRenameDialog {
    /// Applies every previewed rename via `ops::rename_in_place` (resolved names
    /// in preview order), collects per-file errors, then fires `on_committed`.
    pub fn apply(&mut self, cx: &mut Context<Self>) {
        let mut errors: Vec<String> = Vec::new();
        for preview in &self.preview {
            let Some(entry) = self
                .entries
                .iter()
                .find(|e| e.name == preview.old_name)
            else {
                continue;
            };
            let src = std::path::Path::new(&entry.path);
            match chronos_fm_services::fs::ops::rename_in_place(src, &preview.new_name) {
                Ok(_) => {}
                Err(error) => errors.push(format!("{}: {error}", preview.old_name)),
            }
        }
        let on_committed = std::mem::replace(
            &mut self.on_committed,
            Rc::new(|_cx, _errors| {}),
        );
        on_committed(cx, errors);
    }
}
```

> Apply must be disabled (Step 3 render) whenever `preview.is_empty()`, `last_error.is_some()`,
> or no row's `new_name != old_name`.

- [ ] **Step 3: Render** — `impl Render for BatchRenameDialog` returning an `elevated_card` +
`section_header(cx, "Batch Rename", &format!("{} files", self.entries.len()))`, containing:
  - four labeled `Input::new(&input)` widgets bound to the four `InputState`s, each with the
    search-bar-style text-changed listener that calls `this.refresh_preview(cx)` — verify the exact
    event-hook name (`on_input`/`on_text_changed`) against the pinned `gpui-component` checkout;
  - the `last_error` banner in the danger color (`theme::danger(cx)`) when set;
  - the preview list: one row per `RenamePreview`, `old_name → new_name`, with
    `ResolvedCollision` rows dimmed (`opacity`/`theme::muted`) and suffixed `(auto-resolved)`;
  - a footer: **Apply** (`Button`; disabled per the rule above; listener → `this.apply(cx)`) and
    **Cancel** (listener → pane `close_batch_rename` via the same `Entity<ExplorerPane>`-handle
    technique the context-menu plan uses: `pane.update(cx, |pane, cx| pane.close_batch_rename(cx))`).
  - Apply/Cancel wiring matches the repo rule that menu/dialog handlers that need the pane capture
    `let pane = cx.entity();` in the *caller's* scope.

- [ ] **Step 4: Verify it builds** — `cargo build -p chronos-fm-pages`

---

## Task 3: Pane wiring — state + open/close

**Files:** `crates/chronos-fm-pages/src/explorer/state.rs` (field),
`crates/chronos-fm-pages/src/explorer/batch_rename.rs` (impl block).

- [ ] **Step 1: Field** — in `ExplorerPane` (state.rs), next to `properties_dialog`:

```rust
    /// Batch-rename dialog, if open (T006).
    pub batch_rename: Option<Entity<super::batch_rename::BatchRenameDialog>>,
```
with `batch_rename: None` in `ExplorerPane::new`.

- [ ] **Step 2: Methods** (impl block in `batch_rename.rs`):

```rust
impl ExplorerPane {
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
        let on_committed: Rc<dyn Fn(&mut App, Vec<String>)> = Rc::new(move |cx, errors| {
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
        let dialog = cx.new(|cx| BatchRenameDialog::new(entries, window, cx, on_committed));
        self.batch_rename = Some(dialog);
        cx.notify();
    }

    /// Closes the Batch Rename dialog without applying anything.
    pub(crate) fn close_batch_rename(&mut self, cx: &mut Context<Self>) {
        self.batch_rename = None;
        cx.notify();
    }
}
```

- [ ] **Step 3: Basic dialog test** (same file's `#[cfg(test)] mod tests`,
`#![allow(clippy::disallowed_methods)]`, harness `new_explorer_for_tests`): create a tempdir with
`a.txt`+`b.txt`, reload, `open_batch_rename` with those entries, assert `page.batch_rename.is_some()`;
then set the pattern input (`dialog.pattern` is private — expose `pub(crate) fn pattern_input(&self) ->
&Entity<InputState>` **only if** the test needs it; otherwise set value via the entity handle
through `page.batch_rename`'s `update` and call `refresh_preview`), assert the preview contains
two `Ok` rows matching the pattern, then `close_batch_rename` and assert `None`.

- [ ] **Step 4: Verify** — `cargo test -p chronos-fm-pages batch_rename::`

---

## Task 4: View overlay

**File:** `crates/chronos-fm-pages/src/explorer/view.rs`

- [ ] **Step 1:** add `render_batch_rename_dialog(page, cx)` mirroring
`render_properties_dialog` (view.rs:175): when `page.batch_rename` is `Some`, render an
`absolute().inset_0()` scrim (`bg(hsla(0,0,0,0.4))`) whose `on_mouse_down` closes the dialog
(`close_batch_rename`), wrapping `.child(dialog.view(cx))`; Escape-to-close is handled in the
pane's existing key handler (add a branch like the properties dialog's). Append the result to the
`render` output next to `render_properties_dialog(...)` and `render_context_menu(...)`.
- [ ] **Step 2: Verify** — `cargo build -p chronos-fm-pages`

---

## Task 5: Entry point (gated on b3/b4 — STUB)

**No code in this task.** When b3/b4's row/grid context-menu **Rename** item lands, change its
handler from unconditionally `pane.begin_rename(ix, window, cx)` to:

```rust
// Multi-selection → batch dialog; single selection stays inline.
let selected = pane.read(cx).selected_paths();
if selected.len() > 1 {
    let entries = pane.read(cx).filtered_entries_for_selection(); // build from selection order
    pane.update(cx, |pane, cx| pane.open_batch_rename(entries, window, cx));
} else {
    pane.update(cx, |pane, cx| pane.begin_rename(ix, window, cx));
}
```

(`filtered_entries_for_selection` is a tiny helper the b3/b4 task adds on `ExplorerPane`; the
`Rename` item is already `disabled` for multi-selection in the current b3/b4 plan — that
`disabled` must be removed as part of this wiring.)

---

## Task 6: Verification pass

- [ ] `cargo test --workspace` — all green (new: services `batch_rename::`, pages `batch_rename::`).
- [ ] `cargo build -p chronos-fm` — clean.
- [ ] `cargo clippy -p chronos-fm-services -p chronos-fm-pages` — no new warnings in touched files
  (`missing_docs` on every new pub item; no `unwrap` outside tests).
- [ ] Manual smoke (no GUI harness in this crate — report as manual, per
  `verification-before-completion`): multi-select 3 files → Rename → dialog with live preview →
  type `{name}_{n:3}.{ext}` → preview updates per keystroke → Apply → rows update, `ls` confirms →
  repeat with a deliberate collision (two files → same target) → preview shows `(auto-resolved)` and
  Apply produces `x.txt` + `x (2).txt`.

## Report

`docs/orchestration/tasks/report/T006-batch-rename-report.md` (inbox) when the T-тикет for code is
created and executed. Until then this plan sits in `docs/superpowers/plans/` with Task 5 marked
gated-on-b3/b4.
