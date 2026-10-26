// Port of bfmod.CampeonatoBase: the national U-20 championship, a double round-robin among the
// clubs of the country's top flight (always with the player's club), on the senior league's
// match days, played with the real match engine (c.b with the tactics) on the clubs' youth
// squads (ah.ky()), stand-ins of strength 16 filling the gaps. Its stars get a development bonus,
// the podium prize money, and the juniors who play develop faster. The newspaper's story is not
// ported.

use std::collections::{HashMap, HashSet};

use brasfoot_engine::engine::MatchEngine;
use brasfoot_engine::tables::FORMATIONS;
use brasfoot_engine::tactics::best_value;
use brasfoot_engine::value::mod_value;
use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::mods::{FanEvent, YouthLeagueChampion, YouthLeagueGoal, YouthLeagueMatch, YouthLeagueStats, YouthLeagueState};
use brasfoot_model::saved::{CompetitionKind, Match, Player};
use brasfoot_model::world::World;

use crate::awards::season::player_key;
use crate::config::Switch;
use crate::economy::{clubs_by_id, year};
use crate::staff::my_club;
use crate::{Mod, sorte};

/// `FORMACOES`: the formations a youth XI is tried in.
const FORMATIONS_TRIED: [usize; 5] = [4, 2, 9, 5, 7];
/// The positions of the stand-ins after a goalkeeper (`posPadrao`).
const STAND_IN_POSITIONS: [i32; 11] = [2, 1, 3, 4, 2, 3, 1, 4, 3, 2, 4];
const CHAMPION_PRIZE: i64 = 1_500_000;
const RUNNER_UP_PRIZE: i64 = 750_000;
const THIRD_PRIZE: i64 = 350_000;

/// `CampeonatoBase.LinhaTabela`
#[derive(Clone, Debug, Default)]
pub struct TableRow {
    pub club: i32,
    pub name: String,
    pub country: i32,
    pub points: i32,
    pub games: i32,
    pub wins: i32,
    pub draws: i32,
    pub losses: i32,
    pub goals_for: i32,
    pub goals_against: i32,
}

impl TableRow {
    pub fn goal_difference(&self) -> i32 {
        self.goals_for - self.goals_against
    }
}

/// `String.compareToIgnoreCase`
fn compare_ignore_case(a: &str, b: &str) -> std::cmp::Ordering {
    let fold = |c: u16| -> u32 {
        let ch = char::from_u32(c as u32).unwrap_or('\u{FFFD}');
        let upper = ch.to_uppercase().next().filter(|_| ch.to_uppercase().count() == 1).unwrap_or(ch);
        let lower = upper.to_lowercase().next().filter(|_| upper.to_lowercase().count() == 1).unwrap_or(upper);
        lower as u32
    };
    let (x, y): (Vec<u16>, Vec<u16>) = (a.encode_utf16().collect(), b.encode_utf16().collect());
    for (&c1, &c2) in x.iter().zip(&y) {
        if c1 != c2 {
            let (l1, l2) = (fold(c1), fold(c2));
            if l1 != l2 {
                return l1.cmp(&l2);
            }
        }
    }
    x.len().cmp(&y.len())
}

/// `tabelaInterna()`: the table of the played matches.
pub fn table(world: &World) -> Vec<TableRow> {
    let s = &world.mods.youth_league;
    let by_id = clubs_by_id(world);
    let mut rows: Vec<TableRow> = s
        .clubs
        .iter()
        .map(|(id, name)| {
            let club = by_id.get(id).copied();
            TableRow {
                club: *id,
                name: club.map(|c| world.club(c).name.clone()).unwrap_or_else(|| name.clone()),
                country: club.map(|c| world.club(c).country).unwrap_or(s.country),
                ..Default::default()
            }
        })
        .collect();
    for m in s.matches.iter().filter(|m| m.played) {
        for (id, scored, conceded) in [(m.home, m.home_goals, m.away_goals), (m.away, m.away_goals, m.home_goals)] {
            // LinkedHashMap.get: the first row of that id
            let Some(r) = rows.iter_mut().find(|r| r.club == id) else { continue };
            r.games += 1;
            r.goals_for += scored;
            r.goals_against += conceded;
            if scored > conceded {
                r.wins += 1;
                r.points += 3;
            } else if scored == conceded {
                r.draws += 1;
                r.points += 1;
            } else {
                r.losses += 1;
            }
        }
    }
    // the map keeps one row per id
    let mut seen = HashSet::new();
    rows.retain(|r| seen.insert(r.club));
    rows.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.wins.cmp(&a.wins))
            .then(b.goal_difference().cmp(&a.goal_difference()))
            .then(b.goals_for.cmp(&a.goals_for))
            .then_with(|| compare_ignore_case(&a.name, &b.name))
    });
    rows
}

