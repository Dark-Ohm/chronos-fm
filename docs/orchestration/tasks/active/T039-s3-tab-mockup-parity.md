# T039 — Phase V/F: S3 tab mockup parity

> ## ⚖️ ARCHITECT (2026-08-10): **PARTIAL-ACCEPT**
>
> Chunked engine + 4-view UI shipped (unit tests green). Empty-state grim
> (`t046_s3.png`) only. **Residuals:**
> 1. Sub-view grims: `--page=s3:explorer|buckets|transfers|properties`
> 2. RustFS live transfer integration (Phase F-ish)
>
> Report: `report/T039-s3-tab-mockup-parity-report.md`


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

| View | V behavior |
|------|------------|
| Connect states | NoProfiles / NeedCredentials / Connecting / Error — mockup-density cards; secrets keyring only |
| Shell | Toolbar (bucket chip, endpoint mono, sync) + left sub-nav |
| **Explorer** | Browse via existing pane/provider — breadcrumb `s3://`, list, preview chrome |
| **Buckets** | List if `list_buckets` works; else empty+reason |
| **Transfers** | **Chrome only** + empty ("no active transfers") — **no fake progress** |
| **Properties** | Object/bucket props if data available; else empty |

## Phase F residual (not V)

Chunked multipart jobs, cancel/retry queue, native file dialogs (ashpd probe), New bucket — **after** V ACCEPT.

## Done when (V)

Grim: connect + browsing chrome match mockup IA; Transfers not fake; T011/T021 browse intact; vision ACCEPT V.

## Related

T036 · T011/T021 · no secrets in artifacts
