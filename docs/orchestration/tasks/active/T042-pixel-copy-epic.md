   1 # T042 — Epic: smart pixel-copy of product mockups (Phase V)
   2 
   3 **Priority:** P0 program. **Role:** index only — work lives in children.
   4 
   5 ## Strategy
   6 
   7 | Phase | Meaning |
   8 |-------|---------|
   9 | **V** | Visual/IA = mockup; real data only; missing API → empty/disabled honestly |
  10 | **F** | Function fill (git history, S3 transfers, plugin host…) after V ACCEPT |
  11 
  12 Mockup = product vision. Live догоняет. Icons: `crates/chronos-fm-ui/assets/icons/`.  
  13 Source truth: `/home/neo/projects/chronos-ecosystem/Source`. Facts only. Vision for visual ACCEPT.
  14 
  15 ## Children
  16 
  17 ### Phase V pages
  18 
  19 | Ticket | Surface | Mockup | Status |
  20 |--------|---------|--------|--------|
  21 | **T037** | Explorer shell | `chronos-file-manager.dc.html` | **done ACCEPT** |
  22 | **T038** | Git | `Chronos-Git-Tab.dc.html` | **PARTIAL-ACCEPT** (code; vision residual) |
  23 | **T039** | S3 | `Chronos-S3-Tab.dc.html` | **PARTIAL-ACCEPT** (code; vision+RustFS residual) |
  24 | **T040** | Settings | `Chronos-File-Manager-Settings.dc.html` | **PARTIAL-ACCEPT** (code; vision residual) |
  25 | **T041** | Extensions | `Chronos-Extensions-Tab.dc.html` | **PARTIAL-ACCEPT** (code; vision residual) |
  26 
  27 ### Residuals (shell)
  28 
  29 | Ticket | Issue | Status |
  30 |--------|-------|--------|
  31 | **T043** | Places empty / text missing | **done** |
  32 | **T044** | Listing not in tree / empty pane | **done** |
  33 | **T045** | `snapshot_memo` sibling poison | **done ACCEPT** (Source fix) |
  34 
  35 ## Order
  36 
  37 1. **T045** → unblocks T037 §7  
  38 2. **T037** ACCEPT → shell language stable  
  39 3. **T038–T041** Phase V (parallel or serial)  
  40 4. Phase F per page after each V ACCEPT  
  41 
  42 ## Non-negotiables
  43 
  44 - Source edits only better + what/why/зачем  
  45 - `class=chronos-fm` grims only  
  46 - No secrets in artifacts  
  47 - Executors do not self-ACCEPT  
  48 
  49 ## Done when (epic)
  50 
  51 All five page children have Phase V ACCEPT (or PARTIAL + residual tickets). Phase F separate.
