// Port of the creation of a match, best.I(L, int, ah, ah, int, at, v) with I.hb(), and of the
// calendar's free dates, best.a.a(int, boolean).

use brasfoot_model::ids::{ClubId, CompetitionId, MatchId, StadiumId, StageId};
use brasfoot_model::saved::Match;

use crate::Sim;

impl Sim<'_> {
    /// `best.a.a(int, boolean)`: the days of a kind not played yet, skipping the first two.
    pub fn free_days(&self, kind: i32, skip_two: bool) -> Vec<usize> {
        let mut skip = if skip_two { 2 } else { 0 };
        let mut days = Vec::new();
        for (i, d) in self.world.game.days.iter().enumerate() {
            if d.kind == kind && !d.done {
                if skip <= 0 {
                    days.push(i);
                }
                if skip > 0 {
                    skip -= 1;
                }
            }
        }
        days
    }

    /// `best.I(L, int, ah, ah, int, at, v)`: a match on a calendar day, at the home club's
    /// ground unless `stadium` says otherwise. `round` only picks the venue of continental finals.
    pub fn new_match(
        &mut self,
        stage: StageId,
        round: i32,
        home: ClubId,
        away: ClubId,
        day: usize,
        competition: CompetitionId,
        stadium: Option<StadiumId>,
    ) -> MatchId {
        let mut m = Match {
            stage: Some(stage),
            home: Some(home),
            away: Some(away),
            day: day as i32,
            competition: Some(competition),
            stadium: stadium.or(self.world.club(home).stadium),
            ..Default::default()
        };
        if let Some(s) = m.stadium {
            m.stadium_condition = self.world.stadium(s).condition;
        }
        let stage_kind = self.world.stage(stage).kind_code();
        if stage_kind == 9 {
            // a qualifier: a ground of the home team's country (c.a.SR.s(pais))
            let country = self.world.club(home).country;
            if let Some(z) = self.any_country(country) {
                m.stadium = self.national_ground(z, false);
            }
        } else if matches!(stage_kind, 4 | 6 | 12) {
            let subkind = self.world.competition(competition).subkind;
            if (subkind == 0 || subkind == 1) && self.final_on_neutral_ground(competition, round) {
                // at.mF(): the year's host country of the final
                m.stadium = None;
                if let Some(z) = self.final_host(competition) {
                    m.stadium = self.national_ground(z, stage_kind != 6);
                }
            }
        }
        let id = MatchId(self.world.matches.len() as u32);
        self.world.matches.push(m);
        let d = &mut self.world.game.days[day];
        d.matches.push(id);
        if !d.stages.contains(&stage) {
            d.stages.push(stage);
        }
        if !d.competitions.contains(&competition) {
            d.competitions.push(competition);
        }
        let kind = self.world.competition(competition).kind;
        if kind == 5 || kind == 15 {
            self.world.game_match_mut(id).stadium = None;
        }
        self.plan_changes(id);
        id
    }

    /// `I.hb()`: the minutes the AI will make its changes, drawn from the shared pools.
    fn plan_changes(&mut self, m: MatchId) {
        let pools = &mut self.runtime.minute_pools.pools;
        for (i, range) in [(19, 38), (5, 15), (16, 35), (36, 42), (43, 47)].into_iter().enumerate() {
            if pools[i].is_none() {
                pools[i] = Some((range.0..=range.1).collect());
            }
        }
        let rng = &mut self.runtime.rng;
        rng.shuffle(pools[0].as_mut().unwrap());
        let mut random = rng.new_random();
        let draw = random.next_int_bounded(100);
        let mut tactical = vec![vec![-1; 3]; 2];
        let mut substitutions = vec![vec![-1; 4]; 2];
        let late = pools[0].as_ref().unwrap();
        tactical[0][0] = late[0];
        tactical[0][1] = late[1];
        tactical[1][0] = late[2];
        tactical[1][1] = late[3];
        if random.next_int_bounded(100) > 30 {
            tactical[0][2] = late[4];
        }
        if random.next_int_bounded(100) > 30 {
            tactical[1][2] = late[5];
        }
        // I.F(ArrayList): early, middle or late first changes
        let pool = if draw > 90 { 1 } else if draw > 50 { 2 } else { 3 };
        let list = pools[pool].as_mut().unwrap();
        rng.shuffle(list);
        substitutions[0][0] = list[0];
        substitutions[0][1] = list[1];
        substitutions[1][0] = list[2];
        substitutions[1][1] = list[3];
        let last = pools[4].as_mut().unwrap();
        rng.shuffle(last);
        if random.next_int_bounded(100) > 20 {
            substitutions[0][2] = last[0];
        }
        if random.next_int_bounded(100) > 50 {
            substitutions[0][3] = last[1];
        }
        if random.next_int_bounded(100) > 20 {
            substitutions[1][2] = last[2];
        }
        if random.next_int_bounded(100) > 50 {
            substitutions[1][3] = last[3];
        }
        let g = self.world.game_match_mut(m);
        g.tactical_change_minutes = tactical;
        g.substitution_minutes = substitutions;
    }
}

