//! Differential test of the simulation with the mod's systems, against the game. The fixtures
//! are those of `crates/simulation/tests` (`tools/exporter/export.sh sim <days>`): a fictional
//! career played headless with `bfmod.config` turning every system off but the tactics and
//! with the mod's market off. Some parts of the mod run anyway (the economy the game's market
//! asks), so the days from the second season on are checked here.

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
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../simulation/tests/fixtures").join(name);
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
    let mut world = load("sim-start.s22");
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    // the game's name lists (game data, read-only)
    runtime.names = NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let mut systems = ModSystems::new(ModConfig::parse(SYSTEMS_OFF));
    world.game.interactive = false;
    for _ in 0..days {
        advance(&mut world, &mut runtime, &mut systems);
    }
    // the harness saves the result and the exporter loads it (g.b.Ai, g.b.Aj)
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    let expected = canonical(&load(&format!("sim-after-{days}.s22")));
    if let Ok(dir) = std::env::var("BF_DUMP") {
        std::fs::write(Path::new(&dir).join("rust.txt"), canonical(&world)).unwrap();
        std::fs::write(Path::new(&dir).join("java.txt"), &expected).unwrap();
    }
    if let Some(difference) = first_difference(&canonical(&world), &expected) {
        panic!("after {days} days: {difference}");
    }
}

/// `f.aa()`, the new season, and its first match day.
#[test]
fn second_season_starts_like_the_game() {
    compare(49);
}

/// The whole second season, to its last match day and `f.al()`.
#[test]
fn second_season_matches_the_game() {
    compare(97);
}

/// Five seasons, with the yearly turnover of players, coaches and leagues. A transfer of a
/// player who retired shows, after the save and load, whoever took his save number.
#[test]
fn five_seasons_match_the_game() {
    compare(242);
}
