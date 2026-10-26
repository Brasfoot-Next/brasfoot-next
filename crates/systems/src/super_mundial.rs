// Port of bfmod.SuperMundial: the 32-club Club World Cup as FIFA played it in 2025, every four
// years from 2025. Each confederation's places go first to the winners of its top club
// competition in the four seasons before, the rest by a four-year coefficient (2 points a win, 1 a
// draw in the continental competitions, half a point more in the top tier's knockouts), at most two
// clubs per country unless more of its clubs were champions; the host's national champion makes
// 32. The tournament runs on the game's own friendly tournament (f.L): 8 groups of 4 in one round,
// the top two to a single-match knockout, on seven free days of mid-June to mid-July turned into
// friendly-tournament days; the game plays it and the mod pays FIFA's 2025 prize money and names
// the best player, top scorer and goalkeeper. The game's yearly world club cup (f.A) becomes the
// Copa Intercontinental. The newspaper's stories are not ported.

use std::collections::{BTreeMap, HashMap, HashSet};

use brasfoot_engine::hooks::NoSystems;
use brasfoot_model::countries::COUNTRIES;
use brasfoot_model::host_cities::{GAME_CLUB_WORLD_CUP_NAME, SUPER_MUNDIAL_BIDDERS, SUPER_MUNDIAL_DEFAULT_CITY, SUPER_MUNDIAL_NAME};
use brasfoot_model::ids::{ClubId, CompetitionId, MatchId, PlayerId};
use brasfoot_model::mods::{SuperMundialAward, SuperMundialEdition, SuperMundialRoute, SuperMundialSlot, SuperMundialState};
use brasfoot_model::saved::StageKind;
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;
use brasfoot_simulation::stages::StageOptions;

use crate::config::Switch;
use crate::economy::{clubs_by_id, is_human, is_national_team, year};
use crate::{Mod, sorte};

/// `TIPO_DIA`: the day type of the game's friendly tournament.
const DAY_KIND: i32 = 15;
pub const UEFA: i32 = 0;
pub const CONMEBOL: i32 = 1;
pub const CAF: i32 = 2;
pub const AFC: i32 = 3;
pub const CONCACAF: i32 = 4;
pub const OFC: i32 = 5;
/// `VAGAS`, by confederation.
const PLACES: [usize; 6] = [12, 6, 4, 4, 4, 1];
/// `DATAS`: the target dates (day, month from 1) of the seven match days, as in 2025.
const DATES: [(i32, i32); 7] = [(15, 6), (19, 6), (23, 6), (28, 6), (4, 7), (8, 7), (13, 7)];

// The prize money of 2025, one to one
const WIN: i64 = 2_000_000;
const DRAW: i64 = 1_000_000;
const ROUND_OF_16: i64 = 7_500_000;
const QUARTER_FINALS: i64 = 13_125_000;
const SEMI_FINALS: i64 = 21_000_000;
const RUNNER_UP: i64 = 30_000_000;
const CHAMPION: i64 = 40_000_000;
const UEFA_MIN: i64 = 12_810_000;
const UEFA_MAX: i64 = 38_190_000;
const CONMEBOL_FEE: i64 = 15_210_000;
const OTHERS_FEE: i64 = 9_550_000;
const OFC_FEE: i64 = 3_580_000;

/// `anoDeEdicao(int)`
pub fn edition_year(year: i32) -> bool {
    year >= 2025 && (year - 2025) % 4 == 0
}

/// `proximaEdicao(int)`
pub fn next_edition(year: i32) -> i32 {
    let mut e = year;
    while !edition_year(e) {
        e += 1;
    }
    e
}

/// `confederacao(int)`: `best.ac.gg()` of the country, -1 for none.
pub fn confederation(country: i32) -> i32 {
    COUNTRIES.iter().find(|c| c.id == country).map(|c| c.confederation).unwrap_or(-1)
}

/// The friendly tournament carries the Super Mundial's name.
fn tournament_named(world: &World, tournament: CompetitionId) -> bool {
    world.mods.super_mundial.tournament_named || world.competition(tournament).name == SUPER_MUNDIAL_NAME
}

/// `ehSuperMundial(at)`: the game's friendly tournament while it holds this year's edition.
pub fn is_super_mundial(world: &World, competition: CompetitionId) -> bool {
    world.game.friendly_tournament == Some(competition) && tournament_named(world, competition) && world.mods.super_mundial.days.contains_key(&year(world))
}

/// `at.cS()`: the competition's champion of this season.
fn champion_now(world: &World, competition: CompetitionId) -> Option<ClubId> {
    let season = world.season();
    let t = world.competition(competition).titles.iter().find(|t| t.season == season)?;
    if t.champion >= 0 { world.club_by_game_id(t.champion) } else { None }
}

/// `at.mo().cf()`: the runner-up of this season's title.
fn runner_up_now(world: &World, competition: CompetitionId) -> Option<ClubId> {
    let season = world.season();
    let t = world.competition(competition).titles.iter().find(|t| t.season == season)?;
    if t.runner_up >= 0 { world.club_by_game_id(t.runner_up) } else { None }
}

