// Port of bfmod.Taticas (setting TATICAS): tactics that change the engine's duels. Side 0 is the
// home side. The comments of the original explain the design; the numbers are its calibration.

use std::collections::HashMap;

use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::rng::JavaRandom;
use brasfoot_model::runtime::Runtime;
use brasfoot_model::world::World;

use crate::hooks::MatchHooks;
use crate::value::mod_value;

pub const BALANCED: i32 = 0;
pub const ALL_OUT_ATTACK: i32 = 1;
pub const COUNTER_ATTACK: i32 = 2;

const ATTACK: f64 = 0.10;
const EXPOSURE: f64 = 0.08;
const POSSESSION_GIVEN: f64 = 0.12;
const SHOT: f64 = 0.05;
const HURRY: f64 = 0.08;
const MARKING_DEFENCE: f64 = 0.03;
const MARKING_MIDFIELD: f64 = 0.03;
const FOULS: f64 = 0.06;
const CONCENTRATE: f64 = 0.5;
const CYCLE: f64 = 0.4;
const CYCLE_DUEL: f64 = 0.06;
const AI_NOISE: i32 = 25;
const CARD_COST: f64 = 0.01;

/// `Taticas.Escalacao`: a side's players and, for a forecast, the slots they would take.
#[derive(Clone, Debug, Default)]
pub struct Lineup {
    pub players: Vec<PlayerId>,
    pub slots: Option<HashMap<PlayerId, i32>>,
}

impl Lineup {
    fn slot(&self, world: &World, p: PlayerId) -> i32 {
        self.slots.as_ref().and_then(|s| s.get(&p).copied()).unwrap_or(world.player(p).slot)
    }
}

/// `Taticas.Partida`
#[derive(Clone, Debug, Default)]
pub struct TacticsMatch {
    pub clubs: [Option<ClubId>; 2],
    pub lineups: [Lineup; 2],
    /// {style, marking, where to attack} per side for a forecast.
    pub fixed: Option<[Option<[i32; 3]>; 2]>,
    pub midfield: [f64; 2],
    pub fit_midfield: [f64; 2],
    pub fit_speed: [f64; 2],
    pub fit_defending: [f64; 2],
    pub changed_losing: [bool; 2],
    pub changed_winning: [bool; 2],
    pub automatic: bool,
    pub answered_style: i32,
    pub rival_initial_style: i32,
}

/// What a forecast or an advice needs besides the match: the attribute mode.
fn individual(world: &World) -> bool {
    world.game.individual_skills
}

fn tactic(world: &World, club: Option<ClubId>, index: usize, max: i32) -> i32 {
    club.map(|c| world.club(c).tactics.get(index).copied().unwrap_or(0).clamp(0, max)).unwrap_or(0)
}

/// `Taticas.estilo(ah)`
pub fn club_style(world: &World, club: Option<ClubId>) -> i32 {
    tactic(world, club, 1, 2)
}

/// `Taticas.marcacao(ah)`
pub fn club_marking(world: &World, club: Option<ClubId>) -> i32 {
    tactic(world, club, 2, 2)
}

/// `Taticas.concentrar(ah)`
pub fn club_wings(world: &World, club: Option<ClubId>) -> i32 {
    tactic(world, club, 3, 1)
}

impl TacticsMatch {
    fn fixed_tactic(&self, side: usize) -> Option<[i32; 3]> {
        self.fixed.as_ref().map(|f| f[side].expect("a forecast fixes both sides"))
    }

    pub fn style(&self, world: &World, side: usize) -> i32 {
        self.fixed_tactic(side).map(|t| t[0]).unwrap_or_else(|| club_style(world, self.clubs[side]))
    }

    pub fn marking(&self, world: &World, side: usize) -> i32 {
        self.fixed_tactic(side).map(|t| t[1]).unwrap_or_else(|| club_marking(world, self.clubs[side]))
    }

