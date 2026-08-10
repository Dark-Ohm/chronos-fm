[38;5;8m   1[0m [37m# T040 — Phase V: Settings tab smart pixel-copy[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m**Epic:** T042. **Priority:** P1.[0m
[38;5;8m   4[0m [37m**Mockup:** `docs/design/mockups/Chronos-File-Manager-Settings.dc.html`[0m
[38;5;8m   5[0m [37m**Code:** `crates/chronos-fm-pages/src/settings.rs` + existing `ConfigField` patch paths[0m
[38;5;8m   6[0m 
[38;5;8m   7[0m 
[38;5;8m   8[0m [37m## Strategy: smart pixel-copy (Phase V) first[0m
[38;5;8m   9[0m 
[38;5;8m  10[0m [37m**Agreed (client + architect):** mockup = product vision. Live may lag.[0m
[38;5;8m  11[0m [37mDelivery: **Phase V = visual/IA copy**, then **Phase F = function fill**.[0m
[38;5;8m  12[0m 
[38;5;8m  13[0m [37m### Phase V (THIS ticket — Must)[0m
[38;5;8m  14[0m 
[38;5;8m  15[0m [37m1. Layout/IA/density **match mockup** (shell chrome, nav, panels, empty states).[0m
[38;5;8m  16[0m [37m2. Wire **already-real** data/actions only (no fake history/transfers/progress).[0m
[38;5;8m  17[0m [37m3. Views without backend: **render chrome + honest empty/disabled** ("needs …"), do not delete from IA.[0m
[38;5;8m  18[0m [37m4. Icons: `crates/chronos-fm-ui/assets/icons/` only — not mockup SVG paths.[0m
[38;5;8m  19[0m [37m5. Theme: ChronOS dark tokens (`chronos.theme.json` / `chronos_fm_ui::theme`).[0m
[38;5;8m  20[0m [37m6. Proof: release binary + grim + **vision** review vs mockup.[0m
[38;5;8m  21[0m [37m7. Do **not** mix mega-backend (multipart, git log engine, plugin host) into V PR.[0m
[38;5;8m  22[0m 
[38;5;8m  23[0m [37m### Phase F (residual / follow-up tickets)[0m
[38;5;8m  24[0m 
[38;5;8m  25[0m [37mBackend + full interactivity for mockup-only views. Separate tickets after V ACCEPT.[0m
[38;5;8m  26[0m 
[38;5;8m  27[0m [37m### Non-negotiables[0m
[38;5;8m  28[0m 
[38;5;8m  29[0m [37m| Rule | Detail |[0m
[38;5;8m  30[0m [37m|------|--------|[0m
[38;5;8m  31[0m [37m| Source truth | `/home/neo/projects/chronos-ecosystem/Source` + `crates/*` — not Zed, not datasets |[0m
[38;5;8m  32[0m [37m| Source edits | Only for the better; commit+report **what / why / зачем** |[0m
[38;5;8m  33[0m [37m| Facts only | Claim → Evidence → Truth base; UNVERIFIED ≠ ACCEPT |[0m
[38;5;8m  34[0m [37m| Executor | **Vision/omnimodal** for visual ACCEPT |[0m
[38;5;8m  35[0m [37m| No secrets | in report/grim/config (S3 keys = keyring only) |[0m
[38;5;8m  36[0m 
[38;5;8m  37[0m 
[38;5;8m  38[0m [37m## Phase V scope (this ticket)[0m
[38;5;8m  39[0m 
[38;5;8m  40[0m [37m| Category (mockup IA) | V behavior |[0m
[38;5;8m  41[0m [37m|----------------------|------------|[0m
[38;5;8m  42[0m [37m| Left nav | Files / Appearance / Preview / Behavior / Terminal / Plugins / Keybindings / About |[0m
[38;5;8m  43[0m [37m| **Live fields** | theme/ui/explorer (and any already-patching keys) in mockup chrome |[0m
[38;5;8m  44[0m [37m| **Not wired yet** | Same categories **shown** disabled/coming — do **not** delete IA |[0m
[38;5;8m  45[0m [37m| Search / export / reset | Chrome OK; wire only if already supported |[0m
[38;5;8m  46[0m 
[38;5;8m  47[0m [37m## Phase F residual[0m
[38;5;8m  48[0m 
[38;5;8m  49[0m [37mNew config schema keys (terminal, full preview columns, keybind capture, plugins runtime, export JSON) — separate tickets.[0m
[38;5;8m  50[0m 
[38;5;8m  51[0m [37m## Done when (V)[0m
[38;5;8m  52[0m 
[38;5;8m  53[0m [37mGrim: category nav + cards match mockup density; live toggles still persist; unwired sections visible honestly; vision ACCEPT V.[0m
[38;5;8m  54[0m 
[38;5;8m  55[0m [37m## Related[0m
[38;5;8m  56[0m 
[38;5;8m  57[0m [37mT036 · T009 settings live[0m
