// Port of the national teams' squads and coaches: ag.jP(), Z.z(boolean), Z.ji(), Z.g(al) and
// Z.f(al).

use std::cmp::Ordering;

use brasfoot_model::ids::{ClubId, CoachId, CountryId, PlayerId};
use brasfoot_model::world::World;

use crate::Sim;

/// `Z.hF`: strength, then the stars first.
fn by_strength_then_star(world: &World, a: PlayerId, b: PlayerId) -> Ordering {
    let (pa, pb) = (world.player(a), world.player(b));
    pb.strength.cmp(&pa.strength).then(pb.star.cmp(&pa.star))
}

/// `h.a.abe` (`h.y`): position, style, then strength and the stars first.
fn by_position(world: &World, a: PlayerId, b: PlayerId) -> Ordering {
    let (pa, pb) = (world.player(a), world.player(b));
    pa.position
        .cmp(&pb.position)
        .then(pa.style.cmp(&pb.style))
        .then(pb.strength.cmp(&pa.strength))
        .then(pb.star.cmp(&pa.star))
}

impl Sim<'_> {
    /// `ag.jP()`: each AI national team calls up its squad (a human one keeps it).
    pub(crate) fn national_squads(&mut self) {
        for country in self.world.game.countries.clone() {
            if let Some(team) = self.world.country(country).national_team
                && !self.world.club(team).human
            {
                self.call_up(country, false);
            }
        }
    }

    /// `Z.z(boolean)`: the national team's squad of the country's best players by place on the
    /// pitch (3 keepers; 4 full-backs, 4 centre-backs, 5 midfielders, 4 forwards and 3 more
    /// midfielders by side and style), filled by position when there are fewer than 23; with
    /// `new_coach`, or without one, an AI team gets a coach.
    pub(crate) fn call_up(&mut self, country: CountryId, new_coach: bool) {
        self.runtime.checkpoint("best.Z.z(Z)V");
        let team = self.world.country(country).national_team.expect("Z.jo() makes the national team first");
        self.world.club_mut(team).squad.clear();
        if (new_coach || self.world.club(team).coach.is_none())
            && !self.world.club(team).human
            && let Some(coach) = self.national_coach_candidate(country)
        {
            self.appoint_national_coach(country, coach);
        }
        let c = self.world.country(country);
        if !c.flag_hz && !self.enough_players(country) {
            let (id, level) = (c.country, c.level);
            self.made_up_players(id, level);
        }
        let id = self.world.country(country).country;
        let mut candidates: Vec<PlayerId> = self.world.game.players.iter().copied().filter(|&p| self.world.player(p).country == id && self.world.player(p).club.is_some()).collect();
        candidates.extend(self.world.game.generated_players.iter().copied().filter(|&p| self.world.player(p).country == id));
        candidates.sort_by(|&a, &b| by_strength_then_star(self.world, a, b));
        let mut total = [3, 4, 4, 5, 4, 3];
        let mut left = [0, 2, 2, 2, 2, 2];
        let mut right = [0, 2, 2, 3, 2, 1];
        let mut squad: Vec<PlayerId> = Vec::new();
        for &p in &candidates {
            let pl = self.world.player(p);
            let (pos, side, style) = (pl.position, pl.side, pl.style);
            // the counters of the place the player would fill, as the original checks them
            let place: Option<(&mut [i32; 6], usize, usize)> = if pos == 0 {
                if total[0] > 0 {
                    squad.push(p);
                    total[0] -= 1;
                }
                None
            } else if pos >= 3 || side != 0 {
                if pos < 3 && side == 1 {
                    Some((&mut right, pos as usize, pos as usize))
                } else if pos == 3 && side == 0 && style == 1 {
                    Some((&mut left, 3, 3))
                } else if pos == 3 && side == 1 && style == 1 {
                    Some((&mut right, 3, 3))
                } else if pos == 3 && side == 0 && style == 0 {
                    Some((&mut left, 5, 5))
                } else if pos == 3 && side == 1 && style == 0 {
                    Some((&mut right, 5, 5))
                } else if pos == 4 && side == 0 {
                    Some((&mut left, 4, 4))
                } else if pos == 4 && side == 1 {
                    Some((&mut right, 4, 4))
                } else {
                    None
                }
            } else {
                Some((&mut left, pos as usize, pos as usize))
            };
            if let Some((counter, i, t)) = place
                && counter[i] > 0
            {
                squad.push(p);
                counter[i] -= 1;
                total[t] -= 1;
            }
        }
        if candidates.len() < 23 {
            for (i, &left) in total.iter().enumerate() {
                let pos = if i < 5 { i as i32 } else { 3 };
                if left > 0 {
                    for &p in &candidates {
                        if self.world.player(p).position == pos && !squad.contains(&p) {
                            squad.push(p);
                        }
                    }
                }
            }
            for &p in &candidates {
                if !squad.contains(&p) {
                    squad.push(p);
                }
            }
        }
        squad.sort_by(|&a, &b| by_position(self.world, a, b));
        let t = self.world.club_mut(team);
        t.squad = squad;
        t.corner_taker = None;
        t.free_kick_taker = None;
        crate::human::lineup::pick_penalty_taker(self.world, team);
        crate::human::lineup::pick_captain(self.world, team);
    }

    /// `Z.jo()`: the country's national team, made on first use: the country's name, level and
    /// reputation by level (`Z.jt()`), the next club number and its flag's colors.
    pub(crate) fn national_team(&mut self, country: CountryId) -> ClubId {
        if let Some(team) = self.world.country(country).national_team {
            return team;
        }
        let c = self.world.country(country);
        let id = c.country;
        let level = c.level;
        // Z.jt()
        // ah.setNivel(int)
        let level = if (1..=25).contains(&level) { level } else { 10 };
        let reputation = match c.level {
            20.. => 5,
            19 => 4,
            17 | 18 => 3,
            15 | 16 => 2,
            _ => 1,
        };
        let info = brasfoot_model::countries::country(id).expect("a country of the game");
        let hex = |rgb: [u8; 3]| format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        // ah's field initialiser: hr = new Random().nextInt(1000) + 1
        let tiebreak = self.runtime.rng.fresh_int(1000) + 1;
        let team = brasfoot_model::saved::Club {
            name: c.text_hb.clone(),
            team_file_id: -1,
            country: id,
            state: -1,
            level,
            reputation,
            tiebreak_number: tiebreak,
            id: self.world.game.next_club_id,
            // d(jB()) the second color, c(jC()) the first
            color1: hex(brasfoot_model::countries::flag_color(info.color_table2)),
            color2: hex(brasfoot_model::countries::flag_color(info.color_table1)),
            ..Default::default()
        };
        self.world.game.next_club_id += 1;
        let team_id = ClubId(self.world.clubs.len() as u32);
        self.world.clubs.push(team);
        self.world.country_mut(country).national_team = Some(team_id);
        team_id
    }

    /// `Z.jk()`: sixteen outfield players and two keepers of the country.
    fn enough_players(&self, country: CountryId) -> bool {
        let id = self.world.country(country).country;
        let (mut outfield, mut keepers) = (0, 0);
        for &p in self.world.game.players.iter().chain(&self.world.game.generated_players) {
            let pl = self.world.player(p);
            if pl.country == id {
                if pl.position == 0 {
                    keepers += 1;
                } else {
                    outfield += 1;
                }
            }
        }
        outfield >= 16 && keepers >= 2
    }

    /// `Z.ji()`: an AI coach without a national team: of the country with reputation 5, then 4,
    /// then any with 5, any with 4, the country's with 3 and any with 3.
    fn national_coach_candidate(&self, country: CountryId) -> Option<CoachId> {
        let id = self.world.country(country).country;
        let mut lists: [Vec<CoachId>; 6] = Default::default();
        for &c in &self.world.game.coaches {
            let coach = self.world.coach(c);
            if coach.human || coach.national_team.is_some() {
                continue;
            }
            let own = coach.country == id;
            match coach.reputation {
                5 if own => lists[0].push(c),
                4 if own => lists[1].push(c),
                3 if own => lists[4].push(c),
                _ => {}
            }
            match coach.reputation {
                5 => lists[2].push(c),
                4 => lists[3].push(c),
                3 => lists[5].push(c),
                _ => {}
            }
        }
        lists.iter().find_map(|l| l.first().copied())
    }

    /// `Z.g(al)`: the coach takes the national team, leaving the one he had (`Z.f(al)`); the
    /// team's last coach goes.
    fn appoint_national_coach(&mut self, country: CountryId, coach: CoachId) {
        let team = self.world.country(country).national_team.expect("the country's national team");
        if let Some(old) = self.world.club(team).coach {
            self.world.coach_mut(old).national_team = None;
        }
        if self.world.coach(coach).national_team.is_some() {
            self.release_national_coach(coach);
        }
        self.set_team_coach(team, Some(coach));
        self.world.coach_mut(coach).national_team = Some(team);
        self.world.club_mut(team).human = self.world.coach(coach).human;
    }

    /// `ah.h(al)`
    fn set_team_coach(&mut self, team: ClubId, coach: Option<CoachId>) {
        self.world.club_mut(team).coach = coach;
    }

    /// `Z.f(al)`: the coach leaves his national team, which gets another (picked before he
    /// goes) and calls up again.
    fn release_national_coach(&mut self, coach: CoachId) {
        let team = self.world.coach(coach).national_team.expect("the coach has a national team");
        let id = self.world.club(team).country;
        let Some(other) = self.any_country(id) else { return };
        let next = self.national_coach_candidate(other);
        self.world.coach_mut(coach).national_team = None;
        let other_team = self.world.country(other).national_team.expect("Z.jo()");
        self.set_team_coach(other_team, None);
        self.world.club_mut(other_team).human = false;
        if let Some(next) = next {
            self.appoint_national_coach(other, next);
        }
        self.call_up(other, false);
    }

    /// `best.a.b(boolean)`: with every national team, the human coaches without one (all of
    /// them with `all`) see the national teams' offers (`ay.a(al, boolean)`) and may take one
    /// (`a.gO.rT()`: `Z.g(al)`).
    pub(crate) fn national_team_offers_to_humans(&mut self, all: bool) {
        if !self.world.game.plays_all_national_teams {
            return;
        }
        for coach in self.world.game.human_coaches.clone() {
            let c = self.world.coach(coach);
            if !c.human || (!all && c.national_team.is_some()) {
                continue;
            }
            let offers = self.national_team_offers(coach);
            // f.bD() (transient, false here) would skip the screen
            if offers.is_empty() {
                continue;
            }
            let pick = self.ask(|h, w| h.national_team_offer(w, coach, &offers));
            if let Some(team) = pick
                && offers.contains(&team)
            {
                let id = self.world.club(team).country;
                if let Some(country) = self.any_country(id) {
                    self.appoint_national_coach(country, coach);
                }
            }
        }
    }

    /// `ay.a(al, boolean)`: the AI national teams of the job market's countries, shuffled; by
    /// the coach's reputation (or his club's, if higher) a few of each reputation, the
    /// confederations in turn and at most one from Oceania; the country of the first human
    /// coach's nationality takes the last place when it is small or he coaches a big club.
    fn national_team_offers(&mut self, coach: CoachId) -> Vec<ClubId> {
        const WANTED: [[i32; 5]; 6] = [[2, 1, 1, 1, 1], [2, 2, 1, 1, 1], [3, 2, 2, 1, 1], [4, 3, 2, 2, 1], [5, 4, 5, 3, 2], [5, 4, 4, 3, 2]];
        const CONFEDERATIONS: [i32; 6] = [0, 1, 3, 2, 4, 5];
        let mut teams: Vec<ClubId> = Vec::new();
        for &id in &self.world.job_market.countries.clone() {
            if let Some(country) = self.any_country(id)
                && let Some(t) = self.world.country(country).national_team
                && !self.world.club(t).human
            {
                teams.push(t);
            }
        }
        self.runtime.rng.shuffle(&mut teams);
        let mut offers: Vec<ClubId> = Vec::new();
        if teams.is_empty() {
            return offers;
        }
        let confederation = |sim: &Self, t: ClubId| brasfoot_model::countries::confederation(sim.world.club(t).country);
        let mut reputation = self.world.coach(coach).reputation;
        if let Some(club) = self.coach_club(coach)
            && self.world.club(club).reputation > reputation
        {
            reputation = self.world.club(club).reputation;
        }
        let wanted = WANTED[reputation.clamp(0, 5) as usize];
        let mut oceania = false;
        for _ in 0..2 {
            let mut turn = 0;
            for &rep in &wanted {
                if rep < 1 {
                    continue;
                }
                for &t in &teams {
                    if self.world.club(t).reputation == rep && confederation(self, t) == CONFEDERATIONS[turn] && !offers.contains(&t) {
                        if CONFEDERATIONS[turn] == 5 && !oceania {
                            offers.push(t);
                            oceania = true;
                        } else if CONFEDERATIONS[turn] != 5 {
                            offers.push(t);
                        }
                        turn = if turn + 1 < CONFEDERATIONS.len() { turn + 1 } else { 0 };
                        break;
                    }
                }
            }
            if offers.len() < wanted.len() {
                for &rep in &wanted {
                    if rep >= 1 && offers.len() < wanted.len() {
                        for &t in &teams {
                            if self.world.club(t).reputation == rep && !offers.contains(&t) {
                                if confederation(self, t) == 5 && !oceania {
                                    offers.push(t);
                                    oceania = true;
                                    break;
                                }
                                if confederation(self, t) != 5 {
                                    offers.push(t);
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
        // c.a.SR.M().get(0): the first human coach's nationality
        if let Some(&first) = self.world.game.human_coaches.first() {
            let nationality = self.world.coach(first).country;
            if nationality >= 0 {
                let first_club = self.coach_club(first);
                let mut own = None;
                let mut take = false;
                for &t in &teams {
                    if self.world.club(t).country == nationality && !self.world.club(t).human && !offers.contains(&t) {
                        own = Some(t);
                        if self.world.club(t).level < 18 || first_club.is_some_and(|c| self.world.club(c).reputation >= 3) {
                            take = true;
                        }
                    }
                }
                if take && let Some(t) = own {
                    if offers.is_empty() {
                        offers.push(t);
                    } else {
                        let last = offers.len() - 1;
                        offers[last] = t;
                    }
                }
            }
        }
        offers
    }
}
