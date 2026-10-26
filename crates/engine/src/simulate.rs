// Port of the AI match of best.I: V() and what it calls (incidents a(I, int, int), events
// a(int, int, I, ah, F, F, int, int), substitutions, the half-time and late changes m(int, int),
// and the shoot-out of a level knockout tie).

use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::runtime::Runtime;
use brasfoot_model::saved::{MatchEvent, StageKind};
use brasfoot_model::world::World;

use crate::engine::MatchEngine;
use crate::hooks::MatchHooks;
use crate::lineup::pick_for_slot;
use crate::stadium;
use crate::tables::INCIDENT_SLOTS;

/// Event kinds (`A.w`).
pub mod event_kind {
    pub const GOAL: i32 = 1;
    pub const YELLOW: i32 = 2;
    pub const SECOND_YELLOW: i32 = 3;
    pub const RED: i32 = 4;
    pub const INJURY: i32 = 5;
    pub const SUBSTITUTION: i32 = 6;
    pub const MISSED_PENALTY: i32 = 7;
    pub const ASSIST: i32 = 8;
}

/// Everything one AI match needs, borrowed for the match.
pub struct Play<'a> {
    pub world: &'a mut World,
    pub runtime: &'a mut Runtime,
    pub hooks: &'a mut dyn MatchHooks,
    pub game_match: MatchId,
}

fn home(world: &World, m: MatchId) -> ClubId {
    world.game_match(m).home.expect("home club")
}

fn away(world: &World, m: MatchId) -> ClubId {
    world.game_match(m).away.expect("away club")
}

