#!/bin/bash
# Most common call sites of the given methods in the decompiled game (and mod patches).
# Usage: usage.sh <method>...
cd ~/brasfoot-mods
for m in "$@"; do
  echo "== $m"
  grep -rhoE "[^ ]{0,50}\.$m\([^;]{0,60}" --include='*.java' src mod/patch mod/src | sort | uniq -c | sort -rn | head -${N:-4} | cut -c1-170
done
