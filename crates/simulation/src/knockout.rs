// Port of the knockout stages: f.z.pO(), zs(), zx() and the rounds of f.G (creation, prize,
// resolution of the ties).

use brasfoot_model::ids::{ClubId, CoachId, MatchId, StageId};
use brasfoot_model::saved::KnockoutRound;

use crate::Sim;
use crate::tables::*;

impl Sim<'_> {
    /// `f.z.pO()`: a leg of a knockout round is over.
    pub fn knockout_round(&mut self, stage: StageId) {
        let s = self.world.stage(stage);
        let two_legged = s.rounds[s.round as usize].two_legged;
        if (s.leg == 1 && !two_legged) || s.leg == 2 {
            self.next_knockout_round(stage);
        } else {
            self.world.stage_mut(stage).leg = 2;
        }
    }

    /// `f.z.zs()`: the round is decided; the winners meet in the next one, or the stage ends.
    fn next_knockout_round(&mut self, stage: StageId) {
        let s = self.world.stage(stage);
        if s.phase_code == 2029 || s.round == s.last_round {
            // f.z.zx()
            let computed = s.rounds_computed;
            self.resolve_round(stage, computed, true);
            return;
        }
        self.resolve_round(stage, false, false);
        let s = self.world.stage(stage);
        let current = &s.rounds[s.round as usize];
        let winners = current.winners.clone();
        let away_goals = current.away_goals;
        let next = s.round + 1;
        let two_legged = s.two_legged_knockout_rounds[next as usize];
        let kind = s.competition_kind;
        self.new_knockout_round(stage, &winners, next, two_legged, 0, kind, away_goals);
        let s = self.world.stage_mut(stage);
        s.round += 1;
        s.leg = 1;
    }

    /// `f.G.c(int, int, boolean)`: the prize for winning a round.
    fn round_prize(&self, stage: StageId, kind: i32, round: i32) -> i32 {
        let s = self.world.stage(stage);
        let at = |table: &[i32], i: i32| if i >= 0 && (i as usize) < table.len() { table[i as usize] } else { 0 };
        match kind {
            2 => at(CUP_ROUND_PRIZES, round + CUP_ROUND_PRIZES.len() as i32 - 1 - s.last_round),
            5 => at(CLUB_WORLD_CUP_PRIZES, round),
            8 => INTERCONTINENTAL_PRIZES[0],
            10 => at(STATE_CUP_ROUND_PRIZES, round),
            11 => SUPER_CUP_PRIZES[0],
            4 => {
                // aq.sw by confederation; the qualifying rounds pay a fixed sum
                const PRIZES: [[i32; 6]; 6] = [
                    [2000000, 5000000, 7000000, 25000000, 30000000, 0],
                    [2000000, 3500000, 5000000, 20000000, 40000000, 0],
                    [500000, 1500000, 2000000, 4000000, 40000000, 0],
                    [500000, 1500000, 2000000, 4000000, 40000000, 0],
                    [500000, 1500000, 2000000, 4000000, 40000000, 0],
                    [200000, 500000, 1000000, 2000000, 20000000, 0],
                ];
                let competition = s.competition.expect("a knockout belongs to a competition");
                let mut confederation = self.world.competition(competition).subkind;
                if confederation < 0 {
                    confederation = 1;
                }
                match s.phase_code {
                    4101..=4103 => 500000,
                    4000..=4004 => 1000000,
                    _ => at(&PRIZES[confederation as usize], round),
                }
            }
            6 | 12 => {
                // aq.sy (kind 6) and aq.aeG (kind 12), the same sums: Europe's or else South America's
                const PRIZES: [[i32; 6]; 2] = [[500000, 700000, 2000000, 2500000, 7000000, 0], [500000, 1000000, 1500000, 5000000, 5000000, 0]];
                let competition = s.competition.expect("a knockout belongs to a competition");
                let mut confederation = self.world.competition(competition).subkind;
                if !(0..=1).contains(&confederation) {
                    confederation = 1;
                }
                match s.phase_code {
                    6100 => 300000,
                    6000..=6004 => 500000,
                    _ => at(&PRIZES[confederation as usize], round),
                }
            }
            _ => 0,
        }
    }

    /// `f.G.b(z, ArrayList, int, boolean, int, int, int, boolean)`: a round of a knockout, the
    /// clubs paired in order, on the next free dates of the kind.
    pub fn new_knockout_round(
        &mut self,
        stage: StageId,
        clubs: &[ClubId],
        round: i32,
        two_legged: bool,
        value_aan: i32,
        kind: i32,
        away_goals: bool,
    ) {
        let prize = self.round_prize(stage, kind, round);
        let mut r = KnockoutRound {
            kind,
            knockout: Some(stage),
            club_count: clubs.len() as i32,
            round,
            away_goals,
            two_legged,
            prize,
            value_aan,
            ..Default::default()
        };
        let competition = self.world.stage(stage).competition.expect("a knockout belongs to a competition");
        let skip_two = self.world.stage(stage).phase_code == 14006;
        let days = self.free_days(kind, skip_two);
        for pair in clubs.chunks(2) {
            r.first_leg.push(self.new_match(stage, round, pair[0], pair[1], days[0], competition, None));
        }
        r.days.push(days[0] as i32);
        if two_legged {
            for pair in clubs.chunks(2) {
                r.second_leg.push(self.new_match(stage, round, pair[1], pair[0], days[1], competition, None));
            }
            r.days.push(days[1] as i32);
        }
        let single = r.first_leg.len() == 1;
        self.world.stage_mut(stage).rounds.push(r);
        if single && self.world.competition(competition).third_place_match {
            let losers = self.semi_final_losers(stage);
            if losers.len() == 2 {
                let m = self.new_match(stage, round, losers[0], losers[1], days[0], competition, None);
                let s = self.world.stage_mut(stage);
                let last = s.rounds.len() - 1;
                s.rounds[last].first_leg.push(m);
            }
        }
    }

    /// `f.z.zt()`: the losers of the rounds with two ties, latest round first.
    fn semi_final_losers(&self, stage: StageId) -> Vec<ClubId> {
        let s = self.world.stage(stage);
        let mut losers = Vec::new();
        for r in s.rounds.iter().rev() {
            if r.first_leg.len() == 2 {
                for &m in &r.first_leg {
                    let g = self.world.game_match(m);
                    for club in [g.home, g.away].into_iter().flatten() {
                        if !r.winners.contains(&club) {
                            losers.push(club);
                        }
                    }
                }
            }
        }
        losers
    }

    /// `f.G.b(boolean, boolean)`: who goes through in each tie of the current round. `last`
    /// ends the stage; `computed` (`z.ZL`) records the title of a plain knockout.
    fn resolve_round(&mut self, stage: StageId, computed: bool, last: bool) {
        let s = self.world.stage(stage);
        let index = s.round as usize;
        let r = s.rounds[index].clone();
        let mut winners = r.winners.clone();
        let mut losers = Vec::new();
        for (i, &first) in r.first_leg.iter().enumerate() {
            let f = self.world.game_match(first);
            let (a_first, b_first) = (f.home_goals, f.away_goals);
            let (mut a_total, mut b_total) = (a_first, b_first);
            let (mut a_wins, mut b_wins) = (0, 0);
            if a_first > b_first {
                a_wins += 1;
            } else if b_first > a_first {
                b_wins += 1;
            }
            let mut a_away = 0;
            if r.two_legged {
                let g = self.world.game_match(r.second_leg[i]);
                a_away = g.away_goals;
                let b_second = g.home_goals;
                if a_away > b_second {
                    a_wins += 1;
                } else if b_second > a_away {
                    b_wins += 1;
                }
                a_total = a_first + a_away;
                b_total = b_first + b_second;
            }
            let mut outcome = if a_wins > b_wins {
                1
            } else if b_wins > a_wins {
                2
            } else {
                0
            };
            if outcome == 0 {
                outcome = if a_total > b_total {
                    1
                } else if b_total > a_total {
                    2
                } else {
                    0
                };
            }
            if outcome == 0 && r.two_legged && r.away_goals {
                if a_away > b_first {
                    outcome = 1;
                } else if b_first > a_away {
                    outcome = 2;
                }
            }
            if outcome == 0 && r.format_code == 2129 {
                outcome = 2;
            } else if outcome == 0 {
                let decider = if r.two_legged { r.second_leg[i] } else { first };
                let d = self.world.game_match(decider);
                outcome = if d.penalties_if_level && d.winner.is_some() {
                    if d.winner == f.home { 1 } else { 2 }
                } else {
                    // the original compares the home club's tiebreak with itself: never less
                    2
                };
            }
            let (home, away) = (f.home.expect("home club"), f.away.expect("away club"));
            let (winner, loser) = if outcome == 1 { (home, away) } else { (away, home) };
            winners.push(winner);
            losers.push(loser);
            let decider = if r.two_legged { r.second_leg[i] } else { first };
            let (home_result, away_result) = if outcome == 1 { (1, 2) } else { (2, 1) };
            self.knockout_mood(decider, self.world.club(home).coach, home_result);
            self.knockout_mood(decider, self.world.club(away).coach, away_result);
            self.collect_round_prize(stage, index, winner);
            if (90..=95).contains(&self.world.stage(stage).phase_code) {
                crate::unported!("f.q.a(at, Z): the World Cup's qualified countries");
            }
        }
        self.world.stage_mut(stage).rounds[index].winners = winners.clone();
        if last {
            self.end_knockout(stage, computed, &winners, &losers);
        }
    }

    /// `f.G.a(I, al, int)`: the board and the fans react to a tie won (1) or lost (2).
    fn knockout_mood(&mut self, decider: MatchId, coach: Option<CoachId>, result: i32) {
        let Some(coach) = coach else { return };
        let kind = self.world.game_match(decider).competition.map(|c| self.world.competition(c).kind);
        if kind.is_some_and(|k| k != 7 && k != 9) {
            self.coach_mood(coach, decider, true, result);
        }
    }

    /// `f.G.V(ah)`: the winner of a tie takes the prize (human clubs only).
    fn collect_round_prize(&mut self, stage: StageId, index: usize, winner: ClubId) {
        let r = &self.world.stage(stage).rounds[index];
        let prize = self.round_prize(stage, r.kind, r.round);
        self.world.stage_mut(stage).rounds[index].prize = prize;
        if prize > 0 && self.world.club(winner).human {
            self.world.club_income(winner, prize, 3);
            if let Some(coach) = self.world.club(winner).coach
                && self.world.coach(coach).human
            {
                let about = brasfoot_engine::messages::Composed { stage: Some(stage), money: Some(prize as i64), ..Default::default() };
                brasfoot_engine::messages::send_composed(self.world, coach, 26, 79, about);
            }
        }
    }

    /// The end of `f.G.b(boolean, boolean)`: what each format does with the last round.
    fn end_knockout(&mut self, stage: StageId, computed: bool, winners: &[ClubId], losers: &[ClubId]) {
        let s = self.world.stage(stage);
        let (phase, format) = (s.phase_code, s.rounds[s.round as usize].format_code);
        match phase {
            2029 if format != 2729 => {
                let (competition, index) = (s.competition.expect("a cup"), s.round as usize);
                self.cup_next_phase(competition, format, index);
            }
            1802 | 1803 => {}
            // f.B.BO(), f.B.r(ArrayList, ArrayList): the promotion knockout, the relegation playoff
            1099 => {
                let league = s.competition.expect("a league");
                self.promotion_knockout_over(league);
            }
            1098 => {
                let league = s.competition.expect("a league");
                let (winners, losers) = (winners.to_vec(), losers.to_vec());
                self.relegation_playoff_over(league, &winners, &losers);
            }
            -1 | 2029 if computed => {
                let competition = s.competition.expect("a knockout belongs to a competition");
                let group_stage = s.group_stage;
                let first = s.rounds[s.round as usize].first_leg[0];
                let g = self.world.game_match(first);
                let (champion, runner_up) = if Some(winners[0]) == g.home { (g.home, g.away) } else { (g.away, g.home) };
                self.record_title(competition, group_stage, champion, runner_up);
            }
            -1 | 2029 => {}
            // f.v.aj, g, h, j, k: the Champions League's qualifying rounds
            4000..=4004 => {
                let cup = s.competition.expect("the Champions League");
                let (winners, losers) = (winners.to_vec(), losers.to_vec());
                match phase {
                    4000 => self.champions_league_first_qualifying(cup, &winners),
                    4001 => self.champions_league_second_qualifying(cup, &winners, &losers),
                    4002 => self.champions_league_third_qualifying(cup, &winners, &losers),
                    4003 => self.champions_league_playoffs(cup, &winners, &losers),
                    _ => self.champions_league_groups(cup, &winners, &losers),
                }
            }
            // f.r.g, h, i: the Libertadores' qualifying rounds
            4101..=4103 => {
                let cup = s.competition.expect("the Libertadores");
                let (winners, losers) = (winners.to_vec(), losers.to_vec());
                match phase {
                    4101 => self.libertadores_second_round(cup, &winners, &losers),
                    4102 => self.libertadores_third_round(cup, &winners, &losers),
                    _ => self.libertadores_groups(cup, &winners, &losers),
                }
            }
            // f.x.g, m, l: the Europa League's rounds
            6001 | 6002 | 6004 | 6006 | 12001..=12004 | 12006 | 6100 => {
                let cup = s.competition.expect("a continental cup");
                let (winners, losers) = (winners.to_vec(), losers.to_vec());
                match phase {
                    6001 => self.europa_league_second_round(cup, &winners, &losers),
                    6002 | 6004 => self.europa_league_groups(cup, &winners, &losers),
                    // f.J.g, h, j, m, l: the Conference League's
                    12001 => self.conference_league_second_round(cup, &winners, &losers),
                    12002 => self.conference_league_third_round(cup, &winners, &losers),
                    12003 => self.conference_league_playoffs(cup, &winners, &losers),
                    12004 => self.conference_league_groups(cup, &winners, &losers),
                    // f.H.o: the Sudamericana's
                    6100 => self.sudamericana_groups(cup, &winners, &losers),
                    _ => self.second_tier_final_knockout(cup, &winners, &losers),
                }
            }
            // the World Cup's qualifiers: f.i.aw, f.j.aw, f.i.p, f.j.ax, f.k.ay, f.m.p
            9200 | 9300 | 9202 | 9303 | 9401 | 9502 => {
                let q = s.competition.expect("a qualifier");
                let (winners, losers) = (winners.to_vec(), losers.to_vec());
                match phase {
                    9200 | 9300 => self.qualifiers_groups(q, &winners),
                    9202 => self.african_qualifiers_end(q, &winners, &losers),
                    9303 => self.intercontinental_playoff(q, &winners),
                    9401 => self.concacaf_qualifiers_final_stage(q, &winners),
                    _ => self.oceania_qualifiers_end(q, &winners, &losers),
                }
            }
            // f.l.az: the qualified for the Euro or the World Cup
            9002 => {
                let q = s.competition.expect("the Euro qualifiers");
                let winners = winners.to_vec();
                self.euro_qualifiers_qualified(q, &winners);
            }
            // f.M.aA: the relegation playoff's losers go down
            14006 => {
                let cup = s.competition.expect("the Nations League");
                self.world.competition_mut(cup).list_xc = losers.to_vec();
            }
            // f.A.al: the world club cup's semi-finals
            150 => {
                let cup = s.competition.expect("the world club cup");
                let winners = winners.to_vec();
                self.intercontinental_semi_finals(cup, &winners);
            }
            _ => crate::unported!(format!("f.G.b(boolean, boolean): the end of format {phase}")),
        }
    }
}
