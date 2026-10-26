// Port of the new contract screen a.fy: the salary limit (rF), whether the player accepts
// (dr), the offer (rE) and the signature (dq).

use brasfoot_model::ids::PlayerId;
use brasfoot_model::world::World;

/// The durations of the combo, in days ("6 meses", "1 ano", "2 anos", "3 anos").
pub const DURATIONS: [i64; 4] = [180, 365, 730, 1095];
/// The salary cut a player near the end of his contract accepts, by duration (%).
const CUT_ACCEPTED: [i32; 4] = [1, 3, 5, 12];
/// The raise any other player asks, by duration (%).
const RAISE_ASKED: [i32; 4] = [10, 12, 15, 5];

/// What the player says to an offer (`fy.rE()`).
#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    /// He signed.
    Signed,
    /// He wants this salary, at most the limit; the original asks "Salário recusado" and
    /// signs it on "Sim".
    Asks(i32),
    /// The salary is at the limit or above it (`rF()`).
    OverLimit(i32),
    /// No salary: nothing happens.
    Nothing,
}

/// `fy.rF()`: a quarter of the market value.
pub fn salary_limit(world: &World, p: PlayerId) -> i32 {
    brasfoot_model::rng::java_round(world.player(p).market_value as f64 * 0.25) as i32
}

/// `fy.dr(int)`: the least the player accepts, a cut if his contract ends within 60 days
/// (`F.fR()`), a raise otherwise.
fn least_accepted(world: &World, p: PlayerId, duration: usize) -> i32 {
    let player = world.player(p);
    let salary = player.salary;
    if player.contract_days_left(world.today_millis()) < 60 {
        salary - salary.wrapping_mul(CUT_ACCEPTED[duration]) / 100
    } else {
        salary + salary.wrapping_mul(RAISE_ASKED[duration]) / 100
    }
}

/// `fy.rE()`: offers `salary` for `duration` (an index of [`DURATIONS`]).
pub fn offer(world: &mut World, p: PlayerId, salary: i32, duration: usize) -> Answer {
    let limit = salary_limit(world, p);
    if salary > 0 && salary < limit {
        let least = least_accepted(world, p, duration);
        if salary >= least {
            sign(world, p, salary, duration);
            Answer::Signed
        } else {
            Answer::Asks(least.min(limit))
        }
    } else if salary >= limit {
        Answer::OverLimit(limit)
    } else {
        Answer::Nothing
    }
}

/// `fy.dq(int)`: the new contract runs `duration` from the end of the current one
/// (`F.a(long, false)`), at `salary`; the price goes back to the market value (`F.fm()`).
pub fn sign(world: &mut World, p: PlayerId, salary: i32, duration: usize) {
    let player = world.player_mut(p);
    player.price = player.market_value;
    player.contract_until += DURATIONS[duration] * 86_400_000;
    // F.c(false), F.aB(int) and F.fr are transient (the contract screen's state)
    player.salary = salary;
}
