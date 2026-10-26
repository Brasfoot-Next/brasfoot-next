// Port of the new game: best.p.dd() (the leagues to choose from the team files), best.p.de()
// (the countries), best.p.dg() (the clubs, the calendar, the leagues and cups, the youth) with
// what it calls: ah(e.t), F(e.g, boolean, ah), al(String), v(String, int, ah), Z.iT(), Z.Ar(),
// f.B(Z, int), f.s(LoadLigaOptions, ...), f.r(int), f.aS(), a.g(), f.aU(), f.c(false), ay.zj().
// The state leagues, regionals, international cups and national teams are not ported yet.

use std::cmp::Ordering;

use brasfoot_model::countries;
use brasfoot_model::ids::{ClubId, CoachId, CompetitionId, CountryId, PlayerId, StadiumId, StageId};
use brasfoot_model::options::GameOptions;
use brasfoot_model::saved::{Club, Coach, Competition, CompetitionKind, Country, Player, Stadium, Stage, StageKind};
use brasfoot_model::templates::{LeagueConfig, PlayerTemplate, TeamTemplate};
use brasfoot_model::world::World;

use crate::Sim;

/// `best.O`: a national league the new game offers, with its number of teams.
#[derive(Clone, Debug, PartialEq)]
pub struct LeagueChoice {
    pub country: i32,
    pub teams: i32,
    pub selected: bool,
}

/// What `best.p.dd()` gathers before the "Configurações iniciais" screen.
#[derive(Clone, Debug, Default)]
pub struct NewGame {
    /// `f.bC`: the team files, by level (`p.cN`, the files' order among equals).
    pub templates: Vec<TeamTemplate>,
    /// `f.bD`: teams per country.
    pub team_counts: Vec<i32>,
    /// `f.bE`: Brazilian teams per state.
    pub state_counts: Vec<i32>,
    /// `f.bF`: the leagues to choose (countries with 10 teams, 16 for some).
    pub leagues: Vec<LeagueChoice>,
    /// `f.bG`: the league configurations (`conf_ligas_nacionais`).
    pub configs: Vec<LeagueConfig>,
}

/// Countries whose league needs 16 teams (Germany, Argentina, Mexico?, England?, France).
const SIXTEEN: [i32; 5] = [3, 11, 97, 104, 72];
/// `aq.rX`: the Brazilian states.
pub const STATES: usize = 27;

impl NewGame {
    /// `p.dd()` with the team files `U.w(false)` read.
    pub fn new(mut templates: Vec<TeamTemplate>, configs: Vec<LeagueConfig>) -> NewGame {
        let mut team_counts = vec![0; countries::COUNTRIES.len()];
        let mut state_counts = vec![0; STATES];
        for t in &templates {
            if t.country >= 0 && (t.country as usize) < team_counts.len() {
                team_counts[t.country as usize] += 1;
                if t.country == 29 && t.state >= 0 && (t.state as usize) < STATES {
                    state_counts[t.state as usize] += 1;
                }
            }
        }
        templates.sort_by(|a, b| b.level.cmp(&a.level));
        let mut leagues = Vec::new();
        for (country, &n) in team_counts.iter().enumerate() {
            let needed = if SIXTEEN.contains(&(country as i32)) { 16 } else { 10 };
            if n >= needed {
                leagues.push(LeagueChoice { country: country as i32, teams: n, selected: false });
            }
        }
        NewGame { templates, team_counts, state_counts, leagues, configs }
    }

    /// `f.t(int)`: the country's league is chosen.
    fn selected(&self, country: i32) -> bool {
        self.leagues.iter().any(|l| l.country == country && l.selected)
    }

    /// `Z.t(int, int)`: the configuration of a division.
    fn config(&self, country: i32, division: i32) -> Option<&LeagueConfig> {
        self.configs.iter().find(|c| c.country == country && c.division == division)
    }
}

/// `est.LoadLigaOptions` as `Z.Ar()` fills it.
struct LeagueOptions {
    teams: usize,
    two_rounds: bool,
    groups: i32,
    games_within_group: bool,
    knockout_teams: i32,
    two_legged_knockout: Vec<bool>,
    tiebreak: i32,
    qualify_by_overall_table: bool,
    best_third_places: bool,
    round_count: i32,
    division_name: String,
}

