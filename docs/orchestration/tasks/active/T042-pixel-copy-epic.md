[38;5;8m   1[0m [37m# T042 — Epic: smart pixel-copy of product mockups (Phase V)[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m**Приоритет:** P0 program (orders visual delivery for all page shells).[0m
[38;5;8m   4[0m [37m**Роль:** epic / index — implementation lives in child tickets.[0m
[38;5;8m   5[0m [37m**Стратегия:** visual/IA first → function later. Mockup = vision SoT.[0m
[38;5;8m   6[0m 
[38;5;8m   7[0m [37m## Children (Phase V)[0m
[38;5;8m   8[0m 
[38;5;8m   9[0m [37m| Ticket | Surface | Mockup |[0m
[38;5;8m  10[0m [37m|--------|---------|--------|[0m
[38;5;8m  11[0m [37m| **T037** | Explorer shell (FM window) | `docs/design/mockups/chronos-file-manager.dc.html` (rev **2026-08-10**, also `chronos-file-manager.dc.html`) + `docs/design/T037-explorer-visual-spec.md` |[0m
[38;5;8m  12[0m [37m| **T038** | Git tab | `docs/design/mockups/Chronos-Git-Tab.dc.html` |[0m
[38;5;8m  13[0m [37m| **T039** | S3 tab | `docs/design/mockups/Chronos-S3-Tab.dc.html` |[0m
[38;5;8m  14[0m [37m| **T040** | Settings tab | `docs/design/mockups/Chronos-File-Manager-Settings.dc.html` |[0m
[38;5;8m  15[0m [37m| **T041** | Extensions tab |
| **T043** | Places sidebar empty (T037 blocker) | (no mockup — residual) | `docs/design/mockups/Chronos-Extensions-Tab.dc.html` |[0m
[38;5;8m  16[0m 
[38;5;8m  17[0m [37m## Order (recommended)[0m
[38;5;8m  18[0m 
[38;5;8m  19[0m [37m1. **T037** shell — unblocks shared chrome language; window `app_id` already fixed.[0m
[38;5;8m  20[0m [37m2. **T038 / T039 / T040 / T041** in parallel after shell tokens stable (or serial if one executor).[0m
[38;5;8m  21[0m [37m3. Phase F residuals only after each V grim ACCEPT (or explicit PARTIAL with residual list).[0m
[38;5;8m  22[0m 
[38;5;8m  23[0m [37m## Shared rules[0m
[38;5;8m  24[0m 
[38;5;8m  25[0m 
[38;5;8m  26[0m [37m## Strategy: smart pixel-copy (Phase V) first[0m
[38;5;8m  27[0m 
[38;5;8m  28[0m [37m**Agreed (client + architect):** mockup = product vision. Live may lag.[0m
[38;5;8m  29[0m [37mDelivery: **Phase V = visual/IA copy**, then **Phase F = function fill**.[0m
[38;5;8m  30[0m 
[38;5;8m  31[0m [37m### Phase V (THIS ticket — Must)[0m
[38;5;8m  32[0m 
[38;5;8m  33[0m [37m1. Layout/IA/density **match mockup** (shell chrome, nav, panels, empty states).[0m
[38;5;8m  34[0m [37m2. Wire **already-real** data/actions only (no fake history/transfers/progress).[0m
[38;5;8m  35[0m [37m3. Views without backend: **render chrome + honest empty/disabled** ("needs …"), do not delete from IA.[0m
[38;5;8m  36[0m [37m4. Icons: `crates/chronos-fm-ui/assets/icons/` only — not mockup SVG paths.[0m
[38;5;8m  37[0m [37m5. Theme: ChronOS dark tokens (`chronos.theme.json` / `chronos_fm_ui::theme`).[0m
[38;5;8m  38[0m [37m6. Proof: release binary + grim + **vision** review vs mockup.[0m
[38;5;8m  39[0m [37m7. Do **not** mix mega-backend (multipart, git log engine, plugin host) into V PR.[0m
[38;5;8m  40[0m 
[38;5;8m  41[0m [37m### Phase F (residual / follow-up tickets)[0m
[38;5;8m  42[0m 
[38;5;8m  43[0m [37mBackend + full interactivity for mockup-only views. Separate tickets after V ACCEPT.[0m
[38;5;8m  44[0m 
[38;5;8m  45[0m [37m### Non-negotiables[0m
[38;5;8m  46[0m 
[38;5;8m  47[0m [37m| Rule | Detail |[0m
[38;5;8m  48[0m [37m|------|--------|[0m
[38;5;8m  49[0m [37m| Source truth | `/home/neo/projects/chronos-ecosystem/Source` + `crates/*` — not Zed, not datasets |[0m
[38;5;8m  50[0m [37m| Source edits | Only for the better; commit+report **what / why / зачем** |[0m
[38;5;8m  51[0m [37m| Facts only | Claim → Evidence → Truth base; UNVERIFIED ≠ ACCEPT |[0m
[38;5;8m  52[0m [37m| Executor | **Vision/omnimodal** for visual ACCEPT |[0m
[38;5;8m  53[0m [37m| No secrets | in report/grim/config (S3 keys = keyring only) |[0m
[38;5;8m  54[0m 
[38;5;8m  55[0m 
[38;5;8m  56[0m [37m## Blockers[0m
[38;5;8m  57[0m 
[38;5;8m  58[0m [37m- Shared window/lifecycle: prefer `class=chronos-fm` for hypr/grim targeting.[0m
[38;5;8m  59[0m [37m- Do not wait on full option-C backends before V chrome.[0m
[38;5;8m  60[0m 
[38;5;8m  61[0m [37m## Done when (epic)[0m
[38;5;8m  62[0m 
[38;5;8m  63[0m [37mAll five children have **visual Phase V ACCEPT** (or PARTIAL with residual tickets filed). Phase F tracked separately.[0m
