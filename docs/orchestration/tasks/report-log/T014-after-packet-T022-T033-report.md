[38;5;8m   1[0m [37m# T014 AFTER packet — post T022+T033 (2026-08-09)[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m> ## ✅ ARCHITECT VERDICT: **ACCEPTED as calibration** (2026-08-09)[0m
[38;5;8m   4[0m [37m>[0m
[38;5;8m   5[0m [37m> Single AFTER packet after T022 (debounce 5s) + T033 (Path 4 loop kill).[0m
[38;5;8m   6[0m [37m> **T014-A (layout memoization): GO** — main-thread layout still ~50% of[0m
[38;5;8m   7[0m [37m> UI-thread self samples; process-wide % is masked by residual tantivy[0m
[38;5;8m   8[0m [37m> merge/index threads (not the watcher feedback loop).[0m
[38;5;8m   9[0m 
[38;5;8m  10[0m [37m## Setup[0m
[38;5;8m  11[0m 
[38;5;8m  12[0m [37m| Field | Value |[0m
[38;5;8m  13[0m [37m| --- | --- |[0m
[38;5;8m  14[0m [37m| Monitor | **DP-1** Samsung LC32G5xT **2560×1440@144 Hz** (confirmed `hyprctl monitors`) |[0m
[38;5;8m  15[0m [37m| Window | pid 1053772, **ws 4 mon 1**, geom **1245×692+1265+738** (tiled, not fullscreen) |[0m
[38;5;8m  16[0m [37m| Binary | `target/release/chronos-fm` (T022+T033, not stripped), build ~13:28 |[0m
[38;5;8m  17[0m [37m| Index | warm, 3651 docs; log: `Index already has 3651 documents, skipping initial indexing` |[0m
[38;5;8m  18[0m [37m| Method | `perf record -F 1000 -g -p $PID -- sleep 10` × idle / scroll / hover |[0m
[38;5;8m  19[0m [37m| Coords | Fresh `hyprctl clients -j` → `ydotool mousemove --absolute` in same shell before each mode |[0m
[38;5;8m  20[0m [37m| Artifacts | `/tmp/perf_after_{idle,scroll,hover}.data`, log `/tmp/cfm-after.log` |[0m
[38;5;8m  21[0m 
[38;5;8m  22[0m [37m**Caveats (honest):**[0m
[38;5;8m  23[0m [37m1. Window not full-screen — smaller layout tree than T014 114-item full frame; absolute taffy cost lower.[0m
[38;5;8m  24[0m [37m2. `ydotool mousemove -w` wheel CLI failed on this ydotool build (help printed); scroll mode = focus + pointer churn, **not** reliable wheel scroll. Prefer re-run scroll with PageDown keys or fixed wheel once CLI known.[0m
[38;5;8m  25[0m [37m3. Hyprland 0.56 broke legacy `hyprctl dispatch` (Lua-only); relaunch-on-focused-monitor used instead of movetoworkspace.[0m
[38;5;8m  26[0m 
[38;5;8m  27[0m [37m## Loop / watcher (T033 smoke)[0m
[38;5;8m  28[0m 
[38;5;8m  29[0m [37m| Metric | T014/T022 baseline | AFTER |[0m
[38;5;8m  30[0m [37m| --- | ---: | ---: |[0m
[38;5;8m  31[0m [37m| `Starting merge` in app log during capture window | ~1/s | **0** total in `/tmp/cfm-after.log` |[0m
[38;5;8m  32[0m [37m| `chronos_fm_services` lines | high | **1** (startup index skip only) |[0m
[38;5;8m  33[0m [37m| `notify-rs` command share (atom event) | ~30% (T014) / ~14%+ (T022 re-profile) | **~4–5%** (debou+inoti combined in top) |[0m
[38;5;8m  34[0m 
[38;5;8m  35[0m [37m**Conclusion:** consumer-side feedback loop remains **dead**. notify-rs no longer headline.[0m
[38;5;8m  36[0m 
[38;5;8m  37[0m [37m## Process-wide CPU (cpu_atom/cycles, command overhead)[0m
[38;5;8m  38[0m 
[38;5;8m  39[0m [37m| Mode | merge_thread_0 | thrd-tantivy-in | chronos-fm (main) | notify-rs* |[0m
[38;5;8m  40[0m [37m| --- | ---: | ---: | ---: | ---: |[0m
[38;5;8m  41[0m [37m| Idle | **43%** | 17% | 28% | ~5% |[0m
[38;5;8m  42[0m [37m| Scroll | **54%** | 18% | 18% | ~4% |[0m
[38;5;8m  43[0m [37m| Hover | **33%** | 29% | 22% | ~5% |[0m
[38;5;8m  44[0m 
[38;5;8m  45[0m [37m\*notify-rs debouncer/inotify threads combined ~4–5% process — **order-of-magnitude drop** vs T014 ~30%.[0m
[38;5;8m  46[0m 
[38;5;8m  47[0m [37m**Residual merge/index:** still high. This is **not** the T033 loop (no `process_changes` path in log). It is tantivy's own merge/postings work on a warm multi-segment index under load from prior sessions / background writer. Separate from watcher filter. Optional follow-up: wait until merge_thread idle **or** `rm -rf ~/.chronos-fm/index` + one full reindex + cool-down before layout-only perf.[0m
[38;5;8m  48[0m 
[38;5;8m  49[0m [37m## Layout (what matters for T014-A)[0m
[38;5;8m  50[0m 
[38;5;8m  51[0m [37m| Mode | taffy self-sum process-wide | taffy among **main-thread** self symbols |[0m
[38;5;8m  52[0m [37m| --- | ---: | ---: |[0m
[38;5;8m  53[0m [37m| Idle | ~9% | **~48%** of main self |[0m
[38;5;8m  54[0m [37m| Scroll | ~6% | **~53%** of main self |[0m
[38;5;8m  55[0m [37m| Hover | ~6% | **~47%** of main self |[0m
[38;5;8m  56[0m 
[38;5;8m  57[0m [37mT014 BEFORE quoted process-wide taffy ~40/70/65% because notify/merge mix differed. AFTER process-wide taffy is diluted by merge_thread samples; **UI-thread still spends ~half of its self samples in taffy**. That is the A-relevant signal.[0m
[38;5;8m  58[0m 
[38;5;8m  59[0m [37mTop main-thread taffy symbols (idle, atom): `compute_preliminary` ~3.7%, `compute_inner` ~1.3%, `compute_child_layout` ~1.1%, `cache_get` ~0.9%.[0m
[38;5;8m  60[0m 
[38;5;8m  61[0m [37m## Decision table (architect)[0m
[38;5;8m  62[0m 
[38;5;8m  63[0m [37m| Question | Answer |[0m
[38;5;8m  64[0m [37m| --- | --- |[0m
[38;5;8m  65[0m [37m| Is T033 loop still dead? | **Yes** (0 merges log, notify ~5%) |[0m
[38;5;8m  66[0m [37m| Did T022 debounce help? | Bundled into AFTER; cannot split; idle notify no longer ~30% |[0m
[38;5;8m  67[0m [37m| Is scroll/hover capture clean? | **Partial** — hover OK (mousemove); scroll wheel failed |[0m
[38;5;8m  68[0m [37m| **Start T014-A memoization?** | **YES** — main-thread taffy ~50% of UI self work |[0m
[38;5;8m  69[0m [37m| Block A on merge_thread quiet? | **No** — A is layout path; merge is orthogonal ticket if needed |[0m
[38;5;8m  70[0m 
[38;5;8m  71[0m [37m## Next[0m
[38;5;8m  72[0m 
[38;5;8m  73[0m [37m1. **T014-A** — layout memoization in gpui fork (~80 LOC budget from T014 recon).[0m
[38;5;8m  74[0m [37m2. Optional: AFTER-scroll re-run with working scroll input + larger window.[0m
[38;5;8m  75[0m [37m3. Optional: tantivy merge-policy / cool-down ticket if idle merge_thread stays >20% after hours.[0m
[38;5;8m  76[0m 
[38;5;8m  77[0m [37m## Files[0m
[38;5;8m  78[0m 
[38;5;8m  79[0m [37m- This report: `docs/orchestration/tasks/report/T014-after-packet-T022-T033-report.md` (inbox → architect may move to `report-log/`)[0m
[38;5;8m  80[0m [37m- Perf: `/tmp/perf_after_{idle,scroll,hover}.data`[0m
