// Port of what best.I does around a match day: I.he() (play), I.hg() (records), I.hf() (gate
// income), I.hh() (players' numbers and ratings) and the player methods they call
// (F.b(at, ah), F.a(at, ah), F.a(int, at, ah), F.a(at, I, int, int, ah)).

use brasfoot_engine::events::{competition_stats, record, record_season};
use brasfoot_engine::messages;
use brasfoot_engine::simulate::{Play, prepare_squads};
use brasfoot_engine::value::out_of_position;
use brasfoot_model::ids::{ClubId, CompetitionId, MatchId, PlayerId};
use brasfoot_model::rng::java_round;
use brasfoot_model::saved::{CompetitionKind, HeadToHead, MatchRating, ScorerRecord};

use crate::Sim;

impl Sim<'_> {
    /// `ah.kh()`: AI clubs pick their line-ups for today's matches.
    pub fn pick_ai_lineups(&mut self) {
        for m in self.today_matches() {
            let (home, away) = self.sides(m);
            for (club, side) in [(home, 1), (away, 2)] {
                let c = self.world.club(club);
                if !c.human && !c.lineup_set {
                    brasfoot_engine::lineup::pick_ai_lineup(self.world, self.runtime, club, Some(m), side, -1, true);
                }
            }
        }
    }

    /// `I.he()`: today's matches.
    pub fn play_today(&mut self) {
        self.play_today_each(|_, _, _| {});
    }

    /// `I.he()`, handing each match's engine to `keep` (the round screen goes on with those of
    /// the human clubs' matches).
    pub fn play_today_each(&mut self, mut keep: impl FnMut(&mut Self, MatchId, brasfoot_engine::engine::MatchEngine)) {
        let mut counter = 0;
        let mut last_competition: Option<CompetitionId> = None;
        // the grounds of the days before are gone with their matches
        self.runtime.venues.clear();
        for m in self.today_matches() {
            if let Some(c) = self.world.game_match(m).competition {
                let kind = self.world.competition(c).kind;
                if kind == 15 {
                    // I.a(at, int, true, f.L.Bt()): the friendly tournament's city, no stadium
                    self.away_from_stadium(m);
                }
                let nations_league_final = kind == 14
                    && self.world.game.nations_league_m.is_some_and(|nl| {
                        let final_stage = self.world.competition(nl).playoff_round;
                        final_stage.is_some() && self.world.game_match(m).stage == final_stage
                    });
                if kind == 7 || kind == 13 || nations_league_final {
                    if last_competition != Some(c) {
                        counter = 0;
                    }
                    last_competition = Some(c);
                    // I.a(at, int, false, ""): the host's city (at.cx) and a ground of its size
                    let capacity = self.venue_capacity(c, counter);
                    self.runtime.venues.insert(m, venue(capacity));
                    counter += 1;
                }
                if kind == 9 {
                    let stage = self.world.game_match(m).stage;
                    if stage.is_some_and(|s| self.world.stage(s).format_code == 7701) {
                        // I.a(at, int, true, aY().eY(2).jf()): the intercontinental playoff's
                        // city, no stadium
                        self.away_from_stadium(m);
                    }
                }
            }
            let engine = Play { world: self.world, runtime: self.runtime, hooks: self.hooks, game_match: m }.play_ai_match();
            keep(self, m, engine);
        }
    }

    /// `I.a(at, int, true, String)`: a match on neutral ground with no stadium of its own.
    fn away_from_stadium(&mut self, m: MatchId) {
        self.world.game_match_mut(m).stadium = None;
        self.runtime.venues.remove(&m);
    }

    /// `at.cy(int)`: the capacity of a tournament's grounds, by the match's place in the day
    /// (each class its own list, bigger in the hosts that fill them).
    fn venue_capacity(&mut self, c: CompetitionId, counter: i32) -> i32 {
        use brasfoot_model::saved::CompetitionKind::*;
        let comp = self.world.competition(c);
        let host = comp.host.map(|h| self.world.country(h).country);
        const BIG_HOSTS: [i32; 8] = [65000, 55000, 52698, 48134, 45600, 34990, 37215, 28065];
        const OTHER_HOSTS: [i32; 8] = [45000, 42000, 32698, 38134, 35600, 34990, 37215, 28065];
        let table: [i32; 8] = match comp.class {
            WorldCup => {
                if host.is_some_and(|h| h != 11 && h != 29) {
                    OTHER_HOSTS
                } else {
                    [80000, 65000, 52698, 48134, 45600, 34990, 37215, 28065]
                }
            }
            CopaAmerica => {
                if host.is_some_and(|h| h != 11 && h != 29) {
                    OTHER_HOSTS
                } else {
                    BIG_HOSTS
                }
            }
            GoldCup => {
                if host.is_some_and(|h| h != 131 && h != 68) {
                    OTHER_HOSTS
                } else {
                    [75000, 65000, 52698, 48134, 45600, 34990, 37215, 28065]
                }
            }
            OceaniaNationsCup => {
                if host.is_some_and(|h| h != 143) {
                    [15000, 12000, 12698, 9500, 15600, 14990, 17215, 18065]
                } else {
                    [42000, 35000, 32698, 30134, 25600, 24990, 22215, 20065]
                }
            }
            AfricaCupOfNations | AsianCup => BIG_HOSTS,
            EuropeanChampionship => [80000, 75000, 72698, 68134, 55600, 44990, 47215, 38065],
            NationsLeagueM | NationsLeagueN | Finalissima => {
                // a random ground of the three
                const GROUNDS: [i32; 3] = [65000, 55000, 52698];
                let i = self.runtime.rng.fresh_int(GROUNDS.len() as i32) as usize;
                return GROUNDS[i] + self.runtime.rng.fresh_int(5000);
            }
            _ => return 40000,
        };
        if counter <= 7 {
            table[counter as usize]
        } else if ((counter - 8) as usize) < table.len() {
            table[(counter - 8) as usize]
        } else {
            45000
        }
    }

    pub fn today_matches(&self) -> Vec<MatchId> {
        self.world.game.days[self.world.game.day as usize].matches.clone()
    }

    pub fn sides(&self, m: MatchId) -> (ClubId, ClubId) {
        let g = self.world.game_match(m);
        (g.home.expect("home club"), g.away.expect("away club"))
    }

    /// The part of `f.as()` for one match: `a(true)`, `hg()`, `hi()`, `hf()`, `hh()`.
    pub fn after_match(&mut self, m: MatchId) {
        self.world.game_match_mut(m).played = true;
        self.update_records(m);
        prepare_squads(self.world, self.runtime, m);
        self.gate_income(m);
        self.count_players(m);
        let (home, away) = self.sides(m);
        self.world.club_mut(home).lineup_set = false;
        self.world.club_mut(away).lineup_set = false;
    }

    fn competition_kind(&self, m: MatchId) -> Option<i32> {
        self.world.game_match(m).competition.map(|c| self.world.competition(c).kind)
    }

    /// `I.hg()`: head-to-head, the clubs' and the coaches' records.
    fn update_records(&mut self, m: MatchId) {
        let kind = self.competition_kind(m).unwrap_or(-1);
        let (home, away) = self.sides(m);
        let (hg, ag) = (self.world.game_match(m).home_goals, self.world.game_match(m).away_goals);
        let (home_id, away_id) = (self.world.club(home).id, self.world.club(away).id);
        // the last matching record wins, like the original's loop
        let existing = self.world.game.head_to_head.iter().rposition(|t| {
            (t.club1 == home_id && t.club2 == away_id) || (t.club2 == home_id && t.club1 == away_id)
        });
        let season = self.world.season();
        match existing {
            Some(i) => update_head_to_head(&mut self.world.game.head_to_head[i], home_id, hg, ag, kind, season),
            None => self.world.game.head_to_head.push(new_head_to_head(home_id, away_id, hg, ag, kind, season)),
        }
        self.club_match_record(home, m);
        self.club_match_record(away, m);
        let counted = matches!(kind, 1 | 3 | 2 | 4 | 5 | 6 | 8 | 10);
        for club in [home, away] {
            if let Some(coach) = self.world.club(club).coach {
                self.coach_match_record(coach, m);
                if counted {
                    self.coach_mood(coach, m, false, -1);
                }
            }
        }
    }

    /// `I.hf()`: the human home club takes the gate.
    fn gate_income(&mut self, m: MatchId) {
        let kind = self.competition_kind(m).unwrap_or(-1);
        let (home, _) = self.sides(m);
        if kind != 7 && kind != 5 && self.world.club(home).human {
            let income = self.world.game_match(m).income;
            self.world.club_income(home, income, 5);
        }
    }

    /// `I.hh()`: appearances, ratings, goals and cards.
    fn count_players(&mut self, m: MatchId) {
        let competition = self.world.game_match(m).competition;
        let (home, away) = self.sides(m);
        let starters = [self.world.game_match(m).home_starters.clone(), self.world.game_match(m).away_starters.clone()];
        for (side, list) in starters.iter().enumerate() {
            let club = if side == 0 { home } else { away };
            for (i, &p) in list.iter().enumerate() {
                self.appearance(p, competition, club);
                self.rate(p, competition, m, side, 1 - side, club);
                let slot = self.world.player(p).slot;
                let slots = &mut self.world.game_match_mut(m).lineup_slots[side];
                if i < slots.len() {
                    slots[i] = slot;
                }
            }
        }
        let came_on = [self.world.game_match(m).home_list_fl.clone(), self.world.game_match(m).away_list_fm.clone()];
        for (side, list) in came_on.iter().enumerate() {
            let club = if side == 0 { home } else { away };
            for &p in list {
                self.appearance(p, competition, club);
                self.rate(p, competition, m, side, 1 - side, club);
            }
        }
        let events = self.world.game_match(m).events.clone();
        for e in events {
            // goals and cards always carry their club
            let (Some(p), Some(club)) = (e.player, e.club) else { continue };
            if e.kind == 1 && e.detail != 2 {
                self.goal(p, competition, club);
            } else if matches!(e.kind, 2..=4) {
                self.card(p, e.kind, competition, club);
            }
        }
    }

    fn is_friendly(&self, competition: Option<CompetitionId>) -> bool {
        competition.is_some() && competition == self.world.game.friendly
    }

    /// `F.b(at, ah)`: an appearance.
    fn appearance(&mut self, p: PlayerId, competition: Option<CompetitionId>, club: ClubId) {
        if self.world.player(p).youth {
            return;
        }
        if competition.is_some() {
            competition_stats(self.world, p, competition).games += 1;
        }
        record_season(self.world, p, record::GAME, club, 0.0, None);
        self.world.player_mut(p).career_games += 1;
    }

    /// `F.a(at, ah)`: a goal.
    fn goal(&mut self, p: PlayerId, competition: Option<CompetitionId>, club: ClubId) {
        if self.world.player(p).youth {
            return;
        }
        if let Some(c) = competition {
            competition_stats(self.world, p, competition).goals += 1;
            self.scorer_record(c, p).goals += 1;
        }
        record_season(self.world, p, record::GOAL, club, 0.0, None);
        self.world.player_mut(p).career_goals += 1;
    }

    /// `at.p(F)`: the competition's scorer entry of a player, created on first use.
    pub fn scorer_record(&mut self, c: CompetitionId, p: PlayerId) -> &mut ScorerRecord {
        let scorers = &mut self.world.competition_mut(c).scorers;
        let i = match scorers.iter().position(|s| s.player == Some(p)) {
            Some(i) => i,
            None => {
                scorers.push(ScorerRecord { player: Some(p), ..Default::default() });
                scorers.len() - 1
            }
        };
        &mut scorers[i]
    }

    /// `F.a(int, at, ah)`: a card. A red one suspends for a random number of matches.
    fn card(&mut self, p: PlayerId, kind: i32, competition: Option<CompetitionId>, club: ClubId) {
        if self.world.player(p).youth {
            return;
        }
        if competition.is_some() && !self.is_friendly(competition) {
            match kind {
                2 => competition_stats(self.world, p, competition).yellow_cards += 1,
                3 => {
                    let h = competition_stats(self.world, p, competition);
                    h.yellow_cards += 1;
                    h.red_cards += 1;
                }
                4 => {
                    let draw = self.runtime.rng.fresh_int(1000);
                    let matches = match draw {
                        ..700 => 1,
                        ..900 => 2,
                        ..970 => 3,
                        ..=990 => 5,
                        _ => 10,
                    };
                    let human_coach = self.world.player(p).club.and_then(|c| self.world.club(c).coach).filter(|&c| self.world.coach(c).human);
                    if let Some(coach) = human_coach {
                        let name = self.world.player(p).name.clone();
                        if matches == 2 {
                            let id = self.runtime.rng.fresh_int(6) + 10;
                            messages::send(self.world, coach, 1, id, name, String::new());
                        } else if matches > 1 {
                            let id = match matches {
                                3 => self.runtime.rng.fresh_int(5) + 5,
                                5 => self.runtime.rng.fresh_int(3) + 2,
                                _ => self.runtime.rng.fresh_int(2),
                            };
                            let salary = self.world.player(p).salary;
                            messages::send_about_player(self.world, coach, 1, id, name.clone(), String::new(), name, salary, true, 1);
                        }
                    }
                    let h = competition_stats(self.world, p, competition);
                    h.red_cards += matches;
                }
                _ => {}
            }
        }
        record_season(self.world, p, kind, club, 0.0, None);
    }

    /// Port of `F.a(at, I, int, int, ah)`: the player's rating of the match.
    fn rate(&mut self, p: PlayerId, competition: Option<CompetitionId>, m: MatchId, side: usize, other: usize, club: ClubId) {
        const DEFAULT_SLOT: [i32; 5] = [1, 2, 7, 15, 23];
        let g = self.world.game_match(m).clone();
        let mut minutes = 90;
        for e in &g.events {
            if e.player == Some(p) {
                minutes = if e.half == 1 { e.minute } else { 48 + e.minute };
            }
            if e.other_player == Some(p) {
                minutes = if e.half == 1 { 48 + (50 - e.minute) } else { 50 - e.minute };
            }
        }
        let (result, scored, conceded) = if side == 0 {
            (if g.home_goals > g.away_goals { 1 } else if g.home_goals < g.away_goals { 2 } else { 0 }, g.home_goals, g.away_goals)
        } else {
            (if g.away_goals > g.home_goals { 1 } else if g.away_goals < g.home_goals { 2 } else { 0 }, g.away_goals, g.home_goals)
        };
        let _ = scored;
        let player = self.world.player(p).clone();
        let mut slot = player.slot;
        if slot < 1 {
            slot = DEFAULT_SLOT[player.position as usize];
            self.world.player_mut(p).slot = slot;
        }
        let strength = player.strength;
        let mut rating = match (result, strength) {
            (0, ..=30) => 5.5,
            (0, ..=60) => 5.8,
            (0, ..=90) => 6.2,
            (0, _) => 6.8,
            (1, ..=30) => 6.0,
            (1, ..=60) => 6.0,
            (1, ..=90) => 6.7,
            (1, _) => 7.2,
            (_, ..=30) => 5.0,
            (_, ..=60) => 5.2,
            (_, ..=90) => 5.5,
            (_, _) => 6.0,
        };
        let mut moved = player.clone();
        moved.slot = slot;
        // The five `-= 1.5` of this method show as `var21--` in the decompiled source (a
        // Vineflower bug, see tools/decompile-check); the bytecode subtracts 1.5.
        if out_of_position(&moved) {
            rating -= 1.5;
            if slot == 1 {
                rating -= 1.5;
            }
        }
        let mut random = self.runtime.rng.new_random();
        if (10..=17).contains(&slot) {
            let possession = &g.possession;
            if possession[side] > possession[other] {
                rating += if random.next_int_bounded(3) == 1 { 0.3 } else { 0.8 };
                if player.style == 0 {
                    rating += 0.3;
                }
                if player.trait1 == 11 || player.trait1 == 4 {
                    rating += 0.5;
                }
            } else if possession[side] < possession[other] {
                rating -= if random.next_int_bounded(3) == 1 { 0.3 } else { 0.8 };
                if player.style == 0 {
                    rating -= 0.5;
                }
            }
        }
        let s = self.runtime.stats(p).clone();
        if s.goals > 0 {
            rating += s.goals as f64 * 0.9;
        }
        if s.own_goals > 0 {
            rating -= s.own_goals as f64 * 1.5;
        }
        // the original multiplies the own goals again here, not Pi
        if s.count_pi > 0 {
            rating -= s.own_goals as f64 * 1.2;
        }
        if s.yellow_cards > 0 {
            rating -= s.yellow_cards as f64 * 0.2;
        }
        if s.red_cards > 0 {
            rating -= s.red_cards as f64 * 0.8;
        }
        if s.assists > 0 {
            rating += s.assists as f64 * 0.4;
        }
        if (1..=13).contains(&slot) {
            let tackles = &g.tackles;
            if tackles[side] > tackles[other] {
                rating += if random.next_int_bounded(3) == 1 { 0.9 } else { 0.6 };
                if (2..=9).contains(&slot) && random.next_int_bounded(3) == 1 {
                    rating += 0.6;
                }
                if (11..=13).contains(&slot) && random.next_int_bounded(3) == 1 {
                    rating += 0.6;
                }
            } else if tackles[side] < tackles[other] {
                rating -= 0.5;
                if (3..=8).contains(&slot) && random.next_int_bounded(4) == 1 {
                    rating -= 0.6;
                }
                if (11..=13).contains(&slot) && random.next_int_bounded(4) == 1 {
                    rating -= 0.6;
                }
            }
        }
        if s.shots_saved > 0 {
            rating += s.shots_saved as f64 * 0.3;
        }
        if slot == 1 {
            rating -= 0.8;
            let shots_against = g.shots[other];
            let on_target_against = g.shots_on_target[other];
            rating += on_target_against as f64 * 0.2;
            if s.count_pl > 0 {
                rating += s.count_pl as f64 * 1.2;
            }
            // the later branches can never run once the first one fails: kept as written
            if shots_against > 10 {
                rating += 0.2;
            } else if shots_against > 15 {
                rating += 0.2;
            } else if shots_against > 20 {
                rating += 0.3;
            }
            if conceded >= 5 {
                rating -= 2.0;
            } else if conceded >= 4 {
                rating -= 1.5;
            } else if conceded >= 2 {
                rating -= 1.0;
            } else if conceded >= 1 {
                rating -= 0.5;
            } else if conceded == 0 {
                rating += 1.0;
            }
            if on_target_against == 0 {
                rating -= 1.5;
            }
        }
        if (1..=13).contains(&slot) {
            if conceded == 0 {
                rating += 0.5;
                if (2..=9).contains(&slot) {
                    rating += 0.5;
                }
                if (11..=13).contains(&slot) && random.next_int_bounded(3) == 1 {
                    rating += 0.5;
                }
            } else if conceded > 1 {
                rating -= 0.1 * conceded as f64;
            }
            if (2..=13).contains(&slot) && random.next_int_bounded(3) == 1 {
                rating -= 0.4;
            }
        }
        if player.star {
            rating += 0.4;
        }
        if player.world_class {
            rating += 0.6;
        }
        if rating > 10.0 {
            rating = 10.0;
        }
        if rating < 0.0 {
            rating = 1.0;
        }
        if minutes < 15 {
            rating -= 2.5;
        } else if minutes < 45 {
            rating -= 1.5;
        }
        if rating < 2.0 {
            rating = 2.0;
        }
        if minutes < 20 && rating <= 2.0 {
            rating = 0.0;
        }
        self.world.player_mut(p).last_rating = rating;
        let day = self.world.game.day;
        let index = self.today_matches().iter().position(|&x| x == m).map(|i| i as i32).unwrap_or(-1);
        self.world.player_mut(p).ratings.push(MatchRating { rating, day, match_index: index });
        if self.world.player(p).club.is_some() {
            record_season(self.world, p, record::RATING, club, rating, None);
        }
        if rating > 0.0 && let Some(c) = competition && self.world.competition(c).class == CompetitionKind::NationalLeague {
            self.league_rating(c, p);
            self.round_rating(c, p);
        }
        if rating > 0.0 && let Some(c) = competition && self.world.competition(c).kind > 0 && !self.world.player(p).youth {
            competition_stats(self.world, p, Some(c)).add_rating(rating);
        }
    }
}

