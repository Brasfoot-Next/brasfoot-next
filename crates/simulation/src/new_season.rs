// Port of the new season, f.aa(), and of what it calls: the new calendar (f.ah, f.r, f.aS,
// a.g, a.q), the clubs' and coaches' year (f.ag, ah.kk, ah.kp, ah.kA, ah.kz, f.iZ, al.kk), the
// national super cups (f.Z, f.I.Ab), promotion and relegation with the new leagues and cups
// (f.am, Z.ee, f.s.aN, best.t.a, f.f.a), the players' year (f.af) and the coach market's
// lists (ay.zi, ay.zj).

use brasfoot_model::clock::Clock;
use brasfoot_model::ids::{ClubId, CompetitionId, CountryId, StageId};
use brasfoot_model::saved::{Competition, CompetitionKind, Day, Stage, StageKind};

use crate::Sim;
use crate::tables::DAY_KINDS;

/// `best.a.b(int, int, int)`: the names of the calendar's day tasks.
const TASKS: [&str; 12] = ["", "cw", "ds", "aj", "cD", "dJ", "cS", "cO", "cSempregado", "cPromocao", "criaMataMataAscenso", "criaplayOffRebaixamento"];

impl Sim<'_> {
    /// `f.aa()`
    pub fn new_season(&mut self) {
        for club in self.world.game.human_clubs.clone() {
            // ah.kI(): C.eA() starts the year's books
            let f = &mut self.world.club_mut(club).finances;
            f.income_type5 = 0;
            f.income_transfers = 0;
            f.income_type3 = 0;
            f.income_type6 = 0;
            f.expense_type7 = 0;
            f.expense_salaries = 0;
            f.expense_type4 = 0;
            f.expense_other = 0;
            f.expense_type8 = 0;
            f.expense_transfers = 0;
        }
        if self.world.game.plays_international_clubs {
            self.continental_champions_places();
        }
        if self.world.game.plays_all_national_teams {
            self.finalissima_places();
        }
        self.new_calendar();
        self.clubs_year();
        for coach in self.world.game.coaches.clone() {
            self.coach_reputation_year(coach);
        }
        if self.world.game.plays_international_clubs {
            self.continental_cups_new_season();
        }
        self.runtime.checkpoint("best.f.Y");
        if let Some(brazil) = self.league_country(29) {
            self.brazilian_cup_seeds(brazil);
        }
        self.runtime.checkpoint("best.f.Z");
        self.national_super_cups();
        self.runtime.checkpoint("best.f.am");
        self.new_leagues_and_cups();
        self.runtime.checkpoint("am end");
        if !self.world.game.state_leagues.is_empty() {
            crate::unported!("f.ak(): the state championships");
        }
        if self.world.game.plays_regionals {
            crate::unported!("f.aj(): the regional cups");
        }
        if self.world.game.flag_bc {
            crate::unported!("f.aa(): the Brazilian cup's tasks (bk)");
        }
        self.schedule_promotion_offers();
        self.runtime.checkpoint("best.f.af");
        self.players_year();
        self.runtime.checkpoint("af end");
        if self.world.season() >= 5 {
            // f.W(): the transfer history keeps ten years
            let oldest = self.world.game.start_year - 1;
            self.world.game.transfers.retain(|t| t.year + 10 >= oldest);
        }
        let market = &mut self.world.job_market;
        market.countries.clear();
        market.shuffled_countries.clear();
        market.list_jj.clear();
        market.clubs.clear();
        self.coach_market_clubs();
        self.season_market();
        if self.world.game.plays_all_national_teams {
            self.national_tournaments_new_season();
        }
        if let Some(t) = self.world.game.friendly_tournament {
            // L.xZ(), at.clear()
            self.world.competition_mut(t).main_stage = None;
        }
        self.world.game.season_over = false;
        self.find_next_day();
        // V(): f.ap() plays the first day unless a human club plays it (the screens then ask for
        // the line-up)
        if self.human_waiting().is_none() {
            self.advance();
        }
    }

    /// `f.ah()`: one season more, the old days dropped and the new year's calendar made.
    fn new_calendar(&mut self) {
        self.world.game.season += 1;
        for day in std::mem::take(&mut self.world.game.days) {
            // a.clear() -> at.clear() on each competition: a league's round ratings (ZZ), team
            // of the round (aaa) and season ratings (ZW) go (f.B.clear())
            for &c in &day.competitions {
                if self.world.competition(c).class == CompetitionKind::NationalLeague {
                    self.runtime.round_ratings.remove(&c);
                    let comp = self.world.competition_mut(c);
                    comp.selections_aaa.clear();
                    comp.ratings.clear();
                }
            }
            // a.clear() -> I.clear() on each match
            for m in day.matches {
                let g = self.world.game_match_mut(m);
                g.events.clear();
                g.home_starters.clear();
                g.away_starters.clear();
                g.home_bench.clear();
                g.away_bench.clear();
                g.home_played.clear();
                g.away_played.clear();
                g.home_list_fl.clear();
                g.away_list_fm.clear();
            }
        }
        let year = self.world.game.start_year + (self.world.season() - 1);
        self.year_calendar(year);
        self.assign_day_kinds();
    }

    /// `f.r(int)` and `f.aS()`: the days of the year, today the first Sunday of January, with
    /// the days of the wages, the interest and the evolution.
    pub(crate) fn year_calendar(&mut self, year: i32) {
        let mut sundays = 0;
        for (i, (y, m, d, weekday)) in Clock::days_of_year(year).enumerate() {
            let date = self.runtime.clock.date_now(y, m, d);
            self.world.game.days.push(Day { date, ..Default::default() });
            if m == 0 && weekday == 1 {
                sundays += 1;
                if sundays == 1 {
                    self.world.game.day = i as i32;
                }
            }
        }
        // f.aS(): wages, interest and evolution
        let seconds: Vec<usize> = (0..self.world.game.days.len()).filter(|&i| self.day_fields(i).day == 2).collect();
        let sundays: Vec<usize> = (0..self.world.game.days.len()).filter(|&i| self.day_fields(i).weekday == 1).collect();
        let wages = if self.world.game.monthly_salary { seconds.clone() } else { sundays.clone() };
        for &d in &wages {
            self.add_task(d, 2, false);
        }
        for &d in &seconds {
            self.add_task(d, 5, false);
        }
        for &d in &sundays {
            self.add_task(d, 3, false);
        }
    }

    /// `a.g()`: the kinds of the match days, weekday by weekday.
    pub(crate) fn assign_day_kinds(&mut self) {
        for &(weekday, kind) in DAY_KINDS {
            self.assign_day_kind(weekday, kind);
        }
    }

    fn day_fields(&self, day: usize) -> brasfoot_model::clock::DateFields {
        self.runtime.clock.fields(self.world.game.days[day].date)
    }

    /// `best.a.b(int, int, int)`
    pub(crate) fn add_task(&mut self, day: usize, task: usize, after: bool) {
        let d = &mut self.world.game.days[day];
        let list = if after { &mut d.tasks_after } else { &mut d.tasks_before };
        list.push(TASKS[task].to_owned());
    }

    /// `best.a.b(int, int)`: the first day without a kind on that weekday, after the last day
    /// with one (and not before today), gets the kind.
    fn assign_day_kind(&mut self, weekday: i32, kind: i32) {
        let days = &self.world.game.days;
        let mut from = 0;
        for i in (1..days.len()).rev() {
            if days[i].kind > 0 {
                from = i;
                break;
            }
        }
        let today = self.world.game.day.max(0) as usize;
        if from < today {
            from = today;
        }
        let mut found = 0;
        for i in from..days.len() {
            if days[i].kind == 0 && self.day_fields(i).weekday == weekday {
                found = i;
                break;
            }
        }
        if found > 0 {
            self.world.game.days[found].kind = kind;
        }
    }

    /// `best.a.a(int, int)`: the n-th day of a kind.
    fn nth_day_of_kind(&self, kind: i32, n: i32) -> Option<usize> {
        let mut count = 1;
        for (i, d) in self.world.game.days.iter().enumerate() {
            if d.kind == kind && count == n {
                return Some(i);
            }
            if d.kind == kind {
                count += 1;
            }
        }
        None
    }

    /// `best.a.q()`: with every national team, the days the national coaches call up ("cS",
    /// "cO", and "cSempregado" for the unemployed); the day the coaches' promotions are offered.
    pub(crate) fn schedule_promotion_offers(&mut self) {
        if self.world.game.plays_all_national_teams {
            let mut tasks: Vec<(Option<usize>, usize)> = Vec::new();
            if self.world.game.plays_state {
                tasks.push((self.nth_day_of_kind(3, 4), 6));
            }
            // a.e(7): the first day of kind 7
            let first_of_7 = self.world.game.days.iter().position(|d| d.kind == 7);
            tasks.push((self.nth_day_of_kind(1, 1), 8));
            tasks.push((first_of_7, 6));
            tasks.push((first_of_7, 7));
            tasks.push((self.nth_day_of_kind(1, 12), 6));
            tasks.push((self.nth_day_of_kind(1, 18), 6));
            for (day, task) in tasks {
                if let Some(d) = day
                    && d > 0
                {
                    self.add_task(d, task, true);
                }
            }
        }
        if let Some(d) = self.nth_day_of_kind(1, 20)
            && d > 0
        {
            self.add_task(d, 9, true);
        }
    }

    /// `f.ag()`: the clubs' year.
    fn clubs_year(&mut self) {
        for club in self.world.game.clubs.clone() {
            self.world.club_mut(club).stage_records.clear();
            self.club_reputation_year(club);
            if !self.world.club(club).human {
                self.list_surplus_players(club);
                self.choose_penalty_taker(club);
                self.choose_captain(club);
            }
            if let Some(coach) = self.world.club(club).coach {
                let c = self.world.coach_mut(coach);
                c.board_confidence = (c.board_confidence + 50).clamp(0, 100);
                c.fan_support = (c.fan_support + 50).clamp(0, 100);
            }
        }
        // ah.kG(): the human clubs' year: three wage bills more outside Brazil when the state
        // championships are on, and the division's yearly income (aq.sD, kind 6)
        for club in self.world.game.human_clubs.clone() {
            let c = self.world.club(club);
            if c.country != 29 && self.world.game.plays_state {
                let wages = self.world.club_wages(club);
                let c = self.world.club_mut(club);
                c.cash = (c.cash as f64 + 3.2 * wages as f64) as i64;
            }
            let division = self.world.club(club).division;
            if (0..=4).contains(&division) {
                self.world.club_income(club, crate::tables::BASE_INCOME[division as usize][0], 6);
            }
        }
    }

    /// `ah.kk()`: reputation fades without results; points raise it.
    fn club_reputation_year(&mut self, club: ClubId) {
        let c = self.world.club_mut(club);
        match c.reputation {
            5 => {
                c.reputation_points -= 6000;
                if c.reputation_points < -90000 {
                    c.reputation = 4;
                    c.reputation_points = 0;
                }
            }
            4 => {
                c.reputation_points -= 600;
                if c.reputation_points < -9000 {
                    c.reputation = 3;
                    c.reputation_points = 0;
                }
            }
            3 if c.league_club => {
                c.reputation_points -= 50;
                if c.reputation_points < -1000 {
                    c.reputation = 2;
                    c.reputation_points = 0;
                }
            }
            2 if c.league_club => {
                c.reputation_points -= 5;
                // the original keeps 2 here
                if c.reputation_points < -1000 {
                    c.reputation = 2;
                    c.reputation_points = 0;
                }
            }
            _ => {}
        }
        let earned = if c.reputation_points > 100_000 {
            c.reputation_points = 100_001;
            5
        } else if c.reputation_points > 10_000 {
            4
        } else if c.reputation_points > 1000 {
            3
        } else if c.reputation_points > 100 {
            2
        } else if c.reputation_points > 10 {
            1
        } else {
            0
        };
        if earned > c.reputation {
            c.reputation = earned;
        }
    }

    /// `al.kk()`
    fn coach_reputation_year(&mut self, coach: brasfoot_model::ids::CoachId) {
        let league_club = self.coach_club(coach).is_some_and(|c| self.world.club(c).league_club);
        let c = self.world.coach_mut(coach);
        match c.reputation {
            5 => {
                c.reputation_points -= 6000;
                if c.reputation_points < -20000 {
                    c.reputation = 4;
                    c.reputation_points = 0;
                }
            }
            4 => {
                c.reputation_points -= 600;
                if c.reputation_points < -3000 {
                    c.reputation = 3;
                    c.reputation_points = 0;
                }
            }
            3 if league_club => {
                c.reputation_points -= 50;
                if c.reputation_points < -500 {
                    c.reputation = 2;
                    c.reputation_points = 0;
                }
            }
            2 if league_club => {
                c.reputation_points -= 5;
                if c.reputation_points < -50 {
                    c.reputation = 1;
                    c.reputation_points = 0;
                }
            }
            _ => {}
        }
        let earned = if c.reputation_points > 22000 {
            5
        } else if c.reputation_points > 10000 {
            4
        } else if c.reputation_points > 1000 {
            3
        } else if c.reputation_points > 100 {
            2
        } else if c.reputation_points > 10 {
            1
        } else {
            0
        };
        if earned > c.reputation {
            c.reputation = earned;
        }
    }

    /// `ah.kp()`: an AI club with too many players of a position lists one for sale.
    pub(crate) fn list_surplus_players(&mut self, club: ClubId) {
        const LIMIT: [i32; 5] = [2, 3, 3, 5, 4];
        // ah.J(false): players not on loan, by position
        let mut counts = [0i32; 6];
        for &p in &self.world.club(club).squad {
            let player = self.world.player(p);
            if !player.on_loan {
                counts[player.position as usize] += 1;
                if player.position == 3 && player.style == 0 {
                    counts[5] += 1;
                }
            }
        }
        // ah.ks()
        for p in self.world.club(club).squad.clone() {
            let player = self.world.player_mut(p);
            player.listed_for_sale = false;
            player.listed_for_loan = false;
        }
        let mut squad = std::mem::take(&mut self.world.club_mut(club).squad);
        self.runtime.rng.shuffle(&mut squad);
        self.world.club_mut(club).squad = squad;
        for position in 0..=4 {
            if counts[position] > LIMIT[position] && self.runtime.rng.fresh_int(100) > 50 {
                let pick = self.world.club(club).squad.iter().copied().find(|&p| {
                    let x = self.world.player(p);
                    x.position == position as i32 && x.status == 0 && !x.star && !x.world_class && !x.on_loan
                });
                if let Some(p) = pick {
                    let player = self.world.player_mut(p);
                    player.price = player.market_value;
                    player.listed_for_sale = true;
                    if self.runtime.rng.fresh_int(100) > 70 && self.world.player(p).strength < 42 {
                        self.world.player_mut(p).listed_for_loan = true;
                    }
                }
            }
        }
    }

    /// `ah.kA()`: the squad by strength, and the penalty taker.
    pub(crate) fn choose_penalty_taker(&mut self, club: ClubId) {
        let mut squad = std::mem::take(&mut self.world.club_mut(club).squad);
        if squad.is_empty() {
            return;
        }
        brasfoot_engine::lineup::sort_by_strength(self.world, &mut squad);
        let w = &*self.world;
        let taker = squad
            .iter()
            .copied()
            .find(|&p| w.player(p).status == 1 && w.player(p).trait1 == 9)
            .or_else(|| squad.iter().copied().find(|&p| w.player(p).status == 1 && w.player(p).position != 0))
            .or_else(|| squad.iter().copied().find(|&p| w.player(p).position != 0));
        let c = self.world.club_mut(club);
        c.squad = squad;
        if let Some(t) = taker {
            c.penalty_taker = Some(t);
        }
    }

    /// `ah.kz()`: the squad by strength then age (`ai`), the first one captain.
    pub(crate) fn choose_captain(&mut self, club: ClubId) {
        let mut squad = std::mem::take(&mut self.world.club_mut(club).squad);
        if squad.is_empty() {
            return;
        }
        squad.sort_by(|&a, &b| {
            let (x, y) = (self.world.player(a), self.world.player(b));
            if x.strength == y.strength { y.age.cmp(&x.age) } else { y.strength.cmp(&x.strength) }
        });
        let c = self.world.club_mut(club);
        c.captain = Some(squad[0]);
        c.squad = squad;
    }

    /// `f.Y()` and `Z.y(true)`: the twelve clubs the Brazilian cup seeds (`f.f.afU`): the first
    /// two divisions by last season's places (`f.s.yV()`), the clubs outside the leagues when
    /// there are fewer than twelve, then the second division's champion, last season's cup
    /// winner and the continental champions moved to the front.
    fn brazilian_cup_seeds(&mut self, brazil: brasfoot_model::ids::CountryId) {
        let divisions = self.world.country(brazil).divisions.clone();
        let mut seeds: Vec<ClubId> = Vec::new();
        if let Some(&first) = divisions.first() {
            seeds.extend(self.league_order(first));
            let mut promoted = None;
            if let Some(&second) = divisions.get(1) {
                let order = self.league_order(second);
                promoted = order.first().copied();
                seeds.extend(order);
            }
            if seeds.len() < 12 {
                seeds.extend(self.world.country(brazil).clubs.clone());
            }
            let to_front = |seeds: &mut Vec<ClubId>, club: ClubId| {
                if let Some(i) = seeds.iter().position(|&c| c == club) {
                    seeds.remove(i);
                }
                seeds.insert(0, club);
            };
            if let Some(p) = promoted {
                to_front(&mut seeds, p);
            }
            let last = self.world.season() - 1;
            if let Some(cup) = self.world.country(brazil).cup
                && let Some(winner) = self.champion_of(cup, last)
            {
                to_front(&mut seeds, winner);
            }
            // the Sudamericana's title holder, then the Libertadores', when Brazilian
            let id = self.world.country(brazil).country;
            let sudamericana = self.world.game.copa_sudamericana.and_then(|c| self.world.competition(c).title_holder);
            let libertadores = self.world.game.libertadores.and_then(|c| self.world.competition(c).club_yt);
            for holder in [sudamericana, libertadores].into_iter().flatten() {
                if self.world.club(holder).country == id {
                    to_front(&mut seeds, holder);
                }
            }
        }
        let cup = self.world.country(brazil).cup.expect("the original fails without a Brazilian cup");
        let c = self.world.competition_mut(cup);
        c.cup_clubs.clear();
        if seeds.len() >= 12 {
            for &club in &seeds[..12] {
                if !c.cup_clubs.contains(&club) {
                    c.cup_clubs.push(club);
                }
            }
        }
    }

    /// `f.s.yV()` of a league's table: the clubs of its knockout from the last round back, then
    /// the table's.
    pub(crate) fn league_order(&self, league: CompetitionId) -> Vec<ClubId> {
        let stage = self.world.competition(league).league.expect("a league has its table");
        let s = self.world.stage(stage);
        if s.format_code == 1020 {
            crate::unported!("f.s.yV(): a table of format 1020");
        }
        let mut order = Vec::new();
        if let Some(ko) = s.knockout {
            for r in self.world.stage(ko).rounds.iter().rev() {
                for &w in &r.winners {
                    if !order.contains(&w) {
                        order.push(w);
                    }
                }
            }
        }
        for &c in &s.clubs {
            if !order.contains(&c) {
                order.push(c);
            }
        }
        order
    }

    /// `at.cv(int)`: a competition's champion in a season.
    fn champion_of(&self, c: CompetitionId, season: i32) -> Option<ClubId> {
        let t = self.world.competition(c).titles.iter().find(|t| t.season == season)?;
        if t.champion >= 0 { self.world.club_by_game_id(t.champion) } else { None }
    }

    /// `f.Z()`: each league country's super cup between the league and cup winners.
    fn national_super_cups(&mut self) {
        if !self.world.game.national_super_cups.is_empty() {
            for c in self.world.game.national_super_cups.clone() {
                // f.I.Ab(), at.mw()
                let id = self.world.competition(c).country;
                if let Some(country) = self.league_country(id) {
                    self.super_cup_draw(c, country);
                }
                self.world.competition_mut(c).scorers.clear();
            }
            return;
        }
        for country in self.world.game.league_countries.clone() {
            let id = self.world.country(country).country;
            let cup = Competition {
                class: CompetitionKind::NationalSuperCup,
                kind: 11,
                subkind: id,
                country: id,
                ..Default::default()
            };
            let c = CompetitionId(self.world.competitions.len() as u32);
            self.world.competitions.push(cup);
            self.world.game.national_super_cups.push(c);
            self.super_cup_draw(c, country);
        }
    }

    /// `f.I.Ab()` and `f.I.b(ah, ah)`: last season's league champion against the cup winner
    /// (or, if the same club, the cup's runner-up).
    fn super_cup_draw(&mut self, c: CompetitionId, country: CountryId) {
        let divisions = self.world.country(country).divisions.clone();
        if divisions.is_empty() {
            return;
        }
        let last = self.world.season() - 1;
        let league_champion = self.champion_of(divisions[0], last);
        let mut other = None;
        if let Some(cup) = self.world.country(country).cup
            && let Some(t) = self.world.competition(cup).titles.iter().find(|t| t.season == last)
        {
            let (winner, runner_up) = (self.world.club_by_game_id(t.champion), self.world.club_by_game_id(t.runner_up));
            other = if league_champion != winner { winner } else { runner_up };
        }
        let comp = self.world.competition_mut(c);
        comp.participants.clear();
        comp.final_stage = None;
        if let (Some(a), Some(b)) = (league_champion, other) {
            self.world.competition_mut(c).participants = vec![a, b];
            let stage = self.new_knockout_stage(c, None, 2, 11, 0, vec![false; 7], -1);
            self.world.competition_mut(c).final_stage = Some(stage);
            self.new_knockout_round(stage, &[a, b], 0, false, 0, 11, false);
        }
    }

    /// `new f.z(s, int, int, int, boolean[], at, int)`: a knockout stage.
    pub fn new_knockout_stage(
        &mut self,
        competition: CompetitionId,
        group_stage: Option<StageId>,
        clubs: i32,
        kind: i32,
        fixed_rounds: i32,
        two_legged: Vec<bool>,
        phase_code: i32,
    ) -> StageId {
        let mut stage = Stage {
            class: StageKind::Knockout,
            competition: Some(competition),
            group_stage,
            competition_kind: kind,
            phase_code,
            two_legged_knockout_rounds: two_legged,
            ..Default::default()
        };
        if fixed_rounds > 0 {
            stage.last_round = fixed_rounds - 1;
            stage.rounds_computed = false;
        } else {
            let rounds = if phase_code == 2029 {
                7
            } else {
                match clubs {
                    128 => 7,
                    64 => 6,
                    32 => 5,
                    16 => 4,
                    8 => 3,
                    4 => 2,
                    2 => 1,
                    _ => 1,
                }
            };
            stage.last_round = rounds - 1;
        }
        stage.round = 0;
        stage.leg = 1;
        let id = StageId(self.world.stages.len() as u32);
        self.world.stages.push(stage);
        id
    }

    /// `f.am()`: promotion and relegation, the leagues' new fixtures and the national cups.
    fn new_leagues_and_cups(&mut self) {
        for country in self.world.game.league_countries.clone() {
            self.promotion_and_relegation(country);
            if let Some(cup) = self.world.country(country).cup {
                self.world.competition_mut(cup).scorers.clear();
            }
        }
        for country in self.world.game.league_countries.clone() {
            for league in self.world.country(country).divisions.clone() {
                let stage = self.world.competition(league).league.expect("a league has its table");
                self.reset_league_stage(stage);
                self.world.competition_mut(league).season_finished = false;
            }
        }
        for country in self.world.game.league_countries.clone() {
            if self.world.country(country).country == 29 && (self.world.game.flag_bc || self.world.game.plays_regionals) {
                crate::unported!("Z.a(null): the Brazilian cup after the regionals");
            }
            let mut clubs: Vec<ClubId> = Vec::new();
            for league in self.world.country(country).divisions.clone() {
                let stage = self.world.competition(league).league.expect("a league has its table");
                clubs.extend(self.world.stage(stage).clubs.iter().copied());
            }
            let mut others = self.world.country(country).clubs_hv.clone();
            others.sort_by(|&a, &b| crate::sorting::by_level(self.world, a, b));
            for c in others {
                if !clubs.contains(&c) {
                    clubs.push(c);
                }
            }
            self.national_cup_draw(country, clubs);
        }
    }

    /// `Z.ee()`: the clubs go up and down between the divisions.
    fn promotion_and_relegation(&mut self, country: CountryId) {
        let divisions = self.world.country(country).divisions.clone();
        let tables: Vec<StageId> = divisions.iter().map(|&d| self.world.competition(d).league.expect("a league has its table")).collect();
        let relegated: Vec<i32> = divisions.iter().map(|&d| self.world.competition(d).relegated).collect();
        // f.s.ah(int, int): who goes up and who goes down, from each table
        let mut up: Vec<Vec<ClubId>> = Vec::new();
        let mut down: Vec<Vec<ClubId>> = Vec::new();
        for (i, &t) in tables.iter().enumerate() {
            let goes_up = if i == 0 { 0 } else { relegated[i - 1] };
            up.push(self.promoted_from(t, goes_up, true));
            down.push(self.relegated_from(t, relegated[i]));
        }
        for i in 0..tables.len() {
            if i + 1 < tables.len() {
                let (going_down, going_up) = (down[i].clone(), up[i + 1].clone());
                if going_down.len() == going_up.len() {
                    for (k, &d) in going_down.iter().enumerate() {
                        if self.in_relegation_playoff(divisions[i], d) {
                            continue;
                        }
                        let u = going_up[k];
                        self.set_relegation_mark(d, divisions[i], 0);
                        self.set_relegation_mark(u, divisions[i + 1], 1);
                        self.world.stage_mut(tables[i]).clubs.retain(|&x| x != d);
                        self.world.stage_mut(tables[i + 1]).clubs.push(d);
                        self.world.stage_mut(tables[i + 1]).clubs.retain(|&x| x != u);
                        self.world.stage_mut(tables[i]).clubs.push(u);
                    }
                }
            } else {
                let mut count = relegated[i];
                let loose = self.world.country(country).clubs.clone();
                if (loose.len() as i32) < count {
                    count = loose.len() as i32;
                }
                if count > 0 {
                    let going_down: Vec<ClubId> = down[i].iter().copied().take(count as usize).collect();
                    let mut going_up: Vec<ClubId> = loose.iter().copied().take(count as usize).collect();
                    if going_down.len() == going_up.len() {
                        going_up.reverse();
                        for (k, &d) in going_down.iter().enumerate() {
                            if self.in_relegation_playoff(divisions[i], d) {
                                continue;
                            }
                            self.set_relegation_mark(d, divisions[i], 0);
                            self.world.stage_mut(tables[i]).clubs.retain(|&x| x != d);
                            self.world.country_mut(country).clubs.push(d);
                            self.world.club_mut(d).division = 0;
                            let u = going_up[k];
                            let loose = &mut self.world.country_mut(country).clubs;
                            if let Some(pos) = loose.iter().position(|&x| x == u) {
                                loose.remove(pos);
                            }
                            self.world.stage_mut(tables[i]).clubs.push(u);
                        }
                    }
                }
            }
            // f.s.yI()
            let division = self.world.stage(tables[i]).division;
            for c in self.world.stage(tables[i]).clubs.clone() {
                self.world.club_mut(c).division = division;
            }
        }
        for &d in &divisions {
            // f.B.BJ()
            let c = self.world.competition_mut(d);
            c.promotion_knockout = None;
            c.relegation_playoff = None;
        }
    }

    /// `f.B.ac(ah)`: the club won its relegation playoff.
    fn in_relegation_playoff(&self, league: CompetitionId, club: ClubId) -> bool {
        let Some(playoff) = self.world.competition(league).relegation_playoff else { return false };
        self.world.stage(playoff).rounds.first().is_some_and(|r| r.winners.contains(&club))
    }

    /// `ah.b(at, int)`: the season's record marks the club relegated (0) or promoted (1).
    fn set_relegation_mark(&mut self, club: ClubId, league: CompetitionId, mark: i32) {
        // ah.n(at): last season's record of the competition, if any
        let season = self.world.season() - 1;
        let kind = self.world.competition(league).kind;
        let subkind = self.world.competition(league).subkind;
        let records = &mut self.world.club_mut(club).competition_records;
        if let Some(r) = records.iter_mut().find(|r| r.season == season && r.competition_kind == kind && r.competition_subkind == subkind) {
            r.league_move = mark;
        }
    }

    /// `f.s.l(int, boolean)`: the first clubs of the table; with a knockout after it, its
    /// winners from the last round back first; with `knockout` and a promotion knockout of the
    /// league (`f.B.BR()`), its winners (of its only round, or of its second) take the last
    /// places, and the list comes reversed.
    pub(crate) fn promoted_from(&self, stage: StageId, count: i32, knockout: bool) -> Vec<ClubId> {
        let s = self.world.stage(stage);
        if s.format_code == 1020 {
            crate::unported!("f.s.l(int, boolean) of format 1020");
        }
        let count = count.max(0) as usize;
        let mut list: Vec<ClubId> = Vec::new();
        if s.knockout_teams == 0 {
            if !s.clubs.is_empty() {
                list = s.clubs[..count].to_vec();
            }
        } else if let Some(ko) = s.knockout
            && count > 0
        {
            for r in self.world.stage(ko).rounds.iter().rev() {
                for &w in &r.winners {
                    if count > list.len() && !list.contains(&w) {
                        list.push(w);
                    }
                }
            }
            for &c in s.clubs.iter().take(count) {
                if count > list.len() && !list.contains(&c) {
                    list.push(c);
                }
            }
        } else {
            for &c in s.clubs.iter().take(count) {
                if count > list.len() && !list.contains(&c) {
                    list.push(c);
                }
            }
        }
        let promotion = s.competition.and_then(|c| self.world.competition(c).promotion_knockout);
        if knockout
            && s.country_ref.is_some()
            && let Some(ko) = promotion
        {
            let rounds = &self.world.stage(ko).rounds;
            let mut out: Vec<ClubId> = match rounds.len() {
                1 => rounds[0].winners.clone(),
                2 => rounds[1].winners.clone(),
                _ => Vec::new(),
            };
            let rest = list.len().saturating_sub(out.len());
            out.extend_from_slice(&list[..rest]);
            out.reverse();
            list = out;
        }
        list
    }

    /// `f.s.fG(int)`: the last clubs, the bottom one first; a league played in groups
    /// (`f.B.zc()`) sends down the last of each group (the last two when four go down); with
    /// twelve groups or more and a knockout, the clubs of the knockout are spared.
    pub(crate) fn relegated_from(&self, stage: StageId, count: i32) -> Vec<ClubId> {
        let s = self.world.stage(stage);
        let n = s.clubs.len();
        let bottom = |count: usize| -> Vec<ClubId> { (n.saturating_sub(count)..n).rev().map(|i| s.clubs[i]).collect() };
        let count = count.max(0) as usize;
        if s.knockout_teams == 0 {
            return if n > 0 { bottom(count) } else { Vec::new() };
        }
        let in_groups = s.competition.is_some_and(|c| self.world.competition(c).flag_agz);
        if in_groups && !s.groups.is_empty() {
            let per = if count == 4 { 2 } else { 1 };
            let mut out = Vec::new();
            for g in &s.groups {
                let len = g.clubs.len();
                for i in (len.saturating_sub(per)..len).rev() {
                    out.push(g.clubs[i]);
                }
            }
            return out;
        }
        if s.group_count >= 12
            && let Some(ko) = s.knockout
        {
            let in_knockout = |c: ClubId| {
                self.world.stage(ko).rounds.iter().any(|r| r.first_leg.iter().any(|&m| {
                    let g = self.world.game_match(m);
                    g.home == Some(c) || g.away == Some(c)
                }))
            };
            return bottom(count).into_iter().filter(|&c| !in_knockout(c)).collect();
        }
        bottom(count)
    }

    /// `f.s.aN(false)`: the table starts over with new fixtures.
    fn reset_league_stage(&mut self, stage: StageId) {
        let s = self.world.stage(stage);
        if s.format_code == 1020 || s.format_code == 1068 {
            crate::unported!("f.s.aN(boolean) of format 1020 or 1068");
        }
        let competition = s.competition.expect("a stage belongs to a competition");
        let country = s.country_ref;
        let s = self.world.stage_mut(stage);
        s.club_ns = None;
        s.groups.clear();
        s.knockout = None;
        s.numbers.clear();
        s.round = 1;
        self.world.competition_mut(competition).scorers.clear();
        if let Some(country) = country {
            // Z.jx()
            for d in self.world.country(country).divisions.clone() {
                let c = self.world.competition_mut(d);
                c.ratings.clear();
                c.selections_aaa.clear();
            }
        }
        let s = self.world.stage(stage);
        if !s.clubs.is_empty() {
            if s.group_count <= 1 {
                let clubs = s.clubs.clone();
                let both = s.double_round;
                self.league_fixtures(stage, &clubs, both);
            } else {
                // f.s.j(int, false): the clubs dealt one to each group
                self.group_fixtures(stage, false);
            }
        }
    }

    /// `f.s.a(ArrayList, boolean, boolean, ArrayList)`: the rounds of a table.
    pub(crate) fn league_fixtures(&mut self, stage: StageId, clubs: &[ClubId], two_rounds: bool) {
        let s = self.world.stage(stage);
        let kind = s.kind_code();
        let n = clubs.len();
        if let Some(table) = fixed_table(n, two_rounds, s.format_code) {
            // best.t's fixed tables: no draw
            let competition = s.competition.expect("a stage belongs to a competition");
            let days = self.free_days(kind, s.skip_two_dates);
            for (r, pairs) in table.iter().enumerate() {
                for &(home, away) in pairs.iter().take(n / 2) {
                    self.new_match(stage, r as i32, clubs[home], clubs[away], days[r], competition, None);
                }
                let day = days[r] as i32;
                if !self.world.stage(stage).numbers.contains(&day) {
                    self.world.stage_mut(stage).numbers.push(day);
                }
            }
            return;
        }
        if n == 19 {
            crate::unported!("best.t.e(f.s, ArrayList): a table of 19 clubs");
        }
        let mut two_rounds = two_rounds;
        if kind == 1 {
            two_rounds = !matches!(n, 30 | 26 | 28 | 36);
        }
        let competition = s.competition.expect("a stage belongs to a competition");
        if kind == 1 && s.group_count == 0 && self.world.competition(competition).class == CompetitionKind::NationalLeague {
            // f.s.aS(false)
            self.world.competition_mut(competition).flag_agz = false;
        }
        let s = self.world.stage(stage);
        let turns = if kind == 1 {
            if n == 20 && s.format_code == 1020 {
                1
            } else if n == 8 && s.group_count == 0 {
                4
            } else if n == 10 && s.group_count == 0 {
                match s.round_count {
                    2 => 2,
                    3 => 3,
                    _ => 4,
                }
            } else if (n == 12 || n == 14) && s.group_count == 0 {
                if s.round_count == 2 { 2 } else { 3 }
            } else if two_rounds {
                2
            } else {
                1
            }
        } else if two_rounds {
            2
        } else {
            1
        };
        let skip_two = s.skip_two_dates;
        let rounds = self.round_robin(clubs, turns);
        let days = self.free_days(kind, skip_two);
        for (r, matches) in rounds.iter().enumerate() {
            for &(home, away) in matches.iter().take(n / 2) {
                if let (Some(h), Some(a)) = (home, away) {
                    self.new_match(stage, r as i32, h, a, days[r], competition, None);
                }
            }
            let day = days[r] as i32;
            if !self.world.stage(stage).numbers.contains(&day) {
                self.world.stage_mut(stage).numbers.push(day);
            }
        }
    }

    /// `best.t.a(f.s, ArrayList, int)`: a shuffled round robin, played `turns` times with home
    /// and away swapped every other time.
    fn round_robin(&mut self, clubs: &[ClubId], turns: i32) -> Vec<Vec<(Option<ClubId>, Option<ClubId>)>> {
        let mut order: Vec<usize> = (0..clubs.len()).collect();
        self.runtime.rng.shuffle(&mut order);
        let mut teams: Vec<Option<ClubId>> = order.iter().map(|&i| Some(clubs[i])).collect();
        let n = teams.len();
        if n % 2 == 1 {
            teams.push(None);
        }
        let rounds = teams.len() - 1;
        let per_round = teams.len() / 2;
        // the modulus and the fixed club use the count before the bye, as in the original
        let mut table: Vec<Vec<(Option<ClubId>, Option<ClubId>)>> = (0..rounds)
            .map(|r| {
                (0..per_round)
                    .map(|i| {
                        let home = teams[(r + i) % (n - 1)];
                        let away = if i == 0 { teams[n - 1] } else { teams[(n - 1 - i + r) % (n - 1)] };
                        (home, away)
                    })
                    .collect()
            })
            .collect();
        let mut interleaved = Vec::with_capacity(rounds);
        let (mut first, mut second) = (0, teams.len() / 2);
        for r in 0..rounds {
            if r % 2 == 0 {
                interleaved.push(table[first].clone());
                first += 1;
            } else {
                interleaved.push(table[second].clone());
                second += 1;
            }
        }
        table = interleaved;
        for (r, round) in table.iter_mut().enumerate() {
            if r % 2 == 1 {
                let (h, a) = round[0];
                round[0] = (a, h);
            }
        }
        let swapped: Vec<Vec<(Option<ClubId>, Option<ClubId>)>> =
            table.iter().map(|round| round.iter().map(|&(h, a)| (a, h)).collect()).collect();
        let mut all = Vec::new();
        for t in 0..turns.max(1) {
            if turns <= 1 {
                all.extend(table.iter().cloned());
                break;
            }
            all.extend(if t % 2 == 0 { table.iter().cloned() } else { swapped.iter().cloned() });
        }
        all
    }

    /// `Z.a(ArrayList, boolean, String)` with `f.f.a(f.f, ArrayList, boolean[])`: the national
    /// cup's draw, the best half against the rest.
    pub(crate) fn national_cup_draw(&mut self, country: CountryId, clubs: Vec<ClubId>) {
        let n = clubs.len();
        let id = self.world.country(country).country;
        if id == 29 && n >= 91 {
            crate::unported!("f.f.b(): the Brazilian cup's new format");
        }
        if n < 8 {
            return;
        }
        let Some(cup) = self.world.country(country).cup else {
            crate::unported!("new f.f(Z): a country's first cup");
        };
        let size = match n {
            128.. => 128,
            64.. => 64,
            32.. => 32,
            16.. => 16,
            _ => 8,
        };
        self.world.competition_mut(cup).team_count = size as i32;
        let mut top: Vec<ClubId> = clubs[..size / 2].to_vec();
        let mut bottom: Vec<ClubId> = clubs[size / 2..size].to_vec();
        self.runtime.rng.shuffle(&mut top);
        self.runtime.rng.shuffle(&mut bottom);
        let pairs: Vec<ClubId> = top.iter().zip(&bottom).flat_map(|(&t, &b)| [b, t]).collect();
        let two_legged = self.world.country(country).two_legged_rounds.clone();
        let stage = self.new_knockout_stage(cup, None, pairs.len() as i32, 2, 0, two_legged, -1);
        self.world.competition_mut(cup).knockout = Some(stage);
        self.new_knockout_round(stage, &pairs, 0, true, 0, 2, false);
    }

    /// `ay.zj()`: three clubs from the bottom half of every league may want a coach.
    pub fn coach_market_clubs(&mut self) {
        self.world.job_market.clubs.clear();
        let mut half = 0;
        for country in self.world.game.league_countries.clone() {
            for league in self.world.country(country).divisions.clone() {
                let stage = self.world.competition(league).league.expect("a league has its table");
                let clubs = self.world.stage(stage).clubs.clone();
                // a table without clubs keeps the previous half, as in the original
                if !clubs.is_empty() {
                    half = clubs.len() / 2;
                }
                let mut ids: Vec<i32> = clubs.iter().skip(half).map(|&c| self.world.club(c).id).collect();
                // ay.M(ArrayList)
                self.runtime.rng.shuffle(&mut ids);
                self.world.job_market.clubs.extend(ids.into_iter().take(3));
            }
        }
        let mut clubs = std::mem::take(&mut self.world.job_market.clubs);
        self.runtime.rng.shuffle(&mut clubs);
        self.world.job_market.clubs = clubs;
    }
}

