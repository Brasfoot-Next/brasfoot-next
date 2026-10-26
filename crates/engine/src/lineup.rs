//! The AI's line-up (`ah.a(ah, I, int, int, boolean)`) and who may play (`F.a(I, boolean)`).

use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;

use crate::events::competition_stats;
use crate::tables::{BENCH_SLOTS, FORMATIONS, POSITION_FALLBACK, SLOT_ROLE};

/// `F.a(I, boolean)`: fit, not suspended in the match's competition (friendlies ignore
/// suspensions) and, for a human club, under contract unless `ignore_contract`.
/// Looking the suspension up creates the competition's numbers, as `F.h(at)` does.
pub fn available(world: &mut World, player: PlayerId, game_match: Option<MatchId>, ignore_contract: bool) -> bool {
    let competition = game_match.and_then(|m| world.game_match(m).competition);
    let counts_suspensions = competition.is_some_and(|c| world.competition(c).kind != 0);
    let today = world.today_millis();
    if world.player(player).is_injured(today) {
        return false;
    }
    let mut fit = true;
    if counts_suspensions && competition_stats(world, player, competition).is_suspended() {
        fit = false;
    }
    let p = world.player(player);
    let human_club = p.club.is_some_and(|c| world.club(c).human);
    if !ignore_contract && human_club && p.contract_until < today { false } else { fit }
}

/// `h.a.abh`: strength, then fitness, both descending. `Collections.sort` is stable.
pub fn sort_by_strength(world: &World, players: &mut [PlayerId]) {
    players.sort_by(|&a, &b| {
        let (pa, pb) = (world.player(a), world.player(b));
        pb.strength.cmp(&pa.strength).then(pb.fitness.cmp(&pa.fitness))
    });
}

/// `ah.a(ArrayList, int, boolean, boolean)`: the first player who fits a slot, trying the
/// positions of `aq.sH` in order and loosening side and style. The original keeps the side and
/// style it last read across the loops; so does this port.
pub fn pick_for_slot(world: &World, candidates: &[PlayerId], slot: i32, _bench: bool, _keep_keeper: bool) -> Option<PlayerId> {
    let role = SLOT_ROLE[slot as usize];
    let position = role[0];
    let mut side = role[1];
    let mut style = role[2];
    let any_style = style == -1;
    let any_side = side == -1;
    if slot >= 18 {
        style = -1;
    }
    for fallback in 0..=4 {
        let wanted = POSITION_FALLBACK[position as usize][fallback];
        for pass in 1..=3 {
            for &p in candidates {
                let player = world.player(p);
                if any_side {
                    side = player.side;
                }
                if any_style {
                    style = player.style;
                }
                if pass == 2 {
                    side = player.side;
                }
                if pass == 3 {
                    side = player.side;
                    style = player.style;
                }
                if wanted == player.position && side == player.side && style == player.style {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// The formation odds of `ah.a(ah, I, int, int, boolean)`, by a draw from 1 to 100.
fn formation_for(draw: i32) -> usize {
    match draw {
        0..=2 => 1,
        3..=4 => 2,
        5..=7 => 3,
        8..=38 => 4,
        39..=49 => 5,
        50..=60 => 6,
        61..=65 => 7,
        66..=72 => 8,
        73..=90 => 9,
        91..=92 => 10,
        93..=101 => 11,
        _ => 4,
    }
}

/// Port of `ah.a(ah, I, int, int, boolean)`: the AI picks its XI and bench. `side` 1 fills the
/// match's home lists, 2 the away ones; `formation` > 0 forces one; `pick_tactics` also draws
/// style, marking and wings.
pub fn pick_ai_lineup(
    world: &mut World,
    runtime: &mut Runtime,
    club: ClubId,
    game_match: Option<MatchId>,
    side: i32,
    formation: i32,
    pick_tactics: bool,
) {
    let squad = world.club(club).squad.clone();
    let mut candidates = Vec::new();
    for p in squad {
        world.player_mut(p).slot = -1;
        if available(world, p, game_match, false) {
            candidates.push(p);
        }
    }
    sort_by_strength(world, &mut candidates);
    world.club_mut(club).last_starters.clear();
    world.club_mut(club).last_bench.clear();
    let mut chosen = formation_for(runtime.rng.fresh_int(100) + 1);
    if formation > 0 {
        chosen = formation as usize;
    }
    for &slot in &FORMATIONS[chosen] {
        if let Some(p) = pick_for_slot(world, &candidates, slot, false, false) {
            world.player_mut(p).slot = slot;
            world.player_mut(p).started_recently = true;
            world.club_mut(club).last_starters.push(p);
            if let Some(m) = game_match {
                let m = world.game_match_mut(m);
                match side {
                    1 => {
                        m.home_starters.push(p);
                        m.home_played.push(p);
                    }
                    2 => {
                        m.away_starters.push(p);
                        m.away_played.push(p);
                    }
                    _ => {}
                }
            }
            candidates.retain(|&c| c != p);
        }
    }
    for &slot in &BENCH_SLOTS {
        if let Some(p) = pick_for_slot(world, &candidates, slot, true, false) {
            world.club_mut(club).last_bench.push(p);
            if let Some(m) = game_match {
                let m = world.game_match_mut(m);
                match side {
                    1 => m.home_bench.push(p),
                    2 => m.away_bench.push(p),
                    _ => {}
                }
            }
            candidates.retain(|&c| c != p);
        }
    }
    world.club_mut(club).lineup_set = true;
    if pick_tactics {
        let style = match runtime.rng.fresh_int(100) + 1 {
            0..=70 => 0,
            71..=80 => 1,
            _ => 2,
        };
        let marking = match runtime.rng.fresh_int(100) + 1 {
            1..=5 => 2,
            6..=70 => 0,
            _ => 1,
        };
        let wings = if runtime.rng.fresh_int(100) + 1 <= 70 { 0 } else { 1 };
        world.club_mut(club).tactics = vec![chosen as i32, style, marking, wings];
    }
}
