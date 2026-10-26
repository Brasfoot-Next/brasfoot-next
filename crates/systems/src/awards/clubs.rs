// Port of bfmod.Clubes: one row per league club and season (sav/<save>.clubes), and each club's
// top scorers and most capped players (sav/<save>.idolos), kept after they retire.

use std::collections::HashMap;

use brasfoot_model::ids::ClubId;
use brasfoot_model::java_hash::JavaHashMap;
use brasfoot_model::mods::{ClubSeason, Idol};
use brasfoot_model::world::World;

use super::coaches::campaigns;
use super::season::{Season, player_key};

/// `TOP_IDOLOS`
pub const TOP_IDOLS: usize = 10;

/// `Clubes.registrar(Temporada, boolean)`: the season of every league club. The standings of a
/// past season are gone, so its rows have only the titles and the top scorer.
pub fn register(world: &mut World, season: &Season, replace: bool) {
    let rows = &world.mods.awards.club_seasons;
    if rows.iter().any(|l| l.year == season.year) && !replace {
        return;
    }
    let campaigns = if season.current { campaigns(world) } else { HashMap::new() };
    let top_scorers = top_scorers(world, season.season);
    let mut coaches: HashMap<ClubId, String> = HashMap::new();
    if season.current {
        for &c in world.game.coaches.iter().chain(&world.game.human_coaches) {
            let coach = world.coach(c);
            if let Some(club) = coach.club_id {
                coaches.insert(club, coach.name.clone());
            }
        }
    }
    let mut new = Vec::new();
    for club in league_clubs(world) {
        let t = world.club(club);
        let k = campaigns.get(&club);
        let row = ClubSeason {
            year: season.year,
            club: t.id,
            name: t.name.clone(),
            country: t.country,
            league: k.map(|k| k.league.clone()),
            division: k.map_or(0, |k| k.division),
            position: k.map_or(0, |k| k.position),
            clubs: k.map_or(0, |k| k.clubs),
            titles: season.titles(club),
            top_scorer: top_scorers.get(&t.id).cloned(),
            coach: coaches.get(&club).cloned(),
        };
        if season.current || !row.titles.is_empty() || row.top_scorer.is_some() {
            new.push(row);
        }
    }
    let rows = &mut world.mods.awards.club_seasons;
    rows.retain(|l| l.year != season.year);
    rows.extend(new);
    rows.sort_by(|a, b| a.year.cmp(&b.year).then(a.club.cmp(&b.club)));
}

/// `Clubes.historia(int)`: a club's seasons, newest first.
pub fn history(world: &World, club: i32) -> Vec<&ClubSeason> {
    let mut out: Vec<&ClubSeason> = world.mods.awards.club_seasons.iter().filter(|l| l.club == club).collect();
    out.sort_by(|a, b| b.year.cmp(&a.year));
    out
}

/// `Clubes.carreira(String)`: the seasons a coach led, newest first.
pub fn coach_career<'a>(world: &'a World, coach: &str) -> Vec<&'a ClubSeason> {
    let mut out: Vec<&ClubSeason> = world.mods.awards.club_seasons.iter().filter(|l| l.coach.as_deref() == Some(coach)).collect();
    out.sort_by(|a, b| b.year.cmp(&a.year));
    out
}

/// `Clubes.clubesDasLigas()`: the clubs of the national leagues.
pub fn league_clubs(world: &World) -> Vec<ClubId> {
    let mut out = Vec::new();
    for &country in &world.game.league_countries {
        for &league in &world.country(country).divisions {
            if let Some(stage) = world.competition(league).league {
                out.extend(world.stage(stage).clubs.iter().copied());
            }
        }
    }
    out
}

/// `artilheiros(int)`: each club's top scorer of a season and his goals, by club id; the first
/// player found keeps a tie.
fn top_scorers(world: &World, season: i32) -> HashMap<i32, (String, i32)> {
    let mut out: HashMap<i32, (String, i32)> = HashMap::new();
    for &p in &world.game.players {
        let player = world.player(p);
        for s in &player.season_stats {
            if s.season == season && s.goals > out.get(&s.club_id).map_or(0, |t| t.1) {
                out.insert(s.club_id, (player.name.clone(), s.goals));
            }
        }
    }
    out
}

/// `Clubes.atualizarIdolos()`: merges every player's totals per club into the idols, keeping
/// each club's ten top scorers and ten most capped.
pub fn update_idols(world: &mut World) {
    // LinkedHashMap by "club#key"
    let mut all: Vec<Idol> = world.mods.awards.idols.clone();
    let mut index: HashMap<String, usize> = all.iter().enumerate().map(|(i, x)| (format!("{}#{}", x.club, x.key), i)).collect();
    for &p in &world.game.players {
        let player = world.player(p);
        let mut by_club: JavaHashMap<i32, [i32; 2]> = JavaHashMap::new();
        for s in &player.season_stats {
            let v = by_club.entry_or_insert_with(s.club_id, || [0; 2]);
            v[0] += s.games;
            v[1] += s.goals;
        }
        let key = player_key(world, p);
        for (&club, v) in by_club.iter() {
            if v[0] == 0 {
                continue;
            }
            let i = *index.entry(format!("{club}#{key}")).or_insert_with(|| {
                all.push(Idol { club, key: String::new(), name: String::new(), country: 0, position: 0, games: 0, goals: 0 });
                all.len() - 1
            });
            let idol = &mut all[i];
            idol.club = club;
            idol.key = key.clone();
            idol.name = player.name.clone();
            idol.country = player.country;
            idol.position = player.position;
            idol.games = idol.games.max(v[0]);
            idol.goals = idol.goals.max(v[1]);
        }
    }
    let mut by_club: JavaHashMap<i32, Vec<usize>> = JavaHashMap::new();
    for (i, idol) in all.iter().enumerate() {
        by_club.entry_or_insert_with(idol.club, Vec::new).push(i);
    }
    let mut keep = Vec::new();
    for list in by_club.into_values() {
        for i in best(&all, list) {
            keep.push(all[i].clone());
        }
    }
    world.mods.awards.idols = keep;
}

/// `melhores(List)`: the ten top scorers, then the ten most capped not among them (the second
/// sort starts from the first one's order).
fn best(all: &[Idol], mut list: Vec<usize>) -> Vec<usize> {
    let mut out = Vec::new();
    list.sort_by(|&a, &b| all[b].goals.cmp(&all[a].goals));
    out.extend(list.iter().take(TOP_IDOLS));
    list.sort_by(|&a, &b| all[b].games.cmp(&all[a].games));
    for &i in list.iter().take(TOP_IDOLS) {
        if !out.contains(&i) {
            out.push(i);
        }
    }
    out
}

/// `Clubes.idolosPorClube()`
pub fn idols_by_club(world: &World) -> HashMap<i32, Vec<&Idol>> {
    let mut out: HashMap<i32, Vec<&Idol>> = HashMap::new();
    for i in &world.mods.awards.idols {
        out.entry(i.club).or_default().push(i);
    }
    out
}
