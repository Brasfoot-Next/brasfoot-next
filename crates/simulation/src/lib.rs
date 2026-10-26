//! The game without screens: the calendar, the match days, the competitions, the AI clubs, the
//! game's own market and the players' evolution. A port of `best.f` and the classes it drives.
//!
//! Every class of the original becomes a module of methods on [`Sim`], which borrows the world,
//! the transient state and the hooks of the mod systems for the length of a call.

pub mod clubs;
pub mod coach_market;
pub mod coaches;
pub mod continental;
pub mod cup;
pub mod evolution;
pub mod fixture_tables;
pub mod fixtures;
pub mod game;
pub mod human;
pub mod knockout;
pub mod league;
pub mod market;
pub mod matchday;
pub mod nations;
pub mod national_cups;
pub mod national_teams;
pub mod new_game;
pub mod new_season;
pub mod player_gen;
pub mod player_search;
pub mod season;
pub mod second_tiers;
pub mod stages;
pub mod sorting;
pub mod trophy;

use brasfoot_engine::hooks::MatchHooks;
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;

/// The world, the state the original keeps outside the save, the mod systems' hooks and the
/// human's decisions.
pub struct Sim<'a> {
    pub world: &'a mut World,
    pub runtime: &'a mut Runtime,
    pub hooks: &'a mut dyn SimulationHooks,
    /// Who answers the game's dialogs; `None` changes and accepts nothing.
    pub human: Option<&'a mut dyn human::HumanDecisions>,
}

/// Where the mod systems step into the game's own simulation, beyond the match. The defaults
/// are the game without the mod.
pub trait SimulationHooks: MatchHooks {
    /// `Mercado.compradorDoJogo`: the buyer of a sale of the game's yearly market, given the one
    /// the game drew.
    fn game_buyer(&mut self, _world: &mut World, _runtime: &mut Runtime, _player: PlayerId, drawn: Option<ClubId>) -> Option<ClubId> {
        drawn
    }

    /// `Mercado.aprovarVendaDoJogo`: whether a sale of the game's market goes ahead.
    fn approve_game_sale(&mut self, _world: &mut World, _runtime: &mut Runtime, _player: PlayerId, _buyer: ClubId, _fee: i32) -> bool {
        true
    }

    /// The season-end screen `a.cR` opens (`S.aY(6)`, right after `f.al()`); the mod's patched
    /// screen hands the season to `BolaDeOuro.fimDeTemporada`.
    fn season_end_screen(&mut self, _world: &mut World, _runtime: &mut Runtime) {}
}

impl SimulationHooks for brasfoot_engine::hooks::NoSystems {}

/// `c.a.SR.V()` with the screens off (`bs` false): plays the current match day and moves to the
/// next one, or starts a new season when the last one is over.
pub fn advance(world: &mut World, runtime: &mut Runtime, hooks: &mut dyn SimulationHooks) {
    Sim { world, runtime, hooks, human: None }.advance();
}

/// [`advance`] with the human's answers to the game's dialogs.
pub fn advance_with(world: &mut World, runtime: &mut Runtime, hooks: &mut dyn SimulationHooks, human: &mut dyn human::HumanDecisions) {
    Sim { world, runtime, hooks, human: Some(human) }.advance();
}

/// A part of the original not ported yet: stops with its name rather than diverging silently.
#[macro_export]
macro_rules! unported {
    ($what:expr) => {
        panic!("not ported yet: {}", $what)
    };
}

/// A value the original computes in a part not ported yet; `$default` until then.
#[macro_export]
macro_rules! unported_value {
    ($what:expr, $default:expr) => {{
        let _ = $what;
        $default
    }};
}

pub mod tables;
pub mod titles;
pub mod world_cup_qualifiers;
pub mod youth;

/// `I.ic()`: a classic, by the two clubs' team files.
pub fn matchday_rivalry(world: &brasfoot_model::world::World, m: brasfoot_model::ids::MatchId) -> bool {
    let g = world.game_match(m);
    let (Some(h), Some(a)) = (g.home, g.away) else { return false };
    let (fh, fa) = (&world.club(h).file_ref, &world.club(a).file_ref);
    if fh.is_empty() || fa.is_empty() {
        return false;
    }
    tables::CLASSIC_DERBIES.iter().any(|pair| (pair[0] == fh && pair[1] == fa) || (pair[0] == fa && pair[1] == fh))
}
