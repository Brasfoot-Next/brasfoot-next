// Port of the AI's coach changes: ah.kE() (a board replaces its coach), f.a(ah, int) and f.bh()
// (an unemployed coach for the job), ay.b(al, boolean) with Z.a and f.s.b (clubs that would
// swap coaches), al.i(al) (a coach leaves), al.E(ah) (a coach arrives) and ah.lg() (the club's
// finances restart).

use brasfoot_model::ids::{ClubId, CoachId, CountryId, StageId};
use brasfoot_model::saved::{ClubFinances, CoachChange};

use crate::Sim;
use crate::tables::{BASE_CASH, BASE_INCOME};

impl Sim<'_> {
    /// `ah.kE()`: the club sacks its coach and finds another; returns him.
    pub fn replace_coach(&mut self, club: ClubId) -> Option<CoachId> {
        let old = self.world.club(club).coach.expect("a club that sacks has a coach");
        self.remember_last_club(old);
        let reputation = self.world.club(club).reputation;
        let mut new = self.unemployed_coach(club, 0);
        if new.is_none() {
            new = self.unemployed_coach(club, 1);
        }
        if new.is_none() && reputation <= 3 {
            new = self.unemployed_coach(club, 2);
        }
        let human = self.world.coach(old).human;
        if new.is_none() && human {
            new = self.unemployed_coach(club, -1);
        }
        if new.is_none() && human {
            // f.bh(): any coach without a club
            new = self.world.game.coaches.iter().copied().find(|&c| {
                let coach = self.world.coach(c);
                coach.club_id.is_none() && !coach.human
            });
        }
        if new.is_none() && !human {
            let offers = self.job_offers(old, false);
            if let Some(&other) = offers.first() {
                new = self.world.club(other).coach;
                if let Some(n) = new
                    && n != old
                {
                    self.swap_coaches(n, old);
                }
            }
        } else if let Some(n) = new
            && n != old
        {
            self.leave(old, n);
            self.arrive(n, club);
        }
        new
    }

    /// `al.lN()`: the coach remembers the club he is leaving.
    pub(crate) fn remember_last_club(&mut self, coach: CoachId) {
        let club = self.coach_club(coach).expect("a coach with a club");
        let (country, division) = (self.world.club(club).country, self.world.club(club).division);
        let c = self.world.coach_mut(coach);
        c.previous_club = Some(club);
        c.previous_country = country;
        c.previous_division = division - 1;
    }

    /// `f.a(ah, int)`: an unemployed AI coach of the club's country with a fitting reputation:
    /// tier 0 the club's, 1 one below, 2 two below to one above, -1 any.
    fn unemployed_coach(&mut self, club: ClubId, tier: i32) -> Option<CoachId> {
        let (reputation, country) = (self.world.club(club).reputation, self.world.club(club).country);
        let (low, high) = match tier {
            -1 => (0, 5),
            0 => (reputation, reputation),
            1 => (reputation - 1, reputation),
            _ => (reputation - 2, reputation + 1),
        };
        let mut candidates: Vec<CoachId> = self
            .world
            .game
            .coaches
            .iter()
            .copied()
            .filter(|&c| {
                let coach = self.world.coach(c);
                coach.club_id.is_none()
                    && !coach.human
                    && (coach.previous_country == country || coach.country == country)
                    && coach.reputation >= low
                    && coach.reputation <= high
            })
            .collect();
        if !candidates.is_empty() {
            if self.runtime.rng.fresh_int(100) < 50 {
                candidates.sort_by(|&a, &b| crate::sorting::coaches(self.world, a, b));
            } else {
                self.runtime.rng.shuffle(&mut candidates);
            }
        }
        candidates.first().copied()
    }

    /// `f.o(int)`: the league country with a country id.
    pub fn league_country(&self, id: i32) -> Option<CountryId> {
        self.world.game.league_countries.iter().copied().find(|&c| self.world.country(c).country == id)
    }

    /// `f.s(int)`: the country with a country id.
    pub(crate) fn any_country(&self, id: i32) -> Option<CountryId> {
        self.world.game.countries.iter().copied().find(|&c| self.world.country(c).country == id)
    }

    /// `ay.b(al, boolean)`: clubs whose coach could swap with this one.
    pub fn job_offers(&mut self, coach: CoachId, top: bool) -> Vec<ClubId> {
        let mut offers = Vec::new();
        let home = self.league_country(self.world.coach(coach).country);
        let last = self.any_country(self.world.coach(coach).previous_country);
        if let Some(last) = last {
            self.country_offers(last, &mut offers, coach, top, true);
        }
        if let Some(home) = home
            && Some(home) != last
        {
            self.country_offers(home, &mut offers, coach, top, false);
        }
        let c = self.world.coach(coach);
        if c.human && (c.previous_division < 2 || c.reputation > 3) {
            // a human coach also hears from the other league countries, in a shuffled order,
            // until there are seven offers
            let mut abroad: Vec<CountryId> =
                self.world.game.league_countries.iter().copied().filter(|&k| Some(k) != last && Some(k) != home).collect();
            if !abroad.is_empty() {
                self.runtime.rng.shuffle(&mut abroad);
                for country in abroad {
                    self.country_offers(country, &mut offers, coach, top, false);
                    if offers.len() > 6 {
                        break;
                    }
                }
            }
        }
        offers
    }

    /// `Z.a(ArrayList, al, boolean, boolean)`: a club from some divisions of a country, chosen
    /// by the coach's division (`same_country` is the country he last worked in).
    fn country_offers(&mut self, country: CountryId, offers: &mut Vec<ClubId>, coach: CoachId, top: bool, same_country: bool) {
        const NEAR: [[i32; 6]; 5] = [[0, 0, 1, 2, -1, -1], [1, 1, 2, 3, -1, -1], [2, 2, 3, -1, -1, -1], [2, 3, 3, -1, -1, -1], [-1; 6]];
        const NEAR_B: [[i32; 6]; 5] = [[0, -1, -1, 2, -1, -1], [1, -1, 2, -1, -1, -1], [2, -1, -1, -1, -1, -1], [2, 3, -1, -1, -1, -1], [-1; 6]];
        const NEAR_C: [[i32; 6]; 5] = [[0, -1, -1, -1, -1, -1], [1, -1, -1, -1, -1, -1], [2, -1, -1, -1, -1, -1], [2, 3, -1, -1, -1, -1], [-1; 6]];
        const OTHER: [[i32; 6]; 5] = [[0, 1, -1, -1, -1, -1], [1, 2, -1, -1, -1, -1], [2, 8, -1, -1, -1, -1], [3, 8, -1, -1, -1, -1], [-1; 6]];
        const OTHER_B: [[i32; 6]; 5] = [[0, -1, -1, -1, -1, -1], [1, -1, -1, -1, -1, -1], [8, 8, -1, -1, -1, -1], [8, 8, -1, -1, -1, -1], [-1; 6]];
        let divisions = self.world.country(country).divisions.clone();
        let size = divisions.len() as i32;
        let mut division = match self.coach_club(coach) {
            None => self.world.coach(coach).previous_division,
            Some(club) => self.world.club(club).division - 1,
        };
        if division < 0 {
            division = size - 1;
        }
        if division > size - 1 {
            division = size - 1;
        }
        if division < 0 {
            return;
        }
        let d = division as usize;
        let draw = self.runtime.rng.fresh_int(1000);
        let mut picks = NEAR[d];
        if draw > 700 {
            picks = NEAR_B[d];
        } else if draw > 500 {
            picks = NEAR_C[d];
        }
        if !same_country {
            picks = if draw > 500 { OTHER_B[d] } else { OTHER[d] };
        }
        for mut pick in picks {
            if pick == 8 {
                pick = size - 1;
            }
            if pick >= 0 && pick < size {
                let league = divisions[pick as usize];
                let stage = self.world.competition(league).league.expect("a league has its table");
                if !self.world.stage(stage).clubs.is_empty() {
                    self.stage_offer(stage, offers, coach, top);
                } else if !self.world.coach(coach).human {
                    self.loose_club_offer(country, offers, coach);
                }
            }
        }
    }

    /// `f.s.b(ArrayList, al, boolean)`: an AI club from the top (places 3 to 6) or the bottom
    /// six of a table, other than the coach's last club.
    fn stage_offer(&mut self, stage: StageId, offers: &mut Vec<ClubId>, coach: CoachId, top: bool) {
        let clubs = self.world.stage(stage).clubs.clone();
        let (from, to) = if top { (2, 6) } else { (clubs.len() as i32 - 6, clubs.len() as i32) };
        let to = to.min(clubs.len() as i32);
        let previous = self.world.coach(coach).previous_club;
        let mut candidates = Vec::new();
        for i in from..to {
            // a table shorter than six makes the original read from a negative index and fail
            let club = clubs[i as usize];
            if !self.world.club(club).human && Some(club) != previous {
                candidates.push(club);
            }
        }
        self.runtime.rng.shuffle(&mut candidates);
        if let Some(&club) = candidates.iter().find(|c| !offers.contains(c)) {
            offers.push(club);
        }
    }

    /// `Z.a(ArrayList, al, boolean)`: one of the country's clubs outside the leagues.
    fn loose_club_offer(&mut self, country: CountryId, offers: &mut Vec<ClubId>, coach: CoachId) {
        let previous = self.world.coach(coach).previous_club;
        let mut candidates: Vec<ClubId> = self
            .world
            .country(country)
            .clubs
            .iter()
            .copied()
            .filter(|&c| !self.world.club(c).human && Some(c) != previous)
            .collect();
        self.runtime.rng.shuffle(&mut candidates);
        if let Some(&club) = candidates.first()
            && !offers.contains(&club)
        {
            offers.push(club);
        }
    }

    /// `f.a(al, al)`: two coaches swap clubs.
    fn swap_coaches(&mut self, a: CoachId, b: CoachId) {
        let (club_a, club_b) = (self.coach_club(a), self.coach_club(b));
        self.leave(a, b);
        self.leave(b, a);
        self.arrive(a, club_b.expect("a coach with a club"));
        self.arrive(b, club_a.expect("a coach with a club"));
    }

    /// `al.i(al)`: the coach leaves his club, replaced by `successor`.
    fn leave(&mut self, coach: CoachId, successor: CoachId) {
        let club = self.coach_club(coach);
        // the original stamps the change with the calendar day and the wall clock's time of
        // day; the calendar days carry the time of day of the season's creation
        let change = CoachChange {
            coach_out: Some(coach),
            coach_in: Some(successor),
            date: self.world.today_millis(),
            club_id: club.map(|c| self.world.club(c).id).unwrap_or(-1),
            ..Default::default()
        };
        self.world.game.coach_changes.push(change);
        self.remember_last_club(coach);
        let club = club.expect("a coach with a club");
        if self.world.coach(coach).human {
            // the club goes back to the AI, with new books (ah.lg()); the game remembers its
            // country (f.v(int)) and the coach's messages are gone (al.T(null))
            if self.world.club(club).country == 29 {
                crate::unported!("f.u(int): the state of the Brazilian club a human coach leaves");
            }
            self.world.game.value_bf = self.world.club(club).country;
            self.world.club_mut(club).human = false;
            self.restart_finances(club);
            self.world.club_mut(club).auto_training = true;
            self.world.game.human_clubs.retain(|&c| c != club);
            self.world.coach_mut(coach).messages.clear();
        }
        self.world.club_mut(club).finances.bank_loan = 0;
        self.world.club_mut(club).coach = None;
        self.world.coach_mut(coach).club_id = None;
    }

    /// `al.E(ah)`: the coach takes over a club.
    fn arrive(&mut self, coach: CoachId, club: ClubId) {
        self.world.club_mut(club).coach = Some(coach);
        let c = self.world.coach_mut(coach);
        c.club_id = Some(club);
        c.board_confidence = 95;
        c.fan_support = 85;
        c.seasons_at_club = 0;
        self.restart_finances(club);
        if self.world.coach(coach).human {
            // the club becomes the human's: nobody listed, a welcome message (0/74), the pitch's
            // condition drawn again (v.i(ah)) and six more months for every contract
            self.world.club_mut(club).human = true;
            self.world.game.human_clubs.push(club);
            for p in self.world.club(club).squad.clone() {
                let player = self.world.player_mut(p);
                player.listed_for_sale = false;
                player.listed_for_loan = false;
            }
            let name = self.world.club(club).name.clone();
            brasfoot_engine::messages::send(self.world, coach, 0, 74, name, String::new());
            if let Some(stadium) = self.world.club(club).stadium {
                let draw = self.runtime.rng.fresh_int(100);
                self.world.stadium_mut(stadium).condition = if draw >= 70 { 1 } else { 0 };
            }
            for p in self.world.club(club).squad.clone() {
                self.extend_contract(p, 180, true);
            }
        }
    }

    /// `gO.rT()` for a club's offer: the coach leaves his club, if he has one (`ah.kE()`), and
    /// takes over the new one from its coach (`f.a(ah, al, al)`).
    pub fn take_job(&mut self, coach: CoachId, club: ClubId) {
        if let Some(current) = self.coach_club(coach) {
            self.replace_coach(current);
        }
        if let Some(previous) = self.world.club(club).coach {
            self.leave(previous, coach);
        }
        self.arrive(coach, club);
    }

    /// `ah.lg()` with `ah.kH()`: new books and the division's base cash.
    fn restart_finances(&mut self, club: ClubId) {
        let c = self.world.club(club);
        let division = c.division as usize;
        let mut cash = BASE_CASH[division][0];
        if c.country != 29 && self.world.game.plays_state {
            let wages = self.world.club_wages(club);
            cash = (cash as f64 + 3.2 * wages as f64) as i64;
        }
        let c = self.world.club_mut(club);
        c.cash = cash;
        c.finances = ClubFinances { income_type6: BASE_INCOME[division][0], ..ClubFinances::default() };
    }
}
