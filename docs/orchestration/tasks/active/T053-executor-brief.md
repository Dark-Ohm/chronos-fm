# T053 — Executor brief: Paste/drop conflict dialog

**Ticket:** `active/T053-paste-conflict-dialog.md` · **Epic:** T048
**Priority:** P1 — replaces the current silent `unique_name` behavior in
both paste (T049) and drop (T051/T052) paths.

## What exists today (don't rediscover, start here)

- `crates/chronos-fm-pages/src/explorer/file_ops.rs:43` —
  `paste_clipboard` calls the service's `unique_name` on collision
  (silent auto-rename). Test at `:235`
  (`paste_collision_uses_the_service_unique_name`) documents current
  behavior — **this test's assertion changes** once the dialog ships;
  don't leave it green by accident, update it to assert the new flow.
- `crates/chronos-fm-pages/src/explorer/dnd.rs` — both in-app drop
  (T051) and external drop (T052) already funnel through
  `transfer_paths` + `unique_name` + `finish_target_drop`. This is the
  **other** call site that needs the same conflict dialog, not just
  paste — the ticket says "paste or in-app drop", T052 added external
  drop as a third caller of the same `unique_name` path. Check whether
  external drop should route through the same dialog too (spec doesn't
  exclude it) and say explicitly if you scope it out.

## Spec (verbatim mock, `docs/explorer-essentials.md` §1.2)

```text
┌──────────────────────────────────────────────────────────┐
│  ⚠  "report.pdf" already exists in "Documents"            │
│                                                          │
│  ┌────────────┐                                          │
│  │  [ICON]    │   Replacing it will overwrite its        │
│  │  report.pdf│   current contents.                      │
│  └────────────┘                                          │
│     2.4 MB · Modified 2026-05-30 14:02                   │
│                                                          │
│  [✓] Apply to all remaining conflicts (3 more)           │
│                                                          │
│              [ Skip ]  [ Rename ]  [ Cancel ]  [ Overwrite ]│
└──────────────────────────────────────────────────────────┘
```

- Default focus: **Rename** (non-destructive). `Enter` = default button,
  `Esc` = Cancel.
- **Overwrite** is rightmost, accent color — deliberately separated from
  Rename to prevent misclick.
- **Apply to all** ON → remaining conflicts of the *same operation*
  apply silently, with remaining-count shown.
- Conflicts during a bulk op (§1.4 progress) are **queued**, one dialog
  shown at a time — not N simultaneous dialogs.

## Must (from the ticket, repeated so nothing is missed)

1. Modal: **Skip · Rename · Overwrite · Cancel**, Apply-to-all checkbox.
2. Default focus Rename.
3. Apply-to-all persists for the rest of that paste/drop session.
4. v1 may be sequential (one dialog at a time) — progress-bar
   integration stays T056's scope, don't pull it in.
5. Tests for the **decision-application logic** as pure functions (no
   GPUI context needed to test "given decision X and N remaining
   conflicts, what happens") — mirror how `dnd.rs`'s `can_accept_*`
   functions are already pure and unit-tested without a window.

## Design note expected before wiring UI

Where does the new decision type live — a new enum next to
`TransferError`/existing conflict signaling in the S3 transfer engine
(T039) reused, or Explorer-local? They're structurally similar
(Skip/Overwrite/Rename-style decisions exist informally today via
`unique_name`'s auto-behavior) but S3's transfer conflict handling and
Explorer's paste/drop are different code paths — don't force a shared
type if it doesn't fit; a short note on why shared-or-not is enough,
this isn't a full design-gate ticket like T052's.

## Tests (mandatory, not optional)

- Pure logic: decision (Skip/Rename/Overwrite/Cancel) × Apply-to-all →
  correct outcome per conflict, remaining-count tracked correctly.
- `paste_collision_uses_the_service_unique_name` (file_ops.rs:235)
  updated to reflect the new dialog-driven flow, not deleted silently —
  if the old auto-unique-name behavior is fully replaced, the test name
  and assertion must say so.
- `cargo test --workspace` green, new test count visible.

## Done when

Unit tests green + live grim of the dialog on a real conflict (paste or
drop a file that already exists at the destination, `class=chronos-fm`).
Report with Claim → Evidence blocks, no self-ACCEPT, ticket stays in
`active/` until architect stamp.

## Related

T048 (epic) · T049 (paste) · T051 (in-app drop) · T052 (external drop —
check scope question above) · `docs/explorer-essentials.md` §1.2
