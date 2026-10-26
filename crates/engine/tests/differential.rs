//! Differential test of the match engine against the game. `tools/exporter/export.sh engine`
//! builds a fictional world with the game's classes (`fixtures/engine-world.s22`), plays a day of
//! AI matches with every `new Random()` seeded from a counter, and writes what happened
//! (`fixtures/engine-<tactics>.txt`). This test plays the same day from the same seeds and must
//! write the same text.

use std::fmt::Write as _;
use std::path::Path;

use brasfoot_engine::clubs;
use brasfoot_engine::hooks::NoSystems;
use brasfoot_engine::lineup::pick_ai_lineup;
use brasfoot_engine::simulate::{Play, prepare_squads};
use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;

const FIRST_SEED: i64 = 1000;

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn club_name(world: &World, c: Option<ClubId>) -> String {
    c.map(|c| world.club(c).name.clone()).unwrap_or_else(|| "-".into())
}

fn player_name(world: &World, p: Option<PlayerId>) -> String {
    p.map(|p| world.player(p).name.clone()).unwrap_or_else(|| "-".into())
}

fn ints(values: &[i32]) -> String {
    values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")
}

fn names(world: &World, list: &[PlayerId]) -> String {
    list.iter().map(|&p| format!(" {}#{}", world.player(p).name, world.player(p).slot)).collect()
}

fn bits(d: f64) -> String {
    format!("{:x}", d.to_bits())
}

fn dump_match(out: &mut String, world: &World, index: usize, m: MatchId) {
    let g = world.game_match(m);
    let competition = world.competition(g.competition.unwrap());
    writeln!(out, "M {index} {}|{} kind={}/{}", club_name(world, g.home), club_name(world, g.away), competition.kind, competition.subkind).unwrap();
    writeln!(
        out,
        "score {} {} added {} winner {} shootout {} flags {},{},{} gg {} gh {}",
        g.home_goals,
        g.away_goals,
        ints(&g.values_fq),
        club_name(world, g.winner),
        ints(&g.penalty_shootout),
        g.knockout,
        g.penalties_if_level,
        g.went_to_penalties,
        g.value_gg,
        g.value_gh
    )
    .unwrap();
    writeln!(
        out,
        "stats possession={} shots={} on_target={} wide={} tackles={} pass_errors={} duels={} attendance={} income={} subs_left={} incidents={},{},{}",
        ints(&g.possession),
        ints(&g.shots),
        ints(&g.shots_on_target),
        ints(&g.values_ga),
        ints(&g.tackles),
        ints(&g.values_gc),
        ints(&g.values_fx),
        ints(&g.attendance),
        g.income,
        ints(&g.substitutions_left),
        g.value_fs,
        g.value_ft,
        g.value_fu
    )
    .unwrap();
    let tactics = |c: Option<ClubId>| ints(&world.club(c.unwrap()).tactics);
    writeln!(out, "tactics home={} away={}", tactics(g.home), tactics(g.away)).unwrap();
    for e in &g.events {
        writeln!(
            out,
            "E {} {} {} {} side={} {} {} {} flag={}",
            e.kind,
            e.detail,
            e.minute,
            e.half,
            e.value_df,
            player_name(world, e.player),
            player_name(world, e.other_player),
            club_name(world, e.club),
            e.flag_dg
        )
        .unwrap();
    }
    for (label, list) in [
        ("home_starters", &g.home_starters),
        ("away_starters", &g.away_starters),
        ("home_bench", &g.home_bench),
        ("away_bench", &g.away_bench),
        ("home_played", &g.home_played),
        ("away_played", &g.away_played),
        ("home_came_on", &g.home_list_fl),
        ("away_came_on", &g.away_list_fm),
    ] {
        writeln!(out, "L {label}{}", names(world, list)).unwrap();
    }
}

fn dump_players(out: &mut String, world: &mut World, runtime: &Runtime) {
    for club in world.game.clubs.clone() {
        let captain = clubs::captain(world, club);
        let penalties = clubs::penalty_taker(world, club);
        let free_kicks = clubs::free_kick_taker(world, club);
        let c = world.club(club);
        writeln!(
            out,
            "C {} captain={} penalties={} free_kicks={} lineup_set={} starters{} bench{}",
            c.name,
            player_name(world, captain),
            player_name(world, penalties),
            player_name(world, free_kicks),
            c.lineup_set,
            names(world, &c.last_starters),
            names(world, &c.last_bench)
        )
        .unwrap();
        for &p in &world.club(club).squad {
            let player = world.player(p);
            let s = runtime.match_stats.get(&p).cloned().unwrap_or_default();
            writeln!(
                out,
                "P {} slot={} fitness={} strength={} injured={} started={} cards={} match={},{},{},{},{},{},{},{},{},{},{}",
                player.name,
                player.slot,
                player.fitness,
                player.strength,
                player.injured_until,
                player.started_recently,
                runtime.match_cards.get(&p).copied().unwrap_or(0),
                s.shots,
                s.shots_saved,
                s.count_pf,
                s.count_pg,
                s.goals,
                s.own_goals,
                s.count_pi,
                s.yellow_cards,
                s.red_cards,
                s.assists,
                s.count_pl
            )
            .unwrap();
            for h in &player.competition_stats {
                let name = h.competition_id.map(|c| world.competition(c).name.clone()).unwrap_or_else(|| "-".into());
                writeln!(
                    out,
                    "  H {} {} {} {},{} {} {} {}",
                    name,
                    h.games,
                    h.goals,
                    h.yellow_cards,
                    h.red_cards,
                    h.assists,
                    h.rated_games,
                    bits(h.average_rating())
                )
                .unwrap();
            }
            for k in &player.season_stats {
                writeln!(
                    out,
                    "  K {} {} {} {} {} {} {} {} {} {}",
                    k.season,
                    k.club_id,
                    k.games,
                    k.goals,
                    k.yellow_cards,
                    k.red_cards,
                    k.assists,
                    k.injuries,
                    k.rated_games,
                    bits(k.rating_sum)
                )
                .unwrap();
            }
        }
    }
}