fn new_head_to_head(club1: i32, club2: i32, goals1: i32, goals2: i32, kind: i32, season: i32) -> HeadToHead {
    let mut t = HeadToHead { club1, club2, goals1, goals2, games: 1, ..Default::default() };
    if goals1 != goals2 {
        if goals1 > goals2 {
            t.wins1 += 1;
            t.best_win1_kind = kind;
            t.best_win1_season = season;
            t.best_win1_for = goals1;
            t.best_win1_against = goals2;
        } else {
            t.wins2 += 1;
            t.best_win2_kind = kind;
            t.best_win2_season = season;
            t.best_win2_for = goals1;
            t.best_win2_against = goals2;
        }
    }
    t
}

/// `T.a(ah, ah, int, int, int)` and `T.a(int, int, int, int)`.
fn update_head_to_head(t: &mut HeadToHead, home_id: i32, home_goals: i32, away_goals: i32, kind: i32, season: i32) {
    let best = |t: &mut HeadToHead, which: i32, scored: i32, conceded: i32| {
        if which == 1 {
            let gap = t.best_win1_for - t.best_win1_against;
            let new_gap = scored - conceded;
            if new_gap > gap || (new_gap == gap && scored >= t.best_win1_for) {
                t.best_win1_season = season;
                t.best_win1_kind = kind;
                t.best_win1_for = scored;
                t.best_win1_against = conceded;
            }
        } else {
            // the original compares with club 1's goals conceded here
            let gap = t.best_win2_for - t.best_win1_against;
            let new_gap = scored - conceded;
            if new_gap > gap || (new_gap == gap && scored >= t.best_win2_for) {
                t.best_win2_season = season;
                t.best_win2_kind = kind;
                t.best_win2_for = scored;
                t.best_win2_against = conceded;
            }
        }
    };
    t.games += 1;
    if home_id == t.club1 {
        t.goals1 += home_goals;
        t.goals2 += away_goals;
        if home_goals > away_goals {
            t.wins1 += 1;
            best(t, 1, home_goals, away_goals);
        } else if away_goals > home_goals {
            t.wins2 += 1;
            best(t, 2, away_goals, home_goals);
        }
    } else {
        t.goals1 += away_goals;
        t.goals2 += home_goals;
        if home_goals > away_goals {
            t.wins2 += 1;
            best(t, 2, home_goals, away_goals);
        } else if away_goals > home_goals {
            t.wins1 += 1;
            best(t, 1, away_goals, home_goals);
        }
    }
}

/// `new v(true, String, int)`: a ground of that capacity (`v.N(int)`, its sectors in the usual
/// shares), with the automatic prices. Its name is the city (`I.ik()`), for the screens.
fn venue(capacity: i32) -> brasfoot_model::saved::Stadium {
    let capacity = if !(1000..=120_000).contains(&capacity) { 10_000 } else { capacity };
    let mut ground = brasfoot_model::saved::Stadium::default();
    let general = java_round(capacity as f64 * 0.15) as i32;
    let seats = java_round(capacity as f64 * 0.09) as i32;
    let boxes = java_round(capacity as f64 * 0.009) as i32;
    ground.capacity = vec![general, capacity - (general + seats + boxes), seats, boxes];
    ground
}