/// `totalRodadasInterno()`
pub fn total_rounds(s: &YouthLeagueState) -> i32 {
    s.matches.iter().map(|m| m.round).max().unwrap_or(0).max(0)
}

/// `rodadasDisputadasInterno()`
pub fn rounds_played(s: &YouthLeagueState) -> i32 {
    s.matches.iter().filter(|m| m.played).map(|m| m.round).max().unwrap_or(0).max(0)
}

/// `jogosDisputados(String)`: a junior's youth league games this season.
pub fn games_played(world: &World, key: &str) -> i32 {
    world.mods.youth_league.stats.iter().find(|e| e.key == key).map(|e| e.games).unwrap_or(0)
}

/// The XI of `escalarEquipe`: the players, their slots, and which are real.
struct YouthXi {
    players: Vec<PlayerId>,
    slots: Vec<i32>,
    real: Vec<PlayerId>,
}

impl Mod<'_> {
    fn youth_league(&mut self) -> &mut YouthLeagueState {
        &mut self.world.mods.youth_league
    }

    fn youth_league_on(&self) -> bool {
        self.sys.config.on(Switch::Youth) && self.sys.config.on(Switch::YouthLeague)
    }

    /// `CampeonatoBase.tick()`: a new season's league, the rounds whose senior matches were
    /// played, and the end.
    pub(crate) fn youth_league_tick(&mut self) {
        if !self.youth_league_on() {
            return;
        }
        let Some(mine) = my_club(self.world) else { return };
        let season = self.world.season();
        let country = self.world.club(mine).country;
        let s = &self.world.mods.youth_league;
        if s.season != season || s.country != country || s.matches.is_empty() {
            if s.season > 0 && s.season != season && !s.finished && rounds_played(s) > 0 {
                self.finish_youth_league(false);
            }
            self.start_youth_league(season, country, mine);
        }
        self.drop_old_priority_starters(mine);
        self.play_due_youth_rounds(mine);
        let s = &self.world.mods.youth_league;
        if !s.finished && total_rounds(s) > 0 && rounds_played(s) >= total_rounds(s) {
            self.finish_youth_league(true);
        }
    }

    /// `CampeonatoBase.fimDeTemporada()`: what is left is played before the champion is crowned.
    pub(crate) fn youth_league_season_end(&mut self) {
        if !self.youth_league_on() {
            return;
        }
        let Some(mine) = my_club(self.world) else { return };
        let s = &self.world.mods.youth_league;
        if s.season == self.world.season() && !s.finished && !s.matches.is_empty() {
            for i in 0..self.world.mods.youth_league.matches.len() {
                if !self.world.mods.youth_league.matches[i].played {
                    self.play_youth_match(i, mine);
                }
            }
            self.finish_youth_league(true);
        }
    }

    /// `iniciarTemporada(int, int, ah)`: the clubs, and the rounds from the senior league's days.
    fn start_youth_league(&mut self, season: i32, country: i32, mine: ClubId) {
        let participants = self.youth_league_clubs(country, mine);
        let clubs: Vec<(i32, String)> = participants.iter().map(|&t| (self.world.club(t).id, self.world.club(t).name.clone())).collect();
        let ids: HashSet<i32> = clubs.iter().map(|c| c.0).collect();
        // a club of the second division plays in place of the first division's club it replaced
        let mut replaced = None;
        if self.world.club(mine).division != 1 {
            replaced = self.world.game.clubs.iter().copied().find(|&t| {
                let c = self.world.club(t);
                c.country == country && c.division == 1 && !clubs.iter().any(|x| x.0 == c.id)
            });
        }
        let mut matches = Vec::new();
        let mut round = 0;
        for (d, day) in self.world.game.days.iter().enumerate() {
            let of_day: Vec<MatchId> = day
                .matches
                .iter()
                .copied()
                .filter(|&m| {
                    let g = self.world.game_match(m);
                    let (Some(home), Some(_)) = (g.home, g.away) else { return false };
                    let h = self.world.club(home);
                    h.country == country && h.division == 1 && g.competition.is_some_and(|c| self.world.competition(c).class == CompetitionKind::NationalLeague)
                })
                .collect();
            if of_day.is_empty() {
                continue;
            }
            round += 1;
            let f = self.runtime.clock.fields(day.date);
            let date = (f.day as i32, f.month as i32 + 1, f.year);
            for m in of_day {
                let g = self.world.game_match(m);
                let swap = |c: ClubId| if Some(c) == replaced { mine } else { c };
                let (home, away) = (swap(g.home.unwrap()), swap(g.away.unwrap()));
                let (h, a) = (self.world.club(home), self.world.club(away));
                if !ids.contains(&h.id) || !ids.contains(&a.id) {
                    continue;
                }
                matches.push(YouthLeagueMatch {
                    round,
                    day: d as i32,
                    date,
                    home: h.id,
                    away: a.id,
                    home_name: h.name.clone(),
                    away_name: a.name.clone(),
                    played: false,
                    home_goals: 0,
                    away_goals: 0,
                    home_shots: 0,
                    away_shots: 0,
                    home_possession: 50,
                    goals: Vec::new(),
                });
            }
        }
        // the round-robin of its own when the calendar has no league fixtures yet
        if matches.is_empty() && participants.len() >= 2 {
            matches = self.default_youth_fixtures(&participants);
        }
        let s = self.youth_league();
        s.season = season;
        s.country = country;
        s.finished = false;
        s.clubs = clubs;
        s.matches = matches;
        s.stats.clear();
    }

    /// `escolherClubes(int, ah)`: the country's first division (the whole country with fewer
    /// than four), the player's club in place of the weakest.
    fn youth_league_clubs(&self, country: i32, mine: ClubId) -> Vec<ClubId> {
        let world = &*self.world;
        let mut first: Vec<ClubId> = world.game.clubs.iter().copied().filter(|&t| world.club(t).country == country && world.club(t).division == 1).collect();
        if first.len() < 4 {
            for &t in &world.game.clubs {
                if world.club(t).country == country && !first.contains(&t) {
                    first.push(t);
                }
            }
        }
        if !first.contains(&mine) {
            first.sort_by_key(|&t| world.club(t).reputation * 100 + world.club(t).level);
            if first.is_empty() {
                first.push(mine);
            } else {
                first[0] = mine;
            }
        }
        first
    }

    /// `gerarTabelaPadrao(List)`: a double round-robin by rotation, a week apart.
    fn default_youth_fixtures(&self, clubs: &[ClubId]) -> Vec<YouthLeagueMatch> {
        let mut l = clubs.to_vec();
        if l.len() % 2 != 0 {
            l.pop();
        }
        let n = l.len();
        let rounds = n - 1;
        let year = year(self.world);
        let mut out = Vec::new();
        for t in 0..2 {
            let mut rot = l.clone();
            for r in 0..rounds {
                let number = (t * rounds + r + 1) as i32;
                let day = 350.min(60 + number * 7);
                let date = (day % 28 + 1, 12.min(day / 30 + 1), year);
                for i in 0..n / 2 {
                    let (a, b) = (rot[i], rot[n - 1 - i]);
                    let home = if (r + i + t) % 2 == 0 { a } else { b };
                    let away = if home == a { b } else { a };
                    let (h, w) = (self.world.club(home), self.world.club(away));
                    out.push(YouthLeagueMatch {
                        round: number,
                        day,
                        date,
                        home: h.id,
                        away: w.id,
                        home_name: h.name.clone(),
                        away_name: w.name.clone(),
                        played: false,
                        home_goals: 0,
                        away_goals: 0,
                        home_shots: 0,
                        away_shots: 0,
                        home_possession: 50,
                        goals: Vec::new(),
                    });
                }
                let last = rot.pop().expect("a rotation");
                rot.insert(1, last);
            }
        }
        out
    }

    /// `simularRodadasPendentes(ah)`: the matches of the days gone by, and of today once a
    /// senior league match of the country was played.
    fn play_due_youth_rounds(&mut self, mine: ClubId) {
        let today = self.world.game.day;
        let country = self.world.mods.youth_league.country;
        let mut done: HashMap<i32, bool> = HashMap::new();
        for i in 0..self.world.mods.youth_league.matches.len() {
            let m = &self.world.mods.youth_league.matches[i];
            if m.played {
                continue;
            }
            let day = m.day;
            let due = *done.entry(day).or_insert_with(|| {
                if day < today {
                    return true;
                }
                if day == today && day >= 0 && (day as usize) < self.world.game.days.len() {
                    return self.world.game.days[day as usize].matches.iter().any(|&m| {
                        let g = self.world.game_match(m);
                        g.home.is_some_and(|h| self.world.club(h).country == country)
                            && g.competition.is_some_and(|c| self.world.competition(c).class == CompetitionKind::NationalLeague)
                            && g.played
                    });
                }
                false
            });
            if due {
                self.play_youth_match(i, mine);
            }
        }
    }

    /// `jogarPartida(JogoBase, ah)`: the engine on two youth XIs, minute by minute; the
    /// senior side of things (slots, tactics, the human flag) put back after.
    fn play_youth_match(&mut self, index: usize, mine: ClubId) {
        let by_id = clubs_by_id(self.world);
        let (home_id, away_id) = {
            let m = &self.world.mods.youth_league.matches[index];
            (m.home, m.away)
        };
        let (Some(&home), Some(&away)) = (by_id.get(&home_id), by_id.get(&away_id)) else {
            self.youth_league().matches[index].played = true;
            return;
        };
        let players_before = self.world.players.len();
        let home_xi = self.youth_xi(home, home == mine);
        let away_xi = self.youth_xi(away, away == mine);
        let stand_ins: HashSet<PlayerId> = (players_before..self.world.players.len()).map(|i| PlayerId(i as u32)).collect();
        let mut original_slots: Vec<(PlayerId, i32)> = Vec::new();
        for &p in home_xi.players.iter().chain(&away_xi.players) {
            original_slots.push((p, self.world.player(p).slot));
        }
        let tactics = [self.world.club(home).tactics.clone(), self.world.club(away).tactics.clone()];
        let humans = [self.world.club(home).human, self.world.club(away).human];

        // new I(): the clubs and the first league competition of the calendar
        let competition = self.first_league_competition();
        let id = MatchId(self.world.matches.len() as u32);
        self.world.matches.push(Match { home: Some(home), away: Some(away), competition, ..Default::default() });
        for (i, &p) in home_xi.players.iter().enumerate() {
            self.world.player_mut(p).slot = home_xi.slots[i];
            let g = self.world.game_match_mut(id);
            g.home_starters.push(p);
            g.home_played.push(p);
        }
        for (i, &p) in away_xi.players.iter().enumerate() {
            self.world.player_mut(p).slot = away_xi.slots[i];
            let g = self.world.game_match_mut(id);
            g.away_starters.push(p);
            g.away_played.push(p);
        }
        // Taticas.inicio picks the AI tactics of both sides
        self.world.club_mut(home).human = false;
        self.world.club_mut(away).human = false;
        let mut engine = MatchEngine::new(self.world, self.runtime, self.sys, id);
        // the player's own youth tactic, unless automatic
        let style = self.world.mods.youth_league.style;
        if (home == mine || away == mine) && style >= 0 {
            let side = if home == mine { home } else { away };
            let (marking, wings) = (self.world.mods.youth_league.marking, self.world.mods.youth_league.side);
            let c = self.world.club_mut(side);
            c.human = true;
            if c.tactics.len() >= 4 {
                c.tactics[1] = style;
                c.tactics[2] = marking;
                c.tactics[3] = wings;
            }
        }
        let first_half = 45 + sorte!(self, "CampeonatoBase").next_int_bounded(3);
        let second_half = 45 + 1 + sorte!(self, "CampeonatoBase").next_int_bounded(5);
        let mut goals = Vec::new();
        let mut scored: Vec<(PlayerId, i32)> = Vec::new();
        let mut assisted: Vec<(PlayerId, i32)> = Vec::new();
        let add = |list: &mut Vec<(PlayerId, i32)>, p: PlayerId| match list.iter_mut().find(|(x, _)| *x == p) {
            Some(e) => e.1 += 1,
            None => list.push((p, 1)),
        };
        for minute in 1..=first_half + second_half {
            let Some(ev) = engine.tick(self.world, self.runtime, self.sys) else { continue };
            if ev.kind != 1 || ev.flag_dg {
                continue;
            }
            let own_goal = ev.detail == 2;
            let real = |p: Option<PlayerId>| p.filter(|p| !stand_ins.contains(p));
            if let Some(p) = real(ev.player)
                && !own_goal
            {
                add(&mut scored, p);
            }
            if let Some(p) = real(ev.other_player)
                && !own_goal
            {
                add(&mut assisted, p);
            }
            let club = if ev.value_df == 0 { self.world.club(home).name.clone() } else { self.world.club(away).name.clone() };
            goals.push(YouthLeagueGoal {
                minute,
                scorer: real(ev.player).map(|p| self.world.player(p).name.clone()),
                kind: ev.detail,
                assist: real(ev.other_player).filter(|_| !own_goal).map(|p| self.world.player(p).name.clone()),
                club,
            });
        }
        let score = engine.goals;
        let g = self.world.game_match(id);
        let shots = [g.shots[0], g.shots[1]];
        let (p0, p1) = (g.tackles[0] + g.values_gc[0], g.tackles[1] + g.values_gc[1]);
        let possession = if p0 + p1 > 0 { brasfoot_model::rng::java_round(p0 as f64 * 100.0 / (p0 + p1) as f64) as i32 } else { 50 };
        {
            let m = &mut self.youth_league().matches[index];
            m.goals = goals;
            m.home_goals = score[0];
            m.away_goals = score[1];
            m.home_shots = shots[0];
            m.away_shots = shots[1];
            m.home_possession = possession;
            m.played = true;
        }
        self.record_youth_games(home, &home_xi.real, score[0], score[1], &scored, &assisted);
        self.record_youth_games(away, &away_xi.real, score[1], score[0], &scored, &assisted);
        // the finally block
        for (p, slot) in original_slots {
            self.world.player_mut(p).slot = slot;
        }
        for (side, club) in [home, away].into_iter().enumerate() {
            let c = self.world.club_mut(club);
            let n = tactics[side].len().min(c.tactics.len());
            c.tactics[..n].copy_from_slice(&tactics[side][..n]);
            c.human = humans[side];
        }
        // the match and the stand-ins were never part of the game
        self.world.matches.truncate(id.0 as usize);
        for p in &stand_ins {
            self.runtime.match_stats.remove(p);
        }
        self.world.players.truncate(players_before);
    }

    /// `encontrarCompeticaoLiga()`: the first national league of the calendar, or else the
    /// first competition.
    fn first_league_competition(&self) -> Option<brasfoot_model::ids::CompetitionId> {
        let days = &self.world.game.days;
        let league = days.iter().flat_map(|d| &d.matches).find_map(|&m| {
            self.world.game_match(m).competition.filter(|&c| self.world.competition(c).class == CompetitionKind::NationalLeague)
        });
        league.or_else(|| days.iter().flat_map(|d| &d.matches).find_map(|&m| self.world.game_match(m).competition))
    }

    /// `escalarEquipe(ah, boolean, ...)`: the academy kids, their attributes aligned, the
    /// player's priority starters first, the gaps (or a missing goalkeeper) filled with
    /// stand-ins; the best of the formations tried.
    fn youth_xi(&mut self, club: ClubId, mine: bool) -> YouthXi {
        let first_stand_in = self.world.players.len();
        let mut candidates: Vec<PlayerId> = Vec::new();
        let mut keeper = false;
        for p in self.world.club(club).youth_squad.clone() {
            self.sync_attributes_now(p);
            candidates.push(p);
            if self.world.player(p).position == 0 {
                keeper = true;
            }
        }
        if !keeper {
            candidates.push(self.stand_in(club, 0));
        }
        let mut next = 0;
        while candidates.len() < 11 {
            candidates.push(self.stand_in(club, STAND_IN_POSITIONS[next % STAND_IN_POSITIONS.len()]));
            next += 1;
        }
        let world = &*self.world;
        let priority: HashSet<String> = if mine { world.mods.youth_league.priority_starters.iter().cloned().collect() } else { HashSet::new() };
        let stand_in = |p: PlayerId| p.0 as usize >= first_stand_in;
        let prioritised = |p: PlayerId| mine && !stand_in(p) && priority.contains(&player_key(world, p));
        let individual = world.game.individual_skills;
        let mut best_score = f64::NEG_INFINITY;
        let mut best: Option<(Vec<i32>, [i32; 11])> = None;
        for f in FORMATIONS_TRIED {
            let slots = FORMATIONS[f];
            let choice = assign(world, &candidates, &slots, &|p| prioritised(p), &|p| stand_in(p), individual);
            let mut sum = 0.0;
            for (i, &c) in choice.iter().enumerate() {
                if c >= 0 {
                    let p = candidates[c as usize];
                    sum += mod_value(individual, world.player(p), slots[i]) as f64;
                    if prioritised(p) {
                        sum += 200.0;
                    }
                    if !stand_in(p) {
                        sum += 25.0;
                    }
                }
            }
            if sum > best_score {
                best_score = sum;
                best = Some((choice, slots));
            }
        }
        let (choice, slots) = best.expect("five formations");
        let mut xi = YouthXi { players: Vec::new(), slots: Vec::new(), real: Vec::new() };
        for (i, &slot) in slots.iter().enumerate() {
            let p = candidates[choice[i] as usize];
            xi.players.push(p);
            xi.slots.push(slot);
            if !stand_in(p) {
                xi.real.push(p);
            }
        }
        xi
    }

    /// `Base.sincronizarAtributos(F)`: to the academy's target strength, or his own.
    fn sync_attributes_now(&mut self, p: PlayerId) {
        let mut target = self.world.player(p).strength;
        if let Some(kids) = &self.world.mods.academy {
            let key = player_key(self.world, p);
            if let Some(kid) = kids.iter().find(|k| k.key == key) {
                target = crate::academy::target_strength(kid);
            }
        }
        self.sync_attributes(p, target);
    }

    /// `criarAspirante(ah, int)`: a stand-in of the club ("Aspirante"), 16 years old, strength
    /// 16, every attribute 10; never in the club's lists, and gone after the match.
    fn stand_in(&mut self, club: ClubId, position: i32) -> PlayerId {
        let mut p = Player { country: self.world.club(club).country, club: Some(club), position, ..Default::default() };
        p.set_age(16);
        p.side = 0;
        p.youth = true;
        p.strength = 16;
        p.attributes = [10; 7];
        let id = PlayerId(self.world.players.len() as u32);
        self.world.players.push(p);
        id
    }

    /// `registrarAtuacoes(ah, List, int, int, Map, Map)`: each real junior's game, goals,
    /// assists and a rating.
    fn record_youth_games(&mut self, club: ClubId, real: &[PlayerId], scored: i32, conceded: i32, goals: &[(PlayerId, i32)], assists: &[(PlayerId, i32)]) {
        let balance = scored - conceded;
        for &p in real {
            let key = player_key(self.world, p);
            let g = goals.iter().find(|(x, _)| *x == p).map(|e| e.1).unwrap_or(0);
            let a = assists.iter().find(|(x, _)| *x == p).map(|e| e.1).unwrap_or(0);
            let mut rating = 6.0;
            if balance > 0 {
                rating += 0.45 + 0.45f64.min(balance as f64 * 0.15);
            } else if balance < 0 {
                rating -= 0.35 + 0.45f64.min(-balance as f64 * 0.15);
            }
            let player = self.world.player(p);
            if player.position <= 2 && conceded == 0 {
                rating += 0.55;
            }
            rating += g as f64 * 1.15 + a as f64 * 0.70;
            rating += (best_value(self.world, p) - 28) as f64 * 0.015;
            rating += (sorte!(self, "CampeonatoBase").next_double() - 0.5) * 0.5;
            let rating = rating.clamp(4.0, 10.0);
            let player = self.world.player(p);
            let (name, country, position, age) = (player.name.clone(), player.country, player.position, player.age);
            let (club_id, club_name) = (self.world.club(club).id, self.world.club(club).name.clone());
            let stats = &mut self.youth_league().stats;
            let i = match stats.iter().position(|e| e.key == key) {
                Some(i) => i,
                None => {
                    stats.push(YouthLeagueStats {
                        key: key.clone(),
                        name: String::new(),
                        country: 0,
                        position: 0,
                        age: 0,
                        club: 0,
                        club_name: String::new(),
                        games: 0,
                        goals: 0,
                        assists: 0,
                        rating_sum: 0.0,
                    });
                    stats.len() - 1
                }
            };
            let e = &mut stats[i];
            e.name = name;
            e.country = country;
            e.position = position;
            e.age = age;
            e.club = club_id;
            e.club_name = club_name;
            e.games += 1;
            e.goals += g;
            e.assists += a;
            e.rating_sum += rating;
        }
    }

    /// `finalizarTemporada(boolean)`: the champion, the top scorer and the best junior with
    /// their development bonus, the history, and (`reward`) the podium's money and the fans.
    fn finish_youth_league(&mut self, reward: bool) {
        self.youth_league().finished = true;
        let rows = table(self.world);
        if rows.is_empty() {
            return;
        }
        let champion = rows[0].clone();
        let runner_up = rows.get(1).cloned().unwrap_or_else(|| champion.clone());
        let mine = my_club(self.world);
        let my_place = mine.and_then(|m| rows.iter().position(|r| r.club == self.world.club(m).id)).map(|i| i as i32 + 1).unwrap_or(-1);
        let s = &self.world.mods.youth_league;
        let mut scorers = s.stats.clone();
        scorers.sort_by(|a, b| b.goals.cmp(&a.goals).then(b.average().total_cmp(&a.average())));
        let top_scorer = scorers.first().filter(|e| e.goals > 0).cloned();
        let least = 3.max(rounds_played(s) / 3);
        let mut best: Vec<YouthLeagueStats> = s.stats.iter().filter(|e| e.games >= least).cloned().collect();
        if best.is_empty() {
            best = s.stats.clone();
        }
        let score = |e: &YouthLeagueStats| e.average() + e.goals as f64 * 0.04 + e.assists as f64 * 0.03;
        best.sort_by(|a, b| score(b).total_cmp(&score(a)));
        let best = best.first().cloned();
        // the development bonus of the standouts and of the champion's regulars
        let mut rewarded = HashSet::new();
        if let Some(e) = &best {
            self.highlight_bonus(&e.key, 2, 0.45);
            rewarded.insert(e.key.clone());
        }
        if let Some(e) = &top_scorer
            && !rewarded.contains(&e.key)
        {
            self.highlight_bonus(&e.key, 1, 0.35);
            rewarded.insert(e.key.clone());
        }
        for e in self.world.mods.youth_league.stats.clone() {
            if e.club == champion.club && e.games >= least && !rewarded.contains(&e.key) {
                self.highlight_bonus(&e.key, 1, 0.25);
            }
        }
        let s = &self.world.mods.youth_league;
        let year = self.world.game.start_year + s.season - 1;
        let country = s.country;
        let entry = YouthLeagueChampion {
            year,
            country,
            champion: champion.name.clone(),
            runner_up: runner_up.name.clone(),
            top_scorer: top_scorer.map(|e| (e.name, e.club_name, e.goals)),
            best_player: best.map(|e| (e.name.clone(), e.club_name.clone(), e.average())),
            player_position: my_place,
        };
        let history = &mut self.youth_league().history;
        history.sort_by(|a, b| b.year.cmp(&a.year));
        history.retain(|h| !(h.year == year && h.country == country));
        history.push(entry);
        if !reward {
            return;
        }
        let by_id = clubs_by_id(self.world);
        self.youth_prize(by_id.get(&champion.club).copied(), CHAMPION_PRIZE);
        if rows.len() > 1 {
            self.youth_prize(by_id.get(&runner_up.club).copied(), RUNNER_UP_PRIZE);
        }
        if rows.len() > 2 {
            self.youth_prize(by_id.get(&rows[2].club).copied(), THIRD_PRIZE);
        }
        if let Some(m) = mine {
            let id = self.world.club(m).id;
            if champion.club == id {
                self.fans_event(4, FanEvent::YouthLeagueTitle);
            } else if runner_up.club == id {
                self.fans_event(2, FanEvent::YouthLeagueRunnerUp);
            }
        }
        // Jornal.noticia: the league's story is not ported
    }

    /// `creditarPremio(ah, long)`: cash, category 3, for any club.
    fn youth_prize(&mut self, club: Option<ClubId>, amount: i64) {
        let Some(club) = club else { return };
        if amount <= 0 {
            return;
        }
        self.world.club_income(club, amount.min(i32::MAX as i64) as i32, 3);
    }

    /// `fatorDesenvolvimento(F)`: a junior of the league develops by how much and how well he
    /// plays; one who sits it out, slower.
    pub(crate) fn youth_league_development_factor(&self, p: PlayerId) -> f64 {
        let Some(club) = self.world.player(p).club else { return 1.0 };
        if !self.sys.config.on(Switch::YouthLeague) {
            return 1.0;
        }
        let s = &self.world.mods.youth_league;
        let played = rounds_played(s);
        if played < 2 || !s.clubs.iter().any(|c| c.0 == self.world.club(club).id) {
            return 1.0;
        }
        let key = player_key(self.world, p);
        let Some(e) = s.stats.iter().find(|e| e.key == key).filter(|e| e.games > 0) else { return 0.60 };
        let presence = (e.games as f64 / (played as f64 * 0.65)).min(1.0);
        let base = 0.65 + 0.40 * presence;
        let bonus = ((e.average() - 6.0) * 0.18).max(0.0) + 0.25f64.min((e.goals * 2 + e.assists) as f64 * 0.015);
        1.45f64.min(base + bonus)
    }

    /// `BaseDialog.mostrar`: `Base.tick()` and `CampeonatoBase.tick()` before the window.
    pub fn open_youth_academy(&mut self) {
        self.academy_tick();
        self.youth_league_tick();
    }

    /// `escalacaoAtual(ah)`: the juniors the club's XI would field now (their attributes are
    /// aligned on the way, as in the original); the stand-ins are dropped again.
    pub fn current_youth_xi(&mut self, club: ClubId) -> Vec<PlayerId> {
        let players = self.world.players.len();
        let mine = my_club(self.world) == Some(club);
        let xi = self.youth_xi(club, mine);
        self.world.players.truncate(players);
        xi.real
    }

    /// `alternarTitular(F)`: marks or unmarks a priority starter; a goalkeeper takes the other
    /// one's place. False when eleven are marked already.
    pub fn toggle_priority_starter(&mut self, p: PlayerId) -> bool {
        let key = player_key(self.world, p);
        if let Some(i) = self.youth_league().priority_starters.iter().position(|k| *k == key) {
            self.youth_league().priority_starters.remove(i);
            return true;
        }
        if let Some(mine) = my_club(self.world) {
            self.drop_old_priority_starters(mine);
        }
        if self.youth_league().priority_starters.len() >= 11 {
            return false;
        }
        if self.world.player(p).position == 0
            && let Some(mine) = my_club(self.world)
        {
            let keepers: Vec<String> = self.world.club(mine).youth_squad.iter().filter(|&&o| o != p && self.world.player(o).position == 0).map(|&o| player_key(self.world, o)).collect();
            self.youth_league().priority_starters.retain(|k| !keepers.contains(k));
        }
        self.youth_league().priority_starters.push(key);
        true
    }

    /// `definirTaticaBase(int, int, int)`: -1 for the automatic style.
    pub fn set_youth_tactic(&mut self, style: i32, marking: i32, side: i32) {
        let s = self.youth_league();
        s.style = style;
        s.marking = marking;
        s.side = side;
    }

    /// `Base.renomear(String, F)` and `CampeonatoBase.renomearJogador`: a junior's new name
    /// carries his academy record, priority and youth league numbers to the new key.
    pub fn rename_junior(&mut self, p: PlayerId, name: String) {
        let old = player_key(self.world, p);
        self.world.player_mut(p).name = name.clone();
        let new = player_key(self.world, p);
        if let Some(kid) = self.world.mods.academy.as_mut().and_then(|kids| kids.iter_mut().find(|k| k.key == old)) {
            kid.key = new.clone();
            self.sys.academy_generation += 1;
        }
        let s = self.youth_league();
        if let Some(i) = s.priority_starters.iter().position(|k| *k == old) {
            s.priority_starters.remove(i);
            s.priority_starters.push(new.clone());
        }
        if let Some(i) = s.stats.iter().position(|e| e.key == old) {
            let mut e = s.stats.remove(i);
            e.key = new;
            e.name = name;
            s.stats.push(e);
        }
    }

    /// `limparTitularesAntigos(ah)`: the priority starters still in the player's youth squad.
    fn drop_old_priority_starters(&mut self, mine: ClubId) {
        let keys: HashSet<String> = self.world.club(mine).youth_squad.iter().map(|&p| player_key(self.world, p)).collect();
        self.youth_league().priority_starters.retain(|k| keys.contains(k));
    }
}

