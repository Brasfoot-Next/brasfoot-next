// Port of bfmod.Olheiros: the scouting centre. What a player is worth on the pitch (the engine's
// value, Motor) is known exactly for the human's own squad and the players a scout watched, within
// two either side for the stars, and otherwise as a range of five either side around a fixed error
// per player. A scout costs 50 thousand per point of the club's reputation and reports after two
// weeks; three at most at once. The report's newspaper story is not ported.

use brasfoot_engine::tactics::{TYPICAL_SLOT, best_kind, best_value};
use brasfoot_engine::value::mod_value;
use brasfoot_model::ids::PlayerId;
use brasfoot_model::java_hash::JavaHash;
use brasfoot_model::mods::ScoutObservation;
use brasfoot_model::world::World;

use crate::Mod;
use crate::awards::season::player_key;
use crate::economy::is_national_team;
use crate::staff::my_club;

/// `MAX_SIMULTANEAS`
pub const MAX_AT_ONCE: usize = 3;
/// `DURACAO`: two weeks.
const DURATION: i64 = 14 * 86_400_000;
/// `MARGEM`: half the width of an unknown player's range.
const MARGIN: i32 = 5;

/// `observado(F)`: a scout's report came.
pub fn watched(world: &World, p: PlayerId) -> bool {
    let key = player_key(world, p);
    world.mods.scouting.iter().any(|o| o.key == key && o.done)
}

/// `emObservacao(F)`
pub fn being_watched(world: &World, p: PlayerId) -> bool {
    let key = player_key(world, p);
    world.mods.scouting.iter().any(|o| o.key == key && !o.done)
}

/// `pendentes()`
pub fn pending(world: &World) -> usize {
    world.mods.scouting.iter().filter(|o| !o.done).count()
}

/// `margem(F)`: none for the human's own and the watched players, two for the stars.
pub fn margin(world: &World, p: PlayerId) -> i32 {
    let x = world.player(p);
    if (x.club.is_some() && x.club == my_club(world)) || watched(world, p) {
        return 0;
    }
    if x.world_class || x.star {
        return 2;
    }
    MARGIN
}

/// `erro(F)`: the player's fixed error, from his key's hash.
fn error(world: &World, p: PlayerId) -> i32 {
    let m = margin(world, p);
    if m == 0 { 0 } else { player_key(world, p).java_hash().rem_euclid(2 * m + 1) - m }
}

/// `estimada(F)`: the centre of the visible range at his best slot.
pub fn estimate(world: &World, p: PlayerId) -> i32 {
    best_value(world, p) + error(world, p)
}

/// `estimada(F, int)`: the same in a slot.
pub fn estimate_in(world: &World, p: PlayerId, slot: i32) -> i32 {
    mod_value(world.game.individual_skills, world.player(p), slot) + error(world, p)
}

/// `faixa(F, int)`: what the scouts can say: the value, or the lowest and highest.
pub fn range(world: &World, p: PlayerId, estimate: i32) -> (i32, i32) {
    let m = margin(world, p);
    if m == 0 { (estimate, estimate) } else { (1.max(estimate - m), 99.min(estimate + m)) }
}

/// `custo()`: 50 thousand per point of the club's reputation.
pub fn cost(world: &World) -> i64 {
    let reputation = my_club(world).map(|c| world.club(c).reputation.max(1)).unwrap_or(1);
    50_000 * reputation as i64
}

/// `Olheiros.Achado`: a player of the search.
#[derive(Clone, Debug)]
pub struct Finding {
    pub player: PlayerId,
    pub estimate: i32,
    pub range: (i32, i32),
    /// `Motor.melhorTipo`: the kind of slot he is best at (an index into `Motor.NOMES`).
    pub kind: usize,
    pub watched: bool,
    pub being_watched: bool,
}

/// `buscar(int, int, int, long)`: the players of any club (no national team) by position (-1
/// for any), at most an age and a price (0 for any), from an estimated value on; the best 300.
pub fn search(world: &World, position: i32, max_age: i32, min_value: i32, max_price: i64) -> Vec<Finding> {
    let mut out = Vec::new();
    for &p in &world.game.players {
        let x = world.player(p);
        let Some(club) = x.club else { continue };
        if is_national_team(world, club) {
            continue;
        }
        if (position >= 0 && x.position != position) || x.age > max_age || (max_price > 0 && x.market_value as i64 > max_price) {
            continue;
        }
        let e = estimate(world, p);
        if e < min_value {
            continue;
        }
        out.push(Finding { player: p, estimate: e, range: range(world, p, e), kind: best_kind(world, p), watched: watched(world, p), being_watched: being_watched(world, p) });
    }
    out.sort_by(|a, b| b.estimate.cmp(&a.estimate));
    out.truncate(300);
    out
}

/// Why a scout does not go (`observar(F)`'s messages).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoutError {
    /// "Sem clube."
    NoClub,
    /// "Você já conhece esse jogador."
    AlreadyKnown,
    /// "Um olheiro já está observando esse jogador."
    AlreadyWatching,
    /// "Seus 3 olheiros estão ocupados. Espere um relatório chegar."
    AllBusy,
    /// "Sem dinheiro em caixa para o olheiro."
    NoCash,
}

impl Mod<'_> {
    /// `observar(F)`: a scout sets off, paid now.
    pub fn send_scout(&mut self, p: PlayerId) -> Result<(), ScoutError> {
        let Some(mine) = my_club(self.world) else { return Err(ScoutError::NoClub) };
        if margin(self.world, p) == 0 {
            return Err(ScoutError::AlreadyKnown);
        }
        if being_watched(self.world, p) {
            return Err(ScoutError::AlreadyWatching);
        }
        if pending(self.world) >= MAX_AT_ONCE {
            return Err(ScoutError::AllBusy);
        }
        let cost = cost(self.world);
        if self.world.club(mine).cash < cost {
            return Err(ScoutError::NoCash);
        }
        self.world.club_expense(mine, cost);
        let key = player_key(self.world, p);
        let since = self.world.today_millis();
        let list = &mut self.world.mods.scouting;
        // LinkedHashMap.put: a key already there keeps its place
        match list.iter_mut().find(|o| o.key == key) {
            Some(o) => {
                o.done = false;
                o.since = since;
            }
            None => list.push(ScoutObservation { key, done: false, since }),
        }
        Ok(())
    }

    /// `Olheiros.tick()`: the reports whose two weeks are over.
    pub(crate) fn scouts_tick(&mut self) {
        if self.world.game.days.is_empty() {
            return;
        }
        let now = self.world.today_millis();
        for o in &mut self.world.mods.scouting {
            if o.done || now - o.since < DURATION {
                continue;
            }
            o.done = true;
            // Jornal.noticia: the report (Base.relatorio, the value at his typical slot) is news only
        }
    }
}

/// `Motor.vagaTipica(Motor.melhorTipo(p))`: the slot his report is about.
pub fn report_slot(world: &World, p: PlayerId) -> i32 {
    TYPICAL_SLOT[best_kind(world, p)]
}
