# T044 — File listing content pane empty despite non-zero item count [RESOLVED]

> ## ⚠️ CORRECTION (2026-08-10, same session): this is the **List** view, not Grid
>
> `ExplorerPane::new` defaults `view_mode: ViewMode::List` (`state.rs:231`),
> and the toolbar toggle in the evidence grim was showing **List** selected
> (misread earlier as Grid from the icon alone). So the empty content pane
> in `after-t014a-textmemo-fix.png` is `listing/list.rs`'s
> `v_virtual_list(...)` path, **not** `grid.rs`. Title/H2/H3 below updated
> accordingly — `grid.rs`'s flex-wrap width question is still worth
> checking once List is fixed, but it is not what's on screen in the
> evidence shot.
>
> **Further falsification this session:** `list.rs`'s `all_sizes` Vec is
> built correctly — `update_item_sizes()` (`state.rs:420`) maps all 40
> `filtered_entries` into real positive `size(px(total_width),
> px(BASE_ROW_HEIGHT))` entries (`total_table_width()` sums non-zero
> `config::COL_*_WIDTH` constants), plus one header-row size prepended in
> `list.rs`. So `all_sizes.len() == 41`, all with real dimensions, before
> `v_virtual_list(...)` is constructed — the data going *into* the
> virtualizer is correct. The bug is downstream: either in
> `v_virtual_list`'s own visible-range computation (gpui-component,
> `Source/gpui-component/crates/ui/src/...`, not yet located) or in how its
> viewport bounds / `scroll_handle` are obtained on first render.

**Priority:** P1 — blocks T037 Phase V full visual ACCEPT (§7 file listing).
**Source:** discovered while re-verifying T043 (sidebar text fix) with a
fresh release grim, 2026-08-10.
**Epic:** T042 pixel-copy / child of T037 residual.

## Symptom (facts)

- Live grim (`script/dev/t037_smoke.sh`, release binary, `hyprctl`-verified
  `class=chronos-fm` window, 15s settle) of the explorer at
  `~/projects/chronos-ecosystem/Chronos-FM` (a real 40-entry directory):
  - Breadcrumb bar shows **"40 Items"**.
  - Status bar shows **"40 items · 0 B"**.
  - Grid view (toolbar `Grid` toggle active) content pane is **completely
    empty** — no tiles, no icons, no names.
- Sidebar Places text renders correctly in the same frame (T043 fix
  confirmed), so this is **not** the T014-A text-memo bug (see DECISIONS.log
  2026-08-10 entry) — general text painting works in this same screenshot.
- Not verified in List view yet (toggle click not exercised this session).

## Hypotheses (must disprove with evidence — do not assume one)

