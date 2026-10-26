//! Writes the canonical dump of a `.s22`, to compare with `tools/exporter/export.sh canon`.
//! Usage: cargo run --release -p brasfoot-persistence --example canonical_dump <file.s22> <out.txt>

use std::io::BufWriter;

use brasfoot_persistence::kryo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1])?;
    let started = std::time::Instant::now();
    let graph = kryo::read_graph(bytes)?;
    eprintln!("read {} nodes, {} strings in {:?}", graph.nodes.len(), graph.strings.len(), started.elapsed());
    let out = args[2].clone();
    kryo::with_deep_stack(move || -> std::io::Result<()> {
        let mut writer = BufWriter::with_capacity(1 << 20, std::fs::File::create(out)?);
        kryo::write_canonical(&graph, &mut writer)
    })?;
    Ok(())
}
