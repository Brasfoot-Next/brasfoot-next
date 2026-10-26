#!/bin/bash
# Shows how the mod (readable code) and patched classes call the given methods.
# Usage: mod_usage.sh <method> [<method>...]   e.g. mod_usage.sh fk ad aq
M=~/brasfoot-mods/mod
for m in "$@"; do
  echo "== $m()"
  grep -rhoE "[A-Za-z_][A-Za-z0-9_]* *=? *[A-Za-z_.()]*\.$m\([^;]{0,60}" $M/src $M/patch 2>/dev/null | sort | uniq -c | sort -rn | head -4
done