    pub fn wings(&self, world: &World, side: usize) -> i32 {
        self.fixed_tactic(side).map(|t| t[2]).unwrap_or_else(|| club_wings(world, self.clubs[side]))
    }

    /// `Partida.previsao()`: the same squads with tactics to be fixed.
    pub fn forecast(&self) -> TacticsMatch {
        TacticsMatch {
            clubs: self.clubs,
            lineups: self.lineups.clone(),
            fixed: Some([None, None]),
            midfield: self.midfield,
            fit_midfield: self.fit_midfield,
            fit_speed: self.fit_speed,
            fit_defending: self.fit_defending,
            ..Default::default()
        }
    }
}

/// `Taticas.media`: average engine value (0-10) of the line-up's players in these slots.
fn average(world: &World, l: &Lineup, slots: &[i32]) -> f64 {
    let mut sum = 0.0;
    let mut n = 0;
    for &p in &l.players {
        let slot = l.slot(world, p);
        if slots.contains(&slot) {
            sum += mod_value(individual(world), world.player(p), slot) as f64 / 10.0;
            n += 1;
        }
    }
    if n > 0 { sum / n as f64 } else { 0.0 }
}

fn limit(f: f64) -> f64 {
    f.min(1.3).max(0.6)
}

/// `Taticas.encaixe` for the screens: midfield, fast forwards and tough defenders, 0.6 to 1.3.
pub fn fit_of(world: &World, l: &Lineup) -> [f64; 3] {
    fit(world, l)
}

/// `Taticas.encaixe`
fn fit(world: &World, l: &Lineup) -> [f64; 3] {
    let midfield = average(world, l, &[10, 11, 12, 13, 14, 15, 16, 17]);
    let (mut speed, mut defending, mut ns, mut nd) = (0.0, 0.0, 0, 0);
    for &p in &l.players {
        let slot = l.slot(world, p);
        let player = world.player(p);
        if (18..=25).contains(&slot) {
            speed += player.attributes[brasfoot_model::player::attribute::SPEED] as f64;
            ns += 1;
        }
        if (3..=8).contains(&slot) || (11..=13).contains(&slot) {
            defending += player.attributes[brasfoot_model::player::attribute::DEFENDING] as f64;
            nd += 1;
        }
    }
    let individual = individual(world);
    [
        limit(midfield / 4.3),
        if individual && ns > 0 { limit(speed / ns as f64 / 35.0) } else { 1.0 },
        if individual && nd > 0 { limit(defending / nd as f64 / 57.0) } else { 1.0 },
    ]
}

/// `Taticas.medir`
pub fn measure(world: &World, home: Option<ClubId>, away: Option<ClubId>, home_lineup: Lineup, away_lineup: Lineup) -> TacticsMatch {
    let mut p = TacticsMatch { clubs: [home, away], lineups: [home_lineup, away_lineup], ..Default::default() };
    for side in 0..2 {
        p.midfield[side] = average(world, &p.lineups[side], &[10, 11, 12, 13, 14, 15, 16, 17]);
        let f = fit(world, &p.lineups[side]);
        p.fit_midfield[side] = f[0];
        p.fit_speed[side] = f[1];
        p.fit_defending[side] = f[2];
    }
    p
}

fn beats(a: i32, b: i32) -> bool {
    a == ALL_OUT_ATTACK && b == BALANCED || a == BALANCED && b == COUNTER_ATTACK || a == COUNTER_ATTACK && b == ALL_OUT_ATTACK
}

/// `Taticas.vantagem`
fn advantage(world: &World, p: &TacticsMatch, a: usize, b: usize) -> i32 {
    let (ea, eb) = (p.style(world, a), p.style(world, b));
    if beats(ea, eb) {
        1
    } else if beats(eb, ea) {
        -1
    } else {
        0
    }
}

