// Port of bfmod.Diretor: the director of football's targets for the weakest kind of slot of the
// player's line-up that his cash can pay, judged by what the scouts see they would be worth there
// on the pitch (Motor, Olheiros), with the chance each accepts by the clubs' reputations. The
// window's report in the newspaper is not ported.

use brasfoot_engine::tactics::{TYPICAL_SLOT, is_starter};
use brasfoot_engine::value::{mod_value, slot_kind};
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::world::World;

use crate::Mod;
use crate::economy::{is_human, is_national_team};
use crate::scouts::estimate_in;
use crate::staff::my_club;

/// `Motor.ATACANTE`
const FORWARD_KIND: usize = 6;

/// `Diretor.Sugestao`
#[derive(Clone, Debug)]
pub struct Target {
    pub player: PlayerId,
    pub club: ClubId,
    pub price: i64,
    pub chance: i32,
    /// The slot he is wanted for.
    pub slot: i32,
}

/// `Diretor.tipoAlvo(ah)`: the kind of slot where today's line-up is worth the least.
pub fn target_kind(world: &World, club: ClubId) -> usize {
    let individual = world.game.individual_skills;
    let (mut total, mut count) = ([0.0f64; 8], [0usize; 8]);
    for &p in &world.club(club).squad {
        if is_starter(world, p) {
            let slot = world.player(p).slot;
            let k = slot_kind(slot);
            total[k] += mod_value(individual, world.player(p), slot) as f64;
            count[k] += 1;
        }
    }
    let mut worst: Option<usize> = None;
    for k in 0..8 {
        if count[k] > 0 && worst.is_none_or(|w| total[k] / (count[k] as f64) < total[w] / count[w] as f64) {
            worst = Some(k);
        }
    }
    worst.unwrap_or(FORWARD_KIND)
}

/// `Diretor.mediaNoTipo(ah, int)`
fn kind_average(world: &World, club: ClubId, kind: usize) -> f64 {
    let individual = world.game.individual_skills;
    let (mut total, mut n) = (0.0, 0);
    for &p in &world.club(club).squad {
        let slot = world.player(p).slot;
        if is_starter(world, p) && slot_kind(slot) == kind {
            total += mod_value(individual, world.player(p), slot) as f64;
            n += 1;
        }
    }
    if n > 0 { total / n as f64 } else { 0.0 }
}

/// `Diretor.chance(F, ah, ah)`: players move up, rarely down.
pub fn chance(world: &World, p: PlayerId, seller: ClubId, buyer: ClubId) -> i32 {
    let gap = world.club(buyer).reputation - world.club(seller).reputation;
    let mut chance = match gap {
        1.. => 85,
        0 => 65,
        -1 => 35,
        -2 => 15,
        _ => 5,
    };
    let x = world.player(p);
    if x.world_class {
        chance -= 20;
    } else if x.star {
        chance -= 10;
    }
    chance.clamp(5, 95)
}

impl Mod<'_> {
    /// `Diretor.sugestoes(int)`: the best players first, then the likelier to come.
    pub fn director_targets(&mut self, limit: usize) -> Vec<Target> {
        let Some(mine) = my_club(self.world) else { return Vec::new() };
        let kind = target_kind(self.world, mine);
        let slot = TYPICAL_SLOT[kind];
        let position = brasfoot_engine::tables::SLOT_ROLE[slot as usize][0];
        let minimum = brasfoot_model::rng::java_round(kind_average(self.world, mine, kind) + 3.0) as i32;
        let cash = self.world.club(mine).cash;
        let mut out = Vec::new();
        for p in self.world.game.players.clone() {
            let x = self.world.player(p);
            let Some(owner) = x.club else { continue };
            if owner == mine || is_national_team(self.world, owner) || is_human(self.world, owner) || x.on_loan {
                continue;
            }
            // the director sees what the scouts see, not the hidden true strength
            if x.position != position || estimate_in(self.world, p, slot) < minimum || x.age < 18 || x.age > 31 {
                continue;
            }
            let price = self.price(p, owner, None);
            if price as f64 > cash as f64 * 0.8 {
                continue;
            }
            let chance = chance(self.world, p, owner, mine);
            if chance >= 10 {
                out.push(Target { player: p, club: owner, price, chance, slot });
            }
        }
        let world = &*self.world;
        out.sort_by(|a, b| estimate_in(world, b.player, b.slot).cmp(&estimate_in(world, a.player, a.slot)).then(b.chance.cmp(&a.chance)));
        out.truncate(limit);
        out
    }
}
