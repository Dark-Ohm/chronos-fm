[38;5;8m   1[0m [37m# T037 — Phase V: Explorer shell smart pixel-copy[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m**Epic:** T042. **Priority:** P1.[0m
[38;5;8m   4[0m 
[38;5;8m   5[0m [37m## Эталон (SoT)[0m
[38;5;8m   6[0m 
[38;5;8m   7[0m [37m**`docs/design/mockups/chronos-file-manager.dc.html`** (2026-08-10)  [0m
[38;5;8m   8[0m [37mArchive: `archive/chronos-file-manager.v2-2026-08-09.dc.html`  [0m
[38;5;8m   9[0m [37mSpec `docs/design/T037-explorer-visual-spec.md` may lag — **HTML wins**.[0m
[38;5;8m  10[0m 
[38;5;8m  11[0m [37m## Architect status (2026-08-10)[0m
[38;5;8m  12[0m 
[38;5;8m  13[0m [37m| Gate | State |[0m
[38;5;8m  14[0m [37m|------|--------|[0m
[38;5;8m  15[0m [37m| Window `app_id=chronos-fm` | **done** |[0m
[38;5;8m  16[0m [37m| Dark theme / density / icons | **landed** (code) |[0m
[38;5;8m  17[0m [37m| Places **text** (T043) | **done** — Source T014-A memo + resizable path; grim verified |[0m
[38;5;8m  18[0m [37m| Listing content (T044) | **done** — `listing::render` was dropped from tree; restored under `h_resizable` |[0m
[38;5;8m  19[0m [37m| Places **alongside** listing | **blocked by T045** — zero-size repaint storm; sidebar missing when full tree present |[0m
[38;5;8m  20[0m [37m| §7 full ACCEPT | **HOLD** |[0m
[38;5;8m  21[0m 
[38;5;8m  22[0m [37m**Verdict: PARTIAL — not closed.**  [0m
[38;5;8m  23[0m [37mOnly remaining gate for Phase V ACCEPT: **T045**.[0m
[38;5;8m  24[0m 
[38;5;8m  25[0m [37m## Phase V Must (still)[0m
[38;5;8m  26[0m 
[38;5;8m  27[0m [37mMatch mockup shell: title, page-nav, address, Places, list columns, preview, footer — **all visible together** on release grim `class=chronos-fm`, vision §7.[0m
[38;5;8m  28[0m 
[38;5;8m  29[0m [37m## Strategy[0m
[38;5;8m  30[0m 
[38;5;8m  31[0m [37mSmart pixel-copy (Phase V) first — see T042. Icons ours only. Facts only. No self-ACCEPT.[0m
[38;5;8m  32[0m 
[38;5;8m  33[0m [37m## Related[0m
[38;5;8m  34[0m 
[38;5;8m  35[0m [37m- **T045** (blocker) — zero-size storm / sidebar under full tree  [0m
[38;5;8m  36[0m [37m- T043 done · T044 done  [0m
[38;5;8m  37[0m [37m- Reports: `report/T037-…`, `report/T044-…`, `report/T045-…`  [0m
[38;5;8m  38[0m [37m- Shots: `report-log/T037-shots/`, `T044-shots/`, `T045-shots/`[0m
