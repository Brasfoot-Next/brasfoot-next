#!/usr/bin/env python3
"""Finds the places where Vineflower printed `x -= c` / `x += c` on a double or float as
`x--` / `x++` because 1 <= c < 2 (it compares the constant's integer part with 1).

Reads `javap -c -p` of every class of the original jar and reports each
load x; push c; add|sub; store x with c in [1, 2) and c != 1. Usage: scan.py <javap output>...."""
import os
import re
import sys

CONST = re.compile(r"(ldc2_w|ldc_w|ldc)\s+#\d+\s+// (double|float) ([-0-9.eE]+)[dfDF]")
OP = re.compile(r"\d+: ([dfil](add|sub))\b")

CLASS = re.compile(r"^(?:public |final |abstract |private |protected |static )*(?:class|interface|enum) (\S+)")

def scan(path):
    cls = None
    method = None
    lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
    out = []
    for i, line in enumerate(lines):
        c = CLASS.match(line)
        if c:
            cls = c.group(1)
        s = line.strip()
        if s.endswith(");") or (s.endswith(";") and "(" in s and not s[0].isdigit()):
            method = s
        m = CONST.search(line)
        if not m:
            continue
        value = float(m.group(3))
        if not (1.0 <= abs(value) < 2.0) or abs(value) == 1.0:
            continue
        nxt = lines[i + 1] if i + 1 < len(lines) else ""
        op = OP.search(nxt)
        if op and op.group(1)[0] in "df":
            prev = lines[i - 1].strip() if i > 0 else ""
            out.append((cls, method, value, op.group(1), prev, lines[i + 2].strip() if i + 2 < len(lines) else ""))
    return out

for path in sys.argv[1:]:
        for cls, method, value, op, prev, after in scan(path):
            print(f"{cls}\t{value}\t{op}\t{method}\t[{prev.split(': ',1)[-1]}] -> [{after.split(': ',1)[-1]}]")
