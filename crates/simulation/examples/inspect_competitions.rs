//! Debug aid: lists the competitions and stages of a fixture save of the simulation tests.
//! Usage: inspect_competitions <fixture name>

use std::path::Path;

fn main() {
    let name = std::env::args().nth(1).unwrap_or_else(|| "sim-start.s22".into());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    let world = brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap();
    for (i, c) in world.competitions.iter().enumerate() {
        println!(
            "competition {i} {:?} kind {} subkind {} division {} relegated {}/{} promotion knockout places {} league {:?} knockout {:?} titles {}",
            c.class, c.kind, c.subkind, c.division, c.relegated, c.relegated_directly, c.promotion_knockout_places, c.league, c.knockout, c.titles.len()
        );
    }
    for (i, s) in world.stages.iter().enumerate() {
        println!(
            "stage {i} {:?} kind {} phase {} format {} clubs {} round {} last {} value_ze {} knockout_teams {} groups {} division {} competition {:?}",
            s.class, s.kind_code(), s.phase_code, s.format_code, s.clubs.len(), s.round, s.last_round, s.value_ze, s.knockout_teams, s.groups.len(), s.division, s.competition
        );
    }
    for &c in &world.game.countries {
        let k = world.country(c);
        println!("country {} national team {:?} divisions {} cup {:?}", k.country, k.national_team, k.divisions.len(), k.cup);
    }
    println!("league countries {:?}", world.game.league_countries);
    println!("day {} season {} days {}", world.game.day, world.game.season, world.game.days.len());
}
