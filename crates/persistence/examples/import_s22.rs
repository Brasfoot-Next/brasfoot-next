//! Imports an original `.s22` and writes it as `.bfn`, printing what it found.
//! Usage: cargo run --release -p brasfoot-persistence --example import_s22 <in.s22> <out.bfn>

use brasfoot_persistence::{import, native};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let started = std::time::Instant::now();
    let world = import::import_s22(std::fs::read(&args[1])?)?;
    let imported = started.elapsed();
    println!(
        "players {} clubs {} coaches {} stadiums {} matches {} competitions {} stages {} countries {} states {} days {}",
        world.players.len(),
        world.clubs.len(),
        world.coaches.len(),
        world.stadiums.len(),
        world.matches.len(),
        world.competitions.len(),
        world.stages.len(),
        world.countries.len(),
        world.state_leagues.len(),
        world.game.days.len()
    );
    let human = world.clubs.iter().find(|c| c.human);
    let meta = native::SaveMeta {
        club: human.map(|c| c.name.clone()).unwrap_or_default(),
        season: world.game.start_year + world.game.season - 1,
        imported_from: std::path::Path::new(&args[1]).file_name().map(|n| n.to_string_lossy().into_owned()),
        ..Default::default()
    };
    let path = std::path::Path::new(&args[2]);
    native::write_save(path, &meta, &world)?;
    let written = started.elapsed();
    let (_, back): (_, brasfoot_model::world::World) = native::read_save(path)?;
    println!(
        "import {imported:?}, write {:?}, read back {:?}; {} bytes; players equal: {}",
        written - imported,
        started.elapsed() - written,
        std::fs::metadata(path)?.len(),
        back.players.len() == world.players.len()
    );
    Ok(())
}
