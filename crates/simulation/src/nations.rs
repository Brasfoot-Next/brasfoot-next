// Port of the national teams' competitions of the game: the end of their league stages and the
// knockouts that follow (f.N, f.M, f.q, f.x, f.J and the continental cups of national teams).

use brasfoot_model::ids::{ClubId, CompetitionId};

use crate::Sim;

impl Sim<'_> {
    /// `yt()` of f.M and f.N: the end of a Nations League's league A: its four group winners,
    /// shuffled, play the finals in the first one's country.
    pub(crate) fn nations_league_finals(&mut self, competition: CompetitionId) {
        let c = self.world.competition(competition);
        let league_a = c.league_a.expect("the Nations League has a league A");
        let mut winners: Vec<ClubId> = self.world.stage(league_a).groups.iter().take(4).map(|g| g.clubs[0]).collect();
        self.runtime.rng.shuffle(&mut winners);
        self.world.competition_mut(competition).value_agy = self.world.club(winners[0]).country;
        let kind = self.world.competition(competition).kind;
        let stage = self.new_knockout_stage(competition, None, winners.len() as i32, kind, 0, vec![false; 7], -1);
        self.world.competition_mut(competition).playoff_round = Some(stage);
        self.new_knockout_round(stage, &winners, 0, false, 0, kind, false);
    }

    /// `f.M.BB()`: the end of the Nations League's league C: the last of its four groups,
    /// shuffled, play the relegation playoff (14006).
    pub(crate) fn nations_league_relegation_playoff(&mut self, competition: CompetitionId) {
        let league_c = self.world.competition(competition).league_c.expect("the Nations League has a league C");
        let mut last: Vec<ClubId> = self.world.stage(league_c).groups.iter().take(4).map(|g| g.clubs[3]).collect();
        self.runtime.rng.shuffle(&mut last);
        let stage = self.cup_knockout(competition, &last, 1, vec![true, false, false, false, false, false, false], 14006);
        self.world.competition_mut(competition).knockout_agx = Some(stage);
    }

    /// `f.q.yt()`: the end of the World Cup's groups: the first two of each group, in the
    /// bracket's order (`Zw`), go to the knockout of the group stage.
    pub(crate) fn world_cup_knockout(&mut self, competition: CompetitionId) {
        /// `f.q.Zw`
        const BRACKET: [usize; 16] = [0, 3, 4, 7, 8, 11, 12, 15, 2, 1, 6, 5, 10, 9, 14, 13];
        let groups_stage = self.world.competition(competition).main_stage.expect("the World Cup has groups");
        let mut first_two = Vec::new();
        for g in &self.world.stage(groups_stage).groups {
            first_two.push(g.clubs[0]);
            first_two.push(g.clubs[1]);
        }
        let mut drawn = Vec::new();
        if first_two.len() == 16 {
            drawn = BRACKET.iter().map(|&i| first_two[i]).collect();
        }
        if first_two.len() == 32 {
            drawn.extend(BRACKET.iter().map(|&i| first_two[i]));
            drawn.extend(BRACKET.iter().map(|&i| first_two[i + 16]));
        }
        let kind = self.world.competition(competition).kind;
        let stage = self.new_knockout_stage(competition, None, drawn.len() as i32, kind, 0, vec![false; 7], -1);
        self.world.stage_mut(groups_stage).knockout = Some(stage);
        self.new_knockout_round(stage, &drawn, 0, false, 0, kind, false);
    }
}
