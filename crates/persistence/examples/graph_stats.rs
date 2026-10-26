//! Ownership analysis of a `.s22` graph: per class, how many instances there are, how many are
//! referenced from more than one place, and from which fields they are referenced.
//! Usage: cargo run --release -p brasfoot-persistence --example graph_stats <file.s22> [class]

use std::collections::{BTreeMap, HashMap};

use brasfoot_persistence::kryo::{self, NodeData, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let graph = kryo::read_graph(std::fs::read(&args[1])?)?;
    let only = args.get(2).cloned();
    let mut incoming = vec![0u32; graph.nodes.len()];
    // (target class) -> (holder "class.field" or "class[]") -> count
    let mut holders: HashMap<u32, BTreeMap<String, u64>> = HashMap::new();
    for node in &graph.nodes {
        let holder_class = graph.class_name(node.class);
        let mut visit = |v: &Value, place: String| {
            if let Value::Node(t) = v {
                incoming[*t as usize] += 1;
                let target = graph.node(*t).class;
                *holders.entry(target).or_default().entry(place).or_default() += 1;
            }
        };
        match &node.data {
            NodeData::Object(values) => {
                for (i, v) in values.iter().enumerate() {
                    visit(v, format!("{holder_class}.{}", graph.classes[node.class as usize].fields[i]));
                }
            }
            NodeData::Items(items) => items.iter().for_each(|v| visit(v, format!("{holder_class}[]"))),
            NodeData::Map(entries) => entries.iter().for_each(|(k, v)| {
                visit(k, format!("{holder_class}{{key}}"));
                visit(v, format!("{holder_class}{{value}}"));
            }),
            _ => {}
        }
    }
    let mut per_class: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut class_ids: HashMap<String, u32> = HashMap::new();
    for (i, node) in graph.nodes.iter().enumerate() {
        let name = graph.class_name(node.class).to_string();
        class_ids.insert(name.clone(), node.class);
        let e = per_class.entry(name).or_default();
        e.0 += 1;
        if incoming[i] > 1 {
            e.1 += 1;
        }
    }
    for (name, (count, shared)) in &per_class {
        if only.as_ref().is_some_and(|o| o != name) {
            continue;
        }
        println!("{name}: {count} instances, {shared} shared");
        let Some(by_place) = holders.get(&class_ids[name]) else { continue };
        let mut list: Vec<_> = by_place.iter().collect();
        list.sort_by(|a, b| b.1.cmp(a.1));
        for (place, n) in list.iter().take(if only.is_some() { 100 } else { 8 }) {
            println!("    {n:>8}  {place}");
        }
    }
    Ok(())
}