impl Default for LeagueOptions {
    fn default() -> Self {
        LeagueOptions {
            teams: 0,
            two_rounds: true,
            groups: 0,
            games_within_group: true,
            knockout_teams: 0,
            two_legged_knockout: vec![true; 7],
            tiebreak: 0,
            qualify_by_overall_table: false,
            best_third_places: false,
            round_count: 0,
            division_name: String::new(),
        }
    }
}

/// `h.a.cN`: by level, then the tie-break number, both descending.
fn by_level(world: &World, a: ClubId, b: ClubId) -> Ordering {
    let (x, y) = (world.club(a), world.club(b));
    y.level.cmp(&x.level).then(y.tiebreak_number.cmp(&x.tiebreak_number))
}

/// `h.a.abm`: by level, reputation, then the tie-break number, all descending.
fn by_level_and_reputation(world: &World, a: ClubId, b: ClubId) -> Ordering {
    let (x, y) = (world.club(a), world.club(b));
    y.level.cmp(&x.level).then(y.reputation.cmp(&x.reputation)).then(y.tiebreak_number.cmp(&x.tiebreak_number))
}

/// `best.n.f(String)` as `h.a.VS` compares club names: without accents.
fn plain(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            'ñ' => 'n',
            'Ñ' => 'N',
            other => other,
        })
        .collect()
}

