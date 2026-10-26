#!/usr/bin/env python3
"""Generates the save-mirroring part of the game model and its importer from:
- crates/persistence/data/kryo-schema.json (the serialized fields of every class),
- tools/modelgen/list_elements.json (element type of every list field),
- tools/modelgen/names.py (readable names).

Outputs (do not edit by hand):
- crates/model/src/saved.rs: the structs,
- crates/persistence/src/import/generated.rs: graph to model conversion,
- docs/mapa/fields.md: the field dictionary.

Usage: gen_model.py [--elements-from <exporter summary.txt>]   (the option refreshes list_elements.json)
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
import names as N  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
SCHEMA = json.load(open(os.path.join(ROOT, "crates/persistence/data/kryo-schema.json")))["classes"]
ELEMENTS_FILE = os.path.join(os.path.dirname(__file__), "list_elements.json")

# class -> base class it is folded into (itself for plain classes)
BASE = {c: c for c in N.CLASSES}
for base, subs in N.SUBCLASSES.items():
    for sub in subs:
        BASE[sub] = base
CONCRETE = [c for c in N.CLASSES if c not in N.SUBCLASSES] + [s for subs in N.SUBCLASSES.values() for s in subs]

ID_OF = {"best.F": "PlayerId", "best.ah": "ClubId", "best.al": "CoachId", "best.v": "StadiumId", "best.I": "MatchId",
         "best.at": "CompetitionId", "best.L": "StageId", "best.Z": "CountryId", "best.w": "StateLeagueId"}
PRIMITIVES = {"int": "i32", "long": "i64", "double": "f64", "float": "f32", "boolean": "bool", "short": "i16",
              "byte": "i8", "char": "u16"}


def refresh_elements(summary_path):
    observed = {}
    cls = None
    for line in open(summary_path):
        m = re.match(r"^(\S+) \| (\d+) \|", line)
        if m:
            cls = m.group(1)
            continue
        m = re.match(r"^    (\S+) : java\.util\.ArrayList(?:\s+\[\S+\])?\s+-> (.*)$", line.rstrip())
        if m and cls:
            types = set()
            nullable = False
            for part in re.findall(r"ArrayList<([^>]*)>", m.group(2)):
                for t in part.split("|"):
                    if t == "null":
                        nullable = True
                    elif t and t != "...":
                        types.add(t)
            observed[f"{cls}.{m.group(1)}"] = {"types": sorted(types), "nullable": nullable}
    for (c, f), t in N.LIST_ELEMENTS.items():
        entry = observed.setdefault(f"{c}.{f}", {"types": [], "nullable": False})
        if not entry["types"]:
            entry["types"] = [t]
    json.dump(observed, open(ELEMENTS_FILE, "w"), indent=1, sort_keys=True)


MISSING = []
DEFAULTS = {}
SOURCES = os.path.expanduser("~/brasfoot-mods")
_source_cache = {}


def initializer(declaring, field):
    """The Java initializer of a field, as written in the decompiled source ('' if none)."""
    if declaring not in _source_cache:
        rel = declaring.replace(".", "/") + ".java"
        text = ""
        for base in ("mod/patch", "src"):
            path = os.path.join(SOURCES, base, rel)
            if os.path.exists(path):
                text = open(path, encoding="utf-8").read()
                break
        _source_cache[declaring] = text
    m = re.search(r"^   [\w ]*?[\w.<>\[\]]+ " + re.escape(field) + r" = (.+?);$", _source_cache[declaring], re.M | re.S)
    return m.group(1).strip() if m else ""


def rust_default(rtype, init):
    """A Rust expression for a Java initializer, or None for the type's default."""
    init = " ".join(init.split())
    if not init or init == "null":
        return None
    if rtype in ("i32", "i64", "i16", "i8") and re.fullmatch(r"-?\d+L?", init):
        value = init.rstrip("L")
        return None if int(value) == 0 else value
    if rtype == "f64" and re.fullmatch(r"-?\d+\.\d+", init):
        return None if float(init) == 0 else init
    if rtype == "bool" and init in ("true", "false"):
        return "true" if init == "true" else None
    if rtype == "String" and re.fullmatch(r'"[^"]*"', init):
        return None if init == '""' else f"{init}.to_string()"
    if rtype.startswith("Vec<"):
        m = re.fullmatch(r"new (?:int|boolean|double)\[(\d+)\]", init)
        if m:
            zero = {"Vec<i32>": "0", "Vec<bool>": "false", "Vec<f64>": "0.0"}[rtype]
            return f"vec![{zero}; {m.group(1)}]"
        m = re.fullmatch(r"new int\[(\d+)\]\[(\d+)\]", init)
        if m:
            return f"vec![vec![0; {m.group(2)}]; {m.group(1)}]"
        m = re.fullmatch(r"new (?:int|boolean|double)\[\]\{(.*)\}", init)
        if m:
            return f"vec![{m.group(1)}]"
        m = re.fullmatch(r"new int\[\]\[\]\{(.*)\}", init)
        if m:
            rows = re.findall(r"\{([^{}]*)\}", m.group(1))
            return "vec![" + ", ".join(f"vec![{r}]" for r in rows) + "]"
    return None



