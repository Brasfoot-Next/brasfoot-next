//! A career with a human club: what the game does through its screens. The line-up screen
//! (`a.bz`), the new contract (`a.fy`), the round screen that plays the human clubs' matches
//! (`best.s`, `a.dV`) and the decisions the game asks of the human in its dialogs.

pub mod contract;
pub mod finances;
pub mod lineup;
pub mod purchase;
pub mod messages;
pub mod round;
pub mod stadium;
pub mod substitutions;

use brasfoot_model::ids::{ClubId, CoachId, CompetitionId, MatchId, PlayerId};
use brasfoot_model::world::World;

use crate::Sim;

/// What the human decides in the game's dialogs, at the moment the game asks. The defaults are
/// the human who changes nothing and accepts nothing: the engine's penalty taker, the shoot-out
/// order as listed, no substitution, no job.
pub trait HumanDecisions {
    /// `a.gh`: the player the human sends to take a penalty; `None` keeps the engine's.
    fn penalty_taker(&mut self, _world: &World, _game_match: MatchId, _club: ClubId) -> Option<PlayerId> {
        None
    }

    /// `a.C`: the shoot-out order of the human's club, from its players on the pitch as listed.
    fn shootout_order(&mut self, _world: &World, _game_match: MatchId, _club: ClubId, listed: Vec<PlayerId>) -> Vec<PlayerId> {
        listed
    }

    /// `a.cD`: what the human changes in the substitutions window, opened at half time (when
    /// the game's option says so) or after a red card or an injury of his club.
    fn substitutions(&mut self, _world: &mut World, _game_match: MatchId, _half_time: bool) {}

    /// `a.dJ` (`S.a(ah, at, int)`): a human club won a title; the screen only celebrates it.
    fn title(&mut self, _world: &World, _club: ClubId, _competition: CompetitionId) {}

    /// `a.gO`: the club the coach takes among the offers (`ay.b(al, boolean)`), if any.
    fn job_offer(&mut self, _world: &World, _coach: CoachId, _clubs: &[ClubId]) -> Option<ClubId> {
        None
    }

    /// `a.gO` with national teams (`S.a(list, coach, 1)`): the national team the coach takes
    /// among the offers (`ay.a(al, boolean)`), if any.
    fn national_team_offer(&mut self, _world: &World, _coach: CoachId, _teams: &[ClubId]) -> Option<ClubId> {
        None
    }
}

/// The human who never changes or accepts anything.
pub struct NoChanges;

impl HumanDecisions for NoChanges {}

impl Sim<'_> {
    /// Asks the human (the one who changes nothing when the caller gave none).
    pub(crate) fn ask<R>(&mut self, question: impl FnOnce(&mut dyn HumanDecisions, &mut World) -> R) -> R {
        let mut human = self.human.take();
        let mut nobody = NoChanges;
        let decisions: &mut dyn HumanDecisions = match human.as_deref_mut() {
            Some(h) => h,
            None => &mut nobody,
        };
        let answer = question(decisions, self.world);
        self.human = human;
        answer
    }
}
