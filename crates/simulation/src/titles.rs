// Port of the end of a competition: the title record (best.i(at, L, ah, ah)), the top scorer
// (at.mu), the final places (f.z.z(at), ah.a(at, int, int)) with the reputation and ranking
// points they give the clubs and their coaches (ah.b, ah.d, al.a, al.b) and the trophies
// (ah.q, al.q).

use brasfoot_model::ids::{ClubId, CoachId, CompetitionId, StageId};
use brasfoot_model::rng::java_round;
use brasfoot_model::saved::{CompetitionKind, StageKind, TitleRecord, Trophy};

use crate::Sim;

/// `aq.fs(int)`: national-team competitions.
pub fn national_teams_kind(kind: i32) -> bool {
    matches!(kind, 7 | 9 | 13 | 14)
}

impl Sim<'_> {
    /// `at.ip()`: the division of a league, -1 for the other competitions.
    pub fn competition_division(&self, c: CompetitionId) -> i32 {
        let comp = self.world.competition(c);
        match comp.class {
            CompetitionKind::NationalLeague | CompetitionKind::StateChampionship => comp.division,
            _ => -1,
        }
    }

    /// `L.ip()`: a league stage's division; a knockout asks the stage before it.
    fn stage_division(&self, stage: StageId) -> i32 {
        let s = self.world.stage(stage);
        match s.class {
            StageKind::League => s.division,
            StageKind::Knockout => s.group_stage.map(|g| self.stage_division(g)).unwrap_or(-1),
        }
    }

    /// `at.mu()` with `at.mv()`: the scorers' games brought up to date, sorted by goals and then
    /// fewer games; the first one.
    fn top_scorer(&mut self, c: CompetitionId) -> Option<usize> {
        let mut scorers = std::mem::take(&mut self.world.competition_mut(c).scorers);
        if scorers.is_empty() {
            return None;
        }
        for s in &mut scorers {
            if let Some(p) = s.player
                && let Some(stats) = self.world.player(p).competition_stats.iter().find(|h| h.competition_id == Some(c))
            {
                s.games = stats.games;
            }
        }
        scorers.sort_by(|a, b| b.goals.cmp(&a.goals).then(a.games.cmp(&b.games)));
        self.world.competition_mut(c).scorers = scorers;
        Some(0)
    }

    /// `best.i(at, L, ah, ah)`: the season's title of a competition.
    pub fn record_title(&mut self, c: CompetitionId, stage: Option<StageId>, champion: Option<ClubId>, runner_up: Option<ClubId>) {
        let coach_id = |sim: &Self, club: Option<ClubId>| club.and_then(|x| sim.world.club(x).coach).map(|k| sim.world.coach(k).id).unwrap_or(-1);
        let mut record = TitleRecord {
            season: self.world.season(),
            champion: champion.map(|x| self.world.club(x).id).unwrap_or(-1),
            champion_coach: coach_id(self, champion),
            runner_up: runner_up.map(|x| self.world.club(x).id).unwrap_or(-1),
            runner_up_coach: coach_id(self, runner_up),
            ..Default::default()
        };
        let scorer = self.top_scorer(c).map(|i| self.world.competition(c).scorers[i].clone());
        let kind = self.world.competition(c).kind;
        self.world.competition_mut(c).titles.push(record.clone());
        self.title_hook(c);
        if let Some(scorer) = scorer {
            record.top_scorer_goals = scorer.goals;
            if let Some(p) = scorer.player {
                let player = self.world.player(p);
                record.top_scorer_name = Some(player.name.clone());
                if let Some(club) = player.club {
                    record.top_scorer_club = self.world.club(club).id;
                }
                if stage.is_some_and(|s| self.world.stage(s).kind_code() == 1 && self.stage_division(s) == 1) {
                    // F.a(Boolean): the top scorer of a first division becomes a star
                    self.world.player_mut(p).star = true;
                }
            }
            *self.world.competition_mut(c).titles.last_mut().expect("just added") = record;
        }
        if let Some(club) = champion.filter(|&x| self.world.club(x).human) {
            self.ask(|human, world| human.title(world, club, c));
        }
        if kind == 1 && self.world.competition(c).class == CompetitionKind::NationalLeague {
            self.league_season_over(c);
        }
    }

    /// `at.mr()`: what each competition does when its title is decided.
    fn title_hook(&mut self, c: CompetitionId) {
        match self.world.competition(c).class {
            CompetitionKind::NationalLeague => {
                // f.B.mr()
                let comp = self.world.competition(c);
                let table = comp.league.expect("a league has its table");
                if self.world.stage(table).format_code == 1020 && comp.second_league.is_some() {
                    crate::unported!("f.B.mr(): the places of a league with two tables");
                }
                self.league_places(table);
            }
            CompetitionKind::NationalSuperCup | CompetitionKind::SuperCupE | CompetitionKind::SuperCupD | CompetitionKind::Finalissima => {
                // f.I.mr(), f.E.mr(), f.D.mr(), f.K.mr()
                let knockout = self.world.competition(c).final_stage.expect("the super cup's final");
                self.knockout_places(knockout, c);
            }
            CompetitionKind::NationalCup => {
                let knockout = self.world.competition(c).knockout.expect("the cup's knockout");
                self.knockout_places(knockout, c);
            }
            CompetitionKind::WorldCup
            | CompetitionKind::EuropeanChampionship
            | CompetitionKind::CopaAmerica
            | CompetitionKind::AfricaCupOfNations
            | CompetitionKind::AsianCup
            | CompetitionKind::GoldCup
            | CompetitionKind::OceaniaNationsCup
            | CompetitionKind::FriendlyTournament => {
                // f.q.mr(), f.p, f.d, f.c, f.e, f.h, f.g, f.L: the groups' places
                let groups = self.world.competition(c).main_stage.expect("the World Cup's groups");
                self.league_places(groups);
            }
            CompetitionKind::NationsLeagueM | CompetitionKind::NationsLeagueN => {
                // f.M.mr(), f.N.mr(): the finals' places, then each team's league (BG())
                let comp = self.world.competition(c);
                let finals = comp.playoff_round.expect("the Nations League's finals");
                let leagues = [comp.league_a, comp.league_b, comp.league_c, comp.league_d];
                self.knockout_places(finals, c);
                let subkind = self.world.competition(c).subkind;
                for (i, league) in leagues.into_iter().enumerate() {
                    let Some(league) = league else { continue };
                    for club in self.world.stage(league).clubs.clone() {
                        // ah.e(at, int)
                        self.competition_record(club, c, subkind).division = i as i32 + 1;
                    }
                }
            }
            CompetitionKind::ChampionsLeague
            | CompetitionKind::Libertadores
            | CompetitionKind::CopaSudamericana
            | CompetitionKind::CafChampionsLeague
            | CompetitionKind::AfcChampionsLeague
            | CompetitionKind::ConcacafChampionsCup
            | CompetitionKind::OfcChampionsLeague => {
                // f.v.mr(), f.r.mr(), f.H.mr(), f.t.mr(), f.u.mr(), f.w.mr(), f.y.mr()
                let groups = self.world.competition(c).main_stage.expect("the cup's groups");
                self.league_places(groups);
            }
            CompetitionKind::EuropaLeague | CompetitionKind::ConferenceLeague => {
                // f.x.mr(), f.J.mr()
                let comp = self.world.competition(c);
                let groups = comp.main_stage.expect("the cup's groups");
                let knockout = comp.playoff_round.expect("the cup's final knockout");
                self.places_after_final_knockout(groups, knockout);
            }
            CompetitionKind::IntercontinentalCup => {
                // f.A.mr()
                let semi_finals = self.world.competition(c).semi_finals.expect("the world club cup's semi-finals");
                self.knockout_places(semi_finals, c);
            }
            other => crate::unported!(format!("at.mr() of {other:?}")),
        }
    }

    /// `f.z.z(at)`: the final places from the last round back, the winner of each tie first.
    fn knockout_places(&mut self, stage: StageId, c: CompetitionId) {
        let rounds = self.world.stage(stage).rounds.clone();
        let mut placed: Vec<ClubId> = Vec::new();
        let mut position = 1;
        for (depth, r) in rounds.iter().rev().enumerate() {
            for &m in &r.first_leg {
                let g = self.world.game_match(m);
                let (home, away) = (g.home.expect("home club"), g.away.expect("away club"));
                let (winner, loser) = if r.winners.contains(&home) {
                    (home, away)
                } else if r.winners.contains(&away) {
                    (away, home)
                } else {
                    crate::unported!("f.z.z(at) with a tie neither club won");
                };
                for club in [winner, loser] {
                    if !placed.contains(&club) {
                        placed.push(club);
                        self.place(club, c, position, depth as i32);
                        position += 1;
                    }
                }
            }
        }
    }

    /// `ah.a(at, int, int)`: a club's final place and the round it reached.
    pub fn place(&mut self, club: ClubId, c: CompetitionId, position: i32, round_reached: i32) {
        let subkind = self.world.competition(c).subkind;
        let record = self.competition_record(club, c, subkind);
        record.position = position;
        record.round_reached = round_reached;
        if position > 10 {
            return;
        }
        self.club_place_points(club, c, position);
        if let Some(coach) = self.world.club(club).coach {
            self.coach_place_points(coach, c, position);
        }
        let kind = self.world.competition(c).kind;
        if self.world.club(club).human && (kind == 1 || kind == 3) {
            self.league_prize(club, c, position);
        }
        if position == 1 {
            self.club_trophy(club, c);
        }
    }

    /// `ah.c(at, int)`: a human club's prize for its place in a national league (`aq.sr` by
    /// division) or a state league's first level (`aq.su`), with a message to its coach.
    fn league_prize(&mut self, club: ClubId, c: CompetitionId, position: i32) {
        const NATIONAL: [[i32; 6]; 5] = [
            [0; 6],
            [5000000, 3500000, 2000000, 1500000, 1000000, 500000],
            [2500000, 2000000, 1000000, 500000, 250000, 100000],
            [1500000, 1000000, 500000, 300000, 150000, 75000],
            [750000, 500000, 300000, 200000, 100000, 50000],
        ];
        let index = (position - 1) as usize;
        let division = self.world.club(club).division;
        let kind = self.world.competition(c).kind;
        let prize = if kind != 1 || division as usize >= NATIONAL.len() {
            if kind == 3 {
                crate::unported!("at.ip() of a state league: the prize of aq.su")
            }
            0
        } else {
            NATIONAL[division as usize].get(index).copied().unwrap_or(0)
        };
        if prize > 0 {
            self.world.club_income(club, prize, 3);
            if let Some(coach) = self.world.club(club).coach
                && self.world.coach(coach).human
            {
                let about = brasfoot_engine::messages::Composed { competition: Some(c), money: Some(prize as i64), ..Default::default() };
                brasfoot_engine::messages::send_composed(self.world, coach, 26, 80, about);
            }
        }
    }

    /// `ah.b(at, int, int)`: reputation for the first two places and ranking points.
    fn club_place_points(&mut self, club: ClubId, c: CompetitionId, position: i32) {
        let kind = self.world.competition(c).kind;
        let division = self.competition_division(c);
        let detail = match kind {
            1 => division,
            7 => self.world.competition(c).subkind,
            _ => -1,
        };
        if position == 1 || position == 2 {
            self.club_reputation(club, position - 1, kind, detail);
        }
        const POINTS: [[i32; 11]; 16] = [
            [0; 11],
            [0, 15, 10, 8, 7, 6, 5, 4, 3, 2, 1],
            [0, 12, 7, 3, 3, 0, 0, 0, 0, 0, 0],
            [0, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 20, 15, 7, 7, 3, 3, 0, 0, 0, 0],
            [0, 10, 5, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 12, 10, 5, 5, 0, 0, 0, 0, 0, 0],
            [0, 50, 35, 10, 5, 0, 0, 0, 0, 0, 0],
            [0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0; 11],
            [0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 10, 8, 4, 4, 0, 0, 0, 0, 0, 0],
            [0; 11],
            [0, 15, 10, 5, 0, 0, 0, 0, 0, 0, 0],
            [0; 11],
        ];
        let confederation = brasfoot_model::countries::confederation(self.world.club(club).country);
        let table: [i32; 11] = if kind == 7 {
            if detail != 7 { [0, 20, 15, 10, 5, 0, 0, 0, 0, 0, 0] } else { POINTS[7] }
        } else if kind == 1 && division > 1 {
            if division == 2 { [0, 5, 3, 2, 1, 0, 0, 0, 0, 0, 0] } else { [0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0] }
        } else if kind == 3 && division > 1 {
            [0; 11]
        } else if kind == 4 && confederation > 1 {
            [0, 18, 12, 3, 3, 1, 1, 0, 0, 0, 0]
        } else {
            POINTS[kind as usize]
        };
        if kind <= 14 && (0..11).contains(&position) {
            self.world.club_mut(club).ranking_points[kind as usize] += table[position as usize];
        }
    }

    /// `ah.d(int, int, int)`: reputation points of a club (and, through `al.a`, of its coach)
    /// for a title (0) or a second place (1). The original reaches the coach through the
    /// transient `mZ`, set once `ka()` ran since loading; a club that just played has it.
    fn club_reputation(&mut self, club: ClubId, place: i32, kind: i32, detail: i32) {
        const POINTS: [[i32; 15]; 2] = [
            [0, 500, 300, 10, 5000, 40000, 2000, 0, 500, 0, 50, 0, 0, 1000, 0],
            [0, 90, 50, 5, 1000, 1000, 500, 0, 0, 0, 0, 0, 0, 500, 0],
        ];
        let c = self.world.club(club);
        let confederation = brasfoot_model::countries::confederation(c.country);
        let league_club = c.league_club;
        // the original reads nothing beyond its table (the friendly tournament, kind 15)
        let mut points = POINTS.get(place as usize).and_then(|row| row.get(kind as usize)).copied().unwrap_or(0);
        if points > 1000 && !league_club && confederation > 1 {
            points = java_round(points as f64 * 0.6) as i32;
        }
        if kind == 1 && detail > 1 {
            points = 50;
        }
        self.world.club_mut(club).reputation_points += points;
        if let Some(coach) = self.world.club(club).coach {
            self.coach_reputation(coach, place, kind, detail, league_club, confederation);
        }
    }

    /// `al.a(int, int, int, boolean, int)`
    fn coach_reputation(&mut self, coach: CoachId, place: i32, kind: i32, detail: i32, league_club: bool, confederation: i32) {
        const POINTS: [[i32; 15]; 2] = [
            [0, 600, 500, 30, 6000, 10000, 2000, 0, 500, 0, 50, 100, 3000, 3000, 20000],
            [0, 150, 150, 5, 1000, 1000, 500, 2000, 0, 0, 0, 0, 300, 500, 0],
        ];
        // the original reads nothing beyond its table (the friendly tournament, kind 15)
        let mut points = POINTS.get(place as usize).and_then(|row| row.get(kind as usize)).copied().unwrap_or(0);
        if points > 1000 && !league_club && confederation > 1 {
            points = java_round(points as f64 * 0.6) as i32;
        }
        if kind == 1 && detail > 1 {
            points = 50;
        }
        if kind == 7 {
            points = if detail == 7 && place == 0 { 23000 } else { 6000 };
        }
        self.world.coach_mut(coach).reputation_points += points;
    }

    /// `al.b(at, int, int)`: the coach's ranking points and, for a title, his trophy.
    fn coach_place_points(&mut self, coach: CoachId, c: CompetitionId, position: i32) {
        let kind = self.world.competition(c).kind;
        let division = self.competition_division(c);
        const POINTS: [[i32; 11]; 16] = [
            [0; 11],
            [0, 25, 10, 8, 7, 6, 5, 4, 3, 2, 1],
            [0, 20, 7, 3, 3, 0, 0, 0, 0, 0, 0],
            [0, 15, 3, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 30, 15, 7, 7, 0, 0, 0, 0, 0, 0],
            [0, 25, 5, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 20, 10, 5, 5, 0, 0, 0, 0, 0, 0],
            [0, 35, 20, 10, 5, 0, 0, 0, 0, 0, 0],
            [0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0; 11],
            [0, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 18, 8, 4, 4, 0, 0, 0, 0, 0, 0],
            [0, 20, 10, 0, 0, 0, 0, 0, 0, 0, 0],
            [0, 25, 15, 7, 3, 0, 0, 0, 0, 0, 0],
            [0; 11],
        ];
        let mut table = POINTS[kind as usize];
        if kind == 1 && division > 1 {
            table = if division == 2 { [0, 5, 3, 2, 1, 0, 0, 0, 0, 0, 0] } else { [0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0] };
        }
        if kind == 3 && division > 1 {
            table = [0; 11];
        }
        if kind == 7 && self.world.competition(c).subkind != 7 {
            table = [0, 25, 15, 5, 3, 0, 0, 0, 0, 0, 0];
        }
        if kind <= 14 && (0..11).contains(&position) {
            self.world.coach_mut(coach).points += table[position as usize];
        }
        if position == 1 {
            // al.D(ah): a title in the season with the club (or the national team)
            let club = if national_teams_kind(kind) { self.world.coach(coach).national_team } else { self.coach_club(coach) };
            self.coach_title(coach, club);
            self.coach_trophy(coach, c);
        }
    }

    /// `al.q(at)`: the coach's trophy, and the board and fans' joy.
    fn coach_trophy(&mut self, coach: CoachId, c: CompetitionId) {
        let kind = self.world.competition(c).kind;
        let club = if kind == 7 { self.world.coach(coach).national_team } else { self.coach_club(coach) };
        let mut trophy = Trophy {
            season: self.world.season(),
            club_id: club.map(|x| self.world.club(x).id).unwrap_or(-1),
            competition_kind: kind,
            competition: Some(c),
            ..Default::default()
        };
        if kind == 1 || kind == 3 {
            let division = self.competition_division(c);
            trophy.competition_subkind = division;
            if division == 1 {
                self.fans_and_board(coach, 20, 15);
                if kind == 1 {
                    self.fans_and_board(coach, 20, 15);
                }
            } else {
                self.fans_and_board(coach, 20, 10);
            }
        } else if kind == 11 {
            if let Some(x) = self.coach_club(coach) {
                trophy.competition_subkind = self.world.club(x).country;
            }
            self.fans_and_board(coach, 10, 0);
        } else {
            trophy.competition_subkind = self.world.competition(c).subkind;
            if matches!(kind, 2 | 4 | 5 | 6 | 12) {
                self.fans_and_board(coach, 30, 30);
            }
        }
        self.world.coach_mut(coach).trophies.push(trophy);
        if self.world.coach(coach).human
            && let Some(club) = self.coach_club(coach)
        {
            // the board's and the fans' congratulations; the original names the continental cup
            // of the club's confederation in the text, the port keeps the confederation
            match kind {
                4 => {
                    let confederation = brasfoot_model::countries::confederation(self.world.club(club).country);
                    brasfoot_engine::messages::send_continental_title(self.world, coach, confederation);
                }
                1 | 2 | 6 => brasfoot_engine::messages::send(self.world, coach, 24, 77, String::new(), String::new()),
                5 => brasfoot_engine::messages::send(self.world, coach, 25, 78, String::new(), String::new()),
                _ => {}
            }
        }
    }

    /// `al.ck(int)` then `al.cj(int)`.
    fn fans_and_board(&mut self, coach: CoachId, fans: i32, board: i32) {
        let c = self.world.coach_mut(coach);
        c.fan_support = (c.fan_support + fans).clamp(0, 100);
        if board != 0 {
            c.board_confidence = (c.board_confidence + board).clamp(0, 100);
        }
    }

    /// `ah.q(at)`: the club's trophy.
    fn club_trophy(&mut self, club: ClubId, c: CompetitionId) {
        let kind = self.world.competition(c).kind;
        let subkind = match kind {
            1 | 3 => self.competition_division(c),
            11 => self.world.club(club).country,
            _ => self.world.competition(c).subkind,
        };
        let trophy = Trophy {
            season: self.world.season(),
            club_id: self.world.club(club).id,
            competition_kind: kind,
            competition: Some(c),
            competition_subkind: subkind,
        };
        self.world.club_mut(club).trophies.push(trophy);
    }
}
