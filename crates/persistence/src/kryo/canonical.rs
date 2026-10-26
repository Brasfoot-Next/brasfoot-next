//! The canonical text form of a graph, byte-identical to `tools/exporter`'s `CanonicalDump`
//! (see that class for the format). Used only to validate the reader against the game itself.

use std::io::{self, Write};

use super::graph::{Graph, NodeData, Value};

pub fn write_canonical(graph: &Graph, out: &mut impl Write) -> io::Result<()> {
    for &root in &graph.roots {
        // ids restart per root, like Kryo's references
        let mut dump = Dump { graph, ids: vec![0; graph.nodes.len()], next_id: 1, out: &mut *out };
        dump.value(root)?;
    }
    Ok(())
}

struct Dump<'a, W: Write> {
    graph: &'a Graph,
    ids: Vec<u32>,
    next_id: u32,
    out: &'a mut W,
}

impl<W: Write> Dump<'_, W> {
    fn string(&mut self, s: &str) -> io::Result<()> {
        let mut line = String::with_capacity(s.len() + 3);
        line.push_str("S ");
        for c in s.chars() {
            match c {
                '\\' => line.push_str("\\\\"),
                '\n' => line.push_str("\\n"),
                '\r' => line.push_str("\\r"),
                c if (c as u32) < 0x20 => line.push_str(&format!("\\x{:02x}", c as u32)),
                c => line.push(c),
            }
        }
        line.push('\n');
        self.out.write_all(line.as_bytes())
    }

    fn value(&mut self, value: Value) -> io::Result<()> {
        let graph = self.graph;
        match value {
            Value::Null => writeln!(self.out, "N"),
            Value::Bool(b) => writeln!(self.out, "z {}", b as u8),
            Value::Byte(n) => writeln!(self.out, "b {n}"),
            Value::Short(n) => writeln!(self.out, "s {n}"),
            Value::Char(n) => writeln!(self.out, "c {n}"),
            Value::Int(n) => writeln!(self.out, "i {n}"),
            Value::Long(n) => writeln!(self.out, "l {n}"),
            Value::Float(f) => writeln!(self.out, "f {:x}", f.to_bits()),
            Value::Double(d) => writeln!(self.out, "d {:x}", d.to_bits()),
            Value::Str(id) => self.string(graph.string(id)),
            Value::Enum { class, ordinal } => writeln!(self.out, "E {} {ordinal}", graph.class_name(class)),
            Value::Node(node_id) => {
                let seen = self.ids[node_id as usize];
                if seen != 0 {
                    return writeln!(self.out, "^{seen}");
                }
                let id = self.next_id;
                self.next_id += 1;
                self.ids[node_id as usize] = id;
                let node = graph.node(node_id);
                let class = &graph.classes[node.class as usize];
                let name = &class.name;
                match &node.data {
                    NodeData::Object(values) => {
                        writeln!(self.out, "{{{id} {name} {}", values.len())?;
                        for (field, &v) in class.fields.iter().zip(values) {
                            writeln!(self.out, ".{field}")?;
                            self.value(v)?;
                        }
                        Ok(())
                    }
                    NodeData::Items(items) => {
                        writeln!(self.out, "[{id} {name} {}", items.len())?;
                        items.iter().try_for_each(|&v| self.value(v))
                    }
                    NodeData::Map(entries) => {
                        writeln!(self.out, "<{id} {name} {}", entries.len())?;
                        entries.iter().try_for_each(|&(k, v)| {
                            self.value(k)?;
                            self.value(v)
                        })
                    }
                    NodeData::Calendar(c) => {
                        writeln!(self.out, "@{id} {name}")?;
                        self.string(&c.time_zone)?;
                        writeln!(self.out, "l {}", c.millis)?;
                        writeln!(self.out, "z {}", c.lenient as u8)?;
                        writeln!(self.out, "i {}", c.first_day_of_week)?;
                        writeln!(self.out, "i {}", c.minimal_days_in_first_week)?;
                        writeln!(self.out, "l {}", c.gregorian_change)
                    }
                    data => {
                        let lines: Vec<String> = match data {
                            NodeData::Ints(a) => a.iter().map(|n| format!("i {n}")).collect(),
                            NodeData::Longs(a) => a.iter().map(|n| format!("l {n}")).collect(),
                            NodeData::Doubles(a) => a.iter().map(|d| format!("d {:x}", d.to_bits())).collect(),
                            NodeData::Floats(a) => a.iter().map(|f| format!("f {:x}", f.to_bits())).collect(),
                            NodeData::Booleans(a) => a.iter().map(|&b| format!("z {}", b as u8)).collect(),
                            NodeData::Shorts(a) => a.iter().map(|n| format!("s {n}")).collect(),
                            NodeData::Bytes(a) => a.iter().map(|n| format!("b {n}")).collect(),
                            NodeData::Chars(a) => a.iter().map(|n| format!("c {n}")).collect(),
                            _ => unreachable!(),
                        };
                        writeln!(self.out, "[{id} {name} {}", lines.len())?;
                        lines.iter().try_for_each(|l| writeln!(self.out, "{l}"))
                    }
                }
            }
        }
    }
}
