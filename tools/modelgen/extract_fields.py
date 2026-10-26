#!/usr/bin/env python3
"""Lists every serialized field of the classes found in saves: class, field, declaring class,
declared type, the element types observed in a save export summary, and the Java initializer.
Input: crates/persistence/data/kryo-schema.json, an exporter summary, the decompiled sources.
Usage: extract_fields.py <summary.txt> > fields.tsv
"""
import json, re, sys, os

ROOT = os.path.join(os.path.dirname(__file__), "../..")
schema = json.load(open(os.path.join(ROOT, "crates/persistence/data/kryo-schema.json")))["classes"]
SRC = os.path.expanduser("~/brasfoot-mods")

observed = {}
cls = None
for line in open(sys.argv[1]):
    m = re.match(r"^(\S+) \| (\d+) \|", line)
    if m:
        cls = m.group(1)
        continue
    m = re.match(r"^    (\S+) : (\S+)(?:\s+\[\S+\])?(?:\s+-> (.*))?$", line.rstrip())
    if m and cls:
        observed[(cls, m.group(1).split("#")[-1])] = (m.group(3) or "").strip()

def source(c):
    rel = c.replace(".", "/") + ".java"
    for base in ("mod/patch", "src"):
        p = os.path.join(SRC, base, rel)
        if os.path.exists(p):
            return open(p, encoding="utf-8").read()
    return ""

save_classes = sorted({c for (c, _) in observed if not c.startswith(("java.", "["))})
for c in save_classes:
    s = schema.get(c)
    if not s or s.get("serializer") != "field":
        continue
    for f in s["fields"]:
        decl = source(f["declaring"])
        m = re.search(r"^   [\w ]*?[\w.<>\[\]]+ " + re.escape(f["name"]) + r"(?: = (.+?))?;$", decl, re.M)
        init = (m.group(1) if m and m.group(1) else "").replace("\t", " ")
        ftype = f["type"] if f["type"] != "object" else f["declared"]
        print("\t".join([c, f["name"], f["declaring"], ftype, observed.get((c, f["name"]), ""), init[:80]]))