impl Play<'_> {
    fn m(&self) -> &brasfoot_model::saved::Match {
        self.world.game_match(self.game_match)
    }

    fn m_mut(&mut self) -> &mut brasfoot_model::saved::Match {
        self.world.game_match_mut(self.game_match)
    }

    /// `I.V()`: plays an AI match from kick-off to the final whistle. The engine comes back
    /// for whoever wants to look at it.
    pub fn play_ai_match(&mut self) -> MatchEngine {
        let mut engine = MatchEngine::new(self.world, self.runtime, self.hooks, self.game_match);
        self.knockout_flags();
        // I.V(): the match's stadium, or the ground made for it (I.il())
        if let Some(stadium) = self.m().stadium {
            let ground = self.world.stadium(stadium).clone();
            stadium::play_gate(self.world, self.runtime, &ground, self.game_match);
        } else if let Some(ground) = self.runtime.venues.get(&self.game_match).cloned() {
            stadium::play_gate(self.world, self.runtime, &ground, self.game_match);
        }
        let (h, a) = (home(self.world, self.game_match), away(self.world, self.game_match));
        if !self.world.club(h).human && !self.world.club(a).human {
            let added_first = self.runtime.rng.fresh_int(3);
            let added_second = self.runtime.rng.fresh_int(5) + 1;
            self.m_mut().values_fq[0] = added_first;
            self.m_mut().values_fq[1] = added_second;
            for minute in 0..45 + added_first {
                self.incidents(1, minute);
                self.engine_tick(&mut engine, 1, minute);
            }
            self.ai_changes(2, 0);
            for minute in 0..45 + added_second {
                self.incidents(2, minute);
                self.engine_tick(&mut engine, 2, minute);
            }
            let m = self.m();
            if m.knockout && m.penalties_if_level && self.level_after_both_legs() {
                let home_kicks = self.runtime.rng.fresh_int(7) + 2;
                let away_kicks = self.runtime.rng.fresh_int(7) + 2;
                let m = self.m_mut();
                if home_kicks >= away_kicks {
                    m.winner = Some(h);
                    m.penalty_shootout = vec![home_kicks, home_kicks - 1];
                } else {
                    m.winner = Some(a);
                    m.penalty_shootout = vec![home_kicks, home_kicks + 1];
                }
            }
        }
        self.world.club_mut(h).lineup_set = false;
        self.world.club_mut(a).lineup_set = false;
        engine
    }

    /// `I.q(int, int)`: one minute of a match the round screen plays (one with a human club),
    /// with the engine `I.V()` left for it (`I.go`).
    pub fn play_minute(&mut self, engine: &mut MatchEngine, minute: i32, half: i32) {
        self.incidents(half, minute);
        self.engine_tick(engine, half, minute);
    }

    /// `cD.K(int, int)`: a substitution the human makes in the window `a.cD`
    /// (`I.a(side, out, in, half, minute, -1)`), whose event the window marks as shown
    /// (`A.p(true)`). Nothing happens without substitutions left.
    pub fn human_substitution(&mut self, side: usize, leaving: PlayerId, coming: PlayerId, half: i32, minute: i32) -> bool {
        if self.m().substitutions_left[side] <= 0 {
            return false;
        }
        self.substitute(side, leaving, coming, half, minute, -1);
        if let Some(event) = self.m_mut().events.last_mut() {
            event.done = true;
        }
        true
    }

    /// `I.m(2, 0)`: the AI side's changes at half time of a match the round screen plays.
    pub fn half_time_changes(&mut self) {
        self.ai_changes(2, 0);
    }

    /// `I.hk()` of a knockout match: level after both legs, so a shoot-out decides.
    pub fn goes_to_penalties(&self) -> bool {
        self.level_after_both_legs()
    }

    /// One engine tick and the goal it may bring, with its assist (`A(int)` of kind 8).
    fn engine_tick(&mut self, engine: &mut MatchEngine, half: i32, minute: i32) {
        if let Some(mut goal) = engine.tick(self.world, self.runtime, self.hooks) {
            goal.minute = minute;
            goal.half = half;
            let assist = goal.other_player;
            let side = goal.value_df;
            self.m_mut().events.push(goal);
            if let Some(helper) = assist {
                let event = MatchEvent { value_df: side, minute, player: Some(helper), kind: event_kind::ASSIST, half, ..Default::default() };
                self.m_mut().events.push(event);
            }
        }
    }

    /// The knockout part of `I.V()`: aggregate of a second leg and whether a shoot-out decides.
    fn knockout_flags(&mut self) {
        let Some(stage) = self.m().stage else { return };
        if self.world.stage(stage).class != StageKind::Knockout {
            return;
        }
        let (two_legged, leg) = {
            let s = self.world.stage(stage);
            let tie = &s.rounds[s.round as usize];
            (tie.two_legged, s.leg)
        };
        if two_legged {
            if leg == 2 {
                let first_leg = self.first_leg_score(stage);
                let m = self.m_mut();
                m.value_gg = first_leg[1];
                m.value_gh = first_leg[0];
                m.went_to_penalties = true;
                m.knockout = true;
            }
        } else {
            self.m_mut().knockout = true;
        }
        if self.m().knockout {
            let state_tiebreak = self.state_tiebreak(stage);
            self.m_mut().penalties_if_level = !state_tiebreak;
        }
    }

    /// `f.G.o(I)`: the first leg's score of this second leg, {away goals, home goals}.
    fn first_leg_score(&self, stage: brasfoot_model::ids::StageId) -> [i32; 2] {
        let s = self.world.stage(stage);
        let tie = &s.rounds[s.round as usize];
        for (i, &second) in tie.second_leg.iter().enumerate() {
            if second == self.game_match {
                let first = self.world.game_match(tie.first_leg[i]);
                return [first.away_goals, first.home_goals];
            }
        }
        [-1, -1]
    }

    /// `f.z.zA()`: a knockout after a state group stage that breaks ties without penalties.
    fn state_tiebreak(&self, stage: brasfoot_model::ids::StageId) -> bool {
        let s = self.world.stage(stage);
        s.competition_kind == 3
            && s.group_stage.is_some_and(|g| {
                let group = self.world.stage(g);
                group.class == StageKind::League && group.state_tiebreak == 1
            })
    }

    /// `I.hk()`: level after both legs (away goals counting where the tie says so); format
    /// 2129 never goes to penalties.
    fn level_after_both_legs(&self) -> bool {
        let stage = self.m().stage.expect("a knockout match");
        let s = self.world.stage(stage);
        let tie = &s.rounds[s.round as usize];
        let away_goals_rule = tie.away_goals;
        let format = tie.format_code;
        let m = self.m();
        let (home_goals, away_goals) = (m.home_goals, m.away_goals);
        let (mut home_wins, mut away_wins) = (0, 0);
        let (mut home_total, mut away_total) = (home_goals, away_goals);
        if home_goals > away_goals {
            home_wins += 1;
        } else if away_goals > home_goals {
            away_wins += 1;
        }
        let mut first_home = 0;
        if m.went_to_penalties {
            let first_away = m.value_gg;
            first_home = m.value_gh;
            if first_home > first_away {
                home_wins += 1;
            } else if first_away > first_home {
                away_wins += 1;
            }
            home_total = home_goals + first_home;
            away_total = away_goals + first_away;
        }
        let mut winner = 0;
        if home_wins > away_wins {
            winner = 1;
        } else if away_wins > home_wins {
            winner = 2;
        }
        if winner == 0 {
            if home_total > away_total {
                winner = 1;
            } else if away_total > home_total {
                winner = 2;
            }
        }
        if winner == 0 && m.went_to_penalties && away_goals_rule {
            if first_home > away_goals {
                winner = 1;
            } else if away_goals > first_home {
                winner = 2;
            }
        }
        if winner == 0 && format == 2129 { false } else { winner == 0 }
    }

    /// `I.a(I, int, int)`: cards, injuries and AI substitutions of one minute.
    fn incidents(&mut self, half: i32, minute: i32) {
        const YELLOW_FIRST: [i32; 3] = [70, 40, 30];
        const YELLOW_SECOND: [i32; 3] = [45, 40, 30];
        const RED_FIRST: [i32; 3] = [1200, 900, 800];
        const RED_SECOND: [i32; 3] = [800, 700, 550];
        const INJURY_FIRST: [i32; 3] = [1500, 1000, 800];
        const INJURY_SECOND: [i32; 3] = [800, 600, 600];
        let (h, a) = (home(self.world, self.game_match), away(self.world, self.game_match));
        let (list, mut marking, side) = if self.runtime.rng.fresh_int(100) > 55 {
            (self.m().home_played.clone(), self.world.club(h).tactics.get(2).copied().unwrap_or(0), 0)
        } else {
            (self.m().away_played.clone(), self.world.club(a).tactics.get(2).copied().unwrap_or(0), 1)
        };
        let marking_bonus = [30, 10, 0];
        if marking >= 3 {
            marking = 0;
        }
        let period = if minute < 15 {
            0
        } else if minute < 30 {
            1
        } else {
            2
        };
        if minute % 7 == 0 {
            self.tire_players(half);
        }
        let (mut yellow, red, injury) = if half == 1 {
            (YELLOW_FIRST[period] + marking_bonus[marking as usize], RED_FIRST[period], INJURY_FIRST[period])
        } else {
            (YELLOW_SECOND[period] + marking_bonus[marking as usize], RED_SECOND[period], INJURY_SECOND[period])
        };
        let m = self.m();
        // the original's `> 10` branch comes after `> 5` and never runs
        if m.value_fs > 5 {
            yellow *= 2;
        }
        if m.value_ft >= 2 {
            yellow = red * 2;
        }
        if m.value_fu >= 1 {
            yellow = injury * 5;
        }
        if self.runtime.rng.fresh_int(yellow) == 1 {
            let player = self.pick_booked(&list);
            self.m_mut().value_fs += 1;
            if let Some(p) = player {
                self.yellow_card(side, p, half, minute);
            }
        } else if self.runtime.rng.fresh_int(red) == 1 {
            if let Some(p) = self.pick_sent_off(&list) {
                self.red_card(side, p, half, minute);
            }
            self.m_mut().value_ft += 1;
        } else if self.runtime.rng.fresh_int(injury) == 1 {
            self.m_mut().value_fu += 1;
            let club = if side == 1 { a } else { h };
            if let Some(p) = self.pick_injured(&list) {
                self.event(event_kind::INJURY, -1, club, Some(p), None, half, minute);
            }
        } else if half == 2 && minute >= 5 {
            self.ai_changes(half, minute);
        }
    }

    /// `I.b(int, I, F, int, int)`: a yellow card, red at the second.
    fn yellow_card(&mut self, side: usize, player: PlayerId, half: i32, minute: i32) {
        *self.runtime.match_cards.entry(player).or_insert(0) += 1;
        let club = if side == 1 { away(self.world, self.game_match) } else { home(self.world, self.game_match) };
        let kind = if self.runtime.match_cards[&player] == 2 { event_kind::SECOND_YELLOW } else { event_kind::YELLOW };
        self.event(kind, -1, club, Some(player), None, half, minute);
    }

    /// `I.a(int, I, F, int, int)`: a straight red.
    fn red_card(&mut self, side: usize, player: PlayerId, half: i32, minute: i32) {
        let club = if side == 1 { away(self.world, self.game_match) } else { home(self.world, self.game_match) };
        self.event(event_kind::RED, -1, club, Some(player), None, half, minute);
    }

    /// `I.a(int, int, I, ah, F, F, int, int)`: records an event and its consequences.
    #[allow(clippy::too_many_arguments)]
    fn event(&mut self, kind: i32, detail: i32, club: ClubId, player: Option<PlayerId>, other: Option<PlayerId>, half: i32, minute: i32) -> MatchEvent {
        let (h, a) = (home(self.world, self.game_match), away(self.world, self.game_match));
        let side = if club == h {
            Some(0)
        } else if club == a {
            Some(1)
        } else {
            None
        };
        let mut event = MatchEvent { value_df: side.unwrap_or(0) as i32, kind, player, ..Default::default() };
        if detail == 2 {
            let others = match side {
                Some(0) => self.m().away_played.clone(),
                Some(_) => self.m().home_played.clone(),
                None => Vec::new(),
            };
            match self.pick_own_goal_scorer(&others) {
                Some(p) => {
                    event.player = Some(p);
                    event.detail = 2;
                }
                None => event.detail = 1,
            }
        } else {
            event.detail = detail;
        }
        if other.is_some() {
            event.other_player = other;
        }
        event.minute = minute;
        event.half = half;
        event.club = Some(club);
        self.m_mut().events.push(event.clone());
        if let Some(p) = player {
            match kind {
                event_kind::YELLOW => self.runtime.stats(p).yellow_cards += 1,
                event_kind::RED => self.runtime.stats(p).red_cards += 1,
                event_kind::SECOND_YELLOW => {
                    self.runtime.stats(p).yellow_cards += 1;
                    self.runtime.stats(p).red_cards += 1;
                }
                _ => {}
            }
        }
        let human = self.world.club(club).human;
        if kind != event_kind::SECOND_YELLOW && kind != event_kind::RED {
            if kind == event_kind::INJURY {
                if let Some(p) = player {
                    crate::injury::injure(self.world, self.runtime, p, club);
                }
                if let Some(s) = side
                    && !human
                {
                    self.on_pitch_mut(s).retain_first(player);
                }
                if let (Some(s), Some(p)) = (side, player)
                    && self.m().substitutions_left[s] > 0
                    && !human
                {
                    self.replace(s, false, p, half, minute, true);
                }
            }
        } else if let Some(s) = side {
            self.on_pitch_mut(s).retain_first(player);
            if let Some(p) = player
                && self.world.player(p).slot <= 13
                && !human
                && self.m().substitutions_left[s] > 0
            {
                self.replace(s, true, p, half, minute, false);
            }
        }
        event
    }

    fn on_pitch_mut(&mut self, side: usize) -> &mut Vec<PlayerId> {
        let m = self.m_mut();
        if side == 0 { &mut m.home_played } else { &mut m.away_played }
    }

    /// `I.a(int, boolean, I, F, int, int, boolean)`: who comes on for a player sent off
    /// (`red_card`: a forward makes way for a defender) or injured.
    fn replace(&mut self, side: usize, red_card: bool, player: PlayerId, half: i32, minute: i32, injury: bool) {
        let (bench, on_pitch) = {
            let m = self.m();
            if side == 0 { (m.home_bench.clone(), m.home_played.clone()) } else { (m.away_bench.clone(), m.away_played.clone()) }
        };
        let leaving = if red_card {
            let mut l = self.random_in_slots(&on_pitch, 18, 25);
            if l.is_none() {
                l = self.random_in_slots(&on_pitch, 14, 17);
            }
            if l.is_none() && self.world.player(player).slot == 1 {
                l = self.random_in_slots(&on_pitch, 2, 25);
            }
            l
        } else {
            Some(player)
        };
        let Some(leaving) = leaving else { return };
        let slot = self.world.player(player).slot;
        if slot < 0 {
            return;
        }
        let keep_keeper = self.world.player(player).position != 0;
        let Some(coming) = pick_for_slot(self.world, &bench, slot, false, keep_keeper) else { return };
        if injury {
            if self.world.player(leaving).position == 0 || self.world.player(coming).position != 0 {
                self.substitute(side, leaving, coming, half, minute, slot);
            }
        } else {
            self.substitute(side, leaving, coming, half, minute, slot);
        }
    }

    /// `I.a(int, F, F, int, int, int)`: a substitution.
    fn substitute(&mut self, side: usize, leaving: PlayerId, coming: PlayerId, half: i32, minute: i32, slot: i32) {
        let club = if side == 0 { home(self.world, self.game_match) } else { away(self.world, self.game_match) };
        self.m_mut().substitutions_left[side] -= 1;
        let leaving_slot = self.world.player(leaving).slot;
        let p = self.world.player_mut(coming);
        p.slot = leaving_slot;
        if slot > 0 {
            p.slot = slot;
        }
        p.started_recently = true;
        let m = self.m_mut();
        let (pitch, bench, came_on) = if side == 0 {
            (&mut m.home_played, &mut m.home_bench, &mut m.home_list_fl)
        } else {
            (&mut m.away_played, &mut m.away_bench, &mut m.away_list_fm)
        };
        pitch.retain_first(Some(leaving));
        pitch.push(coming);
        came_on.push(coming);
        bench.retain_first(Some(coming));
        self.event(event_kind::SUBSTITUTION, -1, club, Some(leaving), Some(coming), half, minute);
    }

    /// `I.l(int, int)`: every seven minutes the outfield players tire, the keepers only after
    /// the break.
    fn tire_players(&mut self, half: i32) {
        let m = self.m();
        let players: Vec<PlayerId> = m.home_played.iter().chain(&m.away_played).copied().collect();
        for p in players {
            let player = self.world.player_mut(p);
            if player.slot != 1 || half == 2 {
                player.tire_after_match();
            }
        }
    }

    /// `I.n(int, int)`: side 1 (home) or 2 (away) is behind by at least `by`.
    fn behind(&self, side: i32, by: i32) -> bool {
        let m = self.m();
        if side == 1 { m.away_goals - m.home_goals >= by } else { m.home_goals - m.away_goals >= by }
    }

    /// `I.m(int, int)`: the AI's planned and score-driven substitutions.
    fn ai_changes(&mut self, half: i32, minute: i32) {
        if half != 2 {
            return;
        }
        let (h, a) = (home(self.world, self.game_match), away(self.world, self.game_match));
        let mut done = false;
        let m = self.m();
        let (planned, timed) = (m.tactical_change_minutes.clone(), m.substitution_minutes.clone());
        if !self.world.club(h).human && self.m().substitutions_left[0] > 0 {
            let pitch = self.m().home_played.clone();
            if minute == 0 && self.behind(1, 1) {
                if self.runtime.rng.fresh_int(100) > 50 {
                    done = self.change(2, 0, half, minute, &pitch);
                }
            } else if !planned[0][..3].contains(&minute) {
                if timed[0][..4].contains(&minute) {
                    done = self.change(1, 0, half, minute, &pitch);
                }
            } else if self.behind(1, 1) || self.m().home_goals == self.m().away_goals {
                done = self.change(2, 0, half, minute, &pitch);
            }
        }
        if !done && !self.world.club(a).human && self.m().substitutions_left[1] > 0 {
            let pitch = self.m().away_played.clone();
            if minute == 0 && self.behind(2, 2) {
                if self.runtime.rng.fresh_int(100) > 50 {
                    self.change(2, 1, half, minute, &pitch);
                }
            } else if !planned[1][..3].contains(&minute) {
                if timed[1][..4].contains(&minute) {
                    self.change(1, 1, half, minute, &pitch);
                }
            } else if self.behind(2, 1) {
                self.change(2, 1, half, minute, &pitch);
            }
        }
    }

    /// `I.a(int, int, int, int, ArrayList)`: mode 1 takes off the first tired outfield player,
    /// mode 2 a random one. Mode 2 checks the home side's list of substitutes for both sides.
    fn change(&mut self, mode: i32, side: usize, half: i32, minute: i32, pitch: &[PlayerId]) -> bool {
        if mode == 1 {
            let mut limit = 60;
            let mut from = 0;
            if minute > 40 {
                limit = 90;
                if !pitch.is_empty() {
                    from = self.runtime.rng.fresh_int(pitch.len() as i32) as usize;
                }
            }
            for &p in &pitch[from.min(pitch.len())..] {
                let player = self.world.player(p);
                if player.slot != 1 && player.fitness < limit {
                    self.replace(side, false, p, half, minute, false);
                    return true;
                }
            }
        } else if mode == 2 {
            let mut random = self.runtime.rng.new_random();
            if !pitch.is_empty() {
                let mut index = random.next_int_bounded(pitch.len() as i32) as usize;
                let came_on = self.m().home_list_fl.clone();
                if came_on.contains(&pitch[index]) {
                    index = random.next_int_bounded(pitch.len() as i32) as usize;
                }
                let p = pitch[index];
                if self.world.player(p).slot != 1 && !came_on.contains(&p) {
                    self.replace(side, false, p, half, minute, false);
                    return true;
                }
            }
        }
        false
    }

    /// `I.a(ArrayList, int, int)`: a random player whose slot is in the range.
    fn random_in_slots(&mut self, list: &[PlayerId], from: i32, to: i32) -> Option<PlayerId> {
        let mut candidates: Vec<PlayerId> =
            list.iter().copied().filter(|&p| (from..=to).contains(&self.world.player(p).slot)).collect();
        self.runtime.rng.shuffle(&mut candidates);
        candidates.first().copied()
    }

    fn by_table(&mut self, list: &[PlayerId], row: usize) -> Option<PlayerId> {
        let [from, to] = INCIDENT_SLOTS[row];
        self.random_in_slots(list, from, to)
    }

    /// `I.G(ArrayList)`: who scores an own goal.
    fn pick_own_goal_scorer(&mut self, list: &[PlayerId]) -> Option<PlayerId> {
        let row = match self.runtime.rng.fresh_int(1000) {
            0 => 6,
            1..=149 => 0,
            150..=349 => 1,
            350..=399 => 2,
            400..=449 => 3,
            450..=499 => 4,
            _ => 5,
        };
        self.by_table(list, row)
    }

    /// `I.H(ArrayList)`: who is booked.
    fn pick_booked(&mut self, list: &[PlayerId]) -> Option<PlayerId> {
        let row = match self.runtime.rng.fresh_int(100) {
            0..=24 => 0,
            25..=39 => 1,
            40..=64 => 2,
            65..=72 => 3,
            73..=81 => 4,
            82..=84 => 6,
            _ => 5,
        };
        self.by_table(list, row)
    }

    /// `I.I(ArrayList)`: who gets injured.
    fn pick_injured(&mut self, list: &[PlayerId]) -> Option<PlayerId> {
        let row = match self.runtime.rng.fresh_int(500) {
            0 => 6,
            1..=149 => 0,
            150..=249 => 1,
            250..=319 => 2,
            320..=359 => 3,
            360..=419 => 4,
            _ => 5,
        };
        self.by_table(list, row)
    }

    /// `I.J(ArrayList)`: who is sent off.
    fn pick_sent_off(&mut self, list: &[PlayerId]) -> Option<PlayerId> {
        let row = match self.runtime.rng.fresh_int(200) {
            0 => 6,
            1..=79 => 0,
            80..=109 => 1,
            110..=159 => 2,
            160..=169 => 3,
            170..=189 => 4,
            _ => 5,
        };
        self.by_table(list, row)
    }
}

/// `ArrayList.remove(Object)`: drops the first occurrence.
trait RetainFirst {
    fn retain_first(&mut self, player: Option<PlayerId>);
}

impl RetainFirst for Vec<PlayerId> {
    fn retain_first(&mut self, player: Option<PlayerId>) {
        if let Some(p) = player
            && let Some(i) = self.iter().position(|&x| x == p)
        {
            self.remove(i);
        }
    }
}

/// Port of `I.hi()`: before a match, both squads' cards of the match are reset and whoever is
/// suspended in its competition serves a match. Looking the suspension up creates the
/// competition's numbers, as `F.c(at)` does.
pub fn prepare_squads(world: &mut World, runtime: &mut Runtime, game_match: MatchId) {
    let competition = world.game_match(game_match).competition;
    for club in [home(world, game_match), away(world, game_match)] {
        for p in world.club(club).squad.clone() {
            runtime.match_cards.insert(p, 0);
            let stats = crate::events::competition_stats(world, p, competition);
            if stats.is_suspended() {
                stats.serve_suspension();
            }
        }
    }
}
