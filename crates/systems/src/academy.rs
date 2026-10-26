// Port of bfmod.Base: the youth academy rework. The game rolls a youth's potential from
// club-dependent tables and sets his strength from the club's reputation at 20; here every kid
// gets a hidden ceiling from the same world distribution, his strength follows a development
// curve (by the calendar in the academy, by minutes once professional), AI clubs promote him when
// he is ready, and scouts show a range that narrows with age.

use std::collections::{HashMap, HashSet};

use brasfoot_engine::tactics::best_value;
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::mods::AcademyKid;
use brasfoot_model::rng::java_round;
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;

use crate::awards::season::player_key;
use crate::config::Switch;
use crate::economy::{humans, is_national_team};
use crate::market::{NORMAL_RATIO, SQUAD_TARGET};
use crate::{Mod, sorte};

/// `TETO_MEDIO`, `TETO_DESVIO`
const MEAN_CEILING: f64 = 55.0;
const CEILING_DEVIATION: f64 = 11.0;
/// `IDADE_FIM`: the age the development ends.
pub const END_AGE: f64 = 25.0;
/// `CURVA`: the share of the ceiling reached at each age, from 16 to 25.
const CURVE: [f64; 10] = [0.45, 0.52, 0.60, 0.68, 0.75, 0.82, 0.88, 0.94, 0.98, 1.0];

/// The kids' places by key, rebuilt when the list may have changed under it (a run of the tick
/// inside another, or a kid added).
#[derive(Default)]
struct KidIndex {
    generation: u64,
    len: usize,
    places: HashMap<String, usize>,
}

impl KidIndex {
    fn find(&mut self, m: &mut Mod, key: &str) -> Option<usize> {
        let kids = m.world.mods.academy.get_or_insert_with(Vec::new);
        if self.generation != m.sys.academy_generation || self.len != kids.len() || self.places.is_empty() {
            self.places = kids.iter().enumerate().map(|(i, k)| (k.key.clone(), i)).collect();
            self.generation = m.sys.academy_generation;
            self.len = kids.len();
        }
        self.places.get(key).copied()
    }

    /// A kid just added at `place`: the index keeps up instead of being made again, which
    /// with thousands of new kids took seconds.
    fn added(&mut self, key: &str, place: usize, len: usize) {
        self.places.insert(key.to_owned(), place);
        self.len = len;
    }
}

/// The juniors screen's squad limit.
const MAX_SQUAD: usize = 35;

/// Why the juniors screen refused a promotion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromoteError {
    /// "Não há vagas no elenco profissional (limite de 35 jogadores)."
    SquadFull,
}

/// `Base.fracaoTemporada()`: a Brasfoot season is the calendar year.
pub fn season_fraction(m: &Mod) -> f64 {
    (m.runtime.clock.day_of_year(m.world.today_millis()) - 1) as f64 / 365.0
}

/// `Base.forcaAlvo(Garoto)`
pub fn target_strength(kid: &AcademyKid) -> i32 {
    java_round(kid.ceiling as f64 * curve(kid.development_age)) as i32
}

/// `Base.curva(double)`
fn curve(age: f64) -> f64 {
    if age <= 16.0 {
        return CURVE[0] * (age / 16.0).max(0.8);
    }
    if age >= END_AGE {
        return 1.0;
    }
    let i = (age.floor() - 16.0) as usize;
    let f = age - age.floor();
    CURVE[i] + (CURVE[i + 1] - CURVE[i]) * f
}

/// `Base.avancar(Garoto, double, double)`
fn advance(kid: &mut AcademyKid, now: f64, factor: f64) {
    let dt = now - kid.last_time;
    if dt > 0.0 && dt < 1.5 {
        kid.development_age += dt * factor;
    }
    kid.last_time = now;
}

/// `Base.potencialDoJogo(int)`: the game's 0-10 potential, kept roughly in line.
fn game_potential(ceiling: i32) -> i32 {
    java_round((ceiling - 30) as f64 / 6.5).clamp(1, 10) as i32
}

