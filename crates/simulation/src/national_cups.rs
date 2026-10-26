// Port of the national teams' tournaments of a new season (best.f.ac()): the countries' levels
// (f.ad()), every tournament starting over (xZ(), mw()), and the draws of the ones of the year:
// the continental cups of national teams (ya() of f.c, f.e, f.h), the Euro qualifiers
// (f.l.fF(int)) and the Nations Leagues (f.M.Ar(), f.N.Ar()).

use brasfoot_model::ids::{ClubId, CompetitionId, CountryId};
use brasfoot_model::saved::{Competition, CompetitionKind};

use crate::Sim;
use crate::stages::StageOptions;

/// `h.a.ad(int, int)`: the draw order of `groups` groups of four from four pots of `groups`
/// places each, every pot shuffled: in each group the first pot, the third, the second and
/// the fourth.
fn four_pots_order(sim: &mut Sim, groups: usize) -> Vec<usize> {
    sim.runtime.checkpoint("h.a.ad");
    let mut pots: Vec<Vec<usize>> = Vec::new();
    for p in 0..4 {
        // h.a.ac(int, int)
        let mut pot: Vec<usize> = (p * groups..p * groups + groups).collect();
        sim.runtime.rng.shuffle(&mut pot);
        pots.push(pot);
    }
    (0..groups).flat_map(|i| [pots[0][i], pots[2][i], pots[1][i], pots[3][i]]).collect()
}

/// The continental cups of national teams: `f.c` (Africa), `f.e` (Asia), `f.h` (Gold Cup),
/// `f.g` (Oceania).
struct CupDraw {
    confederation: i32,
    /// the countries drawn besides the host (one more without it); the Gold Cup draws 16 and,
    /// with its host, keeps 17 of which the groups take the first 16
    picks: i32,
    teams: usize,
    groups: usize,
    best_thirds: bool,
    hosts: [i32; 10],
}

const AFRICA_CUP: CupDraw = CupDraw { confederation: 2, picks: 23, teams: 24, groups: 6, best_thirds: true, hosts: [36, 50, 81, 190, 57, 141, 1, 5, 129, 10] };
const ASIAN_CUP: CupDraw = CupDraw { confederation: 3, picks: 23, teams: 24, groups: 6, best_thirds: true, hosts: [43, 107, 59, 14, 98, 174, 39, 49, 99, 199] };
const OCEANIA_CUP: CupDraw = CupDraw { confederation: 5, picks: 7, teams: 8, groups: 2, best_thirds: false, hosts: [143, 148, 93, 184, 143, 148, 93, 184, 143, 148] };
const GOLD_CUP: CupDraw = CupDraw { confederation: 4, picks: 16, teams: 16, groups: 4, best_thirds: false, hosts: [131, 68, 38, 51, 53, 131, 68, 139, 131, 68] };

