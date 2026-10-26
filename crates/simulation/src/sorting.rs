//! The comparators of `h.a` the simulation sorts with. `Collections.sort` is stable, like
//! `slice::sort_by`.

use std::cmp::Ordering;

use brasfoot_model::ids::{ClubId, StageId};
use brasfoot_model::world::World;

use crate::clubs::stage_record_numbers;

/// `h.K` (`h.a.abn`): a stage's table: points, wins, goal difference, goals for. The original
/// reads the stage from the static `f.s.Zl`; here it is an argument.
pub fn standings(world: &World, stage: StageId, a: ClubId, b: ClubId) -> Ordering {
    let x = stage_record_numbers(world, a, stage);
    let y = stage_record_numbers(world, b, stage);
    y[0].cmp(&x[0]).then(y[2].cmp(&x[2])).then(y[7].cmp(&x[7])).then(y[5].cmp(&x[5]))
}

/// `h.D` (`h.a.cN`): level, then the club's tiebreak number, both descending.
pub fn by_level(world: &World, a: ClubId, b: ClubId) -> Ordering {
    let (x, y) = (world.club(a), world.club(b));
    y.level.cmp(&x.level).then(y.tiebreak_number.cmp(&x.tiebreak_number))
}

/// `best.g` (`f.bO`): coaches by points, wins, then fewer games.
pub fn coaches(world: &World, a: brasfoot_model::ids::CoachId, b: brasfoot_model::ids::CoachId) -> Ordering {
    let (x, y) = (world.coach(a), world.coach(b));
    y.points.cmp(&x.points).then(y.wins.cmp(&x.wins)).then(x.games.cmp(&y.games))
}

/// `h.J` (`h.a.abm`): level, reputation, then the tiebreak number, all descending.
pub fn by_level_and_reputation(world: &World, a: ClubId, b: ClubId) -> Ordering {
    let (x, y) = (world.club(a), world.club(b));
    y.level.cmp(&x.level).then(y.reputation.cmp(&x.reputation)).then(y.tiebreak_number.cmp(&x.tiebreak_number))
}
