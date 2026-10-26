// Port of bfmod.Premios: the awards of every season of a save (sav/<save>.premios).

use std::collections::BTreeMap;

use brasfoot_model::java_hash::JavaHashMap;
use brasfoot_model::mods::{Award, AwardExtra, AwardKind, AwardsState};
use brasfoot_model::world::World;

use super::coaches::CoachCandidate;
use super::season::{Candidate, Season};

/// `CHAVE_TECNICO`: coaches share the list with the players; their keys carry this prefix.
pub const COACH_KEY: &str = "tec|";

/// The years of each award a key won (`EnumMap`).
pub type AwardYears = BTreeMap<AwardKind, Vec<i32>>;

/// `Premios.porJogador()`: player key, award, years won.
pub fn by_player(state: &AwardsState) -> JavaHashMap<String, AwardYears> {
    let mut out: JavaHashMap<String, AwardYears> = JavaHashMap::new();
    for a in &state.awards {
        out.entry_or_insert_with(a.key.clone(), AwardYears::new).entry(a.kind).or_default().push(a.year);
    }
    out
}

/// `Premios.doTipo(Tipo)`
pub fn of_kind(state: &AwardsState, kind: AwardKind) -> Vec<&Award> {
    state.awards.iter().filter(|a| a.kind == kind).collect()
}

/// `Premios.registrar(Temporada, boolean)`: every award of a season. Without `replace`, the
/// kinds already recorded for that year stay.
pub fn register(world: &mut World, season: &Season, coaches: &[CoachCandidate], replace: bool) {
    let year = season.year;
    let mut new: Vec<(AwardKind, Vec<Award>)> = vec![
        (AwardKind::BallonDor, first(world, &season.ballon_dor(1), AwardKind::BallonDor, year)),
        (AwardKind::GoldenShoe, first(world, &season.golden_shoe(1), AwardKind::GoldenShoe, year)),
        (AwardKind::GoldenGlove, first(world, &season.golden_glove(world, 1), AwardKind::GoldenGlove, year)),
        (AwardKind::GoldenBoy, first(world, &season.golden_boy(1), AwardKind::GoldenBoy, year)),
    ];
    let team = season
        .team_of_the_year(world)
        .into_iter()
        .map(|(slot, c)| Award { extra: AwardExtra::Slot(slot), ..award(world, c, AwardKind::TeamOfTheYear, year) })
        .collect();
    new.push((AwardKind::TeamOfTheYear, team));
    if let Some(v) = coaches.first() {
        let coach = world.coach(v.coach);
        let club = world.club(v.club);
        new.push((
            AwardKind::CoachOfTheYear,
            vec![Award {
                year,
                kind: AwardKind::CoachOfTheYear,
                key: format!("{COACH_KEY}{}", coach.name),
                name: coach.name.clone(),
                club: Some(club.name.clone()),
                country: club.country,
                position: 0,
                age: 0,
                rating: 0.0,
                goals: 0,
                assists: 0,
                extra: AwardExtra::Coach(v.summary()),
            }],
        ));
    }
    let list = &mut world.mods.awards.awards;
    for (kind, awards) in new {
        if awards.is_empty() {
            continue;
        }
        let exists = list.iter().any(|a| a.year == year && a.kind == kind);
        if exists && !replace {
            continue;
        }
        list.retain(|a| !(a.year == year && a.kind == kind));
        list.extend(awards);
    }
    list.sort_by(|a, b| a.year.cmp(&b.year).then(a.kind.cmp(&b.kind)));
}

/// `primeiro`
fn first(world: &World, list: &[&Candidate], kind: AwardKind, year: i32) -> Vec<Award> {
    let Some(c) = list.first() else { return Vec::new() };
    let mut a = award(world, c, kind, year);
    if kind == AwardKind::GoldenShoe {
        a.extra = AwardExtra::WeightedGoals(c.weighted_goals());
    }
    vec![a]
}

/// `premio`
fn award(world: &World, c: &Candidate, kind: AwardKind, year: i32) -> Award {
    let p = world.player(c.player);
    Award {
        year,
        kind,
        key: c.key.clone(),
        name: p.name.clone(),
        club: c.club.map(|k| world.club(k).name.clone()),
        country: p.country,
        position: p.position,
        age: c.age,
        rating: c.rating,
        goals: c.goals,
        assists: c.assists,
        extra: AwardExtra::None,
    }
}
