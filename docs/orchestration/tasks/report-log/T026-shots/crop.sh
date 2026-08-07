#!/usr/bin/env bash
# Стена 1 evidence stays the FULL-output frame; this only makes a readable crop
# of the window neighbourhood (window + 300px margin) so the menu can be examined
# by eye without downscaling the whole 4480x1440 composite.
set -u
S=/tmp/claude-1000/-home-neo-projects-chronos-ecosystem-Source/68c3db57-8a34-40c2-bf0a-4a6b84ae2efc/scratchpad/t026
src=$1; out=$2; x=$3; y=$4; w=$5; h=$6
magick "$S/shots/$src" -crop "${w}x${h}+${x}+${y}" +repage "$S/shots/$out"
identify "$S/shots/$out"
