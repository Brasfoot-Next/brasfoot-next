#!/usr/bin/env python3
"""Generate the class inventory of the Brasfoot game, the mod and the patched classes.

Outputs (under docs/inventario/):
  classes.tsv    one row per class (top-level and named nested types)
  campos/*.txt   javap -p field list for every non-Swing game class
  strings.tsv    user-facing string literals per class (rough index)
  README.md      counts, 30 largest classes, failures, how to regenerate

Structure comes from javap (class files); line counts, Swing detection and
strings come from the decompiled sources. Run from anywhere:
    python3 tools/inventory/generate.py
"""
import bisect
import datetime
import os
import re
import shutil
import subprocess
import sys
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HOME = Path.home()
MODS = HOME / "brasfoot-mods"
NEXT = Path(__file__).resolve().parents[2]
OUTPUT_DIR = NEXT / "docs" / "inventario"
JAVAP = HOME / ".jdks" / "corretto-21.0.11" / "bin" / "javap"
ORIG = MODS / "orig" / "jar"
BUILD = MODS / "build" / "classes"

ORIGIN_ORDER = {"game": 0, "patch": 1, "mod": 2}
SWING_PKGS = ("javax.swing", "java.awt")
# Simple names that usually come from javax.swing / java.awt when imported by wildcard.
SWING_NAMES = re.compile(
    r"\b(J[A-Z]\w*|Color|Font|Graphics2?|Dimension|Point|Rectangle|Image|Insets|Cursor|"
    r"BorderLayout|GridBagLayout|GridBagConstraints|FlowLayout|CardLayout|Component|"
    r"Container|Window|Frame|Dialog|ActionEvent|MouseEvent|KeyEvent|ActionListener|"
    r"MouseListener|MouseAdapter|KeyAdapter|ImageIcon|Icon|Toolkit|Robot|EventQueue|"
    r"Shape|Polygon|BasicStroke|AlphaComposite|RenderingHints|Paint|GradientPaint|"
    r"Border|BorderFactory|Event|Graphics)\b"
)
DECL = re.compile(r"(?<![\w.])(class|interface|enum)\s+([A-Za-z_$][\w$]*)|([{}])")
ESC = re.compile(r"\\(u+[0-9a-fA-F]{4}|.)", re.S)
ESC_MAP = {"n": "\n", "t": "\t", "r": "\r", '"': '"', "'": "'", "\\": "\\",
           "0": "\0", "b": "\b", "f": "\f", "s": " "}

CLASS_COLUMNS = ["origin", "class", "file", "lines", "superclass", "interfaces",
                 "field_count", "method_count", "swing", "serializable", "patched"]


def unescape(s):
    def rep(m):
        g = m.group(1)
        if g.startswith("u"):
            return chr(int(g.lstrip("u"), 16))
        return ESC_MAP.get(g, g)
    return ESC.sub(rep, s)


def scan(src):
    """Return (code, strings). code is src with comments and literal contents blanked
    (same length, so offsets match). strings is [(offset, value)]."""
    n = len(src)
    code = list(src)
    strings = []

    def blank(a, b):
        for k in range(a, b):
            if code[k] != "\n":
                code[k] = " "

    i = 0
    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            j = n if j == -1 else j
            blank(i, j)
            i = j
            continue
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            j = src.find("*/", i + 2)
            j = n if j == -1 else j + 2
            blank(i, j)
            i = j
            continue
        if c == '"':
            if src.startswith('"""', i):
                j = src.find('"""', i + 3)
                j = n if j == -1 else j
                strings.append((i, src[i + 3:j].replace("\n", " ")))
                blank(i + 1, j)
                i = j + 3
                continue
            j = i + 1
            buf = []
            while j < n and src[j] != '"' and src[j] != "\n":
                if src[j] == "\\" and j + 1 < n:
                    buf.append(src[j:j + 2])
                    j += 2
                else:
                    buf.append(src[j])
                    j += 1
            strings.append((i, unescape("".join(buf))))
            blank(i + 1, j)
            i = j + 1
            continue
        if c == "'":
            j = i + 1
            if j < n and src[j] == "\\":
                j += 2
            else:
                j += 1
            if j < n and src[j] == "'":
                blank(i + 1, j)
                i = j + 1
                continue
        i += 1
    return "".join(code), strings


