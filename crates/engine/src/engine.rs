// Port of c.b as patched by the mod (mod/patch/c/b.java): the match engine. Each tick one side
// attacks: a midfield duel decides who keeps the ball, then attack against defence decides if a
// shot comes out, then the shot against the keeper decides goal, save or wide.

use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::rng::{JavaRandom, java_round};
use brasfoot_model::runtime::Runtime;
use brasfoot_model::saved::MatchEvent;
use brasfoot_model::world::World;

use crate::clubs;
use crate::events;
use crate::hooks::MatchHooks;
use crate::lineup;
use crate::tables::percent;
use crate::tactics::{self, TacticsMatch};
use crate::value::{engine_value, out_of_position};

/// `c.b.Tg`: weights of the midfield duel (attacker, defender).
const MIDFIELD_BASE: [f64; 2] = [55.0, 45.0];
/// `c.b.Th`: weights of the attack-vs-defence duel.
const ATTACK_BASE: [f64; 2] = [50.0, 50.0];
/// `c.b.TU`..`TX`: goal, saved and wide weights, by goals already scored.
const SHOT_BASE: [f64; 3] = [5.5, 35.55, 15.0];
const SHOT_AFTER_3: [f64; 3] = [4.5, 40.55, 15.0];
const SHOT_AFTER_5: [f64; 3] = [3.0, 40.55, 15.0];
const SHOT_AFTER_6: [f64; 3] = [0.5, 40.55, 15.0];

/// `c.b.Tx`: who scores, weight by slot.
pub const SCORER_WEIGHT: [f64; 26] = [
    -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 8.0, 4.0, 4.0, 4.0, 8.0, 8.0, 8.0, 8.0, 22.0, 22.0, 22.0, 22.0,
    22.0, 22.0, 22.0, 22.0,
];
/// `c.b.Ty`: who assists, weight by slot.
pub const ASSIST_WEIGHT: [f64; 26] = [
    1.0, 1.0, 10.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 10.0, 10.0, 4.0, 4.0, 4.0, 20.0, 20.0, 20.0, 10.0, 10.0, 10.0, 10.0,
    10.0, 10.0, 10.0, 10.0, 10.0,
];
/// `c.b.Tz`: who scores an own goal, weight by slot.
pub const OWN_GOAL_WEIGHT: [f64; 26] = [
    -1.0, 1.0, 5.0, 18.0, 18.0, 18.0, 18.0, 18.0, 18.0, 5.0, 1.0, 5.0, 5.0, 5.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    1.0, 1.0, 1.0, 1.0,
];
/// `c.b.TY`: slots of `vW()`.
const SET_PIECE_SLOTS: [i32; 11] = [1, 22, 24, 11, 13, 14, 16, 2, 9, 3, 5];

/// Goal kinds (`A.dz` of a goal).
pub mod goal_kind {
    pub const NORMAL: i32 = 1;
    pub const OWN_GOAL: i32 = 2;
    pub const PENALTY: i32 = 3;
    pub const FREE_KICK: i32 = 4;
    pub const OLYMPIC: i32 = 5;
}

