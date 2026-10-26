//! The game's data files that new careers start from: the team files (`teams/*.ban`, `e.t` with
//! its players `e.g`) and the national league configurations (`conf_ligas_nacionais/*.cfg`,
//! `est.ConfigLigaType`). Game data, read only.

use serde::{Deserialize, Serialize};

/// `e.g`: a player of a team file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerTemplate {
    /// `a`
    pub name: String,
    /// `b`
    pub star: bool,
    /// `c`
    pub country: i32,
    /// `d`
    pub age: i32,
    /// `e`
    pub position: i32,
    /// `f`
    pub status: i32,
    /// `g`
    pub trait1: i32,
    /// `h`
    pub trait2: i32,
    /// `i`
    pub side: i32,
    /// `hash`
    pub hash: i32,
    /// `j`
    pub world_class: bool,
    pub tid: i32,
    pub sid: i32,
    pub aid: i32,
}

/// `e.t`: a team file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TeamTemplate {
    /// The file's name without `.ban` (`d`, set by `U.a` when it reads the file).
    pub file_ref: String,
    pub valid: bool,
    pub id: i32,
    /// `a`
    pub country: i32,
    /// `b`
    pub state: i32,
    /// `c`
    pub level: i32,
    /// `e`
    pub name: String,
    /// `f`
    pub stadium: String,
    /// `g`
    pub capacity: i32,
    /// `h`
    pub coach: String,
    /// `i`
    pub coach_country: i32,
    /// `l`
    pub players: Vec<PlayerTemplate>,
    /// `m`
    pub youth: Vec<PlayerTemplate>,
    /// `n`
    pub reputation: i32,
    /// `o`
    pub base_color: i32,
    pub tid: i32,
    pub sid: i32,
    pub aid: i32,
    /// The format version; the game reads only 185.
    pub vid: i32,
    pub color1: String,
    pub color2: String,
    /// The file references of its classic derbies. Team files have none (the original's derbies
    /// are a fixed table, `aq`'s pairs in `I.ic()`); data packs list them per club.
    #[serde(default)]
    pub rivals: Vec<String>,
}

/// `est.ConfigLigaType`: one division of a national league as the player configured it, with
/// the Java field initialisers as defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeagueConfig {
    /// `pais`
    pub country: i32,
    /// `divisao`
    pub division: i32,
    /// `formula`: the number of turns of a short league.
    pub formula: i32,
    /// `desempate`
    pub tiebreak: i32,
    /// `nTimes`
    pub teams: i32,
    /// `doisTurnos`
    pub two_rounds: bool,
    /// `nGrupos`
    pub groups: i32,
    /// `jogosDentroGrupo`
    pub games_within_group: bool,
    /// `rebaixadoPeloGrupo`
    pub relegated_by_group: bool,
    /// `numeroTimesMataMata`
    pub knockout_teams: i32,
    /// `valido`
    pub valid: bool,
    /// `nRebaixados`
    pub relegated: i32,
    /// `duasVoltasMataMata`
    pub two_legged_knockout: Vec<bool>,
    /// `classificaPeloGeral`
    pub qualify_by_overall_table: bool,
    /// `melhoresTerceiros`
    pub best_third_places: bool,
    /// `nome` (`getNome2()`)
    pub name: Option<String>,
    /// `nomeDivisao`
    pub division_name: Option<String>,
    /// `playoffRebaixamento`
    pub relegation_playoff: i32,
    /// `duasVoltasplayoffReb`
    pub two_legged_relegation_playoff: Vec<bool>,
    /// `duasVoltasMataMataSobe`
    pub two_legged_promotion_knockout: Vec<bool>,
    /// `vagasSobemPeloMataMata`
    pub promotion_places: i32,
    /// `rebaixadosDireto`
    pub relegated_directly: i32,
    /// `versaoArquivo`
    pub file_version: i32,
}

impl Default for LeagueConfig {
    fn default() -> Self {
        LeagueConfig {
            country: -1,
            division: -1,
            formula: 0,
            tiebreak: 0,
            teams: 0,
            two_rounds: true,
            groups: 0,
            games_within_group: true,
            relegated_by_group: false,
            knockout_teams: 0,
            valid: false,
            relegated: 2,
            two_legged_knockout: vec![true; 7],
            qualify_by_overall_table: false,
            best_third_places: false,
            name: None,
            division_name: None,
            relegation_playoff: 0,
            two_legged_relegation_playoff: vec![false; 3],
            two_legged_promotion_knockout: vec![false; 3],
            promotion_places: 0,
            relegated_directly: 2,
            file_version: 0,
        }
    }
}

/// `est.ConfigEstadualType`: one division of a Brazilian state league as the player configured
/// it, with the Java field initialisers as defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateLeagueConfig {
    /// `id`: the state.
    pub state: i32,
    /// `divisao`
    pub division: i32,
    /// `formula`: the row of `aq.sL` (teams, groups, knockout teams, two rounds, ...).
    pub formula: i32,
    /// `nRebaixados`
    pub relegated: i32,
    /// `desempate`
    pub tiebreak: i32,
    /// `finaisIdaVolta`: 2 for two legs, per final round.
    pub two_legged_finals: [i32; 3],
}

impl Default for StateLeagueConfig {
    fn default() -> Self {
        StateLeagueConfig { state: -1, division: -1, formula: 0, relegated: 2, tiebreak: 0, two_legged_finals: [2, 2, 2] }
    }
}