/// `Taticas.ciclo`
fn cycle(world: &World, p: &TacticsMatch, a: usize, b: usize) -> f64 {
    1.0 + CYCLE * advantage(world, p, a, b) as f64
}

/// `Taticas.meio(Partida, int)`
pub fn midfield_weight(world: &World, p: &TacticsMatch, side: usize) -> f64 {
    let mut f = 1.0;
    if p.style(world, side) == COUNTER_ATTACK {
        f *= 1.0 - POSSESSION_GIVEN;
    }
    f * (1.0 + MARKING_MIDFIELD * p.marking(world, side) as f64 * p.fit_defending[side])
}

/// `Taticas.chutar(Partida, int)`
pub fn shoot_weight(world: &World, p: &TacticsMatch, attacker: usize) -> f64 {
    let defender = 1 - attacker;
    let mut f = 1.0 + CYCLE_DUEL * advantage(world, p, attacker, defender) as f64;
    if p.style(world, attacker) == ALL_OUT_ATTACK {
        f *= 1.0 + ATTACK * p.fit_midfield[attacker] * cycle(world, p, attacker, defender);
    }
    f
}

/// `Taticas.bloquear(Partida, int)`
pub fn block_weight(world: &World, p: &TacticsMatch, defender: usize) -> f64 {
    let attacker = 1 - defender;
    let mut f = (1.0 + MARKING_DEFENCE * p.marking(world, defender) as f64 * p.fit_defending[defender])
        * (1.0 + CYCLE_DUEL * advantage(world, p, defender, attacker) as f64);
    if p.style(world, defender) == ALL_OUT_ATTACK {
        f *= 1.0 - EXPOSURE * cycle(world, p, attacker, defender);
    }
    f
}

/// `Taticas.goleiro(Partida, int)`
pub fn keeper_weight(world: &World, p: &TacticsMatch, attacker: usize) -> f64 {
    let mut f = 1.0 - FOULS * p.marking(world, 1 - attacker) as f64;
    match p.style(world, attacker) {
        COUNTER_ATTACK => f *= 1.0 - SHOT * p.fit_speed[attacker] * cycle(world, p, attacker, 1 - attacker),
        ALL_OUT_ATTACK => f *= 1.0 + HURRY,
        _ => {}
    }
    f
}

/// `Taticas.desvio`
pub fn deviation(world: &World, p: &TacticsMatch, attacker: usize, wings: bool) -> f64 {
    let attack = &p.lineups[attacker];
    let defence = &p.lineups[1 - attacker];
    let mut target_attack = if wings {
        average(world, attack, &[10, 17, 18, 25])
    } else {
        average(world, attack, &[14, 15, 16, 19, 20, 21, 22, 23, 24])
    };
    let mut target_defence =
        if wings { average(world, defence, &[2, 9, 10, 17]) } else { average(world, defence, &[3, 4, 5, 6, 7, 8, 11, 12, 13]) };
    if target_attack <= 0.0 {
        target_attack = 0.6 * average(world, attack, &[19, 20, 21, 22, 23, 24, 25]);
    }
    if target_defence <= 0.0 {
        target_defence = 0.6 * average(world, defence, &[3, 4, 5, 6, 7, 8]);
    }
    let base_attack = average(world, attack, &[19, 20, 21, 22, 23, 24, 25]);
    let base_defence = average(world, defence, &[2, 3, 4, 5, 6, 7, 8, 9]);
    CONCENTRATE * ((target_attack - target_defence) - (base_attack - base_defence))
}