impl Sim<'_> {
    /// `Z.C(boolean)`: one of the country's six biggest grounds (`f.bw` in its order), at random,
    /// or with `first_two` one of the first two (none when the draw falls past a single one).
    pub(crate) fn national_ground(&mut self, country: brasfoot_model::ids::CountryId, first_two: bool) -> Option<StadiumId> {
        if !self.runtime.national_grounds.contains_key(&country) {
            let id = self.world.country(country).country;
            let all = self.stadiums_by_size();
            let grounds: Vec<StadiumId> = all.into_iter().filter(|&s| self.world.stadium(s).country == id).take(6).collect();
            self.runtime.national_grounds.insert(country, grounds);
        }
        let grounds = self.runtime.national_grounds[&country].clone();
        if grounds.is_empty() {
            return None;
        }
        if !first_two {
            return Some(grounds[self.runtime.rng.fresh_int(grounds.len() as i32) as usize]);
        }
        let i = self.runtime.rng.fresh_int(2) as usize;
        grounds.get(i).copied()
    }

    /// `f.cb()`: the clubs' stadiums, biggest first.
    fn stadiums_by_size(&mut self) -> Vec<StadiumId> {
        if self.runtime.stadiums_by_size.is_none() {
            let mut list: Vec<StadiumId> = self.world.game.clubs.iter().filter_map(|&c| self.world.club(c).stadium).collect();
            list.sort_by(|&a, &b| {
                let size = |s: StadiumId| self.world.stadium(s).capacity.iter().sum::<i32>();
                size(b).cmp(&size(a))
            });
            self.runtime.stadiums_by_size = Some(list);
        }
        self.runtime.stadiums_by_size.clone().unwrap_or_default()
    }

    /// `at.cz(int)`: the round is the final of a cup that plays it on neutral ground (f.v, f.r and
    /// f.H by their main stage's knockout, f.x and f.J by their playoff round).
    fn final_on_neutral_ground(&self, competition: CompetitionId, round: i32) -> bool {
        use brasfoot_model::saved::CompetitionKind::*;
        let c = self.world.competition(competition);
        let knockout = match c.class {
            ChampionsLeague | Libertadores | CopaSudamericana => c.main_stage.and_then(|s| self.world.stage(s).knockout),
            EuropaLeague | ConferenceLeague => c.playoff_round,
            _ => None,
        };
        knockout.is_some_and(|k| self.world.stage(k).last_round == round)
    }

    /// `at.mF()`: the country of the year's final, by the season.
    fn final_host(&self, competition: CompetitionId) -> Option<brasfoot_model::ids::CountryId> {
        use brasfoot_model::saved::CompetitionKind::*;
        let i = ((self.world.season() - 1) % 10) as usize;
        let hosts: [i32; 10] = match self.world.competition(competition).class {
            CopaSudamericana => [11, 29, 150, 42, 46, 151, 195, 26, 198, 60],
            ChampionsLeague => [192, 162, 3, 104, 154, 97, 72, 152, 65, 52],
            Libertadores => [42, 11, 29, 46, 150, 195, 26, 198, 60, 151],
            EuropaLeague | ConferenceLeague => [152, 65, 52, 31, 21, 154, 3, 72, 162, 97],
            _ => return None,
        };
        self.any_country(hosts[i])
    }
}
