# T044 — File listing content pane empty despite item count — Report

**Status:** ✅ RESOLVED (Places sidebar spun off separately as T045 — see below)
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5), same session as T043's correction

## Summary

The ticket was originally filed (and titled) as "Grid view empty" from a
misread of the toolbar — the actual default is **List** view
(`ExplorerPane::new` → `view_mode: ViewMode::List`, `state.rs:231`).

**Real root cause: not a rendering/layout/virtualization bug at all.**
T043's sidebar-bypass rewrite of `view.rs` (committed this session as part
of correcting T043's record) replaced the *entire* flex row's children —
sidebar **and** the listing/preview panel construction — with just the
sidebar div. `grep -rn "listing::render\b" crates/chronos-fm-pages/src/`
returned zero matches: `listing::render` was dead code, never called from
anywhere. The empty content pane on every prior grim (T037's, T043's, and
this ticket's first evidence) was simply an element that was never built,
not a virtualization bug producing zero-size output.

## Investigation trail (what was tried and falsified first)

Before finding the real cause, the following was falsified with evidence
(kept in the ticket for anyone re-deriving the same wrong leads):

1. **H1/H4 (empty/racing data):** falsified — `header.rs:72`'s
   `entry_count = page.filtered_entries.len()` (rendered as "N items" in the
   breadcrumb) and `grid.rs`'s item source read the *same* already-populated
   field in the same render pass; local (non-provider) `reload()` is fully
   synchronous, no async race.
2. **List-view `v_virtual_list` internals:** instrumented with unconditional
   `eprintln!` debug lines in `listing.rs`, `grid.rs`, and (temporarily,
   fully reverted after) `Source/gpui-component/crates/ui/src/
   virtual_list.rs`'s `prepaint`. **None fired** — proof `listing::render`
   itself was never reached, which is what led to grepping for its call
   site and finding it missing from `view.rs`.

## Fix

**File:** `crates/chronos-fm-pages/src/explorer/view.rs`

Restored `listing::render(...)`/`preview::render(...)` under
`gpui_component::resizable::h_resizable("file-explorer")` with its original
two panels (listing `flex_1`, preview `240px` resizable to `2000px`), as
siblings of T043's sidebar-bypass div (kept as-is — only the sidebar panel
had `h_resizable`'s allocation bug, per T043's evidence). Removed the
now-unused `gpui::prelude::FluentBuilder` import (was only used by the old
`.when()` call this replaced).

## Verification

3 independent release-binary runs, each with `class=chronos-fm` re-verified
via `hyprctl` **immediately before** `grim` (not just before the settle
sleep — see "tooling fix" below):

| Check | Result |
|-------|--------|
| Build | `cargo build --release -p chronos-fm` clean |
| Grim 1 (`t037_verify2.png`, 15s settle) | All 40 entries, Name column populated |
| Grim 2 (`t044_verify3.png` / `report-log/T044-shots/listing-with-columns-hover.png`, 25s settle) | Name/Type/Size/Modified all populated (real sizes like "2.1 KB", real dates), row hover highlight works, "Preview → No file selected" honest empty state |
| Grim 3 (`t044_final.png` / `report-log/T044-shots/listing-populated-40-items.png`, 20s settle, via hardened `t037_smoke.sh`) | Same — reproducible, not a one-off |

Matches `docs/design/mockups/chronos-file-manager.dc.html`'s listing region.

## New residual spun off: T045

All 3 verification grims also show the **Places sidebar missing** (nav rail
present, no Places column) — reproducible, correlated with a continuous
"can't render at a zero size" error storm starting ~8s after launch (30k+
occurrences by settle time, never stopping). This pre-existed as an
undiagnosed log-noise-only residual ("T037#5" in T037's report) but now
visibly disrupts the sidebar once the full three-panel tree is back. Not
investigated further here — filed as **T045** with its own hypothesis
matrix rather than folded into T044's now-closed scope.

## Tooling fix (same session)

`script/dev/t037_smoke.sh` sampled window geometry, then slept for the
settle duration, then grimmed that geometry — but grim captures raw screen
pixels at those coordinates regardless of which window occupies them *by
grim time*. This silently grimmed the wrong app twice during this
investigation (a terminal running another AI agent's session, a Hebrew
attendance-clock app), reproducing T037's own documented "settled.png is
not FM" trap. Hardened: re-verify `class=chronos-fm` right before `grim`,
bail loudly (`CLASS_GONE_BEFORE_GRIM`) instead of trusting stale geometry.

## Verdict

**RESOLVED.** File listing (List view, the actual default) now renders real
directory contents with populated columns, matching the mockup. T037 stays
open pending **T045** (sidebar + repaint-storm interaction).
