//! Conversion of the original game's data folder (`teams/*.ban`, `conf_ligas_nacionais/*.cfg`,
//! `conf_estadual/*.ces` and the pictures) into a pack. Read-only on the game folder.

use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};

use brasfoot_model::templates::{LeagueConfig, PlayerTemplate, StateLeagueConfig, TeamTemplate};
use brasfoot_persistence::game_data;

use crate::codes;
use crate::format::*;
use crate::trophies;

/// What a conversion did and what it had to leave out.
#[derive(Debug, Default)]
pub struct Report {
    pub clubs: usize,
    pub players: usize,
    pub leagues: usize,
    pub state_leagues: usize,
    pub crests: usize,
    pub kits: usize,
    pub trophies: usize,
    pub national_team_pictures: usize,
    pub rivalries: usize,
    /// Clubs whose file name ends in another country's suffix (`<name>_bra.ban` of a club of
    /// another country).
    pub misplaced_suffix: Vec<(String, String)>,
    /// Clubs that got a numbered id because another club of the country has the same name.
    pub renamed: Vec<(String, String)>,
    pub without_crest: Vec<String>,
    /// Pictures of the clubs' folders that belong to no club.
    pub orphan_pictures: BTreeMap<String, usize>,
    /// Trophy files whose name the game never asks for.
    pub unknown_trophies: Vec<String>,
    /// Classic derbies of the original's table with a side missing from the data.
    pub missing_derbies: Vec<(String, String)>,
    /// Clubs whose potentials were the players' places in the list (1 to N), drawn again,
    /// and how many players that changed.
    pub potentials_drawn: (usize, usize),
    /// League and state league configurations the game never reads (division outside 1..4,
    /// a second one for the same division).
    pub ignored_configs: Vec<String>,
}

/// A slug of a name: ASCII, lowercase, words joined by hyphens.
pub fn slug(name: &str) -> String {
    let ascii = deunicode::deunicode(name).to_lowercase();
    let mut out = String::new();
    for c in ascii.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() { "club".to_owned() } else { trimmed.to_owned() }
}

/// The files of a folder by lowercase name, since the original's pictures are named with
/// whatever case the file reference had.
fn index(dir: &Path) -> HashMap<String, PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return HashMap::new() };
    entries.flatten().filter(|e| e.path().is_file()).map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path())).collect()
}

fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

fn write_toml<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
    write(path, toml::to_string(value).map_err(io::Error::other)?.as_bytes())
}

fn code(id: i32) -> String {
    codes::country_code(id).unwrap_or("BRA").to_owned()
}

fn player_row(p: &PlayerTemplate) -> PlayerRow {
    PlayerRow {
        // "TESTE" is the original's placeholder for a drawn name
        name: if p.name == "TESTE" { String::new() } else { p.name.clone() },
        position: codes::position_code(p.position).unwrap_or("MF").to_owned(),
        // F.setLado: anything but 0 is left
        side: codes::SIDES[usize::from(p.side != 0)].to_owned(),
        age: p.age,
        country: code(p.country),
        potential: p.hash,
        starter: p.status == 1,
        star: p.star,
        world_class: p.world_class,
        trait1: codes::trait_code(p.trait1).unwrap_or("").to_owned(),
        trait2: codes::trait_code(p.trait2).unwrap_or("").to_owned(),
    }
}

/// `String.hashCode()`
fn java_string_hash(s: &str) -> i32 {
    s.encode_utf16().fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

/// Whether the list's potentials are just the players' places in it (1 to N, in any order when
/// the list was sorted afterwards), as the tool that made many team files wrote them; more than
/// ten players, so that it cannot be chance.
fn potentials_are_places(players: &[PlayerTemplate]) -> bool {
    let mut values: Vec<i32> = players.iter().map(|p| p.hash).collect();
    values.sort_unstable();
    players.len() > 10 && values.iter().enumerate().all(|(i, &v)| v == i as i32 + 1)
}

/// New potentials for such a list, drawn as the game draws a youth player's for the club
/// (`youth_potential`), from a seed of the club's id and the player's place: the same data
/// always converts the same way.
fn draw_potentials(players: &mut [PlayerTemplate], club: &str, level: i32, reputation: i32) {
    for (i, p) in players.iter_mut().enumerate() {
        let mut rng = brasfoot_model::rng::JavaRandom::new(i64::from(java_string_hash(&format!("{club}#{i}"))));
        let draw = rng.next_int_bounded(100) + 1;
        p.hash = brasfoot_simulation::player_gen::youth_potential(level, reputation, draw);
    }
}

fn write_players(path: &Path, players: &[PlayerTemplate]) -> io::Result<()> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    for p in players {
        writer.serialize(player_row(p)).map_err(io::Error::other)?;
    }
    write(path, &writer.into_inner().map_err(io::Error::other)?)
}

