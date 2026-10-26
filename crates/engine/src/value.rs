//! What the engine sees of a player.

use brasfoot_model::countries;
use brasfoot_model::ids::{MatchId, PlayerId};
use brasfoot_model::player::attribute::*;
use brasfoot_model::rng::java_round;
use brasfoot_model::saved::Player;
use brasfoot_model::world::World;

use crate::tables::SLOT_ROLE;

/// `Math.round(x * w)` as the engine sums attribute shares.
fn share(value: i32, weight: f64) -> i32 {
    java_round(value as f64 * weight) as i32
}

/// Port of `F.gF()`: playing out of his position in his slot. The original's `|| position == 3`
/// can never change the result; kept as written.
pub fn out_of_position(player: &Player) -> bool {
    let slot = player.slot;
    if slot <= 0 {
        return true;
    }
    if slot <= 25 && SLOT_ROLE[slot as usize][0] != player.position {
        return if slot != 10 && slot != 17 { true } else { player.position != 1 || player.position == 3 };
    }
    false
}

/// The individual-skills mix of attributes for a slot (`c.b.B`), before the out-of-position cut.
pub fn attribute_mix(player: &Player, slot: i32) -> i32 {
    let a = |i: usize| player.attributes[i];
    match slot {
        1 => share(a(GOALKEEPING), 0.6) + share(a(TECHNIQUE), 0.15) + share(a(SPEED), 0.15) + share(a(PASSING), 0.1),
        3..=8 => {
            share(a(DEFENDING), 0.5)
                + share(a(TECHNIQUE), 0.1)
                + share(a(SPEED), 0.25)
                + share(a(PASSING), 0.1)
                + share(a(PLAYMAKING), 0.05)
        }
        2 | 9 => {
            share(a(DEFENDING), 0.4)
                + share(a(SPEED), 0.1)
                + share(a(TECHNIQUE), 0.1)
                + share(a(PASSING), 0.3)
                + share(a(PLAYMAKING), 0.05)
                + share(a(FINISHING), 0.05)
        }
        11..=13 => {
            share(a(DEFENDING), 0.4)
                + share(a(SPEED), 0.15)
                + share(a(TECHNIQUE), 0.1)
                + share(a(PASSING), 0.2)
                + share(a(PLAYMAKING), 0.1)
                + share(a(FINISHING), 0.05)
        }
        14..=16 => {
            share(a(DEFENDING), 0.05)
                + share(a(SPEED), 0.1)
                + share(a(TECHNIQUE), 0.1)
                + share(a(PASSING), 0.25)
                + share(a(PLAYMAKING), 0.4)
                + share(a(FINISHING), 0.1)
        }
        10 | 17 => {
            share(a(DEFENDING), 0.05)
                + share(a(SPEED), 0.25)
                + share(a(TECHNIQUE), 0.15)
                + share(a(PASSING), 0.25)
                + share(a(PLAYMAKING), 0.2)
                + share(a(FINISHING), 0.1)
        }
        19..=24 => {
            share(a(SPEED), 0.25)
                + share(a(TECHNIQUE), 0.25)
                + share(a(PASSING), 0.05)
                + share(a(PLAYMAKING), 0.05)
                + share(a(FINISHING), 0.4)
        }
        18 | 25 => {
            share(a(SPEED), 0.25)
                + share(a(TECHNIQUE), 0.15)
                + share(a(PASSING), 0.15)
                + share(a(PLAYMAKING), 0.05)
                + share(a(FINISHING), 0.4)
        }
        _ => 0,
    }
}

