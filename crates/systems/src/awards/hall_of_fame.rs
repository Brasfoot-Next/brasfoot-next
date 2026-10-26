// Port of bfmod.HallDaFama: the all-time records of a save. The game recycles a retiring player
// into a new one and wipes his career, so the notable careers are copied (sav/<save>.carreiras)
// every time this runs; a copy whose player is gone is a retirement.

use std::collections::HashMap;

use brasfoot_model::ids::PlayerId;
use brasfoot_model::java_hash::JavaHashMap;
use brasfoot_model::mods::{AwardKind, Career, RecordKind, RecordMark};
use brasfoot_model::world::World;

use super::history::{AwardYears, COACH_KEY, by_player};
use super::season::player_key;
use crate::economy::{is_national_team, year};

/// `TOP_RECORDES`
pub const TOP_RECORDS: usize = 10;
/// `TOP_ARTILHEIROS`
pub const TOP_SCORERS: usize = 50;

/// `Carreira.lenda(Map)`
pub fn is_legend(c: &Career, awards: &JavaHashMap<String, AwardYears>) -> bool {
    awards.contains_key(&c.key) || c.goals >= 150 || (c.games >= 500 && c.rating() >= 7.0)
}

/// `Carreira.rastrear(Map)`: a career worth keeping.
fn worth_tracking(c: &Career, awards: &JavaHashMap<String, AwardYears>) -> bool {
    awards.contains_key(&c.key) || c.goals >= 50 || c.games >= 300
}

/// `HallDaFama.atualizar(int)`: copies the careers, finds the retirements and merges the season
/// records, the season `final_season` counting as finished. Returns the careers found retired.
pub fn update(world: &mut World, final_season: i32) -> Vec<Career> {
    let awards = by_player(&world.mods.awards);
    let mut saved = world.mods.awards.careers.clone();
    let mut saved_index: HashMap<String, usize> = saved.iter().enumerate().map(|(i, c)| (c.key.clone(), i)).collect();
    let active = active_careers(world);
    let this_year = year(world);
    for c in active.values() {
        if worth_tracking(c, &awards) || saved_index.contains_key(&c.key) {
            let c = Career { last_year: this_year, ..c.clone() };
            match saved_index.get(&c.key) {
                Some(&i) => saved[i] = c,
                None => {
                    saved_index.insert(c.key.clone(), saved.len());
                    saved.push(c);
                }
            }
        }
    }
    let mut retired = Vec::new();
    for c in &mut saved {
        if c.retired == 0 && !active.contains_key(&c.key) {
            c.retired = c.last_year;
            retired.push(c.clone());
        }
    }
    world.mods.awards.careers = saved;
    world.mods.awards.records = merge_records(world, &world.mods.awards.records, final_season);
    retired
}

/// `HallDaFama.recordes()`: the saved records with the seasons still in the players' stats.
pub fn records(world: &World) -> Vec<RecordMark> {
    merge_records(world, &world.mods.awards.records, world.season() - 1)
}

/// `HallDaFama.recordesSalvos(Recorde)`
pub fn saved_records(world: &World, kind: RecordKind) -> Vec<&RecordMark> {
    world.mods.awards.records.iter().filter(|m| m.kind == kind).collect()
}

/// `HallDaFama.artilheiros()`: the top scorers of all time, active and retired.
pub fn top_scorers(world: &World) -> Vec<Career> {
    let mut out: Vec<Career> = active_careers(world).into_values();
    out.extend(world.mods.awards.careers.iter().filter(|c| c.retired > 0).cloned());
    out.sort_by(|a, b| b.goals.cmp(&a.goals).then(a.games.cmp(&b.games)));
    out.truncate(TOP_SCORERS);
    out
}

/// `HallDaFama.lendas()`: the retired legends, greatest first.
pub fn legends(world: &World) -> Vec<Career> {
    let awards = by_player(&world.mods.awards);
    let mut out: Vec<Career> = world.mods.awards.careers.iter().filter(|c| c.retired > 0 && is_legend(c, &awards)).cloned().collect();
    out.sort_by(|a, b| greatness(b, &awards).total_cmp(&greatness(a, &awards)));
    out
}

/// `grandeza`
fn greatness(c: &Career, awards: &JavaHashMap<String, AwardYears>) -> f64 {
    let mut g = c.goals as f64 / 10.0 + c.games as f64 / 50.0;
    for (kind, years) in awards.get(&c.key).into_iter().flatten() {
        g += (years.len() * kind_weight(*kind, 100, 10, 30)) as f64;
    }
    g
}

fn kind_weight(kind: AwardKind, ballon: usize, team: usize, other: usize) -> usize {
    match kind {
        AwardKind::BallonDor => ballon,
        AwardKind::TeamOfTheYear => team,
        _ => other,
    }
}

/// `HallDaFama.galeria()`: the players with awards, most decorated first.
pub fn gallery(world: &World) -> Vec<(String, AwardYears)> {
    let mut out: Vec<(String, AwardYears)> =
        by_player(&world.mods.awards).iter().filter(|(k, _)| !k.starts_with(COACH_KEY)).map(|(k, v)| (k.clone(), v.clone())).collect();
    let weight = |m: &AwardYears| m.iter().fold(0.0, |p, (kind, years)| p + (years.len() * kind_weight(*kind, 1000, 1, 30)) as f64);
    out.sort_by(|a, b| weight(&b.1).total_cmp(&weight(&a.1)));
    out
}

