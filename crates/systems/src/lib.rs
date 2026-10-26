//! The mod systems (`bfmod`): what they keep, how they step into the game's simulation and
//! matches, and their own yearly and daily work. Each module ports one `bfmod` class.

pub mod academy;
pub mod assets;
pub mod awards;
pub mod cash;
pub mod clock;
pub mod config;
pub mod credit;
pub mod director;
pub mod economy;
pub mod fans;
pub mod investor;
pub mod loans;
pub mod market;
pub mod news;
pub mod performance;
pub mod satellites;
pub mod scouts;
pub mod sponsors;
pub mod staff;
pub mod stadiums;
pub mod super_mundial;
pub mod tactics;
pub mod windows;
pub mod youth_league;

use std::collections::{HashMap, HashSet};

use brasfoot_engine::hooks::MatchHooks;
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::rng::{JavaRandom, RandomSource};
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;
use brasfoot_simulation::SimulationHooks;

use config::{ModConfig, Switch};

/// The mod classes with a `static final Random SORTE`, in the order the Java harness gives them
/// their fixed seeds (`900000 + index`). `Taticas.SORTE` draws from the counter instead.
pub const STATIC_RANDOMS: [&str; 16] = [
    "Comissao", "Base", "Estadios", "Emprestimos", "Cotas", "Credito", "Janelas", "CampeonatoBase", "Economia", "Investidor", "Patrocinio",
    "Mercado", "Patrimonio", "SuperMundial", "Satelites", "Motor",
];
pub const STATIC_RANDOM_SEED: i64 = 900_000;

/// The systems' `SORTE`, created when first used.
#[derive(Debug, Default)]
pub struct StaticRandoms {
    randoms: HashMap<&'static str, JavaRandom>,
}

impl StaticRandoms {
    pub fn get(&mut self, rng: &mut RandomSource, class: &'static str) -> &mut JavaRandom {
        let index = STATIC_RANDOMS.iter().position(|&c| c == class).expect("a mod class with a static Random") as i64;
        self.randoms.entry(class).or_insert_with(|| rng.static_random(STATIC_RANDOM_SEED + index))
    }
}

/// The mod as installed: its switches and the state its classes keep in static fields.
#[derive(Debug, Default)]
pub struct ModSystems {
    pub config: ModConfig,
    pub randoms: StaticRandoms,
    market: market::MarketDay,
    /// `BolaDeOuro.ultimaTemporadaAnunciada`
    last_awards_season: i32,
    /// `BolaDeOuro.historicoEm`: the save and day the history was last brought up to date.
    awards_refreshed_at: Option<(Option<String>, i32)>,
    /// `Economia.humanosAntes`: the human clubs at the last tick (by game id).
    humans_before: Option<HashSet<i32>>,
    /// `Janelas.negociados`: the players the window has moved already.
    traded: HashSet<PlayerId>,
    /// `Taticas.preparado` and what was written with it.
    prepared_tactic: Option<tactics::PreparedTactic>,
    /// `Base.garotos != null`: the academy's state was loaded in this run.
    academy_loaded: bool,
    /// `Base.ultimaVez`
    academy_last_run: Option<(i64, usize, usize)>,
    /// Counts the academy's runs, to notice one inside another.
    academy_generation: u64,
    /// `Torcida.motivos != null`: the fans' state was loaded in this run.
    fans_loaded: bool,
    /// `Investidor.save`: the investor's state was loaded in this run.
    investor_loaded: bool,
    /// `Investidor.pedidoDeTeste`
    investor_test_request: bool,
    /// `Patrimonio.estruturas != null`: the assets' state was loaded in this run.
    assets_loaded: bool,
    /// `Patrimonio.ultimoMesIA`
    assets_last_month: i32,
    /// `Patrimonio.lesoesTratadas`: the injury end each player's treatment set, by key.
    treated_injuries: HashMap<String, i64>,
    /// The economy as `Economia.tick` last wrote it to its file (the systems after it in the
    /// tick, the loans' fines say, change it in memory only until the next tick).
    pub economy_saved: Option<brasfoot_model::mods::EconomyState>,
    /// `Satelites.vinculos != null`: the networks were loaded in this run.
    satellites_loaded: bool,
    /// `Credito.emprestimosPorClube != null`: the bank was loaded in this run.
    credit_loaded: bool,
    /// `Patrimonio.partidasProcessadas`: the matches whose gate was raised.
    gates_applied: HashSet<brasfoot_model::ids::MatchId>,
    /// `Mercado.mostrarPropostas`: the offers and loan requests for the human squads the
    /// window made, waiting for the game's offer screen (`a.jm`), in order.
    pub squad_offers: Vec<windows::SquadOffer>,
}