/// `Base.melhor(ah, int)`: the strength of the n-th best player of a squad (0 when smaller).
fn nth_best(world: &World, club: ClubId, n: usize) -> i32 {
    let mut strengths: Vec<i32> = world.club(club).squad.iter().map(|&p| world.player(p).strength).collect();
    strengths.sort_by(|a, b| b.cmp(a));
    if strengths.len() >= n { strengths[n - 1] } else { 0 }
}

/// `F.gu()`: the games of the current season.
fn season_games(world: &World, p: PlayerId) -> i32 {
    let season = world.season();
    world.player(p).season_stats.iter().filter(|s| s.season == season).map(|s| s.games).sum()
}

/// `Base.relatorio(F)`'s verdict on a kid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YouthLabel {
    /// "Craque em potencial"
    Star,
    /// "Promessa"
    Promise,
    /// "Jogador de elenco"
    Squad,
    /// "Dispensável"
    Expendable,
}

/// `Base.relatorio(F)`: the scouts' range of the ceiling, their verdict and the strength the kid
/// is at now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YouthReport {
    pub low: i32,
    pub high: i32,
    pub label: YouthLabel,
    pub strength: i32,
}

/// `Base.largura(F)`: the range narrows with age and youth league games.
fn report_width(world: &World, p: PlayerId, key: &str) -> i32 {
    let age = world.player(p).age;
    let base = match age {
        ..=16 => 10,
        17 => 7,
        18 => 5,
        19 => 3,
        _ => 2,
    };
    (base - crate::youth_league::games_played(world, key) / 4).max(2)
}

/// `Base.centro(Garoto, F)`
fn report_centre(kid: &AcademyKid, width: i32) -> i32 {
    java_round(kid.ceiling as f64 + kid.scout_error * width as f64 * 0.5) as i32
}

/// `Base.relatorio(F)`: none with the academy off or for a kid it does not follow. A kid a scout
/// watched, or one of the human's satellite, shows his ceiling within two.
pub fn youth_report(world: &World, config: &crate::config::ModConfig, p: PlayerId) -> Option<YouthReport> {
    if !config.on(Switch::Youth) {
        return None;
    }
    let key = player_key(world, p);
    let kid = world.mods.academy.as_ref()?.iter().find(|k| k.key == key)?;
    let width = report_width(world, p, &key);
    let centre = report_centre(kid, width);
    let (low, high) = if crate::scouts::watched(world, p) || crate::satellites::seen_by_satellite(world, config, p) {
        ((kid.ceiling - 2).max(30), (kid.ceiling + 2).min(99))
    } else {
        ((centre - width).max(30), (centre + width).min(99))
    };
    let label = match centre {
        82.. => YouthLabel::Star,
        72.. => YouthLabel::Promise,
        58.. => YouthLabel::Squad,
        _ => YouthLabel::Expendable,
    };
    Some(YouthReport { low, high, label, strength: target_strength(kid) })
}

/// `Base.teto(F)`: the hidden ceiling of a kid the academy follows.
pub fn ceiling(world: &World, p: PlayerId) -> Option<i32> {
    let key = player_key(world, p);
    world.mods.academy.as_ref()?.iter().find(|k| k.key == key).map(|k| k.ceiling)
}

