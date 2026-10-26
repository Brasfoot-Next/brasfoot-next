// Port of the substitutions window a.cD: its grid of a side (oY(), dd(int), de(int)), a player
// moved on the pitch or swapped with the bench (H(int, int), K(int, int)) and the formation shown
// (V(ArrayList)).

use brasfoot_engine::simulate::{Play, event_kind};
use brasfoot_model::ids::{MatchId, PlayerId};
use brasfoot_model::world::World;

use crate::Sim;

/// `cD.AE` / `cD.Dc`: 37 places of a side, 1 to 25 the pitch, 26 to 36 the bench.
pub const MATCH_PLACES: usize = 37;
pub const FIRST_MATCH_BENCH: usize = 26;

/// The grid of one side of a match in the window.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchGrid {
    pub game_match: MatchId,
    /// 0 home, 1 away (`cD.Di` - 1).
    pub side: usize,
    pub places: Vec<Option<PlayerId>>,
}

/// `cD.dd(int)`: the side's starters with the substitutions shown so far.
fn on_pitch_now(world: &World, m: MatchId, side: usize) -> Vec<PlayerId> {
    let g = world.game_match(m);
    let club = if side == 0 { g.home } else { g.away };
    let mut list = if side == 0 { g.home_starters.clone() } else { g.away_starters.clone() };
    for e in &g.events {
        if e.done && e.kind == event_kind::SUBSTITUTION && e.club == club {
            if let Some(out) = e.player
                && let Some(i) = list.iter().position(|&p| p == out)
            {
                list.remove(i);
            }
            if let Some(came) = e.other_player {
                list.push(came);
            }
        }
    }
    list
}

/// `cD.oY()`: the grid of a side. A match with a human club shows the players on the pitch
/// now (`hp()`, `hq()`) and its bench; the others are rebuilt from the starters and the
/// substitutions shown (`dd`), with the bench and those who came on (`de`).
pub fn match_grid(world: &World, m: MatchId, side: usize) -> MatchGrid {
    let g = world.game_match(m);
    let human = [g.home, g.away].into_iter().flatten().any(|c| world.club(c).human);
    let (pitch, bench) = if human {
        let pitch = if side == 0 { g.home_played.clone() } else { g.away_played.clone() };
        let bench = if side == 0 { g.home_bench.clone() } else { g.away_bench.clone() };
        (pitch, bench)
    } else {
        let mut bench = if side == 0 { g.home_bench.clone() } else { g.away_bench.clone() };
        bench.extend(if side == 0 { g.home_list_fl.iter() } else { g.away_list_fm.iter() });
        (on_pitch_now(world, m, side), bench)
    };
    let mut places = vec![None; MATCH_PLACES];
    for &p in &pitch {
        let slot = world.player(p).slot;
        if (1..=25).contains(&slot) {
            places[slot as usize] = Some(p);
        }
    }
    let mut next = FIRST_MATCH_BENCH;
    for &p in &bench {
        if next < MATCH_PLACES && !pitch.contains(&p) {
            places[next] = Some(p);
            next += 1;
        }
    }
    MatchGrid { game_match: m, side, places }
}

/// `cD.V(ArrayList)`: the formation "d-m-a" of the places 1-9, 10-17 and 18-25, the keeper
/// not counted.
pub fn formation(grid: &MatchGrid) -> [i32; 3] {
    let mut lines = [-1, 0, 0];
    for (i, place) in grid.places.iter().enumerate() {
        if place.is_some() {
            if i <= 9 {
                lines[0] += 1;
            } else if i <= 17 {
                lines[1] += 1;
            } else if i <= 25 {
                lines[2] += 1;
            }
        }
    }
    lines
}

impl Sim<'_> {
    /// `cD.H(int, int)` with `cD.K(int, int)`: a player of the human side dragged from one place
    /// to another. On the pitch he changes place (`F.as`); between the pitch and the bench,
    /// both taken, it is a substitution while there are substitutions left, at the screen's
    /// minute (1 at half time). False when nothing changed.
    pub fn move_in_match(&mut self, grid: &mut MatchGrid, from: usize, to: usize, half: i32, minute: i32, half_time: bool) -> bool {
        if from == 0 || to == 0 || from >= MATCH_PLACES || to >= MATCH_PLACES {
            return false;
        }
        let shown = |i: usize| grid.places[i].is_some();
        let (mut swap, mut substitute) = (false, false);
        if from > 1 && from <= 25 && to <= 25 {
            swap = true;
        } else if from == 1 && to < 25 {
            swap = shown(from) && shown(to);
        } else if from <= 25 && to > 25 {
            substitute = shown(from) && shown(to);
        } else if from > 25 && to <= 25 {
            substitute = shown(from) && shown(to);
        }
        if substitute {
            // K(): the place on the pitch is the lower index
            let (pitch, bench) = if from < to { (from, to) } else { (to, from) };
            let (Some(leaving), Some(coming)) = (grid.places[pitch], grid.places[bench]) else { return false };
            let minute = if half_time { 1 } else { minute };
            let done = Play { world: self.world, runtime: self.runtime, hooks: self.hooks, game_match: grid.game_match }
                .human_substitution(grid.side, leaving, coming, half, minute);
            if done {
                grid.places[pitch] = Some(coming);
                grid.places[bench] = None;
            }
            return done;
        }
        if swap {
            grid.places.swap(from, to);
            for i in [from, to] {
                if let Some(p) = grid.places[i] {
                    self.world.player_mut(p).slot = i as i32;
                }
            }
            return true;
        }
        false
    }
}
