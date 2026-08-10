[38;5;8m   1[0m [37m# AGENTS — Chronos-FM[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37mProject-specific agent rules. Read **`HANDOFF.md` first** on a new session[0m
[38;5;8m   4[0m [37m(latest checkpoint is at the top). Generic coding defaults may still live in[0m
[38;5;8m   5[0m [37m`.rules` / tool config; **this file wins** on Chronos-FM product process.[0m
[38;5;8m   6[0m 
[38;5;8m   7[0m [37m## What this repo is[0m
[38;5;8m   8[0m 
[38;5;8m   9[0m [37mRust **file manager** (gpui / ChronOS design language) under[0m
[38;5;8m  10[0m [37m`/home/neo/projects/chronos-ecosystem/Chronos-FM`.  [0m
[38;5;8m  11[0m [37mShared UI fork: **`/home/neo/projects/chronos-ecosystem/Source`** via path-deps[0m
[38;5;8m  12[0m [37m`../Source/*` in workspace `Cargo.toml`.[0m
[38;5;8m  13[0m 
[38;5;8m  14[0m [37m## Roles[0m
[38;5;8m  15[0m 
[38;5;8m  16[0m [37m| Role | Does |[0m
[38;5;8m  17[0m [37m|------|------|[0m
[38;5;8m  18[0m [37m| **Client** | Product vision (mockups), priorities |[0m
[38;5;8m  19[0m [37m| **Architect** | Decide, accept/reject reports, ticket hygiene — not default implementer |[0m
[38;5;8m  20[0m [37m| **Executor** | Implement, release binary, grim, evidence — **no self-ACCEPT** |[0m
[38;5;8m  21[0m 
[38;5;8m  22[0m [37m## Canonical sources of truth[0m
[38;5;8m  23[0m 
[38;5;8m  24[0m [37m| Concern | Authority |[0m
[38;5;8m  25[0m [37m|---------|-----------|[0m
[38;5;8m  26[0m [37m| Product UI vision | `docs/design/mockups/*.html` (shell: `chronos-file-manager.dc.html`) |[0m
[38;5;8m  27[0m [37m| gpui / component API | **`Source/` only** — not Zed, not RAG/datasets, not “memory” |[0m
[38;5;8m  28[0m [37m| App behavior | `crates/*` + live config/runtime |[0m
[38;5;8m  29[0m [37m| Session memory | `HANDOFF.md` (top checkpoint) + `docs/DECISIONS.log` |[0m
[38;5;8m  30[0m [37m| Work queue | `docs/orchestration/tasks/active/` |[0m
[38;5;8m  31[0m 
[38;5;8m  32[0m [37mIf mockup HTML and an older written spec disagree → **HTML wins** until[0m
[38;5;8m  33[0m [37mspec is re-extracted.[0m
[38;5;8m  34[0m 
[38;5;8m  35[0m [37m## UI delivery strategy (locked 2026-08-10)[0m
[38;5;8m  36[0m 
[38;5;8m  37[0m [37m1. **Phase V — smart pixel-copy:** match mockup layout/IA/density; real data[0m
[38;5;8m  38[0m [37m   only; missing backend → empty/disabled UI (no fake progress/history).[0m
[38;5;8m  39[0m [37m2. **Phase F — function fill:** separate tickets after V ACCEPT.[0m
[38;5;8m  40[0m [37m3. Icons only from `crates/chronos-fm-ui/assets/icons/`.[0m
[38;5;8m  41[0m [37m4. Epic: **T042**. Children: T037 shell, T038 Git, T039 S3, T040 Settings,[0m
[38;5;8m  42[0m [37m   T041 Extensions. Residuals: T044 (list empty), etc.[0m
[38;5;8m  43[0m 
[38;5;8m  44[0m [37m## Evidence (ACCEPT)[0m
[38;5;8m  45[0m 
[38;5;8m  46[0m [37mEvery accept-critical claim:[0m
[38;5;8m  47[0m 
[38;5;8m  48[0m [37m```[0m
[38;5;8m  49[0m [37mClaim: …[0m
[38;5;8m  50[0m [37mEvidence: path:lines | command+exit | grim path | config key (no secrets)[0m
[38;5;8m  51[0m [37mTruth base: Source | Chronos-FM | mockup | runtime | config[0m
[38;5;8m  52[0m [37m```[0m
[38;5;8m  53[0m 
[38;5;8m  54[0m [37m- Visual ACCEPT requires **release binary + grim** of `class=chronos-fm`[0m
[38;5;8m  55[0m [37m  (set `app_id` in `chronos-fm-ui` window options) **and** vision review.[0m
[38;5;8m  56[0m [37m- Unit green alone is not enough for window/UX.[0m
[38;5;8m  57[0m [37m- Do not file browser/other app screenshots as Chronos-FM evidence.[0m
[38;5;8m  58[0m 
[38;5;8m  59[0m [37m## Source edits[0m
[38;5;8m  60[0m 
[38;5;8m  61[0m [37mAllowed when they make Chronos-FM (and the shared fork) **better**[0m
[38;5;8m  62[0m [37m(bugfix, missing API, layout/theme hook, crash, perf).  [0m
[38;5;8m  63[0m [37mEach Source change: **what / why / for what** in commit + report.  [0m
[38;5;8m  64[0m [37mNo silent fork churn, no Zed copy-paste as authority.[0m
[38;5;8m  65[0m 
[38;5;8m  66[0m [37m## Orchestration hygiene[0m
[38;5;8m  67[0m 
[38;5;8m  68[0m [37m- Tickets: `docs/orchestration/tasks/active|done|report|report-log|rejected/`[0m
[38;5;8m  69[0m [37m- Architect stamps reports; executors do not self-close ACCEPT.[0m
[38;5;8m  70[0m [37m- Stage **by file**, never whole `active/` directories (history: buried tickets).[0m
[38;5;8m  71[0m [37m- `git push` only when the user asks.[0m
[38;5;8m  72[0m 
[38;5;8m  73[0m [37m## Build / run (Linux Hyprland)[0m
[38;5;8m  74[0m 
[38;5;8m  75[0m [37m```bash[0m
[38;5;8m  76[0m [37mcargo build --release -p chronos-fm[0m
[38;5;8m  77[0m [37m./target/release/chronos-fm   # log optional; match hypr class chronos-fm[0m
[38;5;8m  78[0m [37m```[0m
[38;5;8m  79[0m 
[38;5;8m  80[0m [37mPrefer verifying GPU path (Vulkan) and window mapping before claiming UI done.[0m
[38;5;8m  81[0m 
[38;5;8m  82[0m [37m## Do not[0m
[38;5;8m  83[0m 
[38;5;8m  84[0m [37m- Invent APIs from Zed docs.[0m
[38;5;8m  85[0m [37m- Fake S3 transfer progress or git history for screenshots.[0m
[38;5;8m  86[0m [37m- Put AWS secrets in `config.toml` or reports (keyring only).[0m
[38;5;8m  87[0m [37m- Drop mockup sections because “live is thinner”.[0m
[38;5;8m  88[0m [37m- Push origin or force-push without explicit user request.[0m
[38;5;8m  89[0m 
[38;5;8m  90[0m [37m## Quick links[0m
[38;5;8m  91[0m 
[38;5;8m  92[0m [37m- `HANDOFF.md` — session checkpoint (read first)[0m
[38;5;8m  93[0m [37m- `docs/DECISIONS.log` — ratified decisions[0m
[38;5;8m  94[0m [37m- `docs/design/mockups/README.md` — mockup index[0m
[38;5;8m  95[0m [37m- `docs/architecture.md` — crate map[0m
[38;5;8m  96[0m [37m- `docs/orchestration/tasks/active/` — current work[0m