impl Sim<'_> {
    /// `f.ac()`
    pub(crate) fn national_tournaments_new_season(&mut self) {
        self.national_levels();
        let g = &self.world.game;
        let cups = [g.world_cup, g.european_championship, g.copa_america, g.africa_cup_of_nations, g.asian_cup, g.gold_cup, g.oceania_nations_cup];
        for cup in cups.into_iter().flatten() {
            self.national_cup_restart(cup);
            self.world.competition_mut(cup).scorers.clear();
        }
        if let Some(f) = self.world.game.finalissima {
            self.world.competition_mut(f).scorers.clear();
        }
        for nl in [self.world.game.nations_league_m, self.world.game.nations_league_n].into_iter().flatten() {
            self.world.competition_mut(nl).scorers.clear();
            self.nations_league_restart(nl);
        }
        let g = &self.world.game;
        let qualifiers = [g.south_american_qualifiers, g.euro_qualifiers, g.african_qualifiers, g.concacaf_qualifiers, g.asian_qualifiers, g.oceania_qualifiers];
        for q in qualifiers.into_iter().flatten() {
            self.qualifiers_restart(q);
        }
        for q in qualifiers.into_iter().flatten() {
            self.world.competition_mut(q).scorers.clear();
        }
        let season = self.world.season();
        if self.world.game.world_cup_season == season {
            if season == 1 {
                crate::unported!("f.q.ya() of the first season: the World Cup of 32");
            }
            let cup = self.world.game.world_cup.expect("the World Cup of the game");
            self.world_cup_draw(cup);
            self.world.game.world_cup_season += 4;
        }
        if self.world.game.euro_season - 1 == season {
            let q = self.national_competition(CompetitionKind::EuroQualifiers, 9, 0);
            self.world.game.euro_qualifiers = Some(q);
            self.euro_qualifiers_draw(q, 0);
        }
        if self.world.game.world_cup_season - 1 == season {
            self.world_cup_qualifiers_draw();
        }
        let g = &self.world.game;
        if g.euro_season == season {
            let euro = self.national_competition(CompetitionKind::EuropeanChampionship, 7, 0);
            self.world.game.european_championship = Some(euro);
            self.euro_draw(euro);
            self.world.game.euro_season += 4;
        }
        // else f.p.p(null): xZ() took its groups already
        let g = &self.world.game;
        if g.copa_america_season == season {
            let cup = self.national_competition(CompetitionKind::CopaAmerica, 7, 1);
            self.world.game.copa_america = Some(cup);
            self.copa_america_draw(cup);
            self.world.game.copa_america_season += 4;
        }
        let g = &self.world.game;
        if g.africa_cup_season == season {
            let cup = self.national_competition(CompetitionKind::AfricaCupOfNations, 7, 2);
            self.world.game.africa_cup_of_nations = Some(cup);
            self.continental_nations_cup_draw(cup, &AFRICA_CUP);
            self.world.game.africa_cup_season += 4;
        }
        if self.world.game.asian_cup_season == season {
            let cup = self.national_competition(CompetitionKind::AsianCup, 7, 3);
            self.world.game.asian_cup = Some(cup);
            self.continental_nations_cup_draw(cup, &ASIAN_CUP);
            self.world.game.asian_cup_season += 4;
        }
        if self.world.game.gold_cup_season == season {
            let cup = self.national_competition(CompetitionKind::GoldCup, 7, 4);
            self.world.game.gold_cup = Some(cup);
            self.continental_nations_cup_draw(cup, &GOLD_CUP);
            self.world.game.gold_cup_season += 2;
        }
        let g = &self.world.game;
        if g.oceania_cup_season == season {
            let cup = self.national_competition(CompetitionKind::OceaniaNationsCup, 7, 5);
            self.world.game.oceania_nations_cup = Some(cup);
            self.continental_nations_cup_draw(cup, &OCEANIA_CUP);
            self.world.game.oceania_cup_season += 4;
        }
        let g = &self.world.game;
        if g.finalissima_season == season {
            if let Some(f) = g.finalissima {
                self.finalissima_draw(f);
            }
            self.world.game.finalissima_season += 4;
        }
        if self.world.game.nations_league_m_season == season {
            if let Some(nl) = self.world.game.nations_league_m {
                self.nations_league_draw(nl);
            }
            self.world.game.nations_league_m_season += 2;
        }
        if self.world.game.nations_league_n_season == season {
            if let Some(nl) = self.world.game.nations_league_n {
                self.nations_league_draw(nl);
            }
            self.world.game.nations_league_n_season += 2;
        }
    }

    /// `f.ad()`: each country's level from the game's table (`Z.jm()`) and whether it has
    /// fifteen outfield players and two keepers (`Z.jj()`).
    fn national_levels(&mut self) {
        for country in self.world.game.countries.clone() {
            let id = self.world.country(country).country;
            let level = brasfoot_model::countries::country(id).map(|c| c.level).unwrap_or(0);
            let (mut outfield, mut keepers) = (0, 0);
            for &p in &self.world.game.players {
                let pl = self.world.player(p);
                if pl.country == id {
                    if pl.position == 0 {
                        keepers += 1;
                    } else {
                        outfield += 1;
                    }
                }
            }
            let c = self.world.country_mut(country);
            c.level = level;
            c.flag_hz = outfield >= 15 && keepers >= 2;
        }
    }

    /// The tournament of the game's field, or a new one (`new f.l()`, `c.a.SR.a(...)`): kind
    /// and subkind of its constructor; its name is the screens'.
    pub(crate) fn national_competition(&mut self, class: CompetitionKind, kind: i32, subkind: i32) -> CompetitionId {
        let g = &self.world.game;
        let existing = match class {
            CompetitionKind::EuroQualifiers => g.euro_qualifiers,
            CompetitionKind::EuropeanChampionship => g.european_championship,
            CompetitionKind::CopaAmerica => g.copa_america,
            CompetitionKind::SouthAmericanQualifiers => g.south_american_qualifiers,
            CompetitionKind::AfricanQualifiers => g.african_qualifiers,
            CompetitionKind::ConcacafQualifiers => g.concacaf_qualifiers,
            CompetitionKind::AsianQualifiers => g.asian_qualifiers,
            CompetitionKind::OceaniaQualifiers => g.oceania_qualifiers,
            CompetitionKind::AfricaCupOfNations => g.africa_cup_of_nations,
            CompetitionKind::AsianCup => g.asian_cup,
            CompetitionKind::GoldCup => g.gold_cup,
            CompetitionKind::OceaniaNationsCup => g.oceania_nations_cup,
            other => unreachable!("{other:?} is made elsewhere"),
        };
        if let Some(c) = existing {
            return c;
        }
        let mut c = Competition { class, kind, subkind, ..Default::default() };
        // O(true) of the cups (f.c, f.e, f.h, f.p, f.d); the qualifiers have no third place
        if !matches!(
            class,
            CompetitionKind::EuroQualifiers
                | CompetitionKind::SouthAmericanQualifiers
                | CompetitionKind::AfricanQualifiers
                | CompetitionKind::ConcacafQualifiers
                | CompetitionKind::AsianQualifiers
                | CompetitionKind::OceaniaQualifiers
        ) {
            c.third_place_match = true;
        }
        let id = CompetitionId(self.world.competitions.len() as u32);
        self.world.competitions.push(c);
        id
    }

    /// `xZ()` of f.q, f.p, f.d, f.c, f.e, f.h and f.g: the groups start over and go, with the
    /// countries and the teams (and the Gold Cup's final round).
    fn national_cup_restart(&mut self, cup: CompetitionId) {
        if let Some(main) = self.world.competition(cup).main_stage {
            self.restart_stage(main, true);
        }
        let c = self.world.competition_mut(cup);
        c.main_stage = None;
        if c.class == CompetitionKind::GoldCup {
            c.final_round = None;
        }
        c.countries.clear();
        c.clubs.clear();
    }

    /// `xZ()` of the World Cup's qualifiers (f.n, f.l, f.i, f.k, f.j, f.m): their stages start
    /// over and go.
    fn qualifiers_restart(&mut self, q: CompetitionId) {
        use CompetitionKind::*;
        let c = self.world.competition(q);
        let first = if c.class == ConcacafQualifiers { c.stage_afx } else { c.main_stage };
        if let Some(stage) = first {
            self.restart_stage(stage, true);
        }
        let c = self.world.competition_mut(q);
        match c.class {
            SouthAmericanQualifiers => {
                c.main_stage = None;
                c.club_agf = None;
                c.countries.clear();
                c.clubs.clear();
            }
            EuroQualifiers => {
                c.knockout_afv = None;
                c.main_stage = None;
            }
            AfricanQualifiers => {
                c.main_stage = None;
                c.final_round = None;
                c.knockout_afv = None;
                c.clubs.clear();
                c.club_afw = None;
            }
            ConcacafQualifiers => {
                c.stage_agb = None;
                c.stage_afx = None;
                c.clubs_agc.clear();
                c.club_afw = None;
            }
            AsianQualifiers => {
                c.stage_afz = None;
                c.main_stage = None;
                c.final_round = None;
                c.stage_afx = None;
                c.knockout_afy = None;
                c.club_aga = None;
                c.clubs.clear();
            }
            OceaniaQualifiers => {
                c.main_stage = None;
                c.stage_agb = None;
                c.club_agf = None;
                c.playoff_round = None;
                c.clubs.clear();
            }
            other => unreachable!("{other:?} is not a qualifier"),
        }
    }

    /// `xZ()` of f.M and f.N: the leagues' moves (`BD()`), then the leagues start over and go.
    fn nations_league_restart(&mut self, nl: CompetitionId) {
        if self.world.competition(nl).class == CompetitionKind::NationsLeagueM {
            self.runtime.checkpoint("f.M.xZ");
        }
        if self.world.competition(nl).league_a.is_some() {
            self.nations_league_moves(nl);
        }
        let c = self.world.competition(nl);
        for league in [c.league_a, c.league_b, c.league_c, c.league_d].into_iter().flatten() {
            self.restart_stage(league, true);
        }
        let c = self.world.competition_mut(nl);
        let europe = c.class == CompetitionKind::NationsLeagueM;
        c.league_a = None;
        c.league_b = None;
        c.league_c = None;
        c.league_d = None;
        c.playoff_round = None;
        if europe {
            c.knockout_agx = None;
        }
    }

    /// `BD()` of f.M and f.N: the last of each group goes down, swapping places with the winner
    /// of the same group a league below (in f.M the last two of league C with the losers of the
    /// relegation playoff, `xc`); each keeps its move in last season's record (`ah.b(at, int)`).
    fn nations_league_moves(&mut self, nl: CompetitionId) {
        let c = self.world.competition(nl);
        let europe = c.class == CompetitionKind::NationsLeagueM;
        let leagues = [c.league_a, c.league_b, c.league_c, c.league_d];
        let place = |sim: &Self, league: Option<brasfoot_model::ids::StageId>, group: usize, i: usize| -> ClubId {
            sim.world.stage(league.expect("the Nations League's leagues")).groups[group].clubs[i]
        };
        let groups = |sim: &Self, league: Option<brasfoot_model::ids::StageId>| sim.world.stage(league.expect("a league")).groups.len();
        // f.N's league A has groups of three
        let last_of_a = if europe { 3 } else { 2 };
        let mut swaps: Vec<(usize, ClubId, ClubId)> = Vec::new();
        for g in 0..groups(self, leagues[0]) {
            swaps.push((0, place(self, leagues[0], g, last_of_a), place(self, leagues[1], g, 0)));
        }
        // each swap reads the tables after the ones before it, as the original does
        for (upper, down, up) in swaps.drain(..) {
            self.nations_league_swap(nl, upper, down, up);
        }
        for g in 0..groups(self, leagues[1]) {
            swaps.push((1, place(self, leagues[1], g, 3), place(self, leagues[2], g, 0)));
        }
        for (upper, down, up) in swaps.drain(..) {
            self.nations_league_swap(nl, upper, down, up);
        }
        if europe {
            let xc = self.world.competition(nl).list_xc.clone();
            for (i, &down) in xc.iter().take(2).enumerate() {
                let up = place(self, leagues[3], i, 0);
                self.nations_league_swap(nl, 2, down, up);
            }
            self.world.competition_mut(nl).list_xc.clear();
        }
    }

    /// `f.M.a(ArrayList, ArrayList, int, int)`: the team going down and the one going up swap
    /// places in the leagues' lists.
    fn nations_league_swap(&mut self, nl: CompetitionId, upper: usize, down: ClubId, up: ClubId) {
        let c = self.world.competition_mut(nl);
        let lists = [&mut c.clubs_a, &mut c.clubs_b, &mut c.clubs_c, &mut c.clubs_d];
        let [a, b, cc, d] = lists;
        let (high, low) = match upper {
            0 => (a, b),
            1 => (b, cc),
            _ => (cc, d),
        };
        let i = high.iter().position(|&x| x == down).expect("the team is in its league");
        let j = low.iter().position(|&x| x == up).expect("the team is in its league");
        high[i] = up;
        low[j] = down;
        self.last_season_move(down, nl, 0);
        self.last_season_move(up, nl, 1);
    }

    /// `ah.b(at, int)`: the move (0 down, 1 up) in the team's record of the tournament last
    /// season, when it has one.
    fn last_season_move(&mut self, team: ClubId, competition: CompetitionId, movement: i32) {
        let last = self.world.season() - 1;
        let c = self.world.competition(competition);
        let (kind, subkind) = (c.kind, c.subkind);
        if let Some(r) = self.world.club_mut(team).competition_records.iter_mut().find(|r| r.season == last && r.competition_kind == kind && r.competition_subkind == subkind) {
            r.league_move = movement;
        }
    }

    /// `Z.a(boolean, int, int, ArrayList, boolean, Z)`: `count` countries of the
    /// confederation, by level, each with a chance by its tier of levels (lower as the levels
    /// go down), the rest in order after.
    pub(crate) fn pick_countries(&mut self, chances_kind: bool, confederation: i32, mut count: i32, out: &mut Vec<CountryId>, world_cup: bool, except: Option<CountryId>) {
        self.runtime.checkpoint("best.Z.a(ZIILjava/util/ArrayList;ZLbest/Z;)V");
        let mut list: Vec<CountryId> = self.world.game.countries.iter().copied().filter(|&c| brasfoot_model::countries::confederation(self.world.country(c).country) == confederation).collect();
        list.sort_by(|&a, &b| {
            let (ca, cb) = (self.world.country(a), self.world.country(b));
            cb.level.cmp(&ca.level).then(cb.value_hr.cmp(&ca.value_hr))
        });
        let mut chances: &[i32] = match (chances_kind, confederation) {
            (true, 0) => &[95, 80, 70, 50, 30, 20, 10, 5],
            (true, 1) => &[100, 95, 30, 10, 5],
            (true, 2) => &[80, 80, 70, 70, 70, 50],
            (true, 3) => &[90, 50, 30, 10, 5],
            (true, 4) => &[100, 90, 80, 60, 30],
            (true, 5) => &[99, 95, 30, 10, 5],
            (false, 0) => &[100, 100, 90, 70, 60, 20, 10, 5],
            (false, 1) => &[100, 50, 30, 10, 5],
            (false, 2) => &[100, 100, 100, 98, 60, 40],
            (false, 3) => &[100, 100, 90, 65, 60],
            (false, 4) => &[100, 100, 100, 100, 100, 95, 80],
            (false, 5) => &[100, 90, 90, 10, 5],
            _ => &[100, 95, 80, 50, 30, 20, 10, 5],
        };
        if world_cup {
            // the qualifiers: everyone in, and the next World Cup's host out (f.q.eY(1))
            chances = match confederation {
                4 => &[100, 100, 100, 95, 75, 70, 70, 70, 70],
                3 => &[100, 100, 70, 70, 70, 70, 70, 70, 70],
                _ => &[100, 100, 100, 100, 100, 100, 100, 100, 50],
            };
            if let Some(host) = self.world_cup_host(1) {
                list.retain(|&c| c != host);
            }
        }
        let mut tier = 0;
        let mut last_level = 0;
        for (i, &c) in list.iter().enumerate() {
            if count > 0 {
                let roll = self.runtime.rng.fresh_int(100) + 1;
                let level = self.world.country(c).level;
                if i > 0 && level < last_level {
                    tier = (tier + 1).min(chances.len() - 1);
                }
                if Some(c) != except && roll <= chances[tier] && !out.contains(&c) {
                    out.push(c);
                    count -= 1;
                }
                last_level = level;
            }
        }
        for &c in &list {
            if count > 0 && Some(c) != except && !out.contains(&c) {
                out.push(c);
                count -= 1;
            }
        }
    }

    /// `ya()` of f.c, f.e and f.h: the edition's host (when it has the players) and the
    /// confederation's countries, each calling up its squad with a new coach; groups by the
    /// pots of `h.a.ad` of the teams as they were picked (the original sorts the still empty
    /// list of teams before), single games.
    fn continental_nations_cup_draw(&mut self, cup: CompetitionId, draw: &CupDraw) {
        self.runtime.checkpoint(match draw.confederation {
            2 => "f.c.ya",
            3 => "f.e.ya",
            5 => "f.g.ya",
            _ => "f.h.ya",
        });
        let c = self.world.competition_mut(cup);
        c.countries.clear();
        c.clubs.clear();
        c.main_stage = None;
        if c.class == CompetitionKind::GoldCup {
            c.final_round = None;
        }
        c.edition_counter += 1;
        let edition = c.edition_counter;
        let mut count = draw.picks;
        let host = self.any_country(draw.hosts[((edition - 1) % 10) as usize]);
        self.world.competition_mut(cup).host = host;
        let mut countries = Vec::new();
        match host {
            Some(h) if self.world.country(h).flag_hz => countries.push(h),
            _ => count += 1,
        }
        self.pick_countries(false, draw.confederation, count, &mut countries, false, None);
        self.world.competition_mut(cup).countries = countries.clone();
        if countries.len() >= draw.teams {
            let mut teams: Vec<ClubId> = Vec::new();
            for &country in &countries {
                teams.push(self.national_team(country));
                self.call_up(country, true);
            }
            let order = four_pots_order(self, draw.groups);
            let drawn: Vec<ClubId> = order.iter().map(|&i| teams[i]).collect();
            self.world.competition_mut(cup).clubs = drawn.clone();
            let options = StageOptions {
                teams: draw.teams,
                groups: draw.groups as i32,
                knockout_teams: 2,
                two_rounds: false,
                two_legged_knockout: vec![false; 7],
                best_third_places: draw.best_thirds,
                ..Default::default()
            };
            let stage = self.new_league_stage(&options, &drawn, 0, 7, cup, true);
            self.world.stage_mut(stage).name = self.world.competition(cup).name.clone();
            self.world.competition_mut(cup).main_stage = Some(stage);
            self.job_market_countries(&drawn);
        }
    }

    /// `f.l.fF(int)`: the Euro qualifiers (`age` 0, subkind 70) or the World Cup's European
    /// qualifiers (7, subkind 0): every country calls up its squad with a new coach; the
    /// European ones by level in six pots, ten groups of five and six (9001).
    pub(crate) fn euro_qualifiers_draw(&mut self, q: CompetitionId, age: i32) {
        self.runtime.checkpoint("f.l.fF");
        let c = self.world.competition_mut(q);
        c.value_age = age;
        c.kind = 9;
        c.subkind = if age == 7 { 0 } else { 70 };
        let mut european: Vec<CountryId> = Vec::new();
        for country in self.world.game.countries.clone() {
            self.national_team(country);
            if brasfoot_model::countries::confederation(self.world.country(country).country) == 0 {
                european.push(country);
            }
            self.call_up(country, true);
        }
        european.sort_by(|&a, &b| {
            let (ca, cb) = (self.world.country(a), self.world.country(b));
            cb.level.cmp(&ca.level).then(cb.value_hr.cmp(&ca.value_hr))
        });
        let teams: Vec<ClubId> = european.iter().map(|&c| self.world.country(c).national_team.expect("checked")).collect();
        let mut pots: Vec<Vec<ClubId>> = [0..10, 10..20, 20..30, 30..40, 40..50, 50..55].into_iter().map(|r| teams[r].to_vec()).collect();
        for pot in pots.iter_mut() {
            self.runtime.rng.shuffle(pot);
        }
        let mut drawn = Vec::new();
        for i in 0..10 {
            for pot in &pots[..5] {
                drawn.push(pot[i]);
            }
            if i > 4 {
                drawn.push(pots[5][i - 5]);
            }
        }
        let options = StageOptions { teams: 55, groups: 10, two_rounds: true, group_sizes: Some(vec![5, 5, 5, 5, 5, 6, 6, 6, 6, 6]), ..Default::default() };
        let stage = self.new_league_stage(&options, &drawn, 0, 9, q, true);
        self.world.stage_mut(stage).format_code = 9001;
        self.world.stage_mut(stage).name = self.world.competition(q).name.clone();
        self.world.competition_mut(q).main_stage = Some(stage);
        self.job_market_countries(&drawn);
    }

    /// `Ar()` of f.M and f.N: the leagues' teams (the first time from the game's lists), in
    /// pots, each calling up its squad; leagues A to D (14001-14004, 14401-14403).
    fn nations_league_draw(&mut self, nl: CompetitionId) {
        if self.world.competition(nl).class == CompetitionKind::NationsLeagueM {
            self.runtime.checkpoint("f.M.Ar");
        }
        const M: [&[i32]; 4] = [
            &[72, 65, 104, 21, 154, 85, 54, 3, 97, 152, 180, 52, 145, 15, 159, 88],
            &[193, 179, 27, 102, 70, 142, 62, 192, 103, 160, 171, 100, 64, 105, 2, 12],
            &[162, 63, 31, 101, 78, 25, 120, 122, 119, 76, 16, 110, 40, 44, 114, 92],
            &[66, 134, 118, 128, 216, 164, 4],
        ];
        const N: [&[i32]; 3] = [
            &[131, 68, 38, 51, 147, 106, 58, 86, 219, 8, 181, 77],
            &[84, 79, 189, 53, 220, 217, 139, 24, 7, 80, 158, 168, 22, 212, 20, 17],
            &[166, 153, 221, 165, 56, 13, 90, 213, 222, 223, 211, 6, 94],
        ];
        let europe = self.world.competition(nl).class == CompetitionKind::NationsLeagueM;
        if self.world.competition(nl).edition_counter == 0 {
            let lists: &[&[i32]] = if europe { &M } else { &N };
            let mut teams: Vec<Vec<ClubId>> = Vec::new();
            for ids in lists {
                teams.push(ids.iter().map(|&id| self.any_country(id).and_then(|c| self.world.country(c).national_team).expect("Z.jo()")).collect());
            }
            let c = self.world.competition_mut(nl);
            c.clubs_a.extend(&teams[0]);
            c.clubs_b.extend(&teams[1]);
            c.clubs_c.extend(&teams[2]);
            if europe {
                c.clubs_d.extend(&teams[3]);
            }
        }
        let count = if europe { 4 } else { 3 };
        let codes = if europe { [14001, 14002, 14003, 14004] } else { [14401, 14402, 14403, 0] };
        let mut stages = Vec::new();
        for i in 0..count {
            let c = self.world.competition(nl);
            let mut list = [&c.clubs_a, &c.clubs_b, &c.clubs_c, &c.clubs_d][i].clone();
            self.nations_league_pots(&mut list, europe);
            let c = self.world.competition_mut(nl);
            *[&mut c.clubs_a, &mut c.clubs_b, &mut c.clubs_c, &mut c.clubs_d][i] = list;
        }
        for i in 0..count {
            let c = self.world.competition(nl);
            let list = [&c.clubs_a, &c.clubs_b, &c.clubs_c, &c.clubs_d][i].clone();
            stages.push(self.nations_league_league(nl, &list, codes[i], europe));
        }
        let c = self.world.competition_mut(nl);
        c.league_a = stages[0];
        c.league_b = stages[1];
        c.league_c = stages[2];
        if europe {
            c.league_d = stages[3];
        }
        c.value_agy = -1;
        c.edition_counter += 1;
        for i in 0..count {
            let c = self.world.competition(nl);
            let list = [&c.clubs_a, &c.clubs_b, &c.clubs_c, &c.clubs_d][i].clone();
            self.job_market_countries(&list);
        }
    }

    /// `aB(ArrayList)` of f.M and f.N: sixteen in four pots of four; in f.M seven in pots of
    /// four and three; in f.N twelve in three pots of four, and fifteen in pots of four, four
    /// and five that keep only thirteen (the original drops the last two), thirteen as they are.
    fn nations_league_pots(&mut self, list: &mut Vec<ClubId>, europe: bool) {
        let pots_of = |sim: &mut Self, list: &mut Vec<ClubId>, bounds: &[(usize, usize)]| -> Vec<Vec<ClubId>> {
            let mut pots: Vec<Vec<ClubId>> = bounds.iter().map(|&(a, b)| list[a..=b].to_vec()).collect();
            for pot in pots.iter_mut() {
                sim.runtime.rng.shuffle(pot);
            }
            list.clear();
            pots
        };
        if list.len() == 16 {
            let pots = pots_of(self, list, &[(0, 3), (4, 7), (8, 11), (12, 15)]);
            for i in 0..4 {
                for pot in &pots {
                    list.push(pot[i]);
                }
            }
            return;
        }
        if europe {
            let pots = pots_of(self, list, &[(0, 3), (4, 6)]);
            let (a, b) = (&pots[0], &pots[1]);
            *list = vec![a[0], b[0], a[1], b[1], a[2], a[3], b[2]];
            return;
        }
        if list.len() == 15 {
            let pots = pots_of(self, list, &[(0, 3), (4, 7), (8, 12)]);
            for i in 0..4 {
                for pot in &pots {
                    list.push(pot[i]);
                }
            }
            list.push(pots[2][4]);
        }
        if list.len() == 12 {
            let pots = pots_of(self, list, &[(0, 3), (4, 7), (8, 11)]);
            for i in 0..4 {
                for pot in &pots {
                    list.push(pot[i]);
                }
            }
        }
    }

    /// `e(ArrayList, int)` of f.M and f.N: each team calls up its squad; a league of four groups
    /// (in f.M two of four and three with seven teams, none but with 16 or 7), home and away.
    fn nations_league_league(&mut self, nl: CompetitionId, teams: &[ClubId], code: i32, europe: bool) -> Option<brasfoot_model::ids::StageId> {
        if europe && teams.len() != 16 && teams.len() != 7 {
            return None;
        }
        for &t in teams {
            let id = self.world.club(t).country;
            if let Some(country) = self.any_country(id) {
                self.call_up(country, false);
            }
        }
        let mut options = StageOptions { teams: teams.len(), groups: 4, knockout_teams: 0, two_rounds: true, two_legged_knockout: vec![false; 7], ..Default::default() };
        if europe && teams.len() == 7 {
            options.groups = 2;
            options.group_sizes = Some(vec![4, 3]);
        }
        // f.N.e(ArrayList, int): thirteen in groups of four, three, three and three
        if !europe && teams.len() == 13 {
            options.group_sizes = Some(vec![4, 3, 3, 3]);
        }
        let stage = self.new_league_stage(&options, teams, 0, 14, nl, true);
        self.world.stage_mut(stage).format_code = code;
        self.world.stage_mut(stage).name = self.world.competition(nl).name.clone();
        Some(stage)
    }

    /// `best.ay.L(ArrayList)`: the countries of the AI national teams join the coaches' job
    /// market, the second list shuffled.
    pub(crate) fn job_market_countries(&mut self, teams: &[ClubId]) {
        self.runtime.checkpoint("best.ay.L");
        for &t in teams {
            if !self.world.club(t).human {
                let id = self.world.club(t).country;
                let m = &mut self.world.job_market;
                if !m.countries.contains(&id) {
                    m.countries.push(id);
                }
                if !m.shuffled_countries.contains(&id) {
                    m.shuffled_countries.push(id);
                }
            }
        }
        let mut list = std::mem::take(&mut self.world.job_market.shuffled_countries);
        self.runtime.rng.shuffle(&mut list);
        self.world.job_market.shuffled_countries = list;
    }

    /// `f.l.Bk()` (the end of the groups, 9001): for the Euro the eight best thirds (by the
    /// table of each group, the groups of five and six alike), for the World Cup the seconds and
    /// the two best thirds, in the two-legged playoff (9002).
    pub(crate) fn euro_qualifiers_playoff(&mut self, q: CompetitionId) {
        let c = self.world.competition(q);
        let main = c.main_stage.expect("the qualifiers' groups");
        let age = c.value_age;
        let groups = self.world.stage(main).groups.clone();
        let mut thirds: Vec<ClubId> = groups.iter().map(|g| g.clubs[2]).collect();
        // s.r(YD): the table h.a.abn reads
        self.runtime.table_stage = Some(main);
        thirds.sort_by(|&a, &b| crate::sorting::standings(self.world, main, a, b));
        let clubs: Vec<ClubId> = if age == 7 {
            let mut c: Vec<ClubId> = groups.iter().map(|g| g.clubs[1]).collect();
            c.extend_from_slice(&thirds[..2]);
            c
        } else {
            thirds[..8].to_vec()
        };
        let stage = self.cup_knockout(q, &clubs, 1, vec![true; 7], 9002);
        self.world.competition_mut(q).knockout_afv = Some(stage);
    }

    /// `f.l.az(ArrayList)` (the end of the playoff, 9002): the group winners and runners-up and
    /// the playoff's winners go to the Euro (`f.p.a(at, Z)`), each with division 1 in its record
    /// of the qualifiers.
    pub(crate) fn euro_qualifiers_qualified(&mut self, q: CompetitionId, winners: &[ClubId]) {
        let c = self.world.competition(q);
        if c.value_age == 7 {
            self.european_world_cup_qualified(q, winners);
            return;
        }
        let main = c.main_stage.expect("the qualifiers' groups");
        let mut qualified = Vec::new();
        for g in &self.world.stage(main).groups {
            qualified.push(g.clubs[0]);
            qualified.push(g.clubs[1]);
        }
        qualified.extend_from_slice(winners);
        let euro = self.world.game.european_championship.expect("the Euro of the game");
        let subkind = self.world.competition(q).subkind;
        for team in qualified {
            let id = self.world.club(team).country;
            let country = self.any_country(id).expect("a country of the game");
            self.world.competition_mut(euro).list_ym.push(country);
            // ah.e(at, int)
            self.competition_record(team, q, subkind).division = 1;
        }
    }

    /// `f.p.ya()`: the Euro: the 24 qualified (`YM`) call up their squads with new coaches, in
    /// six groups by the pots of `h.a.ad` as they qualified; the edition's host (none the first
    /// of every ten) only names the cities.
    fn euro_draw(&mut self, euro: CompetitionId) {
        const HOSTS: [i32; 10] = [72, 3, 104, 97, 65, 154, 162, 193, 78, 152];
        let c = self.world.competition_mut(euro);
        c.countries.clear();
        c.clubs.clear();
        c.main_stage = None;
        c.edition_counter += 1;
        let i = ((c.edition_counter - 1) % 10) as usize;
        let countries = std::mem::take(&mut c.list_ym);
        c.countries = countries.clone();
        let host = if i == 0 { None } else { self.any_country(HOSTS[i]) };
        self.world.competition_mut(euro).host = host;
        if countries.len() != 24 {
            return;
        }
        let mut teams = Vec::new();
        for &country in &countries {
            teams.push(self.national_team(country));
            self.call_up(country, true);
        }
        let order = four_pots_order(self, 6);
        let drawn: Vec<ClubId> = order.iter().map(|&i| teams[i]).collect();
        self.world.competition_mut(euro).clubs = drawn.clone();
        let options = StageOptions { teams: 24, groups: 6, knockout_teams: 2, two_rounds: false, two_legged_knockout: vec![false; 7], best_third_places: true, ..Default::default() };
        let stage = self.new_league_stage(&options, &drawn, 0, 7, euro, true);
        self.world.stage_mut(stage).name = self.world.competition(euro).name.clone();
        self.world.competition_mut(euro).main_stage = Some(stage);
        self.job_market_countries(&drawn);
    }

    /// `f.d.ya()`: the Copa América: ten South American countries, each calling up its squad
    /// with a new coach, in two groups of five by the pairs of `h.a.Al()`; four of each go on.
    fn copa_america_draw(&mut self, cup: CompetitionId) {
        const HOSTS: [i32; 10] = [11, 60, 46, 29, 195, 150, 26, 198, 42, 151];
        let c = self.world.competition_mut(cup);
        c.countries.clear();
        c.clubs.clear();
        c.main_stage = None;
        c.edition_counter += 1;
        let i = ((c.edition_counter - 1) % 10) as usize;
        let host = self.any_country(HOSTS[i]);
        self.world.competition_mut(cup).host = host;
        let mut countries = Vec::new();
        self.pick_countries(false, 1, 10, &mut countries, false, None);
        self.world.competition_mut(cup).countries = countries.clone();
        if countries.len() < 10 {
            return;
        }
        let mut teams = Vec::new();
        for &country in &countries {
            teams.push(self.national_team(country));
            self.call_up(country, true);
        }
        // h.a.Al(): five shuffled pairs, the first of each pair in the first group
        let mut pairs: Vec<Vec<usize>> = Vec::new();
        for p in 0..5 {
            let mut pair = vec![2 * p, 2 * p + 1];
            self.runtime.rng.shuffle(&mut pair);
            pairs.push(pair);
        }
        let order: Vec<usize> = (0..2).flat_map(|i| pairs.iter().map(move |p| p[i]).collect::<Vec<_>>()).collect();
        let drawn: Vec<ClubId> = order.iter().map(|&i| teams[i]).collect();
        self.world.competition_mut(cup).clubs = drawn.clone();
        let options = StageOptions { teams: 10, groups: 2, knockout_teams: 4, two_rounds: false, two_legged_knockout: vec![false; 7], ..Default::default() };
        let stage = self.new_league_stage(&options, &drawn, 0, 7, cup, true);
        self.world.stage_mut(stage).name = self.world.competition(cup).name.clone();
        self.world.competition_mut(cup).main_stage = Some(stage);
        self.job_market_countries(&drawn);
    }

    /// `f.K.Bs()`: the Finalíssima of the Euro's and the Copa América's champions, one game;
    /// its city drawn, by edition from South America or Europe, until it is in neither
    /// country (`ag(int, int)`).
    fn finalissima_draw(&mut self, f: CompetitionId) {
        let c = self.world.competition_mut(f);
        c.host_city = None;
        let (Some(a), Some(b)) = (c.club_agg, c.club_agh) else { return };
        let stage = self.new_knockout_stage(f, None, 2, 13, 0, vec![false; 7], -1);
        self.world.competition_mut(f).final_stage = Some(stage);
        self.new_knockout_round(stage, &[a, b], 0, false, 0, 13, false);
        let c = self.world.competition_mut(f);
        c.edition_counter += 1;
        let cities: &[(&str, i32)] = if c.edition_counter % 2 == 0 {
            &brasfoot_model::host_cities::FINALISSIMA_SOUTH_AMERICA
        } else {
            &brasfoot_model::host_cities::FINALISSIMA_EUROPE
        };
        let (ca, cb) = (self.world.club(a).country, self.world.club(b).country);
        loop {
            let (city, country) = cities[self.runtime.rng.fresh_int(cities.len() as i32) as usize];
            if country != ca && country != cb {
                self.world.competition_mut(f).host_city = Some(city.to_owned());
                break;
            }
        }
    }
}