/// `continentais()`: the continental competitions whose results count, top tier first.
fn continental(world: &World) -> Vec<CompetitionId> {
    let g = &world.game;
    [
        g.champions_league,
        g.libertadores,
        g.caf_champions_league,
        g.afc_champions_league,
        g.ofc_champions_league,
        g.concacaf_champions_cup,
        g.europa_league,
        g.copa_sudamericana,
        g.conference_league,
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// `principais()`: the top club competition of each confederation.
fn top_tier(world: &World) -> Vec<CompetitionId> {
    let g = &world.game;
    [g.champions_league, g.libertadores, g.caf_champions_league, g.afc_champions_league, g.ofc_champions_league, g.concacaf_champions_cup].into_iter().flatten().collect()
}

/// `principal(int)`: the top club competition of a confederation.
fn top_competition(world: &World, conf: i32) -> Option<CompetitionId> {
    let g = &world.game;
    match conf {
        UEFA => g.champions_league,
        CONMEBOL => g.libertadores,
        CAF => g.caf_champions_league,
        AFC => g.afc_champions_league,
        CONCACAF => g.concacaf_champions_cup,
        _ => g.ofc_champions_league,
    }
}

/// `coeficienteDaTemporada(boolean)`: 2 points a win and 1 a draw in the main stage of the
/// continental competitions; the top tier's knockout matches after the groups add half a point.
/// The knockouts before a competition's group stage don't count; a competition with no group
/// stage at all counts whole once the season is over (`closed`).
fn season_coefficient(world: &World, closed: bool) -> BTreeMap<i32, f64> {
    let mut out: BTreeMap<i32, f64> = BTreeMap::new();
    let comps = continental(world);
    let top = top_tier(world);
    let mut groups_start: HashMap<CompetitionId, usize> = HashMap::new();
    for (d, day) in world.game.days.iter().enumerate() {
        for &m in &day.matches {
            let g = world.game_match(m);
            if let (Some(c), Some(s)) = (g.competition, g.stage)
                && comps.contains(&c)
                && world.stage(s).class == StageKind::League
            {
                groups_start.entry(c).or_insert(d);
            }
        }
    }
    for (d, day) in world.game.days.iter().enumerate() {
        for &m in &day.matches {
            let g = world.game_match(m);
            let (Some(c), Some(home), Some(away)) = (g.competition, g.home, g.away) else { continue };
            if !g.played || !comps.contains(&c) {
                continue;
            }
            let knockout = g.stage.is_some_and(|s| world.stage(s).class == StageKind::Knockout);
            let preliminary = knockout && groups_start.get(&c).map_or(!closed, |&start| d < start);
            if preliminary {
                continue;
            }
            let (a, b) = (g.home_goals, g.away_goals);
            *out.entry(world.club(home).id).or_insert(0.0) += if a > b { 2.0 } else if a == b { 1.0 } else { 0.0 };
            *out.entry(world.club(away).id).or_insert(0.0) += if b > a { 2.0 } else if a == b { 1.0 } else { 0.0 };
            if top.contains(&c) && knockout {
                *out.entry(world.club(home).id).or_insert(0.0) += 0.5;
                *out.entry(world.club(away).id).or_insert(0.0) += 0.5;
            }
        }
    }
    out
}

/// `coeficientePorTitulos(int, List)`: a season from before the coefficient was recorded counts
/// its titles only, from the club histories.
fn coefficient_by_titles(world: &World, year: i32) -> BTreeMap<i32, f64> {
    let mut out: BTreeMap<i32, f64> = BTreeMap::new();
    let mut top: Vec<_> = Vec::new();
    for c in top_tier(world) {
        let label = world.competition_label(c);
        if !top.contains(&label) {
            top.push(label);
        }
    }
    let mut second: Vec<_> = Vec::new();
    for c in [world.game.europa_league, world.game.copa_sudamericana].into_iter().flatten() {
        let label = world.competition_label(c);
        if !second.contains(&label) {
            second.push(label);
        }
    }
    for l in &world.mods.awards.club_seasons {
        if l.year != year || l.titles.is_empty() {
            continue;
        }
        for t in &l.titles {
            for n in &top {
                if t.competition == *n {
                    *out.entry(l.club).or_insert(0.0) += if t.runner_up { 18.0 } else { 25.0 };
                }
            }
            for n in &second {
                if !t.runner_up && t.competition == *n {
                    *out.entry(l.club).or_insert(0.0) += 12.0;
                }
            }
        }
    }
    out
}

/// `ranking(int)`: each club's coefficient over the four seasons before an edition; the season
/// in progress counts as it stands.
pub fn ranking(world: &World, edition: i32) -> HashMap<i32, f64> {
    let state = &world.mods.super_mundial;
    let mut out: HashMap<i32, f64> = HashMap::new();
    for y in edition - 4..edition {
        let by_titles;
        let m = match state.coefficients.get(&y) {
            Some(m) => m,
            None => {
                by_titles = coefficient_by_titles(world, y);
                &by_titles
            }
        };
        for (&club, &points) in m {
            *out.entry(club).or_insert(0.0) += points;
        }
    }
    let now = year(world);
    if now < edition && now >= edition - 4 && !state.coefficients.contains_key(&now) {
        for (club, points) in season_coefficient(world, false) {
            *out.entry(club).or_insert(0.0) += points;
        }
    }
    out
}

/// `Vaga.via.startsWith("País-sede")`
fn is_host_slot(v: &SuperMundialSlot) -> bool {
    matches!(v.route, SuperMundialRoute::Host { .. })
}

/// `vaga(ah, int, String, Map)`
fn slot(world: &World, club: ClubId, confederation: i32, route: SuperMundialRoute, coef: &HashMap<i32, f64>) -> SuperMundialSlot {
    let c = world.club(club);
    SuperMundialSlot { club: c.id, name: c.name.clone(), country: c.country, confederation, route, coefficient: coef.get(&c.id).copied().unwrap_or(0.0), group: -1 }
}

/// `Double.compare(b, a)`: by coefficient, highest first.
fn by_coefficient(a: f64, b: f64) -> std::cmp::Ordering {
    b.total_cmp(&a)
}

/// `calcularVagas(int)`: the clubs of an edition as things stand.
fn compute_slots(world: &World, edition: i32) -> Vec<SuperMundialSlot> {
    let coef = ranking(world, edition);
    let times = clubs_by_id(world);
    let history = &world.mods.awards.club_seasons;
    let now = year(world);
    let mut out: Vec<SuperMundialSlot> = Vec::new();
    let mut inside: HashSet<i32> = HashSet::new();
    // the champions of the four seasons, per confederation, oldest first
    let mut champions: HashMap<i32, Vec<(ClubId, i32, CompetitionId)>> = HashMap::new();
    for y in edition - 4..edition {
        for conf in 0..6 {
            let Some(comp) = top_competition(world, conf) else { continue };
            let t = if y == now {
                champion_now(world, comp)
            } else {
                let label = world.competition_label(comp);
                history
                    .iter()
                    .find(|l| l.year == y && l.titles.iter().any(|t| !t.runner_up && t.competition == label))
                    .and_then(|l| times.get(&l.club).copied())
            };
            if let Some(t) = t
                && confederation(world.club(t).country) == conf
            {
                champions.entry(conf).or_default().push((t, y, comp));
            }
        }
    }
    let national_team = |t: ClubId| is_national_team(world, t);
    for conf in 0..6 {
        let mut of_conf: Vec<SuperMundialSlot> = Vec::new();
        let mut by_country: HashMap<i32, usize> = HashMap::new();
        let mut champions_by_country: HashMap<i32, usize> = HashMap::new();
        let mut list = champions.get(&conf).cloned().unwrap_or_default();
        if conf == OFC {
            // one place: the best-ranked of the champions
            list.sort_by(|a, b| by_coefficient(coef.get(&world.club(a.0).id).copied().unwrap_or(0.0), coef.get(&world.club(b.0).id).copied().unwrap_or(0.0)));
        }
        for (t, y, comp) in list {
            let id = world.club(t).id;
            if of_conf.len() >= PLACES[conf as usize] || inside.contains(&id) {
                continue;
            }
            of_conf.push(slot(world, t, conf, SuperMundialRoute::Champion { competition: world.competition_label(comp), year: y }, &coef));
            inside.insert(id);
            *by_country.entry(world.club(t).country).or_insert(0) += 1;
            *champions_by_country.entry(world.club(t).country).or_insert(0) += 1;
        }
        let mut candidates: Vec<ClubId> = world
            .game
            .clubs
            .iter()
            .copied()
            .filter(|&t| {
                let c = world.club(t);
                !national_team(t) && confederation(c.country) == conf && !inside.contains(&c.id) && c.squad.len() >= 11
            })
            .collect();
        candidates.sort_by(|&a, &b| {
            let (x, y) = (world.club(a), world.club(b));
            by_coefficient(coef.get(&x.id).copied().unwrap_or(0.0), coef.get(&y.id).copied().unwrap_or(0.0)).then(y.reputation.cmp(&x.reputation))
        });
        for t in candidates {
            if of_conf.len() >= PLACES[conf as usize] {
                break;
            }
            let country = world.club(t).country;
            let cap = 2.max(champions_by_country.get(&country).copied().unwrap_or(0));
            if by_country.get(&country).copied().unwrap_or(0) >= cap {
                continue;
            }
            of_conf.push(slot(world, t, conf, SuperMundialRoute::Ranking, &coef));
            inside.insert(world.club(t).id);
            *by_country.entry(country).or_insert(0) += 1;
        }
        out.extend(of_conf);
    }
    // the host's national champion, or the next of its table if already in
    let host = world.mods.super_mundial.hosts.get(&edition).map(|h| h.0);
    if let Some(host) = host {
        let last = (edition - 1).min(now - 1);
        let mut of_country: Vec<_> = history.iter().filter(|l| l.year == last && l.country == host && l.division == 1 && l.position > 0).collect();
        of_country.sort_by_key(|l| l.position);
        let mut chosen = of_country.iter().find_map(|l| times.get(&l.club).copied().filter(|&t| !inside.contains(&world.club(t).id)));
        if chosen.is_none() {
            let mut now_clubs: Vec<ClubId> = world
                .game
                .clubs
                .iter()
                .copied()
                .filter(|&t| !national_team(t) && world.club(t).country == host && !inside.contains(&world.club(t).id))
                .collect();
            now_clubs.sort_by(|&a, &b| world.club(b).reputation.cmp(&world.club(a).reputation));
            chosen = now_clubs.first().copied();
        }
        if let Some(t) = chosen {
            out.push(slot(world, t, confederation(host), SuperMundialRoute::Host { country: host }, &coef));
            inside.insert(world.club(t).id);
        }
    }
    // a confederation without enough clubs hands its places to the best of the rest; the host's
    // place stays open until the host is drawn
    let total = if host.is_some() { 32 } else { 31 };
    if out.len() < total {
        let mut rest: Vec<ClubId> = world
            .game
            .clubs
            .iter()
            .copied()
            .filter(|&t| {
                let c = world.club(t);
                !national_team(t) && !inside.contains(&c.id) && confederation(c.country) >= 0 && c.squad.len() >= 11
            })
            .collect();
        rest.sort_by(|&a, &b| {
            let (x, y) = (world.club(a), world.club(b));
            by_coefficient(coef.get(&x.id).copied().unwrap_or(0.0), coef.get(&y.id).copied().unwrap_or(0.0)).then(y.reputation.cmp(&x.reputation))
        });
        for t in rest {
            if out.len() >= total {
                break;
            }
            out.push(slot(world, t, confederation(world.club(t).country), SuperMundialRoute::Reallocated, &coef));
            inside.insert(world.club(t).id);
        }
    }
    out
}

/// `vagas(int)`: an edition's clubs, fixed or as things stand.
pub fn slots(world: &World, edition: i32) -> Vec<SuperMundialSlot> {
    match world.mods.super_mundial.qualified.get(&edition) {
        Some(fixed) => fixed.clone(),
        None => compute_slots(world, edition),
    }
}

/// `cabe(List, Vaga)`: no two clubs of one confederation in a group, two of Europe at most.
fn fits(group: &[usize], v: usize, slots: &[SuperMundialSlot]) -> bool {
    let same = group.iter().filter(|&&x| slots[x].confederation == slots[v].confederation).count();
    if slots[v].confederation == UEFA { same < 2 } else { same == 0 }
}

/// `cota(Vaga, List)`: the participation fee of 2025, Europe by ranking, a flat amount elsewhere.
fn participation_fee(slots: &[SuperMundialSlot], v: usize, europe: &[usize]) -> i64 {
    match slots[v].confederation {
        UEFA => match europe.iter().position(|&x| x == v) {
            Some(i) if europe.len() >= 2 => UEFA_MAX - (UEFA_MAX - UEFA_MIN) * i as i64 / (europe.len() as i64 - 1),
            _ => UEFA_MIN,
        },
        CONMEBOL => CONMEBOL_FEE,
        OFC => OFC_FEE,
        _ => OTHERS_FEE,
    }
}

/// The matches of the tournament on a calendar day (`jogosDoDia(int, at)`).
fn matches_on(world: &World, day: usize, tournament: CompetitionId) -> Vec<MatchId> {
    world.game.days[day]
        .matches
        .iter()
        .copied()
        .filter(|&m| {
            let g = world.game_match(m);
            g.competition == Some(tournament) && g.home.is_some() && g.away.is_some()
        })
        .collect()
}

impl Mod<'_> {
    fn super_mundial(&mut self) -> &mut SuperMundialState {
        &mut self.world.mods.super_mundial
    }

    /// `SuperMundial.tick()`: on every main screen.
    pub(crate) fn super_mundial_tick(&mut self) {
        if self.world.game.days.is_empty() {
            return;
        }
        let Some(tournament) = self.world.game.friendly_tournament else { return };
        self.rename_intercontinental();
        let year = year(self.world);
        if !edition_year(year) && !self.world.mods.super_mundial.days.contains_key(&year) {
            // the game clears the friendly tournament at the season start; it gets its own name back
            if tournament_named(self.world, tournament) && self.world.competition(tournament).titles.is_empty() {
                self.super_mundial().tournament_named = false;
                if self.world.competition(tournament).name == SUPER_MUNDIAL_NAME {
                    self.world.competition_mut(tournament).name.clear();
                }
            }
            return;
        }
        // testeAgora: the mod's test command is not ported
        if !self.world.mods.super_mundial.days.contains_key(&year) {
            let month = self.runtime.clock.fields(self.world.today_millis()).month;
            if month >= 4 && self.tournament_free(tournament) {
                self.create_edition(year, tournament);
            }
        } else {
            self.follow_edition(year, tournament);
        }
    }

    /// `fimDeTemporada()`: the season's coefficient, the hosts drawn two seasons ahead and the
    /// next edition's clubs fixed.
    pub(crate) fn super_mundial_season_end(&mut self) {
        if self.world.game.days.is_empty() {
            return;
        }
        let year = year(self.world);
        let coefficient = season_coefficient(self.world, true);
        self.super_mundial().coefficients.insert(year, coefficient);
        if edition_year(year + 2) {
            self.draw_host(year + 2);
        }
        if edition_year(year + 1) {
            self.draw_host(year + 1);
            self.fix_qualified(year + 1);
        }
        // anunciarCampeoesClassificados: the newspaper's stories only
    }

    /// `renomearIntercontinental()`: `c.a.SR.aR()` makes the game's yearly world club cup when
    /// international clubs are on.
    fn rename_intercontinental(&mut self) {
        let cup = Sim { world: self.world, runtime: self.runtime, hooks: &mut NoSystems::default(), human: None }.intercontinental_cup();
        let Some(cup) = cup else { return };
        // a cup this app made has no name: the game's, until renamed
        if self.world.competition(cup).name == GAME_CLUB_WORLD_CUP_NAME {
            self.world.competition_mut(cup).name.clear();
        } else if !self.world.competition(cup).name.is_empty() {
            return;
        }
        // Jornal.noticia: the news of the new name is not ported
        self.super_mundial().renamed = true;
    }

    /// `torneioLivre(f.L)`: nothing in the friendly tournament, or a finished pre-season one.
    fn tournament_free(&self, tournament: CompetitionId) -> bool {
        let c = self.world.competition(tournament);
        c.main_stage.is_none() || !c.titles.is_empty() || tournament_named(self.world, tournament)
    }

    /// `sortearSede(int)`: among the real bidders with clubs in the save, never from the last
    /// host's confederation (2025 was in the USA).
    fn draw_host(&mut self, edition: i32) {
        if self.world.mods.super_mundial.hosts.contains_key(&edition) {
            return;
        }
        let last = self.world.mods.super_mundial.hosts.get(&(edition - 4)).map(|h| h.0);
        let last_conf = last.map(confederation).unwrap_or(CONCACAF);
        let with_clubs: HashSet<i32> = self.world.game.clubs.iter().map(|&c| self.world.club(c).country).collect();
        let candidates: Vec<(i32, &str)> = SUPER_MUNDIAL_BIDDERS.iter().copied().filter(|&(country, _)| confederation(country) != last_conf && with_clubs.contains(&country)).collect();
        if candidates.is_empty() {
            return;
        }
        let i = sorte!(self, "SuperMundial").next_int_bounded(candidates.len() as i32) as usize;
        let (country, city) = candidates[i];
        self.super_mundial().hosts.insert(edition, (country, city.to_owned()));
        // Jornal.noticia: the host's news is not ported
    }

    /// `definirClassificados(int)`
    fn fix_qualified(&mut self, edition: i32) {
        if self.world.mods.super_mundial.qualified.contains_key(&edition) {
            return;
        }
        let list = compute_slots(self.world, edition);
        if list.len() < 32 {
            return;
        }
        self.super_mundial().qualified.insert(edition, list);
        // Jornal.noticia: the news of the 32 is not ported
    }

    /// `sortear(List)`: four pots, the host with the four best of Europe and the three best of
    /// South America first, the other eight Europeans second, then the rest by coefficient and
    /// reputation. No group gets two clubs of one confederation but Europe's two; the host opens
    /// group A.
    fn draw_groups(&mut self, mut slots: Vec<SuperMundialSlot>) -> Vec<SuperMundialSlot> {
        let (mut host, mut europe, mut america, mut rest) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (i, v) in slots.iter().enumerate() {
            if is_host_slot(v) {
                host.push(i);
            } else if v.confederation == UEFA {
                europe.push(i);
            } else if v.confederation == CONMEBOL {
                america.push(i);
            } else {
                rest.push(i);
            }
        }
        let _ = rest;
        europe.sort_by(|&a, &b| by_coefficient(slots[a].coefficient, slots[b].coefficient));
        america.sort_by(|&a, &b| by_coefficient(slots[a].coefficient, slots[b].coefficient));
        let mut top = host.clone();
        top.extend_from_slice(&europe[..4.min(europe.len())]);
        for &v in &america {
            if top.len() >= 8 {
                break;
            }
            top.push(v);
        }
        let mut pot2 = Vec::new();
        for &v in &europe {
            if !top.contains(&v) && pot2.len() < 8 {
                pot2.push(v);
            }
        }
        let mut left: Vec<usize> = (0..slots.len()).filter(|v| !top.contains(v) && !pot2.contains(v)).collect();
        let times = clubs_by_id(self.world);
        let reputation = |id: i32| times.get(&id).map(|&t| self.world.club(t).reputation).unwrap_or(0);
        left.sort_by(|&a, &b| by_coefficient(slots[a].coefficient, slots[b].coefficient).then(reputation(slots[b].club).cmp(&reputation(slots[a].club))));
        while top.len() < 8 && !left.is_empty() {
            top.push(left.remove(0));
        }
        while pot2.len() < 8 && !left.is_empty() {
            pot2.push(left.remove(0));
        }
        let cut = 8.min(left.len());
        let pots = [top, pot2, left[..cut].to_vec(), left[cut..].to_vec()];
        let host_in_a = !host.is_empty();
        for _ in 0..2000 {
            let mut groups: Vec<Vec<usize>> = vec![Vec::new(); 8];
            if self.draw_fits(&pots, &mut groups, host_in_a, &slots) {
                let mut out = Vec::new();
                for (g, group) in groups.iter().enumerate() {
                    for &v in group {
                        slots[v].group = g as i32;
                        out.push(slots[v].clone());
                    }
                }
                return out;
            }
        }
        // constraints that can't be met: an unconstrained draw
        let mut out = Vec::new();
        for g in 0..8 {
            for pot in &pots {
                if g < pot.len() {
                    slots[pot[g]].group = g as i32;
                    out.push(slots[pot[g]].clone());
                }
            }
        }
        out
    }

    /// `sorteioCabe(List, List, boolean)`
    fn draw_fits(&mut self, pots: &[Vec<usize>; 4], groups: &mut [Vec<usize>], host_in_a: bool, slots: &[SuperMundialSlot]) -> bool {
        for (p, original) in pots.iter().enumerate() {
            let mut pot = original.clone();
            sorte!(self, "SuperMundial").shuffle(&mut pot);
            let a1 = original.first().copied();
            if p == 0 && host_in_a {
                // the host is always A1
                let a1 = a1.expect("the host's pot");
                if let Some(i) = pot.iter().position(|&x| x == a1) {
                    pot.remove(i);
                }
                pot.insert(0, a1);
            }
            for v in pot {
                let mut free: Vec<usize> = (0..8).filter(|&g| groups[g].len() == p && fits(&groups[g], v, slots)).collect();
                if free.is_empty() {
                    return false;
                }
                let is_host = p == 0 && host_in_a && Some(v) == a1;
                let mut g = if is_host { 0 } else { free[sorte!(self, "SuperMundial").next_int_bounded(free.len() as i32) as usize] };
                if p == 0 && host_in_a && !is_host && g == 0 {
                    free.retain(|&x| x != 0);
                    if free.is_empty() {
                        return false;
                    }
                    g = free[sorte!(self, "SuperMundial").next_int_bounded(free.len() as i32) as usize];
                }
                groups[g].push(v);
            }
        }
        true
    }

    /// `criar(int)`: the edition built on the friendly tournament.
    fn create_edition(&mut self, year: i32, tournament: CompetitionId) {
        self.draw_host(year);
        self.fix_qualified(year);
        let Some(slots) = self.world.mods.super_mundial.qualified.get(&year).cloned() else { return };
        if slots.len() != 32 {
            return;
        }
        let dates = self.pick_days();
        if dates.len() < DATES.len() {
            // "SuperMundial: sem datas livres": the original only prints it
            return;
        }
        let drawn = self.draw_groups(slots);
        let times = clubs_by_id(self.world);
        let mut order = Vec::new();
        for v in &drawn {
            let Some(&t) = times.get(&v.club) else { return };
            order.push(t);
        }
        for &d in &dates {
            self.world.game.days[d as usize].kind = DAY_KIND;
        }
        // a pre-season friendly of this year leaves its title behind; the champion is read there
        let city = self.world.mods.super_mundial.hosts.get(&year).map(|h| h.1.clone()).unwrap_or_else(|| SUPER_MUNDIAL_DEFAULT_CITY.to_owned());
        let c = self.world.competition_mut(tournament);
        c.titles.clear();
        c.text_agn = city;
        self.super_mundial().tournament_named = true;
        let options = StageOptions { teams: 32, groups: 8, knockout_teams: 2, two_rounds: false, two_legged_knockout: vec![false; 7], ..Default::default() };
        let stage = Sim { world: self.world, runtime: self.runtime, hooks: &mut NoSystems::default(), human: None }.new_league_stage(&options, &order, 0, DAY_KIND, tournament, true);
        self.world.competition_mut(tournament).main_stage = Some(stage);
        let state = self.super_mundial();
        state.qualified.insert(year, drawn.clone());
        state.days.insert(year, dates);
        state.paid.insert(year, Default::default());
        self.pay_participation(&drawn);
        // anunciarSorteio: the draw's news is not ported
    }

    /// `escolherDias(int)`: seven free days near the 2025 dates, three days apart at least in
    /// the groups and two in the knockout.
    fn pick_days(&self) -> Vec<i32> {
        let days = &self.world.game.days;
        let mut out = Vec::new();
        let least = self.world.game.day + 7;
        let mut previous = -100;
        for (i, &(day, month)) in DATES.iter().enumerate() {
            let target = self.day_index(day, month - 1);
            let gap = if i < 3 { 3 } else { 2 };
            let start = least.max((target - 2).max(previous + gap));
            let found = (start.max(0) as usize..days.len()).find(|&d| days[d].kind == 0 && !days[d].done && days[d].matches.is_empty());
            let Some(found) = found else { return out };
            out.push(found as i32);
            previous = found as i32;
        }
        out
    }

    /// `indiceDe(int, int)`: the first calendar day on or after a date (month from 0).
    fn day_index(&self, day: i32, month: i32) -> i32 {
        let days = &self.world.game.days;
        for (d, x) in days.iter().enumerate() {
            let f = self.runtime.clock.fields(x.date);
            if f.month > month || f.month == month && f.day >= day {
                return d as i32;
            }
        }
        days.len() as i32
    }

    /// `pagar(ah, long, ah)`: the human club's cash (category 3), an AI club's fortune.
    fn pay_prize(&mut self, club: Option<ClubId>, amount: i64) {
        let Some(club) = club else { return };
        if amount <= 0 {
            return;
        }
        if is_human(self.world, club) {
            self.world.club_income(club, amount.min(i32::MAX as i64) as i32, 3);
        } else {
            self.spend(club, -amount);
        }
    }

    /// `pagarParticipacao(List)`
    fn pay_participation(&mut self, slots: &[SuperMundialSlot]) {
        let mut europe: Vec<usize> = (0..slots.len()).filter(|&i| slots[i].confederation == UEFA && !is_host_slot(&slots[i])).collect();
        europe.sort_by(|&a, &b| by_coefficient(slots[a].coefficient, slots[b].coefficient));
        let times = clubs_by_id(self.world);
        for i in 0..slots.len() {
            let fee = participation_fee(slots, i, &europe);
            self.pay_prize(times.get(&slots[i].club).copied(), fee);
        }
    }

    /// `acompanhar(int)`: each finished match day paid once, and the end.
    fn follow_edition(&mut self, year: i32, tournament: CompetitionId) {
        if !tournament_named(self.world, tournament) {
            return;
        }
        let dates = self.world.mods.super_mundial.days.get(&year).cloned().unwrap_or_default();
        self.super_mundial().paid.entry(year).or_default();
        for d in dates {
            if self.world.mods.super_mundial.paid[&year].contains(&d) || d > self.world.game.day {
                continue;
            }
            let matches = matches_on(self.world, d as usize, tournament);
            if matches.is_empty() || !matches.iter().all(|&m| self.world.game_match(m).played) {
                continue;
            }
            self.super_mundial().paid.get_mut(&year).expect("the edition's days").insert(d);
            self.pay_day(&matches);
        }
        if !self.world.mods.super_mundial.editions.contains_key(&year) && champion_now(self.world, tournament).is_some() {
            self.close_edition(year, tournament);
        }
    }

    /// `pagarDia(int, List)`: a group match pays a win or a draw; a knockout round pays everyone
    /// who reached it.
    fn pay_day(&mut self, matches: &[MatchId]) {
        let groups = self.world.game_match(matches[0]).stage.is_none_or(|s| self.world.stage(s).class != StageKind::Knockout);
        for &m in matches {
            let g = self.world.game_match(m);
            let (home, away, a, b) = (g.home, g.away, g.home_goals, g.away_goals);
            if groups {
                self.pay_prize(home, if a > b { WIN } else if a == b { DRAW } else { 0 });
                self.pay_prize(away, if b > a { WIN } else if a == b { DRAW } else { 0 });
            } else {
                let amount = match matches.len() {
                    8 => ROUND_OF_16,
                    4 => QUARTER_FINALS,
                    2 => SEMI_FINALS,
                    _ => 0,
                };
                self.pay_prize(home, amount);
                self.pay_prize(away, amount);
            }
        }
        // Jornal.noticia: the human club's knockout money is news only
    }

    /// `encerrar(int, f.L)`: the champion's and the runner-up's money and the edition's names.
    fn close_edition(&mut self, year: i32, tournament: CompetitionId) {
        let champion = champion_now(self.world, tournament).expect("the edition's champion");
        let runner_up = runner_up_now(self.world, tournament);
        self.pay_prize(Some(champion), CHAMPION);
        self.pay_prize(runner_up, RUNNER_UP);
        let dates = self.world.mods.super_mundial.days.get(&year).cloned().unwrap_or_default();
        let mut matches = Vec::new();
        let mut last_four: Vec<ClubId> = Vec::new();
        for (i, &d) in dates.iter().enumerate() {
            let of_day = matches_on(self.world, d as usize, tournament);
            if i + 2 >= dates.len() {
                for &m in &of_day {
                    let g = self.world.game_match(m);
                    for c in [g.home, g.away].into_iter().flatten() {
                        if !last_four.contains(&c) {
                            last_four.push(c);
                        }
                    }
                }
            }
            matches.extend(of_day);
        }
        let numbers = stats(self.world, &matches);
        let best = best_player(self.world, &numbers, champion, runner_up);
        let scorer = top_scorer(&numbers);
        let keeper = goalkeeper(self.world, &matches, &last_four, champion);
        let award = |world: &World, p: PlayerId| {
            let n = numbers.iter().find(|(x, _)| *x == p).map(|(_, n)| *n).unwrap_or_default();
            SuperMundialAward { name: world.player(p).name.clone(), club: world.player(p).club.map(|c| world.club(c).name.clone()), goals: n[0], assists: n[1] }
        };
        let (host, city) = self.world.mods.super_mundial.hosts.get(&year).cloned().unwrap_or((-1, String::new()));
        let edition = SuperMundialEdition {
            year,
            host,
            city,
            champion: self.world.club(champion).name.clone(),
            champion_id: self.world.club(champion).id,
            runner_up: runner_up.map(|c| self.world.club(c).name.clone()).unwrap_or_default(),
            runner_up_id: runner_up.map(|c| self.world.club(c).id).unwrap_or(-1),
            best_player: best.map(|p| award(self.world, p)),
            top_scorer: scorer.map(|p| award(self.world, p)),
            goalkeeper: keeper.map(|p| award(self.world, p)),
        };
        self.super_mundial().editions.insert(year, edition);
        if self.sys.config.on(Switch::AwardEffects) {
            for p in [best, scorer, keeper].into_iter().flatten() {
                self.world.player_mut(p).set_star(true);
            }
        }
        // Jornal.noticia: the champion's news is not ported
    }
}

/// `estatisticas(List)`: each player's goals, assists, matches and wins in the tournament, in
/// the order they first appear (`LinkedHashMap`).
fn stats(world: &World, matches: &[MatchId]) -> Vec<(PlayerId, [i32; 4])> {
    let mut out: Vec<(PlayerId, [i32; 4])> = Vec::new();
    let entry = |out: &mut Vec<(PlayerId, [i32; 4])>, p: PlayerId| -> usize {
        match out.iter().position(|(x, _)| *x == p) {
            Some(i) => i,
            None => {
                out.push((p, [0; 4]));
                out.len() - 1
            }
        }
    };
    for &m in matches {
        let g = world.game_match(m);
        if !g.played {
            continue;
        }
        let (a, b) = (g.home_goals, g.away_goals);
        for (lineup, won) in [(&g.home_starters, a > b), (&g.away_starters, b > a)] {
            for &p in lineup {
                let i = entry(&mut out, p);
                out[i].1[2] += 1;
                if won {
                    out[i].1[3] += 1;
                }
            }
        }
        for e in &g.events {
            let Some(p) = e.player else { continue };
            if e.kind == 1 {
                let i = entry(&mut out, p);
                out[i].1[0] += 1;
            } else if e.kind == 8 {
                let i = entry(&mut out, p);
                out[i].1[1] += 1;
            }
        }
    }
    out
}

/// `melhor(Map, ah, ah)`: goals, assists and wins, how far his club went, and his season's
/// average rating.
fn best_player(world: &World, numbers: &[(PlayerId, [i32; 4])], champion: ClubId, runner_up: Option<ClubId>) -> Option<PlayerId> {
    let mut best = None;
    let mut top = -1.0;
    for &(p, n) in numbers {
        let average = crate::loans::season_numbers(world, p).2;
        let average = if average > 0.0 { average } else { 6.0 };
        let mut s = 4.0 * n[0] as f64 + 3.0 * n[1] as f64 + 1.5 * n[3] as f64 + 0.5 * n[2] as f64 + 2.0 * (average - 6.0);
        let club = world.player(p).club;
        if club == Some(champion) {
            s += 4.0;
        } else if club.is_some() && club == runner_up {
            s += 2.0;
        }
        if s > top {
            top = s;
            best = Some(p);
        }
    }
    best
}

/// `artilheiro(Map)`: most goals, then assists, then fewer matches.
fn top_scorer(numbers: &[(PlayerId, [i32; 4])]) -> Option<PlayerId> {
    let mut best: Option<(PlayerId, [i32; 4])> = None;
    for &(p, n) in numbers {
        if n[0] == 0 {
            continue;
        }
        match best {
            None => best = Some((p, n)),
            Some((_, m)) => {
                if n[0] > m[0] || n[0] == m[0] && (n[1] > m[1] || n[1] == m[1] && n[2] < m[2]) {
                    best = Some((p, n));
                }
            }
        }
    }
    best.map(|(p, _)| p)
}

/// `goleiro(List, Set, ah)`: the goalkeeper of the semi-finalist that conceded least per match,
/// the champion a little ahead. The original walks a `HashSet` of clubs and a `HashMap` of
/// keepers in identity-hash order, which only matters on a tie; here the first in the matches'
/// order wins it.
fn goalkeeper(world: &World, matches: &[MatchId], last_four: &[ClubId], champion: ClubId) -> Option<PlayerId> {
    let mut defence: HashMap<ClubId, (i32, i32)> = HashMap::new();
    let mut keepers: HashMap<ClubId, Vec<(PlayerId, i32)>> = HashMap::new();
    for &m in matches {
        let g = world.game_match(m);
        if !g.played {
            continue;
        }
        for side in 0..2 {
            let (club, conceded, lineup) = if side == 0 { (g.home, g.away_goals, &g.home_starters) } else { (g.away, g.home_goals, &g.away_starters) };
            let Some(club) = club else { continue };
            let d = defence.entry(club).or_default();
            d.0 += conceded;
            d.1 += 1;
            for &p in lineup {
                if world.player(p).position == 0 {
                    let list = keepers.entry(club).or_default();
                    match list.iter_mut().find(|(x, _)| *x == p) {
                        Some(e) => e.1 += 1,
                        None => list.push((p, 1)),
                    }
                }
            }
        }
    }
    let mut chosen = None;
    let mut best = f64::MAX;
    for &t in last_four {
        let d = defence.get(&t).copied().unwrap_or_default();
        let mut average = if d.1 > 0 { d.0 as f64 / d.1 as f64 } else { 99.0 };
        if t == champion {
            average -= 0.05;
        }
        if average < best && keepers.contains_key(&t) {
            best = average;
            chosen = Some(t);
        }
    }
    let mut keeper = None;
    let mut most = -1;
    for &(p, n) in keepers.get(&chosen?)? {
        if n > most {
            most = n;
            keeper = Some(p);
        }
    }
    keeper
}
