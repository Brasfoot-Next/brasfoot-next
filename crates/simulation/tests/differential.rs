//! Differential test of the simulation against the game. `tools/exporter/export.sh sim <days>`
//! creates a fictional career with the game's own new-game code (`fixtures/sim-start.s22`), plays
//! `<days>` calls of `c.a.SR.V()` headless with every `new Random()` seeded from a counter and saves
//! the result (`fixtures/sim-after-<days>.s22`). This test plays the same days from the first save
//! and compares its world with the second through the canonical text of `brasfoot_model::canonical`.

use std::path::Path;

use brasfoot_engine::hooks::NoSystems;
use brasfoot_model::canonical::canonical;
use brasfoot_model::clock::Clock;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_simulation::advance;

const FIRST_SEED: i64 = 1000;

fn load(name: &str) -> brasfoot_model::world::World {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap()
}

/// The first differing line with the field path that leads to it.
fn first_difference(actual: &str, expected: &str) -> Option<String> {
    let a: Vec<&str> = actual.lines().collect();
    let e: Vec<&str> = expected.lines().collect();
    let i = a.iter().zip(&e).position(|(x, y)| x != y).or_else(|| (a.len() != e.len()).then(|| a.len().min(e.len())))?;
    let path: Vec<&str> = e[..i].iter().rev().filter(|l| l.starts_with('.')).take(6).copied().collect();
    let show = |lines: &[&str]| lines[i.saturating_sub(4)..(i + 4).min(lines.len())].join("\n");
    Some(format!("line {}, fields {:?}\n--- Rust\n{}\n--- Java\n{}", i + 1, path, show(&a), show(&e)))
}

fn compare(days: u32) {
    let mut world = load("sim-start.s22");
    let mut runtime = Runtime::new(RandomSource::deterministic(FIRST_SEED));
    // the harness's GregorianCalendar.bfSetNow
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    let mut hooks = NoSystems::default();
    // the harness turns the screens off (`bs`) after loading, before the first day
    world.game.interactive = false;
    for _ in 0..days {
        advance(&mut world, &mut runtime, &mut hooks);
    }
    // the harness saves the result and the exporter loads it (g.b.Ai, g.b.Aj)
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    let expected = canonical(&load(&format!("sim-after-{days}.s22")));
    // BF_DUMP=<dir> writes both canonical texts there, to inspect a difference.
    if let Ok(dir) = std::env::var("BF_DUMP") {
        std::fs::write(Path::new(&dir).join("rust.txt"), canonical(&world)).unwrap();
        std::fs::write(Path::new(&dir).join("java.txt"), &expected).unwrap();
        std::fs::write(Path::new(&dir).join("start.txt"), canonical(&load("sim-start.s22"))).unwrap();
    }
    if let Some(difference) = first_difference(&canonical(&world), &expected) {
        panic!("after {days} days: {difference}");
    }
}

#[test]
fn one_day_matches_the_game() {
    compare(1);
}

#[test]
fn ten_days_match_the_game() {
    compare(10);
}

/// The last match day of the first season, with `f.al()`.
#[test]
fn first_season_matches_the_game() {
    compare(48);
}

