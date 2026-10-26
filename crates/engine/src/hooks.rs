//! Where the mod systems step into a match. The engine calls these at the points where the
//! modded game calls into `bfmod` (`Satelites`, `Patrimonio`, `Comissao`, `Config`); the default
//! is what the game does with those systems turned off.

use brasfoot_model::ids::{ClubId, MatchId};
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;

pub trait MatchHooks {
    /// `Config.ligado(TATICAS)`.
    fn tactics_enabled(&self) -> bool {
        true
    }

    /// `Satelites.aoIniciarPartida`: at kick-off, before the tactics are measured.
    fn on_kickoff(&mut self, _world: &mut World, _runtime: &mut Runtime, _game_match: MatchId) {}

    /// `Satelites.diretrizTatica`: a tactic imposed on an AI club, {style, marking, wings}.
    fn tactic_directive(&self, _world: &World, _club: ClubId) -> Option<[i32; 3]> {
        None
    }

    /// `Patrimonio.aplicarBilheteria`, called on every engine tick (it acts once per match).
    fn on_tick(&mut self, _world: &mut World, _runtime: &mut Runtime, _game_match: MatchId) {}

    /// `Patrimonio.bonusPartida`: a multiplier of a side's midfield and shot weights.
    fn match_bonus(&self, _world: &World, _game_match: MatchId, _side: usize) -> f64 {
        1.0
    }

    /// `Taticas.automatica()`: the staff sets the player's tactic.
    fn auto_tactics(&self, _world: &World) -> bool {
        false
    }

    /// `Comissao.nivel(ANALISTA)`.
    fn analyst_level(&self, _world: &World) -> i32 {
        0
    }

    /// The tactic written before the match against this rival today (`Taticas.preparado`):
    /// the tactic applied and the rival style it answers.
    fn prepared_tactic(&self, _world: &World, _rival: ClubId) -> Option<([i32; 3], i32)> {
        None
    }
}

/// The game with every mod system off but the tactics.
#[derive(Debug, Default)]
pub struct NoSystems {
    pub tactics_off: bool,
}

impl MatchHooks for NoSystems {
    fn tactics_enabled(&self) -> bool {
        !self.tactics_off
    }
}
