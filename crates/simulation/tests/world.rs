//! Differential test of the simulation in a career with the international club cups and every
//! national team (`BF_WORLD=1 tools/exporter/export.sh sim <days>`): ten made-up leagues in five
//! confederations, the continental cups, the world club cups, the national teams' tournaments,
//! qualifiers and Nations League, against the game. Without the mod's systems this holds in the
//! first season; the seasons after are checked in `crates/systems/tests/world.rs`.

use std::path::Path;

use brasfoot_engine::hooks::NoSystems;
use brasfoot_model::canonical::canonical;
use brasfoot_model::clock::Clock;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_simulation::advance;

const FIRST_SEED: i64 = 1000;

fn load(name: &str) -> brasfoot_model::world::World {
    // BF_FIXTURE_DIR: the longer runs, kept out of the repository
    let dir = std::env::var("BF_FIXTURE_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    let path = dir.join(name);
    brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap()
}

fn first_difference(actual: &str, expected: &str) -> Option<String> {
    let a: Vec<&str> = actual.lines().collect();
    let e: Vec<&str> = expected.lines().collect();
    let i = a.iter().zip(&e).position(|(x, y)| x != y).or_else(|| (a.len() != e.len()).then(|| a.len().min(e.len())))?;
    let path: Vec<&str> = e[..i].iter().rev().filter(|l| l.starts_with('.')).take(6).copied().collect();
    let show = |lines: &[&str]| lines[i.saturating_sub(4)..(i + 4).min(lines.len())].join("\n");
    Some(format!("line {}, fields {:?}\n--- Rust\n{}\n--- Java\n{}", i + 1, path, show(&a), show(&e)))
}

fn compare(days: u32) {
    let mut world = load("world-sim-start.s22");
    let mut runtime = Runtime::new(RandomSource::deterministic(FIRST_SEED));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.trace_seeds = std::env::var("BF_TRACE_RNG").is_ok();
    runtime.names = brasfoot_model::names::NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let mut hooks = NoSystems::default();
    world.game.interactive = false;
    runtime.checkpoint("start");
    for day in 0..days {
        runtime.checkpoint(&format!("day {day}"));
        advance(&mut world, &mut runtime, &mut hooks);
    }
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    let expected = canonical(&load(&format!("world-sim-after-{days}.s22")));
    if let Ok(dir) = std::env::var("BF_DUMP") {
        std::fs::write(Path::new(&dir).join(format!("rust-{days}.txt")), canonical(&world)).unwrap();
        std::fs::write(Path::new(&dir).join(format!("java-{days}.txt")), &expected).unwrap();
    }
    if let Some(difference) = first_difference(&canonical(&world), &expected) {
        panic!("after {days} days: {difference}");
    }
}

#[test]
fn first_day_of_a_world_career_matches_the_game() {
    compare(1);
}

#[test]
#[ignore]
fn probe() {
    for d in std::env::var("BF_PROBE").unwrap().split(',').map(|d| d.parse::<u32>().unwrap()) {
        let r = std::panic::catch_unwind(|| compare(d));
        println!("{d}: {}", if r.is_ok() { "ok" } else { "DIFF" });
    }
}
