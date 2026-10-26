// Port of bfmod.Temporada: every award of one season, from the per-season stats the game keeps
// (best.k) and each competition's result for that season (best.i).

use std::collections::{HashMap, HashSet};

use brasfoot_model::ids::{ClubId, CompetitionId, PlayerId};
use brasfoot_model::mods::{SeasonTitle, TeamSlot};
use brasfoot_model::saved::{CompetitionKind, TitleRecord};
use brasfoot_model::world::World;

use crate::economy::{is_national_team, league_weights};

/// `MIN_JOGOS`: games for the Ballon d'Or and the others.
pub const MIN_GAMES: i32 = 15;
/// `MIN_NOTAS`
pub const MIN_RATINGS: i32 = 10;
/// `MIN_JOGOS_CHUTEIRA`
pub const MIN_GAMES_SHOE: i32 = 10;
/// `IDADE_GOLDEN_BOY`
pub const GOLDEN_BOY_AGE: i32 = 21;
/// `PESO_VICE`: a runner-up place counts this share of the title.
pub const RUNNER_UP_WEIGHT: f64 = 0.35;

/// `Temporada.chave(F)`: name, nationality and birth year. The game renumbers its players on
/// every save, and a retired player is recycled with a new name and age, hence a new key.
pub fn player_key(world: &World, player: PlayerId) -> String {
    let p = world.player(player);
    key(&p.name, p.country, world.game.start_year + world.season() - 1 - p.age)
}

/// `Temporada.chave(String, int, int)`
pub fn key(name: &str, country: i32, birth_year: i32) -> String {
    format!("{name}|{country}|{birth_year}")
}

/// `Temporada.Candidato`
#[derive(Clone, Debug)]
pub struct Candidate {
    pub player: PlayerId,
    pub key: String,
    /// The current club in the season in progress, else the club he played most for.
    pub club: Option<ClubId>,
    pub age: i32,
    pub games: i32,
    pub goals: i32,
    pub assists: i32,
    pub rating: f64,
    pub points: f64,
    pub league_weight: f64,
    /// The titles of his clubs that season. The original walks an identity set, so its order
    /// varies from run to run; here it is the order the clubs come up.
    pub titles: Vec<SeasonTitle>,
}

impl Candidate {
    /// `golsPonderados()`
    pub fn weighted_goals(&self) -> f64 {
        self.goals as f64 * self.league_weight
    }

    /// `elegivel()`
    fn eligible(&self) -> bool {
        self.games >= MIN_GAMES
    }
}

/// `Temporada`
#[derive(Clone, Debug)]
pub struct Season {
    pub season: i32,
    pub year: i32,
    pub current: bool,
    /// `todos`: the players with enough games, best Ballon d'Or score first.
    pub all: Vec<Candidate>,
    /// `titulosPorTime`: each title with its points.
    titles_by_club: HashMap<ClubId, Vec<(SeasonTitle, f64)>>,
    /// `campeoes`: the competitions each club won (national cups left out).
    champions: HashMap<ClubId, Vec<CompetitionId>>,
    /// `pesosPorTime`
    pub weights: HashMap<ClubId, f64>,
}

impl Season {
    pub fn new(world: &World, season: i32) -> Self {
        let mut s = Season {
            season,
            year: world.game.start_year + season - 1,
            current: season == world.season(),
            all: Vec::new(),
            titles_by_club: HashMap::new(),
            champions: HashMap::new(),
            weights: league_weights(world),
        };
        s.collect_titles(world);
        s.all = s.candidates(world);
        s
    }

    /// `Temporada.atual()`
    pub fn current(world: &World) -> Self {
        Season::new(world, world.season())
    }

    /// `bola(int)`
    pub fn ballon_dor(&self, n: usize) -> Vec<&Candidate> {
        self.top(|c| c.eligible(), None, n)
    }

