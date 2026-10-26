// Port of the line-up screen a.bz: what opening it does to the club (ah.kA(), ah.kz()), who can
// be picked (bz.U(boolean)), the grid (bz.lA(), bz.oe(), bz.a(ag, boolean)), a formation of the
// combo (bz.cU(int)) and the confirmation (bz.oh(), bz.W(boolean), bz.oi()).

use brasfoot_engine::clubs;
use brasfoot_engine::lineup::{available, pick_for_slot, sort_by_strength};
use brasfoot_engine::tables::{BENCH_SLOTS, FORMATIONS};
use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::saved::SavedLineup;
use brasfoot_model::world::World;

/// Places of the grid (`bz.AE`, 92 `components.ao`): 1 to 25 the pitch, 26 to 36 the bench,
/// from 37 on the players left out. Place 0 is never used.
pub const PLACES: usize = 92;
pub const FIRST_BENCH_PLACE: usize = 26;
pub const LAST_BENCH_PLACE: usize = 36;
pub const FIRST_SPARE_PLACE: usize = 37;

/// One place of the grid: its player, and whether it shows an empty shirt (`ao.vf()` set
/// without a player), which the saved line-up keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Place {
    pub player: Option<PlayerId>,
    pub shirt: bool,
}

/// `bz.oh()` refuses a line-up without exactly eleven players on the pitch.
#[derive(Debug, PartialEq, Eq)]
pub struct NotEleven(pub usize);

/// The line-up screen of a human club for one match.
pub struct LineupScreen {
    pub club: ClubId,
    pub game_match: MatchId,
    /// `bz.AF`: the players who can be picked, in squad order.
    pub available: Vec<PlayerId>,
    pub grid: Vec<Place>,
    /// `bz.AT`: 0 the club plays at home, 1 away.
    away: bool,
}

/// `ah.kz()`: the strongest player, the oldest first among equals (`ah.nJ`), sorting the squad.
pub(crate) fn pick_captain(world: &mut World, club: ClubId) {
    let mut squad = world.club(club).squad.clone();
    if squad.is_empty() {
        return;
    }
    squad.sort_by(|&a, &b| {
        let (pa, pb) = (world.player(a), world.player(b));
        pb.strength.cmp(&pa.strength).then(pb.age.cmp(&pa.age))
    });
    world.club_mut(club).captain = Some(squad[0]);
    world.club_mut(club).squad = squad;
}

/// `ah.kA()`: the first of the squad sorted by strength (`h.a.abh`, which sorts the squad) with
/// status 1 and the penalty trait (9), else with status 1 out of goal, else out of goal.
pub(crate) fn pick_penalty_taker(world: &mut World, club: ClubId) {
    let mut squad = world.club(club).squad.clone();
    if squad.is_empty() {
        return;
    }
    sort_by_strength(world, &mut squad);
    world.club_mut(club).squad = squad.clone();
    let find = |test: &dyn Fn(&brasfoot_model::saved::Player) -> bool| squad.iter().copied().find(|&p| test(world.player(p)));
    let taker = find(&|p| p.status == 1 && p.trait1 == 9)
        .or_else(|| find(&|p| p.status == 1 && p.position != 0))
        .or_else(|| find(&|p| p.position != 0));
    if let Some(t) = taker {
        world.club_mut(club).penalty_taker = Some(t);
    }
}

impl LineupScreen {
    /// `new bz(dialog, club, match, false)`.
    pub fn open(world: &mut World, club: ClubId, game_match: MatchId) -> LineupScreen {
        if clubs::penalty_taker(world, club).is_none() {
            pick_penalty_taker(world, club);
        }
        if clubs::captain(world, club).is_none() {
            pick_captain(world, club);
        }
        let away = world.game_match(game_match).home != Some(club);
        let friendly = world.game_match(game_match).competition.is_some_and(|c| world.competition(c).kind == 0);
        if friendly {
            crate::unported!("bz.U(true) and bz.oe() of a friendly: the youth squad and the AY filter");
        }
        // bz.U(false)
        let mut can_play = Vec::new();
        for p in world.club(club).squad.clone() {
            if available(world, p, Some(game_match), false) {
                can_play.push(p);
            }
        }
        let mut screen = LineupScreen { club, game_match, available: can_play, grid: vec![Place::default(); PLACES], away };
        // bz.lA() (V(false): of(), then oe()), then the last line-up when the game keeps it
        // (f.bm())
        screen.empty_shirts();
        screen.fill_spare_places();
        if world.game.flag_bd
            && let Some(saved) = world.club(club).saved_lineup.clone()
        {
            screen.restore(&saved);
        }
        screen
    }

