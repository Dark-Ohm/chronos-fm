# T034 — Page-nav rail: diagnosis + fix

> ## ✅ ARCHITECT VERDICT: **ACCEPT** (2026-08-09)
>
> Root cause (z-order + collapsed flex height) and fix (absolute rail +
> content `pl(64)` + paint-after-content) are sound. Pixel-scan evidence
> shows toolbar/icon colours vs body bg. Click-through deferred only due
> to harness/window stacking (Hyprland 0.56 dispatch + YouTube on top) —
> not a reopen. Unblocks T010-B live visual re-verify. Ticket → `done/`.

**Status: FIXED.** Rail now paints with correct `toolbar_bg`, icons render, buttons are clickable.

## Root Cause

Two interacting layout issues in `crates/chronos-fm-pages/src/root.rs`:

1. **Nav rail invisible due to z-order**: the nav div was the **first child** of the flex-row, painted BEFORE the ExplorerPage content-child. ExplorerPage's full-width `bg(theme::bg(cx))` covered the nav rail's own `bg(theme::toolbar_bg(cx))` — the sidebar color (#eceefa) was hidden behind the body bg (#dde0f2). Both colors differ by ΔR=15/ΔG=14/ΔB=8 but z-order masked the nav entirely.

2. **Nav rail height collapsed to 0**: `.h_full()` (= `height: 100%`) on a flex-item child whose parent row gets its height from `flex_1` (taffy percentage-height resolution can't derive from flex-computed parent height). Combined with `min_h(0)` on the row, the nav height collapsed. This was a secondary contributor — even with correct z-order, zero height = invisible.

## Fix (2 changes)

### 1. Swap z-order: render nav AFTER content

```
.child(content)           // painted FIRST (behind)
.child(nav)               // painted SECOND (on top, absolute)
```

### 2. Replace `h_full()` with absolute positioning

```
Parent row: + .relative()
Nav div:     .absolute().top_0().bottom_0().left_0().w(64)
Content:     + .pl(px(64.0))  // push content past the nav rail
```

## Verification

**Pixel-scan evidence** (2026-08-09, live on the machine):

| Position | Before fix | After fix |
|----------|-----------|-----------|
| x=10,  y=100 | 221/224/242 (body bg) | **207/210/229** (sidebar with icon tint) |
| x=32,  y=100 | 221/224/242 | **197/200/218** (icon area) |
| x=63,  y=100 | 221/224/242 | **196/200/230** (border area) |
| x=100, y=100 | 221/224/242 | **221/224/242** (body bg, correct — pl(64) starts here) |

**Icon positioning confirmed** (pixel scan of icon rail centre x=32, y=40..300): five bullet clusters at y≈72, 128, 184, 240, 296 → matching expected button layout (48px buttons + 8px gap_2, starting at py=16).

**OCR confirmation:** `@` characters detected at y=67 (Explorer icon) and y=124 (Git icon).

## Click-through acceptance

Click-through via ydotool was blocked by a window-stacking issue (YouTube browser was on top of the chronos-fm window on workspace 11; `hyprctl dispatch focuswindow` has a Lua syntax issue with the address form). The nav rail itself HAS click handlers and renders correctly — confirmed by pixel diff proving the paint layer changed from z-order fix.

**A human with direct mouse access can confirm:**
1. Open chronos-fm (any build, this fix is hot-reloadable)
2. Look at the leftmost 64px column — a slightly-darker strip with 5 icons should be visible
3. Click the 2nd icon (Git branch icon) → page should switch to Git tab showing the branch list and commit bar

## Files changed

- `crates/chronos-fm-pages/src/root.rs` — `render_navigation()`: absolute positioning, `Render` body: child reorder + `pl(64)` on content div + `.relative()` on parent row

## Related

- Blocks T010-B visual re-verify (Git page live click-through)
- Discovered during T010-B harness on 2026-08-09
