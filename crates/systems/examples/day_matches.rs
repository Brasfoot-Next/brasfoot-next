//! Prints the matches of a calendar day of a career: score, line-ups and events. For comparing a
//! Rust world (`.bfn`) with the game's (`.s22`) when a differential test differs.
//! Usage: day_matches <save.s22|save.bfn> <calendar day>

use brasfoot_model::world::World;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = std::path::PathBuf::from(args.next().expect("save"));
    let day = args.next().expect("day");
    let world: World = if path.extension().is_some_and(|e| e == "bfn") {
        brasfoot_persistence::native::read_save(&path).unwrap().1
    } else {
        brasfoot_persistence::import::import_s22(std::fs::read(&path).unwrap()).unwrap()
    };
    let name = |p: Option<brasfoot_model::ids::PlayerId>| p.map(|p| world.player(p).name.clone()).unwrap_or_else(|| "-".into());
    // "current": the save's current day
    let day: usize = if day == "current" { world.game.day as usize } else { day.parse().unwrap() };
    println!("day {day}");
    for &m in &world.game.days[day].matches {
        let g = world.game_match(m);
        let club = |c: Option<brasfoot_model::ids::ClubId>| c.map(|c| world.club(c).name.clone()).unwrap_or_default();
        println!("{} {} x {} {}  played={} human={}", club(g.home), g.home_goals, g.away_goals, club(g.away), g.played,
            [g.home, g.away].into_iter().flatten().any(|c| world.club(c).human));
        for e in &g.events {
            println!("    {}' h{} kind {} detail {} {} ({}) other {} done={} dg={}", e.minute, e.half, e.kind, e.detail, name(e.player), club(e.club), name(e.other_player), e.done, e.flag_dg);
        }
    }
}
