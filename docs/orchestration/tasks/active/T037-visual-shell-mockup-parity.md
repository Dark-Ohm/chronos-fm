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

> ## 🔧 EXECUTOR UPDATE (2026-08-10, third pass): T044 resolved, T045 filed
>
> T044's real cause: T043's sidebar-bypass rewrite of `view.rs` dropped the
> listing/preview panels from the render tree entirely (not a layout bug —
> `listing::render` was dead code, zero call sites). Restored under
> `h_resizable`'s original two panels. Verified on 3 independent release
> grims: all 40 real entries render with populated Name/Type/Size/Modified.
> Report: `report/T044-grid-view-empty-despite-item-count-report.md`.
>
> **New blocker for full ACCEPT:** the same 3 grims show the Places sidebar
> **gone** once listing/preview are back in the tree — correlated with a
> continuous "can't render at a zero size" error storm (pre-existing,
> previously log-noise-only "T037#5") that now visibly disrupts the
> sidebar. Filed as **T045** with a hypothesis matrix, not yet root-caused.
>
> T037 stays **not closed**. Chrome + Places-text + listing-content are all
> now independently verified; T045 (sidebar-under-storm) is the one
> remaining gate for §7.

