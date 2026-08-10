# T046 — `--page=<name>` CLI flag + default-view proof — Implementation report

> ## ✅ ARCHITECT FINAL (2026-08-10): **ACCEPT / CLOSED** (with sub-view slice)
>
> Mechanism + default-view grims **ACCEPT**. Residual hypr-click **WAIVED**
> (superseded by `--page=`). Sub-view slice + T047: see companion report.
> Ticket moved `active/` → `done/T046-shared-visual-proof.md`.

**Status:** IMPLEMENTED, NOT ACCEPTED — not claimed done, and T038 is
**not** moved to `done/` (not enough evidence yet, see "What's still
missing" below). Per session policy the executor does not accept its own
work.
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5)
**Follows:** T041 verdict — "Next useful work: T046 (one fix for all
vision proofs), not more page V shells" and the architect note already on
the T046 ticket recommending `--page=<name>` and/or a hypr focus fix.

## What this delivers

A `--page=<name>` debug CLI flag (`explorer|git|s3|extensions|settings`,
case-insensitive, plus `fs`/`files`/`plugins` aliases) that opens the app
directly on a given page — sidesteps the broken `hyprctl dispatch
focuswindow` entirely for **reaching** a page, no synthetic click needed.

- `PageKind::from_cli_name(&str) -> Option<PageKind>` (pure,
  `chronos-fm-pages`) — new, 4 unit tests.
- `Cli::initial_page(&self) -> Option<PageKind>` (`chronos-fm/src/cli.rs`)
  — same "warn and ignore on invalid, never abort the launch" contract as
  the existing `--theme` flag. 4 unit tests.
- `RootView::new` gained an `initial_page: Option<PageKind>` parameter,
  defaulting to `PageKind::Explorer` when `None` (the flag omitted) — the
  product default is unchanged for every normal launch.
- `script/dev/t046_page_smoke.sh` (new) — same launch/settle/re-verify-
  before-grim/kill discipline as `t037_smoke.sh`, parameterized by page
  name.

```
cargo test -p chronos-fm-pages   → 103 passed; 0 failed (incl. 4 new PageKind tests)
cargo test -p chronos-fm         → 8 passed; 0 failed (incl. 4 new Cli tests)
cargo test --workspace --no-fail-fast → 0 failed anywhere
cargo build --release -p chronos-fm   → clean
```

## Real evidence gathered (first genuine live grims this epic, not startup-only)

Ran `script/dev/t046_page_smoke.sh <page> <out.png>` against the release
binary for all four PARTIAL-ACCEPT pages. All four launched clean, zero
log errors, `GRIM_OK`:

- **Git** (`/tmp/t046_git.png`) — no repo at `$HOME` in this environment,
  so it correctly shows the honest "Not a git repository" card with
  Refresh/Pin, and Pull/Push in the toolbar. Confirms the page routes and
  renders without a live repo to browse, not a defect — just what "Git tab
  from `$HOME`" honestly looks like.
- **Settings** (`/tmp/t046_settings.png`) — full T040 shell renders
  correctly: all 9 sidebar categories (Files & Folders active by default,
  Appearance, Preview Panel, Behavior, Terminal, Plugins, Keybindings, S3,
  About), search box, Files & Folders rows with real Show-hidden switch
  (on) and Default-sort buttons (Modified active), "not wired yet" pills
  on every unwired row. Matches the mockup's density and IA for this
  category.
- **Extensions** (`/tmp/t046_extensions.png`) — full T041 shell: 4-item
  sub-nav (Installed active), search box, "pre-P4" status pill, honest
  "No plugins configured" empty state with the `config.toml` snippet
  (no plugins configured in this environment's config, so this is the
  correct empty path, not the populated-list path).
- **S3** (`/tmp/t046_s3.png`) — correctly shows "No S3 profiles
  configured" + "Open Settings" (no profile in this environment's
  config — the credentials-flow path, not the 4-view Browsing shell,
  since there's nothing to connect to).

## What's still missing — this is a partial fix, not a full one

`--page=<name>` gets you to a page's **default** view for free. It does
**not** solve in-page navigation: Git's History/Branches/Stashes/Remotes,
S3's Buckets/Transfers/Properties, Settings' other 8 categories,
Extensions' Available/Permissions/Host — every one of those still needs an
actual click, and this sandbox's `hyprctl dispatch focuswindow` is exactly
as broken as before. I did not touch the Hyprland/ydotool side at all
(H1/H2/H4 on the ticket remain untried).

None of the four pages' environments here happened to have real data
loaded (no repo, no S3 profile, no plugins configured), so even the
default-view grims above prove rendering-without-crashing and the honest-
empty-state paths, not the "real data populated" paths the mockups show.
That's a second, separate gap from the sub-view-navigation one.

## Recommendation

Not moving T038 (or any of T039–T041) to `done/` — the ticket's own "Done
when" checklist for T038 explicitly wants five views, not one. This is
real progress (a working, tested, honest mechanism to reach any page
without fighting the compositor) but only closes a fraction of the
original residual. Two remaining paths, not mutually exclusive:

1. Extend the debug surface further — e.g. a second flag or env var to
   also pick a sub-view/category on launch (`--page=settings:appearance`)
   — would get every page to every named view without ever needing a
   click. Bounded, mechanical work, no compositor dependency.
2. Actually fix the click path (H1/H2 on the ticket) for anything that
   genuinely needs interaction to prove (button hover states, live toggle
   round-trips, drag targets) — `--page=` can't substitute for that
   category of proof.
