# T050 - Marquee multi-select report

> ## ✅ ARCHITECT VERDICT (2026-08-10): **ACCEPT / CLOSED**
>
> ### Re-verified on main `f6c8f99` (FF merge)
> - `cargo test -p chronos-fm-pages --lib marquee` → **22 passed, 0 failed**
> - Report + workspace tests claimed green by executor; release build claimed OK
> - Live grims `class=chronos-fm`: list (6 selected + dashed overlay), grid
>   (5 selected + overlay) under `report-log/T050-marquee-{list,grid}.png`
>
> ### Spec match (2026-08-10-marquee-multi-select-design.md)
> Measured bounds + geometry token; empty-space start; replace/Ctrl-additive;
> 4 px threshold; min/max completion; sole `selection` model for T051 — land.
>
> ### Residual (non-blocking, per report)
> No off-screen virtual selection / auto-scroll; DnD T051; Shift+Arrow residual.
>
> Ticket → `done/T050-marquee-multi-select.md`. Epic T048: next **T051** in-app DnD.



**Status:** IMPLEMENTED - awaiting Architect review. Executor does not self-ACCEPT.

## Outcome

Explorer List and Grid now support a standard empty-space drag marquee. The
drag uses the existing `ExplorerPane::selection` index set, updates selection
live after a 4 px threshold, and paints a clipped dashed accent rectangle.
Plain marquee replaces selection; Ctrl/Cmd marquee unions with the selection
captured at pointer-down.

## Claims and evidence

Claim: Existing row/tile Click, Ctrl/Cmd+Click, and Shift+Click semantics remain
the authoritative item interaction, and a plain marquee establishes stable
min/max indices for a later Shift+Click range.

Evidence: list row dispatch keeps Shift > platform/Ctrl > plain precedence at
`crates/chronos-fm-pages/src/explorer/view/listing/row.rs:186`; Grid keeps the
same order at `crates/chronos-fm-pages/src/explorer/view/listing/grid.rs:131`.
`ExplorerPane::finish_marquee` applies the plain min/max anchor/active result
and preserves the prior additive anchor at `state.rs:581`. Focused lifecycle
tests are in `explorer/marquee.rs`, including plain completion and additive
anchor preservation; `cargo test -p chronos-fm-pages --lib marquee` exited 0
with 22 passed, 0 failed.

Truth base: Chronos-FM.

Claim: Plain marquee replaces selection, while Linux Ctrl marquee is additive
and shares the existing platform/Cmd mirror.

Evidence: `begin_marquee` derives additive mode from
`modifiers.control || modifiers.platform` and snapshots the prior selection at
`state.rs:479`; `update_marquee` writes live selection through
`selection_for_hits` at `state.rs:527`. Pure replace/union, threshold, closed
edge, and reverse-direction cases start at `marquee.rs:132`. Release frames
`docs/orchestration/tasks/report-log/T050-marquee-list.png` and
`docs/orchestration/tasks/report-log/T050-marquee-grid.png` visibly show the
drag rectangle with 6 List rows and 5 Grid tiles highlighted respectively.

Truth base: Chronos-FM | runtime.

Claim: Hit testing uses measured GPUI window coordinates, only current visible
measurements participate, and a press-epoch geometry change cancels without
reusing stale bounds or rolling back the last live selection.

Evidence: the token stores view mode, entry revision, viewport, scroll offset,
and item-size revision at `marquee.rs:10`; viewport/item records are accepted
through `state.rs:431` and `state.rs:455`. The shared surface records its
window-coordinate viewport and routes pointer lifecycle at
`view/listing.rs:18`; list rows record only virtual-list emitted rows at
`view/listing/list.rs:42`, while Grid prunes non-intersecting tile bounds at
`view/listing/grid.rs:103`. Token mismatch cancellation is enforced at
`state.rs:527`. The two `marquee_routing_*_retains_only_*` production-tree tests
and stale-token lifecycle tests are included in the 22/22 marquee result;
`cargo test -p chronos-fm-pages --lib marquee_routing` separately exited 0
with 4 passed, 0 failed.

Truth base: Chronos-FM.

Claim: Marquee starts only on the empty listing surface, does not steal
row/tile/header/control input, and the visual overlay is clipped and
pointer-transparent.

Evidence: `begin_marquee` rejects measured items and registered exclusions at
`state.rs:479`; list header and resize controls register exclusions in
`view/listing/list.rs:107`; routing and overlay construction are in
`view/listing.rs:65`. The Root-wrapped routing tests reject a row, a tile, blank
header space, and a real resize handle before exercising empty-space drags.
The two release grims show the overlay confined to the listing viewport.

Truth base: Chronos-FM | runtime.

Claim: T051 receives no second selection model: marquee continues to feed
`ExplorerPane::selection`, and file operations continue to consume
`selected_paths()`.

Evidence: the only marquee selection write is the existing set at
`state.rs:567`; `selected_paths()` maps that set through the current filtered
entries at `state.rs:630`. Existing selection/path coverage remained green in
the complete 146-test pages library run.

Truth base: Chronos-FM.