/// The world, the runtime and the mod, for the systems' work: they call one another and each
/// draws from its own `SORTE` ([`sorte!`]).
pub struct Mod<'a> {
    pub world: &'a mut World,
    pub runtime: &'a mut Runtime,
    pub sys: &'a mut ModSystems,
}

/// The `static final Random SORTE` of a mod class.
#[macro_export]
macro_rules! sorte {
    ($m:expr, $class:literal) => {
        $m.sys.randoms.get(&mut $m.runtime.rng, $class)
    };
}

impl ModSystems {
    pub fn new(config: ModConfig) -> Self {
        ModSystems { config, last_awards_season: -1, assets_last_month: -1, ..Default::default() }
    }
}

impl ModSystems {
    /// `BolaDeOuro.tick`: what the mod does each time the main screen is built, as far as it is
    /// ported (the windows' calendar).
    pub fn main_screen_tick(&mut self, world: &mut World, runtime: &mut Runtime) {
        Mod { world, runtime, sys: self }.main_screen_tick();
    }

    /// A player's action on a mod screen (signing a sponsor, answering an investor, ...), with
    /// the mod's context.
    pub fn with<R>(&mut self, world: &mut World, runtime: &mut Runtime, action: impl FnOnce(&mut Mod) -> R) -> R {
        action(&mut Mod { world, runtime, sys: self })
    }

    /// The staff screen's candidates of the month (`Comissao.candidatos()`).
    pub fn staff_candidates(&mut self, world: &mut World, runtime: &mut Runtime) -> Vec<brasfoot_model::mods::StaffMember> {
        Mod { world, runtime, sys: self }.staff_candidates()
    }

    /// `Comissao.contratar`: the candidate at that index of [`Self::staff_candidates`].
    pub fn hire_staff(&mut self, world: &mut World, runtime: &mut Runtime, candidate: usize) -> Result<(), staff::HireError> {
        Mod { world, runtime, sys: self }.hire_staff(candidate)
    }

    /// The juniors screen's promotion of a youth player of the club (`BaseDialog`).
    pub fn promote_junior(&mut self, world: &mut World, runtime: &mut Runtime, club: ClubId, p: PlayerId) -> Result<(), academy::PromoteError> {
        Mod { world, runtime, sys: self }.promote_junior(club, p)
    }

    /// The awards window opened from the toolbar (`BolaDeOuro.mostrarHistorico`): the season it
    /// shows first.
    pub fn open_awards(&mut self, world: &mut World) -> awards::season::Season {
        awards::open_history(world, &mut self.awards_refreshed_at)
    }
}

impl MatchHooks for ModSystems {
    fn tactics_enabled(&self) -> bool {
        self.config.on(Switch::Tactics)
    }

    fn auto_tactics(&self, world: &World) -> bool {
        tactics::automatic(world, &self.config)
    }

    fn analyst_level(&self, world: &World) -> i32 {
        staff::staff_level(world, &self.config, brasfoot_model::mods::StaffRole::Analyst)
    }

    fn match_bonus(&self, world: &World, game_match: brasfoot_model::ids::MatchId, side: usize) -> f64 {
        assets::match_bonus(world, &self.config, game_match, side)
    }

    fn on_kickoff(&mut self, world: &mut World, runtime: &mut Runtime, game_match: brasfoot_model::ids::MatchId) {
        Mod { world, runtime, sys: self }.satellites_kickoff(game_match);
    }

    fn tactic_directive(&self, world: &World, club: ClubId) -> Option<[i32; 3]> {
        satellites::satellite_directive_in(world, &self.config, club)
    }

    fn on_tick(&mut self, world: &mut World, runtime: &mut Runtime, game_match: brasfoot_model::ids::MatchId) {
        Mod { world, runtime, sys: self }.apply_gate(game_match);
    }

    fn prepared_tactic(&self, world: &World, rival: ClubId) -> Option<([i32; 3], i32)> {
        let p = self.prepared_tactic?;
        (p.rival == world.club(rival).id && p.day == world.today_millis() / 86_400_000).then_some((p.applied, p.predicted))
    }
}

impl SimulationHooks for ModSystems {
    fn game_buyer(&mut self, world: &mut World, runtime: &mut Runtime, player: PlayerId, drawn: Option<ClubId>) -> Option<ClubId> {
        ModSystems::game_buyer(self, world, runtime, player, drawn)
    }

    fn approve_game_sale(&mut self, world: &mut World, runtime: &mut Runtime, player: PlayerId, buyer: ClubId, fee: i32) -> bool {
        ModSystems::approve_game_sale(self, world, runtime, player, buyer, fee)
    }

    fn season_end_screen(&mut self, world: &mut World, runtime: &mut Runtime) {
        awards::season_end(&mut Mod { world, runtime, sys: self });
    }
}
