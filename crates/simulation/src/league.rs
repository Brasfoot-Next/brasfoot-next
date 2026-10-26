// Port of the parts of f.B (national league) and f.s (league stage) the match day uses: the
// season's ratings table (f.B.D, E), the round's ratings (f.B.F, transient), positions (f.s.P)
// and relegation places (f.s.getnRebaixados).

use brasfoot_model::ids::{ClubId, CompetitionId, PlayerId, StageId};
use brasfoot_model::runtime::RoundRating;
use brasfoot_model::saved::{CompetitionKind, RatingRecord, SeasonSelection, Stage};
use brasfoot_model::world::World;

use crate::Sim;

/// `f.s.P(ah)`: 1-based table position, -1 once a knockout follows the table or if absent.
pub fn position_in(world: &World, stage: StageId, club: ClubId) -> i32 {
    let s = world.stage(stage);
    if s.knockout.is_some() {
        return -1;
    }
    s.clubs.iter().position(|&c| c == club).map(|i| i as i32 + 1).unwrap_or(-1)
}

/// `f.s.getnRebaixados()`: relegation places of the stage's division, from its state league or
/// country.
pub fn relegated_places(world: &World, stage: StageId) -> i32 {
    let s = world.stage(stage);
    let division = (s.division - 1).max(0) as usize;
    let divisions = if let Some(state) = s.state_league {
        &world.state_league(state).divisions
    } else if let Some(country) = s.country_ref {
        &world.country(country).divisions
    } else {
        return 0;
    };
    divisions.get(division).map(|&c| world.competition(c).relegated).unwrap_or(0)
}

/// `f.s.zk()`: the round the mid-season market opens on. It returns half the current round once
/// past the middle, so `round == zk()` never holds after a round is played: the original's
/// mid-season market of `f.s.pO()` never runs.
pub fn mid_season_round(s: &Stage) -> i32 {
    let n = s.clubs.len() as i32;
    let rounds = if s.group_count == 0 { (n - 1) * 2 } else { (n / s.group_count) * 2 - 1 };
    if s.round > rounds / 2 { s.round / 2 } else { 0 }
}

/// The kind of slot `f.B.E` files a rating under: -1 goalkeepers' unused, by `fT()`.
const RATING_SLOT_KIND: [i32; 26] = [-1, 0, 5, 2, 2, 2, 2, 2, 2, 1, 3, 6, 6, 6, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4];

