#!/bin/bash
# Builds and runs the exporter. Usage:
#   export.sh save <name> [--gzip]   dump dados/jogo/sav/<name>.s22  -> dados/export/<name>.json
#   export.sh teams [--gzip]         dump every teams/*.ban         -> dados/export/teams.json
#   export.sh misc [--gzip]          dump conf_ligas_nacionais, conf_estadual, options.bcf
#   export.sh schema                 Kryo wire schema of every game class -> crates/persistence/data/kryo-schema.json
#   export.sh canon <name>           canonical dump of dados/jogo/sav/<name>.s22 -> dados/export/<name>.canon.txt
#                                    (the Rust reader must produce the same bytes: see crates/persistence)
#   export.sh fixture                fictional .s22 + its canonical dump -> crates/persistence/tests/fixtures/
#   export.sh engine                 fictional world + AI matches with deterministic Random -> crates/engine/tests/fixtures/
#   export.sh sim <days>             fictional career + <days> headless days -> crates/simulation/tests/fixtures/
#   export.sh awards <days>          the sim career with the mod's season-end awards -> crates/systems/tests/fixtures/
#   export.sh clock <days>           the awards plus the mod's clock (one tick a day) -> crates/systems/tests/fixtures/
#   export.sh purchase               a human club's offers and purchases (best.l) -> crates/simulation/tests/fixtures/purchase-*
#   export.sh human-days <days>      the same career with a human club, <days> days -> crates/simulation/tests/fixtures/
#   export.sh screens new|career <save>  reference pictures of the game's screens -> dados/telas-ref/
#   export.sh prepare                (re)copy the game folder into dados/jogo (done automatically if missing)
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TOOL="$ROOT/tools/exporter"
ORIG="$HOME/.local/share/brasfoot"
GAME="$ROOT/dados/jogo"
OUT="$ROOT/dados/export"
J="$HOME/.jdks/corretto-21.0.11/bin"
O="$HOME/brasfoot-mods/orig/jar"
# the modded game's classes come first, like in bf22-23-mod.jar
MOD="$HOME/brasfoot-mods/build/classes"
CP="$O:$O/kryo-4.0.2.jar:$O/minlog-1.3.0.jar:$O/objenesis-2.1.jar:$O/reflectasm-1.10.1-shaded.jar"

prepare_game() {
  # Working copy of the game folder (read only on the original): everything but other saves.
  mkdir -p "$GAME/sav" "$OUT"
  for d in teams selecoes conf_ligas_nacionais conf_estadual teams_removidos img imgbf escudos escudosMini trofeus sons; do
    [ -e "$ORIG/$d" ] && [ ! -e "$GAME/$d" ] && cp -r "$ORIG/$d" "$GAME/$d"
  done
  for f in options.bcf colors.csv; do [ -e "$ORIG/$f" ] && cp -n "$ORIG/$f" "$GAME/$f"; done
  # saves are never copied here: tests use the fictional fixtures (export.sh fixture)
  return 0
}

# java.util.Random with deterministic no-arg seeds (tools/exporter/random-patch), for the harnesses
build_random() {
  local out="$TOOL/build-random"
  [ -f "$out/java/util/Random.class" ] && [ -f "$out/sun/util/locale/provider/CalendarProviderImpl.class" ] && return 0
  mkdir -p "$TOOL/build-random-src/java/util" "$TOOL/build-random-src/sun/util/locale/provider" "$out"
  unzip -p "$J/../lib/src.zip" java.base/sun/util/locale/provider/CalendarProviderImpl.java > "$TOOL/build-random-src/CalendarProviderImpl.orig.java"
  unzip -p "$J/../lib/src.zip" java.base/java/util/Random.java > "$TOOL/build-random-src/Random.orig.java"
  unzip -p "$J/../lib/src.zip" java.base/java/util/GregorianCalendar.java > "$TOOL/build-random-src/GregorianCalendar.orig.java"
  python3 "$TOOL/random-patch/patch_random.py" "$TOOL/build-random-src/Random.orig.java" "$TOOL/build-random-src/java/util/Random.java"
  python3 "$TOOL/random-patch/patch_calendar.py" "$TOOL/build-random-src/GregorianCalendar.orig.java" "$TOOL/build-random-src/java/util/GregorianCalendar.java" \
    "$TOOL/build-random-src/CalendarProviderImpl.orig.java" "$TOOL/build-random-src/sun/util/locale/provider/CalendarProviderImpl.java"
  "$J/javac" -nowarn --patch-module java.base="$TOOL/build-random-src" -d "$out" "$TOOL/build-random-src/java/util/Random.java" \
    "$TOOL/build-random-src/java/util/GregorianCalendar.java" "$TOOL/build-random-src/sun/util/locale/provider/CalendarProviderImpl.java"
}

