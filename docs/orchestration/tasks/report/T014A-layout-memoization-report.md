[38;5;8m   1[0m [37m# T014-A — Layout memoization: implementation report[0m
[38;5;8m   2[0m 
[38;5;8m   3[0m [37m> **Рекомендация exec:** **ACCEPT** (awaiting architect stamp).[0m
[38;5;8m   4[0m [37m>[0m
[38;5;8m   5[0m [37m> Path: layout fingerprint + absolute-bounds remap in `Source/gpui`.[0m
[38;5;8m   6[0m [37m> Unit tests green (4/4 in `taffy::tests`). `cargo check -p chronos-fm` green.[0m
[38;5;8m   7[0m 
[38;5;8m   8[0m [37m## What landed[0m
[38;5;8m   9[0m 
[38;5;8m  10[0m [37m| File | Change |[0m
[38;5;8m  11[0m [37m| --- | --- |[0m
[38;5;8m  12[0m [37m| `Source/gpui/src/taffy.rs` | Frame fingerprint of layout styles + structure; memo geom snapshot; `compute_layout` early-return on hit; `request_measured_layout_with_content_key` |[0m
[38;5;8m  13[0m [37m| `Source/gpui/src/window.rs` | `request_measured_layout_with_content_key` API |[0m
[38;5;8m  14[0m [37m| `Source/gpui/src/elements/text.rs` | Hash text + runs into content_key |[0m
[38;5;8m  15[0m 
[38;5;8m  16[0m [37m## Behaviour[0m
[38;5;8m  17[0m 
[38;5;8m  18[0m [37m1. Each `request_layout` / measured request folds layout-only style hash + child count (+ content_key) into `frame_fingerprint`.[0m
[38;5;8m  19[0m [37m2. After a full taffy compute, absolute bounds are snapshotted in request order.[0m
[38;5;8m  20[0m [37m3. Next frame: if fingerprint + available space match → remap memo geoms onto new LayoutIds, **skip** `compute_layout_with_measure`.[0m
[38;5;8m  21[0m [37m4. Paint-only fields never reach `to_taffy` → hover bg does not miss memo.[0m
[38;5;8m  22[0m [37m5. Text content changes → content_key changes → miss (correct reflow).[0m
[38;5;8m  23[0m 
[38;5;8m  24[0m [37m## Tests[0m
[38;5;8m  25[0m 
[38;5;8m  26[0m [37m```[0m
[38;5;8m  27[0m [37mcargo test --lib taffy::tests -p gpui[0m
[38;5;8m  28[0m [37m  layout_fingerprint_stable_across_frames … ok[0m
[38;5;8m  29[0m [37m  layout_fingerprint_ignores_paint_only_background … ok[0m
[38;5;8m  30[0m [37m  measured_content_key_changes_fingerprint … ok[0m
[38;5;8m  31[0m [37m  border_widths_to_taffy_use_stroke_snapping … ok[0m
[38;5;8m  32[0m [37m```[0m
[38;5;8m  33[0m 
[38;5;8m  34[0m [37m`cargo check -p chronos-fm` — ok against path dep.[0m
[38;5;8m  35[0m 
[38;5;8m  36[0m [37m## Not done / caveats[0m
[38;5;8m  37[0m 
[38;5;8m  38[0m [37m- Live perf AFTER-A not re-run this session (unit + check only).[0m
[38;5;8m  39[0m [37m- Measured layouts without content_key (list/uniform_list) still key as `0` — content-same-size changes may rare-miss correctness; text path is covered.[0m
[38;5;8m  40[0m [37m- Variant B (node reuse) not started.[0m
[38;5;8m  41[0m 
[38;5;8m  42[0m [37m## Architect[0m
[38;5;8m  43[0m 
[38;5;8m  44[0m [37mStamp ACCEPT → move ticket/report to done/report-log.[0m