impl Sim<'_> {
    /// The end of `f.s.pO()`: the last round of a league stage was played.
    pub fn end_league_stage(&mut self, stage: StageId) {
        let (kind, knockout_teams) = (self.world.stage(stage).kind, self.world.stage(stage).knockout_teams);
        if kind == 1 && knockout_teams > 0 {
            // ag.b(f.s): each AI club of the table buys and sells by its place
            let mut i = 0;
            while i < self.world.stage(stage).clubs.len() {
                let club = self.world.stage(stage).clubs[i];
                if !self.world.club(club).human {
                    self.club_market(club, i as i32, true);
                }
                i += 1;
            }
        }
        let s = self.world.stage(stage);
        if s.knockout_teams > 0 {
            // ah.bW(int): each club's place in the table
            for (i, club) in s.clubs.clone().into_iter().enumerate() {
                self.world.club_mut(club).value_nd = i as i32;
            }
        }
        let s = self.world.stage(stage);
        let competition = s.competition.expect("a stage belongs to a competition");
        match s.format_code {
            // f.N.yt(): the Concacaf Nations League's finals
            14001 | 14401 => self.nations_league_finals(competition),
            // f.M.BB(): the relegation playoff of league C
            14003 => self.nations_league_relegation_playoff(competition),
            // f.q.yt(): the World Cup's knockout
            7700 => {
                let cup = self.world.game.world_cup.expect("the World Cup");
                self.world_cup_knockout(cup);
            }
            // the other leagues of the Nations Leagues end with nothing more
            14002 | 14004 | 14402 | 14403 => {}
            // f.v.yt(), f.r.yt(), f.H.yt(): the knockouts of the continental groups
            4005 => self.champions_league_knockout(competition),
            4104 => self.libertadores_knockout(competition),
            6105 => self.sudamericana_knockout(competition),
            // f.x.Be(), f.J.Be(): the playoff rounds of the second tiers
            6005 | 12005 => self.second_tier_playoffs(competition),
            // f.l.Bk(): the Euro qualifiers' playoff
            9001 => self.euro_qualifiers_playoff(competition),
            // the World Cup's qualifiers
            7701 => self.intercontinental_playoff_end(competition),
            9100 => self.south_american_qualifiers_end(competition),
            9201 => self.african_qualifiers_playoff(competition),
            9301 => self.asian_qualifiers_final_stage(competition),
            9302 => self.asian_qualifiers_end(competition),
            9400 => self.concacaf_qualifiers_knockout(competition),
            9402 => self.concacaf_qualifiers_end(competition),
            9500 => self.oceania_qualifiers_groups(competition),
            9501 => self.oceania_qualifiers_knockout(competition),
            90 if s.knockout_teams == 0 => crate::unported!("f.s.yN(): the end of the league stage of format 90"),
            // f.s.yO(): the best thirds too
            _ if s.knockout_teams > 0 && s.best_third_places => self.knockout_with_best_thirds(stage),
            // f.s.yP()
            _ if s.knockout_teams > 0 => self.knockout_after_stage(stage),
            _ => {
                let (champion, runner_up) = (s.clubs.first().copied(), s.clubs.get(1).copied());
                self.record_title(competition, Some(stage), champion, runner_up);
            }
        }
    }

    /// `f.s.za()`: every club's final place in the table.
    pub fn league_places(&mut self, stage: StageId) {
        let s = self.world.stage(stage);
        if s.state_league.is_some() {
            crate::unported!("f.s.za(): the state league's ranking (aD)");
        }
        let competition = s.competition.expect("a stage belongs to a competition");
        let Some(knockout) = s.knockout else {
            for (i, club) in s.clubs.clone().into_iter().enumerate() {
                self.place(club, competition, i as i32 + 1, -1);
            }
            return;
        };
        // the knockout's clubs from its last round back, the winner of each tie first; then the
        // rest of the table
        let mut placed: Vec<ClubId> = Vec::new();
        let mut position = 1;
        let rounds = self.world.stage(knockout).rounds.clone();
        for (depth, r) in rounds.iter().rev().enumerate() {
            for &m in &r.first_leg {
                let g = self.world.game_match(m);
                let (home, away) = (g.home.expect("home club"), g.away.expect("away club"));
                let pair = if r.winners.contains(&home) {
                    [home, away]
                } else if r.winners.contains(&away) {
                    [away, home]
                } else {
                    crate::unported!("f.s.za() with a tie neither club won");
                };
                for club in pair {
                    if !placed.contains(&club) {
                        placed.push(club);
                        self.place(club, competition, position, depth as i32);
                        position += 1;
                    }
                }
            }
        }
        let kind = self.world.competition(competition).kind;
        let rest_depth = if matches!(kind, 4 | 6 | 12) { 1001 } else { -1 };
        for club in self.world.stage(stage).clubs.clone() {
            if !placed.contains(&club) {
                self.place(club, competition, position, rest_depth);
                position += 1;
            }
        }
    }

    /// `f.B.BQ()`: the promotion knockout of a lower division, or the season of the league is
    /// over (`GU`) and the relegation playoffs start (`BP`).
    pub fn league_season_over(&mut self, league: CompetitionId) {
        let c = self.world.competition(league);
        let above_relegated = self.relegated_from_above(league);
        if c.division > 1 && c.promotion_knockout.is_none() && above_relegated > 0 && c.promotion_knockout_places > 0 {
            // the four after the direct places play for the rest (one, or two in one round)
            let places = c.promotion_knockout_places;
            let from = (above_relegated - places) as usize;
            let to = from + 4;
            let table = self.world.stage(c.league.expect("a league has its table")).clubs.clone();
            if table.len() >= to {
                let clubs = table[from..to].to_vec();
                let rounds = if places == 2 { 1 } else { 0 };
                let legs = c.two_legged_promotion_knockout.clone();
                let stage = self.cup_knockout(league, &clubs, rounds, legs, 1099);
                self.world.competition_mut(league).promotion_knockout = Some(stage);
                return;
            }
        }
        self.promotion_knockout_over(league);
    }

    /// `f.B.BO()` (the end of 1099, or no promotion knockout): the season of the league is
    /// over (`GU`) and the relegation playoffs start (`BP`).
    pub(crate) fn promotion_knockout_over(&mut self, league: CompetitionId) {
        self.world.competition_mut(league).season_finished = true;
        self.relegation_playoffs(league);
    }

    /// `f.B.r(ArrayList, ArrayList)` (the end of 1098): the playoff's winners stay (1090), its
    /// losers go (1091), in their records of it.
    pub(crate) fn relegation_playoff_over(&mut self, league: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for (list, mark) in [(winners, 1090), (losers, 1091)] {
            for &c in list {
                self.competition_record(c, league, 1098).round_reached = mark;
            }
        }
    }

    /// `f.B.BN()` (`Z.K(int)`): the relegation places of the division above.
    fn relegated_from_above(&self, league: CompetitionId) -> i32 {
        let c = self.world.competition(league);
        let Some(country) = c.country_ref else { return 0 };
        let divisions = &self.world.country(country).divisions;
        let above = c.division - 2;
        if c.division > 1 && (above as usize) < divisions.len() { self.world.competition(divisions[above as usize]).relegated } else { 0 }
    }

    /// `f.B.BP()`: the playoffs against the division below, and of the division above.
    fn relegation_playoffs(&mut self, league: CompetitionId) {
        let c = self.world.competition(league);
        let Some(country) = c.country_ref else { return };
        let divisions = self.world.country(country).divisions.clone();
        let division = c.division as usize;
        let below = divisions.get(division).copied();
        if let Some(below) = below
            && self.world.competition(below).season_finished
        {
            self.relegation_playoff(league, Some(below));
        } else if division >= divisions.len() {
            self.relegation_playoff(league, None);
        }
        if division > 1
            && let Some(&above) = divisions.get(division - 2)
            && self.world.competition(above).season_finished
        {
            self.relegation_playoff(above, Some(league));
        }
    }

    /// `f.B.a(f.B)`: the playoff between the clubs about to go down and the best of the
    /// division below, when not every relegation is direct (`BM`).
    fn relegation_playoff(&mut self, league: CompetitionId, below: Option<CompetitionId>) {
        let c = self.world.competition(league);
        let some_by_playoff = c.relegated > 0 && c.relegated != c.relegated_directly;
        if !some_by_playoff || c.relegation_playoff.is_some() {
            return;
        }
        if c.division == 4 && c.country_ref.is_some_and(|z| self.world.country(z).country == 29) && self.world.game.flag_bc {
            crate::unported!("f.B.BM(): the Brazilian fourth division with the cup's new format");
        }
        let n = c.relegated;
        let playoffs = (c.relegated - c.relegated_directly) as usize;
        let table = c.league.expect("a league has its table");
        // the bottom n, the highest of them first
        let mut going: Vec<ClubId> = self.relegated_from(table, n);
        going.reverse();
        let mut coming: Vec<ClubId> = match below {
            Some(b) => {
                let t = self.world.competition(b).league.expect("a league has its table");
                let mut l = self.promoted_from(t, n, true);
                l.reverse();
                l
            }
            None => c.country_ref.map(|z| self.world.country(z).clubs.clone()).unwrap_or_default(),
        };
        let mut clubs = Vec::new();
        for i in 0..playoffs {
            if i < going.len() && i < coming.len() {
                clubs.push(going[i]);
                clubs.push(coming[i]);
            }
        }
        coming.clear();
        if !clubs.is_empty() {
            let legs = self.world.competition(league).two_legged_relegation_playoff.clone();
            let stage = self.cup_knockout(league, &clubs, 1, legs, 1098);
            self.world.competition_mut(league).relegation_playoff = Some(stage);
        }
    }

    /// `f.B.D(F)`: the player's entry in the league's ratings, created on first use.
    fn rating_record(&mut self, league: CompetitionId, p: PlayerId) -> &mut RatingRecord {
        let ratings = &mut self.world.competition_mut(league).ratings;
        let i = match ratings.iter().position(|e| e.player == Some(p)) {
            Some(i) => i,
            None => {
                ratings.push(RatingRecord { competition: Some(league), player: Some(p), ..Default::default() });
                ratings.len() - 1
            }
        };
        &mut ratings[i]
    }

    /// `f.B.E(F)`: a rating in the league's season table, if the player played in his place.
    pub fn league_rating(&mut self, league: CompetitionId, p: PlayerId) {
        debug_assert_eq!(self.world.competition(league).class, CompetitionKind::NationalLeague);
        let player = self.world.player(p);
        let (position, side, slot, style, rating) = (player.position, player.side, player.slot, player.style, player.last_rating);
        let mut in_place = match position {
            0 => slot == 1,
            1 if side == 0 => slot == 9,
            1 if side == 1 => slot == 2,
            2 => (3..=8).contains(&slot),
            3 => (10..=17).contains(&slot),
            4 => slot >= 18,
            _ => true,
        };
        if position == 1 && side == 0 && slot == 17 {
            in_place = true;
        }
        if position == 1 && side == 1 && slot == 10 {
            in_place = true;
        }
        if !in_place {
            return;
        }
        let kind = RATING_SLOT_KIND[slot as usize];
        let record = self.rating_record(league, p);
        // best.e.a(double)
        record.rating_sum += rating;
        record.rated_games += 1.0;
        record.average = record.rating_sum / record.rated_games;
        record.value_ac = if kind == 3 && style == 0 {
            6
        } else if kind == 3 && style == 1 {
            3
        } else if (kind == 5 || kind == 1) && position == 3 {
            3
        } else {
            kind
        };
    }

    /// `f.B.F(F)`: the round's ratings (`ZZ`, transient), for the team of the round.
    pub fn round_rating(&mut self, league: CompetitionId, p: PlayerId) {
        let tiebreak = self.runtime.rng.fresh_int(100);
        let player = self.world.player(p);
        let entry = RoundRating { player: p, rating: player.last_rating, slot: player.slot, tiebreak };
        self.runtime.round_ratings.entry(league).or_default().push(entry);
    }

    /// `f.B.zH()`: the team of the round, one player per slot range, best rating first (`f.C`).
    pub fn team_of_the_round(&mut self, league: CompetitionId) {
        self.runtime.checkpoint("f.B.zH");
        let mut ratings = self.runtime.round_ratings.remove(&league).unwrap_or_default();
        ratings.sort_by(|a, b| b.rating.total_cmp(&a.rating).then(b.tiebreak.cmp(&a.tiebreak)));
        const FROM: [i32; 11] = [1, 9, 3, 3, 2, 11, 11, 14, 14, 18, 18];
        const TO: [i32; 11] = [1, 9, 8, 8, 2, 13, 13, 16, 16, 25, 25];
        let rounds = self.world.competition(league).selections_aaa.len() as i32;
        let mut team = SeasonSelection { value_oj: rounds - 1, ..Default::default() };
        for i in 0..11 {
            if let Some(r) = ratings.iter().find(|r| r.slot >= FROM[i] && r.slot <= TO[i] && !team.players.contains(&r.player)) {
                team.players.push(r.player);
                team.clubs.push(self.world.player(r.player).club);
            }
        }
        self.world.competition_mut(league).selections_aaa.push(team);
    }
}