/// Java's `String.compareTo`: UTF-16 code units.
fn java_compare(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

impl Sim<'_> {
    /// `new best.f()` with the options, and `p.de()`: one country per `best.ac`, each with its
    /// own number drawn (`Z.hr`).
    pub fn start_new_world(&mut self, options: &GameOptions) {
        *self.world = World::default();
        // the field initialisers: the friendlies (f.a) and the friendly tournament (f.L)
        let friendly = CompetitionId(self.world.competitions.len() as u32);
        self.world.competitions.push(Competition { class: CompetitionKind::Friendly, kind: 0, subkind: 0, ..Default::default() });
        self.world.game.friendly = Some(friendly);
        let tournament = CompetitionId(self.world.competitions.len() as u32);
        self.world.competitions.push(Competition { class: CompetitionKind::FriendlyTournament, kind: 15, subkind: 0, ..Default::default() });
        self.world.game.friendly_tournament = Some(tournament);
        // f.aZ = new f.F[4], f.ad = new InfoArquivoSalvoType()
        self.world.game.regional_cups = vec![None; 4];
        self.world.game.summary = Some(Default::default());
        options.apply(&mut self.world.game);
        for info in countries::COUNTRIES.iter() {
            let id = CountryId(self.world.countries.len() as u32);
            let value_hr = self.runtime.rng.fresh_int(50000);
            self.world.countries.push(Country { country: info.id, value_hr, ..Default::default() });
            self.world.game.countries.push(id);
        }
    }

    fn country_of(&self, id: i32) -> CountryId {
        CountryId(self.world.game.countries.iter().position(|&c| self.world.country(c).country == id).expect("p.de() made every country") as u32)
    }

    /// `best.p.dg()` for the leagues chosen in `new_game`, the season starting in `year`
    /// (`f.I(int)`).
    pub fn create_world(&mut self, new_game: &NewGame, options: &GameOptions, year: i32) {
        let g = &self.world.game;
        if g.plays_state || g.plays_regionals || g.plays_international_clubs || g.plays_all_national_teams {
            crate::unported!("p.dg(): state leagues, regionals, international cups and national teams of a new game");
        }
        self.world.game.start_year = year;
        self.world.game.season = 1;
        // p.x(ArrayList): fifteen clubs of each country, every club of a chosen league
        let mut made = vec![0; countries::COUNTRIES.len()];
        for t in &new_game.templates {
            let c = t.country as usize;
            if c >= made.len() {
                continue;
            }
            if made[c] < 15 || new_game.selected(t.country) {
                self.club_from_template(t);
                made[c] += 1;
            }
        }
        // Z.iT()
        for country in self.world.game.countries.clone() {
            let id = self.world.country(country).country;
            let clubs: Vec<ClubId> = self.world.game.clubs.iter().copied().filter(|&c| self.world.club(c).country == id).collect();
            self.world.country_mut(country).clubs_hv.extend(clubs);
        }
        // f.cb(): the grounds by size, a transient list (f.bw)
        // f.r(int), f.aS(), a.g()
        self.year_calendar(year);
        self.assign_day_kinds();
        // p.w(ArrayList): the chosen leagues
        for choice in new_game.leagues.iter().filter(|l| l.selected) {
            let country = self.country_of(choice.country);
            self.create_leagues(country, new_game, options);
            self.world.game.league_countries.push(country);
        }
        self.schedule_promotion_offers();
        // p.dj(): the clubs' game ids
        let clubs = self.world.game.clubs.clone();
        for (i, &c) in clubs.iter().enumerate() {
            self.world.club_mut(c).id = i as i32;
        }
        self.world.game.next_club_id = clubs.len() as i32;
        // f.aU(): the players' strength, the clubs' surplus players and books
        for p in self.world.game.players.clone() {
            self.initial_strength(p);
        }
        for p in self.world.game.youth_players.clone() {
            self.initial_strength(p);
        }
        for c in clubs.clone() {
            self.list_surplus_players(c);
            self.base_books(c);
        }
        // f.c(false): the youth teams
        for c in clubs {
            let club = self.world.club(c);
            if club.league_club && club.reputation >= 3 {
                if club.youth_squad.len() < 12 {
                    for position in [0, 1, 2, 3, 3, 4] {
                        self.new_youth_player(c, position, None, false);
                    }
                }
            } else if club.youth_squad.len() < 5 {
                for position in [0, 1, 2, 3, 4] {
                    self.new_youth_player(c, position, None, false);
                }
            }
        }
        if year == 2023 {
            // f.rU(): the counters of the 2023 season
            let g = &mut self.world.game;
            g.world_cup_season = 4;
            g.euro_season = 2;
            g.copa_america_season = 2;
            g.finalissima_season = 3;
            g.africa_cup_season = 1;
            g.asian_cup_season = 1;
            g.oceania_cup_season = 3;
            g.gold_cup_season = 1;
            g.nations_league_m_season = 1;
            g.nations_league_n_season = 1;
        }
        self.coach_market_clubs();
        // p.df() (no state leagues nor regionals): the Brazilian cup of the first year, with every
        // Brazilian club (Z.ja())
        for country in self.world.game.league_countries.clone() {
            if self.world.country(country).country == 29 {
                let clubs = self.world.country(country).clubs_hv.clone();
                self.new_cup(country, clubs, options);
            }
        }
        // the clubs by name (h.a.VS)
        let mut clubs = self.world.game.clubs.clone();
        clubs.sort_by(|&a, &b| java_compare(&plain(&self.world.club(a).name), &plain(&self.world.club(b).name)));
        self.world.game.clubs = clubs;
    }

    /// `new ah(e.t)`: a club from its team file, with its ground, its coach, its players and
    /// youth players.
    fn club_from_template(&mut self, t: &TeamTemplate) -> ClubId {
        let id = ClubId(self.world.clubs.len() as u32);
        // ah.hr, a field initialiser
        let tiebreak_number = self.runtime.rng.fresh_int(1000) + 1;
        let level = if (1..=25).contains(&t.level) { t.level } else { 10 };
        self.world.clubs.push(Club {
            name: t.name.clone(),
            file_ref: t.file_ref.clone(),
            team_file_id: t.id,
            country: t.country,
            state: t.state,
            reputation: t.reputation,
            color1: t.color1.clone(),
            color2: t.color2.clone(),
            level,
            tiebreak_number,
            base_color: t.base_color,
            ..Default::default()
        });
        // ah.a(String, int): v(String, int, ah)
        let stadium = self.new_stadium(&t.stadium, t.capacity, id);
        self.world.club_mut(id).stadium = Some(stadium);
        // al(String) with its game id (f.bU())
        self.world.game.last_coach_id += 1;
        let coach = CoachId(self.world.coaches.len() as u32);
        self.world.coaches.push(Coach {
            name: t.coach.clone(),
            id: self.world.game.last_coach_id,
            country: t.coach_country,
            club_id: Some(id),
            reputation: t.reputation,
            ..Default::default()
        });
        self.world.club_mut(id).coach = Some(coach);
        self.world.game.coaches.push(coach);
        for p in &t.players {
            let player = self.player_from_template(p, false, id);
            self.world.game.players.push(player);
            self.world.club_mut(id).squad.push(player);
        }
        for p in &t.youth {
            let player = self.player_from_template(p, true, id);
            self.world.game.youth_players.push(player);
            self.world.club_mut(id).youth_squad.push(player);
        }
        self.choose_penalty_taker(id);
        self.choose_captain(id);
        self.world.game.clubs.push(id);
        id
    }

    /// `new F(e.g, boolean, ah)`
    fn player_from_template(&mut self, t: &PlayerTemplate, youth: bool, club: ClubId) -> PlayerId {
        let name = if t.name.is_empty() || t.name == "TESTE" { self.draw_name(t.country).unwrap_or_default() } else { t.name.clone() };
        let mut p = Player { country: t.country, name, club: Some(club), position: t.position, ..Default::default() };
        p.set_side(t.side);
        p.world_class = t.world_class;
        p.set_star(t.star);
        p.set_potential(t.hash);
        p.status = t.status;
        p.trait1 = t.trait1;
        p.trait2 = t.trait2;
        p.youth = youth;
        p.set_age(t.age);
        p.compute_style();
        let id = PlayerId(self.world.players.len() as u32);
        self.world.players.push(p);
        id
    }

    /// `new v(String, int, ah)`: the capacity split by sector (`N(int)`), the prices of the
    /// club's division (`v.a(1, ah)` at creation, division 0) and the pitch drawn (`h(ah)`).
    fn new_stadium(&mut self, name: &str, capacity: i32, club: ClubId) -> StadiumId {
        let total = if (1000..=120000).contains(&capacity) { capacity } else { 10000 };
        let round = |x: f64| brasfoot_model::rng::java_round(x) as i32;
        let mut sectors = vec![0; 4];
        sectors[0] = round(total as f64 * 0.15);
        sectors[2] = round(total as f64 * 0.09);
        sectors[3] = round(total as f64 * 0.009);
        sectors[1] = total - (sectors[0] + sectors[2] + sectors[3]);
        let c = self.world.club(club);
        let prices = brasfoot_engine::stadium::default_prices(self.world, 1, club).to_vec();
        let reputation = c.reputation;
        let country = c.country;
        let draw = self.runtime.rng.fresh_int(100);
        let condition = match reputation {
            4.. => i32::from(draw >= 95),
            3 => {
                if draw >= 50 {
                    1
                } else if draw <= 5 {
                    2
                } else {
                    0
                }
            }
            2 => {
                // the original tests 0 after the 15 and never gets there
                if draw >= 50 {
                    1
                } else if draw <= 15 {
                    2
                } else {
                    0
                }
            }
            _ => match draw {
                90.. => 0,
                50.. => 1,
                25.. => 2,
                _ => 3,
            },
        };
        let id = StadiumId(self.world.stadiums.len() as u32);
        self.world.stadiums.push(Stadium { name: name.to_owned(), capacity: sectors, ticket_prices: prices, condition, country, ..Default::default() });
        id
    }

    /// `ah.kH()`: the division's base cash and yearly income.
    fn base_books(&mut self, club: ClubId) {
        let c = self.world.club(club);
        let division = c.division as usize;
        let mut cash = crate::tables::BASE_CASH[division][0];
        if c.country != 29 && self.world.game.plays_state {
            let wages = self.world.club_wages(club);
            cash = (cash as f64 + 3.2 * wages as f64) as i64;
        }
        let c = self.world.club_mut(club);
        c.cash = cash;
        c.finances.income_type6 += crate::tables::BASE_INCOME[division][0];
    }

    /// `Z.Ar()`: the country's divisions from its best clubs down, each with its table and
    /// fixtures, the clubs outside them, and the cup.
    fn create_leagues(&mut self, country: CountryId, new_game: &NewGame, options: &GameOptions) {
        let id = self.world.country(country).country;
        let mut remaining = self.world.country(country).clubs_hv.clone();
        for &c in &remaining {
            self.world.club_mut(c).league_club = true;
        }
        remaining.sort_by(|&a, &b| by_level(self.world, a, b));
        let all = remaining.clone();
        for division in 1..=4 {
            let config = new_game.config(id, division);
            let defaults = config.is_none_or(|c| c.teams as usize > remaining.len());
            // LoadLigaOptions, with its initialisers where the configuration says nothing
            let mut options = LeagueOptions::default();
            let mut relegated;
            if defaults {
                (options.teams, relegated) = match remaining.len() {
                    20.. => (20, 4),
                    18.. => (18, 2),
                    16.. => (16, 2),
                    14.. => (14, 2),
                    12.. => (12, 2),
                    10.. => (10, 2),
                    _ => (20, 2),
                };
            } else {
                let c = config.expect("checked above");
                options.teams = c.teams as usize;
                options.two_rounds = c.two_rounds;
                options.groups = c.groups;
                options.games_within_group = c.games_within_group;
                options.knockout_teams = c.knockout_teams;
                relegated = c.relegated;
                options.two_legged_knockout = c.two_legged_knockout.clone();
                options.tiebreak = c.tiebreak;
                options.qualify_by_overall_table = c.qualify_by_overall_table;
                options.best_third_places = c.best_third_places;
                options.round_count = c.formula;
                if options.teams <= 10 && relegated > 2 {
                    relegated = 2;
                }
                options.division_name = c.division_name.clone().unwrap_or_default();
            }
            if remaining.len() < options.teams {
                continue;
            }
            if options.groups > 1 || options.knockout_teams > 0 {
                crate::unported!("f.s(LoadLigaOptions, ...): a national league with groups or a knockout");
            }
            let teams = options.teams;
            let clubs: Vec<ClubId> = remaining[..teams].to_vec();
            // f.B(Z, int)
            let number = self.world.country(country).divisions.len() as i32 + 1;
            let competition = CompetitionId(self.world.competitions.len() as u32);
            let mut league = Competition {
                class: CompetitionKind::NationalLeague,
                kind: 1,
                subkind: number,
                division: number,
                country_ref: Some(country),
                relegated,
                relegated_directly: relegated,
                ..Default::default()
            };
            if let Some(c) = config.filter(|_| !defaults) {
                league.flag_agz = c.relegated_by_group;
                league.relegated_directly = c.relegated_directly;
                // f.B.setVagasSobemPeloMataMata(int): more than two is none
                league.promotion_knockout_places = if c.promotion_places > 2 { 0 } else { c.promotion_places };
                league.two_legged_relegation_playoff = c.two_legged_relegation_playoff.clone();
                league.two_legged_promotion_knockout = c.two_legged_promotion_knockout.clone();
                if c.file_version != 22 || c.relegated_directly == 0 || c.relegated_directly > relegated {
                    league.relegated_directly = relegated;
                }
                if league.promotion_knockout_places > 0 {
                    crate::unported!("f.B: the promotion knockout of a configured league");
                }
            }
            self.world.competitions.push(league);
            // f.s(LoadLigaOptions, clubs, 0, null, null, Z, 1, null, false, null, false, f.B)
            self.world.country_mut(country).value_dw += 1;
            let stage_division = self.world.country(country).value_dw;
            let stage = StageId(self.world.stages.len() as u32);
            self.world.stages.push(Stage {
                class: StageKind::League,
                competition: Some(competition),
                kind: 1,
                qualify_by_overall_table: options.qualify_by_overall_table,
                knockout_teams: options.knockout_teams,
                two_legged_rounds: options.two_legged_knockout.clone(),
                group_count: options.groups,
                games_within_group: options.games_within_group,
                double_round: options.two_rounds,
                team_count: teams as i32,
                state_tiebreak: options.tiebreak,
                best_third_places: options.best_third_places,
                round_count: options.round_count,
                division_name: options.division_name.clone(),
                clubs: clubs.clone(),
                division: stage_division,
                country_ref: Some(country),
                ..Default::default()
            });
            self.world.competition_mut(competition).league = Some(stage);
            self.league_fixtures(stage, &clubs, options.two_rounds);
            // f.s.yH(): the number of match days; yI(): the clubs' division
            let rounds = self.world.stage(stage).numbers.len() as i32;
            self.world.stage_mut(stage).value_ze = rounds;
            for &c in &clubs {
                self.world.club_mut(c).division = stage_division;
            }
            self.world.country_mut(country).divisions.push(competition);
            remaining.retain(|c| !clubs.contains(c));
        }
        // Z.iV(): the clubs outside the leagues
        let mut outside: Vec<ClubId> =
            self.world.game.clubs.iter().copied().filter(|&c| self.world.club(c).country == id && self.world.club(c).division == 0).collect();
        outside.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        self.world.country_mut(country).clubs.extend(outside);
        // Brazil gets its cup later (p.df())
        if id != 29 {
            self.new_cup(country, all, options);
        }
    }

    /// `Z.a(ArrayList, false, String)` with no cup yet: `new f.f(Z)` and its draw.
    fn new_cup(&mut self, country: CountryId, clubs: Vec<ClubId>, options: &GameOptions) {
        let id = self.world.country(country).country;
        if id == 29 && clubs.len() >= 91 && options.new_cup_format {
            crate::unported!("f.f.b(): the Brazilian cup's new format");
        }
        if clubs.len() >= 8 && self.world.country(country).cup.is_none() {
            let cup = CompetitionId(self.world.competitions.len() as u32);
            self.world.competitions.push(Competition {
                class: CompetitionKind::NationalCup,
                kind: 2,
                subkind: countries::confederation(id),
                country_ref: Some(country),
                ..Default::default()
            });
            self.world.country_mut(country).cup = Some(cup);
        }
        self.national_cup_draw(country, clubs);
    }
}