def split_top(s):
    """Split a comma list, ignoring commas inside <...>."""
    out, depth, cur = [], 0, ""
    for ch in s:
        if ch == "<":
            depth += 1
        elif ch == ">":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return [x for x in out if x]


def parse_header(h):
    """From a 'class X extends A implements B' header return (kind, superclass, interfaces)."""
    m = re.search(r"\b(class|interface|enum)\s+([\w$.]+)", h)
    if not m:
        return "class", "", []
    kind = m.group(1)
    rest = h[m.end():]
    ext = re.search(r"\bextends\s+(.*?)(?=\bimplements\b|$)", rest, re.S)
    imp = re.search(r"\bimplements\s+(.*)$", rest, re.S)
    exts = split_top(ext.group(1)) if ext else []
    imps = split_top(imp.group(1)) if imp else []
    if kind == "interface":
        return kind, "", exts + imps
    if exts:
        sup = exts[0]
    else:
        sup = "java.lang.Enum" if kind == "enum" else "java.lang.Object"
    return kind, sup, imps


def analyze(path):
    """Parse one source file into its types, strings and imports."""
    src = path.read_text(encoding="utf-8", errors="replace")
    code, strings = scan(src)
    line_breaks = [m.start() for m in re.finditer("\n", src)]
    line_of = lambda off: bisect.bisect_right(line_breaks, off) + 1
    rel = path.relative_to(MODS)
    m = re.search(r"^\s*package\s+([\w.]+)\s*;", src, re.M)
    if m:
        pkg = m.group(1)
    else:
        parts = rel.parts[1:-1] if rel.parts[0] == "src" else rel.parts[2:-1]
        pkg = ".".join(parts)

    imports = []
    for m in re.finditer(r"^\s*import\s+(static\s+)?([\w.]+)(\.\*)?\s*;", code, re.M):
        imports.append((m.group(2), bool(m.group(3))))

    types = []
    stack = []
    depth = 0
    pending = None
    for m in DECL.finditer(code):
        if m.group(3) == "{":
            if pending is not None:
                name, ini = pending
                parent = stack[-1] if stack else None
                if parent:
                    binary = parent["bin"] + "$" + name
                else:
                    binary = (pkg + "." + name) if pkg else name
                stack.append({"name": name, "bin": binary, "depth": depth,
                              "ini": ini, "header": code[ini:m.start()]})
                pending = None
            depth += 1
        elif m.group(3) == "}":
            depth -= 1
            if stack and stack[-1]["depth"] == depth:
                t = stack.pop()
                t["end"] = m.end()
                types.append(t)
        else:
            pending = (m.group(2), m.start())
    for t in types:
        t["lines"] = line_of(t["end"]) - line_of(t["ini"]) + 1
        t["span"] = code[t["ini"]:t["end"]]
    types.sort(key=lambda t: t["ini"])
    return {"rel": rel, "pkg": pkg, "types": types, "strings": strings,
            "imports": imports}


def swing_in_span(span, imports):
    if any(p in span for p in SWING_PKGS):
        return True
    for name, wildcard in imports:
        if name.startswith(SWING_PKGS):
            if wildcard:
                if SWING_NAMES.search(span):
                    return True
            else:
                simple = name.split(".")[-1]
                if re.search(r"\b" + re.escape(simple) + r"\b", span):
                    return True
    return False


def javap(cp, fqn):
    r = subprocess.run([str(JAVAP), "-p", "-cp", str(cp), fqn],
                       capture_output=True, text=True)
    if r.returncode != 0:
        return None
    header = None
    fields, methods = [], 0
    for ln in r.stdout.splitlines():
        if not ln or ln.startswith("Compiled from"):
            continue
        if not ln.startswith(" "):
            if header is None and ln.rstrip().endswith("{"):
                header = ln.rstrip()[:-1].strip()
            continue
        s = ln.strip()
        if s == "static {};":
            continue
        if s.endswith(";"):
            if "(" in s:
                methods += 1
            else:
                fields.append(s)
    if header is None:
        return None
    _, sup, ifs = parse_header(header)
    return {"sup": sup, "ifs": ifs, "fields": fields, "methods": methods}


