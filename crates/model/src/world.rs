//! The whole game state: the root object of the original save (`c.a.SR`, [`Game`]), the second
//! root (`c.a.afQ`, [`CoachJobMarket`]) and one arena per shared class, indexed by the ids of
//! [`crate::ids`].

use serde::{Deserialize, Serialize};

use crate::ids::*;
use crate::saved::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub game: Game,
    pub job_market: CoachJobMarket,
    pub players: Vec<Player>,
    pub clubs: Vec<Club>,
    pub coaches: Vec<Coach>,
    pub stadiums: Vec<Stadium>,
    pub matches: Vec<Match>,
    pub competitions: Vec<Competition>,
    pub stages: Vec<Stage>,
    pub countries: Vec<Country>,
    pub state_leagues: Vec<StateLeague>,
    /// What the mod systems keep beside the save.
    #[serde(default)]
    pub mods: crate::mods::ModState,
}

macro_rules! arena_access {
    ($($get:ident, $get_mut:ident, $field:ident, $id:ident, $ty:ident);* $(;)?) => {
        impl World {$(
            pub fn $get(&self, id: $id) -> &$ty {
                &self.$field[id.index()]
            }

            pub fn $get_mut(&mut self, id: $id) -> &mut $ty {
                &mut self.$field[id.index()]
            }
        )*}
    };
}

arena_access!(
    player, player_mut, players, PlayerId, Player;
    club, club_mut, clubs, ClubId, Club;
    coach, coach_mut, coaches, CoachId, Coach;
    stadium, stadium_mut, stadiums, StadiumId, Stadium;
    game_match, game_match_mut, matches, MatchId, Match;
    competition, competition_mut, competitions, CompetitionId, Competition;
    stage, stage_mut, stages, StageId, Stage;
    country, country_mut, countries, CountryId, Country;
    state_league, state_league_mut, state_leagues, StateLeagueId, StateLeague;
);

impl World {
    /// `c.a.SR.bb()`: the date of the current calendar day (epoch millis).
    pub fn today_millis(&self) -> i64 {
        self.game.days.get(self.game.day.max(0) as usize).map(|d| d.date).unwrap_or(0)
    }

    /// `c.a.SR.H()`: the season index, 1 in the first season.
    pub fn season(&self) -> i32 {
        self.game.season
    }

    /// `c.a.SR.x(int)`: the club with a game id, among the clubs, then the national teams.
    pub fn club_by_game_id(&self, game_id: i32) -> Option<ClubId> {
        if let Some(&c) = self.game.clubs.iter().find(|&&c| self.club(c).id == game_id) {
            return Some(c);
        }
        self.game.countries.iter().filter_map(|&k| self.country(k).national_team).find(|&c| self.club(c).id == game_id)
    }

    /// `ah.kK()`: the wages of the squad and the youth squad.
    pub fn club_wages(&self, club: ClubId) -> i64 {
        let c = self.club(club);
        c.squad.iter().chain(&c.youth_squad).map(|&p| self.player(p).salary as i64).sum()
    }

    /// `c.a.SR.y(int)`: the coach with a game id.
    pub fn coach_by_game_id(&self, game_id: i32) -> Option<CoachId> {
        self.coaches.iter().position(|c| c.id == game_id).map(|i| CoachId(i as u32))
    }

    /// `ah.v(int, int)`: club income, into the cash and, for a human club, its finances by kind.
    pub fn club_income(&mut self, club: ClubId, amount: i32, kind: i32) {
        let c = self.club_mut(club);
        c.cash += amount as i64;
        if c.human {
            let f = &mut c.finances;
            match kind {
                1 => f.income_transfers += amount as i64,
                3 => f.income_type3 += amount,
                5 => f.income_type5 += amount,
                6 => f.income_type6 += amount,
                9 => f.income_type9 += amount,
                _ => {}
            }
        }
    }

    /// `ah.w(int, int)`: a payment; a human club's finances count it by kind (`C.i(int, int)`).
    pub fn club_payment(&mut self, club: ClubId, amount: i32, kind: i32) {
        let c = self.club_mut(club);
        c.cash -= amount as i64;
        if c.human {
            let f = &mut c.finances;
            match kind {
                1 => f.expense_transfers += amount as i64,
                4 => f.expense_type4 += amount,
                7 => f.expense_type7 += amount,
                8 => f.expense_type8 += amount,
                _ => f.expense_other += amount,
            }
        }
    }

    /// `ah.f(long)`: an expense; a human club's finances count it as wages (`C.b(long)`).
    pub fn club_expense(&mut self, club: ClubId, amount: i64) {
        let c = self.club_mut(club);
        c.cash -= amount;
        if c.human {
            c.finances.expense_salaries += amount;
        }
    }
}