    fn on_grid(&self, p: PlayerId) -> bool {
        self.grid[1..=LAST_BENCH_PLACE].iter().any(|place| place.player == Some(p))
    }

    /// `bz.of()`: the empty shirts of a new grid, on the places of the 4-4-2 (`aq.sJ[4]`) and
    /// on the bench. Only a place with a shirt or a player takes a player from the list.
    fn empty_shirts(&mut self) {
        for &slot in &FORMATIONS[4] {
            self.grid[slot as usize].shirt = true;
        }
        for place in &mut self.grid[FIRST_BENCH_PLACE..=LAST_BENCH_PLACE] {
            place.shirt = true;
        }
    }

    /// Port of `bz.H(int, int)`: a player dragged from one place and dropped on another. On the
    /// pitch (2 to 25) a player goes anywhere on the pitch; the goalkeeper's place and the
    /// others only swap with a place that shows a shirt or a player. A place of the list left
    /// without a player is removed, the list closing up. False when nothing moved.
    pub fn move_player(&mut self, from: usize, to: usize) -> bool {
        let count = self.grid.len();
        if from == 0 || to == 0 || from >= count || to >= count {
            return false;
        }
        let shown = |i: usize| self.grid[i].player.is_some() || self.grid[i].shirt;
        let allowed = if from <= LAST_BENCH_PLACE {
            if from > 1 && from <= 25 && to <= 25 {
                true
            } else if from != 1 && to >= 25 {
                (from > 25 || to > 25) && shown(from) && shown(to)
            } else {
                shown(from) && shown(to)
            }
        } else {
            shown(from) && shown(to)
        };
        if !allowed {
            return false;
        }
        self.grid.swap(from, to);
        // the list of 92 keeps its length: a removed place comes back empty at the end
        let emptied = if from > LAST_BENCH_PLACE { from } else { to };
        if emptied > LAST_BENCH_PLACE && self.grid[emptied].player.is_none() {
            self.grid.remove(emptied);
            self.grid.push(Place::default());
        }
        true
    }

    /// `bz.oe()`: the players not on the pitch or the bench fill the places from 37 on.
    fn fill_spare_places(&mut self) {
        for place in &mut self.grid[FIRST_SPARE_PLACE..] {
            *place = Place::default();
        }
        let mut next = FIRST_SPARE_PLACE;
        for p in self.available.clone() {
            if !self.on_grid(p) {
                self.grid[next] = Place { player: Some(p), shirt: true };
                next += 1;
            }
        }
    }

    /// `bz.a(ag, false)`: the saved line-up, its players who can play in their places.
    fn restore(&mut self, saved: &SavedLineup) {
        for place in &mut self.grid[..PLACES - 1] {
            *place = Place::default();
        }
        for (player, &slot) in saved.players.iter().zip(&saved.slots) {
            let slot = slot as usize;
            if slot >= PLACES {
                continue;
            }
            self.grid[slot] = match player {
                Some(p) if self.available.contains(p) => Place { player: Some(*p), shirt: true },
                _ => Place { player: None, shirt: true },
            };
        }
        self.fill_spare_places();
    }

    /// `bz.cU(int)`: a formation of the combo (`aq.rB`); 0 clears the grid, the others pick the
    /// best players for its places (`aq.sJ`) and the bench (`aq.sI`).
    pub fn pick_formation(&mut self, world: &World, formation: usize) {
        self.grid = vec![Place::default(); PLACES];
        if formation == 0 {
            // lA()
            self.empty_shirts();
            self.fill_spare_places();
            return;
        }
        let mut candidates = self.available.clone();
        sort_by_strength(world, &mut candidates);
        let mut picked = Vec::new();
        for &slot in &FORMATIONS[formation] {
            if let Some(p) = pick_for_slot(world, &candidates, slot, false, false) {
                self.grid[slot as usize] = Place { player: Some(p), shirt: true };
                candidates.retain(|&c| c != p);
                picked.push(p);
            }
        }
        let mut next = FIRST_BENCH_PLACE;
        for &slot in &BENCH_SLOTS {
            if let Some(p) = pick_for_slot(world, &candidates, slot, true, false) {
                self.grid[next] = Place { player: Some(p), shirt: true };
                candidates.retain(|&c| c != p);
                picked.push(p);
                next += 1;
            }
        }
        for place in &mut self.grid[FIRST_BENCH_PLACE..=LAST_BENCH_PLACE] {
            if !place.shirt {
                // ao.ej(int)
                *place = Place { player: None, shirt: true };
            }
        }
        let mut next = FIRST_SPARE_PLACE;
        for p in self.available.clone() {
            if !picked.contains(&p) {
                self.grid[next] = Place { player: Some(p), shirt: true };
                next += 1;
            }
        }
    }

