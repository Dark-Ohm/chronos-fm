#!/usr/bin/env bash
# T046: launch release chronos-fm on a given --page (no interactive click
# needed — see T046/chronos_fm_pages::PageKind::from_cli_name), wait for the
# window, let it settle, capture grim, report log errors, kill only our PID.
# Usage: t046_page_smoke.sh <page> <out.png> [settle_seconds]
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PAGE="${1:?usage: t046_page_smoke.sh <page> <out.png> [settle_seconds]}"
OUT="${2:?usage: t046_page_smoke.sh <page> <out.png> [settle_seconds]}"
SETTLE="${3:-8}"
LOG="/tmp/t046_page_smoke_${PAGE}.log"
rm -f "$OUT" "$LOG"
cd "$ROOT" || exit 1
setsid env RUST_LOG=info ./target/release/chronos-fm --theme dark --accent blue --page "$PAGE" \
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
      # Same re-check discipline as t037_smoke.sh: re-verify class=chronos-fm
      # right before grim, don't trust stale geometry.
      G2="$(hyprctl clients -j 2>/dev/null | jq -r 'map(select(type=="object")) | .[] | select((.class // "")=="chronos-fm") | "\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"' | head -1)"
      if [ -z "$G2" ]; then
        echo "CLASS_GONE_BEFORE_GRIM — refusing to grim stale geometry"
        break
      fi
      grim -g "$G2" "$OUT" && echo "GRIM_OK"
      break
    fi
  fi
  sleep 3
done
echo "--- log errors ---"
rg -n -i 'error|panic|failed' "$LOG" | tail -8 || echo none
if [ -n "$PID" ]; then kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null; echo KILLED; fi
test -s "$OUT"
