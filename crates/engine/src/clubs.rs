//! Club getters of `best.ah` that the match uses. Several fix the club as they read it.

use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::world::World;

/// `ah.lp()`: the club is a national team (some country's `jn()`).
pub fn is_national_team(world: &World, club: ClubId) -> bool {
    world.game.countries.iter().any(|&c| world.country(c).national_team == Some(club))
}

/// The pattern of `ah.kd()`, `ke()`, `lq()` and `lr()`: a designated player who left the club is
/// forgotten, except by national teams, which keep him while he is in their squad.
fn designated(world: &mut World, club: ClubId, get: fn(&mut brasfoot_model::saved::Club) -> &mut Option<PlayerId>) -> Option<PlayerId> {
    let player = *get(world.club_mut(club));
    if let Some(p) = player
        && world.player(p).club != Some(club)
    {
        let keep = is_national_team(world, club) && world.club(club).squad.contains(&p);
        if !keep {
            *get(world.club_mut(club)) = None;
        }
    }
    *get(world.club_mut(club))
}

/// `ah.kd()`
pub fn captain(world: &mut World, club: ClubId) -> Option<PlayerId> {
    designated(world, club, |c| &mut c.captain)
}

/// `ah.ke()`
pub fn penalty_taker(world: &mut World, club: ClubId) -> Option<PlayerId> {
    designated(world, club, |c| &mut c.penalty_taker)
}

/// `ah.lq()`
pub fn free_kick_taker(world: &mut World, club: ClubId) -> Option<PlayerId> {
    designated(world, club, |c| &mut c.free_kick_taker)
}

/// `ah.lr()`
pub fn corner_taker(world: &mut World, club: ClubId) -> Option<PlayerId> {
    designated(world, club, |c| &mut c.corner_taker)
}
