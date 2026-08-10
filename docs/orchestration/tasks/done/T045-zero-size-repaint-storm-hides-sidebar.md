[38;5;8m   1[0m [37m# T045 — Zero-size repaint storm disrupts sidebar rendering[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m> ## ✅ ARCHITECT VERDICT: **ACCEPT / CLOSED** (2026-08-10)[0m
[38;5;8m   4[0m [37m>[0m
[38;5;8m   5[0m [37m> **Root cause:** `snapshot_memo` walked global `frame_node_order` after nested[0m
[38;5;8m   6[0m [37m> `VirtualList::measure_item` → `compute_layout`, caching Taffy default[0m
[38;5;8m   7[0m [37m> bounds for not-yet-computed siblings (Places SVG/text).[0m
[38;5;8m   8[0m [37m>[0m
[38;5;8m   9[0m [37m> **Fix (Source):** `gpui/src/taffy.rs` — `snapshot_memo(root, …)` only forces[0m
[38;5;8m  10[0m [37m> `layout_bounds` for `subtree_ids(root)`. Nested probes cannot poison[0m
[38;5;8m  11[0m [37m> siblings. Unit tests + `gpui-component/examples/t045_repro`.[0m
[38;5;8m  12[0m [37m>[0m
[38;5;8m  13[0m [37m> **Live proof:** grim `report-log/T045-shots/t045_fixed_final.png` — Places[0m
[38;5;8m  14[0m [37m> (7 + 2 devices) + 40-item listing together; `class=chronos-fm`; zero[0m
[38;5;8m  15[0m [37m> `can't render at a zero size` in log. Vision architect review: OK.[0m
[38;5;8m  16[0m [37m>[0m
[38;5;8m  17[0m [37m> Unblocks T037 Phase V shell. Reports: `report-log/T045-…` (move with hygiene).[0m
[38;5;8m  18[0m 
[38;5;8m  19[0m [37m**Priority was:** P1. **Epic:** T042 residual of T037.[0m
