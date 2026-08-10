# T041 — Phase V: Extensions tab smart pixel-copy

> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (Phase V, 2026-08-10)
>
> Code PARTIAL upgraded after T046 vision grim:
> - `t046_extensions.png` — 4-view left nav (Installed/Available/Permissions/
>   Host), search, pre-P4, honest "No plugins configured" empty
>
> Phase F (plugin host / marketplace / grants) = separate after V.
>
> Report: `report/T041-extensions-tab-mockup-parity-report.md`
> Shot: `report-log/T046-shots/t046_extensions.png`


# T041 — Phase V: Extensions tab smart pixel-copy

**Epic:** T042. **Priority:** P1.
**Mockup:** `docs/design/mockups/Chronos-Extensions-Tab.dc.html`
**Code:** `crates/chronos-fm-pages/src/extensions.rs` (T012 landing today)


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


## Product split

| Surface | Owns |
|---------|------|
| **Extensions tab** | Catalog lifecycle: installed / available / permissions / detail |
| **Settings → Plugins** | Runtime policy (sandbox, hot-reload) — do not duplicate all toggles here |

## Phase V scope (this ticket)

| View | V behavior |
|------|------------|
| Shell | Title ChronOS·Extensions, search, install CTA chrome |
| **Installed** | Core + config.plugins list in mockup rows; enable/disable only if real |
| **Available / Discover** | Layout + empty/disabled ("plugin host P4") — no fake marketplace |
| **Permissions** | Layout + empty or static deny-by-default copy |
| Detail pane | Selected plugin meta / roadmap honesty |
| Host banner | Pre-P4: architecture + link to Settings if needed |

## Phase F residual

`plugin-host` install/activation, WASM perms, marketplace (P5).

## Done when (V)

Grim vs mockup IA; no fake install success; vision ACCEPT V.

## Related

T036 · T012 extensions landing · architecture plugin-host P4
