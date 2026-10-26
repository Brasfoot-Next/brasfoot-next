// Port of the day loop of best.f: V() -> ap() (a match day) or aa() (a new season), and at()
// with what it calls. Parts still to port stop with `unported!`.

use brasfoot_model::ids::StageId;
use brasfoot_model::saved::StageKind;

use crate::Sim;

impl Sim<'_> {
    /// `f.V()` with the screens off.
    pub fn advance(&mut self) {
        if self.world.game.season_over {
            self.new_season();
            return;
        }
        self.play_match_day();
    }

    /// `f.ay()`: the first human club playing today whose line-up is not confirmed, with its
    /// match; `f.ap()` waits for it instead of playing the day.
    pub fn human_waiting(&self) -> Option<(brasfoot_model::ids::ClubId, brasfoot_model::ids::MatchId)> {
        for &m in &self.today().matches {
            let g = self.world.game_match(m);
            for club in [g.home, g.away].into_iter().flatten() {
                let c = self.world.club(club);
                if c.human && !c.lineup_set {
                    return Some((club, m));
                }
            }
        }
        None
    }

    /// `f.ap()` without a human club waiting for its line-up.
    fn play_match_day(&mut self) {
        self.pick_ai_lineups();
        self.play_today();
        self.after_day();
    }

    fn today(&self) -> &brasfoot_model::saved::Day {
        &self.world.game.days[self.world.game.day as usize]
    }

    /// `f.at()`
    pub(crate) fn after_day(&mut self) {
        self.after_matches();
        self.teams_of_the_round();
        let today = self.world.game.day as usize;
        for day in &mut self.world.game.days[..=today] {
            day.done = true;
        }
        for stage in self.today().stages.clone() {
            match self.world.stage(stage).class {
                StageKind::League => self.league_round(stage),
                StageKind::Knockout => self.knockout_round(stage),
            }
        }
        // f.aB(): at.mx() only clears the competitions' transient lists (tV, tW).
        self.day_tasks(false);
        self.next_day();
    }

    /// `f.as()`
    fn after_matches(&mut self) {
        let kind = self.today().kind;
        for m in self.today_matches() {
            self.after_match(m);
        }
        for p in self.world.game.players.clone() {
            let human = self.world.player(p).club.is_some_and(|c| self.world.club(c).human);
            self.world.player_mut(p).recover_between_matches(human);
        }
        for p in self.world.game.youth_players.clone() {
            self.world.player_mut(p).fitness = 100;
        }
        if kind == 7 || kind == 9 {
            for country in self.world.game.countries.clone() {
                let Some(team) = self.world.country(country).national_team else { continue };
                for p in self.world.club(team).squad.clone() {
                    let player = self.world.player(p);
                    if player.club.is_none() {
                        // fg() is null: the club of a player without one is not human
                        self.world.player_mut(p).recover_between_matches(false);
                    }
                }
            }
        }
        let mut coaches = self.world.game.coaches.clone();
        coaches.sort_by(|&a, &b| crate::sorting::coaches(self.world, a, b));
        self.world.game.coaches = coaches;
    }

    /// `f.an()`: today's national league rounds pick their team of the round (`Z.a(f.s)`).
    fn teams_of_the_round(&mut self) {
        for stage in self.today().stages.clone() {
            let s = self.world.stage(stage);
            if s.class != StageKind::League || s.kind != 1 {
                continue;
            }
            let Some(country) = s.country_ref else { continue };
            for league in self.world.country(country).divisions.clone() {
                if self.world.competition(league).league == Some(stage) {
                    self.team_of_the_round(league);
                }
            }
        }
    }

    /// `f.s.pO()`: the round of a league stage is over.
    fn league_round(&mut self, stage: StageId) {
        for m in self.today_matches() {
            if self.world.game_match(m).stage == Some(stage) {
                let (home, away) = self.sides(m);
                self.stage_match_record(home, m, stage);
                self.stage_match_record(away, m, stage);
            }
        }
        self.runtime.table_stage = Some(stage);
        let mut clubs = self.world.stage(stage).clubs.clone();
        clubs.sort_by(|&a, &b| crate::sorting::standings(self.world, stage, a, b));
        self.world.stage_mut(stage).clubs = clubs;
        for g in 0..self.world.stage(stage).groups.len() {
            let mut clubs = self.world.stage(stage).groups[g].clubs.clone();
            clubs.sort_by(|&a, &b| crate::sorting::standings(self.world, stage, a, b));
            self.world.stage_mut(stage).groups[g].clubs = clubs;
        }
        self.world.stage_mut(stage).round += 1;
        let s = self.world.stage(stage);
        if s.kind == 1 && s.knockout_teams == 0 && s.round == crate::league::mid_season_round(s) {
            crate::unported!("ay.zj() and ag.b(f.s): the mid-season market");
        }
        if s.round > s.value_ze {
            self.end_league_stage(stage);
        }
    }

    /// `f.q(int)`: the calendar's day tasks up to today, before (`false`) or after the move to
    /// the next day.
    fn day_tasks(&mut self, after: bool) {
        for i in 0..=self.world.game.day as usize {
            let day = &mut self.world.game.days[i];
            let tasks = std::mem::take(if after { &mut day.tasks_after } else { &mut day.tasks_before });
            for task in &tasks {
                self.day_task(i, task);
            }
        }
    }

    /// `best.a.f(int)`: one task.
    fn day_task(&mut self, day: usize, task: &str) {
        match task {
            "cw" => self.intercontinental_cup_draw(),
            "ds" => {
                // a.n(): each human club pays its wages (ah.kJ()) if it plays in the task day's
                // month of any year of the calendar (ah.bY(int))
                let month = self.runtime.clock.fields(self.world.game.days[day].date).month;
                for club in self.world.game.human_clubs.clone() {
                    if self.plays_in_month(club, month) {
                        let wages = self.world.club_wages(club);
                        self.world.club_expense(club, wages);
                    }
                }
            }
            "aj" => self.evolve_players(),
            "cD" => crate::unported!("a.m(), a.l(): the Brazilian cup after the regionals"),
            "dJ" => {
                // a.o(): the human clubs pay the interest of their bank loan (C.eQ())
                for club in self.world.game.human_clubs.clone() {
                    let interest = self.world.club(club).finances.bank_interest;
                    if interest > 0 {
                        self.world.club_payment(club, interest, 4);
                    }
                }
            }
            // a.b(false), a.b(true): the national teams' offers to the human coaches
            "cS" => self.national_team_offers_to_humans(false),
            "cSempregado" => self.national_team_offers_to_humans(true),
            "cO" => {
                // a.k(): the call-up screen of a human coach's national team
                if self.world.game.plays_all_national_teams
                    && self.world.game.human_coaches.iter().any(|&c| self.world.coach(c).national_team.is_some())
                {
                    crate::unported!("a.k(): the call-up screen of a human national coach (S.a(ah, true))");
                }
            }
            "cPromocao" => self.offers_to_employed_coaches(),
            // "criaMataMataAscenso" and "criaplayOffRebaixamento" are compared and ignored
            _ => {}
        }
    }

    /// `f.av()`: the players the human clubs listed for sale go to a buyer at their price
    /// (`components.an` without its dialog).
    fn sell_listed_players(&mut self) {
        for club in self.world.game.human_clubs.clone() {
            for p in self.world.club(club).squad.clone() {
                if self.world.player(p).listed_for_sale {
                    crate::unported!("components.an(F, int, true, true): a human club's listed player sold");
                }
            }
        }
    }

    /// `ay.zw()`: the employed human coaches get offers from clubs (`ay.b(al, true)`) in `a.gO`.
    fn offers_to_employed_coaches(&mut self) {
        for coach in self.world.game.coaches.clone() {
            let c = self.world.coach(coach);
            if c.human && c.club_id.is_some() {
                let clubs = self.job_offers(coach, true);
                if !clubs.is_empty() {
                    self.offer_jobs(coach, clubs);
                }
            }
        }
    }

    /// `S.a(ArrayList, al, 0)`: the human sees the offers in `a.gO` and may take one.
    fn offer_jobs(&mut self, coach: brasfoot_model::ids::CoachId, clubs: Vec<brasfoot_model::ids::ClubId>) {
        if let Some(club) = self.ask(|d, w| d.job_offer(w, coach, &clubs)) {
            self.take_job(coach, club);
        }
    }

    /// `ah.bY(int)`: some calendar day of that month has a match of the club.
    fn plays_in_month(&self, club: brasfoot_model::ids::ClubId, month: i32) -> bool {
        self.world.game.days.iter().any(|d| {
            self.runtime.clock.fields(d.date).month == month
                && d.matches.iter().any(|&m| {
                    let g = self.world.game_match(m);
                    g.home == Some(club) || g.away == Some(club)
                })
        })
    }

    /// `f.aw()`: the stadium works whose end is before today add their seats (`B.ex()`).
    fn finish_stadium_works(&mut self) {
        let today = self.world.today_millis();
        let works = std::mem::take(&mut self.world.game.stadium_works);
        for w in works {
            if w.until >= today {
                self.world.game.stadium_works.push(w);
                continue;
            }
            if let Some(s) = w.stadium {
                let stadium = self.world.stadium_mut(s);
                for (i, &seats) in w.seats.iter().enumerate() {
                    if seats > 0 {
                        stadium.capacity[i] += seats;
                    }
                }
            }
        }
    }

    /// `f.au()`: each loan past its end goes back to the owner (a human owner takes him only
    /// with fewer than 35 players, and hears once that he cannot); the human clubs involved
    /// hear of it.
    fn loans_coming_back(&mut self) {
        let today = self.world.today_millis();
        let mut done = Vec::new();
        for i in 0..self.world.game.loans.len() {
            let loan = self.world.game.loans[i].clone();
            if loan.until >= today {
                continue;
            }
            let Some(p) = loan.player else { continue };
            let current = self.world.player(p).club;
            let borrower_coach = current.filter(|&c| self.world.club(c).human).and_then(|c| self.world.club(c).coach);
            let name = self.world.player(p).name.clone();
            // components.t.tN()
            let back = match loan.owner {
                Some(owner) if !self.world.club(owner).human || self.world.club(owner).squad.len() < 35 => {
                    self.return_from_loan(p, owner);
                    true
                }
                _ => false,
            };
            let owner_coach = loan.owner.filter(|&o| self.world.club(o).human).and_then(|o| self.world.club(o).coach);
            if back {
                done.push(i);
                if let Some(coach) = owner_coach {
                    brasfoot_engine::messages::send(self.world, coach, 29, 83, String::new(), name);
                } else if let Some(coach) = borrower_coach {
                    brasfoot_engine::messages::send(self.world, coach, 29, 84, String::new(), name);
                }
            } else if let Some(coach) = owner_coach
                && !loan.flag_po
            {
                brasfoot_engine::messages::send(self.world, coach, 28, 82, String::new(), name);
                self.world.game.loans[i].flag_po = true;
            }
        }
        for i in done.into_iter().rev() {
            self.world.game.loans.remove(i);
        }
    }

    /// `f.ax()`: move to the next match day.
    fn next_day(&mut self) {
        let kind = self.today().matches.first().map(|&m| self.match_kind(m)).unwrap_or(0);
        if kind == 1 || kind == 3 {
            self.coach_changes(kind);
        }
        let next = self.find_next_day();
        if !self.world.game.loans.is_empty() {
            self.loans_coming_back();
        }
        if self.runtime.rng.fresh_int(100) > 50 {
            self.sell_listed_players();
        }
        if !self.world.game.stadium_works.is_empty() {
            self.finish_stadium_works();
        }
        self.day_tasks(true);
        if next == 0 {
            self.world.game.season_over = true;
            self.end_season();
            self.hooks.season_end_screen(self.world, self.runtime);
        }
    }

    /// `I.hy().b()`
    fn match_kind(&self, m: brasfoot_model::ids::MatchId) -> i32 {
        self.world.game_match(m).competition.map(|c| self.world.competition(c).kind).unwrap_or(0)
    }

    /// `f.az()`: the next day not yet played with matches; 0 when the season is over.
    pub fn find_next_day(&mut self) -> usize {
        let from = self.world.game.day.max(0) as usize;
        for i in from..self.world.game.days.len() {
            let d = &self.world.game.days[i];
            if !d.done && d.kind > 0 && !d.matches.is_empty() {
                self.world.game.day = i as i32;
                return i;
            }
        }
        0
    }

    /// `f.n(int)`: after a league day, the boards may sack their coaches.
    fn coach_changes(&mut self, kind: i32) {
        // an unemployed human coach hears from clubs (ay.b(al, false)) in a.gO
        for coach in self.world.game.human_coaches.clone() {
            let c = self.world.coach(coach);
            if c.human && c.club_id.is_none() {
                let clubs = self.job_offers(coach, false);
                if !clubs.is_empty() {
                    self.offer_jobs(coach, clubs);
                }
            }
        }
        if kind == 1 {
            for country in self.world.game.league_countries.clone() {
                for league in self.world.country(country).divisions.clone() {
                    let stage = self.world.competition(league).league.expect("a league has its table");
                    self.sackings(stage);
                }
            }
        } else {
            crate::unported!("w.A(ArrayList): sackings in the state championships");
        }
    }

    /// `f.s.A(ArrayList)`: clubs whose coach lost the board sack him; AI clubs only from the
    /// seventh place down.
    fn sackings(&mut self, stage: StageId) {
        let clubs = self.world.stage(stage).clubs.clone();
        for (i, &club) in clubs.iter().enumerate() {
            let Some(coach) = self.world.club(club).coach else { continue };
            if self.world.coach(coach).board_confidence >= 10 {
                continue;
            }
            if self.world.club(club).human {
                // the reason a.dm shows (aq.sg): 1 the debts, else 2 or 3
                let reason = if self.world.club(club).cash < 0 { 1 } else { self.runtime.rng.fresh_int(2) + 2 };
                let successor = self.replace_coach(club);
                self.runtime.sackings.push(brasfoot_model::runtime::Sacking { coach, successor, club, reason });
            } else if i > 5 {
                self.replace_coach(club);
                // the number of the components.h shown on the coach changes screen
                let _ = self.runtime.rng.fresh_int(3) + 1;
            }
        }
    }
}