    /// The players on the pitch (places 1 to 25).
    pub fn starters(&self) -> usize {
        self.grid[1..FIRST_BENCH_PLACE].iter().filter(|place| place.player.is_some()).count()
    }

    /// `bz.oh()`: eleven on the pitch, then the line-up is saved (`W(true)`) and given to the
    /// match (`oi()`). The game then goes on with `f.ap()`, which is the caller's.
    pub fn confirm(&self, world: &mut World) -> Result<(), NotEleven> {
        let starters = self.starters();
        if starters != 11 {
            return Err(NotEleven(starters));
        }
        // W(true)
        let saved = world.club_mut(self.club).saved_lineup.get_or_insert_with(SavedLineup::default);
        saved.players.clear();
        saved.slots.clear();
        for (slot, place) in self.grid[..FIRST_SPARE_PLACE].iter().enumerate() {
            if place.player.is_some() || place.shirt {
                saved.players.push(place.player);
                saved.slots.push(slot as i32);
            }
        }
        // oi()
        world.club_mut(self.club).last_starters.clear();
        {
            let m = world.game_match_mut(self.game_match);
            if self.away {
                m.away_starters.clear();
                m.away_played.clear();
                m.away_bench.clear();
            } else {
                m.home_starters.clear();
                m.home_played.clear();
                m.home_bench.clear();
            }
        }
        for slot in 1..FIRST_BENCH_PLACE {
            let Some(p) = self.grid[slot].player else { continue };
            world.player_mut(p).slot = slot as i32;
            world.player_mut(p).started_recently = true;
            world.club_mut(self.club).last_starters.push(p);
            let m = world.game_match_mut(self.game_match);
            if self.away {
                m.away_starters.push(p);
                m.away_played.push(p);
            } else {
                m.home_starters.push(p);
                m.home_played.push(p);
            }
        }
        world.club_mut(self.club).last_bench.clear();
        for slot in FIRST_BENCH_PLACE..=LAST_BENCH_PLACE {
            let Some(p) = self.grid[slot].player else { continue };
            world.player_mut(p).slot = slot as i32;
            world.player_mut(p).started_recently = true;
            world.club_mut(self.club).last_bench.push(p);
            let m = world.game_match_mut(self.game_match);
            if self.away {
                m.away_bench.push(p);
            } else {
                m.home_bench.push(p);
            }
        }
        world.club_mut(self.club).lineup_set = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> LineupScreen {
        let mut s = LineupScreen { club: ClubId(0), game_match: MatchId(0), available: Vec::new(), grid: vec![Place::default(); PLACES], away: false };
        s.empty_shirts();
        for (i, place) in s.grid[FIRST_SPARE_PLACE..FIRST_SPARE_PLACE + 3].iter_mut().enumerate() {
            *place = Place { player: Some(PlayerId(i as u32)), shirt: true };
        }
        s
    }

    #[test]
    fn a_player_of_the_list_goes_only_to_a_shirt() {
        let mut s = screen();
        // place 7 has no shirt in the 4-4-2; place 1 (the goalkeeper) has one
        assert!(!s.move_player(FIRST_SPARE_PLACE, 7));
        assert!(s.move_player(FIRST_SPARE_PLACE, 1));
        assert_eq!(s.grid[1].player, Some(PlayerId(0)));
        // the list closed up
        assert_eq!(s.grid[FIRST_SPARE_PLACE].player, Some(PlayerId(1)));
        assert_eq!(s.grid[FIRST_SPARE_PLACE + 2].player, None);
    }

    #[test]
    fn a_player_on_the_pitch_moves_anywhere_on_it() {
        let mut s = screen();
        s.grid[20] = Place { player: Some(PlayerId(9)), shirt: true };
        assert!(s.move_player(20, 2));
        assert_eq!(s.grid[2].player, Some(PlayerId(9)));
        // but not to an empty place of the list
        assert!(!s.move_player(2, FIRST_SPARE_PLACE + 5));
    }
}
