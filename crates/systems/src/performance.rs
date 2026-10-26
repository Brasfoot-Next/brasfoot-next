// Port of bfmod.Desempenho: the performance analysis of the player's club from the matches the
// game keeps for the season (score, stats, line-ups and events). What it shows depends on the
// staff (Comissao): every finding has the level it needs, and the locked ones say who to hire.
// Strength is the engine's (Motor): in individual-skills mode the slot's attributes, not the
// strength on screen. The findings are data; the app words them.

use std::collections::HashSet;

use brasfoot_engine::tactics::{Advice, advise, best_kind, best_value, is_starter, likely_eleven, measure};
use brasfoot_engine::value::{mod_value, slot_kind};
use brasfoot_model::ids::{ClubId, CompetitionId, MatchId, PlayerId, StageId};
use brasfoot_model::mods::StaffRole;
use brasfoot_model::world::World;

use crate::Mod;
use crate::config::Switch;
use crate::loans::season_numbers;
use crate::market::{starters_average, team_average};
use crate::staff::my_club;

const DAY_MILLIS: i64 = 86_400_000;

/// `Desempenho.jogos(ah)`: the club's matches already played, in calendar order.
pub fn played_matches(world: &World, club: ClubId) -> Vec<MatchId> {
    let today = world.game.day.max(0) as usize;
    world
        .game
        .days
        .iter()
        .take(today + 1)
        .flat_map(|d| d.matches.iter().copied())
        .filter(|&m| {
            let g = world.game_match(m);
            (g.home == Some(club) || g.away == Some(club)) && g.played
        })
        .collect()
}

/// `Desempenho.Tipo`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Problem,
    Attention,
    Strong,
    Info,
    Locked,
}

/// What a locked finding would show (`bloqueado(cargo, nivel, oQue)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unlock {
    AnalystStats,
    AnalystDetails,
    AnalystStrength,
    AnalystPlayers,
    AnalystSeason,
    AssistantSuggestions,
    AssistantSwaps,
    AssistantAdjustments,
    AssistantYoungAndOld,
    AssistantGamePlan,
    AssistantTactic,
    PhysioReturn,
    PhysioHistory,
    PhysioSuspensions,
    PhysioRest,
    PhysioSchedule,
}

/// One finding of the analysis: what the original's title and text say, with their numbers.
#[derive(Clone, Debug, PartialEq)]
pub enum Finding {
    Locked { role: StaffRole, level: i32, unlock: Unlock },
    NoGames,
    /// Individual skills: the starters' strength on the pitch against the one on screen.
    PitchStrength { pitch: f64, screen: f64 },
    Decline { recent: f64, games: usize, form: String, before: f64, before_games: usize },
    Rising { recent: f64, games: usize, form: String, before: f64 },
    Form { form: String, recent: f64, games: usize },
    DefenceLeaking { conceded: f64, before: f64, shots_against: f64, shots_against_before: Option<f64> },
    SolidDefence { conceded: f64, games: usize },
    /// `aim`: still shooting (shots, conversion %, conversion % before); else the shots fell.
    AttackFaded { goals: f64, before: f64, shots: f64, aim: Option<(f64, f64)> },
    SharpAttack { goals: f64, games: usize },
    SterilePossession { possession: f64 },
    LateCollapse { late: i32, conceded: i32, games: usize },
    BadStarts { early: i32, conceded: i32 },
    CostlyRedCards { reds: i32, games: usize, points_lost: i32 },
    ManyCards { per_game: f64 },
    AbsencesInDefeats { defeats: i32, average: f64 },
    InjuredStarters { players: Vec<String> },
    Underperforming { expected: f64, games: usize, points: i32 },
    Overperforming { points: i32, games: usize, expected: f64 },
    AsExpected { points: i32, games: usize, expected: f64 },
    ToughRun { recent: f64, season: f64 },
    WeakerLineups { recent: f64, season: f64 },
    WeakAway { home: f64, away: f64 },
    WeakHome { home: f64, away: f64 },
    LowCohesion { changes: f64, players: usize, games: usize },
    ScorerDependence { player: String, goals: i32, total: i32 },
    PlayersInSlump { average: f64, players: Vec<(String, f64)> },
    StrikerWithoutGoals { player: String },
    /// `Reforçar: {posição}` by the market's weakest position.
    Reinforce { position: i32, starters: f64, team: f64 },
    /// Individual skills: the weakest kind of slot.
    ReinforceSlot { kind: usize, slot: f64, team: f64 },
    PlayInSlot { player: String, kind: usize, value: i32, starter: String, current: i32, screen: Option<(i32, i32)> },
    OutOfPlace { player: String, value: i32, kind: usize, best_kind: usize, best: i32 },
    RightLineup,
    ChangeEarlier,
    ProtectDefence,
    FocusAttacks,
    MoreDirect,
    LessAggressive,
    SettleTeam,
    GiveMinutes { player: String, age: i32, value: i32 },
    PlanExit { player: String, age: i32, strength: i32 },
    /// `Plano para o {adversário}`: 0 favourites, 1 the rival is stronger, 2 even.
    GamePlan { rival: ClubId, plan: i32 },
    /// `Tática para o {adversário}` (`conselhoTatico`).
    Tactic { rival: ClubId, advice: TacticAdvice },
    NoInjured,
    /// The injured and, with a physio, the day each comes back.
    Injured { players: Vec<(String, Option<i64>)> },
    InjuryHistory { total: i32, prone: Vec<(String, i32)> },
    NextMatch { stage: Option<StageId>, competition: Option<CompetitionId>, suspended: Vec<String>, booked: Vec<String> },
    RestBooked { first: String, more: bool },
    TightSchedule { games: i32 },
}

