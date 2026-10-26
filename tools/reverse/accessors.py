#!/usr/bin/env python3
"""Lists, for a decompiled game class, each instance field with its type, initializer and the
methods that only return or assign it (getters/setters), to help name obfuscated fields.

Usage: accessors.py <path/to/Class.java>
"""
import re
import sys

src = open(sys.argv[1], encoding="utf-8").read()
first_member = re.search(r"^   (?:public |private |protected )?(?:static )?[\w.<>\[\]]* ?\w+\([^)]*\) (?:throws [\w., ]+ )?\{$", src, re.M)
header = src[: first_member.start()] if first_member else src
fields = re.findall(r"^   (?:private |public |protected )?((?:transient |final )*)([\w.<>\[\], ]+?) (\w+)(?: = ([^;]+))?;$", header, re.M)
methods = list(re.finditer(r"^   (?:public |private |protected )?(?:static )?(?:final )?(?:synchronized )?([\w.<>\[\]]+) (\w+)\(([^)]*)\) \{\n(.*?)^   \}", src, re.M | re.S))
for mods, ftype, name, init in fields:
    if "static" in mods:
        continue
    getters, setters, others = [], [], 0
    for m in methods:
        body = m.group(4).strip()
        if re.fullmatch(rf"return (?:this\.)?{re.escape(name)};", body):
            getters.append(m.group(2) + "()")
        elif re.fullmatch(rf"(?:this\.)?{re.escape(name)} = \w+;", body):
            setters.append(f"{m.group(2)}({m.group(3).split(' ')[0]})")
        elif re.search(rf"\b(?:this\.)?{re.escape(name)}\b", body):
            others += 1
    t = "transient " if "transient" in mods else ""
    print(f"{name:>6} {t}{ftype}{' = ' + init if init else ''}  get={','.join(getters) or '-'} set={','.join(setters) or '-'} used_in={others}")
