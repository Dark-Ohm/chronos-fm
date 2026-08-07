#!/usr/bin/env bash
# T026 — trigger-overlay pass. One launch per row.
#
# Captures BEFORE (freshly opened story) and AFTER (post-interaction) in the SAME
# launch, so the coordinates used are validated against the geometry that was live
# at click time -- Стена 3 (window migrates between monitors between launches).
#
# Usage: overlay.sh <story> <out> <window|full> <settle_secs> <action>...
#   actions: move:X,Y  click:X,Y  rclick:X,Y  sleep:N
#   X,Y are window-relative; ydotool gets them halved (Стена 2).
#
# Стена 4: exit 0 proves nothing. The frames must be looked at.
set -u
S=/tmp/claude-1000/-home-neo-projects-chronos-ecosystem-Source/68c3db57-8a34-40c2-bf0a-4a6b84ae2efc/scratchpad/t026
BIN=/home/neo/projects/chronos-ecosystem/Source/gpui-component/target/debug/gpui-component-story
mkdir -p "$S/shots" "$S/logs"

story=$1; out=$2; capmode=$3; settle=$4; shift 4

"$BIN" "$story" > "$S/logs/$out.log" 2>&1 < /dev/null &
pid=$!
ax=""; ay=""; w=""; h=""
for _ in $(seq 1 60); do
  sleep 0.25
  kill -0 $pid 2>/dev/null || break
  read -r ax ay w h < <(hyprctl clients -j | jq -r --argjson p "$pid" \
    '.[] | select(.pid==$p) | "\(.at[0]) \(.at[1]) \(.size[0]) \(.size[1])"' | head -1)
  [ -n "${w:-}" ] && break
done
if ! kill -0 $pid 2>/dev/null; then echo "$out CRASH rc=$?"; exit 1; fi
[ -z "${w:-}" ] && { echo "$out NO_WINDOW"; kill $pid 2>/dev/null; exit 1; }
sleep 1.5

grim -g "$ax,$ay ${w}x${h}" "$S/shots/$out--before.png"

for a in "$@"; do
  verb=${a%%:*}; arg=${a#*:}
  case "$verb" in
    move|click|rclick)
      rx=${arg%%,*}; ry=${arg##*,}
      # "c<offset>" = offset from the centre of the content column (sidebar is
      # 200px), so targets survive the window changing width between launches.
      if [ "${rx#c}" != "$rx" ]; then
        centre=$(( 200 + (w - 200) / 2 ))
        rx=$(( centre + ${rx#c} ))
      fi
      cx=$(( ax + rx )); cy=$(( ay + ry ))
      ydotool mousemove --absolute -x $(( cx / 2 )) -y $(( cy / 2 ))
      sleep 0.35
      [ "$verb" = click ]  && ydotool click 0xC0
      [ "$verb" = rclick ] && ydotool click 0xC1
      sleep 0.4
      ;;
    sleep) sleep "$arg" ;;
  esac
done

sleep "$settle"
if [ "$capmode" = full ]; then
  # Стена 1: NativeMenu is drawn by the OS and can extend past the window,
  # so grim -g by window geometry would clip it. Capture the whole output.
  grim "$S/shots/$out--after.png"
else
  grim -g "$ax,$ay ${w}x${h}" "$S/shots/$out--after.png"
fi

echo "$out geo=$ax,$ay ${w}x${h} cap=$capmode settle=${settle}s actions=$*"
kill $pid 2>/dev/null; wait $pid 2>/dev/null