/// `Taticas.escolha`: what the AI rules pick, {style, marking, where to attack}.
pub fn ai_choice(world: &World, p: &TacticsMatch, side: usize) -> [i32; 3] {
    let other = 1 - side;
    let gap = p.midfield[side] - p.midfield[other];
    let style = if gap >= 0.5 {
        if p.fit_midfield[side] >= 1.0 { ALL_OUT_ATTACK } else { BALANCED }
    } else if gap <= -0.5 {
        if p.fit_speed[side] >= 0.9 { COUNTER_ATTACK } else { BALANCED }
    } else if p.fit_midfield[side] >= 1.15 {
        ALL_OUT_ATTACK
    } else if p.fit_speed[side] >= 1.15 {
        COUNTER_ATTACK
    } else {
        BALANCED
    };
    let mut marking = if p.fit_defending[side] >= 1.1 { 1 } else { 0 };
    if gap <= -0.8 && p.fit_defending[side] >= 1.0 {
        marking = 2;
    }
    let wings = if deviation(world, p, side, true) > deviation(world, p, side, false) { 1 } else { 0 };
    [style, marking, wings]
}

pub fn set_club_tactic(world: &mut World, club: ClubId, t: [i32; 3]) {
    let kj = &mut world.club_mut(club).tactics;
    if kj.len() < 4 {
        kj.resize(4, 0);
    }
    kj[1] = t[0];
    kj[2] = t[1];
    kj[3] = t[2];
}

/// `Taticas.escolherIA`
fn ai_pick(world: &mut World, runtime: &mut Runtime, hooks: &dyn MatchHooks, p: &TacticsMatch, side: usize) {
    let club = p.clubs[side].expect("a side with a club");
    if let Some(directive) = hooks.tactic_directive(world, club) {
        set_club_tactic(world, club, directive);
        return;
    }
    let t = ai_choice(world, p, side);
    let r = runtime.tactics_random.get_or_insert_with(|| JavaRandom::new(0));
    let mut noisy = |value: i32, choices: i32| if r.next_int_bounded(100) < AI_NOISE { r.next_int_bounded(choices) } else { value };
    let style = noisy(t[0], 3);
    let marking = noisy(t[1], 3);
    let wings = noisy(t[2], 2);
    set_club_tactic(world, club, [style, marking, wings]);
}

/// The tactics of the matches being played (`Taticas.partidas`).
#[derive(Debug, Default)]
pub struct TacticsTable {
    pub matches: HashMap<MatchId, TacticsMatch>,
}

fn is_human(world: &World, club: Option<ClubId>) -> bool {
    club.is_some_and(|c| world.club(c).human)
}

/// `Taticas.inicio`: the AI sides pick, the squads are measured. Returns the match's state.
pub fn kickoff(world: &mut World, runtime: &mut Runtime, hooks: &mut dyn MatchHooks, match_id: MatchId) -> Option<TacticsMatch> {
    hooks.on_kickoff(world, runtime, match_id);
    if !hooks.tactics_enabled() {
        return None;
    }
    let m = world.game_match(match_id);
    let (home, away) = (m.home, m.away);
    let lineups = [Lineup { players: m.home_played.clone(), slots: None }, Lineup { players: m.away_played.clone(), slots: None }];
    let [hl, al] = lineups;
    let mut p = measure(world, home, away, hl, al);
    for side in 0..2 {
        if !is_human(world, p.clubs[side]) && p.clubs[side].is_some() {
            ai_pick(world, runtime, hooks, &p, side);
        }
    }
    for side in 0..2 {
        if is_human(world, p.clubs[side]) && !is_human(world, p.clubs[1 - side]) && hooks.auto_tactics(world) {
            let rival = p.clubs[1 - side];
            let club = p.clubs[side].unwrap();
            let current = [club_style(world, Some(club)), club_marking(world, Some(club)), club_wings(world, Some(club))];
            match hooks.prepared_tactic(world, rival.unwrap()) {
                Some((applied, predicted)) => {
                    // written before the match: kept, unless the player changed it on the line-up screen
                    if current == applied {
                        p.automatic = true;
                        p.answered_style = predicted;
                    }
                }
                None => {
                    let seed = advice_seed(world, rival.unwrap());
                    let advice = advise(world, &p, side, hooks.analyst_level(world), seed);
                    set_club_tactic(world, club, advice.best);
                    p.automatic = true;
                    p.answered_style = advice.predicted_rival.map(|t| t[0]).unwrap_or(-1);
                }
            }
            p.rival_initial_style = club_style(world, rival);
        }
    }
    Some(p)
}