# the debug agent of BF_TRACE_METHODS
build_agent() {
  local out="$TOOL/build-agent"
  [ -f "$out/trace-agent.jar" ] && return 0
  mkdir -p "$out/classes"
  "$J/javac" -nowarn --add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED -d "$out/classes" "$TOOL/trace-agent/exporter/TraceAgent.java"
  printf "Premain-Class: exporter.TraceAgent\n" > "$out/manifest.txt"
  "$J/jar" cfm "$out/trace-agent.jar" "$out/manifest.txt" -C "$out/classes" .
}

compile() {
  mkdir -p "$TOOL/build"
  # the mod classes too: the harnesses call into bfmod
  "$J/javac" -nowarn -cp "$MOD:$CP" -d "$TOOL/build" "$TOOL"/src/exporter/*.java
}

cmd="$1"; shift || true
[ "$cmd" = prepare ] && { prepare_game; echo "game copy at $GAME"; exit 0; }
[ -d "$GAME/teams" ] || prepare_game
compile
OPENS=""
for p in java.lang java.util java.util.concurrent java.util.concurrent.atomic java.util.regex java.text java.io java.net java.math java.time; do
  OPENS="$OPENS --add-opens java.base/$p=ALL-UNNAMED"
done
OPENS="$OPENS --add-opens java.desktop/java.awt=ALL-UNNAMED --add-opens java.desktop/javax.swing=ALL-UNNAMED"
run() { (cd "$GAME" && "$J/java" -Xmx9g -Xss512m -Djava.awt.headless=true $OPENS -cp "$TOOL/build:$CP" exporter.Exporter "$@"); }
GZ=""
args=()
for a in "$@"; do [ "$a" = "--gzip" ] && GZ="--gzip" || args+=("$a"); done

case "$cmd" in
  save)  run save "$OUT" "$GAME/sav/${args[0]:?save name}.s22" $GZ ;;
  teams) run ser "$OUT" teams "$GAME/teams" $GZ ;;
  schema) (cd "$GAME" && "$J/java" -Djava.awt.headless=true $OPENS -cp "$TOOL/build:$MOD:$CP" exporter.KryoSchema "$ROOT/crates/persistence/data/kryo-schema.json" "$O") ;;
  canon) (cd "$GAME" && "$J/java" -Xmx9g -Djava.awt.headless=true $OPENS -cp "$TOOL/build:$MOD:$CP" exporter.CanonicalDump "$GAME/sav/${args[0]:?save name}.s22" "$OUT/${args[0]}.canon.txt") ;;
  fixture)
    FIX="$ROOT/crates/persistence/tests/fixtures"; mkdir -p "$FIX"
    (cd "$GAME" && "$J/java" -Djava.awt.headless=true $OPENS -cp "$TOOL/build:$MOD:$CP" exporter.FictionalFixture "$FIX/fictional.s22" "$TOOL/list-elements.tsv") &&
    (cd "$GAME" && "$J/java" -Djava.awt.headless=true $OPENS -cp "$TOOL/build:$MOD:$CP" exporter.CanonicalDump "$FIX/fictional.s22" "$FIX/fictional.canon.txt") &&
    zstd -q -f -19 "$FIX/fictional.canon.txt" -o "$FIX/fictional.canon.txt.zst" && rm "$FIX/fictional.canon.txt" ;;
  engine)
    build_random
    FIX="$ROOT/crates/engine/tests/fixtures"; mkdir -p "$FIX"
    for tactics in 1 0; do
      (cd "$GAME" && "$J/java" -Xss512m -Djava.awt.headless=true --patch-module java.base="$TOOL/build-random" $OPENS \
        -cp "$TOOL/build:$MOD:$CP" exporter.EngineHarness "$FIX" $tactics 1000) || exit 1
    done ;;
  sim)
    build_random
    # BF_FIXTURE_DIR writes elsewhere; BF_DEBUG_CLASSES puts traced copies of game classes first
    FIX="${BF_FIXTURE_DIR:-$ROOT/crates/simulation/tests/fixtures}"; mkdir -p "$FIX"
    EMPTY="$ROOT/dados/sim-game"; rm -rf "$EMPTY"; mkdir -p "$EMPTY/sav"
    # BF_LEAGUE_CONFIGS=1: the game's league configurations (playoffs, relegations) instead of
    # the defaults of an empty folder
    if [ -n "${BF_LEAGUE_CONFIGS:-}" ]; then
      [ -d "$GAME/conf_ligas_nacionais" ] || prepare
      cp -r "$GAME/conf_ligas_nacionais" "$EMPTY/"
    fi
    DEBUG="${BF_DEBUG_CLASSES:+$BF_DEBUG_CLASSES:}"
    # BF_TRACE_METHODS="best.F.gc,best.ag.jO*": the Random counters at those methods' entries
    # (tools/exporter/trace-agent), to line up with the Rust side's checkpoints
    AGENT=""
    if [ -n "${BF_TRACE_METHODS:-}" ]; then
      build_agent
      AGENT="--add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED -javaagent:$TOOL/build-agent/trace-agent.jar=$BF_TRACE_METHODS"
    fi
    # the made-up teams, as team files, for the Rust new game
    TEAMS="$FIX/${BF_WORLD:+world-}sim-teams"; rm -rf "$TEAMS"; mkdir -p "$TEAMS"
    (cd "$EMPTY" && BF_TEAMS_DIR="$TEAMS" "$J/java" $AGENT -Xss512m -Djava.awt.headless=true -Duser.timezone=America/Sao_Paulo -Duser.language=pt -Duser.country=BR --patch-module java.base="$TOOL/build-random" $OPENS \
      -cp "$DEBUG$TOOL/build:$MOD:$CP" exporter.SimHarness "$FIX" "${args[0]:-1}" 1000) ;;
  awards)
    build_random
    FIX="${BF_FIXTURE_DIR:-$ROOT/crates/systems/tests/fixtures}"; mkdir -p "$FIX"
    EMPTY="$ROOT/dados/sim-game"; rm -rf "$EMPTY"; mkdir -p "$EMPTY/sav"
    (cd "$EMPTY" && "$J/java" -Xss512m -Djava.awt.headless=true -Duser.timezone=America/Sao_Paulo -Duser.language=pt -Duser.country=BR --patch-module java.base="$TOOL/build-random" $OPENS \
      -cp "$TOOL/build:$MOD:$CP" exporter.SimHarness "$FIX" "awards:${args[0]:?days}" 1000) && rm -f "$FIX/sim-start.s22" ;;
  clock)
    build_random
    FIX="${BF_FIXTURE_DIR:-$ROOT/crates/systems/tests/fixtures}"; mkdir -p "$FIX"
    EMPTY="$ROOT/dados/sim-game"; rm -rf "$EMPTY"; mkdir -p "$EMPTY/sav"
    AGENT=""
    if [ -n "${BF_TRACE_METHODS:-}" ]; then
      build_agent
      AGENT="--add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED -javaagent:$TOOL/build-agent/trace-agent.jar=$BF_TRACE_METHODS"
    fi
    (cd "$EMPTY" && "$J/java" $AGENT -Xss512m -Djava.awt.headless=true -Duser.timezone=America/Sao_Paulo -Duser.language=pt -Duser.country=BR --patch-module java.base="$TOOL/build-random" $OPENS \
      -cp "$TOOL/build:$MOD:$CP" exporter.SimHarness "$FIX" "clock:${args[0]:?days}" 1000) && rm -f "$FIX/sim-start.s22" ;;
  purchase)
    build_random
    FIX="${BF_FIXTURE_DIR:-$ROOT/crates/simulation/tests/fixtures}"
    EMPTY="$ROOT/dados/sim-game"; rm -rf "$EMPTY"; mkdir -p "$EMPTY/sav"
    (cd "$EMPTY" && "$J/java" -Xss512m -Djava.awt.headless=true -Duser.timezone=America/Sao_Paulo -Duser.language=pt -Duser.country=BR --patch-module java.base="$TOOL/build-random" $OPENS \
      -cp "$TOOL/build:$MOD:$CP" exporter.PurchaseHarness "$FIX" 1000) ;;
  coach-screen)
    build_random
    FIX="${BF_FIXTURE_DIR:-$ROOT/crates/simulation/tests/fixtures}"; mkdir -p "$FIX"
    EMPTY="$ROOT/dados/sim-game"; rm -rf "$EMPTY"; mkdir -p "$EMPTY/sav"
    (cd "$EMPTY" && "$J/java" -Xss512m -Djava.awt.headless=true -Duser.timezone=America/Sao_Paulo -Duser.language=pt -Duser.country=BR --patch-module java.base="$TOOL/build-random" $OPENS \
      -cp "$TOOL/build:$MOD:$CP" exporter.SimHarness "$FIX" coach-screen 1000) ;;
  human)
    build_random
    FIX="${BF_FIXTURE_DIR:-$ROOT/crates/simulation/tests/fixtures}"; mkdir -p "$FIX"
    EMPTY="$ROOT/dados/sim-game"; rm -rf "$EMPTY"; mkdir -p "$EMPTY/sav"
    (cd "$EMPTY" && "$J/java" -Xss512m -Djava.awt.headless=true -Duser.timezone=America/Sao_Paulo -Duser.language=pt -Duser.country=BR --patch-module java.base="$TOOL/build-random" $OPENS \
      -cp "$TOOL/build:$MOD:$CP" exporter.SimHarness "$FIX" human 1000) ;;
  human-days)
    build_random
    # the round screen a.dV without its timer (human-patch), first on the classpath
    # inside package a the decompiled source means the packages b, c, d, e, f, g, h, which the classes
    # a.b .. a.h shadow: compile against a copy of package a without them (the bytecode names are
    # fully qualified, so the run uses the real classes)
    SHADOW="$TOOL/build-human-cp"
    if [ ! -d "$SHADOW/a" ]; then
      mkdir -p "$SHADOW/a"
      for f in "$O"/a/*.class; do
        n=$(basename "$f" .class); case "${n%%\$*}" in a|b|c|d|e|f|g|h) continue ;; esac
        ln -s "$f" "$SHADOW/a/"
      done
      for d in "$O"/*/; do d=$(basename "$d"); [ "$d" = a ] || ln -s "$O/$d" "$SHADOW/$d"; done
    fi
    "$J/javac" -nowarn -cp "$TOOL/build:$MOD:$SHADOW:$O/kryo-4.0.2.jar" -d "$TOOL/build-human" "$TOOL"/human-patch/a/*.java "$TOOL"/human-patch/best/*.java
    # the mod's dialogs answered by the harness human (mod-patch), first on the classpath too
    rm -rf "$TOOL/build-mod-patch-src" "$TOOL/build-mod-patch"; mkdir -p "$TOOL/build-mod-patch"
    python3 "$TOOL/mod-patch/patch_mod.py" "$HOME/brasfoot-mods/mod/src/bfmod" "$TOOL/build-mod-patch-src"
    "$J/javac" -nowarn -cp "$TOOL/build:$MOD:$CP" -d "$TOOL/build-mod-patch" "$TOOL"/build-mod-patch-src/bfmod/*.java
    FIX="${BF_FIXTURE_DIR:-$ROOT/crates/simulation/tests/fixtures}"; mkdir -p "$FIX"
    EMPTY="$ROOT/dados/sim-game"; rm -rf "$EMPTY"; mkdir -p "$EMPTY/sav"
    DEBUG="${BF_DEBUG_CLASSES:+$BF_DEBUG_CLASSES:}"
    # BF_TRACE_METHODS="best.F.gc,best.ag.jO*": the Random counters at those methods' entries
    # (tools/exporter/trace-agent), to line up with the Rust side's checkpoints
    AGENT=""
    if [ -n "${BF_TRACE_METHODS:-}" ]; then
      build_agent
      AGENT="--add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED -javaagent:$TOOL/build-agent/trace-agent.jar=$BF_TRACE_METHODS"
    fi
    (cd "$EMPTY" && "$J/java" -Xss512m -Djava.awt.headless=true -Duser.timezone=America/Sao_Paulo -Duser.language=pt -Duser.country=BR --patch-module java.base="$TOOL/build-random" $OPENS \
      -cp "$DEBUG$TOOL/build-mod-patch:$TOOL/build-human:$TOOL/build:$MOD:$CP" exporter.HumanHarness "$FIX" "${args[0]:-1}" 1000) ;;
  screens)
    # reference pictures of the game's screens (headless, mod look), in a throwaway copy of the game folder
    SHOTS="$ROOT/dados/telas-ref"; WORK="$ROOT/dados/screens-game"; rm -rf "$WORK"; mkdir -p "$SHOTS"; cp -r "$GAME" "$WORK"; rm -rf "$WORK/sav"/*
    (cd "$WORK" && "$J/java" -Xss512m -Djava.awt.headless=true -Duser.language=pt -Duser.country=BR $OPENS \
      -cp "$TOOL/build:$MOD:$CP:$HOME/brasfoot-mods/mod/lib/flatlaf-3.5.4.jar:$HOME/brasfoot-mods/mod/res" exporter.Screens "$SHOTS" "${args[@]}") ;;
  misc)  run ser "$OUT" misc "$GAME/conf_ligas_nacionais" "$GAME/conf_estadual" "$GAME/options.bcf" $GZ ;;
  *) echo "usage: export.sh save <name>|teams|misc|schema|canon <name>|fixture|engine|sim <days>|prepare [--gzip]"; exit 2 ;;
esac