/// `conselhoTatico`'s pieces.
#[derive(Clone, Debug, PartialEq)]
pub struct TacticAdvice {
    pub best: [i32; 3],
    pub predicted: Option<[i32; 3]>,
    /// The analyst of level 5 also guesses the marking and the side.
    pub full_guess: bool,
    /// Some(true) when the style beats the guessed one, Some(false) when it loses the cycle.
    pub beats: Option<bool>,
    pub fast_forwards: bool,
    pub strong_midfield: bool,
    pub tough_defence: bool,
    pub automatic: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub kind: Kind,
    pub finding: Finding,
}

/// `Desempenho.Jogo`: a finished match from the club's side.
#[derive(Clone, Debug)]
pub struct Game {
    pub id: MatchId,
    pub date: i64,
    pub stage: Option<StageId>,
    pub competition: Option<CompetitionId>,
    pub opponent: Option<ClubId>,
    pub home: bool,
    pub scored: i32,
    pub conceded: i32,
    pub possession: i32,
    pub shots: i32,
    pub shots_against: i32,
    pub on_target: i32,
    pub on_target_against: i32,
    pub starters: Vec<PlayerId>,
    pub strength: f64,
    pub opponent_strength: f64,
}

impl Game {
    pub fn points(&self) -> i32 {
        if self.scored > self.conceded {
            3
        } else if self.scored == self.conceded {
            1
        } else {
            0
        }
    }

    /// 'V', 'E' or 'D'.
    pub fn result(&self) -> char {
        match self.points() {
            3 => 'V',
            1 => 'E',
            _ => 'D',
        }
    }
}

/// `Desempenho.Proximo`
#[derive(Clone, Debug)]
pub struct NextGame {
    pub rival: ClubId,
    pub home: bool,
    pub date: i64,
    pub stage: Option<StageId>,
    pub competition: Option<CompetitionId>,
    /// The rival's last five, most recent first.
    pub rival_form: Vec<Game>,
    pub strength: f64,
    pub rival_strength: f64,
    pub chances: [f64; 3],
    /// The rival's top scorer this season and his goals.
    pub rival_scorer: Option<(String, i32)>,
}

/// `Desempenho.Analise`
#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub analyst: i32,
    pub assistant: i32,
    pub physio: i32,
    /// Most recent last.
    pub season: Vec<Game>,
    /// The window analysed, most recent first.
    pub window: Vec<Game>,
    pub diagnosis: Vec<Item>,
    pub suggestions: Vec<Item>,
    pub medical: Vec<Item>,
    pub tactic: Option<Item>,
    pub next: Option<NextGame>,
    pub ppg_recent: f64,
    pub ppg_before: f64,
    pub expected: f64,
    pub expected_points: i32,
    pub expected_games: usize,
    pub window_points: i32,
    /// `Motor.individual()`: the texts say "força em campo".
    pub individual: bool,
}

fn individual(world: &World) -> bool {
    world.game.individual_skills
}

/// `Motor.valor(F, int)`
fn value_in(world: &World, p: PlayerId, slot: i32) -> i32 {
    mod_value(individual(world), world.player(p), slot)
}

/// `Motor.forca(F)`: in the line-up's slot for a starter, at his best otherwise.
pub fn strength(world: &World, p: PlayerId) -> i32 {
    if is_starter(world, p) { value_in(world, p, world.player(p).slot) } else { best_value(world, p) }
}

fn average_strength(world: &World, players: &[PlayerId]) -> f64 {
    if players.is_empty() {
        return 0.0;
    }
    players.iter().map(|&p| strength(world, p) as f64).sum::<f64>() / players.len() as f64
}

fn game(world: &World, m: MatchId, date: i64, club: ClubId) -> Game {
    let g = world.game_match(m);
    let home = g.home == Some(club);
    let (me, them) = if home { (0, 1) } else { (1, 0) };
    let at = |v: &Vec<i32>, i: usize| v.get(i).copied().unwrap_or(0);
    let starters = if home { g.home_starters.clone() } else { g.away_starters.clone() };
    let theirs = if home { &g.away_starters } else { &g.home_starters };
    Game {
        id: m,
        date,
        stage: g.stage,
        competition: g.competition,
        opponent: if home { g.away } else { g.home },
        home,
        scored: if home { g.home_goals } else { g.away_goals },
        conceded: if home { g.away_goals } else { g.home_goals },
        possession: at(&g.possession, me),
        shots: at(&g.shots, me),
        shots_against: at(&g.shots, them),
        on_target: at(&g.shots_on_target, me),
        on_target_against: at(&g.shots_on_target, them),
        strength: average_strength(world, &starters),
        opponent_strength: average_strength(world, theirs),
        starters,
    }
}

