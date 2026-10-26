// Port of the round screen: best.s.dx() and the groups it shows (dI, dF, dy, dJ), and a.dV, which
// plays the human clubs' matches minute by minute and shows the others (Y, qa, qg, aa, a(v, I, A,
// int), qc, qd, qe, g(I)), with the game logic of its dialogs: the penalty of a.gb, the shoot-out
// of a.bg, and what opening a.cD does.

use std::cmp::Ordering;
use std::collections::HashMap;

use brasfoot_engine::clubs;
use brasfoot_engine::engine::MatchEngine;
use brasfoot_engine::simulate::{Play, event_kind};
use brasfoot_model::ids::{ClubId, CompetitionId, MatchId, PlayerId};
use brasfoot_model::saved::CompetitionKind;
use brasfoot_model::world::World;

use crate::Sim;

/// `MatchEvent::detail` of a penalty.
const PENALTY: i32 = 3;

/// A row of the round screen (`components.v`): a competition's title or a match, with the
/// goals the screen has shown.
#[derive(Clone, Debug)]
pub struct LiveRow {
    pub game_match: Option<MatchId>,
    pub competition: Option<CompetitionId>,
    /// `v.Pt`: a human club plays it.
    pub human: bool,
    /// `v.Pq`, `v.Pr`
    pub goals: [i32; 2],
}

/// The state of `best.s` and `a.dV` while a day's round is on screen.
pub struct LiveRound {
    /// `s.cS`: the groups of competitions shown one after the other, with whether each is still
    /// to be shown (`components.A.uc()`).
    groups: Vec<(Vec<CompetitionId>, bool)>,
    /// `dV.vK`
    rows: Vec<LiveRow>,
    /// `dV.GT`: the minute.
    minute: i32,
    /// `dV.dB`: the half.
    half: i32,
    /// `dV.GU`: the group's matches are over.
    over: bool,
    /// The engines `I.V()` left for the human clubs' matches (`I.go`).
    engines: HashMap<MatchId, MatchEngine>,
    /// `best.s.cR` is set: the screen is on.
    pub on: bool,
}

impl LiveRound {
    pub fn minute(&self) -> i32 {
        self.minute
    }

    pub fn half(&self) -> i32 {
        self.half
    }

