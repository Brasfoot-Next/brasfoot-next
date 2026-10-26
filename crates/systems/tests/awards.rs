//! Differential test of the season-end awards (`bfmod.BolaDeOuro.fimDeTemporada`), against the
//! game. The fixtures come from `tools/exporter/export.sh awards <days>`: the fictional career of
//! the simulation tests with the awards' effects on, the mod's season end called where the game
//! opens its season-end screen. Both the world and the side files the awards write are compared;
//! the side files are written here from the Rust state the way the mod writes them, with the
//! labels from the catalog.

mod common;

use common::java_double;
use std::path::Path;

use brasfoot_i18n::Translator;
use brasfoot_i18n::labels;
use brasfoot_model::canonical::canonical;
use brasfoot_model::clock::Clock;
use brasfoot_model::mods::{AwardExtra, AwardKind, AwardsState, RecordKind};
use brasfoot_model::names::NameLists;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;
use brasfoot_simulation::advance;
use brasfoot_systems::ModSystems;
use brasfoot_systems::config::ModConfig;

/// `SimHarness.AWARDS_ON`: the game alone but the tactics and the awards' effects.
const AWARDS_ON: &str = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=1\nPROPOSTAS=0\nJANELA_USUARIO=0\n\
EMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\n\
PATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load(path: &Path) -> World {
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

/// `Premios.limpo(String)`
fn clean(s: &str) -> String {
    s.replace(['\t', '\n'], " ")
}

fn club_name(club: &Option<String>) -> String {
    clean(club.as_deref().unwrap_or("-"))
}

fn kind_name(kind: AwardKind) -> &'static str {
    match kind {
        AwardKind::BallonDor => "BOLA",
        AwardKind::GoldenShoe => "CHUTEIRA",
        AwardKind::GoldenGlove => "LUVA",
        AwardKind::GoldenBoy => "GOLDEN_BOY",
        AwardKind::TeamOfTheYear => "SELECAO",
        AwardKind::CoachOfTheYear => "TECNICO",
    }
}

/// `Premios.escrever`
fn awards_file(t: &Translator, state: &AwardsState) -> String {
    let mut out = String::new();
    for a in &state.awards {
        let extra = match &a.extra {
            AwardExtra::None => String::new(),
            AwardExtra::WeightedGoals(g) => format!("{g:.1}"),
            AwardExtra::Slot(s) => labels::team_slot(t, *s),
            AwardExtra::Coach(summary) => labels::coach_summary(t, summary),
        };
        out += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t\n",
            a.year,
            kind_name(a.kind),
            clean(&a.key),
            clean(&a.name),
            club_name(&a.club),
            a.country,
            a.position,
            a.age,
            java_double(a.rating),
            a.goals,
            a.assists,
            clean(&extra)
        );
    }
    out
}

/// `HallDaFama.escreverCarreiras`
fn careers_file(state: &AwardsState) -> String {
    let mut out = String::new();
    for c in &state.careers {
        out += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            clean(&c.key),
            clean(&c.name),
            c.country,
            c.position,
            c.games,
            c.goals,
            c.assists,
            java_double(c.rating_sum),
            c.ratings,
            club_name(&c.club),
            c.age,
            c.last_year,
            c.retired
        );
    }
    out
}

/// `HallDaFama.escreverRecordes`
fn records_file(state: &AwardsState) -> String {
    let mut out = String::new();
    for m in &state.records {
        let kind = match m.kind {
            RecordKind::Goals => "GOLS",
            RecordKind::Assists => "ASSISTENCIAS",
            RecordKind::Contributions => "PARTICIPACOES",
            RecordKind::Rating => "NOTA",
        };
        out += &format!("{kind}\t{}\t{}\t{}\t{}\t{}\t{}\n", java_double(m.value), clean(&m.key), clean(&m.name), club_name(&m.club), m.country, m.year);
    }
    out
}

/// `Clubes.escrever`
fn clubs_file(t: &Translator, state: &AwardsState) -> String {
    let mut out = String::new();
    for l in &state.club_seasons {
        let scorer = l.top_scorer.as_ref().map(|(name, goals)| labels::club_top_scorer(t, name, *goals)).unwrap_or_default();
        out += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            l.year,
            l.club,
            clean(&l.name),
            l.country,
            clean(&l.league.as_ref().map(|k| labels::league(t, k)).unwrap_or_default()),
            l.division,
            l.position,
            l.clubs,
            clean(&labels::season_titles(t, &l.titles)),
            clean(&scorer),
            clean(l.coach.as_deref().unwrap_or(""))
        );
    }
    out
}

/// `Clubes.escreverIdolos`
fn idols_file(state: &AwardsState) -> String {
    let mut out = String::new();
    for i in &state.idols {
        out += &format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\n", i.club, clean(&i.key), clean(&i.name), i.country, i.position, i.games, i.goals);
    }
    out
}

fn compare(days: u32) {
    let start = Path::new(env!("CARGO_MANIFEST_DIR")).join("../simulation/tests/fixtures/sim-start.s22");
    let mut world = load(&start);
    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    // the game's name lists (game data, read-only)
    runtime.names = NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let mut systems = ModSystems::new(ModConfig::parse(AWARDS_ON));
    world.game.interactive = false;
    for _ in 0..days {
        advance(&mut world, &mut runtime, &mut systems);
    }
    let t = Translator::new(brasfoot_i18n::DEFAULT_LOCALE);
    let state = &world.mods.awards;
    let files = [
        ("premios", awards_file(&t, state)),
        ("carreiras", careers_file(state)),
        ("recordes", records_file(state)),
        ("clubes", clubs_file(&t, state)),
        ("idolos", idols_file(state)),
    ];
    let mut failures = Vec::new();
    for (ext, actual) in files {
        let expected = std::fs::read_to_string(fixtures().join(format!("awards-after-{days}.{ext}"))).unwrap_or_default();
        if let Some(d) = first_difference(&actual, &expected) {
            failures.push(format!(".{ext} after {days} days: {d}"));
        }
    }
    // the harness saves the result and the exporter loads it (g.b.Ai, g.b.Aj)
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    let expected = canonical(&load(&fixtures().join(format!("awards-after-{days}.s22"))));
    if let Some(d) = first_difference(&canonical(&world), &expected) {
        failures.push(format!("world after {days} days: {d}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// The first season's awards, then the first day of the second season.
#[test]
fn first_season_awards_match_the_game() {
    compare(49);
}

/// Two seasons: the effects of the first awards play along the second.
#[test]
fn second_season_awards_match_the_game() {
    compare(97);
}

/// Five seasons, with retirements, records kept from seasons whose players are gone and the
/// idols merged across seasons.
#[test]
fn five_seasons_of_awards_match_the_game() {
    compare(242);
}
