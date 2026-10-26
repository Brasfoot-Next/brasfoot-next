// Port of the World Cup's qualifiers (f.n South America, f.l Europe with `age` 7, f.i Africa,
// f.j Asia and the intercontinental playoff, f.k Concacaf, f.m Oceania) and of the World Cup of
// 48 that follows them (f.q.Bu()).

use brasfoot_model::ids::{ClubId, CompetitionId, CountryId, StageId};
use brasfoot_model::saved::CompetitionKind;

use crate::Sim;
use crate::sorting::{by_level, by_level_and_reputation, standings};
use crate::stages::StageOptions;

impl Sim<'_> {
    /// `f.q.eY(int)`: the World Cup's host so many editions on.
    pub(crate) fn world_cup_host(&self, ahead: i32) -> Option<CountryId> {
        const HOSTS: [i32; 10] = [39, 131, 65, 11, 104, 97, 43, 3, 29, 162];
        let cup = self.world.game.world_cup?;
        let i = self.world.competition(cup).edition_counter + ahead - 1;
        let i = i % 10;
        if (0..10).contains(&i) { self.any_country(HOSTS[i as usize]) } else { None }
    }

    /// `f.q.a(at, Z)`: the country goes to the World Cup (`YM`), division 1 in its record of the
    /// qualifier (`ah.e(at, int)`).
    fn qualify_for_world_cup(&mut self, qualifier: CompetitionId, team: ClubId) {
        let id = self.world.club(team).country;
        let country = self.any_country(id).expect("a country of the game");
        let cup = self.world.game.world_cup.expect("the World Cup of the game");
        self.world.competition_mut(cup).list_ym.push(country);
        let subkind = self.world.competition(qualifier).subkind;
        let team = self.national_team(country);
        self.competition_record(team, qualifier, subkind).division = 1;
    }

    /// `c.a.SR.s(pais).z(true)` of a team: a new coach and squad.
    fn call_up_team(&mut self, team: ClubId, new_coach: bool) {
        let id = self.world.club(team).country;
        if let Some(country) = self.any_country(id) {
            self.call_up(country, new_coach);
        }
    }

    /// The confederation's countries with a national team (`jo()` makes the missing ones), by
    /// level (`Z.cN`), as their teams.
    fn confederation_teams(&mut self, confederation: i32) -> Vec<ClubId> {
        let mut countries: Vec<CountryId> = Vec::new();
        for c in self.world.game.countries.clone() {
            self.national_team(c);
            if brasfoot_model::countries::confederation(self.world.country(c).country) == confederation {
                countries.push(c);
            }
        }
        countries.sort_by(|&a, &b| {
            let (ca, cb) = (self.world.country(a), self.world.country(b));
            cb.level.cmp(&ca.level).then(cb.value_hr.cmp(&ca.value_hr))
        });
        countries.iter().map(|&c| self.world.country(c).national_team.expect("made above")).collect()
    }

    /// The listed countries' teams (made when missing), each calling up with a new coach.
    fn listed_teams(&mut self, ids: &[i32]) -> Vec<ClubId> {
        let mut teams = Vec::new();
        for &id in ids {
            let country = self.any_country(id).expect("a country of the game");
            teams.push(self.national_team(country));
            self.call_up(country, true);
        }
        teams
    }

    /// `f.ac()` the season before a World Cup: every confederation's qualifiers start.
    pub(crate) fn world_cup_qualifiers_draw(&mut self) {
        let q = self.national_competition(CompetitionKind::SouthAmericanQualifiers, 9, 1);
        self.world.game.south_american_qualifiers = Some(q);
        self.south_american_qualifiers_draw(q);
        let q = self.national_competition(CompetitionKind::EuroQualifiers, 9, 0);
        self.world.game.euro_qualifiers = Some(q);
        self.euro_qualifiers_draw(q, 7);
        let q = self.national_competition(CompetitionKind::AfricanQualifiers, 9, 2);
        self.world.game.african_qualifiers = Some(q);
        self.african_qualifiers_draw(q);
        let q = self.national_competition(CompetitionKind::ConcacafQualifiers, 9, 4);
        self.world.game.concacaf_qualifiers = Some(q);
        self.concacaf_qualifiers_draw(q);
        let q = self.national_competition(CompetitionKind::AsianQualifiers, 9, 3);
        self.world.game.asian_qualifiers = Some(q);
        self.asian_qualifiers_draw(q);
        let q = self.national_competition(CompetitionKind::OceaniaQualifiers, 9, 5);
        self.world.game.oceania_qualifiers = Some(q);
        self.oceania_qualifiers_draw(q);
    }

    /// `f.n.ya()`: South America: ten countries (nine when the next host is one of them), each
    /// with a new coach, shuffled, in one table home and away (9100).
    fn south_american_qualifiers_draw(&mut self, q: CompetitionId) {
        let c = self.world.competition_mut(q);
        c.countries.clear();
        c.clubs.clear();
        let mut countries = Vec::new();
        self.pick_countries(false, 1, 10, &mut countries, true, None);
        self.world.competition_mut(q).countries = countries.clone();
        let mut teams = Vec::new();
        if countries.len() == 10 || countries.len() == 9 {
            for &country in &countries {
                teams.push(self.national_team(country));
                self.call_up(country, true);
            }
            self.runtime.rng.shuffle(&mut teams);
            let options = StageOptions { teams: teams.len(), groups: 0, two_rounds: true, games_within_group: true, format_code: 9100, ..Default::default() };
            let stage = self.new_league_stage(&options, &teams, 0, 9, q, true);
            self.world.stage_mut(stage).name = self.world.competition(q).name.clone();
            self.world.competition_mut(q).main_stage = Some(stage);
        }
        self.world.competition_mut(q).clubs = teams.clone();
        self.job_market_countries(&teams);
    }

    /// `f.n.Bm()` (the end of 9100): the first six to the World Cup, the seventh to the playoff.
    pub(crate) fn south_american_qualifiers_end(&mut self, q: CompetitionId) {
        let main = self.world.competition(q).main_stage.expect("the qualifiers' table");
        let table = self.world.stage(main).clubs.clone();
        for &t in &table[..6] {
            self.qualify_for_world_cup(q, t);
        }
        self.world.competition_mut(q).club_agf = Some(table[6]);
    }

    /// `f.i.Bj()`: Africa: the best 26 wait for the groups; the next 28, the 27th against the
    /// 41st and so on, play a preliminary round home and away (9200).
    fn african_qualifiers_draw(&mut self, q: CompetitionId) {
        let teams = self.confederation_teams(2);
        let seeded = teams[0..=25].to_vec();
        let mut ties = Vec::new();
        for i in 0..14 {
            ties.push(teams[26 + i]);
            ties.push(teams[40 + i]);
        }
        self.world.competition_mut(q).clubs.extend_from_slice(&seeded);
        let all = self.world.competition(q).clubs.clone();
        for &t in all.iter().chain(&ties) {
            self.call_up_team(t, true);
        }
        let stage = self.cup_knockout(q, &ties, 1, vec![true; 7], 9200);
        self.world.competition_mut(q).final_round = Some(stage);
        self.job_market_countries(&ties);
        self.job_market_countries(&all);
    }

    /// `aw(ArrayList)` of f.i (9200) and f.j (9300): the winners and the seeded by level in
    /// pots, eight groups of five home and away (9201 with the winners on, 9301).
    pub(crate) fn qualifiers_groups(&mut self, q: CompetitionId, winners: &[ClubId]) {
        let africa = self.world.competition(q).class == CompetitionKind::AfricanQualifiers;
        let mut clubs = winners.to_vec();
        clubs.extend_from_slice(&self.world.competition(q).clubs);
        clubs.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        let (pots, per) = if africa { (4, 10) } else { (5, 8) };
        let mut pot_lists: Vec<Vec<ClubId>> = (0..pots).map(|p| clubs[p * per..(p + 1) * per].to_vec()).collect();
        for pot in pot_lists.iter_mut() {
            self.runtime.rng.shuffle(pot);
        }
        let mut drawn = Vec::new();
        for i in 0..per {
            for pot in &pot_lists {
                drawn.push(pot[i]);
            }
        }
        if drawn.len() == 40 {
            let options = StageOptions {
                teams: 40,
                groups: 8,
                knockout_teams: if africa { 1 } else { 0 },
                two_rounds: true,
                skip_two_dates: false,
                two_legged_knockout: vec![true, true, true, true, false, false, false],
                ..Default::default()
            };
            let kind = self.world.competition(q).kind;
            let stage = self.new_league_stage(&options, &drawn, 0, kind, q, true);
            self.world.stage_mut(stage).format_code = if africa { 9201 } else { 9301 };
            self.world.stage_mut(stage).name = self.world.competition(q).name.clone();
            self.world.competition_mut(q).main_stage = Some(stage);
        }
    }

    /// The seconds of a stage's groups by the stage's table (`s.r(YD)`, `h.a.abn`).
    fn best_of_place(&mut self, stage: StageId, place: usize) -> Vec<ClubId> {
        self.runtime.table_stage = Some(stage);
        let mut list: Vec<ClubId> = self.world.stage(stage).groups.iter().map(|g| g.clubs[place]).collect();
        list.sort_by(|&a, &b| standings(self.world, stage, a, b));
        list
    }

    /// `f.i.Bk()` (the end of 9201): the four best seconds, shuffled, play a knockout home and
    /// away (9202).
    pub(crate) fn african_qualifiers_playoff(&mut self, q: CompetitionId) {
        let main = self.world.competition(q).main_stage.expect("the qualifiers' groups");
        let mut best = self.best_of_place(main, 1)[..4].to_vec();
        self.runtime.rng.shuffle(&mut best);
        let stage = self.cup_knockout(q, &best, 0, vec![true; 7], 9202);
        self.world.competition_mut(q).knockout_afv = Some(stage);
    }

    /// `f.i.p(ArrayList, ArrayList)` (the end of 9202): the group winners and the playoff's
    /// winner to the World Cup, its finalist to the intercontinental playoff.
    pub(crate) fn african_qualifiers_end(&mut self, q: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        let main = self.world.competition(q).main_stage.expect("the qualifiers' groups");
        let mut qualified: Vec<ClubId> = self.world.stage(main).groups.iter().map(|g| g.clubs[0]).collect();
        qualified.push(winners[0]);
        self.world.competition_mut(q).club_afw = Some(losers[0]);
        for t in qualified {
            self.qualify_for_world_cup(q, t);
        }
    }

    /// `f.j.Bj()`: Asia: the best 34 wait for the groups; the next twelve, the 35th against the
    /// 41st and so on, play a preliminary round home and away (9300).
    fn asian_qualifiers_draw(&mut self, q: CompetitionId) {
        let teams = self.confederation_teams(3);
        let seeded = teams[0..=33].to_vec();
        let mut ties = Vec::new();
        for i in 0..6 {
            ties.push(teams[34 + i]);
            ties.push(teams[40 + i]);
        }
        self.world.competition_mut(q).clubs.extend_from_slice(&seeded);
        let all = self.world.competition(q).clubs.clone();
        for &t in all.iter().chain(&ties) {
            self.call_up_team(t, true);
        }
        let stage = self.cup_knockout(q, &ties, 1, vec![true; 7], 9300);
        self.world.competition_mut(q).final_round = Some(stage);
        self.job_market_countries(&ties);
        self.job_market_countries(&all);
    }

    /// `f.j.yt()` (the end of 9301): the group winners and the four best seconds by level in
    /// two groups of six home and away (9302).
    pub(crate) fn asian_qualifiers_final_stage(&mut self, q: CompetitionId) {
        let main = self.world.competition(q).main_stage.expect("the qualifiers' groups");
        let mut clubs: Vec<ClubId> = self.world.stage(main).groups.iter().map(|g| g.clubs[0]).collect();
        clubs.extend_from_slice(&self.best_of_place(main, 1)[..4]);
        clubs.sort_by(|&a, &b| by_level(self.world, a, b));
        let mut drawn = Vec::new();
        for i in 0..6 {
            drawn.push(clubs[i]);
            drawn.push(clubs[6 + i]);
        }
        let options = StageOptions {
            teams: 12,
            groups: 2,
            two_rounds: true,
            skip_two_dates: false,
            two_legged_knockout: vec![true, true, true, true, false, false, false],
            ..Default::default()
        };
        let kind = self.world.competition(q).kind;
        let stage = self.new_league_stage(&options, &drawn, 0, kind, q, true);
        self.world.stage_mut(stage).format_code = 9302;
        self.world.stage_mut(stage).name = self.world.competition(q).name.clone();
        self.world.competition_mut(q).stage_afx = Some(stage);
    }

    /// `f.j.Bm()` (the end of 9302): the first four of each group to the World Cup; the fifths
    /// play off (9303).
    pub(crate) fn asian_qualifiers_end(&mut self, q: CompetitionId) {
        let stage = self.world.competition(q).stage_afx.expect("the final stage");
        let groups = self.world.stage(stage).groups.clone();
        for g in &groups {
            for &t in &g.clubs[..4] {
                self.qualify_for_world_cup(q, t);
            }
        }
        let fifths = vec![groups[0].clubs[4], groups[1].clubs[4]];
        let ko = self.cup_knockout(q, &fifths, 1, vec![false, false, false, false, true, true, true], 9303);
        self.world.competition_mut(q).knockout_afy = Some(ko);
    }

    /// `f.j.ax(ArrayList)` (the end of 9303): the intercontinental playoff of six in one round
    /// (7701): South America's seventh, Asia's winner, Concacaf's seventh and eighth, Africa's
    /// finalist and Oceania's.
    pub(crate) fn intercontinental_playoff(&mut self, q: CompetitionId, winners: &[ClubId]) {
        let g = &self.world.game;
        let club = |c: Option<CompetitionId>, f: fn(&brasfoot_model::saved::Competition) -> Option<ClubId>| c.and_then(|c| f(self.world.competition(c)));
        let teams: Vec<Option<ClubId>> = vec![
            club(g.south_american_qualifiers, |c| c.club_agf),
            Some(winners[0]),
            club(g.concacaf_qualifiers, |c| c.club_afw),
            self.world.competition(q).club_aga,
            club(g.african_qualifiers, |c| c.club_afw),
            club(g.oceania_qualifiers, |c| c.club_agf),
        ];
        let Some(teams) = teams.into_iter().collect::<Option<Vec<ClubId>>>() else {
            crate::unported!("f.j.ax(ArrayList) with a confederation's playoff place missing");
        };
        let options = StageOptions { teams: 6, two_rounds: false, ..Default::default() };
        let kind = self.world.competition(q).kind;
        let stage = self.new_league_stage(&options, &teams, 0, kind, q, true);
        self.world.stage_mut(stage).format_code = 7701;
        self.world.competition_mut(q).stage_afz = Some(stage);
    }

    /// `f.j.Bn()` (the end of 7701): its first two to the World Cup.
    pub(crate) fn intercontinental_playoff_end(&mut self, q: CompetitionId) {
        let stage = self.world.competition(q).stage_afz.expect("the playoff");
        let table = self.world.stage(stage).clubs.clone();
        for &t in &table[..2] {
            self.qualify_for_world_cup(q, t);
        }
    }

    /// `f.k.Bj()`: Concacaf: its 35 countries by level: the best five wait for the final stage;
    /// the next thirty in six groups of five, one game each (9400). The pots are shuffled after
    /// the groups are drawn, which only spends draws.
    fn concacaf_qualifiers_draw(&mut self, q: CompetitionId) {
        const IDS: [i32; 35] = [131, 68, 51, 106, 86, 58, 38, 8, 147, 84, 189, 7, 79, 166, 181, 139, 158, 77, 20, 80, 168, 24, 22, 165, 153, 53, 212, 56, 90, 17, 13, 213, 211, 94, 6];
        self.world.competition_mut(q).clubs_agc.clear();
        let mut teams = self.listed_teams(&IDS);
        teams.sort_by(|&a, &b| by_level(self.world, a, b));
        self.world.competition_mut(q).clubs_agc = teams[..5].to_vec();
        let mut pots: Vec<Vec<ClubId>> = (0..5).map(|p| teams[5 + p * 6..5 + (p + 1) * 6].to_vec()).collect();
        let mut drawn = Vec::new();
        for i in 0..6 {
            for pot in &pots {
                drawn.push(pot[i]);
            }
        }
        for pot in pots.iter_mut() {
            self.runtime.rng.shuffle(pot);
        }
        let options = StageOptions { teams: 30, groups: 6, two_rounds: false, ..Default::default() };
        let stage = self.new_league_stage(&options, &drawn, 0, 9, q, true);
        self.world.stage_mut(stage).name = self.world.competition(q).name.clone();
        self.world.stage_mut(stage).format_code = 9400;
        self.world.competition_mut(q).stage_agb = Some(stage);
        self.job_market_countries(&drawn);
        let seeded = self.world.competition(q).clubs_agc.clone();
        self.job_market_countries(&seeded);
    }

    /// `f.k.Bp()` (the end of 9400): the group winners play a round home and away (9401).
    pub(crate) fn concacaf_qualifiers_knockout(&mut self, q: CompetitionId) {
        let stage = self.world.competition(q).stage_agb.expect("the first stage");
        let winners: Vec<ClubId> = self.world.stage(stage).groups.iter().map(|g| g.clubs[0]).collect();
        let ko = self.cup_knockout(q, &winners, 1, vec![true, true, true, false, false, false, false], 9401);
        self.world.competition_mut(q).knockout_agd = Some(ko);
    }

    /// `f.k.ay(ArrayList)` (the end of 9401): the five seeded and the winners in one table home
    /// and away (9402).
    pub(crate) fn concacaf_qualifiers_final_stage(&mut self, q: CompetitionId, winners: &[ClubId]) {
        self.world.competition_mut(q).clubs_agc.extend_from_slice(winners);
        let teams = self.world.competition(q).clubs_agc.clone();
        let options = StageOptions { teams: 8, two_rounds: true, ..Default::default() };
        let kind = self.world.competition(q).kind;
        let stage = self.new_league_stage(&options, &teams, 0, kind, q, true);
        self.world.stage_mut(stage).format_code = 9402;
        self.world.stage_mut(stage).name = self.world.competition(q).name.clone();
        self.world.competition_mut(q).stage_afx = Some(stage);
    }

    /// `f.k.Bm()` (the end of 9402): the first six to the World Cup, the seventh and the eighth
    /// (`f.j.X`) to the intercontinental playoff.
    pub(crate) fn concacaf_qualifiers_end(&mut self, q: CompetitionId) {
        let stage = self.world.competition(q).stage_afx.expect("the final stage");
        let table = self.world.stage(stage).clubs.clone();
        for &t in &table[..6] {
            self.qualify_for_world_cup(q, t);
        }
        self.world.competition_mut(q).club_afw = Some(table[6]);
        if let Some(asia) = self.world.game.asian_qualifiers {
            self.world.competition_mut(asia).club_aga = Some(table[7]);
        }
    }

    /// `f.m.Bj()`: Oceania: its eleven countries by level: seven wait for the groups, the other
    /// four play one table home and away (9500).
    fn oceania_qualifiers_draw(&mut self, q: CompetitionId) {
        const IDS: [i32; 11] = [214, 91, 163, 188, 69, 215, 143, 148, 93, 184, 197];
        self.world.competition_mut(q).clubs.clear();
        let mut teams = self.listed_teams(&IDS);
        teams.sort_by(|&a, &b| by_level(self.world, a, b));
        self.world.competition_mut(q).clubs = teams[..7].to_vec();
        let rest = teams[7..].to_vec();
        let options = StageOptions { teams: 4, groups: 0, two_rounds: true, ..Default::default() };
        let stage = self.new_league_stage(&options, &rest, 0, 9, q, true);
        self.world.stage_mut(stage).name = self.world.competition(q).name.clone();
        self.world.stage_mut(stage).format_code = 9500;
        self.world.competition_mut(q).stage_agb = Some(stage);
        self.job_market_countries(&rest);
        let seeded = self.world.competition(q).clubs.clone();
        self.job_market_countries(&seeded);
    }

    /// `f.m.Bq()` (the end of 9500): its winner and the seven in two groups home and away
    /// (9501).
    pub(crate) fn oceania_qualifiers_groups(&mut self, q: CompetitionId) {
        let stage = self.world.competition(q).stage_agb.expect("the first stage");
        let winner = self.world.stage(stage).clubs[0];
        self.world.competition_mut(q).clubs.push(winner);
        let teams = self.world.competition(q).clubs.clone();
        let options = StageOptions { teams: 8, groups: 2, two_rounds: true, ..Default::default() };
        let stage = self.new_league_stage(&options, &teams, 0, 9, q, true);
        self.world.stage_mut(stage).name = self.world.competition(q).name.clone();
        self.world.stage_mut(stage).format_code = 9501;
        self.world.competition_mut(q).main_stage = Some(stage);
    }

    /// `f.m.yt()` (the end of 9501): the first two of each group in a knockout home and away
    /// (9502).
    pub(crate) fn oceania_qualifiers_knockout(&mut self, q: CompetitionId) {
        let main = self.world.competition(q).main_stage.expect("the groups");
        let mut clubs = Vec::new();
        for g in &self.world.stage(main).groups {
            clubs.push(g.clubs[0]);
            clubs.push(g.clubs[1]);
        }
        let ko = self.cup_knockout(q, &clubs, 0, vec![true, true, true, false, false, false, false], 9502);
        self.world.competition_mut(q).playoff_round = Some(ko);
    }

    /// `f.m.p(ArrayList, ArrayList)` (the end of 9502): the winner to the World Cup, the
    /// finalist to the intercontinental playoff.
    pub(crate) fn oceania_qualifiers_end(&mut self, q: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        self.qualify_for_world_cup(q, winners[0]);
        self.world.competition_mut(q).club_agf = Some(losers[0]);
    }

    /// `f.l.az(ArrayList)` with `age` 7: the group winners and the playoff's winners to the
    /// World Cup.
    pub(crate) fn european_world_cup_qualified(&mut self, q: CompetitionId, winners: &[ClubId]) {
        let main = self.world.competition(q).main_stage.expect("the qualifiers' groups");
        let firsts: Vec<ClubId> = self.world.stage(main).groups.iter().map(|g| g.clubs[0]).collect();
        for t in firsts.into_iter().chain(winners.iter().copied()) {
            self.qualify_for_world_cup(q, t);
        }
    }

    /// `f.q.Bu()`: the World Cup of 48: its qualified by level, each with a new coach, in three
    /// pots of sixteen, sixteen groups of three (7700).
    pub(crate) fn world_cup_draw(&mut self, cup: CompetitionId) {
        self.world.competition_mut(cup).edition_counter += 1;
        let host = self.world_cup_host(0);
        let c = self.world.competition_mut(cup);
        c.host = host;
        c.countries = std::mem::take(&mut c.list_ym);
        let mut countries = c.countries.clone();
        countries.sort_by(|&a, &b| {
            let (ca, cb) = (self.world.country(a), self.world.country(b));
            cb.level.cmp(&ca.level).then(cb.value_hr.cmp(&ca.value_hr))
        });
        self.world.competition_mut(cup).countries = countries.clone();
        let mut teams = Vec::new();
        for &country in &countries {
            teams.push(self.national_team(country));
            self.call_up(country, true);
        }
        let mut pots: Vec<Vec<ClubId>> = (0..3).map(|p| teams[p * 16..(p + 1) * 16].to_vec()).collect();
        for pot in pots.iter_mut() {
            self.runtime.rng.shuffle(pot);
        }
        let mut drawn = Vec::new();
        for i in 0..16 {
            for pot in &pots {
                drawn.push(pot[i]);
            }
        }
        let options = StageOptions { teams: 48, groups: 16, two_rounds: false, knockout_teams: 2, two_legged_knockout: vec![false; 7], ..Default::default() };
        let stage = self.new_league_stage(&options, &drawn, 0, 7, cup, true);
        self.world.stage_mut(stage).format_code = 7700;
        self.world.stage_mut(stage).name = self.world.competition(cup).name.clone();
        self.world.competition_mut(cup).main_stage = Some(stage);
    }
}
