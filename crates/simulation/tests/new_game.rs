//! Differential test of the new game against the game's own (`best.p.dg()`): the fictional
//! career of `tools/exporter/export.sh sim` (exporter.SimHarness.newGame) is made from made-up
//! team files, written to `fixtures/sim-teams` in the order the harness made them, and saved as
//! `sim-start.s22`. The Rust side reads the same files and makes the same choices.

use std::path::Path;

use brasfoot_model::canonical::canonical;
use brasfoot_model::clock::Clock;
use brasfoot_model::names::NameLists;
use brasfoot_model::options::GameOptions;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;
use brasfoot_simulation::new_game::NewGame;

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn first_difference(actual: &str, expected: &str) -> Option<String> {
    let a: Vec<&str> = actual.lines().collect();
    let e: Vec<&str> = expected.lines().collect();
    let i = a.iter().zip(&e).position(|(x, y)| x != y).or_else(|| (a.len() != e.len()).then(|| a.len().min(e.len())))?;
    let path: Vec<&str> = e[..i].iter().rev().filter(|l| l.starts_with('.')).take(6).copied().collect();
    let show = |lines: &[&str]| lines[i.saturating_sub(4)..(i + 4).min(lines.len())].join("\n");
    Some(format!("line {}, fields {:?}\n--- Rust\n{}\n--- Java\n{}", i + 1, path, show(&a), show(&e)))
}

/// The new game of the harness, as the Rust side makes it.
fn new_world() -> (World, Runtime) {
    // the made-up teams, in the order the harness made them
    let mut files: Vec<_> = std::fs::read_dir(fixtures().join("sim-teams")).unwrap().flatten().map(|e| e.path()).collect();
    files.sort();
    let templates: Vec<_> = files.iter().map(|f| brasfoot_persistence::game_data::read_team(&std::fs::read(f).unwrap()).unwrap()).collect();
    let mut new_game = NewGame::new(templates.clone(), Vec::new());
    for league in &mut new_game.leagues {
        league.selected = true;
    }
    // SimHarness.newGame: the default options, then four of them off for the career
    let options = GameOptions::default();
    // the harness's counter starts at 1 with aq.tI; Taticas.SORTE was made before it
    let mut rng = RandomSource::deterministic(1);
    let shared = rng.new_random();
    let mut runtime = Runtime { rng, shared_random: Some(shared), ..Default::default() };
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.names = NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let mut world = World::default();
    let mut hooks = brasfoot_engine::hooks::NoSystems::default();
    let mut sim = Sim { world: &mut world, runtime: &mut runtime, hooks: &mut hooks, human: None };
    // SimHarness.player(): the harness drew each made-up player's traits (F.at(int)) before
    for t in &templates {
        for p in t.players.iter().chain(&t.youth) {
            sim.draw_traits(p.position);
        }
    }
    sim.start_new_world(&options);
    let game = &mut sim.world.game;
    game.plays_state = false;
    game.plays_international_clubs = false;
    game.plays_all_national_teams = false;
    game.plays_regionals = false;
    sim.create_world(&new_game, &options, 2022);
    // p.dh() finds no Brazilian club; f.az()
    sim.world.game.plays_regionals = false;
    sim.find_next_day();
    (world, runtime)
}

fn compare(mut world: World, runtime: &Runtime, fixture: &str) {
    // the harness saves it and the exporter loads it
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    brasfoot_model::messages::fill_date_texts(&mut world, &runtime.clock);
    let java = brasfoot_persistence::import::import_s22(std::fs::read(fixtures().join(fixture)).unwrap()).unwrap();
    let expected = canonical(&java);
    if let Ok(dir) = std::env::var("BF_DUMP") {
        std::fs::write(Path::new(&dir).join("rust-new.txt"), canonical(&world)).unwrap();
        std::fs::write(Path::new(&dir).join("java-new.txt"), &expected).unwrap();
    }
    if let Some(difference) = first_difference(&canonical(&world), &expected) {
        panic!("{fixture}: {difference}");
    }
}

#[test]
fn new_game_matches_the_game() {
    let (world, runtime) = new_world();
    compare(world, &runtime, "sim-start.s22");
}

/// The coach screen (`a.bV`): a human coach takes the club with game id 1.
#[test]
fn human_coach_of_the_new_game_matches_the_game() {
    let (mut world, mut runtime) = new_world();
    let club = world.club_by_game_id(1).unwrap();
    let mut hooks = brasfoot_engine::hooks::NoSystems::default();
    Sim { world: &mut world, runtime: &mut runtime, hooks: &mut hooks, human: None }.hire_new_human_coach(club, "Técnico Fictício", 72);
    compare(world, &runtime, "coach-screen-start.s22");
}