    /// `chuteira(int)`: by weighted goals, then fewer games.
    pub fn golden_shoe(&self, n: usize) -> Vec<&Candidate> {
        self.top(
            |c| c.games >= MIN_GAMES_SHOE && c.goals > 0,
            Some(&|a: &Candidate, b: &Candidate| b.weighted_goals().total_cmp(&a.weighted_goals()).then(a.games.cmp(&b.games))),
            n,
        )
    }

    /// `luva(int)`
    pub fn golden_glove<'a>(&'a self, world: &World, n: usize) -> Vec<&'a Candidate> {
        self.top(|c| c.eligible() && world.player(c.player).position == 0, None, n)
    }

    /// `goldenBoy(int)`
    pub fn golden_boy(&self, n: usize) -> Vec<&Candidate> {
        self.top(|c| c.eligible() && c.age <= GOLDEN_BOY_AGE, None, n)
    }

    /// `selecao()`: a 4-3-3 by Ballon d'Or score. A missing full-back on one side is taken from
    /// the other side.
    pub fn team_of_the_year(&self, world: &World) -> Vec<(TeamSlot, &Candidate)> {
        let mut out = Vec::new();
        let mut used = HashSet::new();
        let position = |c: &Candidate| world.player(c.player).position;
        let side = |c: &Candidate| world.player(c.player).side;
        self.pick(&mut out, &mut used, TeamSlot::Goalkeeper, 1, &|c| position(c) == 0);
        self.pick(&mut out, &mut used, TeamSlot::RightBack, 1, &|c| position(c) == 1 && side(c) == 0);
        self.pick(&mut out, &mut used, TeamSlot::CentreBack, 2, &|c| position(c) == 2);
        self.pick(&mut out, &mut used, TeamSlot::LeftBack, 1, &|c| position(c) == 1 && side(c) == 1);
        let left = 1 - out.iter().filter(|(s, _)| *s == TeamSlot::LeftBack).count() as i32;
        self.pick(&mut out, &mut used, TeamSlot::LeftBack, left, &|c| position(c) == 1);
        let right = 1 - out.iter().filter(|(s, _)| *s == TeamSlot::RightBack).count() as i32;
        self.pick(&mut out, &mut used, TeamSlot::RightBack, right, &|c| position(c) == 1);
        self.pick(&mut out, &mut used, TeamSlot::Midfielder, 3, &|c| position(c) == 3);
        self.pick(&mut out, &mut used, TeamSlot::Forward, 3, &|c| position(c) == 4);
        out
    }

    /// `escalar`
    fn pick<'a>(&'a self, out: &mut Vec<(TeamSlot, &'a Candidate)>, used: &mut HashSet<usize>, slot: TeamSlot, mut count: i32, filter: &dyn Fn(&Candidate) -> bool) {
        for (i, c) in self.all.iter().enumerate() {
            if count <= 0 {
                return;
            }
            if c.eligible() && !used.contains(&i) && filter(c) {
                used.insert(i);
                out.push((slot, c));
                count -= 1;
            }
        }
    }

    /// `top`: the filtered players, sorted again if an order is given (stable), the first `n`.
    fn top(&self, filter: impl Fn(&Candidate) -> bool, order: Option<&dyn Fn(&Candidate, &Candidate) -> std::cmp::Ordering>, n: usize) -> Vec<&Candidate> {
        let mut out: Vec<&Candidate> = self.all.iter().filter(|c| filter(c)).collect();
        if let Some(order) = order {
            out.sort_by(|a, b| order(a, b));
        }
        out.truncate(n);
        out
    }

    /// `calcular()`
    fn candidates(&self, world: &World) -> Vec<Candidate> {
        let current_season = world.season();
        let mut out = Vec::new();
        for &p in &world.game.players {
            let player = world.player(p);
            let mut c = Candidate {
                player: p,
                key: player_key(world, p),
                club: if self.current { player.club } else { None },
                age: player.age - (current_season - self.season),
                games: 0,
                goals: 0,
                assists: 0,
                rating: 0.0,
                points: 0.0,
                league_weight: 1.0,
                titles: Vec::new(),
            };
            let mut rating_sum = 0.0;
            let mut ratings = 0;
            let mut games_at_club = 0;
            let mut clubs: Vec<ClubId> = c.club.into_iter().collect();
            for s in &player.season_stats {
                if s.season != self.season {
                    continue;
                }
                c.games += s.games;
                c.goals += s.goals;
                c.assists += s.assists;
                rating_sum += s.rating_sum;
                ratings += s.rated_games;
                if s.games > 0 {
                    // k.cu()
                    if let Some(t) = world.club_by_game_id(s.club_id) {
                        if !clubs.contains(&t) {
                            clubs.push(t);
                        }
                        if !self.current && !is_national_team(world, t) && s.games > games_at_club {
                            c.club = Some(t);
                            games_at_club = s.games;
                        }
                    }
                }
            }
            if c.games < MIN_GAMES_SHOE || ratings < MIN_RATINGS {
                continue;
            }
            c.rating = rating_sum / ratings as f64;
            let mut title_points = 0.0;
            for t in &clubs {
                for (title, points) in self.titles_by_club.get(t).into_iter().flatten() {
                    title_points += points;
                    c.titles.push(title.clone());
                }
            }
            if let Some(club) = c.club {
                c.league_weight = self.weights.get(&club).copied().unwrap_or(1.0);
            }
            let reputation = c.club.map(|k| world.club(k).reputation).unwrap_or(0);
            c.points = (c.rating - 5.5) * 22.0
                + player.strength as f64 * 0.3
                + c.goals as f64 * 0.45
                + c.assists as f64 * 0.3
                + reputation as f64 * 3.0
                + title_points;
            out.push(c);
        }
        out.sort_by(|a, b| b.points.total_cmp(&a.points));
        out
    }

    /// `titulos()`: each country's first division and cup, then the competitions the game state
    /// holds, each once.
    fn collect_titles(&mut self, world: &World) {
        let mut seen = HashSet::new();
        for &country in &world.game.league_countries {
            let k = world.country(country);
            if let Some(&league) = k.divisions.first() {
                seen.extend(k.divisions.iter().copied());
                if let Some(champion) = self.result(world, league).and_then(|r| champion(world, r)) {
                    let points = 3.0 + 2.5 * world.club(champion).reputation as f64;
                    self.add(champion, title(world, league, false), points);
                    self.champions.entry(champion).or_default().push(league);
                }
            }
            if let Some(cup) = k.cup {
                seen.insert(cup);
                if let Some(champion) = self.result(world, cup).and_then(|r| champion(world, r)) {
                    let points = 2.0 + world.club(champion).reputation as f64;
                    self.add(champion, title(world, cup, false), points);
                }
            }
        }
        for competition in global_competitions(world) {
            if !seen.insert(competition) {
                continue;
            }
            let Some(weight) = weight(world, competition).filter(|&w| w > 0.0) else { continue };
            let Some(record) = self.result(world, competition) else { continue };
            if let Some(champion) = champion(world, record) {
                self.add(champion, title(world, competition, false), weight);
                self.champions.entry(champion).or_default().push(competition);
            }
            if let Some(runner_up) = (record.runner_up >= 0).then(|| world.club_by_game_id(record.runner_up)).flatten()
                && weight >= 12.0
            {
                self.add(runner_up, title(world, competition, true), weight * RUNNER_UP_WEIGHT);
            }
        }
    }

    /// `adicionar`
    fn add(&mut self, club: ClubId, title: SeasonTitle, points: f64) {
        self.titles_by_club.entry(club).or_default().push((title, points));
    }

    /// `resultado(at)`: the competition's record of this season.
    fn result<'a>(&self, world: &'a World, competition: CompetitionId) -> Option<&'a TitleRecord> {
        world.competition(competition).titles.iter().find(|t| t.season == self.season)
    }

    /// `campeaoDaLiga(ah)`: won a national league (first division) this season.
    pub fn league_champion(&self, world: &World, club: ClubId) -> bool {
        self.champions.get(&club).into_iter().flatten().any(|&c| world.competition(c).class == CompetitionKind::NationalLeague)
    }

    /// `campeaoContinental(ah)`: won a competition of weight 12 or more this season.
    pub fn continental_champion(&self, world: &World, club: ClubId) -> bool {
        self.champions.get(&club).into_iter().flatten().any(|&c| weight(world, c).is_some_and(|w| w >= 12.0))
    }

    /// `titulos(ah)`
    pub fn titles(&self, club: ClubId) -> Vec<SeasonTitle> {
        self.titles_by_club.get(&club).into_iter().flatten().map(|(t, _)| t.clone()).collect()
    }

    /// `pontosDeTitulos(ah)`
    pub fn title_points(&self, club: ClubId) -> f64 {
        self.titles_by_club.get(&club).into_iter().flatten().fold(0.0, |sum, (_, p)| sum + p)
    }
}

