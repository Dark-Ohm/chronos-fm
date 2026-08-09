# T035 — Fork: text measurement panic in examples (`text.rs:777`)

**Приоритет:** P1 for fork health (shell/IDE examples, possible Chronos-FM
risk if path is shared). **Скоуп:** `Source/gpui` (Wall 1 of T028 lifted —
this ticket owns the fix).
**Источник:** T028 PARTIAL-ACCEPT (2026-08-09).

## Symptom

9/11 `gpui-component` examples panic at runtime:

```
Source/gpui/src/elements/text.rs:777:14
measurement has not been performed on <string>
```

Examples that fail: hello_world, focus_trap, tooltip_top_edge, dialog_overlay,
text_selection, input, window_title, system_monitor, root_borderless.
Survivors: sidebar, app_assets. webview: Gtk-CRITICAL only (no panic).

## Done when

1. Root cause documented (layout/measure order vs paint).
2. Fix in `Source/gpui` (or proven example-only misuse with patch upstreamable).
3. At least the five shell-relevant examples run without panic:
   `focus_trap`, `tooltip_top_edge`, `dialog_overlay`, `sidebar`, `text_selection`.
4. Chronos-FM still builds against the fixed gpui.

## Walls

- Coordinates multi-product: ChronOS, greeter, FM, IDE share Source/gpui —
  keep the fix minimal and covered by a unit/integration test if possible.