/// `atribuirComPrioridade(List, int[], boolean)`: the Hungarian assignment of the candidates
/// to the slots, by value, real juniors (and the priority starters most) first.
fn assign(world: &World, candidates: &[PlayerId], slots: &[i32; 11], prioritised: &dyn Fn(PlayerId) -> bool, stand_in: &dyn Fn(PlayerId) -> bool, individual: bool) -> Vec<i32> {
    let n = candidates.len();
    let m = slots.len();
    let mut cost = vec![vec![0i32; n + 1]; m + 1];
    for i in 0..m {
        for j in 0..n {
            let p = candidates[j];
            let mut val = mod_value(individual, world.player(p), slots[i]);
            if !stand_in(p) {
                val += 40;
                if prioritised(p) {
                    val += 400;
                }
            }
            cost[i + 1][j + 1] = -val;
        }
    }
    let mut u = vec![0i32; m + 1];
    let mut v = vec![0i32; n + 1];
    let mut pair = vec![0usize; n + 1];
    let mut path = vec![0usize; n + 1];
    for i in 1..=m {
        pair[0] = i;
        let mut j0 = 0usize;
        let mut minv = vec![i32::MAX; n + 1];
        let mut used = vec![false; n + 1];
        loop {
            used[j0] = true;
            let i0 = pair[j0];
            let mut delta = i32::MAX;
            let mut j1 = 0usize;
            for j in 1..=n {
                if used[j] {
                    continue;
                }
                let current = cost[i0][j].wrapping_sub(u[i0]).wrapping_sub(v[j]);
                if current < minv[j] {
                    minv[j] = current;
                    path[j] = j0;
                }
                if minv[j] < delta {
                    delta = minv[j];
                    j1 = j;
                }
            }
            for j in 0..=n {
                if used[j] {
                    u[pair[j]] = u[pair[j]].wrapping_add(delta);
                    v[j] = v[j].wrapping_sub(delta);
                } else {
                    minv[j] = minv[j].wrapping_sub(delta);
                }
            }
            j0 = j1;
            if pair[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = path[j0];
            pair[j0] = pair[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }
    let mut answer = vec![-1i32; m];
    for j in 1..=n {
        if pair[j] != 0 {
            answer[pair[j] - 1] = j as i32 - 1;
        }
    }
    answer
}
