[38;5;8m   1[0m [37m# T034 — Page-nav rail: no hit-test / invisible icons (blocks live Git tab)[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m**Приоритет:** P1 for live verification harness; not a product feature.[0m
[38;5;8m   4[0m [37m**Источник:** T010-B live report (2026-08-09) — Path A accept of B still[0m
[38;5;8m   5[0m [37mleft visual Git-tab confirm blocked.[0m
[38;5;8m   6[0m 
[38;5;8m   7[0m [37m## Symptom[0m
[38;5;8m   8[0m 
[38;5;8m   9[0m [37mOn a live debug `chronos-fm` window (e.g. ~1922×1180, second monitor):[0m
[38;5;8m  10[0m 
[38;5;8m  11[0m [37m- Leftmost ~64 px column (page-nav icon rail) samples as **zero non-bg[0m
[38;5;8m  12[0m [37m  pixels** at y∈[18,1180].[0m
[38;5;8m  13[0m [37m- Hover over inferred button centres → `mean=0 max=0` pixel delta.[0m
[38;5;8m  14[0m [37m- Clicks at (x≈32, y=80..280) **do not switch page**.[0m
[38;5;8m  15[0m [37m- Same input channel works for explorer file rows (preview + footer[0m
[38;5;8m  16[0m [37m  update on click) — so ydotool/focus are fine.[0m
[38;5;8m  17[0m 
[38;5;8m  18[0m [37m## Plausible causes (from T010-B report)[0m
[38;5;8m  19[0m 
[38;5;8m  20[0m [37m1. Rail not in render tree / hitbox outside 0..64 px.[0m
[38;5;8m  21[0m [37m2. Icons same colour as `toolbar_bg` **and** listener never fires[0m
[38;5;8m  22[0m [37m   (hover also dead → more than just paint).[0m
[38;5;8m  23[0m [37m3. Parent swallows mouse events (regression in `root.rs` page switcher).[0m
[38;5;8m  24[0m 
[38;5;8m  25[0m [37m## Acceptance[0m
[38;5;8m  26[0m 
[38;5;8m  27[0m [37m1. Identify which cause in `crates/chronos-fm-pages/src/root.rs` (or[0m
[38;5;8m  28[0m [37m   theme/toolbar) with grim + optional hit-test dump.[0m
[38;5;8m  29[0m [37m2. Fix so: hover paints, click switches Explorer ↔ Git ↔ Settings ↔ …[0m
[38;5;8m  30[0m [37m3. Live: open Git page from rail; OCR or screenshot shows "Branches" /[0m
[38;5;8m  31[0m [37m   commit bar.[0m
[38;5;8m  32[0m [37m4. Regression note: T010-B visual confirm can re-run after this.[0m
[38;5;8m  33[0m 
[38;5;8m  34[0m [37m## Not in scope[0m
[38;5;8m  35[0m 
[38;5;8m  36[0m [37m- T010-C push/pull[0m
[38;5;8m  37[0m [37m- Syntect[0m
[38;5;8m  38[0m [37m- Redesign of nav chrome[0m
[38;5;8m  39[0m 
[38;5;8m  40[0m [37m## Coordination[0m
[38;5;8m  41[0m 
[38;5;8m  42[0m [37m- Blocks **live** re-verify of T010-B only; T010-B code accept stands.[0m
[38;5;8m  43[0m [37m- Discovered during T010-B harness, not introduced as T010 content.[0m