impl Mod<'_> {
    /// `Base.tick()`. It works on the kids of the world as the original does on its static map:
    /// the first call of `Patrimonio` (a kid's ceiling or factor) can start it, and its youth
    /// trials run this tick again for the player's club, on the same kids.
    pub(crate) fn academy_tick(&mut self) {
        if !self.sys.config.on(Switch::Youth) {
            return;
        }
        // carregar(): the first run on a save with no kids migrates the academies
        let first = !self.sys.academy_loaded && self.world.mods.academy.as_ref().is_none_or(Vec::is_empty);
        if !self.sys.academy_loaded {
            self.sys.academy_loaded = true;
            self.sys.academy_last_run = None;
        }
        self.world.mods.academy.get_or_insert_with(Vec::new);
        if !first && Some(self.academy_run()) == self.sys.academy_last_run {
            self.youth_league_tick();
            return;
        }
        self.sys.academy_generation += 1;
        let fraction = season_fraction(self);
        let now = self.world.season() as f64 + fraction;
        let preseason = fraction < 0.2;
        let mut index = KidIndex::default();
        let mut seen = HashSet::new();
        let mut twentieth: HashMap<ClubId, i32> = HashMap::new();
        let mut sixteenth: HashMap<ClubId, i32> = HashMap::new();
        let mut promoted_today = HashSet::new();
        let mut notices = 0;
        let humans = humans(self.world);

        for p in self.world.game.youth_players.clone() {
            let Some(club) = self.world.player(p).club else { continue };
            let key = player_key(self.world, p);
            seen.insert(key.clone());
            let i = match index.find(self, &key) {
                Some(i) => i,
                None => {
                    // novo() then put(): a run of this tick inside novo (Patrimonio starting)
                    // may have added him meanwhile; put() keeps that place
                    let kid = self.new_kid(p, club, key.clone(), first, now);
                    match index.find(self, &key) {
                        Some(i) => {
                            self.kids()[i] = kid;
                            i
                        }
                        None => {
                            let kids = self.kids();
                            kids.push(kid);
                            let (i, len) = (kids.len() - 1, kids.len());
                            index.added(&key, i, len);
                            i
                        }
                    }
                }
            };
            self.correct_age_at(i, p);
            let factor = self.youth_league_development_factor(p) * self.academy_factor(club);
            let i = index.find(self, &key).expect("the kid was just seen");
            advance(&mut self.kids()[i], now, factor);
            let target = target_strength(&self.kids()[i]);
            self.sync_attributes(p, target);
            let potential = game_potential(self.kids()[i].ceiling);
            self.world.player_mut(p).set_potential(potential);

            let cut = *twentieth.entry(club).or_insert_with(|| nth_best(self.world, club, 20));
            let ready = target >= cut;
            let age = self.world.player(p).age;
            if humans.contains(&self.world.club(club).id) {
                // only kids who would really fight for a place (top 16), two per round at most
                let place = target >= *sixteenth.entry(club).or_insert_with(|| nth_best(self.world, club, 16));
                if place && !self.kids()[i].notified && notices < 2 {
                    self.kids()[i].notified = true;
                    notices += 1;
                    // Jornal.noticia: the academy's report is news only
                }
            } else if !is_national_team(self.world, club)
                && !promoted_today.contains(&club)
                && self.world.club(club).squad.len() < SQUAD_TARGET
                && (ready || age >= 20 || age >= 19 && preseason)
            {
                // one promotion per club per round, so a migrated academy doesn't flood the squad
                promoted_today.insert(club);
                Sim { world: self.world, runtime: self.runtime, hooks: &mut brasfoot_engine::hooks::NoSystems::default(), human: None }.promote_from_youth_list(p, club);
                self.kids()[i].professional = true;
                self.set_strength(p, target, true);
            }
        }

        for p in self.world.game.players.clone() {
            let key = player_key(self.world, p);
            let Some(i) = index.find(self, &key) else { continue };
            seen.insert(key.clone());
            if !self.kids()[i].professional {
                // promoted by the player or by the game: the game just recomputed his strength
                self.kids()[i].professional = true;
                let target = target_strength(&self.kids()[i]);
                self.set_strength(p, target, true);
            }
            if self.kids()[i].development_age < END_AGE {
                self.correct_age_at(i, p);
                let club = self.world.player(p).club;
                let factor = self.playing_time_factor(p) * self.training_factor(club) * self.loanee_development_factor(p);
                let i = index.find(self, &key).expect("the kid was just seen");
                advance(&mut self.kids()[i], now, factor);
                let target = target_strength(&self.kids()[i]);
                let ceiling = self.kids()[i].ceiling;
                let max = 95.min(ceiling + 2 + self.training_ceiling_bonus(club));
                let strength = self.world.player(p).strength;
                self.set_strength(p, max.min(strength.max(target)), false);
            }
        }

        self.kids().retain(|k| seen.contains(&k.key) && !(k.professional && k.development_age >= END_AGE));
        // taken after this run's own promotions, so the next refresh of the same day finds nothing
        self.sys.academy_last_run = Some(self.academy_run());
        self.youth_league_tick();
    }

    /// `Base.bonusDestaque(String, int, double)`: a kid's ceiling and development age go up.
    pub(crate) fn highlight_bonus(&mut self, key: &str, ceiling: i32, development: f64) {
        self.sys.academy_loaded = true;
        if let Some(kid) = self.kids().iter_mut().find(|k| k.key == key) {
            kid.ceiling = 95.min(kid.ceiling + ceiling);
            kid.development_age = END_AGE.min(kid.development_age + development);
        }
    }

    fn kids(&mut self) -> &mut Vec<AcademyKid> {
        self.world.mods.academy.get_or_insert_with(Vec::new)
    }

    /// `Base.corrigirIdade` on the kid at that place.
    fn correct_age_at(&mut self, i: usize, p: PlayerId) {
        let limit = self.world.player(p).age as f64 + season_fraction(self) + 0.3;
        let kid = &mut self.kids()[i];
        if kid.development_age > limit {
            kid.development_age = limit;
        }
    }

    /// `BaseDialog`'s "Promover ao Profissional": the junior joins the player's squad (at most
    /// 35), `Base.aoPromover` aligns his strength to the academy's.
    pub fn promote_junior(&mut self, club: ClubId, p: PlayerId) -> Result<(), PromoteError> {
        if self.world.club(club).squad.len() >= MAX_SQUAD {
            return Err(PromoteError::SquadFull);
        }
        Sim { world: self.world, runtime: self.runtime, hooks: &mut brasfoot_engine::hooks::NoSystems::default(), human: None }.promote_from_youth_list(p, club);
        // F.aB(300): the contract screen's transient state
        self.on_promoted(p);
        Ok(())
    }

    /// `Base.aoPromover(F)`: a promoted kid turns professional and gets his strength at once.
    pub(crate) fn on_promoted(&mut self, p: PlayerId) {
        let key = player_key(self.world, p);
        let target = self.world.mods.academy.as_mut().and_then(|kids| kids.iter_mut().find(|k| k.key == key)).map(|k| {
            k.professional = true;
            target_strength(k)
        });
        if let Some(target) = target {
            self.set_strength(p, target, true);
        }
    }

    /// `Base.vez()`: the date and the sizes of the youth and player lists.
    fn academy_run(&self) -> (i64, usize, usize) {
        (self.world.today_millis(), self.world.game.youth_players.len(), self.world.game.players.len())
    }

    /// `Base.novo(F, String, boolean, double)`
    fn new_kid(&mut self, p: PlayerId, club: ClubId, key: String, migration: bool, now: f64) -> AcademyKid {
        let mut ceiling = self.draw_ceiling(club);
        if migration {
            // existing kids keep a pull from the potential the game had given them
            ceiling += (self.world.player(p).potential as f64 - 5.5) * 2.5;
        }
        let ceiling = java_round(ceiling).clamp(30, 95) as i32;
        let age = self.world.player(p).age as f64 + season_fraction(self);
        let r = sorte!(self, "Base");
        let development_age = age + (r.next_double() - 0.5) * 0.6;
        let scout_error = r.next_gaussian();
        AcademyKid {
            key,
            ceiling,
            development_age,
            scout_error,
            professional: false,
            notified: false,
            year_joined: crate::economy::year(self.world),
            last_time: now,
        }
    }

    /// `Base.sortearTeto(ah)`: the same distribution everywhere; big clubs (and high-level
    /// academies, `Patrimonio`) scout more and filter better prospects.
    fn draw_ceiling(&mut self, club: ClubId) -> f64 {
        let academy = self.pillar_level(club, brasfoot_model::mods::Pillar::Academy);
        let mean = MEAN_CEILING + academy as f64 * 1.2;
        let reputation = self.world.club(club).reputation;
        let r = sorte!(self, "Base");
        let mut t = mean + r.next_gaussian() * CEILING_DEVIATION;
        let chance = (if reputation >= 5 { 35 } else if reputation == 4 { 20 } else { 0 }) + academy * 12;
        if r.next_int_bounded(100) < chance {
            t = t.max(mean + r.next_gaussian() * CEILING_DEVIATION);
        }
        if academy >= 4 && r.next_int_bounded(100) < (academy - 3) * 20 {
            t = t.max(mean + r.next_gaussian() * CEILING_DEVIATION);
        }
        t
    }

    /// `Base.fatorDeJogo(F)`: half the matches is full speed, a benchwarmer barely develops.
    fn playing_time_factor(&self, p: PlayerId) -> f64 {
        let fraction = season_fraction(self);
        if fraction < 0.15 {
            return 1.0;
        }
        let expected = 32.0 * fraction * 0.5;
        (season_games(self.world, p) as f64 / expected).clamp(0.15, 1.0)
    }

    /// `Base.sincronizarAtributos(F, int)`
    pub(crate) fn sync_attributes(&mut self, p: PlayerId, target: i32) {
        let expected = java_round(NORMAL_RATIO * target as f64) as i32;
        let misaligned = (best_value(self.world, p) - expected).abs() > 2;
        self.set_strength(p, target, misaligned);
    }

    /// `Motor.definirForca(F, int, boolean)`: sets a player's strength and, in individual-skills
    /// mode, carries his attributes along: `align` brings attributes rolled for another strength
    /// to what this one is worth, otherwise they grow in the same proportion as the strength.
    pub fn set_strength(&mut self, p: PlayerId, new: i32, align: bool) {
        let old = self.world.player(p).strength;
        self.world.player_mut(p).strength = new;
        if !self.world.game.individual_skills {
            return;
        }
        if align {
            let target = java_round(NORMAL_RATIO * new as f64) as i32;
            let now = best_value(self.world, p);
            if now != target {
                self.scale_attributes(p, target as f64 / now as f64);
            }
        } else if new > old && old > 0 {
            self.scale_attributes(p, new as f64 / old as f64);
        }
    }

    /// `Motor.escalar(F, double)`, in the order of its setters (`aJ`, `aM`, `aN`, `aO`, `aI`,
    /// `aK`, `aL`).
    fn scale_attributes(&mut self, p: PlayerId, f: f64) {
        for i in [0, 4, 5, 6, 1, 2, 3] {
            let v = self.world.player(p).attributes[i];
            // Motor.limite: the fraction is kept as a chance of the extra point
            let x = v as f64 * f;
            let base = x.floor() as i32;
            let extra = (sorte!(self, "Motor").next_double() < x - base as f64) as i32;
            self.world.player_mut(p).attributes[i] = (base + extra).clamp(1, 100);
        }
    }

    /// `Base.talentosNovos(Set, int)`: the year's best newly seen academy talents (ceiling 85+).
    pub(crate) fn new_talents(&self, announced: &HashSet<String>, limit: usize) -> Vec<(PlayerId, AcademyKid)> {
        let Some(kids) = &self.world.mods.academy else { return Vec::new() };
        let mut out = Vec::new();
        for &p in &self.world.game.youth_players {
            if self.world.player(p).age > 18 {
                continue;
            }
            let key = player_key(self.world, p);
            if let Some(k) = kids.iter().find(|k| k.key == key)
                && k.ceiling >= 85
                && !announced.contains(&key)
            {
                out.push((p, k.clone()));
            }
        }
        out.sort_by(|a, b| b.1.ceiling.cmp(&a.1.ceiling));
        out.truncate(limit);
        out
    }
}