/// Port of `c.b`: the engine of one match.
#[derive(Debug)]
pub struct MatchEngine {
    pub game_match: MatchId,
    clubs: [ClubId; 2],
    /// `TB`: the side on the ball this tick.
    attacker: usize,
    /// `TC`: ticks played.
    ticks: i32,
    /// `TD` (`I.hX()`): neutral ground, no home advantage.
    neutral: bool,
    /// `TE`: the player picked to shoot.
    shooter: Option<PlayerId>,
    /// `TF`
    last_defence: f64,
    /// `zA`: goals per side as the engine counts them.
    pub goals: [i32; 2],
    /// `TG`: shots per side.
    pub shots: [i32; 2],
    /// `TH`..`TL`: never written by the engine (the match screen reads them).
    pub unused_th: [i32; 2],
    pub unused_ti: [i32; 2],
    pub unused_tj: [i32; 2],
    pub unused_tk: [i32; 2],
    pub unused_tl: [i32; 2],
    /// `TM`: attack-vs-defence duels won by the attacker.
    pub attacks_won: [i32; 2],
    /// `TN`, `TO`: midfield duels won as attacker and as defender.
    pub midfield_won_attacking: [i32; 2],
    pub midfield_won_defending: [i32; 2],
    /// `TP`: attack-vs-defence duels won by the defender.
    pub defences_won: [i32; 2],
    /// `fX`: midfield duels won.
    pub midfield_won: [i32; 2],
    /// `TQ`: midfield duels lost.
    pub midfield_lost: [i32; 2],
    /// `TR`
    pub unused_tr: [i32; 2],
    /// `Tn`: draws of the duels and the player picks.
    main_random: JavaRandom,
    /// `To`: tackle or misplaced pass.
    other_random: JavaRandom,
    /// `Tp` (`forms.Randoms`): created like the original, never drawn from.
    _unused_random: JavaRandom,
    /// The mod's tactics for this match (`Taticas.partidas`).
    pub tactics: Option<TacticsMatch>,
}

fn side_list(world: &World, m: MatchId, side: usize) -> Vec<PlayerId> {
    let m = world.game_match(m);
    if side == 0 { m.home_played.clone() } else { m.away_played.clone() }
}

impl MatchEngine {
    /// `new c.b(I)`.
    pub fn new(world: &mut World, runtime: &mut Runtime, hooks: &mut dyn MatchHooks, match_id: MatchId) -> MatchEngine {
        // field initialisers run before the constructor body: Tn, To, Tp
        let mut main_random = runtime.rng.new_random();
        let other_random = runtime.rng.new_random();
        let unused_random = runtime.rng.new_random();
        let m = world.game_match(match_id);
        let clubs = [m.home.expect("a match has a home club"), m.away.expect("a match has an away club")];
        // `vN()`
        let attacker = if main_random.next_boolean() { 0 } else { 1 };
        let neutral = is_neutral(world, match_id);
        if world.game_match(match_id).home_played.is_empty() {
            lineup::pick_ai_lineup(world, runtime, clubs[0], Some(match_id), 1, -1, true);
        }
        if world.game_match(match_id).away_played.is_empty() {
            // the original passes side 1 here too, so the away XI lands in the home lists
            lineup::pick_ai_lineup(world, runtime, clubs[1], Some(match_id), 1, -1, true);
        }
        let m = world.game_match(match_id);
        let everyone: Vec<PlayerId> =
            m.home_starters.iter().chain(&m.away_starters).chain(&m.home_bench).chain(&m.away_bench).copied().collect();
        for p in everyone {
            runtime.reset_stats(p);
        }
        let tactics = tactics::kickoff(world, runtime, hooks, match_id);
        MatchEngine {
            game_match: match_id,
            clubs,
            attacker,
            ticks: 0,
            neutral,
            shooter: None,
            last_defence: 0.0,
            goals: [0; 2],
            shots: [0; 2],
            unused_th: [0; 2],
            unused_ti: [0; 2],
            unused_tj: [0; 2],
            unused_tk: [0; 2],
            unused_tl: [0; 2],
            attacks_won: [0; 2],
            midfield_won_attacking: [0; 2],
            midfield_won_defending: [0; 2],
            defences_won: [0; 2],
            midfield_won: [0; 2],
            midfield_lost: [0; 2],
            unused_tr: [0; 2],
            main_random,
            other_random,
            _unused_random: unused_random,
            tactics,
        }
    }

    /// `vQ()`: the side without the ball.
    fn defender(&self) -> usize {
        1 - self.attacker
    }