/// `config.relegated_directly` as the game reads it (`f.B` in `p.dd()`): from a file of another
/// version, zero, or more than the relegated places, it is the relegated places.
fn effective_relegated_directly(c: &LeagueConfig) -> i32 {
    let relegated = if c.teams <= 10 && c.relegated > 2 { 2 } else { c.relegated };
    if c.file_version != 22 || c.relegated_directly == 0 || c.relegated_directly > relegated { relegated } else { c.relegated_directly }
}

fn division(c: &LeagueConfig) -> DivisionFile {
    DivisionFile {
        division: c.division,
        name: c.name.clone().filter(|s| !s.is_empty()),
        division_name: c.division_name.clone().filter(|s| !s.is_empty()),
        teams: c.teams,
        two_rounds: c.two_rounds,
        round_count: c.formula,
        groups: c.groups,
        games_within_group: c.games_within_group,
        knockout_teams: c.knockout_teams,
        two_legged_knockout: c.two_legged_knockout.clone(),
        qualify_by_overall_table: c.qualify_by_overall_table,
        best_third_places: c.best_third_places,
        tiebreak: c.tiebreak,
        relegated: c.relegated,
        relegated_directly: effective_relegated_directly(c),
        relegated_by_group: c.relegated_by_group,
        relegation_playoff: c.relegation_playoff,
        two_legged_relegation_playoff: c.two_legged_relegation_playoff.clone(),
        promotion_places: c.promotion_places,
        two_legged_promotion_knockout: c.two_legged_promotion_knockout.clone(),
    }
}

fn state_division(c: &StateLeagueConfig) -> StateDivisionFile {
    StateDivisionFile {
        division: c.division,
        format: codes::state_format_code(c.formula).unwrap_or(codes::STATE_FORMATS[0]).to_owned(),
        relegated: c.relegated,
        tiebreak: c.tiebreak,
        two_legged_finals: c.two_legged_finals.map(|n| n == 2),
    }
}

/// The club ids of the team files: `<country>/<slug of the name>`, numbered when two clubs of a
/// country share a slug (in the order the game reads the files).
fn club_ids(teams: &[TeamTemplate], report: &mut Report) -> Vec<String> {
    let mut used: HashMap<String, usize> = HashMap::new();
    teams
        .iter()
        .map(|t| {
            let base = format!("{}/{}", code(t.country).to_lowercase(), slug(&t.name));
            let n = used.entry(base.clone()).or_default();
            *n += 1;
            if *n == 1 {
                base
            } else {
                let id = format!("{base}-{n}");
                report.renamed.push((t.file_ref.clone(), id.clone()));
                id
            }
        })
        .collect()
}

