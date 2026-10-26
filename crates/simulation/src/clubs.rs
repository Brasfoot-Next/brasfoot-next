// Port of the record keeping of best.ah: competition records (aj), stage records (ak) and the
// table position band of ah.p(at).

use brasfoot_model::ids::{ClubId, CompetitionId, MatchId, StageId};
use brasfoot_model::saved::{CompetitionKind, CompetitionRecord, StageKind, StageRecord};
use brasfoot_model::world::World;

use crate::Sim;

/// `ah.d(f.s)`: points, games, wins, draws, losses, goals for, goals against, goal difference.
pub fn stage_record_numbers(world: &World, club: ClubId, stage: StageId) -> [i32; 8] {
    match world.club(club).stage_records.iter().find(|r| r.stage == Some(stage)) {
        Some(r) => [r.points, r.games, r.wins, r.games - (r.wins + r.losses), r.losses, r.goals_for, r.goals_against, r.goals_for - r.goals_against],
        None => [0; 8],
    }
}

impl Sim<'_> {
    /// `ah.a(at, int)`: this season's record of the club in a competition (and phase), created
    /// on first use.
    pub fn competition_record(&mut self, club: ClubId, competition: CompetitionId, subkind: i32) -> &mut CompetitionRecord {
        let season = self.world.season();
        let c = self.world.competition(competition);
        let kind = c.kind;
        let division = match c.class {
            CompetitionKind::NationalLeague | CompetitionKind::StateChampionship => c.division,
            _ => -1,
        };
        let records = &mut self.world.club_mut(club).competition_records;
        let i = match records.iter().position(|r| r.season == season && r.competition_kind == kind && r.competition_subkind == subkind) {
            Some(i) => i,
            None => {
                records.push(CompetitionRecord { season, competition_kind: kind, competition_subkind: subkind, division, ..Default::default() });
                records.len() - 1
            }
        };
        &mut records[i]
    }

    /// `ah.e(int, int, int)`: the all-time numbers per competition kind.
    fn all_time(&mut self, club: ClubId, kind: i32, column: usize, amount: i32) {
        if kind <= 14 {
            self.world.club_mut(club).all_time_record[kind as usize][column] += amount;
        }
    }

    /// `ah.e(I)`: the club's record of a played match.
    pub fn club_match_record(&mut self, club: ClubId, m: MatchId) {
        let g = self.world.game_match(m);
        let Some(competition) = g.competition else { return };
        let (scored, conceded) = if g.home == Some(club) {
            (g.home_goals, g.away_goals)
        } else if g.away == Some(club) {
            (g.away_goals, g.home_goals)
        } else {
            (0, 0)
        };
        let mut subkind = self.world.competition(competition).subkind;
        if let Some(stage) = g.stage {
            let s = self.world.stage(stage);
            if s.class == StageKind::Knockout && s.phase_code == 1098 {
                subkind = 1098;
            }
        }
        let kind = self.world.competition(competition).kind;
        let record = self.competition_record(club, competition, subkind);
        record.games += 1;
        record.goals_for += scored;
        record.goals_against += conceded;
        let won = scored > conceded;
        let lost = scored < conceded;
        if won {
            record.wins += 1;
        } else if lost {
            record.losses += 1;
        }
        self.all_time(club, kind, 1, scored);
        self.all_time(club, kind, 2, conceded);
        if won {
            self.all_time(club, kind, 0, 3);
        } else if !lost {
            self.all_time(club, kind, 0, 1);
        }
    }

    /// `ah.a(I, f.s)`: the club's numbers in a stage of a played match.
    pub fn stage_match_record(&mut self, club: ClubId, m: MatchId, stage: StageId) {
        let g = self.world.game_match(m);
        let (scored, conceded) = if g.home == Some(club) {
            (g.home_goals, g.away_goals)
        } else if g.away == Some(club) {
            (g.away_goals, g.home_goals)
        } else {
            (0, 0)
        };
        let records = &mut self.world.club_mut(club).stage_records;
        let i = match records.iter().position(|r| r.stage == Some(stage)) {
            Some(i) => i,
            None => {
                records.push(StageRecord { stage: Some(stage), ..Default::default() });
                records.len() - 1
            }
        };
        let r = &mut records[i];
        r.games += 1;
        r.goals_for += scored;
        r.goals_against += conceded;
        if scored > conceded {
            r.wins += 1;
            r.points += 3;
        } else if scored < conceded {
            r.losses += 1;
        } else {
            r.points += 1;
        }
    }

    /// `ah.p(at)`: the club's place in a league ({position, band, flag}); band 1 champion,
    /// 2 top, 3 middle, 4 near the drop, 5 relegated; flag -1 outside a league table.
    pub fn table_band(&self, club: ClubId, competition: Option<CompetitionId>) -> [i32; 3] {
        let mut result = [0; 3];
        let mut position = -1;
        let (mut size, mut relegated, mut top) = (0, 0, 0);
        if let Some(c) = competition {
            let comp = self.world.competition(c);
            if comp.kind == 1 || comp.kind == 3 {
                let stage = comp.league.expect("a league competition has its table");
                position = crate::league::position_in(self.world, stage, club);
                size = self.world.stage(stage).clubs.len() as i32;
                relegated = crate::league::relegated_places(self.world, stage);
                top = if comp.kind == 3 { 2 } else { 4 };
            }
        }
        if position == -1 {
            result[2] = -1;
        } else {
            let margin = if size < 20 { 2 } else { 4 };
            let band = if position == 1 {
                1
            } else if position > size - relegated {
                5
            } else if position <= top {
                2
            } else if position > size - (relegated + margin) {
                4
            } else {
                3
            };
            result[0] = position;
            result[1] = band;
        }
        result
    }
}