    /// `vO()`: one tick. Returns the goal it produced, if any.
    pub fn tick(&mut self, world: &mut World, runtime: &mut Runtime, hooks: &mut dyn MatchHooks) -> Option<MatchEvent> {
        self.ticks += 1;
        // Taticas keeps the match's own lists, so it sees every substitution and sending-off
        if let Some(p) = &mut self.tactics {
            let m = world.game_match(self.game_match);
            p.lineups[0].players = m.home_played.clone();
            p.lineups[1].players = m.away_played.clone();
        }
        tactics::tick(world, runtime, hooks, self.game_match, self.tactics.as_mut(), self.ticks, self.goals);
        let mut goal = None;
        let winner = self.midfield_duel(world, hooks);
        let attacker = self.attacker;
        let defender = self.defender();
        if winner == attacker {
            if self.attack_duel(world, hooks) == 0 {
                world.game_match_mut(self.game_match).shots[attacker] += 1;
                goal = self.shot(world, runtime);
            } else if self.other_random.next_int_bounded(100) < 50 {
                world.game_match_mut(self.game_match).tackles[defender] += 1;
            } else {
                world.game_match_mut(self.game_match).values_gc[attacker] += 1;
            }
        } else if self.other_random.next_int_bounded(100) < 50 {
            world.game_match_mut(self.game_match).tackles[defender] += 1;
        } else {
            world.game_match_mut(self.game_match).values_gc[attacker] += 1;
        }
        // `vP()`
        self.attacker = defender;
        goal
    }

    /// `ez(int)`: midfield strength, the first five in slots 10 to 17, plus the marking bonus.
    fn midfield(&self, world: &World, side: usize) -> f64 {
        let marking = world.club(self.clubs[side]).tactics.get(2).copied().unwrap_or(0);
        let bonus = [0.0, 0.04, 0.08];
        let mut sum = bonus[if marking as usize >= bonus.len() { 2 } else { marking.max(0) as usize }];
        let mut count = 0;
        for p in side_list(world, self.game_match, side) {
            let slot = world.player(p).slot;
            if count < 5 && (10..=17).contains(&slot) {
                sum += engine_value(world, Some(self.game_match), p);
                count += 1;
            }
        }
        if count < 3 { 0.01 } else { sum / 5.0 }
    }

    /// `eA(int)`: attack, the first three in slots 19 to 25.
    fn attack(&self, world: &World, side: usize) -> f64 {
        let (mut sum, mut count) = (0.0, 0);
        for p in side_list(world, self.game_match, side) {
            let slot = world.player(p).slot;
            if count < 3 && (19..=25).contains(&slot) {
                sum += engine_value(world, Some(self.game_match), p);
                count += 1;
            }
        }
        if count < 1 { 0.0 } else { sum / 3.0 }
    }

    /// `eB(int)`: the keeper (slot 1); a fifth of it out of position.
    fn keeper(&self, world: &World, side: usize) -> f64 {
        for p in side_list(world, self.game_match, side) {
            if world.player(p).slot == 1 {
                let value = engine_value(world, Some(self.game_match), p);
                return if out_of_position(world.player(p)) { java_round(value * 0.2) as i32 as f64 } else { value };
            }
        }
        0.1
    }

    /// `eC(int)`: picks the shooter.
    fn shooter_value(&mut self, world: &World) -> f64 {
        self.shooter = self.pick_scorer(world);
        self.shooter.map(|p| engine_value(world, Some(self.game_match), p)).unwrap_or(0.1)
    }

    /// `eD(int)`: centre-backs (slots 3 to 8).
    fn centre_backs(&self, world: &World, side: usize) -> i32 {
        side_list(world, self.game_match, side).into_iter().filter(|&p| (3..=8).contains(&world.player(p).slot)).count() as i32
    }

    /// `eE(int)`: defence, the first five in slots 2 to 9.
    fn defence(&self, world: &World, side: usize) -> f64 {
        let (mut sum, mut count) = (0.0, 0);
        for p in side_list(world, self.game_match, side) {
            let slot = world.player(p).slot;
            if count < 5 && (2..=9).contains(&slot) {
                sum += engine_value(world, Some(self.game_match), p);
                count += 1;
            }
        }
        if count < 3 { 0.01 } else { sum / 5.0 }
    }