def main():
    game_src = sorted((MODS / "src").rglob("*.java"))
    mod_src = sorted((MODS / "mod" / "src" / "bfmod").rglob("*.java"))
    patch_src = sorted((MODS / "mod" / "patch").rglob("*.java"))

    files = []  # (origin, path, analysis)
    for origin, group in (("game", game_src), ("mod", mod_src), ("patch", patch_src)):
        for p in group:
            try:
                files.append((origin, p, analyze(p)))
            except Exception as e:  # keep going, report at the end
                print(f"failed to analyze {p}: {e}", file=sys.stderr)

    patched_classes = set()
    for origin, p, a in files:
        if origin == "patch":
            for t in a["types"]:
                patched_classes.add(t["bin"])

    # javap jobs: one per type
    jobs = []
    for idx, (origin, p, a) in enumerate(files):
        cp = ORIG if origin == "game" else BUILD
        for t in a["types"]:
            jobs.append((idx, t["bin"], cp, origin))
    with ThreadPoolExecutor(max_workers=max(4, os.cpu_count() or 4)) as ex:
        results = list(ex.map(lambda j: javap(j[2], j[1]), jobs))

    class_rows = []
    field_files = {}
    failures = []
    fallback = []
    types_by_file = {}
    for (idx, binary, cp, origin), info in zip(jobs, results):
        _, p, a = files[idx]
        t = next(t for t in a["types"] if t["bin"] == binary)
        if info is None:
            # Fallback: source header only, counts left blank.
            _, sup, ifs = parse_header(t["header"])
            info = {"sup": sup, "ifs": ifs, "fields": None, "methods": None}
            if origin == "game":
                failures.append(f"{binary} (no compiled class, data from source)")
            else:
                fallback.append(f"{origin} {binary}")
        serializable = any(i.split("<")[0].split(".")[-1] == "Serializable" for i in info["ifs"])
        swing = swing_in_span(t["span"], a["imports"])
        if origin == "game":
            patched = "yes" if binary in patched_classes else "no"
        elif origin == "patch":
            patched = "yes"
        else:
            patched = "no"
        class_rows.append({
            "origin": origin, "class": binary,
            "file": str(a["rel"]), "lines": t["lines"],
            "superclass": info["sup"], "interfaces": ", ".join(info["ifs"]),
            "field_count": "" if info["fields"] is None else len(info["fields"]),
            "method_count": "" if info["methods"] is None else info["methods"],
            "swing": "yes" if swing else "no",
            "serializable": "yes" if serializable else "no",
            "patched": patched,
        })
        if origin == "game" and not swing and info["fields"] is not None:
            field_files[binary] = info["fields"]
        types_by_file.setdefault(idx, []).append(t)

    # Strings: attribute each literal to the innermost type containing it
    strings_by_class = {}
    for idx, (origin, p, a) in enumerate(files):
        types = types_by_file.get(idx, [])
        for off, val in a["strings"]:
            v = val.strip()
            if len(v) < 3 or not re.search(r"[A-Za-z]", v):
                continue
            enclosing = [t for t in types if t["ini"] <= off < t["end"]]
            if not enclosing:
                continue
            target = max(enclosing, key=lambda t: t["ini"])
            lst = strings_by_class.setdefault(target["bin"], [])
            if v not in lst:
                lst.append(v)

    class_rows.sort(key=lambda r: (r["class"], ORIGIN_ORDER[r["origin"]]))

    # Write outputs
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    fields_dir = OUTPUT_DIR / "campos"
    if fields_dir.exists():
        shutil.rmtree(fields_dir)
    fields_dir.mkdir()
    for name, fields in field_files.items():
        (fields_dir / f"{name}.txt").write_text(
            "".join(f + "\n" for f in fields), encoding="utf-8")

    def clean(x):
        return str(x).replace("\t", " ").replace("\n", " ")
    with open(OUTPUT_DIR / "classes.tsv", "w", encoding="utf-8") as f:
        f.write("\t".join(CLASS_COLUMNS) + "\n")
        for r in class_rows:
            f.write("\t".join(clean(r[c]) for c in CLASS_COLUMNS) + "\n")

    with open(OUTPUT_DIR / "strings.tsv", "w", encoding="utf-8") as f:
        f.write("class\tstring\n")
        for cls in sorted(strings_by_class):
            for s in strings_by_class[cls]:
                f.write(f"{clean(cls)}\t{clean(s)}\n")

    write_readme(class_rows, failures, fallback)
    print(f"classes: {len(class_rows)} rows; field files: {len(field_files)}; "
          f"strings: {sum(len(v) for v in strings_by_class.values())}")
    print(f"no compiled class (game): {len(failures)}; fallback (mod/patch): {len(fallback)}")
    for x in failures + fallback:
        print("  -", x)


