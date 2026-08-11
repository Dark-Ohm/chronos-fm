# T053 — Paste/drop conflict dialog — report

**Epic:** T048 · **Ticket:** `active/T053-paste-conflict-dialog.md` · **Date:** 2026-08-11
**Status:** implemented, unit + live verified — awaiting architect stamp

## 1. Scope answer (explicit, per brief)

**External drop (T052) IS in scope.** Structural reason: `paste_clipboard`,
`begin_file_drop` and `begin_external_drop` all funnel through the *same*
service entry point (`ops::transfer_paths` → internal `unique_name`), so the
conflict dialog pre-flight at that one shared layer covers all three callers
for near-zero extra cost; excluding external drops would leave them silently
auto-renaming while paste/drop dialog — a verification surprise.

## 2. Design note

`docs/superpowers/specs/2026-08-11-paste-drop-conflict-design-note.md`
(light note per brief, not a full design gate).

- **Decision type reused:** `ops::ConflictResolution { Rename, Overwrite, Skip }`
  already existed in the service (built for this, unused by `transfer_paths`).
  `Cancel` is Explorer-level (`ConflictChoice::Cancel`) — it aborts the whole
  operation and is never recorded as a per-item resolution.
- **Architecture:** pre-flight plan on the UI thread (one dialog at a time,
  per mockup), then ONE transfer via the new `transfer_paths_resolved`.
  `Rename` recomputes `unique_name` **at transfer time** (TOCTOU-safe);
  `Overwrite` writes to the original name; `Skip` is omitted from both
  successes and failures; `None` keeps the historic auto-rename.

## 3. What was built

### Service (`crates/chronos-fm-services/src/fs/ops.rs`)
- `transfer_paths_resolved(sources, destination, mode, resolutions)` —
  per-source `Option<ConflictResolution>` plan; `transfer_paths` stays as the
  all-`None` wrapper.

### Pure decision logic (`crates/chronos-fm-pages/src/explorer/conflict.rs`)
- `ConflictQueue` — decision (Skip/Rename/Overwrite/Cancel) × apply-to-all →
  per-conflict resolution recorded, remaining count tracked, apply-all adopts
  the decision for every *remaining* conflict (never clean sources), cancel
  aborts without recording.
- `detect_conflicts` — pre-flights destination collisions in transfer order
  (uses `ops::would_conflict` + metadata reads only).
- **6 unit tests** of the decision matrix (no GPUI context, mirroring
  `dnd.rs`'s pure `can_accept_*` style).

### UI (`crates/chronos-fm-pages/src/explorer/conflict_dialog.rs`)
- `ConflictDialog` — mockup §1.2 verbatim: warning header, file icon + name
  card, "Replacing it will overwrite its current contents.", size ·
  modified line, `[✓] Apply to all remaining conflicts (N more)` checkbox
  (hidden when nothing remains), buttons `[ Skip ] [ Rename ] [ Cancel ]
  [ Overwrite ]` with **Rename default (Enter)**, **Esc = Cancel**,
  **Overwrite rightmost in accent** (via `ButtonCustomVariant`, deliberately
  separated from Rename).
- `PendingTransfer` holds the queue + items + position + apply-all + the
  transfer starter; the dialog is a **view** whose buttons call back into the
  pane. Keyboard decisions run **directly on the pane** — this is what
  prevents re-entering the pane's entity from inside its own update.

### Wiring
- `file_ops.rs::paste_clipboard`, `dnd.rs::begin_file_drop` and
  `begin_external_drop` all route through
  `transfer_with_conflict_dialog`; no conflicts → immediate transfer on the
  caller's original executor (paste stays synchronous as before; drops stay
  on the background executor).
- `view.rs`: overlay render (click-outside = cancel), Esc → cancel, Enter →
  Rename.

## 4. Bug found & fixed by the live run

