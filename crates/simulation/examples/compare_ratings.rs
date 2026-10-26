//! Debug aid for the simulation differential: plays one day from `sim-start.s22` and lists the
//! players whose last rating differs from the game's `sim-after-1.s22`, matched by their place
//! in the game's player list.

use std::path::Path;

use brasfoot_engine::hooks::NoSystems;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;

fn load(name: &str) -> brasfoot_model::world::World {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap()
}

fn main() {
    let mut world = load("sim-start.s22");
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    brasfoot_simulation::advance(&mut world, &mut runtime, &mut NoSystems::default());
    let java = load("sim-after-1.s22");
    let start = load("sim-start.s22");
    let mut shown = 0;
    for (i, (&a, &b)) in world.game.players.iter().zip(&java.game.players).enumerate() {
        let (pa, pb) = (world.player(a), java.player(b));
        if pa.last_rating != pb.last_rating && shown < 30 {
            shown += 1;
            let ps = start.player(start.game.players[i]);
            println!(
                "{i} {} / {} rust {} java {} (list {:?}) slot {}/{} strength start {} rust {} java {} out-of-position {}",
                pa.name, pb.name, pa.last_rating, pb.last_rating, pb.ratings.iter().map(|r| r.rating).collect::<Vec<_>>(), pa.slot, pb.slot, ps.strength, pa.strength, pb.strength,
                brasfoot_engine::value::out_of_position(pa)
            );
        }
    }
}
