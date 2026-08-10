# T040 — Phase V: Settings tab smart pixel-copy

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (Phase V, 2026-08-10)
>
> Code PARTIAL upgraded after T046 vision grims:
> - `t046_settings.png` — 9-cat shell, Files live rows + honest unwired pills
> - `t046_settings_appearance.png` — Theme/Accent/Icon pack live; remaining
>   rows honest "not wired yet" (Phase F residual, not V reject)
>
> Phase F (wire remaining config rows, live-preview FM column, keybindings
> registry) = separate tickets — **not** blocking V close.
>
> Report: `report/T040-settings-tab-mockup-parity-report.md`
> Shots: `report-log/T046-shots/t046_settings*.png`


# T040 — Phase V: Settings tab smart pixel-copy

**Epic:** T042. **Priority:** P1.
**Mockup:** `docs/design/mockups/Chronos-File-Manager-Settings.dc.html`
**Code:** `crates/chronos-fm-pages/src/settings.rs` + existing `ConfigField` patch paths


## Strategy: smart pixel-copy (Phase V) first

**Agreed (client + architect):** mockup = product vision. Live may lag.
Delivery: **Phase V = visual/IA copy**, then **Phase F = function fill**.

### Phase V (THIS ticket — Must)

1. Layout/IA/density **match mockup** (shell chrome, nav, panels, empty states).
2. Wire **already-real** data/actions only (no fake history/transfers/progress).
3. Views without backend: **render chrome + honest empty/disabled** ("needs …"), do not delete from IA.
4. Icons: `crates/chronos-fm-ui/assets/icons/` only — not mockup SVG paths.
5. Theme: ChronOS dark tokens (`chronos.theme.json` / `chronos_fm_ui::theme`).
6. Proof: release binary + grim + **vision** review vs mockup.
7. Do **not** mix mega-backend (multipart, git log engine, plugin host) into V PR.

### Phase F (residual / follow-up tickets)

Backend + full interactivity for mockup-only views. Separate tickets after V ACCEPT.

### Non-negotiables

| Rule | Detail |
|------|--------|
| Source truth | `/home/neo/projects/chronos-ecosystem/Source` + `crates/*` — not Zed, not datasets |
| Source edits | Only for the better; commit+report **what / why / зачем** |
| Facts only | Claim → Evidence → Truth base; UNVERIFIED ≠ ACCEPT |
| Executor | **Vision/omnimodal** for visual ACCEPT |
| No secrets | in report/grim/config (S3 keys = keyring only) |


## Phase V scope (this ticket)

| Category (mockup IA) | V behavior |
|----------------------|------------|
| Left nav | Files / Appearance / Preview / Behavior / Terminal / Plugins / Keybindings / About |
| **Live fields** | theme/ui/explorer (and any already-patching keys) in mockup chrome |
| **Not wired yet** | Same categories **shown** disabled/coming — do **not** delete IA |
| Search / export / reset | Chrome OK; wire only if already supported |

## Phase F residual

New config schema keys (terminal, full preview columns, keybind capture, plugins runtime, export JSON) — separate tickets.

## Done when (V)

Grim: category nav + cards match mockup density; live toggles still persist; unwired sections visible honestly; vision ACCEPT V.

## Related

T036 · T009 settings live