/// `best.i.ce()`
fn champion(world: &World, record: &TitleRecord) -> Option<ClubId> {
    if record.champion >= 0 { world.club_by_game_id(record.champion) } else { None }
}

/// The label of `adicionar`.
fn title(world: &World, competition: CompetitionId, runner_up: bool) -> SeasonTitle {
    SeasonTitle { runner_up, competition: world.competition_label(competition) }
}

/// `PESOS` by the competition's class, the Super Mundial weighing like the Champions League.
fn weight(world: &World, competition: CompetitionId) -> Option<f64> {
    use CompetitionKind::*;
    let class = world.competition(competition).class;
    // SuperMundial.ehSuperMundial(at): the 32-club edition the mod builds on the friendly tournament
    if class == FriendlyTournament && crate::super_mundial::is_super_mundial(world, competition) {
        return Some(20.0);
    }
    Some(match class {
        WorldCup => 25.0,
        ChampionsLeague => 20.0,
        EuropeanChampionship => 18.0,
        Libertadores | CopaAmerica => 16.0,
        IntercontinentalCup => 12.0,
        // f.x, the Liga Europa in the mod's comments
        EuropaLeague | AfricaCupOfNations => 8.0,
        CopaSudamericana | CafChampionsLeague | AfcChampionsLeague | ConcacafChampionsCup | AsianCup | GoldCup => 6.0,
        // f.J (the Conference League in the mod's comments), f.M and f.K (the Finalíssima)
        ConferenceLeague | NationsLeagueM | Finalissima => 4.0,
        NationsLeagueN | SuperCupE | SuperCupD => 3.0,
        OfcChampionsLeague | OceaniaNationsCup => 2.0,
        NationalSuperCup => 1.0,
        _ => return None,
    })
}

