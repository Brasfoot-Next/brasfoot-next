//! Differential test of a world career (`BF_WORLD=1 tools/exporter/export.sh sim <days>`: the
//! international club cups and every national team) with the mod's systems, against the game.
//! The fixtures are those of `crates/simulation/tests` (or `BF_FIXTURE_DIR` for longer runs); as
//! in `simulation.rs`, the economy the game's yearly market asks runs anyway, so the days from
//! the second season on are checked here.

use std::path::Path;

use brasfoot_model::canonical::canonical;
use brasfoot_model::clock::Clock;
use brasfoot_model::names::NameLists;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_simulation::advance;
use brasfoot_systems::ModSystems;
use brasfoot_systems::config::ModConfig;

/// The `bfmod.config` of the harness (`SimHarness.SYSTEMS_OFF`).
const SYSTEMS_OFF: &str = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=0\nPROPOSTAS=0\nJANELA_USUARIO=0\n\
EMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\n\
PATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

fn load(name: &str) -> brasfoot_model::world::World {
    let dir = std::env::var("BF_FIXTURE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../simulation/tests/fixtures"));
    // the long runs are kept compressed (name.zst)
    let bytes = match std::fs::read(dir.join(name)) {
        Ok(bytes) => bytes,
        Err(_) => zstd::decode_all(std::fs::File::open(dir.join(format!("{name}.zst"))).unwrap()).unwrap(),
    };
    brasfoot_persistence::import::import_s22(bytes).unwrap()
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
    // BF_FIXTURE_PREFIX: another career of the harness (BF_COUNTRIES), e.g. "leagues-"
    let prefix = std::env::var("BF_FIXTURE_PREFIX").unwrap_or_else(|_| "world-".into());
    let mut world = load(&format!("{prefix}sim-start.s22"));
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.trace_seeds = std::env::var("BF_TRACE_RNG").is_ok();
    // the game's name lists (game data, read-only)
    runtime.names = NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let mut systems = ModSystems::new(ModConfig::parse(SYSTEMS_OFF));
    world.game.interactive = false;
    runtime.checkpoint("start");
    for day in 0..days {
        runtime.checkpoint(&format!("day {day}"));
        advance(&mut world, &mut runtime, &mut systems);
    }
    runtime.checkpoint("end");
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    let expected = canonical(&load(&format!("{prefix}sim-after-{days}.s22")));
    if let Ok(dir) = std::env::var("BF_DUMP") {
        std::fs::write(Path::new(&dir).join(format!("rust-{days}.txt")), canonical(&world)).unwrap();
        std::fs::write(Path::new(&dir).join(format!("java-{days}.txt")), &expected).unwrap();
    }
    if let Some(difference) = first_difference(&canonical(&world), &expected) {
        panic!("after {days} days: {difference}");
    }
}

/// Three seasons: two new seasons (`f.aa()`) with the continental cups' draws, the Euro's
/// qualifiers, the continental cups of national teams and the Nations Leagues, the Euro and the
/// Copa América of the third.
#[test]
fn a_world_career_of_three_seasons_matches_the_game() {
    compare(365);
}

/// `BF_PROBE=30,60 cargo test --release -p brasfoot-systems --test world probe -- --ignored`:
/// each listed day against `world-sim-after-<day>.s22`.
#[test]
#[ignore]
fn probe() {
    for d in std::env::var("BF_PROBE").unwrap().split(',').map(|d| d.parse::<u32>().unwrap()) {
        let r = std::panic::catch_unwind(|| compare(d));
        println!("{d}: {}", if r.is_ok() { "ok" } else { "DIFF" });
    }
}