/// `Taticas.lance`: the AI goes for it when losing after 60', shuts up shop when winning after 75'.
pub fn tick(world: &mut World, runtime: &mut Runtime, hooks: &mut dyn MatchHooks, match_id: MatchId, p: Option<&mut TacticsMatch>, minute: i32, goals: [i32; 2]) {
    hooks.on_tick(world, runtime, match_id);
    let Some(p) = p else { return };
    for side in 0..2 {
        let Some(club) = p.clubs[side] else { continue };
        if world.club(club).human {
            let now = club_style(world, p.clubs[1 - side]);
            let seen = minute >= 46 || now != p.rival_initial_style;
            if p.automatic && now != p.answered_style && seen {
                p.answered_style = now;
                let rival = p.clubs[1 - side];
                let answer = reply(world, p, side, [club_style(world, rival), club_marking(world, rival), club_wings(world, rival)]);
                set_club_tactic(world, club, answer);
            }
            continue;
        }
        let balance = goals[side] - goals[1 - side];
        if balance < 0 && minute >= 60 && !p.changed_losing[side] {
            p.changed_losing[side] = true;
            world.club_mut(club).tactics[1] = ALL_OUT_ATTACK;
        } else if balance > 0 && minute >= 75 && !p.changed_winning[side] {
            p.changed_winning[side] = true;
            let other = p.style(world, 1 - side);
            world.club_mut(club).tactics[1] = if other == ALL_OUT_ATTACK { COUNTER_ATTACK } else { BALANCED };
        }
    }
}

/// `Taticas.semente`: the analyst's guess is fixed per rival and day.
pub fn advice_seed(world: &World, rival: ClubId) -> i64 {
    31 * world.club(rival).id as i64 + world.today_millis() / 86_400_000
}

/// `Taticas.Conselho`
#[derive(Clone, Debug, Default)]
pub struct Advice {
    pub predicted_rival: Option<[i32; 3]>,
    pub best: [i32; 3],
    pub points: f64,
    pub points_balanced: f64,
}

/// `Taticas.aconselhar(Partida, int, int, long)`
pub fn advise(world: &World, base: &TacticsMatch, my_side: usize, analyst: i32, seed: i64) -> Advice {
    let mut p = base.forecast();
    let mut advice = Advice::default();
    let rule = ai_choice(world, &p, 1 - my_side);
    if analyst >= 3 {
        let mut predicted = rule;
        let mut r = JavaRandom::new(seed);
        let error = if analyst >= 5 { 0 } else if analyst == 4 { 15 } else { 30 };
        if r.next_int_bounded(100) < error {
            predicted[0] = (rule[0] + 1 + r.next_int_bounded(2)) % 3;
        }
        advice.predicted_rival = Some(predicted);
    }
    advice.points = -1.0;
    for style in 0..3 {
        for marking in 0..3 {
            for wings in 0..2 {
                let t = [style, marking, wings];
                set_fixed(&mut p, my_side, t);
                let points = expected(world, &mut p, my_side, advice.predicted_rival, rule) - CARD_COST * (marking * marking) as f64;
                if points > advice.points {
                    advice.points = points;
                    advice.best = t;
                }
            }
        }
    }
    set_fixed(&mut p, my_side, [BALANCED, 0, 0]);
    advice.points_balanced = expected(world, &mut p, my_side, advice.predicted_rival, rule);
    advice
}

fn set_fixed(p: &mut TacticsMatch, side: usize, t: [i32; 3]) {
    p.fixed.get_or_insert([None, None])[side] = Some(t);
}