def snake(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def rust_name(cls, field):
    return N.CLASS.get(cls, {}).get(field) or N.CLASS.get(BASE[cls], {}).get(field) or N.GLOBAL.get(field)


def element_type(elements, cls, field, declaring):
    for key in (f"{cls}.{field}", f"{declaring}.{field}"):
        if key in elements and elements[key]["types"]:
            types = {BASE.get(t, t) for t in elements[key]["types"]}
            if len(types) > 1:
                raise SystemExit(f"{key}: mixed element types {types}")
            return types.pop(), elements[key]["nullable"]
    # look at the same field in the other concrete classes of the same base
    for other in CONCRETE:
        key = f"{other}.{field}"
        if BASE.get(other) == BASE.get(cls) and key in elements and elements[key]["types"]:
            return BASE.get(elements[key]["types"][0], elements[key]["types"][0]), elements[key]["nullable"]
    raise SystemExit(f"{cls}.{field}: unknown list element type (add it to names.LIST_ELEMENTS)")


def value_type(java):
    """(rust type, importer reader) of one value stored in a list or field slot."""
    if java in ID_OF:
        return ID_OF[java], f"ctx.id_of_{snake(ID_OF[java])}(v)"
    if java in BASE and BASE[java] in ID_OF:
        return ID_OF[BASE[java]], f"ctx.id_of_{snake(ID_OF[BASE[java]])}(v)"
    if java in N.CLASSES:
        return N.CLASSES[java][0], f"convert_{snake(N.CLASSES[java][0])}(ctx, ctx.node_of(v))"
    return {
        "java.lang.Integer": ("i32", "ctx.value_int(v)"),
        "java.lang.String": ("String", "ctx.value_string(v)"),
    }[java]


def field_type(cls, f, elements):
    """(rust type, reader expression with `node`) for a schema field."""
    name = f["name"]
    kind = f["type"]
    ref = N.INT_REFS.get((cls, name)) or N.INT_REFS.get((BASE[cls], name))
    if ref:
        return f"Option<{ref}>", f'ctx.int_ref_{snake(ref)}(node, "{name}")'
    if kind in PRIMITIVES:
        return PRIMITIVES[kind], f'ctx.{kind}(node, "{name}")'
    declared = f["declared"]
    if declared == "java.lang.Boolean":
        return "bool", f'ctx.boxed_boolean(node, "{name}")'
    if declared == "java.lang.Integer":
        return "Option<i32>", f'ctx.boxed_int(node, "{name}")'
    if declared == "java.lang.String":
        if (cls, name) in N.NULLABLE_STRINGS or (BASE[cls], name) in N.NULLABLE_STRINGS:
            return "Option<String>", f'ctx.optional_string(node, "{name}")'
        return "String", f'ctx.string(node, "{name}")'
    if declared in ("java.util.Calendar", "java.util.GregorianCalendar"):
        return "EpochMillis", f'ctx.calendar(node, "{name}")'
    arrays = {"[I": ("Vec<i32>", "int_array"), "[[I": ("Vec<Vec<i32>>", "int_matrix"), "[Z": ("Vec<bool>", "boolean_array"),
              "[D": ("Vec<f64>", "double_array"), "[Ljava.lang.String;": ("Vec<String>", "string_array")}
    if declared in arrays:
        t, reader = arrays[declared]
        return t, f'ctx.{reader}(node, "{name}")'
    if declared.startswith("[L") and declared[2:-1] in BASE:
        t, reader = value_type(declared[2:-1])
        return f"Vec<Option<{t}>>", f'ctx.items(node, "{name}", |ctx, v| ctx.nullable(v, |ctx, v| {reader}))'
    if declared in ("java.util.ArrayList", "java.util.List"):
        element, nullable = element_type(elements, cls, name, f["declaring"])
        t, reader = value_type(element)
        if nullable:
            return f"Vec<Option<{t}>>", f'ctx.items(node, "{name}", |ctx, v| ctx.nullable(v, |ctx, v| {reader}))'
        return f"Vec<{t}>", f'ctx.items(node, "{name}", |ctx, v| {reader})'
    if declared in BASE or declared in ID_OF:
        t, reader = value_type(declared)
        if t in ID_OF.values():
            return f"Option<{t}>", f'ctx.reference(node, "{name}", |ctx, v| {reader})'
        if (cls, name) in {("best.ah", "nx")}:
            return t, f'ctx.embedded(node, "{name}", |ctx, v| {reader}).unwrap_or_default()'
        return f"Option<{t}>", f'ctx.reference(node, "{name}", |ctx, v| {reader})'
    raise SystemExit(f"{cls}.{name}: unsupported type {declared}")


def class_fields(cls):
    """Serialized fields of a concrete class: schema order, constants and dropped ones removed."""
    s = SCHEMA[cls]
    return [f for f in s["fields"] if f["name"] not in N.CONSTANTS.get(cls, []) and (cls, f["name"]) not in N.DROPPED
            and (BASE[cls], f["name"]) not in N.DROPPED]


def main():
    if "--elements-from" in sys.argv:
        refresh_elements(sys.argv[sys.argv.index("--elements-from") + 1])
    elements = json.load(open(ELEMENTS_FILE))

    structs = {}  # base class -> ordered {rust field: (type, doc, {concrete class: reader})}
    for cls in CONCRETE:
        base = BASE[cls]
        fields = structs.setdefault(base, {})
        combined = {obf: (group, i) for group, obfs in N.COMBINE.get(cls, []) for i, obf in enumerate(obfs)}
        for f in class_fields(cls):
            name = f["name"]
            if name in combined:
                group, index = combined[name]
                entry = fields.setdefault(group, ["[i32; 7]", [], {}])
                entry[1].append(f"`{name}`")
                entry[2].setdefault(cls, []).append(f'ctx.int(node, "{name}")')
                continue
            custom = N.CUSTOM.get((cls, name)) or N.CUSTOM.get((base, name))
            if custom:
                fields[custom[0]] = [custom[1], [f"`{name}`"], dict(fields.get(custom[0], [None, None, {}])[2], **{cls: custom[2]})]
                continue
            rname = rust_name(cls, name)
            if not rname:
                MISSING.append(f"{cls}.{name} ({f['type'] if f['type'] != 'object' else f['declared']})")
                continue
            rtype, reader = field_type(cls, f, elements)
            entry = fields.get(rname)
            default = rust_default(rtype, initializer(f["declaring"], name))
            if default is not None:
                DEFAULTS.setdefault((base, rname), default)
            if entry is None:
                fields[rname] = [rtype, [f"`{name}`"], {cls: reader}]
            else:
                if entry[0] != rtype:
                    MISSING.append(f"{cls}.{name} -> {rname}: {rtype} clashes with {entry[0]}")
                if f"`{name}`" not in entry[1]:
                    MISSING.append(f"{cls}.{name} -> {rname}: name already used by {entry[1]}")
                entry[2][cls] = reader

    for cls, transients in N.TRANSIENT.items():
        for rname, rtype, doc, reader in transients:
            structs[BASE[cls]][rname] = [rtype, [doc], {cls: reader}]
    for cls, extras in N.EXTRA.items():
        for rname, rtype, doc in extras:
            structs[BASE[cls]][rname] = [rtype, [doc], {cls: "Default::default()"}]
    if MISSING:
        raise SystemExit("no readable name:\n  " + "\n  ".join(MISSING))
    emit_model(structs)
    emit_canonical(structs)
    emit_importer(structs)
    emit_fixture_elements(elements)
    emit_docs(structs)


def kind_enum(base):
    return {"best.at": "CompetitionKind", "best.L": "StageKind"}[base]


def emit_model(structs):
    out = ["// Generated by tools/modelgen/gen_model.py from the Kryo schema and tools/modelgen/names.py.",
           "// Do not edit: change the dictionary and regenerate. Methods live in hand-written modules.",
           "//! The data of the classes the original game saves, one struct per class (subclasses folded",
           "//! into their base with a kind). Each field documents its obfuscated name.", "",
           "use serde::{Deserialize, Serialize};", "",
           "use crate::ids::*;", "",
           "/// Java epoch milliseconds (what `java.util.Calendar` holds).",
           "pub type EpochMillis = i64;", ""]
    for base, subs in N.SUBCLASSES.items():
        out.append(f"/// The concrete class of a `{base}`.")
        out.append("#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]")
        out.append(f"pub enum {kind_enum(base)} {{")
        for i, (sub, variant) in enumerate(subs.items()):
            if i == 0:
                out.append("    #[default]")
            out.append(f"    /// `{sub}`")
            out.append(f"    {variant},")
        out.append("}")
        out.append("")
    for base, fields in structs.items():
        rust, _ = N.CLASSES[base]
        out.append(f"/// `{base}`" + (f" and its subclasses ({', '.join(N.SUBCLASSES[base])})" if base in N.SUBCLASSES else ""))
        has_defaults = any((base, r) in DEFAULTS for r in fields)
        derive_default = "" if has_defaults else "Default, "
        out.append(f"#[derive(Clone, Debug, {derive_default}PartialEq, Serialize, Deserialize)]")
        out.append(f"pub struct {rust} {{")
        if base in N.SUBCLASSES:
            out.append("    pub class: " + kind_enum(base) + ",")
        for rname, (rtype, obfs, readers) in fields.items():
            doc = ", ".join(obfs)
            if obfs and not obfs[0].startswith("`"):
                out.append("    /// " + obfs[0] + " Not in the original save.")
                out.append("    #[serde(default)]")
                out.append(f"    pub {rname}: {rtype},")
                continue
            if base in N.SUBCLASSES and len(readers) < len(N.SUBCLASSES[base]):
                doc += " (" + ", ".join(sorted(readers)) + ")"
            out.append(f"    /// {doc}")
            out.append(f"    pub {rname}: {rtype},")
        out.append("}")
        out.append("")
        if has_defaults:
            out.append(f"/// The Java field initialisers of `{base}`.")
            out.append(f"impl Default for {rust} {{")
            out.append("    fn default() -> Self {")
            out.append(f"        {rust} {{")
            for rname in fields:
                if (base, rname) in DEFAULTS:
                    out.append(f"            {rname}: {DEFAULTS[(base, rname)]},")
            out.append("            ..unsafe_default()".replace("unsafe_default()", f"{rust}::zeroed()"))
            out.append("        }")
            out.append("    }")
            out.append("}")
            out.append("")
            out.append(f"impl {rust} {{")
            out.append("    /// Every field at its type's default, before the Java initialisers.")
            out.append("    fn zeroed() -> Self {")
            out.append(f"        {rust} {{")
            if base in N.SUBCLASSES:
                out.append("            class: Default::default(),")
            for rname in fields:
                out.append(f"            {rname}: Default::default(),")
            out.append("        }")
            out.append("    }")
            out.append("}")
            out.append("")
    open(os.path.join(ROOT, "crates/model/src/saved.rs"), "w").write("\n".join(out))


def emit_canonical(structs):
    out = ["// Generated by tools/modelgen/gen_model.py. Do not edit.",
           "//! The canonical walk of every saved struct (see [`crate::canonical`]).", "",
           "use crate::canonical::{Canonical, Visit};", "use crate::saved::*;", ""]
    for base, fields in structs.items():
        rust, _ = N.CLASSES[base]
        out.append(f"impl Visit for {rust} {{")
        out.append("    fn visit(&self, c: &mut Canonical) {")
        if base in N.SUBCLASSES:
            out.append('        c.field("class");')
            out.append("        c.line(&format!(\"{:?}\", self.class));")
        for rname, (rtype, obfs, readers) in fields.items():
            if obfs and not obfs[0].startswith("`"):
                continue  # added by the new game: not in the original's saves
            if (base, rname) in N.CANONICAL_SKIP:
                continue
            out.append(f'        c.field("{rname}");')
            out.append(f"        self.{rname}.visit(c);")
        out.append("    }")
        out.append("}")
        out.append("")
    open(os.path.join(ROOT, "crates/model/src/canonical_generated.rs"), "w").write("\n".join(out))


def emit_importer(structs):
    out = ["// Generated by tools/modelgen/gen_model.py. Do not edit.",
           "//! Conversion of each saved class from the Kryo graph to the model.", "",
           "use brasfoot_model::saved::*;", "",
           "use super::context::Context;", "use crate::kryo::NodeId;", ""]
    for base, fields in structs.items():
        rust, _ = N.CLASSES[base]
        out.append(f"pub fn convert_{snake(rust)}(ctx: &Context, node: NodeId) -> {rust} {{")
        if base in N.SUBCLASSES:
            out.append("    let class = match ctx.class_name(node) {")
            for sub, variant in N.SUBCLASSES[base].items():
                out.append(f'        "{sub}" => {kind_enum(base)}::{variant},')
            out.append(f'        other => panic!("not a {base}: {{other}}"),')
            out.append("    };")
            out.append(f"    let mut value = {rust} {{ class, ..Default::default() }};")
            out.append("    match class {")
            for sub, variant in N.SUBCLASSES[base].items():
                assigns = []
                for rname, (rtype, obfs, readers) in fields.items():
                    if sub in readers:
                        assigns.append(f"            value.{rname} = {render(readers[sub], rtype)};")
                out.append(f"        {kind_enum(base)}::{variant} => {{")
                out.extend(assigns)
                out.append("        }")
            out.append("    }")
            out.append("    value")
        else:
            out.append(f"    {rust} {{")
            for rname, (rtype, obfs, readers) in fields.items():
                out.append(f"        {rname}: {render(readers[base], rtype)},")
            out.append("    }")
        out.append("}")
        out.append("")
    os.makedirs(os.path.join(ROOT, "crates/persistence/src/import"), exist_ok=True)
    open(os.path.join(ROOT, "crates/persistence/src/import/generated.rs"), "w").write("\n".join(out))


def render(reader, rtype):
    if isinstance(reader, list):
        return "[" + ", ".join(reader) + "]"
    return reader


def emit_fixture_elements(elements):
    """Element class of each list, for tools/exporter's fictional fixture."""
    lines = []
    for key, entry in sorted(elements.items()):
        if entry["types"]:
            lines.append(f"{key}\t{entry['types'][0]}\t{'nullable' if entry['nullable'] else 'nonnull'}")
    open(os.path.join(ROOT, "tools/exporter/list-elements.tsv"), "w").write("\n".join(lines) + "\n")


def emit_docs(structs):
    out = ["# Saved fields", "", "Generated by `tools/modelgen/gen_model.py` from `tools/modelgen/names.py`. Do not edit.", "",
           "Obfuscated field names are global: the same name means the same thing in every class.", ""]
    for base, fields in structs.items():
        rust, storage = N.CLASSES[base]
        subs = N.SUBCLASSES.get(base)
        out.append(f"## `{base}` → `{rust}` ({storage})")
        if subs:
            out.append("")
            out.append(", ".join(f"`{s}` {v}" for s, v in subs.items()))
        out.append("")
        out.append("| Obfuscated | Readable | Type |")
        out.append("|---|---|---|")
        for rname, (rtype, obfs, readers) in fields.items():
            if obfs and not obfs[0].startswith("`"):
                out.append(f"| (new) | `{rname}` | `{rtype}` |")
                continue
            where = "" if not subs or len(readers) == len(subs) else " (" + ", ".join(sorted(readers)) + ")"
            out.append(f"| {', '.join(obfs)}{where} | `{rname}` | `{rtype}` |")
        out.append("")
    dropped = [f"- `{c}.{f}`: {why}" for (c, f), why in N.DROPPED.items()]
    out += ["## Not kept", ""] + dropped + [f"- `{c}.{', '.join(fs)}`: constant tables re-created by the constructor"
                                           for c, fs in N.CONSTANTS.items()] + [""]
    open(os.path.join(ROOT, "docs/mapa/fields.md"), "w").write("\n".join(out))


if __name__ == "__main__":
    main()
