// Port of the game's quick bank loan in best.C (the finances screen a.cW): 500 thousand at a
// time up to a limit by division (C.m), paid back 500 thousand at a time (C.l), at 3% a month
// (C.eP).

use brasfoot_model::ids::ClubId;
use brasfoot_model::world::World;

/// The most a club can owe (`C.m`): by its division, the first when outside 1 to 4.
pub fn emergency_limit(world: &World, club: ClubId) -> i32 {
    const LIMITS: [i32; 5] = [1_000_000, 5_000_000, 3_000_000, 2_000_000, 1_500_000];
    let d = world.club(club).division;
    if (1..=4).contains(&d) { LIMITS[d as usize] } else { LIMITS[0] }
}

/// `C.eP()`: the monthly interest is 3% of the debt.
fn update_interest(world: &mut World, club: ClubId) {
    let f = &mut world.club_mut(club).finances;
    f.bank_interest = if f.bank_loan > 0 { (f.bank_loan * 3 / 100) as f32 as i32 } else { 0 };
}

/// `C.m(ah)`: 500 thousand more of debt, into the cash; false at the limit.
pub fn borrow_emergency(world: &mut World, club: ClubId) -> bool {
    if world.club(club).finances.bank_loan >= emergency_limit(world, club) {
        return false;
    }
    world.club_mut(club).finances.bank_loan += 500_000;
    world.club_income(club, 500_000, -1);
    update_interest(world, club);
    true
}

/// `C.l(ah)`: 500 thousand of debt paid; false without debt or cash.
pub fn repay_emergency(world: &mut World, club: ClubId) -> bool {
    if world.club(club).finances.bank_loan > 0 && world.club(club).cash >= 500_000 {
        world.club_mut(club).finances.bank_loan -= 500_000;
        world.club_payment(club, 500_000, -1);
        update_interest(world, club);
        true
    } else {
        false
    }
}