/// `f.s.a(ArrayList, boolean, boolean, ArrayList)`'s choice of a fixed table of `best.t`.
fn fixed_table(n: usize, two_rounds: bool, format_code: i32) -> Option<Vec<Vec<(usize, usize)>>> {
    use crate::fixture_tables::*;
    fn rows<const K: usize>(t: &[[(usize, usize); K]]) -> Vec<Vec<(usize, usize)>> {
        t.iter().map(|r| r.to_vec()).collect()
    }
    Some(match n {
        // t.v(ArrayList)
        4 if format_code == 7700 => rows(&FOUR_WORLD_CUP),
        // t.a(f.s, ArrayList), t.g(f.s, ArrayList)
        3 if two_rounds => rows(&THREE_TWO_ROUNDS),
        3 => rows(&THREE_ONE_ROUND),
        // t.y(ArrayList): the first five rounds; t.b(f.s, ArrayList): all ten
        5 if !two_rounds => rows(&FIVE[..5]),
        5 => rows(&FIVE),
        // t.h(f.s, ArrayList)
        6 if two_rounds => rows(&SIX_TWO_ROUNDS),
        // t.a(f.s, ArrayList, true): the nine rounds and the nine again with home and away
        // swapped
        9 => {
            let mut t = rows(&NINE);
            let back: Vec<Vec<(usize, usize)>> = t.iter().map(|r| r.iter().map(|&(h, a)| (a, h)).collect()).collect();
            t.extend(back);
            t
        }
        // t.c(f.s, ArrayList), t.d(f.s, ArrayList)
        11 => rows(&ELEVEN),
        25 => rows(&TWENTY_FIVE),
        _ => return None,
    })
}
