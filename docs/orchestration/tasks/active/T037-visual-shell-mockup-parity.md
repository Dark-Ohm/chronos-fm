# T037 — Phase V: Explorer shell smart pixel-copy

> ## Эталон (SoT) — **NEW 2026-08-10**
>
> **`docs/design/mockups/chronos-file-manager.dc.html`**  
> (= `docs/design/mockups/chronos-file-manager.dc.html`, identical)  
> Previous: `archive/chronos-file-manager.v2-2026-08-09.dc.html`  
> Spec `docs/design/T037-explorer-visual-spec.md` may lag — **HTML wins** on conflict.



> ## ⚖️ ARCHITECT (2026-08-10): **PARTIAL-ACCEPT** — Places empty blocks V
>
> Multi-column list + dark + toolbar progress on `after-sidebar-fix.png`.
> Places content still empty → **T043**. Invalid grim: settled.png is not FM.
> Not closed. Report: `report/T037-visual-shell-mockup-parity-report.md`

> ## 🔧 EXECUTOR UPDATE (2026-08-10, second pass): T043 real fix + new T044
>
> T043's self-closed "FIX-ACCEPT" evidence didn't reproduce. Real root cause
> found and fixed in **Source** (`taffy.rs` T014-A frame layout memo silently
> skipping text measurement — see `docs/DECISIONS.log` 2026-08-10). Fresh
> release grim (`report-log/T037-shots/after-t014a-textmemo-fix.png`,
> `hyprctl`-verified `class=chronos-fm`) now shows: Places header + all 7
> labels + 2 devices, toolbar breadcrumb text, "40 Items" count — all
> rendering. Sidebar §7 blocker is resolved.
>
> **Still open:** same grim shows the Grid content pane empty despite
> "40 Items" — filed as **T044** (distinct bug, listing-state not text).
> T037 stays **not closed** until T044 lands; recommend architect review of
> the Places-text fix specifically (it is independently verified and solid).

