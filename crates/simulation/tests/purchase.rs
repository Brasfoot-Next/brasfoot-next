//! Differential test of a human club's purchases against the game (`tools/exporter/export.sh
//! purchase`, exporter.PurchaseHarness): the answers of the negotiation `best.l.a(F, ah, int)`
//! for the players of the first clubs, asked by several buyers as if from each confederation,
//! then three deals closed like `a.cz.oL()`, with the seller filling its squad.

use std::path::Path;

use brasfoot_model::canonical::canonical;
use brasfoot_model::clock::Clock;
use brasfoot_model::ids::PlayerId;
use brasfoot_model::names::NameLists;
use brasfoot_model::rng::RandomSource;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;
use brasfoot_simulation::human::purchase::{OfferAnswer, offer};

fn load(name: &str) -> World {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap()
}

/// `l.a(F, ah, int)`'s number, `cM()` (the salary asked) and, for a counter offer, `cN()`.
fn code(answer: OfferAnswer) -> (i32, i32, Option<i32>) {
    match answer {
        OfferAnswer::Refused => (0, 0, None),
        OfferAnswer::Accepted => (1, 0, None),
        OfferAnswer::OwnPlayer => (2, 0, None),
        OfferAnswer::SquadLimit => (3, 0, None),
        OfferAnswer::Unwilling => (4, 0, None),
        OfferAnswer::NoMoney => (5, 0, None),
        OfferAnswer::WantsSalary(s) => (6, s, None),
        OfferAnswer::Counter { price, salary } => (7, salary.unwrap_or(0), Some(price)),
    }
}

#[test]
fn offers_and_purchases_match_the_game() {
    let mut world = load("human-start.s22");
    let buyer = world.club_by_game_id(1).unwrap();
    // the players asked about, in the harness's order: the squads of the first six other clubs
    let players: Vec<PlayerId> = world.game.clubs.iter().filter(|&&c| c != buyer).take(6).flat_map(|&c| world.club(c).squad.clone()).collect();
    let answers = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/purchase-answers.txt")).unwrap();
    let mut cursor = 0;
    let mut accepted = Vec::new();
    let mut checked = 0;
    for (n, line) in answers.lines().enumerate() {
        let f: Vec<&str> = line.split('|').collect();
        while world.player(players[cursor]).name != f[0] {
            cursor += 1;
        }
        let p = players[cursor];
        let (club_id, country, amount): (i32, i32, i32) = (f[1].parse().unwrap(), f[2].parse().unwrap(), f[3].parse().unwrap());
        let expected: (i32, i32, i32) = (f[4].parse().unwrap(), f[5].parse().unwrap(), f[6].parse().unwrap());
        let b = world.club_by_game_id(club_id).unwrap();
        let home = world.club(b).country;
        world.club_mut(b).country = country;
        let (answer, salary, counter) = code(offer(&world, p, b, amount));
        world.club_mut(b).country = home;
        assert_eq!((answer, salary), (expected.0, expected.1), "line {}: {line}", n + 1);
        if let Some(price) = counter {
            assert_eq!(price, expected.2, "line {}: {line}", n + 1);
        }
        let value = world.player(p).market_value;
        if b == buyer
            && country == home
            && answer == 1
            && amount == (value as f64 * 3.0).round() as i32
            && accepted.len() < 3
            && !accepted.contains(&p)
            && (value as i64) * 3 < world.club(buyer).cash
        {
            accepted.push(p);
        }
        checked += 1;
    }
    assert!(checked > 1000 && accepted.len() == 3);

    let mut runtime = Runtime::new(RandomSource::deterministic(1000));
    runtime.clock = Clock::fixed(1_792_506_615_250, "America/Sao_Paulo");
    runtime.names = NameLists::new(std::env::var("BF_GAME_FILES").unwrap_or_else(|_| format!("{}/brasfoot-mods/orig/jar", std::env::var("HOME").unwrap())));
    let mut hooks = brasfoot_engine::hooks::NoSystems::default();
    for p in accepted {
        let fee = (world.player(p).market_value as f64 * 3.0).round() as i32;
        Sim { world: &mut world, runtime: &mut runtime, hooks: &mut hooks, human: None }.complete_purchase(p, buyer, fee, None);
    }
    // the harness saves (g.b.Ai) and the exporter loads (g.b.Aj)
    world.number_players_for_save();
    world.number_competitions_for_save();
    world.relink_transfers();
    let actual = canonical(&world);
    let expected = canonical(&load("purchase-after.s22"));
    if actual != expected {
        let (a, e): (Vec<&str>, Vec<&str>) = (actual.lines().collect(), expected.lines().collect());
        let i = a.iter().zip(&e).position(|(x, y)| x != y).unwrap_or(a.len().min(e.len()));
        panic!("line {}:\n--- Rust\n{}\n--- Java\n{}", i + 1, a[i.saturating_sub(3)..(i + 3).min(a.len())].join("\n"), e[i.saturating_sub(3)..(i + 3).min(e.len())].join("\n"));
    }
}
