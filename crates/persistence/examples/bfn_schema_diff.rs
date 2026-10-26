//! Compares the field names of an older `.bfn` payload with what the current model writes for an
//! imported `.s22`, by path (array items collapsed), to write a format migration.
//! Usage: bfn_schema_diff <old.bfn> <save.s22>

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

fn collect(v: &ciborium::Value, path: &str, out: &mut BTreeMap<String, BTreeSet<String>>, texts: &mut BTreeMap<String, BTreeSet<String>>) {
    match v {
        ciborium::Value::Map(entries) => {
            let keys = out.entry(path.to_owned()).or_default();
            for (k, _) in entries {
                if let Some(k) = k.as_text() {
                    keys.insert(k.to_owned());
                }
            }
            for (k, v) in entries {
                if let Some(k) = k.as_text() {
                    if k == "class" {
                        if let Some(t) = v.as_text() {
                            texts.entry(path.to_owned()).or_default().insert(t.to_owned());
                        }
                    }
                    collect(v, &format!("{path}.{k}"), out, texts);
                }
            }
        }
        ciborium::Value::Array(items) => {
            for i in items {
                collect(i, &format!("{path}[]"), out, texts);
            }
        }
        _ => {}
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let old = args.next().unwrap();
    let s22 = args.next().unwrap();
    let mut f = std::io::BufReader::new(std::fs::File::open(&old).unwrap());
    // skip the header frame
    let mut head = [0u8; 8];
    f.read_exact(&mut head).unwrap();
    let len = u32::from_le_bytes(head[4..8].try_into().unwrap()) as usize;
    let mut skip = vec![0u8; len];
    f.read_exact(&mut skip).unwrap();
    let old_value: ciborium::Value = ciborium::from_reader(zstd::Decoder::new(f).unwrap()).unwrap();
    let world = brasfoot_persistence::import::import_s22(std::fs::read(&s22).unwrap()).unwrap();
    let mut bytes = Vec::new();
    ciborium::into_writer(&world, &mut bytes).unwrap();
    let new_value: ciborium::Value = ciborium::from_reader(&bytes[..]).unwrap();
    if std::env::var("BF_KEYS").is_ok() {
        // the key order of the first competition map of the old payload that has `semi_final_1`
        if let Some(cs) = old_value.as_map().and_then(|m| m.iter().find(|(k, _)| k.as_text() == Some("competitions"))).and_then(|(_, v)| v.as_array()) {
            if let Some(c) = cs.first() {
                let keys: Vec<_> = c.as_map().unwrap().iter().filter_map(|(k, _)| k.as_text()).collect();
                println!("old: {keys:?}");
            }
        }
        if let Some(cs) = new_value.as_map().and_then(|m| m.iter().find(|(k, _)| k.as_text() == Some("competitions"))).and_then(|(_, v)| v.as_array()) {
            if let Some(c) = cs.first() {
                let keys: Vec<_> = c.as_map().unwrap().iter().filter_map(|(k, _)| k.as_text()).collect();
                println!("new: {keys:?}");
            }
        }
        let g = |v: &ciborium::Value| -> Vec<String> { v.as_map().and_then(|m| m.iter().find(|(k, _)| k.as_text() == Some("game"))).and_then(|(_, v)| v.as_map()).map(|m| m.iter().filter_map(|(k, _)| k.as_text().map(str::to_owned)).collect()).unwrap_or_default() };
        println!("old game: {:?}", g(&old_value));
        for (name, v) in [("old", &old_value), ("new", &new_value)] {
            let mut seen = std::collections::BTreeSet::new();
            if let Some(cs) = v.as_map().and_then(|m| m.iter().find(|(k, _)| k.as_text() == Some("competitions"))).and_then(|(_, v)| v.as_array()) {
                for c in cs {
                    let m = c.as_map().unwrap();
                    let get = |n: &str| m.iter().find(|(k, _)| k.as_text() == Some(n)).map(|(_, v)| v.clone());
                    let class = get("class").and_then(|v| v.as_text().map(str::to_owned)).unwrap_or_default();
                    let kind = get("kind").and_then(|v| v.as_integer()).map(i128::from).unwrap_or(-1);
                    let sub = get("subkind").and_then(|v| v.as_integer()).map(i128::from).unwrap_or(-1);
                    seen.insert(format!("{class} {kind}/{sub}"));
                }
            }
            println!("{name} classes: {seen:?}");
        }
        println!("new game: {:?}", g(&new_value));
        return;
    }
    let (mut a, mut b, mut ta, mut tb) = Default::default();
    collect(&old_value, "", &mut a, &mut ta);
    collect(&new_value, "", &mut b, &mut tb);
    let paths: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    for p in paths {
        let empty = BTreeSet::new();
        let (x, y) = (a.get(p).unwrap_or(&empty), b.get(p).unwrap_or(&empty));
        let gone: Vec<_> = x.difference(y).collect();
        let new: Vec<_> = y.difference(x).collect();
        if (!gone.is_empty() || !new.is_empty()) && a.contains_key(p) && b.contains_key(p) {
            println!("{p}\n  old only: {gone:?}\n  new only: {new:?}");
        }
    }
    for (p, classes) in &ta {
        let now = tb.get(p).cloned().unwrap_or_default();
        let gone: Vec<_> = classes.difference(&now).collect();
        if !gone.is_empty() {
            println!("classes at {p} no longer written: {gone:?}");
        }
    }
}
