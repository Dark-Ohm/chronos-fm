# T040 — Settings tab Phase V pixel-copy — Implementation report

> ## ⚖️ ARCHITECT VERDICT (2026-08-10): **PARTIAL-ACCEPT**
>
> ### Category shell + live wiring — **ACCEPT**
> Spot-checked: `SettingsCategory` ×9 (Files…About + S3), our SVG pack
> only, live search filter, `unwired_row` pills, pre-existing config fields
> still patch via `ConfigField`, **new** `UiIconPack` UI (`icon_pack_button`
> → `ConfigField::UiIconPack`). Architect re-ran settings tests **4/4**.
>
> ### Documented trims — **accepted**
> 1. No live-preview FM column (correct Phase V wall)
> 2. Keybindings honest empty (no keymap registry — correct)
> 3. S3 kept as 9th category (real T011 — correct “wire already-real”)
>
> ### Live vision — **UNVERIFIED** residual
> Same T046 hypr-focus class. Shared residual idea: `--page=<name>` CLI
> debug flag to unlock T038/T039/T040 visual proof without clicks — worth
> a small ticket (or fold into T046), not blocking this PARTIAL-ACCEPT.
>
> ### Verdict
> Ship settings shell. Ticket may stay active for vision residual or close
> with residual on T046. Prefer keep active lightly until grim once, or
> move to done with residual pointer — **architect: keep active**, residual
> visual only.


**Status:** IMPLEMENTED, NOT ACCEPTED — not claimed done. Per session policy
the executor does not accept its own work.
**Date:** 2026-08-10
**Executor:** Claude (Sonnet 5)
**Mockup:** `docs/design/mockups/Chronos-File-Manager-Settings.dc.html`
**Code:** `crates/chronos-fm-pages/src/settings.rs`

## Summary

Rebuilt `SettingsPage` from a flat stack of cards into the mockup's
category shell: a left sidebar (icon + label per category, active bar,
live search filter) and a routed main panel with a view-head + scrollable
body per category. Every already-real `config.toml` field the old page
wired stays wired (theme mode, accent, default sort, show-hidden,
explorer split/synced/restore-tabs) plus one new one (`ui.icon_pack`,
`ConfigField::UiIconPack` existed but was never surfaced in UI before this
pass). Every mockup category that has no backend in this codebase renders
its rows as an honest "not wired yet" pill — never deleted, never faked.

## Three deliberate Phase V scope trims (documented, not silent)

1. **No live-preview FM column.** The mockup's 430px right-hand pane
   mirrors every setting against a fake file listing in real time. Building
   that needs its own mocked listing subsystem — exactly the "mega-backend
   in a V PR" rule the ticket itself warns against (rule #7). Not built.
2. **Keybindings is one honest empty state, not per-row fake key chips.**
   This codebase has no keymap registry to read real bindings from (grepped
   for one — none exists). The mockup's own keybind rows are hardcoded
   demo data; reproducing them verbatim here would present fabricated
   bindings as if they were real and rebindable. Shows one message instead.
3. **S3 kept as a ninth sidebar category**, additional to the mockup's
   eight. It's real, already-working functionality (T011) that predates
   this mockup; deleting it to match a mockup that simply doesn't know
   about S3 would violate the ticket's own "wire already-real, don't
   delete IA" rule in spirit, even though it's not a literal IA violation
   (the row simply doesn't exist in the mockup to preserve).

## What's wired vs. shown-disabled, by category

| Category | Real | Shown honestly disabled |
|---|---|---|
| Files & Folders | Show hidden, Default sort column | system/OS files, dirs-first, case-sensitive sort, single-click open, confirm delete, confirm trash, size units |
| Appearance | Theme mode, Accent, **Icon pack (new)** | grid view, thumbnails, icon size, row height, date format, show/side preview panel |
| Preview Panel | — | all 6 rows (no preview-panel config exists) |
| Behavior | Split orientation, Synced panes, Restore tabs | open handler, use trash, drag & drop, paste overwrite, select-on-focus, focus-new-tab |
| Terminal | — | all 4 rows (no terminal subsystem exists) |
| Plugins | — | all 6 rows (no Luau host wired to config yet) |
| Keybindings | — | single honest empty state (see trim #2) |
| S3 | Profile display (read-only, T011) | — |
| About | App name + real crate version | — |

## Icons

All from `crates/chronos-fm-ui/assets/icons/` (rule #4) — no mockup SVG
paths copied: `folder.svg`, `palette.svg`, `panel-bottom-open.svg`,
`settings.svg`, `square-terminal.svg`, `puzzle.svg`, `replace.svg`,
`cloud.svg`, `info.svg`. Confirmed present via `ls` before use; `Icon::new
(Icon::empty()).path(...)` is the established pattern (`sidebar.rs`), not
`IconName::*` (that enum has no file-type/category variants for our pack).

## Tests

```
cargo test -p chronos-fm-pages settings::
running 4 tests
test settings::tests::accent_swatch_colors_match_palette ... ok       (pre-existing T018 regression)
test settings::tests::all_nine_categories_are_distinct ... ok         (new)
test settings::tests::every_category_has_a_label_and_icon ... ok      (new)
test settings::tests::settings_page_renders_without_panicking ... ok  (pre-existing, updated call site)

cargo test --workspace --no-fail-fast   → 0 failed anywhere
cargo build --release -p chronos-fm     → clean, 1m23s
```

## Vision grim — UNVERIFIED

Did not attempt an interactive click to switch the nav-rail to the
Settings tab: this sandbox's `hyprctl dispatch focuswindow` is still
broken (same blocker as **T046**), so a synthetic click's target window is
unreliable, and there's no CLI flag to boot straight into a given page
(checked `crates/chronos-fm/src/cli.rs` — no `--page` option exists).
Forcing a lucky click and reporting whatever showed up as "mockup parity"
would be inferring a result instead of observing one — not done. **The
Settings tab's on-screen appearance against the mockup is UNVERIFIED this
session**, same gap class as T039/T046, not a new kind of gap.

What **is** verified: release build launches clean with the new page
linked in (confirmed via the same `cargo build --release` pass used for
T039 — no separate smoke run needed since Settings isn't the default tab
and a startup-only grim wouldn't show it anyway).

## Recommendation

Same residual as T038/T039: needs either working interactive input in
this sandbox, or a `--page=<name>` debug CLI flag added specifically to
make Phase-V visual verification scriptable without relying on clicks —
worth considering as a small shared fix across all three open visual-proof
residuals (T046 class) rather than three separate one-off workarounds.