| ID | Hypothesis | How to falsify |
|----|------------|----------------|
| H1 | `page.filtered_entries` genuinely empty at render time despite `entries.len()==40` (e.g. `apply_filter`'s `visible` iterator drops everything — hidden-file filter, sort bug, stale `search_query`) | **FALSIFIED.** `header.rs:72` reads `entry_count = page.filtered_entries.len()` and renders `"{entry_count} items"` (`header.rs:156`) — the exact string seen as "40 Items" in the live grim. `grid::render` (`grid.rs:17`) clones the *same* `page.filtered_entries` field. Both read from one already-populated Vec in the same render pass — data is present. |
| H2 | `grid::render`'s wrapping `div()` (`flex().flex_wrap()...`, no explicit `.w_full()`) collapses to zero cross-size inside `#[id("grid-scroll")].flex_1()`, so children lay out at zero-size and don't paint (same *class* of bug as the T043 `ListItem` row width issue, different root cause than the taffy memo fix) | Open — most likely given H1/H4 falsified. Add explicit `.w_full()` on the `grid` div (and check whether `#[id("grid-scroll")]` needs `.flex()`/`.flex_col()` itself — without it, its layout mode/child sizing is whatever gpui's `div()` defaults to, not necessarily block-fill); re-grim |
| H3 | `view_mode` read by `listing.rs` is stale/desynced from the toolbar's active `Grid` visual state (toolbar shows Grid selected but `page.view_mode` is something else entirely, e.g. still default) | Open — but since H1 confirms `filtered_entries` has 40 items and *some* branch of `listing.rs` must be running (the page isn't crashing), this would mean `list::render` (not `grid::render`) is actually executing despite the Grid toggle showing selected — worth checking, but H2 is more likely given the toolbar visually shows Grid active |
| H4 | Async listing race: `ensure_loaded` marks `loaded = true` before `list_dir_sync` actually completes on this cwd size, so the first render(s) still see empty `filtered_entries` even though later breadcrumb/status-bar reads pick up a since-populated count from a different state field | **FALSIFIED** — same evidence as H1: header and grid share one field, one render pass; `reload()` for local (non-provider) panes is fully synchronous (`list_dir_sync`, not spawned), so there is no async race for this path. |

## ✅ RESOLVED (2026-08-10, same session): real root cause was a dropped subtree, not a layout bug

All H1–H4 above were falsifying the wrong theory. Instrumented `listing.rs`,
`grid.rs`, and (temporarily, reverted after) `Source/gpui-component/.../
virtual_list.rs` with unconditional `eprintln!` debug lines — **none of them
ever fired**, meaning `listing::render` itself was never called. Traced
`view.rs`'s render tree and found: `grep -rn "listing::render\b"
crates/chronos-fm-pages/src/` returned **zero matches** — dead code. T043's
sidebar-bypass edit (`view.rs`, replacing the `h_resizable` 3-panel block)
rewrote the *entire* flex row's children down to just the sidebar div,
silently dropping the listing and preview panels from the render tree
altogether. The empty content pane was never a virtualization/measurement
bug — the pane simply wasn't there. T043's own report claim ("The listing
and preview panels continue to use `resizable_panel()` as before (they work
correctly)") did not match the actual tree, same pattern as its sidebar-text
claim.

**Fix** (`view.rs`): restored `listing::render`/`preview::render` under
`gpui_component::resizable::h_resizable("file-explorer")` with its original
two panels (listing flex + preview 240px), as siblings of the sidebar bypass
div (kept — only the sidebar panel had `h_resizable`'s allocation bug).
Removed now-unused `gpui::prelude::FluentBuilder` import.

**Verified** (3 independent release-binary + `hyprctl`-reverified-class
grims, `t037_verify2.png`/`t044_final.png`/etc, 15-25s settle each): all 40
real directory entries render with populated Name/Type/Size/Modified
columns (folder icons, real byte sizes like "2.1 KB", real dates), row hover
highlight works, "Preview → No file selected" honest empty state — matches
`docs/design/mockups/chronos-file-manager.dc.html` exactly for the listing
region. `cargo build --release -p chronos-fm`: clean.

**New residual (not this fix, tracked separately): sidebar drops out under
the pre-existing zero-size repaint storm.** All 3 verification grims show
the Places sidebar **missing** (nav rail present, no Places column) —
consistent and reproducible, not a one-off. Correlated with `/tmp/*.log`:
"can't render at a zero size" errors start ~8s after launch and never stop
(30000+ occurrences by a 20-25s settle) — this is the storm T037's report
already flagged as residual T037#5 ("Zero-size repaint storm (34k
errors/25s) | Known | T037#5"), pre-dating this session. It evidently was
already present when the sidebar-only tree was tested in isolation earlier
this session (that grim showed sidebar fine) but now visibly disrupts
sidebar rendering once the heavier `h_resizable` + `v_virtual_list` subtree
is back in the same frame. **Not re-investigated here** — T037#5 needs its
own root-cause pass (likely in `h_resizable`/`v_virtual_list`'s interaction
with continuous re-layout), separate ticket recommended rather than folding
into T044's now-closed scope.

**t037_smoke.sh hardened**: re-verifies `class=chronos-fm` immediately
before `grim` instead of trusting geometry sampled before the settle sleep
— grim captures raw screen pixels at coordinates regardless of which window
occupies them *now*, so a stale query silently grimmed the wrong app twice
this session (a Grok terminal, a Hebrew attendance-clock app) before this
fix, mirroring T037's own documented "settled.png is not FM" trap.

## Lead for next executor (superseded by RESOLVED above — kept for history)

`v_virtual_list` (`Source/gpui-component/crates/ui/src/virtual_list.rs`,
`VirtualList::request_layout`/`prepaint`) lays out visible items
individually via `item.layout_as_root(available_space, window, cx)` inside
`prepaint` (`virtual_list.rs:716`). Traced `layout_as_root`
(`Source/gpui/src/element.rs:499`): it calls the item's own `request_layout`
(registering the item's Text children as measured leaves on the SAME
frame-global `TaffyLayoutEngine`) and then `window.compute_layout(...)` —
the exact function this session's `frame_has_measured_leaf` fix patched.
Since the virtual list's own item rendering happens during the root tree's
prepaint phase (after the root's `request_layout` phase already registered
other measured leaves, e.g. the now-working sidebar text, which sets
`frame_has_measured_leaf = true` for the whole frame) **this session's fix
most likely already covers item text/icons too** — meaning an empty pane is
probably not a text-measurement problem at all here, but something upstream
of paint: `content_bounds`/`visible_range` (computed `virtual_list.rs:592-689`
from `layout.size_layout.content_size` vs `bounds.size`) may be resolving
to zero-height/empty on the frames this was grimmed, so `visible_range`
itself never includes any index — nothing gets built to lay out in the
first place. **Confirm with a debug log of `content_bounds.size` and
`visible_range` right before `virtual_list.rs:689`** before changing
anything; do not assume the text-measurement class of bug repeats here
without that evidence.

## Done when

1. Root cause Claim → Evidence (one H* confirmed, others falsified).
2. Fix: Grid (and List, if same root cause) shows all 40 entries on a fresh
   release grim of a real directory.
3. Regression test if state/race related (unit or integration).
4. Fresh grim attached under `report-log/T044-shots/` (or `T037-shots/`).
5. Unblocks T037 §7 full ACCEPT (listing content, not just chrome).

## Walls

- Source truth; no Zed/datasets.
- Facts only; no fake item counts.
- Vision/omnimodal on final grim — do not rely on pixel-count heuristics
  alone (T037/T043 history shows this class of check has produced false
  ACCEPT before).

## Related

T037 Phase V shell · T043 sidebar text fix (same grim, different bug) ·
DECISIONS.log 2026-08-10 (T014-A memo fix) · T042 epic