    pub fn is_over(&self) -> bool {
        self.over
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn rows(&self) -> &[LiveRow] {
        &self.rows
    }

    pub fn human_row(&self) -> Option<&LiveRow> {
        self.rows.iter().find(|r| r.human)
    }
}

/// `h.a.abg`, the shoot-out order of an AI club: higher position first, else strength, else
/// fitness. It never says "after" for a lower position, so it is not consistent and the order
/// is the one Java's sort gives.
fn shootout_rule(world: &World, a: PlayerId, b: PlayerId) -> Ordering {
    let (pa, pb) = (world.player(a), world.player(b));
    if pa.position > pb.position {
        Ordering::Less
    } else if pa.strength > pb.strength {
        Ordering::Less
    } else if pa.strength < pb.strength {
        Ordering::Greater
    } else if pa.fitness > pb.fitness {
        Ordering::Less
    } else if pa.fitness < pb.fitness {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

impl Sim<'_> {
    fn new_play(&mut self, m: MatchId) -> Play<'_> {
        Play { world: self.world, runtime: self.runtime, hooks: self.hooks, game_match: m }
    }

    /// `at.iq()`: the country of a national league (other competitions have none here).
    fn competition_country(&self, c: CompetitionId) -> Option<brasfoot_model::ids::CountryId> {
        let competition = self.world.competition(c);
        if competition.class == CompetitionKind::NationalLeague { competition.country_ref } else { None }
    }

    fn human_plays(&self, m: MatchId) -> bool {
        let (h, a) = self.sides(m);
        self.world.club(h).human || self.world.club(a).human
    }

    /// `Z.ei()`: a human club is in one of the country's leagues.
    fn country_has_human_club(&self, country: brasfoot_model::ids::CountryId) -> bool {
        self.world.country(country).divisions.iter().any(|&d| {
            self.world.competition(d).league.is_some_and(|s| self.world.stage(s).clubs.iter().any(|&c| self.world.club(c).human))
        })
    }

    /// `Z.ej()`: one of the country's leagues plays today.
    fn country_plays_today(&self, country: brasfoot_model::ids::CountryId) -> bool {
        let divisions = &self.world.country(country).divisions;
        self.today_matches().iter().any(|&m| self.world.game_match(m).competition.is_some_and(|c| divisions.contains(&c)))
    }

    /// `s.b(at)`: whether the round screen shows a competition.
    fn round_shows(&self, c: CompetitionId) -> bool {
        if self.today_matches().iter().any(|&m| self.world.game_match(m).competition == Some(c) && self.human_plays(m)) {
            return true;
        }
        let kind = self.world.competition(c).kind;
        if kind == 0 {
            return false;
        }
        crate::unported!("f.ae(int, int) and the international options of the round screen")
    }

    /// The day of a human club's match with the screens on: `f.ap()` once every human line-up
    /// is confirmed. The AI picks its line-ups (`ah.kh()`), the AI-only matches are played
    /// (`I.he()`), the round screen plays the rest (`s.dx()`) and the day ends (`f.at()`).
    pub fn play_human_day(&mut self) {
        let mut round = self.start_human_day();
        while round.on {
            self.round_tick_step(&mut round);
        }
    }

    /// Starts today's round screen: AI lineups and matches are played, and the round screen
    /// is primed for display and stepping.
    pub fn start_human_day(&mut self) -> LiveRound {
        self.runtime.checkpoint("day");
        self.pick_ai_lineups();
        self.runtime.checkpoint("ai line-ups");
        let engines = self.play_today_with_engines();
        self.runtime.checkpoint("ai matches");
        let mut round = LiveRound { groups: Vec::new(), rows: Vec::new(), minute: 0, half: 1, over: false, engines, on: true };
        self.start_round(&mut round);
        round
    }

    /// Steps the round screen forward by one minute tick.
    pub fn round_tick_step(&mut self, round: &mut LiveRound) {
        if round.on {
            self.round_tick(round);
        }
    }

    /// `s.dx()`: the groups of today's competitions, by the kind of the first.
    fn start_round(&mut self, round: &mut LiveRound) {
        let day = &self.world.game.days[self.world.game.day as usize];
        let competitions = day.competitions.clone();
        if let Some(&first) = competitions.first() {
            match self.world.competition(first).kind {
                1 => self.league_groups(round, &competitions),
                2 => self.cup_groups(round, &competitions),
                // s.dB(), s.dC(): every competition of the day on the screen at once (dV.Y), no
                // groups; at its end s.dJ() finds none left
                9 | 11 => round.groups.push((competitions.clone(), true)),
                3 | 7 | 8 | 10 => crate::unported!("s.dE, dA, dz, dD: the round screen of these competitions"),
                _ => {
                    // s.dy()
                    for &c in &competitions {
                        if self.round_shows(c) {
                            round.groups.push((vec![c], true));
                        }
                    }
                }
            }
        }
        self.next_group(round);
    }

    /// `s.dI()`: one group per country with a human club in its leagues (or followed, `Z.jr()`)
    /// that plays today; else the country of `f.bz()`.
    fn league_groups(&mut self, round: &mut LiveRound, competitions: &[CompetitionId]) {
        let mut last = Vec::new();
        for country in self.world.game.league_countries.clone() {
            let followed = self.country_has_human_club(country) || self.world.country(country).flag_hc;
            if followed && self.country_plays_today(country) {
                last = competitions.iter().copied().filter(|&c| self.competition_country(c) == Some(country)).collect();
                if !last.is_empty() {
                    round.groups.push((last.clone(), true));
                }
            }
        }
        if last.is_empty() {
            let favourite = self.world.game.value_bf;
            for country in self.world.game.league_countries.clone().into_iter().rev() {
                if self.world.country(country).country == favourite && self.country_plays_today(country) {
                    let group: Vec<CompetitionId> = competitions.iter().copied().filter(|&c| self.competition_country(c) == Some(country)).collect();
                    if !group.is_empty() {
                        round.groups.push((group, true));
                    }
                }
            }
        }
    }

    /// `s.dF()`: one group per country whose cup has a human club's match today; else the cup
    /// of the first coach's country (`f.bA()`).
    fn cup_groups(&mut self, round: &mut LiveRound, competitions: &[CompetitionId]) {
        let mut last: Vec<CompetitionId> = Vec::new();
        for country in self.world.game.league_countries.clone() {
            let Some(cup) = self.world.country(country).cup else { continue };
            let today: Vec<MatchId> = self.today_matches().into_iter().filter(|&m| self.world.game_match(m).competition == Some(cup)).collect();
            let human = today.iter().any(|&m| self.human_plays(m));
            if human && !today.is_empty() {
                last = competitions.iter().copied().filter(|&c| c == cup).collect();
                if !last.is_empty() {
                    round.groups.push((last.clone(), true));
                }
            }
        }
        if last.is_empty() {
            crate::unported!("s.dF(): the cup of f.bA() when no human club plays a cup today");
        }
    }

    /// `s.dJ()`: the next group on screen (`dV.Y`), or the end of the day (`f.at()`).
    fn next_group(&mut self, round: &mut LiveRound) {
        if let Some(i) = round.groups.iter().position(|g| g.1) {
            round.groups[i].1 = false;
            let group = round.groups[i].0.clone();
            self.show_group(round, &group);
        } else {
            round.on = false;
            round.engines.clear();
            self.after_day();
        }
    }

    /// `dV.Y(ArrayList)`: the rows of a group: a title and the matches of each competition, in
    /// the day's order. Each match fixes its clubs' penalty taker and captain among the players
    /// on the pitch (`ah.Q`, `ah.O`).
    fn show_group(&mut self, round: &mut LiveRound, group: &[CompetitionId]) {
        round.minute = 0;
        round.rows.clear();
        round.half = 1;
        round.over = false;
        for &c in group {
            let mut first = true;
            for m in self.today_matches() {
                if self.world.game_match(m).competition != Some(c) {
                    continue;
                }
                if first {
                    round.rows.push(LiveRow { game_match: None, competition: Some(c), human: false, goals: [0, 0] });
                    first = false;
                }
                let (h, a) = self.sides(m);
                let human = self.world.club(h).human || self.world.club(a).human;
                let home_played = self.world.game_match(m).home_played.clone();
                let away_played = self.world.game_match(m).away_played.clone();
                self.fix_penalty_taker(h, &home_played);
                self.fix_captain(h, &home_played);
                self.fix_penalty_taker(a, &away_played);
                self.fix_captain(a, &away_played);
                let kind = self.world.competition(c).kind;
                if kind == 7 || (self.world.game_match(m).stadium.is_none() && (kind == 4 || kind == 6)) {
                    // I.p(String): the venue text of the row, transient
                }
                round.rows.push(LiveRow { game_match: Some(m), competition: Some(c), human, goals: [0, 0] });
            }
        }
    }

    /// `ah.Q(ArrayList)` and `ah.R(ArrayList)`: a penalty taker who is not on the pitch gives way
    /// to the strongest there with the penalty trait (9), else the strongest out of goal.
    fn fix_penalty_taker(&mut self, club: ClubId, on_pitch: &[PlayerId]) {
        if clubs::penalty_taker(self.world, club).is_some_and(|t| on_pitch.contains(&t)) {
            return;
        }
        let mut list = on_pitch.to_vec();
        if list.is_empty() {
            return;
        }
        brasfoot_engine::lineup::sort_by_strength(self.world, &mut list);
        let taker = list
            .iter()
            .copied()
            .find(|&p| self.world.player(p).trait1 == 9)
            .or_else(|| list.iter().copied().find(|&p| self.world.player(p).position != 0));
        // the third loop of R() reads the squad by the list's index, but has the second loop's
        // test, so it never finds anyone the second did not
        if let Some(t) = taker {
            self.world.club_mut(club).penalty_taker = Some(t);
        }
    }

    /// `ah.O(ArrayList)` and `ah.P(ArrayList)`: a captain who is not on the pitch gives way to
    /// the strongest there, the oldest among equals (`ah.nJ`), if he belongs to the club.
    fn fix_captain(&mut self, club: ClubId, on_pitch: &[PlayerId]) {
        if clubs::captain(self.world, club).is_some_and(|c| on_pitch.contains(&c)) {
            return;
        }
        let mut list = on_pitch.to_vec();
        if list.is_empty() {
            return;
        }
        list.sort_by(|&a, &b| {
            let (pa, pb) = (self.world.player(a), self.world.player(b));
            pb.strength.cmp(&pa.strength).then(pb.age.cmp(&pa.age))
        });
        if self.world.player(list[0]).club == Some(club) {
            self.world.club_mut(club).captain = Some(list[0]);
        }
    }

    /// `dV.qa()`: one tick of the screen's timer.
    fn round_tick(&mut self, round: &mut LiveRound) {
        self.runtime.checkpoint(&format!("tick {} {}", round.half, round.minute));
        round.minute += 1;
        let mut finished = false;
        if round.half == 1 {
            if round.minute > 48 {
                self.half_time(round);
            }
        } else if round.half == 2 && round.minute >= 51 {
            finished = true;
        }
        if finished {
            self.round_over(round);
        } else {
            self.round_minute(round);
        }
    }

    /// `dV.qg()`: half time. The human may get the substitutions window, the AI side of a human
    /// club's match makes its changes (`I.m(2, 0)`).
    fn half_time(&mut self, round: &mut LiveRound) {
        round.minute = 0;
        round.half = 2;
        if self.world.game.show_substitutions_window == 1 {
            for i in 0..round.rows.len() {
                if round.rows[i].human {
                    self.substitutions_window(round, i, true);
                }
            }
        }
        for i in 0..round.rows.len() {
            if round.rows[i].human {
                let m = round.rows[i].game_match.expect("a human row is a match");
                self.new_play(m).half_time_changes();
            }
        }
    }

    /// `dV.aa(false)`: the human clubs' matches play the minute, and every row shows the events
    /// it has reached.
    fn round_minute(&mut self, round: &mut LiveRound) {
        for i in 0..round.rows.len() {
            if !round.rows[i].human {
                continue;
            }
            let m = round.rows[i].game_match.expect("a human row is a match");
            let added = self.world.game_match(m).values_fq.get(round.half as usize).copied().unwrap_or(0);
            if added <= round.minute {
                let mut engine = round.engines.remove(&m).expect("I.V() left the engine of a human club's match");
                self.new_play(m).play_minute(&mut engine, round.minute, round.half);
                round.engines.insert(m, engine);
            }
        }
        for i in 0..round.rows.len() {
            let Some(m) = round.rows[i].game_match else { continue };
            let mut e = 0;
            while e < self.world.game_match(m).events.len() {
                let event = &self.world.game_match(m).events[e];
                if !event.done && event.half == round.half && event.minute <= round.minute {
                    self.show_event(round, i, e);
                }
                e += 1;
            }
        }
    }

    /// `dV.a(v, I, A, int)`: an event on screen. A penalty of a human club's match is played
    /// here (`a.gh`, `a.gb`); a red card or an injury of the human's club opens the
    /// substitutions window.
    fn show_event(&mut self, round: &mut LiveRound, row: usize, index: usize) {
        let m = round.rows[row].game_match.expect("an event row is a match");
        let (h, a) = self.sides(m);
        let event = &mut self.world.game_match_mut(m).events[index];
        if event.kind == event_kind::ASSIST {
            event.done = true;
        }
        if event.done {
            return;
        }
        event.done = true;
        let (kind, detail, club) = (event.kind, event.detail, event.club);
        let side = if club == Some(h) {
            1
        } else if club == Some(a) {
            2
        } else {
            0
        };
        if side == 0 {
            return;
        }
        let club = club.expect("an event of a side has its club");
        let human_club = self.world.club(club).human;
        if kind == event_kind::GOAL {
            if detail == PENALTY && round.rows[row].human {
                if human_club && let Some(taker) = self.ask(|d, w| d.penalty_taker(w, m, club)) {
                    self.world.game_match_mut(m).events[index].player = Some(taker);
                }
                self.penalty(round, row, index, side);
                self.world.game_match_mut(m).events[index].flag_dg = true;
            } else {
                round.rows[row].goals[side - 1] += 1;
            }
        } else if kind != event_kind::SECOND_YELLOW && kind != event_kind::RED {
            if kind == event_kind::INJURY && human_club {
                self.substitutions_window(round, row, false);
            }
        } else if human_club {
            self.substitutions_window(round, row, false);
        }
    }

    /// `a.gb`: the penalty of a human club's match, played on the screen. The taker's and the
    /// keeper's traits move the odds from 70 in 100; the draws of the commentary come first.
    fn penalty(&mut self, round: &mut LiveRound, row: usize, index: usize, side: usize) {
        let m = round.rows[row].game_match.expect("a penalty row is a match");
        let other = if side == 1 { 2 } else { 1 };
        // I.aT(int): the keeper on the pitch (place 1) of the other side
        let played = if other == 1 { &self.world.game_match(m).home_played } else { &self.world.game_match(m).away_played };
        let keeper = played.iter().copied().find(|&p| self.world.player(p).slot == 1);
        let taker = self.world.game_match(m).events[index].player.expect("a penalty has its taker");
        // gb.py(), gb.rJ(): what the commentator says
        let _ = self.runtime.rng.fresh_int(6);
        let _ = self.runtime.rng.fresh_int(5);
        // gb.nO()
        let t = self.world.player(taker);
        let mut odds = 70;
        if t.trait1 == 9 || t.trait2 == 9 || t.world_class {
            odds += 10;
        }
        if t.star {
            odds += 5;
        }
        if let Some(k) = keeper {
            let k = self.world.player(k);
            if k.trait1 == 1 || k.trait2 == 1 {
                odds -= 10;
            }
            if k.world_class {
                odds -= 10;
            }
            if k.star {
                odds -= 5;
            }
        }
        let scored = self.runtime.rng.fresh_int(100) + 1 <= odds;
        let i = side - 1;
        if scored {
            let _ = self.runtime.rng.fresh_int(7);
            round.rows[row].goals[i] += 1;
            if let Some(engine) = round.engines.get_mut(&m) {
                engine.add_goal(i);
            }
            let g = self.world.game_match_mut(m);
            if side == 1 {
                g.home_goals += 1;
            } else {
                g.away_goals += 1;
            }
            g.shots[i] += 1;
            g.shots_on_target[i] += 1;
            self.runtime.stats(taker).add_goal();
        } else {
            self.world.game_match_mut(m).shots[i] += 1;
            self.runtime.stats(taker).count_pi += 1;
            let how = self.runtime.rng.fresh_int(7);
            let g = self.world.game_match_mut(m);
            if how != 2 && how != 6 {
                g.shots_on_target[i] += 1;
            } else {
                g.values_ga[i] += 1;
            }
            g.events[index].kind = event_kind::MISSED_PENALTY;
            if how == 0 || how == 3 || how == 5 {
                g.events[index].other_player = keeper;
                let k = keeper.expect("the original saves only with a keeper on the pitch");
                self.runtime.stats(k).count_pl += 1;
            }
        }
    }

    /// `a.cD` opened and closed: the AI side gets its first or second kit (`cD.oS()`), the human
    /// makes his changes, and closing keeps the tactics of its combos (`cD.pc()`), which show
    /// the club's own.
    fn substitutions_window(&mut self, round: &mut LiveRound, row: usize, half_time: bool) {
        let m = round.rows[row].game_match.expect("a substitutions row is a match");
        let (h, a) = self.sides(m);
        if !self.world.club(h).human {
            self.world.club_mut(h).kit = 0;
        }
        if !self.world.club(a).human {
            self.world.club_mut(a).kit = 1;
        }
        self.ask(|d, w| d.substitutions(w, m, half_time));
    }

    /// `dV.qc()`: the end of the group's matches: the scores the screen did not show are
    /// counted again (`qe()`), then the shoot-outs (`qd()`, `g(I)`), then the next group.
    fn round_over(&mut self, round: &mut LiveRound) {
        if round.over {
            return;
        }
        round.over = true;
        // qe(): only the home goals are compared
        for row in &round.rows {
            let Some(m) = row.game_match else { continue };
            if self.world.game_match(m).home_goals != row.goals[0] {
                let (h, a) = self.sides(m);
                let g = self.world.game_match_mut(m);
                let count = |club| g.events.iter().filter(|e| e.kind == event_kind::GOAL && e.club == Some(club)).count() as i32;
                let (home, away) = (count(h), count(a));
                g.home_goals = home;
                g.away_goals = away;
            }
        }
        // qd()
        let mut shootouts = Vec::new();
        for row in &round.rows {
            let Some(m) = row.game_match else { continue };
            if self.world.game_match(m).knockout && self.new_play(m).goes_to_penalties() {
                if !self.human_plays(m) {
                    if self.world.game.show_ai_penalties == 1 {
                        shootouts.push(m);
                    }
                } else {
                    shootouts.push(m);
                }
            }
        }
        for m in shootouts {
            self.shootout(m);
        }
        self.next_group(round);
    }

    /// `dV.g(I)` and `a.bg`: a shoot-out on the screen. Each kick goes in 70 times in 100; five
    /// each, then sudden death.
    fn shootout(&mut self, m: MatchId) {
        let (h, a) = self.sides(m);
        let mut lists = [self.world.game_match(m).home_played.clone(), self.world.game_match(m).away_played.clone()];
        for (i, club) in [h, a].into_iter().enumerate() {
            if self.world.club(club).human {
                let listed = lists[i].clone();
                lists[i] = self.ask(|d, w| d.shootout_order(w, m, club, listed));
            } else {
                let world = &*self.world;
                brasfoot_model::java_sort::sort_by(&mut lists[i], |&x, &y| shootout_rule(world, x, y));
            }
        }
        let mut next = [0usize; 2];
        let mut score = [0i32; 2];
        let mut kicks = [0i32; 3];
        let mut side = 1;
        let winner = loop {
            // bg.nL(): the next kicker, from the top again after the last
            let i = side - 1;
            if next[i] >= lists[i].len() {
                next[i] = 0;
            }
            // bg.nM()
            let scored = self.runtime.rng.fresh_int(100) < 70;
            next[i] += 1;
            if scored {
                score[i] += 1;
            }
            kicks[side] += 1;
            if let Some(w) = shootout_winner(side, scored, score, kicks) {
                break w;
            }
            side = if side == 1 { 2 } else { 1 };
        };
        let g = self.world.game_match_mut(m);
        g.winner = Some(if winner == 1 { h } else { a });
        g.penalty_shootout = score.to_vec();
    }

    /// `I.he()` keeping the engines `I.V()` leaves for the human clubs' matches.
    fn play_today_with_engines(&mut self) -> HashMap<MatchId, MatchEngine> {
        let mut engines = HashMap::new();
        self.play_today_each(|sim, m, engine| {
            if sim.human_plays(m) {
                engines.insert(m, engine);
            }
        });
        engines
    }
}

/// `bg.cP(int)`: after a kick, the side that can no longer be caught (1 home, 2 away).
fn shootout_winner(side: usize, scored: bool, score: [i32; 2], kicks: [i32; 3]) -> Option<usize> {
    if side == 1 && kicks[1] <= 5 {
        if scored && score[0] > 5 - kicks[2] + score[1] {
            return Some(1);
        }
        if 5 - kicks[1] + score[0] < score[1] {
            return Some(2);
        }
    } else if side == 2 && kicks[2] <= 5 {
        if scored && 5 - kicks[1] + score[0] < score[1] {
            return Some(2);
        }
        if score[0] > 5 - kicks[2] + score[1] {
            return Some(1);
        }
    } else if side == 2 && kicks[2] > 5 {
        if score[0] > score[1] {
            return Some(1);
        }
        if score[0] < score[1] {
            return Some(2);
        }
    }
    None
}