/// `Taticas.resposta`: the best answer to a tactic the player can see.
pub fn reply(world: &World, base: &TacticsMatch, my_side: usize, rival: [i32; 3]) -> [i32; 3] {
    let mut p = base.forecast();
    set_fixed(&mut p, 1 - my_side, rival);
    let mut best = -1.0;
    let mut choice = [BALANCED, 0, 0];
    for style in 0..3 {
        for marking in 0..3 {
            for wings in 0..2 {
                let t = [style, marking, wings];
                set_fixed(&mut p, my_side, t);
                let points = points(world, &p, my_side) - CARD_COST * (marking * marking) as f64;
                if points > best {
                    best = points;
                    choice = t;
                }
            }
        }
    }
    choice
}

/// `Taticas.esperado`
fn expected(world: &World, p: &mut TacticsMatch, my_side: usize, predicted: Option<[i32; 3]>, rule: [i32; 3]) -> f64 {
    if let Some(predicted) = predicted {
        set_fixed(p, 1 - my_side, predicted);
        return points(world, p, my_side);
    }
    let mut sum = 0.0;
    for style in 0..3 {
        set_fixed(p, 1 - my_side, [style, rule[1], rule[2]]);
        sum += points(world, p, my_side);
    }
    sum / 3.0
}

/// `Taticas.pontos`: expected points of a side (side 0 at home).
pub fn points(world: &World, p: &TacticsMatch, side: usize) -> f64 {
    let mine = goals(world, p, side);
    let theirs = goals(world, p, 1 - side);
    let (mut wins, mut draws) = (0.0, 0.0);
    for i in 0..=10 {
        for j in 0..=10 {
            let q = poisson(mine, i) * poisson(theirs, j);
            if i > j {
                wins += q;
            } else if i == j {
                draws += q;
            }
        }
    }
    3.0 * wins + draws
}

fn poisson(l: f64, k: i32) -> f64 {
    let mut q = (-l).exp();
    for i in 1..=k {
        q *= l / i as f64;
    }
    q
}

/// `Taticas.soma`: the first `max` engine values in slots `from..=to`, in slot order.
fn sum(world: &World, l: &Lineup, max: usize, from: i32, to: i32) -> f64 {
    let mut order: Vec<PlayerId> = l.players.clone();
    // List.sort is stable, like Rust's sort_by
    order.sort_by_key(|&p| l.slot(world, p));
    let mut s = 0.0;
    let mut n = 0;
    for p in order {
        let slot = l.slot(world, p);
        if slot >= from && slot <= to && n < max {
            s += mod_value(individual(world), world.player(p), slot) as f64 / 10.0;
            n += 1;
        }
    }
    s
}