    /// `a(double, double)`: a strength gap, divided by 8 (11 from the fifth season on).
    fn gap(&self, world: &World, a: f64, b: f64) -> f64 {
        let divisor = if world.season() >= 5 { 11.0 } else { 8.0 };
        (a - b) / divisor
    }

    /// `b(double, double)`: the same with 8 (10 from the fifth season on).
    fn shot_gap(&self, world: &World, a: f64, b: f64) -> f64 {
        let divisor = if world.season() >= 5 { 10.0 } else { 8.0 };
        (a - b) / divisor
    }

    fn tactics_weight(&self, world: &World, side: usize, f: fn(&World, &TacticsMatch, usize) -> f64) -> f64 {
        self.tactics.as_ref().map(|p| f(world, p, side)).unwrap_or(1.0)
    }

    /// `eF(int)`: a midfield duel won; the possession shares follow.
    fn count_possession(&self, world: &mut World, side: usize) {
        let m = world.game_match_mut(self.game_match);
        m.values_fx[side] += 1;
        let total = m.values_fx[0] + m.values_fx[1];
        m.possession[0] = percent(m.values_fx[0], total);
        m.possession[1] = percent(m.values_fx[1], total);
    }

    /// `vR()`: the midfield duel. Returns the side that keeps the ball.
    fn midfield_duel(&mut self, world: &mut World, hooks: &dyn MatchHooks) -> usize {
        let (a, d) = (self.attacker, self.defender());
        let mid_a = self.midfield(world, a);
        let mid_d = self.midfield(world, d);
        let mut wa = 1.0 + self.gap(world, mid_a, mid_d);
        let mut wd = 1.0 + self.gap(world, mid_d, mid_a);
        if !self.neutral && a == 0 {
            wa += 0.3;
        }
        wa *= self.tactics_weight(world, a, tactics::midfield_weight) * hooks.match_bonus(world, self.game_match, a);
        wd *= self.tactics_weight(world, d, tactics::midfield_weight) * hooks.match_bonus(world, self.game_match, d);
        let wa = wa.max(0.2);
        let wd = wd.max(0.2);
        let winner = self.pick(&MIDFIELD_BASE, &[wa, wd]);
        if winner == 0 {
            self.midfield_lost[d] += 1;
            self.midfield_won[a] += 1;
            self.midfield_won_attacking[a] += 1;
            self.count_possession(world, a);
            a
        } else if winner == 1 {
            self.midfield_lost[a] += 1;
            self.midfield_won[d] += 1;
            self.midfield_won_defending[d] += 1;
            self.count_possession(world, d);
            d
        } else {
            0
        }
    }

    /// `vS()`: attack against defence. 0 when a shot comes out.
    fn attack_duel(&mut self, world: &World, hooks: &dyn MatchHooks) -> usize {
        let (a, d) = (self.attacker, self.defender());
        let mut defence = self.defence(world, d);
        self.last_defence = defence;
        let mut attack = self.attack(world, a);
        if let Some(p) = &self.tactics
            && attack > 0.0
            && defence > 0.0
        {
            let dev = tactics::deviation(world, p, a, p.wings(world, a) == 1);
            attack += dev / 2.0;
            defence -= dev / 2.0;
        }
        let mut wa = 1.0 + self.gap(world, attack, defence);
        let mut wd = 1.0 + self.gap(world, defence, attack);
        if defence == 0.0 {
            wd = 0.1;
        }
        if !self.neutral && a == 0 {
            wa += 0.3;
        }
        wa *= self.tactics_weight(world, a, tactics::shoot_weight) * hooks.match_bonus(world, self.game_match, a);
        wd *= self.tactics_weight(world, d, tactics::block_weight);
        if attack == 0.0 {
            wa = 0.1;
        }
        let wa = wa.max(0.2);
        let mut wd = wd.max(0.2);
        if world.club(self.clubs[0]).human || world.club(self.clubs[1]).human {
            match self.centre_backs(world, d) {
                0 => wd = 0.1,
                1 => wd = 0.05,
                _ => {}
            }
        }
        let result = self.pick(&ATTACK_BASE, &[wa, wd]);
        if result == 0 {
            self.attacks_won[a] += 1;
        } else if result == 1 {
            self.defences_won[d] += 1;
        }
        result
    }