impl Sim<'_> {
    /// `ah.b(String, int)` (`bV.or()`): a human coach takes over a club of the new game. The
    /// club's coach is left without a club, remembering it (`al.lN()`).
    pub fn hire_new_human_coach(&mut self, club: ClubId, name: &str, nationality: i32) -> CoachId {
        if let Some(previous) = self.world.club(club).coach {
            self.remember_last_club(previous);
            self.world.coach_mut(previous).club_id = None;
        }
        self.world.game.last_coach_id += 1;
        let coach = CoachId(self.world.coaches.len() as u32);
        let reputation = self.world.club(club).reputation;
        self.world.coaches.push(Coach {
            name: name.to_owned(),
            id: self.world.game.last_coach_id,
            country: nationality,
            human: true,
            club_id: Some(club),
            reputation,
            ..Default::default()
        });
        let c = self.world.club_mut(club);
        c.coach = Some(coach);
        c.human = true;
        // ah.ks(): nobody listed
        for p in self.world.club(club).squad.clone() {
            let player = self.world.player_mut(p);
            player.listed_for_sale = false;
            player.listed_for_loan = false;
        }
        let club_name = self.world.club(club).name.clone();
        brasfoot_engine::messages::send(self.world, coach, 0, 73, club_name, String::new());
        self.world.game.coaches.push(coach);
        self.world.game.human_coaches.push(coach);
        self.world.game.human_clubs.push(club);
        coach
    }
}

/// `bV.cX(int)` for each division of a country: the coach screen sorts the division's table by
/// name (`h.a.VS`, which changes the table's order for good) and lists its clubs without a
/// human coach. One list per division, empty ones included.
pub fn coach_screen_divisions(world: &mut World, country: CountryId) -> Vec<Vec<ClubId>> {
    let mut out = Vec::new();
    for league in world.country(country).divisions.clone() {
        let Some(stage) = world.competition(league).league else {
            out.push(Vec::new());
            continue;
        };
        let mut clubs = world.stage(stage).clubs.clone();
        clubs.sort_by(|&a, &b| java_compare(&plain(&world.club(a).name), &plain(&world.club(b).name)));
        world.stage_mut(stage).clubs = clubs.clone();
        out.push(clubs.into_iter().filter(|&c| !world.club(c).human).collect());
    }
    out
}