/// `Desempenho.jogos(ah)` with each match read from the club's side.
pub fn season_games(world: &World, club: ClubId) -> Vec<Game> {
    let today = world.game.day.max(0) as usize;
    let mut out = Vec::new();
    for day in world.game.days.iter().take(today + 1) {
        for &m in &day.matches {
            let g = world.game_match(m);
            if (g.home == Some(club) || g.away == Some(club)) && g.played {
                out.push(game(world, m, day.date, club));
            }
        }
    }
    out
}

fn ppg(games: &[Game]) -> f64 {
    if games.is_empty() { 0.0 } else { games.iter().map(|g| g.points() as f64).sum::<f64>() / games.len() as f64 }
}

fn per_game(games: &[Game], f: impl Fn(&Game) -> i32) -> f64 {
    if games.is_empty() { 0.0 } else { games.iter().map(|g| f(g) as f64).sum::<f64>() / games.len() as f64 }
}

fn sum(games: &[Game], f: impl Fn(&Game) -> i32) -> f64 {
    games.iter().map(|g| f(g) as f64).sum()
}

/// `Desempenho.chances(double, double, boolean)`: win, draw and loss by the strength gap.
pub fn chances(world: &World, strength: f64, rival: f64, home: bool) -> [f64; 3] {
    let diff = strength - rival;
    let individual = individual(world);
    let draw = (if individual { 0.26 } else { 0.28 }) * (-diff.abs() / 80.0).exp();
    let win = (1.0 - draw) / (1.0 + (-((if home { 0.04 } else { -0.04 }) + (if individual { 0.07 } else { 0.05 }) * diff)).exp());
    [win, draw, 1.0 - win - draw]
}

fn expected_points(world: &World, g: &Game) -> f64 {
    let p = chances(world, g.strength, g.opponent_strength, g.home);
    3.0 * p[0] + p[1]
}

/// `Desempenho.minuto(A)`: extra time after 90.
fn minute(e: &brasfoot_model::saved::MatchEvent) -> i32 {
    match e.half {
        ..=1 => e.minute,
        2 => 45 + e.minute,
        _ => 90 + e.minute,
    }
}

/// `Desempenho.regulares(List, double)`: who started at least `share` of the matches, in the
/// order they first appear (the original's `HashSet` has no fixed order).
fn regulars(games: &[Game], share: f64) -> Vec<PlayerId> {
    let mut counts: Vec<(PlayerId, usize)> = Vec::new();
    for g in games {
        for &p in &g.starters {
            match counts.iter_mut().find(|(x, _)| *x == p) {
                Some(c) => c.1 += 1,
                None => counts.push((p, 1)),
            }
        }
    }
    counts.into_iter().filter(|&(_, n)| n as f64 >= games.len() as f64 * share).map(|(p, _)| p).collect()
}

/// `Desempenho.forma(List)`: the oldest on the left.
fn form(games: &[Game]) -> String {
    games.iter().rev().map(|g| g.result()).collect()
}

fn locked(role: StaffRole, level: i32, unlock: Unlock) -> Item {
    Item { kind: Kind::Locked, finding: Finding::Locked { role, level, unlock } }
}

fn item(kind: Kind, finding: Finding) -> Item {
    Item { kind, finding }
}

/// `Desempenho.melhores11(ah)`: the engine's best value of the 11 best fit players.
pub fn best_eleven(world: &World, club: ClubId) -> f64 {
    let today = world.today_millis();
    let mut values: Vec<i32> = world.club(club).squad.iter().filter(|&&p| !world.player(p).is_injured(today)).map(|&p| best_value(world, p)).collect();
    values.sort_by(|a, b| b.cmp(a));
    let n = values.len().min(11);
    if n > 0 { values[..n].iter().map(|&v| v as f64).sum::<f64>() / n as f64 } else { 0.0 }
}

/// `Desempenho.proximo(ah)`
pub fn next_game(world: &World, club: ClubId) -> Option<NextGame> {
    for day in world.game.days.iter().skip(world.game.day.max(0) as usize) {
        for &m in &day.matches {
            let g = world.game_match(m);
            if (g.home == Some(club) || g.away == Some(club)) && !g.played {
                let home = g.home == Some(club);
                let rival = if home { g.away } else { g.home }?;
                let games = season_games(world, rival);
                let rival_form: Vec<Game> = games.iter().rev().take(5).cloned().collect();
                let strength = best_eleven(world, club);
                let rival_strength = best_eleven(world, rival);
                let mut scorer: Option<(String, i32)> = None;
                for &p in &world.club(rival).squad {
                    let goals = season_numbers(world, p).1;
                    if goals > scorer.as_ref().map_or(0, |s| s.1) {
                        scorer = Some((world.player(p).name.clone(), goals));
                    }
                }
                return Some(NextGame {
                    rival,
                    home,
                    date: day.date,
                    stage: g.stage,
                    competition: g.competition,
                    rival_form,
                    strength,
                    rival_strength,
                    chances: chances(world, strength, rival_strength, home),
                    rival_scorer: scorer,
                });
            }
        }
    }
    None
}

