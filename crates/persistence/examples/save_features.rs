// Prints which of the game's optional competitions a save uses (read only).
fn main() {
    let path = std::env::args().nth(1).expect("a .s22");
    let world = brasfoot_persistence::import::import_s22(std::fs::read(path).unwrap()).unwrap();
    let g = &world.game;
    println!("season {} start year {} day {}", g.season, g.start_year, g.day);
    println!("plays_international_clubs {}", g.plays_international_clubs);
    println!("plays_all_national_teams {}", g.plays_all_national_teams);
    println!("plays_regionals {}", g.plays_regionals);
    println!("state_leagues {}", g.state_leagues.len());
    println!("individual_skills {}", g.individual_skills);
    println!("flag_bc {}", g.flag_bc);
    println!("league countries {}", g.league_countries.len());
    println!("clubs {} players {} youth {}", g.clubs.len(), g.players.len(), g.youth_players.len());
    println!("friendly tournament {:?}", g.friendly_tournament.map(|c| world.competition(c).name.clone()));
    println!("loans {} stadium works {}", g.loans.len(), g.stadium_works.len());
    let mut kinds = std::collections::BTreeMap::new();
    for c in &world.competitions {
        *kinds.entry((c.kind, format!("{:?}", c.class))).or_insert(0) += 1;
    }
    for ((k, class), n) in kinds {
        println!("competition kind {k} {class}: {n}");
    }
    for &h in &g.human_coaches {
        let c = world.coach(h);
        println!("human coach {} at {:?}", c.name, c.club_id.map(|x| world.club(x).name.clone()));
    }
}
