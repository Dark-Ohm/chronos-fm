#!/usr/bin/env bash
set -uo pipefail
SRC="/home/neo/projects/chronos-ecosystem/Source/gpui-component"
LOG="/tmp/t045_taffy_debug.log"
rm -f "$LOG"
cd "$SRC" || exit 1
setsid env CHRONOS_TAFFY_DEBUG=1 RUST_LOG=info ./target/debug/t045_repro \
  >"$LOG" 2>&1 </dev/null &
sleep 3
PID="$(pgrep -n -x t045_repro || true)"
echo "PID=$PID"
if [ -n "$PID" ]; then kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null; echo KILLED; fi
echo "--- total premature lines ---"
grep -c "\[taffy T045\]" "$LOG" || echo 0
echo "--- first 10 ---"
grep -F "[taffy T045]" "$LOG" | head -10
