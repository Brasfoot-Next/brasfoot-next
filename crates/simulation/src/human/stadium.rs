// Port of the game's stadium expansion a.cq: the ceiling of each sector (cq.Co), the quote
// (oH: cq.I per sector, the fixed 100 000 and cq.cZ days) and the start of the works (oI).

use brasfoot_model::clock::Clock;
use brasfoot_model::ids::ClubId;
use brasfoot_model::saved::StadiumWork;
use brasfoot_model::world::World;

/// `best.aq.pa`: the most seats per sector (general, stands, seats, boxes) by stage.
const CEILINGS: [[i32; 4]; 4] = [[18_000, 80_000, 9_000, 700], [20_000, 100_000, 10_000, 800], [22_000, 110_000, 10_000, 900], [25_000, 120_000, 10_000, 1_200]];

/// `cq.Co`: the original tests `H() >= 2` first, so from the second season on it is always
/// `pa[1]` and the last two are never reached (a bug the mod's `Patrimonio` fixes; kept here).
pub fn ceilings(world: &World) -> [i32; 4] {
    if world.season() >= 2 { CEILINGS[1] } else { CEILINGS[0] }
}

/// `cq.I(int, int)`: the price per seat by the size the sector reaches.
fn sector_cost(sector: usize, current: i32, add: i32) -> i32 {
    const PRICES: [[i32; 5]; 4] = [[80, 160, 240, 500, 700], [120, 380, 640, 700, 1400], [300, 600, 750, 800, 1200], [1500, 3500, 4000, 6000, 6400]];
    const BRACKETS: [[i32; 5]; 4] = [[1000, 2500, 3500, 10000, 18000], [5000, 15000, 30000, 60000, 80000], [1000, 2000, 3000, 5000, 9000], [100, 200, 500, 600, 700]];
    let target = current + add;
    let bracket = BRACKETS[sector].iter().position(|&b| target <= b).unwrap_or(4);
    PRICES[sector][bracket].wrapping_mul(add)
}

/// `cq.cZ(int)`: the works' days.
pub fn days(seats: i32) -> i32 {
    if seats < 1000 {
        15
    } else if seats < 10_000 {
        20
    } else if seats < 30_000 {
        30
    } else {
        40
    }
}

/// `cq.oH()`: the works' cost and end, or `None` when no seat was asked.
pub struct Quote {
    pub cost: i32,
    pub until: i64,
    pub seats: [i32; 4],
}

pub fn quote(world: &World, clock: &Clock, current: &[i32], add: [i32; 4]) -> Option<Quote> {
    if add.iter().sum::<i32>() == 0 {
        return None;
    }
    let mut cost = 0i32;
    for (sector, &n) in add.iter().enumerate() {
        cost = cost.wrapping_add(sector_cost(sector, current.get(sector).copied().unwrap_or(0), n));
    }
    cost = cost.wrapping_add(100_000);
    // dI: a Calendar of now (the clock's time of day) set to the current day, plus the days
    let f = clock.fields(world.today_millis());
    let start = clock.date_now(f.year, f.month, f.day);
    Some(Quote { cost, until: clock.add_days(start, days(add.iter().sum())), seats: add })
}

/// `cq.oI()`: the works start when the club has the money (`kb() < Ct` refuses).
pub fn start(world: &mut World, club: ClubId, q: &Quote) -> bool {
    let Some(stadium) = world.club(club).stadium else { return false };
    if world.club(club).cash < q.cost as i64 {
        return false;
    }
    world.game.stadium_works.push(StadiumWork { stadium: Some(stadium), until: q.until, seats: q.seats.to_vec() });
    world.club_payment(club, q.cost, 7);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Worked out by hand from `cq.I`: 2 000 seats of stands from 4 000 reach 6 000, the second
    /// bracket (380 each); 50 boxes from 100 reach 150 (3 500 each).
    #[test]
    fn sector_costs_follow_the_brackets() {
        assert_eq!(sector_cost(1, 4000, 2000), 760_000);
        assert_eq!(sector_cost(3, 100, 50), 175_000);
        assert_eq!(sector_cost(0, 20_000, 10), 7_000);
        assert_eq!(days(999), 15);
        assert_eq!(days(30_000), 40);
    }
}
