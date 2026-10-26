//! Summary of a game folder's data files: teams per country and league configurations.
//! Usage: game_data <game folder>

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("game folder"));
    let teams = brasfoot_persistence::game_data::read_teams(&dir.join("teams"));
    let mut per_country = std::collections::BTreeMap::new();
    for t in &teams {
        *per_country.entry(t.country).or_insert(0) += 1;
    }
    println!("{} teams, {} countries", teams.len(), per_country.len());
    for (c, n) in &per_country {
        if *n >= 10 {
            print!("{c}:{n} ");
        }
    }
    println!();
    let configs = brasfoot_persistence::game_data::read_league_configs(&dir.join("conf_ligas_nacionais"));
    println!("{} league configurations; first: {:?}", configs.len(), configs.first());
    for c in &configs {
        if [29, 72].contains(&c.country) {
            println!("{c:?}");
        }
    }
    println!("{:?}", brasfoot_persistence::game_data::read_options(&dir.join("options.bcf")));
}
