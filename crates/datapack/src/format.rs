//! The files of a data pack, as serde types. A pack is a folder:
//!
//! ```text
//! manifest.toml
//! clubs/<country>/<slug>/club.toml, squad.csv, youth.csv, crest.png, kit.png, kit2.png, kit3.png
//! leagues/<country>.toml
//! state-leagues/<state>.toml
//! national-teams/<country>/crest.png, kit.png, kit2.png
//! trophies/<kind>/<name>.png
//! ```
//!
//! A club's id is its folder below `clubs/` (`bra/vila-exemplo`); folder names are lowercase.
//! Countries are FIFA trigrams and states the two letters of the Brazilian state (see
//! [`crate::codes`]).

use serde::{Deserialize, Serialize};

/// The version of the format this crate reads and writes.
pub const FORMAT_VERSION: u32 = 1;

pub const MANIFEST: &str = "manifest.toml";
pub const CLUB: &str = "club.toml";
pub const SQUAD: &str = "squad.csv";
pub const YOUTH: &str = "youth.csv";
pub const CREST: &str = "crest.png";
pub const KITS: [&str; 3] = ["kit.png", "kit2.png", "kit3.png"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// A whole database: clubs, leagues and pictures.
    Base,
    /// Changes over a database: clubs and leagues added or replaced, and removals.
    Patch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    /// `owner/name`, usually the GitHub repository.
    pub id: String,
    pub kind: SourceKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The id of the database a patch is made for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    #[serde(default, skip_serializing_if = "Removals::is_empty")]
    pub remove: Removals,
}

/// What a patch takes out of the layers below it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Removals {
    /// Club ids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clubs: Vec<String>,
    /// Country codes whose league configuration goes back to the game's default format.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leagues: Vec<String>,
    /// State codes whose state league configuration goes back to the default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_leagues: Vec<String>,
}

impl Removals {
    pub fn is_empty(&self) -> bool {
        self.clubs.is_empty() && self.leagues.is_empty() && self.state_leagues.is_empty()
    }
}

/// `club.toml`: a team file (`e.t`) without its players.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClubFile {
    pub name: String,
    pub country: String,
    /// Brazilian clubs only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// 1 to 25; the game reads anything else as 10.
    pub level: i32,
    pub reputation: i32,
    /// Two `#rrggbb` colors.
    pub colors: [String; 2],
    /// Club ids of its classic derbies; a pair counts when either side lists the other.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rivals: Vec<String>,
    pub stadium: StadiumFile,
    pub coach: CoachFile,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StadiumFile {
    pub name: String,
    /// 1000 to 120000; the game reads anything else as 10000.
    pub capacity: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoachFile {
    pub name: String,
    pub country: String,
}

/// A line of `squad.csv` or `youth.csv`: a player of a team file (`e.g`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerRow {
    /// Empty draws a name of the player's country when the game starts.
    pub name: String,
    pub position: String,
    pub side: String,
    pub age: i32,
    pub country: String,
    /// `hash`, what the game takes as the player's potential (`F.aq`).
    pub potential: i32,
    /// In the starting line-up of the team file (`status` 1).
    pub starter: bool,
    pub star: bool,
    /// Always a star too.
    pub world_class: bool,
    /// Empty for none.
    pub trait1: String,
    pub trait2: String,
}

/// `leagues/<country>.toml`: the national league configuration of a country
/// (`conf_ligas_nacionais/<code>.cfg`). Without it the game uses its default format.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeagueFile {
    pub country: String,
    pub divisions: Vec<DivisionFile>,
}

/// One division (`est.ConfigLigaType`). Omitted fields take the game's defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DivisionFile {
    /// 1 to 4.
    pub division: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub division_name: Option<String>,
    pub teams: i32,
    pub two_rounds: bool,
    /// `formula`: the number of turns of a short league.
    pub round_count: i32,
    pub groups: i32,
    pub games_within_group: bool,
    pub knockout_teams: i32,
    /// Two legs or one, per knockout round.
    pub two_legged_knockout: Vec<bool>,
    pub qualify_by_overall_table: bool,
    pub best_third_places: bool,
    pub tiebreak: i32,
    pub relegated: i32,
    pub relegated_directly: i32,
    pub relegated_by_group: bool,
    pub relegation_playoff: i32,
    pub two_legged_relegation_playoff: Vec<bool>,
    pub promotion_places: i32,
    pub two_legged_promotion_knockout: Vec<bool>,
}

impl Default for DivisionFile {
    /// The field initialisers of `est.ConfigLigaType`.
    fn default() -> Self {
        DivisionFile {
            division: 1,
            name: None,
            division_name: None,
            teams: 0,
            two_rounds: true,
            round_count: 0,
            groups: 0,
            games_within_group: true,
            knockout_teams: 0,
            two_legged_knockout: vec![true; 7],
            qualify_by_overall_table: false,
            best_third_places: false,
            tiebreak: 0,
            relegated: 2,
            relegated_directly: 2,
            relegated_by_group: false,
            relegation_playoff: 0,
            two_legged_relegation_playoff: vec![false; 3],
            promotion_places: 0,
            two_legged_promotion_knockout: vec![false; 3],
        }
    }
}

/// `state-leagues/<state>.toml`: a Brazilian state league (`conf_estadual/<UF>.ces`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateLeagueFile {
    pub state: String,
    pub divisions: Vec<StateDivisionFile>,
}

/// One division of a state league (`est.ConfigEstadualType`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateDivisionFile {
    /// 1 to 4.
    pub division: i32,
    /// One of [`crate::codes::STATE_FORMATS`].
    pub format: String,
    pub relegated: i32,
    pub tiebreak: i32,
    /// Two legs or one for each of the three final rounds.
    pub two_legged_finals: [bool; 3],
}