def write_readme(rows, failures, fallback):
    today = datetime.date.today().isoformat()
    L = []
    L.append("# Class inventory\n")
    L.append(f"Generated on {today} by `tools/inventory/generate.py`. Do not edit by hand.\n")
    L.append("Origins: `game` = decompiled in `brasfoot-mods/src` (obfuscated), `patch` = the")
    L.append("version recompiled in `brasfoot-mods/mod/patch`, `mod` = `brasfoot-mods/mod/src/bfmod`.")
    L.append("A patched class appears twice: the `game` row (with `patched=yes`) and the")
    L.append("`patch` row. Nested types appear with `$` (e.g. `a.cR$1`); anonymous classes")
    L.append("(`new X() {}`) are not listed.\n")

    L.append("## Counts by origin\n")
    L.append("| origin | classes | swing | serializable | patched |")
    L.append("|---|---|---|---|---|")
    for o in ("game", "patch", "mod"):
        rs = [r for r in rows if r["origin"] == o]
        L.append(f"| {o} | {len(rs)} | {sum(r['swing'] == 'yes' for r in rs)} | "
                 f"{sum(r['serializable'] == 'yes' for r in rs)} | "
                 f"{sum(r['patched'] == 'yes' for r in rs)} |")
    L.append("")

    L.append("## Counts by package and origin\n")
    L.append("| package | game | patch | mod | total |")
    L.append("|---|---|---|---|---|")
    by_pkg = defaultdict(Counter)
    for r in rows:
        by_pkg[r["class"].split(".")[0]][r["origin"]] += 1
    for pk in sorted(by_pkg):
        c = by_pkg[pk]
        L.append(f"| {pk} | {c['game']} | {c['patch']} | {c['mod']} | {sum(c.values())} |")
    L.append("")

    L.append("## 30 largest classes (by lines)\n")
    L.append("| class | origin | lines | file |")
    L.append("|---|---|---|---|")
    for r in sorted(rows, key=lambda r: -int(r["lines"]))[:30]:
        L.append(f"| {r['class']} | {r['origin']} | {r['lines']} | {r['file']} |")
    L.append("")

    L.append("## Not processed\n")
    if not failures and not fallback:
        L.append("Every class was processed by javap.\n")
    for x in failures:
        L.append(f"- game: {x}")
    for x in fallback:
        L.append(f"- {x}: no compiled class in build/classes (source data only, no counts)")
    L.append("")

    L.append("## How to regenerate\n")
    L.append("```sh")
    L.append("cd /home/pedro/brasfoot-next-2 && python3 tools/inventory/generate.py")
    L.append("```\n")
    L.append("Needs `~/.jdks/corretto-21.0.11/bin/javap`, the jar extracted in")
    L.append("`brasfoot-mods/orig/jar` and `brasfoot-mods/build/classes` (run the mod's")
    L.append("`build.sh` if it is missing). `serializable` only detects `java.io.Serializable`:")
    L.append("Kryo registers classes without `register` calls in the code, so the list cannot")
    L.append("be detected.")
    (OUTPUT_DIR / "README.md").write_text("\n".join(L) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
