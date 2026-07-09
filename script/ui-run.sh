#!/usr/bin/env bash
# Agent UI-verification helper for the Chronos-FM GUI on Linux (software Vulkan).
# See docs/agent-ui-verification.md for the full workflow and rationale.
#
# Subcommands:
#   setup            Create unversioned dev symlinks so the GUI links without root.
#   display          Print the X display to use (existing $DISPLAY, or a running Xvnc/Xvfb).
#   launch           (Re)launch the already-built binary, wait until it renders. Prints WINDOW id.
#   shot <out.png>   Capture the whole screen to a PNG (launch first if not running).
#   win              Print the chronos-fm window id (located by PID; WM_NAME is unset).
#   stop             Kill the running chronos-fm instance.
#
# Notes:
#   - Build separately first: RUSTFLAGS="-L $HOME/.local/devlibs" cargo build -p chronos-fm
#   - Drive input yourself with xdotool against the window id from `launch`/`win`.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/debug/chronos-fm"
LOG="${CHRONOS_FM_LOG:-/tmp/chronos-fm.log}"
DEVLIBS="$HOME/.local/devlibs"
LIBDIR="${LIBDIR:-/usr/lib/x86_64-linux-gnu}"

setup() {
  mkdir -p "$DEVLIBS"
  # gpui links these; create unversioned symlinks to the runtime .so (no dev packages / root needed).
  local pairs=(
    "libxcb.so.1:libxcb.so" "libxkbcommon.so.0:libxkbcommon.so"
    "libxkbcommon-x11.so.0:libxkbcommon-x11.so" "libwayland-client.so.0:libwayland-client.so"
    "libwayland-cursor.so.0:libwayland-cursor.so" "libwayland-egl.so.1:libwayland-egl.so"
    "libvulkan.so.1:libvulkan.so"
  )
  for p in "${pairs[@]}"; do
    ln -sf "$LIBDIR/${p%%:*}" "$DEVLIBS/${p##*:}"
  done
  echo "devlibs ready at $DEVLIBS (build with: RUSTFLAGS=\"-L $DEVLIBS\" cargo build -p chronos-fm)"
}

resolve_display() {
  if [ -n "${DISPLAY:-}" ]; then echo "$DISPLAY"; return 0; fi
  local d
  d=$(ps -eo args 2>/dev/null | grep -oE '(Xvnc|Xvfb) :[0-9]+' | grep -oE ':[0-9]+' | head -1)
  if [ -n "$d" ]; then echo "$d"; return 0; fi
  echo "no display: set \$DISPLAY, or start one e.g. 'Xvfb :99 -screen 0 1280x800x24 &'" >&2
  return 1
}

win_id() {
  local pid; pid=$(pgrep -x chronos-fm | head -1) || return 1
  [ -n "$pid" ] || return 1
  DISPLAY="$(resolve_display)" xdotool search --pid "$pid" 2>/dev/null | head -1
}

launch() {
  local disp; disp="$(resolve_display)" || return 1
  [ -x "$BIN" ] || { echo "binary not built: $BIN" >&2; return 1; }
  pkill -x chronos-fm 2>/dev/null   # never use 'pkill -f' here: it matches this script's own path.
  : > "$LOG"
  DISPLAY="$disp" setsid "$BIN" > "$LOG" 2>&1 < /dev/null &
  # Software (llvmpipe) rendering: the window is black until the first real frame.
  # Wait for the steady-state render marker (poll, don't busy-spin), then let UI/fonts settle.
  timeout 30 bash -c "until grep -q 'Refreshing every' '$LOG' 2>/dev/null; do sleep 0.2; done" \
    || { echo "render marker not seen; tail of $LOG:" >&2; tail -5 "$LOG" >&2; return 1; }
  perl -e 'select(undef,undef,undef,4)'   # 'sleep' may be blocked in some agent shells; this is not.
  local w; w="$(win_id)"
  [ -n "$w" ] || { echo "window id not found; tail of $LOG:" >&2; tail -5 "$LOG" >&2; return 1; }
  echo "WINDOW=$w DISPLAY=$disp PID=$(pgrep -x chronos-fm | head -1)"
}

shot() {
  local out="${1:?usage: ui-run.sh shot <out.png>}"
  local disp; disp="$(resolve_display)" || return 1
  pgrep -x chronos-fm >/dev/null || launch >/dev/null || return 1
  local xwd="/tmp/ui-shot.$$.xwd"
  DISPLAY="$disp" xwd -root -silent -out "$xwd" || { echo "xwd failed" >&2; return 1; }
  python3 "$ROOT/script/xwd2png.py" "$xwd" "$out" || { rm -f "$xwd"; echo "xwd2png failed" >&2; return 1; }
  rm -f "$xwd"
}

case "${1:-}" in
  setup)   setup ;;
  display) resolve_display ;;
  launch)  launch ;;
  shot)    shift; shot "$@" ;;
  win)     win_id ;;
  stop)    pkill -x chronos-fm && echo "stopped" || echo "not running" ;;
  *) echo "usage: $0 {setup|display|launch|shot <out.png>|win|stop}" >&2; exit 2 ;;
esac
