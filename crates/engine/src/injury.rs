//! Injuries (`F.p(ah)`).

use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;

use crate::events::{record, record_season};
use crate::messages;

/// Port of `F.p(ah)`: days out by age, fitness and bad luck; veterans lose strength. The human
/// coach hears of injuries of eight days or more.
pub fn injure(world: &mut World, runtime: &mut Runtime, player: PlayerId, club: ClubId) {
    let mut days = 0;
    let base = runtime.rng.fresh_int(14);
    let long = 5 + runtime.rng.fresh_int(20);
    let p = world.player(player);
    if p.fitness < 10 {
        days += 5;
    } else if p.fitness < 50 {
        days += 1;
    }
    days = match p.age {
        ..=20 => base,
        ..=25 => days + base + 1,
        ..=30 => days + base + 2,
        ..=35 => days + base + 3,
        ..=45 => days + base + long,
        _ => days + base + 10 + long,
    };
    if p.age >= 35 {
        let p = world.player_mut(player);
        p.strength -= 5;
        if p.strength < 0 {
            p.strength = 1;
        }
    }
    let luck = runtime.rng.fresh_int(100);
    if luck == 1 {
        days += 70;
    } else if luck < 4 {
        days += 40;
    } else if luck < 10 {
        days += 20;
    }
    let human_coach = world.player(player).club.and_then(|c| world.club(c).coach).filter(|&c| world.coach(c).human);
    if let Some(coach) = human_coach
        && days >= 8
    {
        // message ids by length of the injury
        let (from, span) = match days {
            ..15 => (23, 3),
            ..22 => (26, 2),
            ..29 => (28, 2),
            ..43 => (30, 4),
            ..55 => (34, 2),
            ..70 => (36, 4),
            _ => (40, 4),
        };
        let id = runtime.rng.fresh_int(span) + from;
        let name = world.player(player).name.clone();
        messages::send(world, coach, 3, id, name, String::new());
    }
    if days > 0 {
        let today = world.today_millis();
        world.player_mut(player).injured_until = today + days as i64 * 86_400_000;
    }
    record_season(world, player, record::INJURY, club, 0.0, None);
}