The first live run panicked on the **last** decision: `cannot update
ExplorerPane while it is already being updated` (`entity_map.rs`). The Enter
key handler called `dialog.update` from inside the pane's own update, and the
dialog's resolved-callback then re-entered the pane. Fixed architecturally:
decision state moved onto the pane (`PendingTransfer`), the dialog became a
view with callbacks, and keyboard decisions call `conflict_decide` directly —
no entity re-entry exists anymore (verified live: 0 panics across the full
queue).

## 5. Tests

- Pure queue (`conflict.rs`): decision × apply-to-all matrix, remaining
  count, cancel, detect order — **6 passed**.
- Service matrix (`ops.rs`): mixed plan Skip/Overwrite/Rename/None →
  report contents; Rename recomputed at transfer time when the destination
  frees up — **2 passed** (21 ops tests total green).
- e2e (file_ops): paste collision **rewritten** from
  `paste_collision_uses_the_service_unique_name` (old silent auto-rename) to
  the dialog flow + **3 new**: Overwrite replaces, Cancel aborts (nothing
  transfers + footer "Transfer cancelled"), apply-to-all skips every
  remaining conflict.
- e2e (dnd): 4 same-parent-copy tests updated to decide Rename through the
  pane (they assert the same unique-name outcome, now explicit).

```
cargo test --workspace  → 464 passed / 0 failed
cargo clippy -p chronos-fm-pages  → no new warnings in changed files
cargo build --release -p chronos-fm  → clean
```

## 6. Live grim (class=chronos-fm, fake `$HOME=/tmp/t053_home`)

Launch → Ctrl+A → Ctrl+C → Ctrl+V (copy of the 3 files into their own
directory ⇒ 3 conflicts) → one dialog at a time:

1. `docs/orchestration/tasks/report-log/T053-shots/T053-conflict-dialog.png` —
   `"b.txt" already exists in "t053_home"`, overwrite warning, size/date line,
   `Apply to all remaining conflicts (2 more)`, Skip/Rename/Cancel/Overwrite.
2. Enter → dialog advances to `"c.txt"` `(1 more)`; Enter → `"x.txt"`;
   Enter → all three resolved as Rename.
3. `docs/orchestration/tasks/report-log/T053-shots/T053-after-rename.png` —
   listing now shows 6 items: `b (2).txt`, `c (2).txt`, `x (2).txt`
   created; originals intact.

Process stayed alive (0 panics in log) — the re-entrancy fix holds under the
full queue.

## 7. Honest residuals

- Paste after the dialog runs **synchronously** on the UI thread (same as
  legacy paste; drops remain backgrounded). Not a regression, but a future
  background-paste ticket could move it.
- `detect_conflicts` runs a stat per source on the UI thread — negligible for
  normal selections, unbounded for huge drops (v1 per brief).
- Apply-to-all persists for the current operation only (per mockup "same
  operation"), reset on the next paste/drop.
- Click-through of the dialog **buttons** (Skip/Overwrite/Cancel) was not
  exercised live — only keyboard Enter through the full queue; the button
  path is unit-covered by the e2e decide calls and shares `conflict_decide`.
- Breadcrumb does not accept drops (pre-existing T051 residual, unchanged).

## 8. Evidence map

| Claim | Evidence |
|---|---|
| Dialog matches mockup §1.2 | grim `report-log/T053-shots/T053-conflict-dialog.png` (OCR: title, warning, size/date, apply-to-all counter, 4 buttons) |
| Queue is one-dialog-at-a-time | Enter sequence advanced b.txt → c.txt (1 more) → x.txt (live, grimmed) |
| Rename transfers land | `report-log/T053-shots/T053-after-rename.png` + `ls` shows `(2).txt` triples |
| No re-entrancy crash | live run: `pgrep` ALIVE after full queue, `grep panic` → 0 |
| Decision matrix | conflict.rs 6 tests, ops.rs matrix 2 tests |
| Old paste test not silently deleted | rewritten to `paste_collision_opens_conflict_dialog_and_rename_keeps_both` + 3 new e2e |
