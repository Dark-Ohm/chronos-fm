# T052 — Drag-and-drop external (Chronos-FM ↔ other apps)

**Epic:** T048. **Priority:** P1.  
**Depends:** T051 in-app DnD stable (shared drag payload / drop pipeline).  
**Code:** explorer + Source/gpui Wayland/X11 DnD hooks as needed

## Problem

Replacing Thunar requires dropping files **from** browser/file manager **into**
Chronos-FM and dragging **out** to other apps (e.g. editor, chat, desktop).

## Must

1. **Drop-in:** external file URIs/paths accepted onto cwd or folder row
   (copy into target; policy documented).
2. **Drag-out:** selection exported so other apps receive files (portal /
   wayland data device / X11 — use what Source/gpui already supports;
   research first, no invented APIs).
3. Multi-file selection supported.
4. Failures → status/error, not hang.
5. Security: only local filesystem paths user already sees; no silent network
   fetch.

## Design gate

Before large Source patches: short note in report — which gpui/platform API
exists today, what must be added in Source (**what / why / зачем**).

## Done when

1. Live proof: drop from another FM or `nautilus`/`thunar` into Chronos-FM;
   drag out to another app if platform allows (else partial ACCEPT with
   evidence of supported direction only).
2. Report + architect stamp.

## Related

T048 · T051 · `docs/os-integration.md` · `docs/explorer-essentials.md` §2
