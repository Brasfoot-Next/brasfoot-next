// Port of the automatic tactics of bfmod.Taticas (prepararProximo, automatica): with an assistant
// coach of level 3 or more, the staff writes its advice into the player's tactic as soon as the
// next match is known, so the line-up screen already shows it; a change the player makes there
// afterwards is kept (the engine's kick-off checks it, `Taticas.inicio`).

use brasfoot_engine::tactics::{advise, likely_eleven, measure, set_club_tactic};
use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::StaffRole;
use brasfoot_model::world::World;

use crate::Mod;
use crate::config::{ModConfig, Switch};
use crate::staff::{my_club, staff_level};

const DAY_MILLIS: i64 = 86_400_000;

/// `Taticas.preparado`, `aplicadaPreJogo`, `previstoPreJogo`: the match the advice was written
/// for (rival game id, day), the tactic written and the rival style it answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedTactic {
    pub rival: i32,
    pub day: i64,
    pub applied: [i32; 3],
    pub predicted: i32,
}

/// `Taticas.automatica()`
pub fn automatic(world: &World, config: &ModConfig) -> bool {
    config.on(Switch::AutoTactics) && staff_level(world, config, StaffRole::Assistant) >= 3
}

/// `Desempenho.proximo(ah)` as far as the tactics use it: the rival, whether at home, and the
/// date of the club's next match not played yet.
pub fn next_match(world: &World, club: ClubId) -> Option<(ClubId, bool, i64)> {
    let days = &world.game.days;
    for day in days.iter().skip(world.game.day.max(0) as usize) {
        for &m in &day.matches {
            let g = world.game_match(m);
            if (g.home == Some(club) || g.away == Some(club)) && !g.played {
                let home = g.home == Some(club);
                let rival = if home { g.away } else { g.home };
                return rival.map(|r| (r, home, day.date));
            }
        }
    }
    None
}

/// `Taticas.semente(ah, Calendar)`
fn seed(world: &World, rival: ClubId, date: i64) -> i64 {
    31 * world.club(rival).id as i64 + date / DAY_MILLIS
}

impl Mod<'_> {
    /// `Taticas.prepararProximo()`: once per match.
    pub(crate) fn prepare_next_tactic(&mut self) {
        if !self.sys.config.on(Switch::Tactics) || !automatic(self.world, &self.sys.config) {
            return;
        }
        let Some(club) = my_club(self.world) else { return };
        let Some((rival, home, date)) = next_match(self.world, club) else { return };
        if self.world.club(rival).human {
            return;
        }
        let (rival_id, day) = (self.world.club(rival).id, date / DAY_MILLIS);
        if self.sys.prepared_tactic.is_some_and(|p| p.rival == rival_id && p.day == day) {
            return;
        }
        let (mine, theirs) = (likely_eleven(self.world, club), likely_eleven(self.world, rival));
        let p = if home { measure(self.world, Some(club), Some(rival), mine, theirs) } else { measure(self.world, Some(rival), Some(club), theirs, mine) };
        let analyst = self.staff_level(StaffRole::Analyst);
        let advice = advise(self.world, &p, if home { 0 } else { 1 }, analyst, seed(self.world, rival, date));
        set_club_tactic(self.world, club, advice.best);
        let predicted = advice.predicted_rival.map_or(-1, |t| t[0]);
        self.sys.prepared_tactic = Some(PreparedTactic { rival: rival_id, day, applied: advice.best, predicted });
    }
}
