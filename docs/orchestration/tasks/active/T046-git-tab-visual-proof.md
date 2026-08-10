# T046 — Git tab visual proof (T038 residual)

**Priority:** P2 — blocks T038 full ACCEPT / `done/`, does not block T042
epic progress on T039–T041.
**Source:** T038 PARTIAL-ACCEPT (2026-08-10) — service + page layer
accepted, live visual verification explicitly UNVERIFIED.
**Mockup:** `docs/design/mockups/Chronos-Git-Tab.dc.html`

## Why this is its own ticket

T038's code (service layer + `GitView` sub-nav/History/Remotes/Commit-
Amend) is architect-ACCEPTed. What's missing is the one thing code review
can't give: does it actually look right on screen, across all five views,
next to the mockup. Splitting it out lets T038 code stay landed and
reviewed while this narrower, tooling-blocked task is picked up separately
— same pattern as T037→T043/T044/T045.

## Blocker (from T038's report — read before retrying blindly)

`hyprctl dispatch focuswindow "class:^(chronos-fm)$"` fails in this
session's Lua-Hyprland config: `')' expected near 'class'`. Without a
working focus dispatch, synthetic `ydotool` clicks land on whatever window
already had focus — confirmed via `hyprctl activewindow` showing a browser
window, not chronos-fm, after the click. Verified against an unambiguous,
large target (Places sidebar's "Documents" row) before concluding this is
an input-routing problem, not a coordinate-math problem specific to a nav
icon.

## Hypotheses for unblocking (untested — pick one, don't guess forever)

| ID | Approach | Note |
|----|----------|------|
| H1 | A real keybind (not `hyprctl dispatch`) bound to focus/raise chronos-fm, triggered live | Matches the documented ChronOS workaround: "переключение воркспейса только через реальный keybind или `hl.dsp.*`" for the same dead-`hyprctl-dispatch` class of issue |
| H2 | `hl.dsp.*` Lua-Hyprland API call instead of the CLI dispatch | Same root cause, different entry point — untested here |
| H3 | Manual click by a human at the keyboard (fastest, zero automation risk) | The pragmatic fallback if H1/H2 don't pan out quickly |
| H4 | A different sandbox/session where `hyprctl dispatch` isn't broken | Confirm this is session-specific, not universal, before assuming H1-H3 are required |

## Done when

1. Five live grims (release binary, `class=chronos-fm` reverified
   immediately before each `grim`, not just before a settle sleep — see
   T037/T044's "settled.png is not FM" trap): Changes, History, Branches,
   Stashes, Remotes.
2. Vision review (not programmatic pixel analysis alone) against
   `Chronos-Git-Tab.dc.html`, view by view.
3. Confirm the Commit/Amend segmented control visually reflects state
   (active option highlighted, button label swaps).
4. Confirm History's commit-detail card renders for a selected commit
   (not just "loading…" forever — would indicate the on-demand
   `select_commit` load never resolves in practice).
5. T038 moved to `done/` with this evidence, or a new, specific defect
   found and filed separately if the visuals reveal an actual bug (not
   just "matches/doesn't match mockup" — a real functional problem would
   go back to T038 or its own ticket, not block here indefinitely).

## Related

T038 (code, ACCEPTed) · T042 epic · T037/T044 "settled.png is not FM" trap
(same re-verify-before-grim discipline applies here)

> ## Architect note (2026-08-10)
> Expand scope name if useful: residual covers **Git/S3/Settings/Extensions**
> visual proof (T038–T041 PARTIAL-ACCEPT). Prefer one shared fix
> (`--page=<name>` CLI and/or hypr focus) over four tickets.

