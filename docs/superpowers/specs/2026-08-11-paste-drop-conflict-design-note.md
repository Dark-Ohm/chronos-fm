# T053 — Design note: paste/drop conflict dialog

**Ticket:** `active/T053-paste-conflict-dialog.md` · **Epic:** T048 · **Date:** 2026-08-11
This is a light note (per the brief), not a full design gate.

## Scope answer (explicit, per brief)

**External drop (T052) IS in T053 scope.** Reason is structural, not a
preference: all three entry points — `paste_clipboard`
(`file_ops.rs`), in-app drop (`begin_file_drop`), external drop
(`begin_external_drop`) — already funnel through the *same* service function
`ops::transfer_paths`, whose internal `unique_name` call (ops.rs:186) is the
single silent auto-rename point. Routing the conflict dialog through that one
shared pre-flight costs nothing extra for the third caller, and excluding it
would leave external drops silently auto-renaming while paste/drop dialogs —
an inconsistency that would surface as a verification surprise.

## Decision type: reuse `ops::ConflictResolution`, don't touch S3

`chronos-fm-services::fs::ops::ConflictResolution { Rename, Overwrite, Skip }`
already exists and is documented as "applied by the caller" — it was built for
this and is currently unused by `transfer_paths`. It is *not* S3-specific: the
S3 transfer engine has its own conflict signaling in a different module and
needs no change. Reusing it gives the pure decision logic + the service a
shared vocabulary with zero new types.

`Cancel` is not a `ConflictResolution` — it aborts the whole operation and is
handled at the Explorer queue level (`ConflictChoice::Cancel`), never recorded
as a per-item resolution.

## Architecture: pre-flight plan, then one transfer

The dialog must run on the UI thread; the transfer runs on the background
executor. So the conflict resolution happens in a **pre-flight phase** on the
UI thread (one dialog at a time, per the mockup queue), producing a plan
(`Vec<Option<ConflictResolution>>` parallel to sources; `None` = no conflict,
falls back to historic auto-rename), and then the transfer runs once with the
plan via a new service entry point `transfer_paths_resolved`.

- `Rename` → `unique_name` recomputed **at transfer time** (TOCTOU-safe: the
  destination may change between the dialog and the transfer).
- `Overwrite` → transfer to the original destination name, replacing it.
- `Skip` → omitted from both successes and failures.
- `None` → historic behavior (`unique_name`), for non-conflicting sources.

Same-pane / cross-pane refresh + failure status reuse the existing
`finish_target_drop` / paste status paths unchanged.

## UI

New `explorer::conflict::ConflictDialog` entity, modeled on the existing
`BatchRenameDialog` (pane-held `Option<Entity<..>>` + overlay render in
`view.rs` + Esc keybinding). Mockup §1.2 verbatim: name + destination, size +
modified, "Apply to all remaining conflicts (N more)" checkbox, buttons
`[Skip] [Rename] [Cancel] [Overwrite]` with Rename default-focused and
Overwrite rightmost/accent.

## Tests

Pure queue logic (`conflict.rs`, no GPUI): decision (Skip/Rename/Overwrite/
Cancel) × apply-to-all → recorded resolution per conflict + remaining count +
apply-all adoption. Service matrix (`ops.rs`): Skip/Overwrite/Rename/
mixed-None → report contents. The old paste auto-rename test is rewritten to
assert the new pre-flight/dialog behavior, not deleted.
