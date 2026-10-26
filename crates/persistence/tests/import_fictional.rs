//! Import of the fictional save (`fixtures/fictional.s22`, see kryo_differential.rs) into the
//! model: every shared object lands in its arena and every reference resolves inside it.

use std::path::Path;

use brasfoot_persistence::{import, kryo};

fn fixture() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fictional.s22")).unwrap()
}

#[test]
fn imports_every_object_into_its_arena() {
    let graph = kryo::read_graph(fixture()).unwrap();
    let count = |classes: &[&str]| graph.nodes.iter().filter(|n| classes.contains(&graph.class_name(n.class))).count();
    let world = import::import_graph(&graph).unwrap();
    assert_eq!(world.players.len(), count(&["best.F"]));
    assert_eq!(world.clubs.len(), count(&["best.ah"]));
    assert_eq!(world.coaches.len(), count(&["best.al"]));
    assert_eq!(world.matches.len(), count(&["best.I"]));
    assert_eq!(world.stages.len(), count(&["f.s", "f.z"]));
    assert_eq!(world.countries.len(), count(&["best.Z"]));
    assert_eq!(world.state_leagues.len(), count(&["best.w"]));
    assert!(world.players.len() > 10 && world.clubs.len() > 10 && world.competitions.len() >= 34);
}

#[test]
fn references_resolve_to_the_same_objects() {
    let world = import::import_s22(fixture()).unwrap();
    // every club's squad points at players that exist
    for club in &world.clubs {
        for &player in &club.squad {
            assert!(player.index() < world.players.len());
        }
    }
    // the fictional names survive the trip
    let names: Vec<&str> = world.players.iter().map(|p| p.name.as_str()).collect();
    assert!(names.iter().any(|n| n.contains("Fictício") || n.contains("Fulano") || n.contains("Imaginário") || n.contains("Testes")));
    // the native format keeps the imported world intact
    let dir = std::env::temp_dir().join(format!("bfn-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fictional.bfn");
    let meta = brasfoot_persistence::native::SaveMeta { club: "Esporte Clube Fictício".into(), ..Default::default() };
    brasfoot_persistence::native::write_save(&path, &meta, &world).unwrap();
    let (_, back): (_, brasfoot_model::world::World) = brasfoot_persistence::native::read_save(&path).unwrap();
    // NaN != NaN, so compare through the encoding
    let encode = |w: &brasfoot_model::world::World| {
        let mut bytes = Vec::new();
        ciborium::into_writer(w, &mut bytes).unwrap();
        bytes
    };
    assert!(encode(&back) == encode(&world));
    std::fs::remove_dir_all(&dir).unwrap();
}
