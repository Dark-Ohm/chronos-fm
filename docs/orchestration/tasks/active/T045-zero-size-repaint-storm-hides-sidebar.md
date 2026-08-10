# T045 — Zero-size repaint storm disrupts sidebar rendering (was T037#5)

> ## ⚖️ ARCHITECT (2026-08-10, pass 5): **ROOT CAUSE ACCEPTED — fix not yet shipped**
>
> Isolated repro + Source call-chain review accepted. Not ACCEPT until fix
> + tests + grim (Places + listing together, storm gone or proven benign).
>
> ### Root cause (facts)
> `v_virtual_list::measure_item` → nested `layout_as_root` / `compute_layout`
> mid-`request_layout` → on miss, `TaffyLayoutEngine::snapshot_memo` walks
> **global** `frame_node_order` and `layout_bounds()`-caches Taffy defaults
> `(0,0)/(0,0)` for **siblings already registered but not yet computed**
> this frame (sidebar icons/text). Real outer `compute_layout` later does
> not invalidate those wrapper cache entries → paint forever reads zeros
> for the rest of the frame (repeats every frame). Log site remains
> `svg_renderer.rs:202` on poisoned zero SVG bounds.
>
> Explains H4 (no virtual list → no storm), H1/H5a false, ablation without
> `h_resizable`. Repro: `Source/gpui-component/examples/t045_repro/`
> (uncommitted; keep until fix lands).
>
> ### Approved fix direction (minimal, Source)
> **Preferred:** `snapshot_memo` only snapshots the **subtree of the root
> just computed** (pass `root: LayoutId` from `compute_layout`), not the
> whole `frame_node_order`. Nested measure then cannot poison siblings.
>
> **Acceptable alternative:** skip `snapshot_memo` for nested/measure
> `compute_layout` (nest depth / flag), *or* refuse to **cache**
> `layout_bounds` for nodes whose Taffy layout is still uncomputed
> (must not break legit zero-size widgets).
>
> **Required verification:**
> 1. Unit test in `gpui` taffy: nested compute must not leave sibling
>    absolute bounds at default; full compute then paints non-zero.
> 2. `t045_repro` example green (no zero-size storm / icons lay out).
> 3. Chronos-FM release grim: Places + listing co-visible; `class=chronos-fm`.
> 4. Existing T014-A memo tests still green (`cargo test -p gpui --lib taffy`).
>
> **Do not** blind-patch Chronos-FM `view.rs` around the storm.
> **Do not** self-ACCEPT. Source edits: what/why/зачем in commit.
>
> Prior passes (H1/H4/H5a, frame-trace) remain valid narrowing.

**Priority:** P1 — blocks T037 §7.
**Epic:** T042 residual.

## Symptom (summary)

Places sidebar missing when listing present; sustained
`can't render at a zero size` from SVG paint on zero bounds; listing OK.

## Closed hypotheses

| ID | Result |
|----|--------|
| H1 resizable notify loop | **FALSIFIED** |
| H4 storm without virtual list | **FALSIFIED** (then refined: no measure_item) |
| H5a offset-stack leak | **FALSIFIED** (static) |
| Double paint via Svg::paint | **RETRACTED** — house.svg all zeros on that path |
| **Root cause** | **CONFIRMED** — `snapshot_memo` global walk poisons uncomputed siblings |

## Open work

1. Implement preferred Source fix + unit test.
2. Land / trim `examples/t045_repro` as regression example if useful.
3. FM grim proof → architect vision → unblock T037.

## Related

T037 · T043 · T044 · T014-A memo · `DECISIONS.log` · commit trail through `5305aba` + RC write-up.