/// `competicoesGlobais()`: the competitions among the fields of `best.f`, in their declaration
/// order (`getDeclaredFields`): the national super cups list (`ar`, the only list of
/// competitions), the single ones, then the regional cups array (`aZ`).
fn global_competitions(world: &World) -> Vec<CompetitionId> {
    let g = &world.game;
    let singles = [
        g.intercontinental_cup,
        g.champions_league,
        g.libertadores,
        g.caf_champions_league,
        g.afc_champions_league,
        g.concacaf_champions_cup,
        g.ofc_champions_league,
        g.friendly,
        g.friendly_tournament,
        g.super_cup_e,
        g.super_cup_d,
        g.world_cup,
        g.nations_league_m,
        g.nations_league_n,
        g.finalissima,
        g.south_american_qualifiers,
        g.euro_qualifiers,
        g.african_qualifiers,
        g.asian_qualifiers,
        g.oceania_qualifiers,
        g.concacaf_qualifiers,
        g.copa_america,
        g.european_championship,
        g.africa_cup_of_nations,
        g.asian_cup,
        g.gold_cup,
        g.oceania_nations_cup,
        g.europa_league,
        g.conference_league,
        g.copa_sudamericana,
    ];
    g.national_super_cups.iter().copied().chain(singles.into_iter().flatten()).chain(g.regional_cups.iter().flatten().copied()).collect()
}
