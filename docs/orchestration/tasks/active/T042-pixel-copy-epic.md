[38;5;8m   1[0m [37m# T042 — Epic: smart pixel-copy of product mockups (Phase V)[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m**Priority:** P0 program. **Role:** index only — work lives in children.[0m
[38;5;8m   4[0m 
[38;5;8m   5[0m [37m## Strategy[0m
[38;5;8m   6[0m 
[38;5;8m   7[0m [37m| Phase | Meaning |[0m
[38;5;8m   8[0m [37m|-------|---------|[0m
[38;5;8m   9[0m [37m| **V** | Visual/IA = mockup; real data only; missing API → empty/disabled honestly |[0m
[38;5;8m  10[0m [37m| **F** | Function fill (git history, S3 transfers, plugin host…) after V ACCEPT |[0m
[38;5;8m  11[0m 
[38;5;8m  12[0m [37mMockup = product vision. Live догоняет. Icons: `crates/chronos-fm-ui/assets/icons/`.  [0m
[38;5;8m  13[0m [37mSource truth: `/home/neo/projects/chronos-ecosystem/Source`. Facts only. Vision for visual ACCEPT.[0m
[38;5;8m  14[0m 
[38;5;8m  15[0m [37m## Children[0m
[38;5;8m  16[0m 
[38;5;8m  17[0m [37m### Phase V pages[0m
[38;5;8m  18[0m 
[38;5;8m  19[0m [37m| Ticket | Surface | Mockup | Status |[0m
[38;5;8m  20[0m [37m|--------|---------|--------|--------|[0m
[38;5;8m  21[0m [37m| **T037** | Explorer shell | `chronos-file-manager.dc.html` | **done ACCEPT** |[0m
[38;5;8m  22[0m [37m| **T038** | Git | `Chronos-Git-Tab.dc.html` | **PARTIAL-ACCEPT** (code; vision residual) |[0m
[38;5;8m  23[0m [37m| **T039** | S3 | `Chronos-S3-Tab.dc.html` | **PARTIAL-ACCEPT** (code; vision+RustFS residual) |[0m
[38;5;8m  24[0m [37m| **T040** | Settings | `Chronos-File-Manager-Settings.dc.html` | **PARTIAL-ACCEPT** (code; vision residual) |[0m
[38;5;8m  25[0m [37m| **T041** | Extensions | `Chronos-Extensions-Tab.dc.html` | **PARTIAL-ACCEPT** (code; vision residual) |[0m
[38;5;8m  26[0m 
[38;5;8m  27[0m [37m### Residuals[0m
[38;5;8m  28[0m 
[38;5;8m  29[0m [37m| Ticket | Issue | Status |[0m
[38;5;8m  30[0m [37m|--------|-------|--------|[0m
[38;5;8m  31[0m [37m| **T043** | Places empty / text missing | **done** |[0m
[38;5;8m  32[0m [37m| **T044** | Listing not in tree / empty pane | **done** |[0m
[38;5;8m  33[0m [37m| **T045** | `snapshot_memo` sibling poison | **done ACCEPT** (Source fix) |[0m
[38;5;8m  34[0m [37m| **T046** | Shared visual proof (hypr focus / `--page=`) | **active** — unblocks vision for T038–T041 |[0m
[38;5;8m  35[0m 
[38;5;8m  36[0m [37m## Order[0m
[38;5;8m  37[0m 
[38;5;8m  38[0m [37m1. ~~T037 / T043–T045~~ **done**[0m
[38;5;8m  39[0m [37m2. T038–T041 Phase V **code PARTIAL-ACCEPT** — residual **T046**[0m
[38;5;8m  40[0m [37m3. Phase F per page after vision or client waiver[0m
[38;5;8m  41[0m [37m4. Optional: `--page=<name>` CLI for scripted grims[0m
[38;5;8m  42[0m 
[38;5;8m  43[0m [37m## Non-negotiables[0m
[38;5;8m  44[0m 
[38;5;8m  45[0m [37m- Source edits only better + what/why/зачем[0m
[38;5;8m  46[0m [37m- `class=chronos-fm` grims only[0m
[38;5;8m  47[0m [37m- No secrets in artifacts[0m
[38;5;8m  48[0m [37m- Executors do not self-ACCEPT[0m
[38;5;8m  49[0m 
[38;5;8m  50[0m [37m## Done when (epic)[0m
[38;5;8m  51[0m 
[38;5;8m  52[0m [37mAll five page children have full Phase V ACCEPT (vision) or PARTIAL + residual closed/waived. Phase F separate.[0m