/// Port of `c.b.B(F)`: the player's value in the match (0.1 to about 10), with the cuts the
/// original gives by competition and club reputation. `game_match` is None outside a match.
pub fn engine_value(world: &World, game_match: Option<MatchId>, player_id: PlayerId) -> f64 {
    let player = world.player(player_id);
    let mut value = if world.game.individual_skills { attribute_mix(player, player.slot) } else { player.strength };
    if out_of_position(player) {
        value = share(value, 0.5);
    }
    if value <= 0 {
        value = 1;
    }
    let Some(match_id) = game_match else { return value as f64 / 10.0 };
    let m = world.game_match(match_id);
    let kind = m.competition.map(|c| world.competition(c).kind).unwrap_or(0);
    let home = m.home.map(|c| world.club(c));
    let away = m.away.map(|c| world.club(c));
    let own_club = player.club.map(|c| world.club(c));
    let cut = |value: i32, factor: f64| share(value, factor);
    if kind == 7 {
        let by_reputation = |value: i32, reputation: i32| match reputation {
            r if r < 3 => cut(value, 0.65),
            3 => cut(value, 0.85),
            4 => cut(value, 0.95),
            _ => value,
        };
        if let Some(h) = home.filter(|h| player.country == h.country) {
            value = by_reputation(value, h.reputation);
        } else if let Some(a) = away.filter(|a| player.country == a.country) {
            value = by_reputation(value, a.reputation);
        }
    }
    if kind == 4 {
        if let Some(c) = own_club {
            if c.reputation < 3 {
                value = cut(value, 0.75);
            } else if c.reputation == 3 {
                value = cut(value, 0.85);
            } else if c.country == 29 {
                value = cut(value, 0.9);
            }
        }
    }
    if kind == 5 {
        if let Some(c) = own_club {
            if c.reputation < 3 {
                value = cut(value, 0.55);
            } else if c.reputation == 3 {
                value = cut(value, 0.75);
            } else if countries::confederation(c.country) != 0 {
                value = cut(value, 0.9);
            }
        }
    }
    if kind == 1 {
        if let Some(c) = own_club {
            if c.reputation < 3 {
                value = cut(value, 0.85);
            } else if c.reputation == 3 {
                value = cut(value, 0.95);
            }
        }
    }
    if (kind == 3 || kind == 2)
        && let (Some(h), Some(a)) = (home, away)
        && h.reputation < 3
        && a.reputation >= 3
        && player.club.is_some()
        && player.club == m.away
    {
        value = cut(value, 0.8);
    }
    value as f64 / 10.0
}

/// Port of the mod's `Motor.valor(F, int)`: the value in any slot, 1 to about 100.
pub fn mod_value(individual_skills: bool, player: &Player, slot: i32) -> i32 {
    let mut value = player.strength;
    if individual_skills {
        value = mod_attribute_mix(player, slot);
    }
    if out_of_position_in(player, slot) {
        value = share(value, 0.5);
    }
    value.max(1)
}

/// `Motor.tipo(int)`: the kind of a slot.
pub fn slot_kind(slot: i32) -> usize {
    match slot {
        1 => 0,
        3..=8 => 1,
        2 | 9 => 2,
        10 | 17 => 3,
        11..=13 => 4,
        14..=16 => 5,
        18 | 25 => 7,
        _ => 6,
    }
}

/// `Motor.PESOS`, in the order Gol, Des, Arm, Fin, Vel, Tec, Pas.
const MOD_WEIGHTS: [[f64; 7]; 8] = [
    [0.6, 0.0, 0.0, 0.0, 0.15, 0.15, 0.1],
    [0.0, 0.5, 0.05, 0.0, 0.25, 0.1, 0.1],
    [0.0, 0.4, 0.05, 0.05, 0.1, 0.1, 0.3],
    [0.0, 0.05, 0.2, 0.1, 0.25, 0.15, 0.25],
    [0.0, 0.4, 0.1, 0.05, 0.15, 0.1, 0.2],
    [0.0, 0.05, 0.4, 0.1, 0.1, 0.1, 0.25],
    [0.0, 0.0, 0.05, 0.4, 0.25, 0.25, 0.05],
    [0.0, 0.0, 0.05, 0.4, 0.25, 0.15, 0.15],
];

fn mod_attribute_mix(player: &Player, slot: i32) -> i32 {
    let a = &player.attributes;
    let values = [a[GOALKEEPING], a[DEFENDING], a[PLAYMAKING], a[FINISHING], a[SPEED], a[TECHNIQUE], a[PASSING]];
    let weights = MOD_WEIGHTS[slot_kind(slot)];
    values.iter().zip(weights).map(|(&v, w)| share(v, w)).sum()
}

/// `Motor.foraDePosicao(F, int)`: `F.gF()` for any slot.
pub fn out_of_position_in(player: &Player, slot: i32) -> bool {
    let position = SLOT_ROLE[slot.clamp(0, 25) as usize][0];
    if position == player.position {
        return false;
    }
    slot != 10 && slot != 17 || player.position != 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goalkeeper_mix_and_out_of_position() {
        let mut p = Player { position: 0, slot: 1, attributes: [80, 40, 30, 50, 10, 10, 10], ..Default::default() };
        // 48 + round(4.5)=5 (Java rounds half up) + 6 + 5
        assert_eq!(attribute_mix(&p, 1), 48 + 5 + 6 + 5);
        assert!(!out_of_position(&p));
        p.slot = 20;
        assert!(out_of_position(&p));
        p.slot = 0;
        assert!(out_of_position(&p));
    }

    #[test]
    fn wing_back_slots_accept_full_backs() {
        let p = Player { position: 1, slot: 10, ..Default::default() };
        assert!(!out_of_position(&p));
        assert!(!out_of_position_in(&p, 17));
        assert!(out_of_position_in(&p, 4));
    }
}
