[38;5;8m   1[0m [37m# T043 — Explorer Places sidebar: panel paints empty (T037 blocker)[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m**Priority:** P1 — blocks T037 Phase V visual ACCEPT (§7 #4 Places).[0m
[38;5;8m   4[0m [37m**Source:** T037 report 2026-08-10 (Buffy) + architect vision of[0m
[38;5;8m   5[0m [37m`report-log/T037-shots/after-sidebar-fix.png`.[0m
[38;5;8m   6[0m [37m**Epic:** T042 pixel-copy / child of T037 residual.[0m
[38;5;8m   7[0m 
[38;5;8m   8[0m [37m## Symptom (facts)[0m
[38;5;8m   9[0m 
[38;5;8m  10[0m [37m- Sidebar **panel** is present (width band, `toolbar_bg`, border) on live grim.[0m
[38;5;8m  11[0m [37m- **Places content missing**: no Home/Desktop/… rows, no section header text[0m
[38;5;8m  12[0m [37m  visible on `after-sidebar-fix.png` (large empty column left of Name list).[0m
[38;5;8m  13[0m [37m- List columns Name/Type/Size/Modified **do** render (progress vs earlier Name-only).[0m
[38;5;8m  14[0m [37m- Dark theme / page-nav / footer path present on that frame.[0m
[38;5;8m  15[0m 
[38;5;8m  16[0m [37m## Hypotheses (must disprove with evidence — do not assume one)[0m
[38;5;8m  17[0m 
[38;5;8m  18[0m [37m| ID | Hypothesis | How to falsify |[0m
[38;5;8m  19[0m [37m|----|------------|----------------|[0m
[38;5;8m  20[0m [37m| H1 | `sidebar_visible` false on first paint (lifecycle race) | Log/assert `sidebar_visible` + `shortcuts.len()` on first `sidebar::render`; default is already `true` in `ExplorerPane::new` (`state.rs:235`) — race alone is **weaker** if default holds |[0m
[38;5;8m  21[0m [37m| H2 | `sidebar::render` runs but children zero-size / same-as-bg paint | Layout debug / bounds log on Places rows |[0m
[38;5;8m  22[0m [37m| H3 | `shortcuts` empty or only invisible in process env | Print `compute_shortcuts()` at runtime (Home is always pushed without exists check — expect ≥1) |[0m
[38;5;8m  23[0m [37m| H4 | Wrong entity/page in render (stale pane) | Entity id / pane index in render path |[0m
[38;5;8m  24[0m [37m| H5 | Session restore overwrites visibility/content incorrectly | Trace `restore_session` after `first_tab.update` |[0m
[38;5;8m  25[0m 
[38;5;8m  26[0m [37mExecutor T037 preferred H1; architect requires **H1–H5 inventory** before ACCEPT fix.[0m
[38;5;8m  27[0m 
[38;5;8m  28[0m [37m## Done when[0m
[38;5;8m  29[0m 
[38;5;8m  30[0m [37m1. Root cause Claim→Evidence (one H* confirmed, others falsified).[0m
[38;5;8m  31[0m [37m2. Fix: Places rows + section header visible on release grim (Home at minimum).[0m
[38;5;8m  32[0m [37m3. Active place accent bar visible when applicable.[0m
[38;5;8m  33[0m [37m4. No fake Places entries; still filter missing dirs via `exists` for non-Home.[0m
[38;5;8m  34[0m [37m5. Unit/integration test if race: first-render sees `sidebar_visible` + non-empty[0m
[38;5;8m  35[0m [37m   shortcuts path (or equivalent characterization).[0m
[38;5;8m  36[0m [37m6. Fresh grim attached under `report-log/T037-shots/` or `T043-shots/`.[0m
[38;5;8m  37[0m [37m7. Unblocks T037 re-vision §7.[0m
[38;5;8m  38[0m 
[38;5;8m  39[0m [37m## Walls[0m
[38;5;8m  40[0m 
[38;5;8m  41[0m [37m- Source truth; no Zed/datasets.[0m
[38;5;8m  42[0m [37m- Facts only; no fake capacity/trash counts.[0m
[38;5;8m  43[0m [37m- Prefer Chronos-FM fix; Source only if layout API truly broken + what/why/зачем.[0m
[38;5;8m  44[0m [37m- Vision/omnimodal on final grim.[0m
[38;5;8m  45[0m 
[38;5;8m  46[0m [37m## Related[0m
[38;5;8m  47[0m 
[38;5;8m  48[0m [37mT037 Phase V shell · T015 Places redesign (done) · T042 epic[0m
