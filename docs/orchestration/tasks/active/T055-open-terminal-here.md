# T055 — Open terminal here

**Epic:** T048. **Priority:** P2.  
**Code:** context menu + optional key (Ctrl+Alt+T or configurable later)

## Problem

Settings Terminal category is fully unwired. Every Linux FM offers
“Open Terminal Here” in the current directory.

## Must

1. Context menu on empty listing / folder: **Open Terminal Here**.
2. Spawn user terminal in that directory:
   - prefer `$TERMINAL` if set
   - else common list: `kitty`, `alacritty`, `foot`, `gnome-terminal`, `xterm`
   - or config key when Settings Terminal is wired (v1 can skip settings UI)
3. Failures → status error with command name.
4. No shell injection (argv array, not `sh -c` with path concat).

## Done when

Live: terminal opens in expected cwd; report; architect ACCEPT.

## Related

T048 · Settings Terminal residual
