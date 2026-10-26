//! Upgrades of older `.bfn` payloads.

use std::path::Path;

use brasfoot_model::world::World;
use brasfoot_persistence::native::MIGRATIONS;
use ciborium::Value;

fn encode(w: &impl serde::Serialize) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::into_writer(w, &mut bytes).unwrap();
    bytes
}

fn remove(map: &mut Value, names: &[&str]) {
    if let Some(entries) = map.as_map_mut() {
        entries.retain(|(k, _)| !names.contains(&k.as_text().unwrap_or_default()));
    }
}

/// A version 1 save has no save numbers: it gets those its next save would give, and its
/// transfer history still points at the same players once relinked.
#[test]
fn version_1_gets_the_save_numbers() {
    // a fictional career after five seasons (see crates/simulation/tests)
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../simulation/tests/fixtures/sim-after-242.s22");
    let world = brasfoot_persistence::import::import_s22(std::fs::read(fixture).unwrap()).unwrap();
    let mut v1: Value = ciborium::from_reader(encode(&world).as_slice()).unwrap();
    let entries = v1.as_map_mut().unwrap();
    for (k, v) in entries.iter_mut() {
        match k.as_text() {
            Some("players") => v.as_array_mut().unwrap().iter_mut().for_each(|p| remove(p, &["save_number", "save_list"])),
            Some("game") => {
                let transfers = v.as_map_mut().unwrap().iter_mut().find(|(k, _)| k.as_text() == Some("transfers")).unwrap();
                transfers.1.as_array_mut().unwrap().iter_mut().for_each(|t| remove(t, &["player_number", "player_list"]));
            }
            _ => {}
        }
    }
    assert!(v1.deserialized::<World>().is_err());
    let migration = MIGRATIONS.iter().find(|m| m.from == 1).unwrap();
    (migration.apply)(&mut v1);
    let mut back: World = v1.deserialized().unwrap();
    let mut expected = world.clone();
    expected.number_players_for_save();
    expected.number_competitions_for_save();
    // a player in neither list kept the number of an older save, which version 1 did not keep
    let listed: Vec<_> = world.game.players.iter().chain(&world.game.youth_players).collect();
    for (i, p) in expected.players.iter_mut().enumerate() {
        if !listed.iter().any(|l| l.index() == i) {
            (p.save_number, p.save_list) = (-1, -1);
        }
    }
    assert!(encode(&back) == encode(&expected));
    back.relink_transfers();
    let players = |w: &World| w.game.transfers.iter().map(|t| t.player).collect::<Vec<_>>();
    assert_eq!(players(&back), players(&world));
}