    /// `vT()`: the shot. A goal comes back as its event.
    fn shot(&mut self, world: &mut World, runtime: &mut Runtime) -> Option<MatchEvent> {
        let (a, d) = (self.attacker, self.defender());
        let defence = self.defence(world, d);
        self.last_defence = defence;
        let attack = self.attack(world, a);
        let shooter = self.shooter_value(world);
        let keeper = self.keeper(world, d);
        let mut save = 1.0 + self.shot_gap(world, keeper, shooter);
        save *= self.tactics_weight(world, a, tactics::keeper_weight);
        let mut wide = 1.0 + self.shot_gap(world, defence, attack);
        if world.club(self.clubs[0]).human || world.club(self.clubs[1]).human {
            match self.centre_backs(world, d) {
                0 => save = java_round(save * 0.2) as i32 as f64,
                1 => save = java_round(save * 0.4) as i32 as f64,
                _ => {}
            }
        }
        if !self.neutral {
            // the original overwrites the wide weight with the save weight here
            if a == 0 {
                save += 0.1;
                wide = save + 0.1;
            }
            if a == 1 {
                save -= 0.1;
                wide = save - 0.1;
            }
        }
        let mut base = SHOT_BASE;
        if self.goals[a] >= 6 {
            base = SHOT_AFTER_6;
        } else if self.goals[a] >= 5 {
            base = SHOT_AFTER_5;
        } else if self.goals[a] >= 3 {
            base = SHOT_AFTER_3;
        }
        if self.goals[a] >= 2 && world.club(self.clubs[d]).reputation - world.club(self.clubs[a]).reputation >= 2 {
            base = SHOT_AFTER_5;
        }
        let save = save.max(0.2);
        let wide = wide.max(0.2);
        let outcome = self.pick(&base, &[1.0, save, wide]);
        self.shots[a] += 1;
        match outcome {
            0 => {
                let mut event = MatchEvent { value_df: a as i32, ..Default::default() };
                event.club = Some(self.clubs[a]);
                let shooter = self.shooter;
                self.score(world, runtime, &mut event, shooter);
                world.game_match_mut(self.game_match).shots_on_target[a] += 1;
                Some(event)
            }
            1 => {
                world.game_match_mut(self.game_match).shots_on_target[a] += 1;
                if let Some(p) = self.shooter {
                    runtime.stats(p).shots_saved += 1;
                }
                None
            }
            2 => {
                world.game_match_mut(self.game_match).values_ga[a] += 1;
                None
            }
            _ => None,
        }
    }

    /// `a(double[], double[])` and `b(double[], double[])`: a weighted draw.
    fn pick(&mut self, base: &[f64], weights: &[f64]) -> usize {
        let products: Vec<f64> = base.iter().zip(weights).map(|(b, w)| b * w).collect();
        let total: f64 = products.iter().sum();
        let r = self.main_random.next_double() * total;
        let mut acc = 0.0;
        for (i, p) in products.iter().enumerate() {
            acc += p;
            if r < acc {
                return i;
            }
        }
        base.len()
    }

