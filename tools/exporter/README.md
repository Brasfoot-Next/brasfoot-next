# Exporter

Generic, reflection-based dump of the game's original data (Kryo saves and Java-serialized files) to JSON.
It knows nothing about the meaning of the obfuscated classes; every field of every reachable object is written.

## Run

    tools/exporter/export.sh save <name> [--gzip]    # dados/jogo/sav/<name>.s22 -> dados/export/<name>.json
    tools/exporter/export.sh teams [--gzip]          # all teams/*.ban in ONE file -> dados/export/teams.json
    tools/exporter/export.sh misc [--gzip]           # conf_ligas_nacionais/*, conf_estadual/*, options.bcf -> misc.json
    tools/exporter/export.sh schema                  # Kryo wire schema of every game class -> crates/persistence/data/kryo-schema.json
    tools/exporter/export.sh canon <name>            # canonical dump of a save, the reference for the Rust .s22 reader
    tools/exporter/export.sh awards <days>           # the sim career with the season-end awards -> crates/systems/tests/fixtures/awards-after-<days>.*
    tools/exporter/export.sh human-days <days>       # fictional career with a human club played with the game's screens
    tools/exporter/export.sh prepare                 # re-copy the game folder (done automatically when missing)
    BF_WORLD=1 BF_FIXTURE_DIR=dados/world tools/exporter/export.sh sim <days>   # the world career (international cups, every national team)

Debugging a differential test: `BF_TRACE_RNG=1` prints the `java.util.Random` counters (next `new Random()` seed and the number
of draws so far) at the harness's checkpoints (`start`, `day N`, `end`); `BF_TRACE_METHODS="class.method[(descriptor)][*],..."`
adds the entry of game methods, by bytecode instrumentation (`trace-agent/`, nothing recompiled; `*` prints every call, else the
first one). The Rust side prints the same labels with `Runtime::checkpoint`.

Each run also writes `dados/export/<name>.summary.txt`: per class, instance count and every field with its declared Java type and the observed runtime types.
The tool works on a copy of the game folder in `dados/jogo/` (user.dir); nothing is ever written to `~/.local/share/brasfoot`.
Build output goes to `tools/exporter/build/`. The save is read with a plain `Kryo` (registration not required, references on), twice, like the game: `c.a.SR` (`best.f`) then `c.a.afQ`.
`best.f.bj` is set to `zz_teste_mod` by reflection after the dump.

## Format

Save file: `{"input", "format", "roots": {"SR": <value>, "afQ": <value>}}`. Teams/misc: `{"format", "files": [{"file", "root": <value>}, ...]}` (ids are global across files).

Values:
- `null`, booleans, Strings, ints and doubles: plain JSON (a double always has a `.` or exponent). longs: numbers, strings if above 2^53. NaN/Infinity: strings.
- Boxed Long/Short/Byte/Float/Character in an untyped slot (Object field, collection element): `{"$box": "java.lang.Long", "value": ...}`. chars: one-character strings.
- Enum: `{"$enum": class, "name", "ordinal"}`. `Class` objects: `{"$classRef": name}`.
- Object: `{"$id": n, "$class": name, "fields": {field: value}}`. Later occurrences: `{"$ref": n}` (shared references and cycles).
  Inherited fields are included; a name repeated in the hierarchy is keyed `declaring.Class#name`.
- Arrays: `{"$id", "$class": "[I", "items": [...]}`.
- Collections: `{"$id", "$class", "items": [...]}`; maps: `{"$id", "$class", "entries": [[k, v], ...]}`, both in iteration order. Game subclasses of collections also get a `fields` object.
- Calendar: `millis`, `timeZone`, `lenient`, `firstDayOfWeek`, `minimalDaysInFirstWeek`. TimeZone: `id`. Color: `argb`.
- AWT images, icons, components, fonts, cursors, borders, threads, loaders: `{"$class", "$skipped": true}` (counted at the end of the summary). Unreadable fields: `{"$unreadable": true}`.
