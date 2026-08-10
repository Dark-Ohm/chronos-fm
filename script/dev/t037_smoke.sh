#!/usr/bin/env bash
# T037 live smoke: launch release chronos-fm (dark/blue), wait for the window,
# let it settle, capture grim, report log errors, kill only our PID.
# Usage: t037_smoke.sh <out.png> [settle_seconds]
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${1:?usage: t037_smoke.sh <out.png> [settle_seconds]}"
SETTLE="${2:-20}"
LOG="/tmp/t037_smoke.log"
rm -f "$OUT" "$LOG"
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
      sleep "$SETTLE"
      grim -g "$G" "$OUT" && echo "GRIM_OK"
      break
    fi
  fi
  sleep 3
done
echo "--- log errors ---"
rg -n -i 'error|panic|failed' "$LOG" | tail -8 || echo none
if [ -n "$PID" ]; then kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null; echo KILLED; fi
test -s "$OUT"
