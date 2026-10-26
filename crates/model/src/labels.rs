//! What a competition's name is made of. The original composes some names in Portuguese
//! (`f.s.getNome()`, and the names it gives the competitions it creates); here the parts are
//! data, kept for history (the mod's awards) and turned into text by `brasfoot_i18n::labels`.

use serde::{Deserialize, Serialize};

use crate::ids::{CompetitionId, StageId};
use crate::saved::CompetitionKind;
use crate::world::World;

/// A competition's name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompetitionLabel {
    /// A name from the game data.
    Saved(String),
    /// A national league, named by its table (`f.B.getNome()`).
    Stage(StageLabel),
    /// A national super cup this app created, by the country's game id.
    SuperCup { country: i32 },
    /// A national league with no table yet.
    League { country: i32, division: i32 },
    /// A national cup this app created.
    Cup { country: i32 },
    /// The game's yearly world club cup this app created (`f.A`).
    ClubWorldCup,
    /// That cup once the mod renamed it (`SuperMundial.INTERCONTINENTAL`).
    Intercontinental,
    /// The game's friendly tournament this app created (`f.L`).
    FriendlyTournament,
    /// The friendly tournament holding a Super Mundial (`SuperMundial.NOME`).
    SuperMundial,
    Unnamed,
}

/// `f.s.getNome()`: the league's name without "Campeonato ..." and its division. The original
/// cuts the whole name to 39 characters and "..." after the division is added, so that is done
/// on the text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageLabel {
    pub name: String,
    pub division: StageDivision,
    /// A national league whose name the original composes (`ad.br(country)`, "Campeonato
    /// Brasileiro"): the country's game id, the name then empty.
    #[serde(default)]
    pub league_of: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StageDivision {
    None,
    /// `" - " + divisao + "ª D"`
    Number(i32),
    /// `" - " + nomeDivisao`
    Named(String),
}

/// The words `f.s.getNome()` cuts from the league names of the game data, in its order.
pub const CUT_WORDS: [&str; 4] = ["Campeonato do", "Campeonato de", "Campeonato da", "Campeonato"];

impl World {
    /// The parts of `f.s.getNome()`.
    pub fn stage_label(&self, stage: StageId) -> StageLabel {
        let s = self.stage(stage);
        let mut name = if s.league_name.is_empty() { s.name.clone() } else { s.league_name.clone() };
        for cut in CUT_WORDS {
            name = name.replace(cut, "");
        }
        let division = if !s.division_name.is_empty() {
            StageDivision::Named(s.division_name.clone())
        } else if s.division > 0 && s.kind_code() != 1 {
            StageDivision::Number(s.division)
        } else {
            StageDivision::None
        };
        // Z.Ar(): a national league's table is named by its configuration or ad.br(country)
        let league_of = (s.league_name.is_empty() && s.name.is_empty()).then(|| s.country_ref.map(|c| self.country(c).country)).flatten();
        StageLabel { name, division, league_of }
    }

    /// `at.getNome()`: the saved name, a league's table name, or the parts of the names this app
    /// composes for the competitions it creates.
    pub fn competition_label(&self, c: CompetitionId) -> CompetitionLabel {
        let comp = self.competition(c);
        if comp.class == CompetitionKind::NationalLeague
            && let Some(stage) = comp.league
        {
            return CompetitionLabel::Stage(self.stage_label(stage));
        }
        // bfmod.SuperMundial: the friendly tournament of an edition, the yearly cup renamed
        let super_mundial = &self.mods.super_mundial;
        if comp.class == CompetitionKind::FriendlyTournament && (super_mundial.tournament_named || comp.name == crate::host_cities::SUPER_MUNDIAL_NAME) {
            return CompetitionLabel::SuperMundial;
        }
        if comp.class == CompetitionKind::IntercontinentalCup && comp.name.is_empty() {
            return if super_mundial.renamed { CompetitionLabel::Intercontinental } else { CompetitionLabel::ClubWorldCup };
        }
        if !comp.name.is_empty() {
            return CompetitionLabel::Saved(comp.name.clone());
        }
        if comp.class == CompetitionKind::FriendlyTournament {
            return CompetitionLabel::FriendlyTournament;
        }
        let country = match comp.class {
            CompetitionKind::NationalSuperCup => comp.country,
            _ => comp.country_ref.map(|k| self.country(k).country).unwrap_or(-1),
        };
        match comp.class {
            CompetitionKind::NationalSuperCup => CompetitionLabel::SuperCup { country },
            CompetitionKind::NationalLeague => CompetitionLabel::League { country, division: comp.division },
            CompetitionKind::NationalCup => CompetitionLabel::Cup { country },
            _ => CompetitionLabel::Unnamed,
        }
    }
}
