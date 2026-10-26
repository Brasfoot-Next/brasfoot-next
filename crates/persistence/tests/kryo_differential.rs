//! Differential test of the Kryo reader against the game itself. `fixtures/fictional.s22` is a
//! made-up save written by the game's own classes and Kryo (`tools/exporter/export.sh fixture`,
//! `exporter.FictionalFixture`), and `fixtures/fictional.canon.txt.zst` is its canonical dump made
//! by the game's Kryo (`exporter.CanonicalDump`). The Rust reader must produce the same dump.

use std::path::Path;

use brasfoot_persistence::kryo;

#[test]
fn rust_reader_matches_the_game_on_the_fictional_save() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let expected = zstd::decode_all(std::fs::File::open(fixtures.join("fictional.canon.txt.zst")).unwrap()).unwrap();
    let graph = kryo::read_graph(std::fs::read(fixtures.join("fictional.s22")).unwrap()).unwrap();
    assert_eq!(graph.roots.len(), 2);
    let actual = kryo::with_deep_stack(move || {
        let mut out = Vec::new();
        kryo::write_canonical(&graph, &mut out).unwrap();
        out
    });
    if actual != expected {
        let line = actual.split(|&b| b == b'\n').zip(expected.split(|&b| b == b'\n')).position(|(a, e)| a != e);
        panic!("canonical dumps differ (first different line: {line:?})");
    }
}
