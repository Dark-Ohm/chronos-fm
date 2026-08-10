# T041 — Extensions tab Phase V pixel-copy — Implementation report

> ## ⚖️ ARCHITECT VERDICT (2026-08-10): **PARTIAL-ACCEPT**
>
> ### Shell + honesty — **ACCEPT**
> Spot-checked: `ExtensionsView` ×4 (Installed/Available/Permissions/Host)
> matching mockup IA; search over installed ids; real `config.plugins`
> core/community; Install disabled “needs host · P4”; Available offline
> empty (P5); Permissions list + “not enforced” policy line; Host banner
> (no wasmtime) + `NavigateToSettings`. Icons from our pack; database.svg
> for Permissions noted as weak fit — OK. Architect re-ran extensions
> tests **3/3**.
>
> ### Documented trims — **accepted**
> No detail pane (no metadata), Available always empty (no marketplace),
> no grant toggles (no grants model) — correct Phase V.
>
> ### Live vision — **UNVERIFIED** residual (T046 class)
> Same hypr focus blocker as T038–T040. Shared residual: `--page=` or
> working focus — see T046.
>
> ### Epic T042
> All Phase V page children now at least PARTIAL-ACCEPT on code. Visual
> proof remains one residual class (T046), not four separate tickets.


**Status:** IMPLEMENTED, NOT ACCEPTED — not claimed done. Per session policy
the executor does not accept its own work.
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5)
**Mockup:** `docs/design/mockups/Chronos-Extensions-Tab.dc.html`
**Code:** `crates/chronos-fm-pages/src/extensions.rs`

## Summary

Rebuilt `ExtensionsPage` from a flat 3-card stack into the mockup's
sub-nav shell: left nav (Installed / Available / Permissions / Host ·
Runtime, mockup's own four) + view-head + routed body, same shell shape as
T038's `GitView` and T040's `SettingsCategory`. Search box in the header
filters installed plugin ids client-side (real, no backend needed). Every
view either shows real `config.toml` data or an honest empty/disabled
state — no view was deleted, no data was fabricated.

## Three deliberate Phase V scope trims (documented, not silent)

1. **No detail pane.** The mockup's 380px right column shows per-plugin
   version, description, readme, and a 5-column permission-grant grid.
   `config.plugins` is just `core: Vec<String>` / `community: Vec<String>`
   ids — checked `chronos-fm-core::config`, none of that metadata exists
   anywhere in this codebase. Building the pane would mean inventing fake
   versions/descriptions for real plugin ids, which is worse than not
   having it — a fabricated detail pane reads as real data with a real
   source. Trimmed.
2. **Available is one honest empty state, always** — "Marketplace not
   available yet" — never the mockup's six-card catalog. No marketplace
   backend exists (P5); this deliberately takes the mockup's own
   `marketOnline: false` path rather than its `true` demo-data path.
3. **Permissions has no per-permission allow/deny toggle grid.** No grant
   model exists in `config.toml` (`plugins.grants` or equivalent — none).
   Lists real `community` plugin ids with one honest policy line ("policy
   is stated intent, not an enforced runtime control") instead of a
   5-column switch table implying live, per-permission control that
   doesn't exist.

## What's real vs. honest-empty, by view

| View | Real | Honest |
|---|---|---|
| Installed | `config.plugins.core`/`community` ids, kind badge, search filter | Install… button disabled ("needs host · P4") — never a fake success |
| Available | — | Single empty state; explicitly *not* framed as a network error (there is no catalog to fail to reach) |
| Permissions | `config.plugins.community` ids listed | No community plugins → empty state; policy line makes clear grants aren't enforced |
| Host / Runtime | Roadmap prose (same content as the pre-T041 page, restructured) | Banner states host isn't running and **no wasmtime dependency is linked into this build** — checked, true; button navigates to Settings (`NavigateToSettings`, reused from `s3.rs`, already wired) |

## Icons

`crates/chronos-fm-ui/assets/icons/`: `puzzle.svg` (Installed),
`download.svg` (Available), `database.svg` (Permissions — no literal
"shield" icon exists in the pack; picked for the grants-storage
association, weaker fit than the other three, noted here rather than
silently substituted), `square-terminal.svg` (Host/Runtime — matches the
mockup's own choice for this exact view). All confirmed present via `ls`
before use.

## Tests

```
cargo test -p chronos-fm-pages extensions::
running 3 tests
test extensions::tests::all_four_views_are_distinct ... ok
test extensions::tests::every_view_has_a_label_and_icon ... ok
test extensions::tests::extensions_page_renders_without_panicking ... ok

cargo test --workspace --no-fail-fast   → 0 failed anywhere
cargo build --release -p chronos-fm     → clean, 1m34s
```

## Vision grim — UNVERIFIED

Same as T038/T039/T040: reaching the Extensions tab needs an interactive
click (nav-rail) and this sandbox's `hyprctl dispatch focuswindow` is
still broken. Not attempted — reporting UNVERIFIED rather than inferring
parity from a forced/lucky click, per the same discipline as the prior
three reports. **The Extensions tab's on-screen appearance against the
mockup is UNVERIFIED this session.**

## T042 epic status

All four Phase-V children (T037–T041, sic — T037/T038/T039/T040 plus this
one) are now implementation-complete and reported; every one carries the
same single open residual class: live visual verification blocked on
either a working focus-forcing path in this sandbox or a debug
`--page=<name>` CLI flag (raised as a shared idea in the T040 report).
Worth deciding as one residual ticket across all four rather than four
separate pointers, if that's useful — flagging, not deciding it myself.
