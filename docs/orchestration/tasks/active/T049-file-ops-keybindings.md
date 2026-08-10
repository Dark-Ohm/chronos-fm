# T049 — File-ops keybindings (Ctrl+C/X/V, F2, Del, Ctrl+A)

> ## ⚖️ ARCHITECT (2026-08-10): **DESIGN APPROVED — IMPLEMENT GO**
>
> Spec: `docs/superpowers/specs/2026-08-10-file-ops-keybindings-design.md`
> (APPROVE). Next: implementation plan, then land/salvage code + report.
> No self-ACCEPT.

**Epic:** T048. **Priority:** P0.  
**Code:** `crates/chronos-fm-pages/src/explorer/` (actions + bindings on pane focus)

## Problem

File ops exist via context menu (`copy_selection` / `cut_selection` /
`paste_clipboard` / `begin_rename` / `delete_paths`) but **almost no
keybindings** for them. Pane keys today are only split/tabs/sidebar
(`page.rs` `bind_pane_keys`). Daily FM users hit Ctrl+C and nothing happens.

Settings → Keybindings category is honest empty (no registry yet). This ticket
is **hardcoded platform bindings** first; full user keymap registry is later
(or residual under Settings Phase F).

## Must

| Key (Linux) | Action | Wire to |
|-------------|--------|---------|
| Ctrl+C | Copy selection | `copy_selection` |
| Ctrl+X | Cut selection | `cut_selection` |
| Ctrl+V | Paste into cwd | `paste_clipboard` |
| F2 | Rename (single → inline; multi → batch if already multi) | `begin_rename` / batch |
| Delete | Trash selection (existing confirm path) | same as context Delete |
| Ctrl+A | Select all filtered entries | selection API |
| Enter | Open default / enter dir | existing open path |
| optional Ctrl+Shift+N | New Folder | `new_folder` |

Also bind Cmd-* mirrors where other pane keys already dual-bind.

## Done when

1. Unit/action tests or headless invoke of actions.
2. Live: grim optional; **manual or ydotool** proof preferred for ACCEPT.
3. Report: Claim → Evidence (which keys, which code paths).
4. Architect ACCEPT — no self-ACCEPT.

## Out of scope

- User-editable `[keybindings]` config UI (note residual)
- Undo (T054), DnD (T051/T052)

## Related

T048 · b1 clipboard · `docs/explorer-essentials.md` §6 (if present)
