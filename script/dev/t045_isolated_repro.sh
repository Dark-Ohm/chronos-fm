#!/usr/bin/env bash
set -uo pipefail
SRC="/home/neo/projects/chronos-ecosystem/Source/gpui-component"
LOG="/tmp/t045_isolated_repro.log"
rm -f "$LOG"
cd "$SRC" || exit 1
setsid env CHRONOS_SVG_DEBUG=1 RUST_LOG=info ./target/debug/t045_repro \
  >"$LOG" 2>&1 </dev/null &
sleep 12
PID="$(pgrep -n -x t045_repro || true)"
echo "PID=$PID"
if [ -n "$PID" ]; then kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null; echo KILLED; fi
echo "--- total repro debug lines ---"
grep -c "\[repro T045\]" "$LOG" || echo 0
echo "--- zero=true count ---"
grep -F "[repro T045]" "$LOG" | grep -c "zero=true" || echo 0
echo "--- first 10 ---"
grep -F "[repro T045]" "$LOG" | head -10
echo "--- last 10 ---"
grep -F "[repro T045]" "$LOG" | tail -10
echo "--- other errors ---"
grep -inE 'error|panic' "$LOG" | grep -v "repro T045" | tail -10