/// Where a player of the awards is now (`situacoes()`).
#[derive(Clone, Debug, PartialEq)]
pub enum Situation {
    /// `Aposentado em <year>`
    RetiredIn(i32),
    /// `Aposentado`: a copied career not yet seen gone.
    Retired,
    Club(String),
    /// `Sem clube`
    Free,
}

/// `HallDaFama.situacoes()`: by player key.
pub fn situations(world: &World) -> HashMap<String, Situation> {
    let mut out = HashMap::new();
    for c in &world.mods.awards.careers {
        out.insert(c.key.clone(), if c.retired > 0 { Situation::RetiredIn(c.retired) } else { Situation::Retired });
    }
    for &p in &world.game.players {
        let player = world.player(p);
        let situation = player.club.map(|k| Situation::Club(world.club(k).name.clone())).unwrap_or(Situation::Free);
        out.insert(player_key(world, p), situation);
    }
    out
}

/// `carreirasAtivas()`: every player's career so far, by key.
fn active_careers(world: &World) -> JavaHashMap<String, Career> {
    let mut out = JavaHashMap::new();
    for &p in &world.game.players {
        let player = world.player(p);
        let mut c = Career {
            key: player_key(world, p),
            name: player.name.clone(),
            country: player.country,
            position: player.position,
            age: player.age,
            club: player.club.map(|k| world.club(k).name.clone()),
            ..Career::default()
        };
        for s in &player.season_stats {
            c.games += s.games;
            c.goals += s.goals;
            c.assists += s.assists;
            c.rating_sum += s.rating_sum;
            c.ratings += s.rated_games;
        }
        out.insert(c.key.clone(), c);
    }
    out
}

/// `mesclarRecordes(Map, int)`: the saved records plus every season up to `final_season` in the
/// players' stats, the top ten of each.
fn merge_records(world: &World, saved: &[RecordMark], final_season: i32) -> Vec<RecordMark> {
    let mut candidates: HashMap<RecordKind, Vec<(f64, PlayerId, i32)>> = HashMap::new();
    let mut candidate = |kind: RecordKind, value: f64, p: PlayerId, season: i32| {
        if value > 0.0 {
            candidates.entry(kind).or_default().push((value, p, season));
        }
    };
    for &p in &world.game.players {
        let mut by_season: JavaHashMap<i32, [i32; 3]> = JavaHashMap::new();
        let mut ratings: HashMap<i32, [f64; 2]> = HashMap::new();
        for s in &world.player(p).season_stats {
            if s.season < 1 || s.season > final_season {
                continue;
            }
            let v = by_season.entry_or_insert_with(s.season, || [0; 3]);
            v[0] += s.games;
            v[1] += s.goals;
            v[2] += s.assists;
            let n = ratings.entry(s.season).or_insert([0.0; 2]);
            n[0] += s.rating_sum;
            n[1] += s.rated_games as f64;
        }
        for (&season, v) in by_season.iter() {
            let n = ratings[&season];
            candidate(RecordKind::Goals, v[1] as f64, p, season);
            candidate(RecordKind::Assists, v[2] as f64, p, season);
            candidate(RecordKind::Contributions, (v[1] + v[2]) as f64, p, season);
            if v[0] >= 20 && n[1] >= 10.0 {
                candidate(RecordKind::Rating, n[0] / n[1], p, season);
            }
        }
    }
    let mut out = Vec::new();
    for kind in RecordKind::ALL {
        let mut list: Vec<RecordMark> = saved.iter().filter(|m| m.kind == kind).cloned().collect();
        let mut cs = candidates.remove(&kind).unwrap_or_default();
        cs.sort_by(|a, b| b.0.total_cmp(&a.0));
        for &(value, p, season) in cs.iter().take(TOP_RECORDS * 2) {
            let player = world.player(p);
            let m = RecordMark {
                kind,
                value,
                key: player_key(world, p),
                name: player.name.clone(),
                club: club_in_season(world, p, season),
                country: player.country,
                year: world.game.start_year + season - 1,
            };
            list.retain(|y| !(y.key == m.key && y.year == m.year));
            list.push(m);
        }
        list.sort_by(|a, b| b.value.total_cmp(&a.value));
        list.truncate(TOP_RECORDS);
        out.extend(list);
    }
    out
}

/// `clubeNaTemporada(F, int)`: the club, not a national team, he played most for that season.
fn club_in_season(world: &World, p: PlayerId, season: i32) -> Option<String> {
    let mut best = None;
    let mut games = 0;
    for s in &world.player(p).season_stats {
        if s.season == season && s.games > games {
            if let Some(t) = world.club_by_game_id(s.club_id).filter(|&t| !is_national_team(world, t)) {
                best = Some(t);
                games = s.games;
            }
        }
    }
    best.map(|t| world.club(t).name.clone())
}
