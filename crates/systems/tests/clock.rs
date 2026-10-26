//! Differential test of the mod's clock (`bfmod.BolaDeOuro.tick`) against the game:
//! `tools/exporter/export.sh clock <days>` plays the fictional career with the awards and one
//! tick of the mod each time the main screen would come back (after every day but a season's
//! last). With the market off and no human club, the clock is the economy (fortunes, sponsors,
//! owners, reputations, the cash written back) and the windows (their calendar, the bloated
//! squads trimmed). The world, `.economia` and `.janelas` are compared.

mod common;

use std::path::Path;

use brasfoot_i18n::Translator;
use brasfoot_model::canonical::canonical;
use brasfoot_model::clock::Clock;
use brasfoot_model::names::NameLists;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;
use brasfoot_simulation::advance;
use brasfoot_systems::ModSystems;
use brasfoot_systems::config::ModConfig;

const AWARDS_ON: &str = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=1\nPROPOSTAS=0\nJANELA_USUARIO=0\n\
EMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\n\
PATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

/// BF_FIXTURE_DIR: the harness's runs kept elsewhere, for the probe.
fn fixture(name: &str) -> std::path::PathBuf {
    let dir = std::env::var("BF_FIXTURE_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    dir.join(name)
}

fn load(path: &Path) -> World {
    // the long runs are kept compressed (name.zst)
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => zstd::decode_all(std::fs::File::open(format!("{}.zst", path.display())).unwrap()).unwrap(),
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
    run(days, "clock", "MERCADO=DESLIGADO");
}

fn run(days: u32, prefix: &str, market: &str) {
    run_with(days, prefix, market, &[]);
}

/// The harness's run with the switches of `BF_SYSTEMS` on.
fn run_with(days: u32, prefix: &str, market: &str, systems: &[&str]) {
    // the world career (BF_WORLD) starts from its own fixture
    let start = if prefix.starts_with("world-") { "world-sim-start.s22" } else { "sim-start.s22" };
    let mut world = load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../simulation/tests/fixtures").join(start));
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.names = NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let mut config = AWARDS_ON.replace("MERCADO=DESLIGADO", market);
    for k in systems {
        config = config.replace(&format!("\n{k}=0"), &format!("\n{k}=1"));
    }
    let mut systems = ModSystems::new(ModConfig::parse(&config));
    world.game.interactive = false;
    systems.main_screen_tick(&mut world, &mut runtime);
    for _ in 0..days {
        let over = world.game.season_over;
        advance(&mut world, &mut runtime, &mut systems);
        if over || !world.game.season_over {
            systems.main_screen_tick(&mut world, &mut runtime);
        }
    }
    let t = Translator::new(brasfoot_i18n::DEFAULT_LOCALE);
    let mut failures = Vec::new();
    let java = std::fs::read_to_string(fixture(&format!("{prefix}-after-{days}.economia"))).unwrap();
    let saved = systems.economy_saved.clone().expect("the economy ticked");
    let (actual, processed) = common::economy_file_of(&t, &world, &saved);
    let expected: String = java.lines().filter(|l| !l.starts_with("TRANSF\t") && !l.starts_with("JANELA\t")).map(|l| format!("{l}\n")).collect();
    if let Some(d) = first_difference(&actual, &expected) {
        failures.push(format!(".economia: {d}"));
    }
    let mut java_processed: Vec<String> = java.lines().filter_map(|l| l.strip_prefix("TRANSF\t")).map(str::to_owned).collect();
    java_processed.sort();
    if processed != java_processed {
        failures.push(format!(".economia transfers: {} in Rust, {} in Java", processed.len(), java_processed.len()));
    }
    let janelas = std::fs::read_to_string(fixture(&format!("{prefix}-after-{days}.janelas"))).unwrap();
    let mut notices: Vec<&str> = janelas.lines().filter_map(|l| l.strip_prefix("AVISO\t")).collect();
    notices.sort();
    let mut ours: Vec<&str> = world.mods.windows.notices.iter().map(String::as_str).collect();
    ours.sort();
    if ours != notices {
        failures.push(format!(".janelas notices: {ours:?} / {notices:?}"));
    }
    let deals: Vec<String> = janelas.lines().filter_map(|l| l.strip_prefix("NEGOCIO\t")).map(str::to_owned).collect();
    let ours: Vec<String> = world.mods.windows.deals.iter().map(|d| format!("{}\t{}\t{}\t{}", d.0, d.1, d.2, d.3)).collect();
    if ours != deals {
        failures.push(format!(".janelas deals: {ours:?} / {deals:?}"));
    }
    let assets = fixture(&format!("{prefix}-after-{days}.patrimonio"));
    if assets.exists()
        && let Some(d) = first_difference(&common::assets_file(&world), &std::fs::read_to_string(assets).unwrap())
    {
        failures.push(format!(".patrimonio: {d}"));
    }
    let stadiums = fixture(&format!("{prefix}-after-{days}.estadios"));
    if stadiums.exists()
        && let Some(d) = first_difference(&common::stadiums_file(&world), &std::fs::read_to_string(stadiums).unwrap())
    {
        failures.push(format!(".estadios: {d}"));
    }
    let base = fixture(&format!("{prefix}-after-{days}.base"));
    if base.exists()
        && let Some(d) = first_difference(&common::academy_file(&world), &std::fs::read_to_string(base).unwrap())
    {
        failures.push(format!(".base: {d}"));
    }
    let satellites = fixture(&format!("{prefix}-after-{days}.satelites"));
    if satellites.exists()
        && let Some(d) = first_difference(&common::satellites_file(&t, &world), &common::satellites_java(&std::fs::read_to_string(satellites).unwrap()))
    {
        failures.push(format!(".satelites: {d}"));
    }
    let credit = fixture(&format!("{prefix}-after-{days}.credito"));
    if credit.exists()
        && let Some(d) = first_difference(&common::credit_file(&world), &common::credit_java(&std::fs::read_to_string(credit).unwrap()))
    {
        failures.push(format!(".credito: {d}"));
    }
    let loans = fixture(&format!("{prefix}-after-{days}.emprestimos"));
    if loans.exists()
        && let Some(d) = first_difference(&common::loans_file(&world), &std::fs::read_to_string(loans).unwrap())
    {
        failures.push(format!(".emprestimos: {d}"));
    }
    let super_mundial = fixture(&format!("{prefix}-after-{days}.supermundial"));
    if super_mundial.exists() {
        let mut java: Vec<String> = std::fs::read_to_string(super_mundial).unwrap().lines().map(str::to_owned).collect();
        java.sort();
        let ours = common::super_mundial_file(&t, &world);
        if let Some(d) = first_difference(&(ours.join("\n") + "\n"), &(java.join("\n") + "\n")) {
            failures.push(format!(".supermundial: {d}"));
        }
    }
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    if let Some(d) = first_difference(&canonical(&world), &canonical(&load(&fixture(&format!("{prefix}-after-{days}.s22"))))) {
        failures.push(format!("world: {d}"));
    }
    assert!(failures.is_empty(), "after {days} days:\n{}", failures.join("\n\n"));
}

/// Three seasons: the economy's start, two new seasons with their sponsors, owners and
/// reputations, and six windows.
#[test]
fn three_seasons_of_the_mods_clock_match_the_game() {
    compare(145);
}

/// The same seasons with the market as the mod's default ("Realista"): the auctions of the
/// game's yearly market and the rich clubs' signings in the windows.
#[test]
fn three_seasons_with_the_market_on_match_the_game() {
    run(145, "clock-market", "MERCADO=REALISTA");
}

/// The market at its busiest ("Agitado"): more signings, squads over 28 trimmed in the windows.
#[test]
fn three_seasons_with_a_busy_market_match_the_game() {
    run(145, "clock-busy", "MERCADO=AGITADO");
}

/// The youth academy (`Base`): ceilings drawn for every kid, development by the calendar and by
/// minutes, the AI's promotions and the attributes carried along in individual-skills mode.
#[test]
fn the_youth_academys_first_season_matches_the_game() {
    run_with(49, "clock-base", "MERCADO=DESLIGADO", &["BASE"]);
}

#[test]
fn three_seasons_with_the_youth_academy_match_the_game() {
    run_with(145, "clock-base", "MERCADO=DESLIGADO", &["BASE"]);
}

/// The clubs' infrastructure (`Patrimonio`): the starting levels, the AI's investments, stadium
/// expansions and naming rights, the youth trials, the treated injuries and the effects on the
/// matches.
#[test]
fn three_seasons_with_the_clubs_infrastructure_match_the_game() {
    run_with(145, "clock-patrimonio", "MERCADO=DESLIGADO", &["PATRIMONIO"]);
}

/// Three seasons of `Estadios`: no legend retires so soon, but each AI club draws its chance at
/// every season end.
#[test]
fn three_seasons_with_the_stadiums_match_the_game() {
    run_with(145, "clock-estadios", "MERCADO=DESLIGADO", &["ESTADIOS"]);
}

/// Twelve seasons, long enough for retired legends: the AI clubs' stadiums named after them
/// (`Estadios`), which also starts `Patrimonio` with its switch off. Waits for the Super Mundial
/// of 2025, which needs the game's group stages.
#[test]
#[ignore]
fn twelve_seasons_with_stadiums_named_after_legends_match_the_game() {
    run_with(577, "clock-estadios", "MERCADO=DESLIGADO", &["ESTADIOS"]);
}

/// A world career (`BF_WORLD`) with the mod's clock through the Super Mundial of 2025: the
/// coefficient of each season, the host and the 32 clubs, the draw on the game's friendly
/// tournament, its matches, the prize money and the edition's names (`SuperMundial`).
#[test]
fn a_world_career_through_the_super_mundial_matches_the_game() {
    run(600, "world-clock", "MERCADO=DESLIGADO");
}

/// The AI's loans (`Emprestimos`): the big clubs lend benched youngsters in the windows, the
/// busy market lends the surplus ones, the game brings them back after a year.
#[test]
fn three_seasons_with_the_ais_loans_match_the_game() {
    run_with(145, "clock-emprestimos_ia-market", "MERCADO=AGITADO", &["EMPRESTIMOS_IA"]);
}

/// The multi-club networks (`Satelites`): the AI's first networks and their seasons.
#[test]
fn three_seasons_with_the_satellites_match_the_game() {
    run_with(145, "clock-satelites-market", "MERCADO=AGITADO", &["SATELITES"]);
}

/// The bank (`Credito`): the AI's loans in the windows and in distress.
#[test]
fn three_seasons_with_the_bank_match_the_game() {
    run_with(145, "clock-credito-market", "MERCADO=AGITADO", &["CREDITO"]);
}

/// The loans, the networks and the bank together.
#[test]
fn three_seasons_with_loans_networks_and_the_bank_match_the_game() {
    run_with(145, "clock-emprestimos_ia-satelites-credito-market", "MERCADO=AGITADO", &["EMPRESTIMOS_IA", "SATELITES", "CREDITO"]);
}

/// `BF_PROBE=60,100 BF_PROBE_SYSTEMS=SATELITES,CREDITO BF_PROBE_MARKET=AGITADO BF_FIXTURE_DIR=...
/// cargo test --release -p brasfoot-systems --test clock probe -- --ignored`: the harness's
/// clock runs of those days.
#[test]
#[ignore]
fn probe() {
    let systems: Vec<String> = std::env::var("BF_PROBE_SYSTEMS").map(|s| s.split(',').map(str::to_owned).collect()).unwrap_or_default();
    let market = std::env::var("BF_PROBE_MARKET").ok();
    let mut prefix = String::from("clock-");
    for s in &systems {
        prefix += &format!("{}-", s.to_lowercase());
    }
    let prefix = if market.is_some() { format!("{prefix}market") } else { prefix.trim_end_matches('-').to_owned() };
    let market_line = format!("MERCADO={}", market.as_deref().unwrap_or("DESLIGADO"));
    let refs: Vec<&str> = systems.iter().map(String::as_str).collect();
    for d in std::env::var("BF_PROBE").unwrap().split(',').map(|d| d.parse::<u32>().unwrap()) {
        let r = std::panic::catch_unwind(|| run_with(d, &prefix, &market_line, &refs));
        println!("{d}: {}", if r.is_ok() { "ok" } else { "DIFF" });
    }
}
