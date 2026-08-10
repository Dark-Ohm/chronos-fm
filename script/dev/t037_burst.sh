#!/usr/bin/env bash
# T037 burst capture: launch release chronos-fm (dark/blue), wait for window,
# capture N frames every INTERVAL seconds, then print per-frame "completeness"
# (non-background pixel share) so the most complete frame can be chosen.
# Usage: t037_burst.sh <out_dir> [n_frames] [interval]
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${1:?usage: t037_burst.sh <out_dir> [n_frames] [interval]}"
N="${2:-10}"
INTERVAL="${3:-3}"
LOG="/tmp/t037_burst.log"
mkdir -p "$OUT_DIR"
rm -f "$LOG" "$OUT_DIR"/frame-*.png
cd "$ROOT" || exit 1
setsid env RUST_LOG=info ./target/release/chronos-fm --theme dark --accent blue \
  >"$LOG" 2>&1 </dev/null &
sleep 3
PID="$(pgrep -n -x chronos-fm || true)"
echo "PID=$PID"
FOUND=""
for _ in $(seq 1 20); do
  if [ -z "$FOUND" ]; then
    G="$(hyprctl clients -j 2>/dev/null | jq -r 'map(select(type=="object")) | .[] | select((.class // "")=="chronos-fm") | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"' | head -1)"
    if [ -n "$G" ]; then
      FOUND=1
      echo "GEOM=$G"
      sleep 8
      for i in $(seq -w 1 "$N"); do
        grim -g "$G" "$OUT_DIR/frame-$i.png"
        sleep "$INTERVAL"
      done
      break
    fi
  fi
  sleep 3
done
echo "--- error-line rate (zero-size) ---"
ERR_LINES="$(rg -c "can't render at a zero size" "$LOG" || echo 0)"
echo "zero-size errors: $ERR_LINES"
if [ -n "$PID" ]; then kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null; echo KILLED; fi
ls -la "$OUT_DIR"/*.png 2>/dev/null | head -20