## Verification

- `cargo fmt -p chronos-fm-pages` - exit 0. Formatter-only changes outside the
  exact T050 file list were restored before verification.
- Scoped `git diff --check` over the eight T050 Rust files - exit 0.
- `cargo test -p chronos-fm-pages --lib marquee` - exit 0; 22 passed, 0 failed.
- `cargo test -p chronos-fm-pages --lib marquee_routing` - exit 0; 4 passed,
  0 failed.
- `cargo test -p chronos-fm-pages --lib` - exit 0; 146 passed, 0 failed.
- `cargo test --workspace` - exit 0; every workspace unit and doc-test binary
  completed without failure.
- `cargo build --release -p chronos-fm` - exit 0; release profile finished in
  5m 44s.
- Runtime log `/tmp/chronos-t050-live.log` records selection of the NVIDIA
  GeForce RTX 3070 Vulkan adapter. Hyprland reported PID 881358 as
  `class=chronos-fm` before capture.

Existing warnings remain in the shared Source fork and workspace crates
(`unexpected_cfgs`, missing documentation/dead code, and the
`proc-macro-error2` future-incompatibility notice); no verification command
failed.

## Runtime procedure

Eight real files were created in the isolated home/cwd:

```bash
mkdir -p /tmp/chronos-t050-live
touch /tmp/chronos-t050-live/alpha.txt \
  /tmp/chronos-t050-live/bravo.md \
  /tmp/chronos-t050-live/charlie.rs \
  /tmp/chronos-t050-live/delta.json \
  /tmp/chronos-t050-live/echo.log \
  /tmp/chronos-t050-live/foxtrot.toml \
  /tmp/chronos-t050-live/golf.csv \
  /tmp/chronos-t050-live/hotel.png
```

The verified release binary was launched with the fixture as both cwd and
isolated home:

```bash
cd /tmp/chronos-t050-live
nohup env HOME=/tmp/chronos-t050-live \
  XDG_CONFIG_HOME=/tmp/chronos-t050-live/.config \
  XDG_CACHE_HOME=/tmp/chronos-t050-live/.cache \
  XDG_DATA_HOME=/tmp/chronos-t050-live/.local/share \
  HYPRLAND_INSTANCE_SIGNATURE=efb50993780079460b0cbed1363e2166a2de1d9f_1786369429_887750879 \
  WAYLAND_DISPLAY=wayland-1 XDG_RUNTIME_DIR=/run/user/1000 RUST_LOG=info \
  /home/neo/projects/chronos-ecosystem/Chronos-FM/.worktrees/t050-marquee/target/release/chronos-fm \
  --page explorer >/tmp/chronos-t050-live.log 2>&1 &
```

The release window was isolated full-screen on HDMI workspace 20 with the
compositor's window object, avoiding pointer input in the active desktop:

```bash
hyprctl repl 'local w=hl.get_window("class:chronos-fm"); local mon=hl.get_monitor("HDMI-A-1"); hl.dispatch(hl.dsp.workspace.move({workspace=20, monitor=mon})); hl.dispatch(hl.dsp.window.move({workspace=20, window=w})); local ws=hl.get_workspace(20); hl.dispatch(hl.dsp.focus({workspace=ws})); hl.dispatch(hl.dsp.focus({window=w})); hl.dispatch(hl.dsp.window.fullscreen({window=w})); return mon.name'
```

Pointer-down and movement came from the existing `ydotoold` virtual device.
The following concrete coordinates were used; the screenshot ran before the
matching `0x80` pointer-up, so each frame captures live state rather than the
completed selection:

```bash
# List: blank (3960,500) to (2860,220), selecting 6 of 8.
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1980 -y 250
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool click 0x40
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1900 -y 220
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1800 -y 180
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1600 -y 130
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1430 -y 110
env XDG_RUNTIME_DIR=/run/user/1000 WAYLAND_DISPLAY=wayland-1 \
  grim -o HDMI-A-1 docs/orchestration/tasks/report-log/T050-marquee-list.png
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool click 0x80

# Grid toggle, then blank (3960,500) to (3160,150), selecting 5 of 8.
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 2198 -y 44
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool click 0xC0
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1980 -y 250
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool click 0x40
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1900 -y 220
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1800 -y 180
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1700 -y 130
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool mousemove --absolute -x 1580 -y 75
env XDG_RUNTIME_DIR=/run/user/1000 WAYLAND_DISPLAY=wayland-1 \
  grim -o HDMI-A-1 docs/orchestration/tasks/report-log/T050-marquee-grid.png
YDOTOOL_SOCKET=/run/user/1000/.ydotool_socket ydotool click 0x80
```

`hyprctl cursorpos -j` confirmed the virtual-device Grid-toggle coordinates as
global `(4396, 88)`. Both 1920x1200 grims were visually inspected after capture.

## Residual and non-goals

Only currently laid-out entries participate; there is no off-screen geometry
approximation or marquee auto-scroll. DnD (T051), Shift+Arrow residual work,
spring-loaded folders, and user keymap settings remain out of scope.
