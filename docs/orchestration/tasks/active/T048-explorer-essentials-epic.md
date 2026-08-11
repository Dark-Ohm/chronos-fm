# T048 — Epic: Explorer essentials (Thunar/Dolphin daily-driver parity)

**Priority:** P0 product. **Role:** index only — work lives in children.  
**Orthogonal to:** T042 pixel-copy.  
**Checkpoint #8 (2026-08-11):** T049–T051 **done ACCEPT** on main.

## Why

Context menus already cover basic ops. Daily muscle memory still needs keys,
marquee select, and file DnD (in-app done; external open).

## Children

| ID | Title | Pri | Status |
|----|-------|-----|--------|
| **T049** | File-ops keybindings | P0 | **done ACCEPT** |
| **T050** | Marquee multi-select list+grid | P0 | **done ACCEPT** |
| **T051** | DnD in-app (folder/cwd/cross-pane) | P0 | **done ACCEPT** |
| **T052** | DnD external (↔ other apps) | P1 | open — **next** |
| **T053** | Paste/drop conflict dialog | P1 | open (replaces unique_name) |
| **T054** | Undo stack | P1 | open |
| **T055** | Open terminal here | P2 | open |
| **T056** | New file + bulk progress | P2 | open |

## Order now

1. ~~T049 / T050 / T051~~ **done**
2. **T052** external DnD
3. **T053** conflict UI (paste + drop)
4. **T054** undo · T055 terminal · T056 progress

## Policies locked in shipped work

- Keys: pane focus + input isolation (T049).
- Marquee: measured bounds + epoch cancel (T050).
- DnD: Move default; Ctrl-at-drop Copy; **unique_name** until T053 (T051).
- Paste/Drop share `transfer_paths`.

## Non-negotiables

- Claim → Evidence → Truth base; release + `class=chronos-fm` for visual.
- No self-ACCEPT. Stage by file. No push without user ask.

## Done when (epic)

T049–T052 ACCEPT (or client waiver). T053–T056 ACCEPT or residual filed.