    /// `vU()`: who shoots, by slot and traits (9 finisher, 5 header).
    pub fn pick_scorer(&mut self, world: &World) -> Option<PlayerId> {
        let list = side_list(world, self.game_match, self.attacker);
        let weight = |p: PlayerId| {
            let player = world.player(p);
            let slot = player.slot;
            if slot != 1 && player.position != 0 && slot > 0 && (slot as usize) < SCORER_WEIGHT.len() {
                let mut w = SCORER_WEIGHT[slot as usize];
                if player.trait1 == 9 || player.trait2 == 9 {
                    w += 4.0;
                } else if player.trait1 == 5 || player.trait2 == 5 {
                    w += 2.0;
                    if player.position == 2 {
                        w += 2.0;
                    }
                }
                w
            } else {
                0.0
            }
        };
        let total: f64 = list.iter().map(|&p| weight(p)).sum();
        let r = self.main_random.next_double() * total;
        let mut acc = 0.0;
        for &p in &list {
            acc += weight(p);
            if r <= acc {
                return Some(p);
            }
        }
        // the original walks back to index 1, never 0
        (1..list.len()).rev().map(|i| list[i]).next()
    }

    /// `C(F)`: who assists the scorer; none 19% of the time.
    fn pick_assistant(&mut self, world: &World, runtime: &mut Runtime, scorer: Option<PlayerId>) -> Option<PlayerId> {
        if runtime.rng.fresh_int(100) > 80 {
            return None;
        }
        let list = side_list(world, self.game_match, self.attacker);
        let marking = world.club(self.clubs[self.attacker]).tactics.get(2).copied().unwrap_or(0);
        // the two passes of the original differ: trait 13 alone weighs 1 in the total, 2 in the walk
        let weight = |p: PlayerId, walk: bool| {
            let player = world.player(p);
            let slot = player.slot;
            if Some(p) == scorer || slot <= 0 || slot as usize >= ASSIST_WEIGHT.len() {
                return 0.0;
            }
            let (c1, c2) = (player.trait1, player.trait2);
            let mut w = ASSIST_WEIGHT[slot as usize];
            if c1 == 11 || c2 == 11 {
                w += 10.0;
                if c1 == 4 || c2 == 4 {
                    w += 5.0;
                }
            } else if c1 == 4 || c2 == 4 {
                w += 2.0;
                if c1 == 8 {
                    w += 2.0;
                }
            } else if c1 == 8 || c2 == 8 {
                w += 2.0;
                if c1 == 13 {
                    w += 2.0;
                }
            } else if c1 != 13 && c2 != 13 {
                if c1 == 6 || c2 == 6 {
                    w += 5.0;
                    if player.position == 1 {
                        w += 2.0;
                    }
                }
            } else {
                w += if walk { 2.0 } else { 1.0 };
                if player.position == 1 {
                    w += 2.0;
                }
            }
            if marking == 1 && player.position == 1 {
                w += 20.0;
            }
            w
        };
        let total: f64 = list.iter().map(|&p| weight(p, false)).sum();
        let r = self.main_random.next_double() * total;
        let mut acc = 0.0;
        for &p in &list {
            acc += weight(p, true);
            if r <= acc {
                return Some(p);
            }
        }
        None
    }

    /// `vV()`: who scores an own goal, among the defending side.
    fn pick_own_goal(&mut self, world: &World) -> Option<PlayerId> {
        let list = side_list(world, self.game_match, self.defender());
        let weight = |p: PlayerId| {
            let slot = world.player(p).slot;
            if slot >= 0 && (slot as usize) < OWN_GOAL_WEIGHT.len() { OWN_GOAL_WEIGHT[slot as usize] } else { 0.0 }
        };
        let total: f64 = list.iter().map(|&p| weight(p)).sum();
        let r = self.main_random.next_double() * total;
        let mut acc = 0.0;
        for &p in &list {
            acc += weight(p);
            if r <= acc {
                return Some(p);
            }
        }
        None
    }

    /// `vW()`: a slot for a set piece.
    pub fn pick_set_piece_slot(&mut self) -> i32 {
        let total: f64 = SET_PIECE_SLOTS.iter().filter(|&&s| s != 1).map(|&s| SCORER_WEIGHT[s as usize]).sum();
        let r = self.main_random.next_double() * total;
        let mut acc = 0.0;
        for &s in &SET_PIECE_SLOTS {
            if s != 1 {
                acc += SCORER_WEIGHT[s as usize];
            }
            if r <= acc {
                return s;
            }
        }
        -1
    }