/// `Taticas.gols`: expected goals of `x` in about 46 attacking turns.
pub fn goals(world: &World, p: &TacticsMatch, x: usize) -> f64 {
    let y = 1 - x;
    let home = x == 0;
    let (lx, ly) = (&p.lineups[x], &p.lineups[y]);
    let mx = sum(world, lx, 5, 10, 17) / 5.0;
    let my = sum(world, ly, 5, 10, 17) / 5.0;
    let wa = ((1.0 + (mx - my) / 11.0 + if home { 0.3 } else { 0.0 }) * midfield_weight(world, p, x)).max(0.2);
    let wd = ((1.0 + (my - mx) / 11.0) * midfield_weight(world, p, y)).max(0.2);
    let p_ball = 55.0 * wa / (55.0 * wa + 45.0 * wd);
    let mut attack = sum(world, lx, 3, 19, 25) / 3.0;
    let mut defence = sum(world, ly, 5, 2, 9) / 5.0;
    let d = if attack > 0.0 && defence > 0.0 { deviation(world, p, x, p.wings(world, x) == 1) } else { 0.0 };
    attack += d / 2.0;
    defence -= d / 2.0;
    let w5 = ((1.0 + (attack - defence) / 11.0 + if home { 0.3 } else { 0.0 }) * shoot_weight(world, p, x)).max(0.2);
    let w7 = ((1.0 + (defence - attack) / 11.0) * block_weight(world, p, y)).max(0.2);
    let p_shot = w5 / (w5 + w7);
    let keeper = sum(world, ly, 1, 1, 1);
    let (mut weights, mut p_goal) = (0.0, 0.0);
    for &f in &lx.players {
        let slot = lx.slot(world, f);
        let player = world.player(f);
        if slot <= 1 || slot > 25 || player.position == 0 {
            continue;
        }
        let mut w = if slot >= 18 {
            22.0
        } else if slot == 10 || slot >= 14 {
            8.0
        } else if slot >= 11 {
            4.0
        } else {
            1.0
        };
        if player.trait1 == 9 || player.trait2 == 9 {
            w += 4.0;
        } else if player.trait1 == 5 || player.trait2 == 5 {
            w += if player.position == 2 { 4.0 } else { 2.0 };
        }
        let save = ((1.0 + (keeper - mod_value(individual(world), player, slot) as f64 / 10.0) / 10.0) * keeper_weight(world, p, x)
            + if home { 0.1 } else { -0.1 })
        .max(0.2);
        let wide = (save + if home { 0.1 } else { -0.1 }).max(0.2);
        p_goal += w * 5.5 / (5.5 + 35.55 * save + 15.0 * wide);
        weights += w;
    }
    if weights > 0.0 { 46.5 * p_ball * p_shot * p_goal / weights } else { 0.0 }
}

/// `Motor.vagaTipica`: a slot of each kind, to evaluate a player there.
pub const TYPICAL_SLOT: [i32; 8] = [1, 4, 2, 10, 12, 15, 21, 18];

/// `Motor.melhorValor`: best value in the slot kinds of his own position.
pub fn best_value(world: &World, p: PlayerId) -> i32 {
    let player = world.player(p);
    TYPICAL_SLOT
        .iter()
        .filter(|&&s| crate::tables::SLOT_ROLE[s as usize][0] == player.position)
        .map(|&s| mod_value(individual(world), player, s))
        .fold(1, i32::max)
}

/// `Motor.melhorTipo`
pub fn best_kind(world: &World, p: PlayerId) -> usize {
    let player = world.player(p);
    let mut best = -1;
    let mut kind = 0;
    for (t, &s) in TYPICAL_SLOT.iter().enumerate() {
        let v = mod_value(individual(world), player, s);
        if crate::tables::SLOT_ROLE[s as usize][0] == player.position && v > best {
            best = v;
            kind = t;
        }
    }
    kind
}

/// `Motor.titular`: in the club's last line-up.
pub fn is_starter(world: &World, p: PlayerId) -> bool {
    let player = world.player(p);
    if player.slot < 1 || player.slot > 25 {
        return false;
    }
    match player.club {
        Some(c) => {
            let eleven = &world.club(c).last_starters;
            eleven.is_empty() || eleven.contains(&p)
        }
        None => true,
    }
}

/// `Taticas.onze`: a club's likely XI on paper.
pub fn likely_eleven(world: &World, club: ClubId) -> Lineup {
    let today = world.today_millis();
    let (mut out, mut rest) = (Vec::new(), Vec::new());
    for &p in &world.club(club).squad {
        if world.player(p).is_injured(today) {
            continue;
        }
        if is_starter(world, p) && out.len() < 11 {
            out.push(p);
        } else {
            rest.push(p);
        }
    }
    rest.sort_by(|&a, &b| best_value(world, b).cmp(&best_value(world, a)));
    let mut slots = HashMap::new();
    for p in rest {
        if out.len() >= 11 {
            break;
        }
        slots.insert(p, TYPICAL_SLOT[best_kind(world, p)]);
        out.push(p);
    }
    Lineup { players: out, slots: Some(slots) }
}
