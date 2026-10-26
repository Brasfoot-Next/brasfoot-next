//! Attendance and gate income of a match (`best.v`).

use brasfoot_model::countries;
use brasfoot_model::ids::{ClubId, MatchId};
use brasfoot_model::rng::java_round;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::saved::{Stadium, StageKind};
use brasfoot_model::world::World;

/// `aq.qK`: default ticket prices of the four sectors, by competition kind and then by division,
/// reputation or confederation (see [`default_prices`]).
const DEFAULT_PRICES: [&[[i32; 4]]; 9] = [
    &[[5, 15, 20, 30], [3, 12, 15, 25]],
    &[[3, 12, 15, 30], [10, 15, 25, 80], [7, 13, 20, 70], [5, 12, 17, 40], [3, 12, 15, 30]],
    &[[3, 12, 15, 30], [3, 12, 15, 30], [3, 12, 15, 30], [7, 13, 20, 70], [10, 15, 25, 80], [10, 15, 25, 80]],
    &[[3, 5, 12, 20], [3, 12, 15, 30], [3, 12, 15, 30], [5, 12, 20, 50], [10, 15, 25, 70], [10, 15, 25, 70]],
    &[[30, 45, 65, 200], [20, 35, 55, 150]],
    &[[0, 0, 0, 0]],
    &[[20, 25, 45, 150], [20, 25, 40, 120]],
    &[[0, 0, 0, 0]],
    &[[20, 25, 45, 150], [20, 25, 40, 120]],
];

/// `v.a(int, ah)`: the default prices of a club's ground for a competition kind.
pub fn default_prices(world: &World, kind: i32, club: ClubId) -> [i32; 4] {
    let kind = if kind == 10 { 2 } else { kind };
    let c = world.club(club);
    let confederation = countries::confederation(c.country).min(1);
    match kind {
        1 => DEFAULT_PRICES[1][c.division as usize],
        2 | 3 => DEFAULT_PRICES[kind as usize][c.reputation as usize],
        4 | 6 | 8 => DEFAULT_PRICES[kind as usize][confederation as usize],
        5 | 7 => DEFAULT_PRICES[1][c.division as usize],
        _ => [10, 25, 35, 50],
    }
}

fn scaled(value: i32, factor: f64) -> i32 {
    java_round(value as f64 * factor) as i32
}

/// Port of `v.b(I)`: the match's attendance per sector and its gate income.
pub fn play_gate(world: &mut World, runtime: &mut Runtime, ground: &Stadium, game_match: MatchId) {
    let m = world.game_match(game_match);
    let home = m.home.expect("home club");
    let away = m.away.expect("away club");
    let competition = world.competition(m.competition.expect("a match with a competition"));
    let (kind, subkind) = (competition.kind, competition.subkind);
    let knockout = m.stage.is_some_and(|s| world.stage(s).class == StageKind::Knockout);
    let prices_for_kind = default_prices(world, kind, home);
    let capacity = ground.capacity.clone();
    const BASE: [[i32; 4]; 6] =
        [[200, 500, 50, 0], [1000, 5000, 1200, 20], [2000, 10000, 1500, 50], [4000, 20000, 2500, 300], [4500, 30000, 3500, 400], [5000, 40000, 5500, 500]];
    let home_club = world.club(home);
    let division = home_club.division as usize;
    let mut crowd = BASE[home_club.reputation as usize];
    if kind == 3 || kind == 10 {
        crowd = crowd.map(|v| scaled(v, 0.7));
    }
    if kind == 0 {
        crowd = crowd.map(|v| scaled(v, 0.4));
    }
    let mut share = 0.3;
    if knockout {
        share += 0.15;
    }
    if kind == 4 || kind == 7 || kind == 9 {
        share += 0.3;
    } else if kind == 6 || kind == 8 || kind == 11 {
        share += 0.15;
    }
    for i in 0..4 {
        crowd[i] += scaled(capacity[i], share);
    }
    let gap = world.club(away).reputation - world.club(home).reputation;
    let factor = [0.0, 0.05, 0.1, 0.15, 0.2, 0.25][gap.unsigned_abs() as usize];
    for v in crowd.iter_mut() {
        if gap > 0 {
            *v += scaled(*v, factor);
        } else if gap < 0 {
            *v -= scaled(*v, factor);
        }
    }
    let support = world.club(home).coach.map(|c| world.coach(c).fan_support).unwrap_or(80);
    let support = support as f64 / 100.0;
    for v in crowd.iter_mut() {
        *v = scaled(*v, support);
    }
    const NOISE: [[i32; 4]; 5] = [[10, 20, 5, 0], [100, 500, 200, 10], [300, 1000, 400, 20], [400, 1200, 500, 30], [500, 1500, 1000, 50]];
    for i in 0..4 {
        let bound = NOISE[division][i];
        if bound > 0 {
            crowd[i] += runtime.rng.fresh_int(bound);
        }
    }
    let prices: [i32; 4] = if ground.automatic_prices {
        default_prices(world, kind, home)
    } else {
        let prices = [ground.ticket_prices[0], ground.ticket_prices[1], ground.ticket_prices[2], ground.ticket_prices[3]];
        for i in 0..4 {
            let difference = prices_for_kind[i] - prices[i];
            if difference != 0 {
                crowd[i] += (java_round(0.03 * crowd[i] as f64) * difference as i64) as i32;
            }
        }
        prices
    };
    let mut income = 0;
    for i in 0..4 {
        crowd[i] = crowd[i].max(0).min(capacity[i]);
        income += crowd[i] * prices[i];
    }
    if kind == 7 {
        income *= if subkind != 7 { 3 } else { 5 };
    } else if kind == 9 {
        income *= 2;
    }
    let m = world.game_match_mut(game_match);
    m.attendance = crowd.to_vec();
    m.income = income;
}
