//! Debug aid: how many match days the simulation fixture plays before its season is over.

use std::path::Path;

use brasfoot_engine::hooks::NoSystems;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;

fn main() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sim-start.s22");
    let mut world = brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap();
    world.game.interactive = false;
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    let mut days = 0;
    while !world.game.season_over {
        brasfoot_simulation::advance(&mut world, &mut runtime, &mut NoSystems::default());
        days += 1;
    }
    println!("season over after {days} days");
}