/// The suspension and the yellow cards of a player in a competition (`F.c(at)`, `F.e(at)`).
fn suspended_in(world: &World, p: PlayerId, c: Option<CompetitionId>) -> bool {
    let Some(c) = c else { return false };
    world.player(p).competition_stats.iter().find(|h| h.competition_id == Some(c)).is_some_and(|h| h.is_suspended())
}

fn yellows_in(world: &World, p: PlayerId, c: CompetitionId) -> Option<i32> {
    world.player(p).competition_stats.iter().find(|h| h.competition_id == Some(c)).map(|h| h.yellow_cards)
}

impl Mod<'_> {
    /// `Desempenho.analisar()`
    pub fn analyse_performance(&mut self) -> Analysis {
        let mut a = Analysis { ppg_before: -1.0, expected: -1.0, individual: individual(self.world), ..Default::default() };
        let Some(club) = my_club(self.world) else { return a };
        a.analyst = self.staff_level(StaffRole::Analyst);
        a.assistant = self.staff_level(StaffRole::Assistant);
        a.physio = self.staff_level(StaffRole::FitnessCoach);
        a.season = season_games(self.world, club);
        let n = if a.analyst >= 5 {
            a.season.len()
        } else if a.analyst >= 1 {
            10
        } else {
            5
        };
        a.window = a.season.iter().rev().take(n).cloned().collect();
        a.next = next_game(self.world, club);
        if self.sys.config.on(Switch::Tactics)
            && let Some(next) = &a.next
        {
            a.tactic = Some(if a.assistant >= 3 { self.tactic_advice(club, next, a.analyst) } else { locked(StaffRole::Assistant, 3, Unlock::AssistantTactic) });
        }
        if a.window.is_empty() {
            a.diagnosis.push(item(Kind::Info, Finding::NoGames));
            self.medical(&mut a, club);
            return a;
        }
        let recent: Vec<Game> = a.window.iter().take(5).cloned().collect();
        let before: Vec<Game> = a.season.iter().rev().skip(recent.len()).take(10).cloned().collect();
        a.ppg_recent = ppg(&recent);
        a.ppg_before = if before.is_empty() { -1.0 } else { ppg(&before) };
        a.window_points = a.window.iter().map(|g| g.points()).sum();
        self.diagnosis(&mut a, club, &recent, &before);
        self.suggestions(&mut a, club);
        self.medical(&mut a, club);
        a
    }

    fn diagnosis(&mut self, a: &mut Analysis, club: ClubId, recent: &[Game], before: &[Game]) {
        let world = &*self.world;
        let level = a.analyst;
        let d = &mut a.diagnosis;
        let f = form(recent);
        // level 0: the coach's own read
        if individual(world) {
            let starters: Vec<PlayerId> = world.club(club).squad.iter().copied().filter(|&p| is_starter(world, p)).collect();
            if !starters.is_empty() {
                let screen = starters.iter().map(|&p| world.player(p).strength as f64).sum::<f64>() / starters.len() as f64;
                let pitch = average_strength(world, &starters);
                d.push(item(if screen - pitch >= 10.0 { Kind::Attention } else { Kind::Info }, Finding::PitchStrength { pitch, screen }));
            }
        }
        if a.ppg_before >= 0.0 && a.ppg_recent <= a.ppg_before - 0.5 {
            d.push(item(Kind::Problem, Finding::Decline { recent: a.ppg_recent, games: recent.len(), form: f.clone(), before: a.ppg_before, before_games: before.len() }));
        } else if a.ppg_before >= 0.0 && a.ppg_recent >= a.ppg_before + 0.5 {
            d.push(item(Kind::Strong, Finding::Rising { recent: a.ppg_recent, games: recent.len(), form: f.clone(), before: a.ppg_before }));
        } else {
            d.push(item(Kind::Info, Finding::Form { form: f.clone(), recent: a.ppg_recent, games: recent.len() }));
        }
        if level < 1 {
            d.push(locked(StaffRole::Analyst, 1, Unlock::AnalystStats));
            return;
        }
        // level 1: attack and defence
        let goals_r = per_game(recent, |g| g.scored);
        let goals_b = per_game(before, |g| g.scored);
        let conceded_r = per_game(recent, |g| g.conceded);
        let conceded_b = per_game(before, |g| g.conceded);
        let shots_r = per_game(recent, |g| g.shots);
        let shots_against_r = per_game(recent, |g| g.shots_against);
        let shots_against_b = per_game(before, |g| g.shots_against);
        let on_target_r = sum(recent, |g| g.on_target);
        let conversion = if on_target_r > 0.0 { sum(recent, |g| g.scored) / on_target_r } else { 0.0 };
        let on_target_b = sum(before, |g| g.on_target);
        let conversion_b = if on_target_b > 0.0 { sum(before, |g| g.scored) / on_target_b } else { -1.0 };
        if !before.is_empty() && conceded_r >= conceded_b + 0.5 {
            let more = (shots_against_r > shots_against_b + 2.0).then_some(shots_against_b);
            d.push(item(Kind::Problem, Finding::DefenceLeaking { conceded: conceded_r, before: conceded_b, shots_against: shots_against_r, shots_against_before: more }));
        } else if conceded_r <= 0.6 {
            d.push(item(Kind::Strong, Finding::SolidDefence { conceded: conceded_r, games: recent.len() }));
        }
        if !before.is_empty() && goals_r <= goals_b - 0.6 {
            let aim = conversion_b > 0.0 && conversion < conversion_b * 0.7 && shots_r >= per_game(before, |g| g.shots) - 2.0;
            d.push(item(Kind::Problem, Finding::AttackFaded { goals: goals_r, before: goals_b, shots: shots_r, aim: aim.then_some((conversion * 100.0, conversion_b * 100.0)) }));
        } else if goals_r >= 2.2 {
            d.push(item(Kind::Strong, Finding::SharpAttack { goals: goals_r, games: recent.len() }));
        }
        let possession = per_game(recent, |g| g.possession);
        if possession >= 56.0 && a.ppg_recent < 1.5 {
            d.push(item(Kind::Attention, Finding::SterilePossession { possession }));
        }
        if level < 2 {
            d.push(locked(StaffRole::Analyst, 2, Unlock::AnalystDetails));
            return;
        }
        // level 2: when goals happen, cards, absences
        let (mut conceded, mut late, mut early, mut yellows, mut reds, mut lost) = (0, 0, 0, 0, 0, 0);
        for g in &a.window {
            let mut sent_off = false;
            for e in &world.game_match(g.id).events {
                let ours = e.club == Some(club);
                if e.kind == 1 && !ours {
                    conceded += 1;
                    let m = minute(e);
                    if m >= 75 {
                        late += 1;
                    } else if m <= 15 {
                        early += 1;
                    }
                } else if e.kind == 2 && ours {
                    yellows += 1;
                } else if (e.kind == 3 || e.kind == 4) && ours {
                    reds += 1;
                    sent_off = true;
                }
            }
            if sent_off {
                lost += 3 - g.points();
            }
        }
        let games = a.window.len();
        if conceded >= 4 && late as f64 >= conceded as f64 * 0.4 {
            d.push(item(Kind::Problem, Finding::LateCollapse { late, conceded, games }));
        }
        if conceded >= 4 && early as f64 >= conceded as f64 * 0.35 {
            d.push(item(Kind::Attention, Finding::BadStarts { early, conceded }));
        }
        if reds >= 2 || (reds >= 1 && lost >= 3) {
            d.push(item(Kind::Problem, Finding::CostlyRedCards { reds, games, points_lost: lost }));
        } else if yellows as f64 / games as f64 >= 2.5 {
            d.push(item(Kind::Attention, Finding::ManyCards { per_game: yellows as f64 / games as f64 }));
        }
        let regular = regulars(&a.window, 0.6);
        let (mut missing, mut defeats) = (0, 0);
        for g in recent {
            if g.points() == 0 {
                defeats += 1;
                missing += regular.iter().filter(|p| !g.starters.contains(p)).count() as i32;
            }
        }
        if defeats >= 2 && missing >= defeats * 2 {
            d.push(item(Kind::Problem, Finding::AbsencesInDefeats { defeats, average: missing as f64 / defeats as f64 }));
        }
        let today = world.today_millis();
        let out: Vec<String> = regular.iter().filter(|&&p| world.player(p).is_injured(today) && world.player(p).club == Some(club)).map(|&p| world.player(p).name.clone()).collect();
        if !out.is_empty() {
            d.push(item(Kind::Attention, Finding::InjuredStarters { players: out }));
        }
        if level < 3 {
            d.push(locked(StaffRole::Analyst, 3, Unlock::AnalystStrength));
            return;
        }
        // level 3: opponents and expected points (today's strength, so only the recent ones)
        let base = &a.window[..a.window.len().min(10)];
        let expected: f64 = base.iter().map(|g| expected_points(world, g)).sum();
        let points: i32 = base.iter().map(|g| g.points()).sum();
        a.expected = expected;
        a.expected_points = points;
        a.expected_games = base.len();
        let gap = points as f64 - expected;
        let rivals_r = recent.iter().map(|g| g.opponent_strength).sum::<f64>() / recent.len() as f64;
        let rivals_s = a.season.iter().map(|g| g.opponent_strength).sum::<f64>() / a.season.len() as f64;
        if gap <= -3.0 {
            d.push(item(Kind::Problem, Finding::Underperforming { expected, games: base.len(), points }));
        } else if gap >= 3.0 {
            d.push(item(Kind::Strong, Finding::Overperforming { points, games: base.len(), expected }));
        } else {
            d.push(item(Kind::Info, Finding::AsExpected { points, games: base.len(), expected }));
        }
        if rivals_r >= rivals_s + 3.0 {
            d.push(item(Kind::Attention, Finding::ToughRun { recent: rivals_r, season: rivals_s }));
        }
        let strength_r = recent.iter().map(|g| g.strength).sum::<f64>() / recent.len() as f64;
        let strength_s = a.season.iter().map(|g| g.strength).sum::<f64>() / a.season.len() as f64;
        if strength_r <= strength_s - 2.5 {
            d.push(item(Kind::Problem, Finding::WeakerLineups { recent: strength_r, season: strength_s }));
        }
        let home: Vec<Game> = a.season.iter().filter(|g| g.home).cloned().collect();
        let away: Vec<Game> = a.season.iter().filter(|g| !g.home).cloned().collect();
        if home.len() >= 3 && away.len() >= 3 && (ppg(&home) - ppg(&away)).abs() >= 0.8 {
            let (h, w) = (ppg(&home), ppg(&away));
            d.push(item(Kind::Attention, if h > w { Finding::WeakAway { home: h, away: w } } else { Finding::WeakHome { home: h, away: w } }));
        }
        if level < 4 {
            d.push(locked(StaffRole::Analyst, 4, Unlock::AnalystPlayers));
            return;
        }
        // level 4: players
        let mut used: HashSet<PlayerId> = HashSet::new();
        let mut changes = 0;
        for i in 0..recent.len() {
            used.extend(recent[i].starters.iter().copied());
            if i + 1 < recent.len() {
                changes += recent[i].starters.iter().filter(|p| !recent[i + 1].starters.contains(p)).count();
            }
        }
        let per = if recent.len() > 1 { changes as f64 / (recent.len() - 1) as f64 } else { 0.0 };
        if per >= 4.0 {
            d.push(item(Kind::Attention, Finding::LowCohesion { changes: per, players: used.len(), games: recent.len() }));
        }
        let mut scorers: Vec<(PlayerId, i32)> = Vec::new();
        let mut total = 0;
        for g in &a.season {
            for e in &world.game_match(g.id).events {
                if e.kind == 1 && e.club == Some(club)
                    && let Some(p) = e.player
                {
                    match scorers.iter_mut().find(|(x, _)| *x == p) {
                        Some(s) => s.1 += 1,
                        None => scorers.push((p, 1)),
                    }
                    total += 1;
                }
            }
        }
        let mut top: Option<(PlayerId, i32)> = None;
        for &(p, n) in &scorers {
            if top.is_none_or(|t| n > t.1) {
                top = Some((p, n));
            }
        }
        if let Some((p, n)) = top
            && total >= 8
            && n as f64 >= total as f64 * 0.4
        {
            d.push(item(Kind::Attention, Finding::ScorerDependence { player: world.player(p).name.clone(), goals: n, total }));
        }
        let mut ratings: Vec<(PlayerId, f64)> = Vec::new();
        for p in regulars(&a.window, 0.5) {
            let r = season_numbers(world, p).2;
            if r > 0.0 {
                ratings.push((p, r));
            }
        }
        if !ratings.is_empty() {
            let average = ratings.iter().map(|r| r.1).sum::<f64>() / ratings.len() as f64;
            let slump: Vec<(String, f64)> = ratings.iter().filter(|r| r.1 <= average - 0.5).map(|&(p, r)| (world.player(p).name.clone(), r)).collect();
            if !slump.is_empty() {
                d.push(item(Kind::Attention, Finding::PlayersInSlump { average, players: slump }));
            }
        }
        for p in regulars(recent, 0.8) {
            if world.player(p).position == 4 && !scorers.iter().any(|s| s.0 == p) && a.season.len() >= 5 {
                d.push(item(Kind::Attention, Finding::StrikerWithoutGoals { player: world.player(p).name.clone() }));
            }
        }
        if level < 5 {
            d.push(locked(StaffRole::Analyst, 5, Unlock::AnalystSeason));
        }
    }

    fn suggestions(&mut self, a: &mut Analysis, club: ClubId) {
        let level = a.assistant;
        if level < 1 {
            a.suggestions.push(locked(StaffRole::Assistant, 1, Unlock::AssistantSuggestions));
            return;
        }
        let starters: Vec<PlayerId> = self.world.club(club).squad.iter().copied().filter(|&p| is_starter(self.world, p)).collect();
        if individual(self.world) && !starters.is_empty() {
            let world = &*self.world;
            let (mut total, mut count) = ([0.0f64; 8], [0usize; 8]);
            for &p in &starters {
                let k = slot_kind(world.player(p).slot);
                total[k] += strength(world, p) as f64;
                count[k] += 1;
            }
            let mut worst: Option<usize> = None;
            for k in 0..8 {
                if count[k] > 0 && worst.is_none_or(|w| total[k] / (count[k] as f64) < total[w] / count[w] as f64) {
                    worst = Some(k);
                }
            }
            if let Some(w) = worst {
                a.suggestions.push(item(Kind::Info, Finding::ReinforceSlot { kind: w, slot: total[w] / count[w] as f64, team: average_strength(world, &starters) }));
            }
        } else {
            let position = self.urgency(club)[0];
            let world = &*self.world;
            a.suggestions.push(item(Kind::Info, Finding::Reinforce { position: position as i32, starters: starters_average(world, club, position), team: team_average(world, club) }));
        }
        if level < 2 {
            a.suggestions.push(locked(StaffRole::Assistant, 2, Unlock::AssistantSwaps));
            return;
        }
        let world = &*self.world;
        let today = world.today_millis();
        let competition = a.next.as_ref().and_then(|n| n.competition);
        let mut order = starters.clone();
        order.sort_by_key(|&p| strength(world, p));
        let mut swaps = 0;
        let mut suggested: HashSet<PlayerId> = HashSet::new();
        for &starter in &order {
            let slot = world.player(starter).slot;
            let current = value_in(world, starter, slot);
            let mut best: Option<PlayerId> = None;
            for &p in &world.club(club).squad {
                if is_starter(world, p) || suggested.contains(&p) || world.player(p).is_injured(today) || suspended_in(world, p, competition) {
                    continue;
                }
                if value_in(world, p, slot) >= current + 3 && best.is_none_or(|b| value_in(world, p, slot) > value_in(world, b, slot)) {
                    best = Some(p);
                }
            }
            if let Some(b) = best
                && swaps < 3
            {
                swaps += 1;
                suggested.insert(b);
                let screen = (individual(world) && world.player(b).strength < world.player(starter).strength).then(|| (world.player(b).strength, world.player(starter).strength));
                a.suggestions.push(item(
                    Kind::Attention,
                    Finding::PlayInSlot { player: world.player(b).name.clone(), kind: slot_kind(slot), value: value_in(world, b, slot), starter: world.player(starter).name.clone(), current, screen },
                ));
            }
        }
        let mut misplaced = 0;
        for &starter in &order {
            let slot = world.player(starter).slot;
            let current = value_in(world, starter, slot);
            let kind = best_kind(world, starter);
            let best = best_value(world, starter);
            if misplaced < 2 && kind != slot_kind(slot) && best >= current + 8 {
                misplaced += 1;
                a.suggestions.push(item(Kind::Attention, Finding::OutOfPlace { player: world.player(starter).name.clone(), value: current, kind: slot_kind(slot), best_kind: kind, best }));
            }
        }
        if swaps == 0 && misplaced == 0 {
            a.suggestions.push(item(Kind::Strong, Finding::RightLineup));
        }
        if level < 3 {
            a.suggestions.push(locked(StaffRole::Assistant, 3, Unlock::AssistantAdjustments));
            return;
        }
        let mut adjustments = Vec::new();
        for d in &a.diagnosis {
            let f = match d.finding {
                Finding::LateCollapse { .. } => Finding::ChangeEarlier,
                Finding::DefenceLeaking { .. } => Finding::ProtectDefence,
                Finding::AttackFaded { .. } => Finding::FocusAttacks,
                Finding::SterilePossession { .. } => Finding::MoreDirect,
                Finding::CostlyRedCards { .. } | Finding::ManyCards { .. } => Finding::LessAggressive,
                Finding::LowCohesion { .. } => Finding::SettleTeam,
                _ => continue,
            };
            adjustments.push(item(Kind::Info, f));
        }
        a.suggestions.extend(adjustments);
        if level < 4 {
            a.suggestions.push(locked(StaffRole::Assistant, 4, Unlock::AssistantYoungAndOld));
            return;
        }
        let regular = regulars(&a.window, 0.5);
        let team = average_strength(world, &starters);
        for &p in &world.club(club).squad {
            let x = world.player(p);
            let of_position: Vec<PlayerId> = starters.iter().copied().filter(|&s| world.player(s).position == x.position).collect();
            let position_average = average_strength(world, &of_position);
            let value = best_value(world, p);
            if x.age <= 21 && !regular.contains(&p) && position_average > 0.0 && value as f64 >= position_average - 3.0 {
                a.suggestions.push(item(Kind::Info, Finding::GiveMinutes { player: x.name.clone(), age: x.age, value }));
            }
            if x.age >= 33 && regular.contains(&p) && (strength(world, p) as f64) < team - 3.0 {
                a.suggestions.push(item(Kind::Attention, Finding::PlanExit { player: x.name.clone(), age: x.age, strength: strength(world, p) }));
            }
        }
        if level < 5 {
            a.suggestions.push(locked(StaffRole::Assistant, 5, Unlock::AssistantGamePlan));
            return;
        }
        if let Some(next) = &a.next
            && !self.sys.config.on(Switch::Tactics)
        {
            let diff = next.strength - next.rival_strength;
            let plan = if diff >= 4.0 {
                0
            } else if diff <= -4.0 {
                1
            } else {
                2
            };
            a.suggestions.push(item(Kind::Info, Finding::GamePlan { rival: next.rival, plan }));
        }
    }

    /// `Desempenho.conselhoTatico(Analise, ah)`
    fn tactic_advice(&mut self, club: ClubId, next: &NextGame, analyst: i32) -> Item {
        let world = &*self.world;
        let seed = 31 * world.club(next.rival).id as i64 + next.date / DAY_MILLIS;
        let (mine, theirs) = (likely_eleven(world, club), likely_eleven(world, next.rival));
        let fit = brasfoot_engine::tactics::fit_of(world, &mine);
        let p = if next.home { measure(world, Some(club), Some(next.rival), mine, theirs) } else { measure(world, Some(next.rival), Some(club), theirs, mine) };
        let advice: Advice = advise(world, &p, if next.home { 0 } else { 1 }, analyst, seed);
        let beats = advice.predicted_rival.and_then(|r| {
            if beats(advice.best[0], r[0]) {
                Some(true)
            } else if beats(r[0], advice.best[0]) {
                Some(false)
            } else {
                None
            }
        });
        use brasfoot_engine::tactics::{ALL_OUT_ATTACK, COUNTER_ATTACK};
        let t = TacticAdvice {
            best: advice.best,
            predicted: advice.predicted_rival,
            full_guess: analyst >= 5,
            beats,
            fast_forwards: advice.best[0] == COUNTER_ATTACK && fit[1] >= 1.0,
            strong_midfield: advice.best[0] == ALL_OUT_ATTACK && fit[0] >= 1.0,
            tough_defence: advice.best[1] > 0 && fit[2] >= 1.0,
            automatic: crate::tactics::automatic(world, &self.sys.config),
        };
        item(Kind::Info, Finding::Tactic { rival: next.rival, advice: t })
    }

    fn medical(&mut self, a: &mut Analysis, club: ClubId) {
        let world = &*self.world;
        let level = a.physio;
        let today = world.today_millis();
        let m = &mut a.medical;
        let injured: Vec<(String, Option<i64>)> =
            world.club(club).squad.iter().filter(|&&p| world.player(p).is_injured(today)).map(|&p| (world.player(p).name.clone(), (level >= 1).then_some(world.player(p).injured_until))).collect();
        m.push(if injured.is_empty() { item(Kind::Strong, Finding::NoInjured) } else { item(Kind::Attention, Finding::Injured { players: injured }) });
        if level < 1 {
            m.push(locked(StaffRole::FitnessCoach, 1, Unlock::PhysioReturn));
            return;
        }
        if level < 2 {
            m.push(locked(StaffRole::FitnessCoach, 2, Unlock::PhysioHistory));
            return;
        }
        let mut injuries: Vec<(String, i32)> = Vec::new();
        for g in &a.season {
            for e in &world.game_match(g.id).events {
                if e.kind == 5 && e.club == Some(club)
                    && let Some(p) = e.player
                {
                    let name = world.player(p).name.clone();
                    match injuries.iter_mut().find(|(n, _)| *n == name) {
                        Some(x) => x.1 += 1,
                        None => injuries.push((name, 1)),
                    }
                }
            }
        }
        let total: i32 = injuries.iter().map(|x| x.1).sum();
        let prone: Vec<(String, i32)> = injuries.into_iter().filter(|x| x.1 >= 2).collect();
        let problem = total >= a.season.len() as i32 / 4 + 2;
        m.push(item(if problem { Kind::Problem } else { Kind::Info }, Finding::InjuryHistory { total, prone }));
        if level < 3 {
            m.push(locked(StaffRole::FitnessCoach, 3, Unlock::PhysioSuspensions));
            return;
        }
        if let Some(next) = &a.next
            && let Some(c) = next.competition
        {
            let (mut suspended, mut booked) = (Vec::new(), Vec::new());
            for &p in &world.club(club).squad {
                if suspended_in(world, p, Some(c)) {
                    suspended.push(world.player(p).name.clone());
                } else if yellows_in(world, p, c) == Some(2) {
                    booked.push(world.player(p).name.clone());
                }
            }
            let kind = if suspended.is_empty() { Kind::Strong } else { Kind::Attention };
            let first = booked.first().cloned();
            let more = booked.len() > 1;
            m.push(item(kind, Finding::NextMatch { stage: next.stage, competition: next.competition, suspended, booked }));
            if level >= 4
                && let Some(first) = first
            {
                m.push(item(Kind::Info, Finding::RestBooked { first, more }));
            }
        }
        if level < 4 {
            m.push(locked(StaffRole::FitnessCoach, 4, Unlock::PhysioRest));
            return;
        }
        if level >= 5 {
            let limit = today + 14 * DAY_MILLIS;
            let mut games = 0;
            'days: for day in world.game.days.iter().skip(world.game.day.max(0) as usize) {
                if day.date > limit {
                    break 'days;
                }
                for &mm in &day.matches {
                    let g = world.game_match(mm);
                    if (g.home == Some(club) || g.away == Some(club)) && !g.played {
                        games += 1;
                    }
                }
            }
            if games >= 4 {
                m.push(item(Kind::Attention, Finding::TightSchedule { games }));
            }
        } else {
            m.push(locked(StaffRole::FitnessCoach, 5, Unlock::PhysioSchedule));
        }
    }
}

/// `Taticas.vence(int, int)`: the cycle of styles.
fn beats(a: i32, b: i32) -> bool {
    use brasfoot_engine::tactics::{ALL_OUT_ATTACK, BALANCED, COUNTER_ATTACK};
    (a == ALL_OUT_ATTACK && b == BALANCED) || (a == BALANCED && b == COUNTER_ATTACK) || (a == COUNTER_ATTACK && b == ALL_OUT_ATTACK)
}