/// Converts the game folder `game` into a pack in `out` (which should be empty).
pub fn convert(game: &Path, out: &Path, manifest: &Manifest) -> io::Result<Report> {
    let mut report = Report::default();
    let mut teams = game_data::read_teams(&game.join("teams"));
    // the game reads them in the directory's order; a pack has no order, ids sort it
    teams.sort_by(|a, b| a.file_ref.cmp(&b.file_ref));
    let ids = club_ids(&teams, &mut report);
    let by_ref: HashMap<String, String> = teams.iter().zip(&ids).map(|(t, id)| (t.file_ref.to_lowercase(), id.clone())).collect();

    // classic derbies: the original's fixed table, by file reference, on the first club of a pair
    let mut rivals: HashMap<String, Vec<String>> = HashMap::new();
    for pair in brasfoot_simulation::tables::CLASSIC_DERBIES {
        match (by_ref.get(pair[0]), by_ref.get(pair[1])) {
            (Some(a), Some(b)) => {
                rivals.entry(a.clone()).or_default().push(b.clone());
                report.rivalries += 1;
            }
            _ => report.missing_derbies.push((pair[0].to_owned(), pair[1].to_owned())),
        }
    }

    // the game shows the crests of escudos/; teams/escudos is a copy some patches carry
    let crests = [index(&game.join("escudos")), index(&game.join("teams/escudos"))];
    let kits = ["camisas", "camisas2", "camisas3"].map(|d| index(&game.join("teams").join(d)));
    for (t, id) in teams.iter_mut().zip(&ids) {
        for (list, kind) in [(&mut t.players, "squad"), (&mut t.youth, "youth")] {
            if potentials_are_places(list) {
                draw_potentials(list, &format!("{id}/{kind}"), t.level, t.reputation);
                report.potentials_drawn.0 += usize::from(kind == "squad");
                report.potentials_drawn.1 += list.len();
            }
        }
        let t = &*t;
        let dir = out.join("clubs").join(id);
        let country = code(t.country);
        if let Some((_, suffix)) = t.file_ref.rsplit_once('_')
            && suffix.eq_ignore_ascii_case("bra")
            && t.country != codes::BRAZIL
        {
            report.misplaced_suffix.push((t.file_ref.clone(), country.clone()));
        }
        let club = ClubFile {
            name: t.name.clone(),
            country: country.clone(),
            state: if t.country == codes::BRAZIL { codes::state_code(t.state).map(str::to_owned) } else { None },
            level: t.level,
            reputation: t.reputation,
            colors: [t.color1.clone(), t.color2.clone()],
            rivals: rivals.remove(id).unwrap_or_default(),
            stadium: StadiumFile { name: t.stadium.clone(), capacity: t.capacity },
            coach: CoachFile { name: t.coach.clone(), country: code(t.coach_country) },
        };
        write_toml(&dir.join(CLUB), &club)?;
        write_players(&dir.join(SQUAD), &t.players)?;
        if !t.youth.is_empty() {
            write_players(&dir.join(YOUTH), &t.youth)?;
        }
        report.clubs += 1;
        report.players += t.players.len() + t.youth.len();

        let file = format!("{}.png", t.file_ref.to_lowercase());
        match crests.iter().find_map(|c| c.get(&file)) {
            Some(path) => {
                write(&dir.join(CREST), &std::fs::read(path)?)?;
                report.crests += 1;
            }
            None => report.without_crest.push(t.file_ref.clone()),
        }
        for (folder, name) in kits.iter().zip(KITS) {
            if let Some(path) = folder.get(&file) {
                write(&dir.join(name), &std::fs::read(path)?)?;
                report.kits += 1;
            }
        }
    }
    let crest_names: std::collections::HashSet<String> = teams.iter().map(|t| format!("{}.png", t.file_ref.to_lowercase())).collect();
    for (label, folder) in [("escudos", &crests[0]), ("teams/escudos", &crests[1])] {
        report.orphan_pictures.insert(label.to_owned(), folder.keys().filter(|k| !crest_names.contains(*k)).count());
    }
    for (label, folder) in ["teams/camisas", "teams/camisas2", "teams/camisas3"].iter().zip(&kits) {
        report.orphan_pictures.insert((*label).to_owned(), folder.keys().filter(|k| !crest_names.contains(*k)).count());
    }

    // national leagues: the first configuration of a country and division is the one read
    let mut leagues: BTreeMap<i32, LeagueFile> = BTreeMap::new();
    for c in game_data::read_league_configs(&game.join("conf_ligas_nacionais")) {
        let Some(country) = codes::country_code(c.country) else {
            report.ignored_configs.push(format!("league country {} division {}", c.country, c.division));
            continue;
        };
        let league = leagues.entry(c.country).or_insert_with(|| LeagueFile { country: country.to_owned(), divisions: Vec::new() });
        if !(1..=4).contains(&c.division) || league.divisions.iter().any(|d| d.division == c.division) {
            report.ignored_configs.push(format!("league {country} division {}", c.division));
            continue;
        }
        league.divisions.push(division(&c));
    }
    for league in leagues.values_mut() {
        league.divisions.sort_by_key(|d| d.division);
        write_toml(&out.join("leagues").join(format!("{}.toml", league.country.to_lowercase())), league)?;
        report.leagues += 1;
    }

    let mut states: BTreeMap<i32, StateLeagueFile> = BTreeMap::new();
    for c in game_data::read_state_league_configs(&game.join("conf_estadual")) {
        let Some(state) = codes::state_code(c.state) else {
            report.ignored_configs.push(format!("state league {} division {}", c.state, c.division));
            continue;
        };
        let league = states.entry(c.state).or_insert_with(|| StateLeagueFile { state: state.to_owned(), divisions: Vec::new() });
        if !(1..=4).contains(&c.division) || league.divisions.iter().any(|d| d.division == c.division) {
            report.ignored_configs.push(format!("state league {state} division {}", c.division));
            continue;
        }
        league.divisions.push(state_division(&c));
    }
    for league in states.values_mut() {
        league.divisions.sort_by_key(|d| d.division);
        write_toml(&out.join("state-leagues").join(format!("{}.toml", league.state.to_lowercase())), league)?;
        report.state_leagues += 1;
    }

    // national teams: selecoes/escudos, camisas, camisas2 by the game's country code
    for (folder, name) in [("escudos", "crest.png"), ("camisas", "kit.png"), ("camisas2", "kit2.png")] {
        for (file, path) in index(&game.join("selecoes").join(folder)) {
            let Some(stem) = file.strip_suffix(".png") else { continue };
            let Some(country) = brasfoot_model::countries::COUNTRIES.iter().find(|c| c.code.eq_ignore_ascii_case(stem)) else { continue };
            let fifa = code(country.id).to_lowercase();
            write(&out.join("national-teams").join(fifa).join(name), &std::fs::read(path)?)?;
            report.national_team_pictures += 1;
        }
    }

    let mut trophy_files: Vec<_> = index(&game.join("trofeus")).into_iter().collect();
    trophy_files.sort();
    for (file, path) in trophy_files {
        let stem = file.strip_suffix(".png").unwrap_or(&file);
        match trophies::pack_path(stem) {
            Some(target) => {
                write(&out.join(target), &std::fs::read(path)?)?;
                report.trophies += 1;
            }
            None => report.unknown_trophies.push(path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(file)),
        }
    }

    write_toml(&out.join(MANIFEST), manifest)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_ascii_words_joined_by_hyphens() {
        assert_eq!(slug("União Fictícia"), "uniao-ficticia");
        assert_eq!(slug("  Vila  Exemplo F.C. "), "vila-exemplo-f-c");
        assert_eq!(slug("Łódź Inventado"), "lodz-inventado");
        assert_eq!(slug("!!!"), "club");
    }

    #[test]
    fn relegated_directly_is_written_as_the_game_reads_it() {
        let config = |teams, relegated, directly, version| LeagueConfig { teams, relegated, relegated_directly: directly, file_version: version, ..Default::default() };
        assert_eq!(effective_relegated_directly(&config(20, 4, 2, 22)), 2);
        // another version, zero, or more than the relegated places: the relegated places
        assert_eq!(effective_relegated_directly(&config(20, 4, 2, 21)), 4);
        assert_eq!(effective_relegated_directly(&config(20, 4, 0, 22)), 4);
        assert_eq!(effective_relegated_directly(&config(20, 3, 5, 22)), 3);
        // ten teams relegate at most two
        assert_eq!(effective_relegated_directly(&config(10, 4, 3, 22)), 2);
    }

    #[test]
    fn potentials_that_are_list_places_are_drawn_again_the_same_way_every_time() {
        let list = |n: i32, f: &dyn Fn(i32) -> i32| (0..n).map(|i| PlayerTemplate { hash: f(i), ..Default::default() }).collect::<Vec<_>>();
        assert!(potentials_are_places(&list(14, &|i| i + 1)));
        assert!(!potentials_are_places(&list(10, &|i| i + 1)));
        assert!(!potentials_are_places(&list(14, &|i| i % 10)));
        assert!(potentials_are_places(&list(14, &|i| 14 - i)));
        let mut a = list(30, &|i| i + 1);
        let mut b = a.clone();
        draw_potentials(&mut a, "bra/vila-exemplo/squad", 12, 3);
        draw_potentials(&mut b, "bra/vila-exemplo/squad", 12, 3);
        assert_eq!(a, b);
        assert!(a.iter().all(|p| (1..=10).contains(&p.hash)));
        assert!(!potentials_are_places(&a));
        assert_eq!(java_string_hash("Brasfoot"), 43_951_120);
    }

    #[test]
    fn players_keep_every_field_the_game_reads() {
        let p = PlayerTemplate { name: "TESTE".into(), star: true, country: 11, age: 23, position: 4, status: 1, trait1: 9, trait2: -1, side: 7, hash: 6, world_class: false, ..Default::default() };
        let row = player_row(&p);
        assert_eq!(row.name, "");
        assert_eq!((row.position.as_str(), row.side.as_str(), row.country.as_str()), ("FW", "L", "ARG"));
        assert_eq!((row.potential, row.starter, row.star), (6, true, true));
        assert_eq!((row.trait1.as_str(), row.trait2.as_str()), ("finishing", ""));
    }
}