    /// `a(A, F)`: a goal. Fills the event and counts it.
    fn score(&mut self, world: &mut World, runtime: &mut Runtime, event: &mut MatchEvent, scorer: Option<PlayerId>) {
        use goal_kind::*;
        event.kind = 1;
        event.player = scorer;
        let draw = runtime.rng.fresh_int(1000);
        let mut kind = match draw {
            0..=899 => NORMAL,
            900..=949 => PENALTY,
            950..=979 => FREE_KICK,
            980..=989 => OWN_GOAL,
            990..=994 => OLYMPIC,
            _ => NORMAL,
        };
        let mut scorer = scorer;
        let a = self.attacker;
        if scorer.is_none() {
            if side_list(world, self.game_match, a).is_empty() {
                lineup::pick_ai_lineup(world, runtime, self.clubs[a], Some(self.game_match), a as i32 + 1, -1, true);
            }
            scorer = self.pick_scorer(world);
        }
        if let Some(p) = scorer
            && kind != PENALTY
            && kind != OWN_GOAL
        {
            runtime.stats(p).add_goal();
        }
        if kind == NORMAL {
            let assistant = self.pick_assistant(world, runtime, scorer);
            if let Some(helper) = assistant
                && Some(helper) != scorer
            {
                runtime.stats(helper).assists += 1;
                let player = world.player(helper);
                if player.club.is_some() && !player.youth {
                    let competition = world.game_match(self.game_match).competition;
                    events::record_assist(world, helper, event.club, competition);
                }
                event.other_player = Some(helper);
            }
        }
        if scorer.is_some() && kind == OLYMPIC {
            let list = side_list(world, self.game_match, a);
            let taker = clubs::free_kick_taker(world, self.clubs[a]);
            if let Some(t) = taker.filter(|t| list.contains(t)) {
                event.player = Some(t);
            } else if world.player(scorer.unwrap()).position == 0 {
                kind = NORMAL;
            }
        }
        if kind == OWN_GOAL {
            match self.pick_own_goal(world) {
                Some(p) => {
                    event.player = Some(p);
                    runtime.stats(p).own_goals += 1;
                }
                None => {
                    kind = NORMAL;
                    if let Some(p) = scorer {
                        runtime.stats(p).add_goal();
                    }
                }
            }
        }
        if kind == PENALTY || kind == FREE_KICK {
            let list = side_list(world, self.game_match, a);
            if let Some(t) = clubs::penalty_taker(world, self.clubs[a]).filter(|t| list.contains(t)) {
                event.player = Some(t);
            }
        }
        let mut left_to_the_screen = false;
        if kind == PENALTY {
            event.flag_dg = true;
            if world.club(self.clubs[0]).human || world.club(self.clubs[1]).human {
                // the match screen plays the penalty
                left_to_the_screen = true;
                event.flag_dg = false;
            }
        }
        if !left_to_the_screen {
            self.goals[a] += 1;
            let m = world.game_match_mut(self.game_match);
            if a == 0 {
                m.home_goals += 1;
            } else {
                m.away_goals += 1;
            }
            if let Some(p) = scorer {
                runtime.stats(p).add_goal();
            }
        }
        event.detail = kind;
    }

    /// `eG(int)`
    pub fn add_goal(&mut self, side: usize) {
        self.goals[side] += 1;
    }
}

/// `I.hX()`: neutral ground. The original's precedence makes it
/// `(competition != null && kind == 5) || kind == 7`.
pub fn is_neutral(world: &World, match_id: MatchId) -> bool {
    match world.game_match(match_id).competition {
        Some(c) => {
            let kind = world.competition(c).kind;
            kind == 5 || kind == 7
        }
        None => false,
    }
}