fn dump_tactics(out: &mut String, world: &World, engine: &brasfoot_engine::engine::MatchEngine) {
    use brasfoot_engine::tactics;
    let Some(p) = &engine.tactics else {
        writeln!(out, "T none").unwrap();
        return;
    };
    writeln!(
        out,
        "T meio={},{} fits={},{},{},{},{},{}",
        bits(p.midfield[0]),
        bits(p.midfield[1]),
        bits(p.fit_midfield[0]),
        bits(p.fit_midfield[1]),
        bits(p.fit_speed[0]),
        bits(p.fit_speed[1]),
        bits(p.fit_defending[0]),
        bits(p.fit_defending[1])
    )
    .unwrap();
    let mut w = String::from("W");
    let functions: [(&str, fn(&World, &tactics::TacticsMatch, usize) -> f64); 4] = [
        ("meio", tactics::midfield_weight),
        ("chutar", tactics::shoot_weight),
        ("bloquear", tactics::block_weight),
        ("goleiro", tactics::keeper_weight),
    ];
    for (name, f) in functions {
        for side in 0..2 {
            write!(w, " {name}{side}={}", bits(f(world, p, side))).unwrap();
        }
    }
    for side in 0..2 {
        let d = tactics::deviation(world, p, side, p.wings(world, side) == 1);
        write!(w, " setores{side}={},{}", bits(3.0 + d / 2.0), bits(2.5 - d / 2.0)).unwrap();
    }
    writeln!(out, "{w}").unwrap();
    let mut v = String::from("V");
    for &p in &world.game_match(engine.game_match).home_starters {
        let player = world.player(p);
        write!(v, " {}:{}[{} {}]", player.slot, brasfoot_engine::value::mod_value(true, player, player.slot), player.position, ints(&player.attributes)).unwrap();
    }
    writeln!(out, "{v}").unwrap();
}

fn play_day(tactics: bool) -> String {
    let bytes = std::fs::read(fixtures().join("engine-world.s22")).unwrap();
    let mut world = brasfoot_persistence::import::import_s22(bytes).unwrap();
    let mut runtime = Runtime::new(RandomSource::deterministic(FIRST_SEED));
    let mut hooks = NoSystems { tactics_off: !tactics };
    let mut out = String::new();
    let matches = world.game.days[0].matches.clone();
    for (index, &m) in matches.iter().enumerate() {
        prepare_squads(&mut world, &mut runtime, m);
        let (home, away) = (world.game_match(m).home.unwrap(), world.game_match(m).away.unwrap());
        pick_ai_lineup(&mut world, &mut runtime, home, Some(m), 1, -1, true);
        pick_ai_lineup(&mut world, &mut runtime, away, Some(m), 2, -1, true);
        let engine = Play { world: &mut world, runtime: &mut runtime, hooks: &mut hooks, game_match: m }.play_ai_match();
        dump_match(&mut out, &world, index, m);
        if std::env::var("BF_TRACE_TACTICS").is_ok() {
            dump_tactics(&mut out, &world, &engine);
        }
    }
    dump_players(&mut out, &mut world, &runtime);
    out
}

/// With BF_TRACE_TACTICS set, writes the Rust dump next to the fixtures for a manual diff.
fn compare(tactics: bool) {
    if std::env::var("BF_TRACE_TACTICS").is_ok() {
        std::fs::write(std::env::temp_dir().join(format!("engine-rust-{}.txt", tactics as u8)), play_day(tactics)).unwrap();
    }
    let expected = std::fs::read_to_string(fixtures().join(format!("engine-{}.txt", tactics as u8))).unwrap();
    let actual = play_day(tactics);
    if actual != expected {
        let line = actual.lines().zip(expected.lines()).position(|(a, e)| a != e);
        let show = |text: &str, i: usize| text.lines().skip(i.saturating_sub(3)).take(6).collect::<Vec<_>>().join("\n");
        match line {
            Some(i) => panic!("first difference at line {}:\n--- Rust\n{}\n--- Java\n{}", i + 1, show(&actual, i), show(&expected, i)),
            None => panic!("same lines, different length: {} vs {}", actual.lines().count(), expected.lines().count()),
        }
    }
}

#[test]
fn ai_matches_with_tactics_match_the_game() {
    compare(true);
}

#[test]
fn ai_matches_without_tactics_match_the_game() {
    compare(false);
}
