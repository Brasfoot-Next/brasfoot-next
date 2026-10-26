// Port of bfmod.Patrimonio: the clubs' infrastructure (arena, training centre and medical
// department, youth academy, global marketing, levels 0 to 5), stadium expansions, naming rights,
// marketing campaigns and the AI clubs' investments, with their effects on the pitch, the
// academy and the club's income, and the monthly upkeep.

use brasfoot_model::brands;
use brasfoot_model::ids::{ClubId, MatchId, PlayerId};
use brasfoot_model::mods::{AssetsState, ClubAssets, FanEvent, Pillar};
use brasfoot_model::rng::{JavaRandom, java_round};
use brasfoot_model::saved::StadiumWork;
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;

use crate::awards::season::player_key;
use crate::config::{ModConfig, Switch};
use crate::economy::{MI, ai_clubs, humans, is_national_team, league_weights, year};
use crate::staff::my_club;
use crate::{Mod, sorte};

/// `NIVEL_MAX`
pub const MAX_LEVEL: i32 = 5;
/// `CUSTO_NIVEL`: the works of each level.
pub const LEVEL_COST: [i64; 6] = [0, 15 * MI, 35 * MI, 80 * MI, 160 * MI, 300 * MI];
/// `MANUTENCAO_ARENA`, `MANUTENCAO_CT`, `MANUTENCAO_ACADEMIA`, `MANUTENCAO_MARKETING`: monthly.
const UPKEEP_ARENA: [i64; 6] = [0, 120_000, 280_000, 650_000, 1_400_000, 2_800_000];
const UPKEEP_TRAINING: [i64; 6] = [0, 100_000, 240_000, 550_000, 1_200_000, 2_400_000];
const UPKEEP_ACADEMY: [i64; 6] = [0, 80_000, 200_000, 480_000, 1_000_000, 2_000_000];
const UPKEEP_MARKETING: [i64; 6] = [0, 90_000, 220_000, 500_000, 1_100_000, 2_200_000];
/// `CAMPANHAS_CUSTO`, `CAMPANHAS_REQ_MKT`
pub const CAMPAIGN_COST: [i64; 4] = [0, 20 * MI, 65 * MI, 140 * MI];
pub const CAMPAIGN_MARKETING: [i32; 4] = [0, 1, 3, 4];

/// `Pilar.manutencao`
fn upkeep(pillar: Pillar) -> &'static [i64; 6] {
    match pillar {
        Pillar::Arena => &UPKEEP_ARENA,
        Pillar::TrainingCentre => &UPKEEP_TRAINING,
        Pillar::Academy => &UPKEEP_ACADEMY,
        Pillar::Marketing => &UPKEEP_MARKETING,
    }
}

/// `Pilar.manutencao[level]` for the screens.
pub fn pillar_upkeep(pillar: Pillar, level: i32) -> i64 {
    upkeep(pillar)[at(level)]
}

/// The level index of a table, clamped as the original does where it does.
fn at(level: i32) -> usize {
    level.clamp(0, MAX_LEVEL) as usize
}

/// `Patrimonio.limiteSetores()`: the most seats per sector (general, stands, seats, boxes) this
/// season (the mod fixes the game's screen, where the last two were out of reach).
pub fn sector_limits(world: &World) -> [i32; 4] {
    let season = world.season();
    if season >= 10 {
        [25_000, 130_000, 20_000, 1_200]
    } else if season >= 6 {
        [25_000, 120_000, 15_000, 1_000]
    } else if season >= 2 {
        // best.aq.pa[1]
        [20_000, 100_000, 10_000, 800]
    } else {
        [18_000, 80_000, 9_000, 700]
    }
}

/// `Patrimonio.limiteTotal()`
pub fn total_limit(world: &World) -> i32 {
    sector_limits(world).iter().sum()
}

/// `Patrimonio.custoSetor(int, int, int)`: the cost of adding seats to a sector.
pub fn sector_cost(sector: usize, current: i32, add: i32) -> i64 {
    if add <= 0 {
        return 0;
    }
    const PRICES: [[i64; 6]; 4] = [[80, 160, 240, 500, 700, 1100], [120, 380, 640, 700, 1400, 2200], [300, 600, 750, 800, 1200, 2000], [1500, 3500, 4000, 6000, 6400, 10000]];
    const BANDS: [[i32; 6]; 4] =
        [[1000, 2500, 3500, 10000, 20000, 30000], [5000, 15000, 30000, 60000, 100000, 140000], [1000, 2000, 3000, 5000, 10000, 25000], [100, 200, 500, 600, 800, 1500]];
    let target = current + add;
    let band = BANDS[sector].iter().position(|&b| target <= b).unwrap_or(5);
    PRICES[sector][band] * add as i64
}

/// `Patrimonio.diasObra(int)`
pub fn work_days(seats: i32) -> i32 {
    if seats < 1000 {
        15
    } else if seats < 10_000 {
        20
    } else if seats < 30_000 {
        30
    } else {
        40
    }
}

/// `Patrimonio.metaJuniores(ah)`
pub fn youth_target(world: &World, config: &ModConfig, club: ClubId) -> i32 {
    10 + level(world, config, club, Pillar::Academy) * 2
}

/// `Patrimonio.nivel(ah, Pilar)` for the readers of the world: 0 with the system off. A club the
/// system does not know yet counts as level 0 (the original adds it then, at level 0).
pub fn level(world: &World, config: &ModConfig, club: ClubId, pillar: Pillar) -> i32 {
    if !config.on(Switch::Assets) {
        return 0;
    }
    assets_of(world, club).map_or(0, |e| e.level(pillar))
}

fn assets_of(world: &World, club: ClubId) -> Option<&ClubAssets> {
    world.mods.assets.as_ref()?.club(world.club(club).id)
}

/// `Patrimonio.fatorSocio(ah)` of a club's infrastructure.
fn members_factor_of(e: &ClubAssets, season: i32) -> f64 {
    const MARKETING: [f64; 6] = [0.0, 0.08, 0.16, 0.26, 0.38, 0.50];
    let mut f = 1.0 + MARKETING[at(e.marketing)];
    if e.campaign_season == season {
        const CAMPAIGN: [f64; 4] = [0.0, 0.15, 0.20, 0.30];
        f += CAMPAIGN[e.campaign_kind.clamp(0, 3) as usize];
    }
    f
}

/// `Patrimonio.fatorPatrocinio(ah)` of a club's infrastructure.
fn sponsorship_factor_of(e: &ClubAssets) -> f64 {
    const MARKETING: [f64; 6] = [0.0, 0.06, 0.12, 0.20, 0.30, 0.45];
    1.0 + MARKETING[at(e.marketing)]
}

/// `Patrimonio.fatorSocio(ah)` for the screens (the system already started).
pub fn members_factor_now(world: &World, config: &ModConfig, club: ClubId) -> f64 {
    if !config.on(Switch::Assets) {
        return 1.0;
    }
    members_factor_of(&assets_of(world, club).cloned().unwrap_or_default(), world.season())
}

/// `Patrimonio.fatorPatrocinio(ah)` for the screens (the system already started).
pub fn sponsorship_factor_now(world: &World, config: &ModConfig, club: ClubId) -> f64 {
    if !config.on(Switch::Assets) {
        return 1.0;
    }
    sponsorship_factor_of(&assets_of(world, club).cloned().unwrap_or_default())
}

/// `Patrimonio.bonusPartida(I, int)`: a side's arena (home only) and physical preparation.
pub fn match_bonus(world: &World, config: &ModConfig, m: MatchId, side: usize) -> f64 {
    if !config.on(Switch::Assets) {
        return 1.0;
    }
    let g = world.game_match(m);
    let Some(club) = (if side == 0 { g.home } else { g.away }) else { return 1.0 };
    if is_national_team(world, club) {
        return 1.0;
    }
    let e = assets_of(world, club).cloned().unwrap_or_default();
    let mut bonus = 1.0 + 0.004 * e.training_centre as f64;
    if side == 0 && !g.knockout {
        const HOME: [f64; 6] = [0.0, 0.0, 0.010, 0.015, 0.022, 0.030];
        bonus += HOME[at(e.arena)];
    }
    bonus
}

/// `Patrimonio.metaCapacidadeIA(ah, int, boolean, long)`: the stadium an AI club aims for.
fn capacity_target(world: &World, club: ClubId, reputation: i32, owner: bool, fortune: i64) -> i32 {
    let id = world.club(club).id;
    let spread = |factor: i32, modulo: i32| id.wrapping_mul(factor).wrapping_abs() % modulo;
    let mut base = if reputation >= 5 || owner {
        78_000 + spread(37, 32_000)
    } else if reputation == 4 {
        52_000 + spread(37, 24_000)
    } else if reputation == 3 {
        34_000 + spread(37, 14_000)
    } else {
        22_000 + spread(37, 8_000)
    };
    if fortune >= 800 * MI {
        base = base.max(95_000 + spread(19, 25_000));
    } else if fortune >= 350 * MI {
        base = base.max(72_000 + spread(19, 18_000));
    }
    base.min(total_limit(world))
}

/// `Patrimonio.escolherPilarIA(Estrutura, int)`: the lowest pillar under the ceiling.
fn ai_pillar(e: &ClubAssets, ceiling: i32) -> Option<Pillar> {
    let mut best = None;
    let mut min = 99;
    for p in Pillar::ALL {
        let v = e.level(p);
        if v < ceiling && v < min {
            min = v;
            best = Some(p);
        }
    }
    best
}

/// Why the player's infrastructure action was refused (the original's messages).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetsError {
    /// "Clube sem estádio." / "Clube inválido." / "Proposta inválida."
    NoStadium,
    /// "Já existe uma obra de ampliação em andamento no estádio."
    WorkInProgress,
    /// "Selecione pelo menos 1 lugar para construir."
    NoSeats,
    /// Not enough cash for the cost.
    Cash { cost: i64 },
    /// The pillar is at level 5 already.
    MaxLevel,
    /// "Já existe um contrato de Naming Rights ativo até {year}."
    NamingActive { until: i32 },
    /// "Requer Modernização da Arena Nível 3+ e Marketing Global Nível 2+."
    NamingNeedsLevels,
    /// "Não há contrato ativo de Naming Rights."
    NoNaming,
    /// "Campanha inválida."
    BadCampaign,
    /// The club already ran a campaign this season.
    CampaignDone { kind: i32 },
    /// "Requer Departamento de Marketing Global no Nível {n} ou superior."
    CampaignNeedsMarketing { level: i32 },
}

/// `Patrimonio.PropostaNaming`
#[derive(Clone, Debug, PartialEq)]
pub struct NamingOffer {
    /// Index into `brands::NAMING_BRANDS`.
    pub brand: usize,
    pub stadium_name: String,
    pub signing_fee: i64,
    pub yearly: i64,
    pub years: i32,
}

impl Mod<'_> {
    /// `Patrimonio.carregar()` then `iniciar()` when this save has no state yet.
    fn load_assets(&mut self) {
        if self.sys.assets_loaded && self.world.mods.assets.is_some() {
            return;
        }
        self.sys.assets_loaded = true;
        self.sys.assets_last_month = -1;
        self.sys.treated_injuries.clear();
        if self.world.mods.assets.as_ref().is_none_or(|a| a.clubs.is_empty()) {
            let season = self.world.season();
            self.world.mods.assets = Some(AssetsState { clubs: Vec::new(), last_season: season });
            self.start_assets();
        }
    }

    fn assets(&mut self) -> &mut AssetsState {
        self.world.mods.assets.as_mut().expect("the assets' state is loaded")
    }

    /// `Patrimonio.estrutura(ah)`: a club the system does not know yet joins, with its stadium's
    /// name as the base.
    fn club_assets(&mut self, club: ClubId) -> &mut ClubAssets {
        let id = self.world.club(club).id;
        let name = self.world.club(club).stadium.map(|s| self.world.stadium(s).name.clone());
        let state = self.assets();
        let i = match state.clubs.iter().position(|c| c.id == id) {
            Some(i) => i,
            None => {
                state.clubs.push(ClubAssets { id, stadium_base_name: name.unwrap_or_default(), ..Default::default() });
                state.clubs.len() - 1
            }
        };
        &mut state.clubs[i]
    }

    /// `Patrimonio.iniciar()`: every club's starting levels by its reputation (the player's
    /// modest, to leave plenty to build), one AI investment pass and the youth trials.
    fn start_assets(&mut self) {
        self.assets().last_season = self.world.season();
        let humans = humans(self.world);
        for club in self.world.game.clubs.clone() {
            if is_national_team(self.world, club) {
                continue;
            }
            let c = self.world.club(club);
            let mut e = ClubAssets { id: c.id, ..Default::default() };
            if let Some(s) = c.stadium {
                e.stadium_base_name = self.world.stadium(s).name.clone();
            }
            let reputation = c.reputation.clamp(0, 5);
            let owner = self.has_owner(club);
            if humans.contains(&e.id) {
                e.arena = if reputation >= 5 { 1 } else { 0 };
                e.training_centre = if reputation >= 5 { 2 } else if reputation == 4 { 1 } else { 0 };
                e.academy = if reputation >= 5 { 2 } else if reputation >= 3 { 1 } else { 0 };
                e.marketing = if reputation >= 5 { 1 } else { 0 };
            } else {
                e.arena = if reputation >= 5 { 2 } else if reputation == 4 { 1 } else { 0 };
                let r = sorte!(self, "Patrimonio");
                e.training_centre = match reputation {
                    5 => 2 + owner as i32,
                    4 => 1 + r.next_int_bounded(2),
                    3 => r.next_int_bounded(2),
                    _ => 0,
                };
                e.academy = match reputation {
                    5 => 2 + r.next_int_bounded(2),
                    4 => 1 + r.next_int_bounded(2),
                    3 => 1,
                    _ => 0,
                };
                e.marketing = if reputation >= 5 { 2 + owner as i32 } else if reputation == 4 { 1 } else { 0 };
            }
            if e.arena >= 1
                && let Some(s) = self.world.club(club).stadium
            {
                self.world.stadium_mut(s).condition = 0;
            }
            // put(): the economy, started by temDono above, may have added the club already
            // (fatorPatrocinio); it keeps that place
            let clubs = &mut self.assets().clubs;
            match clubs.iter().position(|c| c.id == e.id) {
                Some(i) => clubs[i] = e,
                None => clubs.push(e),
            }
        }
        // one pass at once, so the rich clubs of an existing save start upgrading
        self.ai_investments(true);
        for club in self.world.game.clubs.clone() {
            if !is_national_team(self.world, club) {
                self.youth_trials(club, false);
            }
        }
    }

    /// `Patrimonio.tick()`
    pub fn assets_tick(&mut self) {
        if !self.sys.config.on(Switch::Assets) {
            return;
        }
        self.load_assets();
        self.ensure_stadium_names();
        self.treat_injuries();
        let season = self.world.season();
        if season > self.assets().last_season {
            self.assets().last_season = season;
            // novaTemporada(): the youth trials, then the AI's season investments
            for club in self.world.game.clubs.clone() {
                if !is_national_team(self.world, club) {
                    self.youth_trials(club, false);
                }
            }
            self.ai_investments(true);
        }
        let month = self.runtime.clock.fields(self.world.today_millis()).month + 1;
        if month == 7 && self.sys.assets_last_month != 7 {
            self.sys.assets_last_month = 7;
            self.ai_investments(false);
        } else if month != 7 && month > 0 {
            self.sys.assets_last_month = month;
        }
    }

    /// `Patrimonio.fimDeTemporada()`: naming rights that end, clubs that climb a reputation
    /// tier with world-class infrastructure, and the AI's investments.
    pub(crate) fn assets_season_end(&mut self) {
        if !self.sys.config.on(Switch::Assets) {
            return;
        }
        self.load_assets();
        let year = year(self.world);
        for club in self.world.game.clubs.clone() {
            if is_national_team(self.world, club) {
                continue;
            }
            let e = self.club_assets(club);
            if !e.naming_brand.is_empty() && year >= e.naming_until {
                e.naming_brand.clear();
                e.naming_yearly = 0;
                e.naming_until = 0;
                self.sync_stadium_name(club);
                // the end of the player's naming rights is news only
            }
            let e = self.club_assets(club).clone();
            let sum = e.arena + e.training_centre + e.academy + e.marketing;
            if sum >= 16 && self.world.club(club).reputation < 5 && sorte!(self, "Patrimonio").next_int_bounded(100) < 45 {
                self.world.club_mut(club).reputation += 1;
            }
        }
        self.ai_investments(true);
    }

    /// `Patrimonio.garantirNomesEstadios()`
    fn ensure_stadium_names(&mut self) {
        for club in self.world.game.clubs.clone() {
            let Some(s) = self.world.club(club).stadium else { continue };
            if is_national_team(self.world, club) {
                continue;
            }
            let name = self.world.stadium(s).name.clone();
            let e = self.club_assets(club);
            if e.stadium_base_name.is_empty() {
                e.stadium_base_name = name;
            }
            if e.arena >= 1 {
                self.world.stadium_mut(s).condition = 0;
            }
        }
    }

    /// `Patrimonio.tratarLesoes()`: newly seen injuries are shortened by the club's medical
    /// department and hybrid pitch.
    fn treat_injuries(&mut self) {
        let now = self.world.today_millis();
        for p in self.world.game.players.clone() {
            let Some(club) = self.world.player(p).club else { continue };
            let key = player_key(self.world, p);
            if !self.world.player(p).is_injured(now) {
                self.sys.treated_injuries.remove(&key);
                continue;
            }
            let end = self.world.player(p).injured_until;
            if self.sys.treated_injuries.get(&key).is_some_and(|&t| (t - end).abs() < 120_000) {
                continue;
            }
            let e = self.club_assets(club);
            let mut reduction = 0.08 * e.training_centre as f64;
            if e.arena >= 1 {
                // the hybrid pitch lowers the average severity
                reduction += 0.07;
            }
            let reduction = reduction.min(0.50);
            if reduction > 0.01 && end > now + 86_400_000 {
                let length = end - now;
                let shorter = 86_400_000_i64.max(java_round(length as f64 * (1.0 - reduction)));
                let new_end = now + shorter;
                self.world.player_mut(p).injured_until = new_end;
                self.sys.treated_injuries.insert(key, new_end);
            } else {
                self.sys.treated_injuries.insert(key, end);
            }
        }
    }

    /// `Patrimonio.peneiraAnual(ah, boolean)`: the season's youth trials fill the club's youth
    /// squad up to the academy's target (or at least one or two kids).
    pub(crate) fn youth_trials(&mut self, club: ClubId, forced: bool) -> i32 {
        if is_national_team(self.world, club) {
            return 0;
        }
        self.load_assets();
        let season = self.world.season();
        let academy = {
            let e = self.club_assets(club);
            if !forced && e.last_trials == season {
                return 0;
            }
            e.last_trials = season;
            e.academy
        };
        let mut by_position = [0; 5];
        let mut total = 0;
        for &p in &self.world.game.youth_players {
            let player = self.world.player(p);
            if player.club == Some(club) {
                total += 1;
                by_position[player.position.clamp(0, 4) as usize] += 1;
            }
        }
        let target = youth_target(self.world, &self.sys.config, club);
        let minimum = if forced { 2 } else { 1 + academy / 2 };
        let room = ((target + 2).min(22) - total).max(0);
        let count = room.min((target - total).max(minimum));
        if count <= 0 {
            return 0;
        }
        const WANTED: [i32; 5] = [2, 3, 3, 4, 3];
        let mut made = 0;
        for _ in 0..count {
            let mut position = -1;
            let mut missing = -99;
            for pos in 0..5 {
                let m = WANTED[pos] - by_position[pos];
                if m > missing {
                    missing = m;
                    position = pos as i32;
                }
            }
            let p = Sim { world: self.world, runtime: self.runtime, hooks: &mut brasfoot_engine::hooks::NoSystems::default(), human: None }
                .new_youth_player(club, position, None, false);
            let age = 16 + sorte!(self, "Patrimonio").next_int_bounded(2);
            self.world.player_mut(p).set_age(age);
            by_position[position as usize] += 1;
            made += 1;
        }
        if made > 0 && Some(club) == my_club(self.world) {
            self.academy_tick();
            // Jornal.noticia: the trials' news is not ported
        }
        made
    }

    /// `Patrimonio.investimentosIA(boolean)`: the AI clubs' upkeep (at a season's start), seat
    /// expansions, one infrastructure level and naming rights, in a shuffled order.
    fn ai_investments(&mut self, season_start: bool) {
        let mut clubs = ai_clubs(self.world);
        sorte!(self, "Patrimonio").shuffle(&mut clubs);
        let year = year(self.world);
        for club in clubs {
            let Some(stadium) = self.world.club(club).stadium else { continue };
            self.club_assets(club);
            let mut fortune = self.fortune(club);
            let yearly_wages = (8 * MI).max(self.world.club_wages(club) * 12);
            let reputation = self.world.club(club).reputation.clamp(0, 5);
            let owner = self.has_owner(club);
            if season_start {
                let e = self.club_assets(club).clone();
                let upkeep_year = (UPKEEP_ARENA[at(e.arena)] + UPKEEP_TRAINING[at(e.training_centre)] + UPKEEP_ACADEMY[at(e.academy)] + UPKEEP_MARKETING[at(e.marketing)]) * 12;
                let naming = if !e.naming_brand.is_empty() && year <= e.naming_until { e.naming_yearly } else { 0 };
                self.spend(club, upkeep_year - naming);
                fortune = self.fortune(club);
            }
            let reserve = (20 * MI).max(java_round(yearly_wages as f64 * 0.65));
            let mut spare = fortune - reserve;
            if spare < 15 * MI {
                continue;
            }

            // 1) seats
            let capacity: i32 = self.world.stadium(stadium).capacity.iter().sum();
            let target = capacity_target(self.world, club, reputation, owner, fortune);
            if capacity < target && spare >= 18 * MI {
                let missing = target - capacity;
                let r = sorte!(self, "Patrimonio");
                let mut add = missing.min(7000 + r.next_int_bounded(14_001));
                if spare > 300 * MI {
                    add = missing.min(18_000 + r.next_int_bounded(22_001));
                }
                let per_seat = 1800.0 + reputation as f64 * 300.0;
                let mut cost = (15 * MI).max(java_round(add as f64 * per_seat));
                if spare < cost && spare >= 16 * MI {
                    add = 5000_i64.max((missing as i64).min(java_round(spare as f64 * 0.75 / per_seat))) as i32;
                    cost = (14 * MI).max(java_round(add as f64 * per_seat));
                }
                if spare >= cost {
                    self.expand_stadium(club, add);
                    self.spend(club, cost);
                    spare -= cost;
                    // the expansion's news is not ported
                }
            }

            // 2) one infrastructure level
            let ceiling = if reputation >= 5 || owner || fortune >= 450 * MI {
                5
            } else if reputation == 4 {
                4
            } else if reputation == 3 {
                3
            } else {
                2
            };
            let e = self.club_assets(club).clone();
            if let Some(pillar) = ai_pillar(&e, ceiling) {
                let next = self.pillar_level(club, pillar) + 1;
                let cost = LEVEL_COST[next as usize];
                if spare >= cost {
                    self.club_assets(club).set_level(pillar, next);
                    if pillar == Pillar::Arena {
                        self.world.stadium_mut(stadium).condition = 0;
                        self.rename_generic_stadium(club);
                    }
                    self.spend(club, cost);
                    // the inauguration's news is not ported
                }
            }

            // 3) naming rights
            let e = self.club_assets(club).clone();
            if season_start && e.arena >= 3 && e.marketing >= 2 && e.naming_brand.is_empty() && sorte!(self, "Patrimonio").next_int_bounded(100) < 25 {
                let brand = brands::brand(self.new_brand());
                let value = java_round(self.naming_base_value(club));
                let until = year + 2 + sorte!(self, "Patrimonio").next_int_bounded(3);
                let e = self.club_assets(club);
                e.naming_brand = brand;
                e.naming_yearly = value;
                e.naming_until = until;
                self.sync_stadium_name(club);
            }
        }
    }

    /// `Patrimonio.nivel(ah, Pilar)`: 0 with the system off; the first call starts it.
    pub fn pillar_level(&mut self, club: ClubId, pillar: Pillar) -> i32 {
        if !self.sys.config.on(Switch::Assets) {
            return 0;
        }
        self.load_assets();
        self.club_assets(club).level(pillar)
    }

    /// `Patrimonio.dados(ah)` with the system on (the factors check the switch first).
    pub fn assets_data(&mut self, club: ClubId) -> ClubAssets {
        self.load_assets();
        self.club_assets(club).clone()
    }

    /// `Patrimonio.fatorSocio(ah)`: the members' income.
    pub(crate) fn members_factor(&mut self, club: ClubId) -> f64 {
        if !self.sys.config.on(Switch::Assets) {
            return 1.0;
        }
        let e = self.assets_data(club);
        members_factor_of(&e, self.world.season())
    }

    /// `Patrimonio.fatorPatrocinio(ah)`: the master sponsor's offers (and the economy's).
    pub(crate) fn sponsorship_factor(&mut self, club: ClubId) -> f64 {
        if !self.sys.config.on(Switch::Assets) {
            return 1.0;
        }
        let e = self.assets_data(club);
        sponsorship_factor_of(&e)
    }

    /// `Patrimonio.fatorEvolucaoAcademia(ah)`: the academy's development speed.
    pub(crate) fn academy_factor(&mut self, club: ClubId) -> f64 {
        1.0 + 0.06 * self.pillar_level(club, Pillar::Academy) as f64
    }

    /// `Patrimonio.fatorEvolucaoCT(ah)`: the under-25 professionals' development speed.
    pub(crate) fn training_factor(&mut self, club: Option<ClubId>) -> f64 {
        match club {
            Some(c) => 1.0 + 0.07 * self.pillar_level(c, Pillar::TrainingCentre) as f64,
            None => 1.0,
        }
    }

    /// `Patrimonio.bonusTetoCT(ah)`: the extra ceiling of elite training centres (level 4: +1,
    /// level 5: +2).
    pub(crate) fn training_ceiling_bonus(&mut self, club: Option<ClubId>) -> i32 {
        let n = match club {
            Some(c) => self.pillar_level(c, Pillar::TrainingCentre),
            // nivel(null, CT) is 0
            None => 0,
        };
        if n >= 5 {
            2
        } else if n == 4 {
            1
        } else {
            0
        }
    }

    /// `Patrimonio.expandirEstadioIA(ah, Estrutura, int)`: the seats at once, by the sectors'
    /// shares.
    fn expand_stadium(&mut self, club: ClubId, add: i32) {
        let limits = sector_limits(self.world);
        let Some(s) = self.world.club(club).stadium else { return };
        const SHARE: [f64; 4] = [0.14, 0.68, 0.16, 0.02];
        for sector in 0..4 {
            let seats = java_round(add as f64 * SHARE[sector]) as i32;
            let room = (limits[sector] - self.world.stadium(s).capacity[sector]).max(0);
            self.world.stadium_mut(s).capacity[sector] += room.min(seats);
        }
        self.rename_generic_stadium(club);
    }

    /// `Patrimonio.rebatizarGenericoSePreciso(ah, Estrutura)`: a municipal stadium grown past
    /// 32 000 seats becomes the club's arena.
    fn rename_generic_stadium(&mut self, club: ClubId) {
        let base = self.stadium_base_name(club);
        let Some(s) = self.world.club(club).stadium else { return };
        let seats: i32 = self.world.stadium(s).capacity.iter().sum();
        if base.starts_with(brands::MUNICIPAL_STADIUM) && seats >= 32_000 {
            let c = self.world.club(club);
            let name = brands::club_arena(c.country, c.id, &c.name);
            self.club_assets(club).stadium_base_name = name;
            self.sync_stadium_name(club);
        }
    }

    /// `Patrimonio.nomeBaseEstadio(ah, Estrutura)`
    fn stadium_base_name(&mut self, club: ClubId) -> String {
        let name = self.world.club(club).stadium.map(|s| self.world.stadium(s).name.clone());
        let e = self.club_assets(club);
        if !e.stadium_base_name.is_empty() {
            return e.stadium_base_name.clone();
        }
        match name {
            Some(n) => {
                e.stadium_base_name = n.clone();
                n
            }
            None => brands::base_stadium_name(),
        }
    }

    /// `Patrimonio.sincronizarNomeEstadio(ah, Estrutura)`: the stadium's name with its active
    /// naming rights, or its base name.
    fn sync_stadium_name(&mut self, club: ClubId) {
        let Some(s) = self.world.club(club).stadium else { return };
        let base = self.stadium_base_name(club);
        let year = year(self.world);
        let e = self.club_assets(club).clone();
        let name = if !e.naming_brand.is_empty() && year <= e.naming_until {
            brands::sponsored_stadium(Some(&base), &e.naming_brand).unwrap_or(base)
        } else {
            base
        };
        self.world.stadium_mut(s).name = name;
    }

    /// `Patrimonio.aoRenomearEstadio(ah, String)`: a legend's name (`Estadios`) becomes the
    /// base. It runs with the system off too, and starts it.
    pub fn on_stadium_renamed(&mut self, club: ClubId, base: String) {
        self.load_assets();
        self.club_assets(club).stadium_base_name = base;
        self.sync_stadium_name(club);
    }

    /// `Patrimonio.valorBaseNaming(ah, Estrutura)`
    fn naming_base_value(&mut self, club: ClubId) -> f64 {
        let e = self.club_assets(club).clone();
        let c = self.world.club(club);
        let yearly_wages = (10 * MI) as f64;
        let yearly_wages = yearly_wages.max(self.world.club_wages(club) as f64 * 12.0);
        let reputation = c.reputation.max(1);
        let weight = league_weights(self.world).get(&club).copied().unwrap_or(1.0);
        let seats: i32 = c.stadium.map_or(0, |s| self.world.stadium(s).capacity.iter().sum());
        let arena = 0.75 + 0.12 * e.arena as f64 + (seats as f64 / 350_000.0).min(0.25);
        let marketing = 0.80 + 0.10 * e.marketing as f64;
        ((4 * MI) as f64).max(yearly_wages * (0.04 + 0.015 * reputation as f64) * (weight / 1.5) * arena * marketing)
    }

    /// `Patrimonio.pagarMes()`: the human club's upkeep and naming rights; a modern arena now
    /// and then pleases the crowd.
    pub(crate) fn pay_assets_month(&mut self) {
        let Some(club) = my_club(self.world) else { return };
        if !self.sys.config.on(Switch::Assets) {
            return;
        }
        self.load_assets();
        let e = self.club_assets(club).clone();
        let arena = UPKEEP_ARENA[at(e.arena)];
        if arena > 0 {
            self.world.club_payment(club, arena.min(i32::MAX as i64) as i32, 4);
        }
        let others = UPKEEP_TRAINING[e.training_centre as usize] + UPKEEP_ACADEMY[e.academy as usize] + UPKEEP_MARKETING[e.marketing as usize];
        if others > 0 {
            self.world.club_payment(club, others.min(i32::MAX as i64) as i32, 8);
        }
        if !e.naming_brand.is_empty() && e.naming_yearly > 0 && year(self.world) <= e.naming_until {
            self.world.club_income(club, (e.naming_yearly / 12).min(i32::MAX as i64) as i32, 6);
        }
        if e.arena >= 3 && crate::fans::confidence(self.world) < 95 && sorte!(self, "Patrimonio").next_int_bounded(100) < 35 {
            self.fans_event(1, FanEvent::ArenaComfort);
        }
    }

    /// `Patrimonio.aplicarBilheteria(I)`: once per match, at kick-off: the home club's arena and
    /// marketing raise the crowd and the gate.
    pub(crate) fn apply_gate(&mut self, m: MatchId) {
        if !self.sys.config.on(Switch::Assets) {
            return;
        }
        let g = self.world.game_match(m);
        let (Some(home), Some(stadium)) = (g.home, g.stadium) else { return };
        if g.knockout || !self.sys.gates_applied.insert(m) {
            return;
        }
        self.load_assets();
        let e = self.club_assets(home).clone();
        if e.arena >= 1 {
            self.world.stadium_mut(stadium).condition = 0;
        }
        const CROWD_ARENA: [f64; 6] = [0.0, 0.0, 0.02, 0.04, 0.08, 0.12];
        const CROWD_MARKETING: [f64; 6] = [0.0, 0.0, 0.03, 0.06, 0.10, 0.15];
        const GATE_ARENA: [f64; 6] = [0.0, 0.04, 0.08, 0.14, 0.22, 0.32];
        let mut crowd = 1.0 + CROWD_ARENA[e.arena as usize] + CROWD_MARKETING[e.marketing as usize];
        if e.campaign_season == self.world.season() {
            const CAMPAIGN: [f64; 4] = [0.0, 0.05, 0.08, 0.12];
            crowd += CAMPAIGN[e.campaign_kind.clamp(0, 3) as usize];
        }
        let mut gate = 1.0 + GATE_ARENA[e.arena as usize];
        let capacity = self.world.stadium(stadium).capacity.clone();
        let g = self.world.game_match_mut(m);
        if crowd > 1.001 && g.attendance.len() == 4 {
            let before = g.attendance.iter().sum::<i32>().max(1);
            let mut after = 0;
            for i in 0..4 {
                g.attendance[i] = capacity[i].min(java_round(g.attendance[i] as f64 * crowd) as i32);
                after += g.attendance[i];
            }
            gate *= after as f64 / before as f64;
        }
        if gate > 1.001 && g.income > 0 {
            g.income = java_round(g.income as f64 * gate).min(i32::MAX as i64) as i32;
        }
    }

    // The player's actions

    /// `Patrimonio.iniciarExpansao(ah, int[])`: a stadium work for the player's club.
    pub fn start_stadium_expansion(&mut self, club: ClubId, add: [i32; 4]) -> Result<(), AssetsError> {
        let Some(stadium) = self.world.club(club).stadium else { return Err(AssetsError::NoStadium) };
        if self.world.game.stadium_works.iter().any(|w| w.stadium == Some(stadium)) {
            return Err(AssetsError::WorkInProgress);
        }
        let limits = sector_limits(self.world);
        let current = self.world.stadium(stadium).capacity.clone();
        let mut total = 0;
        let mut cost = 100_000;
        let mut seats = vec![0; 4];
        for s in 0..4 {
            let max = (limits[s] - current[s]).max(0);
            let n = add[s].min(max).max(0);
            seats[s] = n;
            total += n;
            cost += sector_cost(s, current[s], n);
        }
        if total <= 0 {
            return Err(AssetsError::NoSeats);
        }
        if self.world.club(club).cash < cost {
            return Err(AssetsError::Cash { cost });
        }
        let today = self.world.today_millis();
        // Calendar.getInstance() set to today's date: the clock's time of day
        let f = self.runtime.clock.fields(today);
        let start = self.runtime.clock.date_now(f.year, f.month, f.day);
        let until = self.runtime.clock.add_days(start, work_days(total));
        self.world.game.stadium_works.push(StadiumWork { stadium: Some(stadium), until, seats });
        self.world.club_payment(club, cost.min(i32::MAX as i64) as i32, 7);
        Ok(())
    }

    /// `Patrimonio.evoluir(ah, Pilar)`: the next level of a pillar.
    pub fn upgrade_pillar(&mut self, club: ClubId, pillar: Pillar) -> Result<(), AssetsError> {
        self.load_assets();
        let current = self.pillar_level(club, pillar);
        if current >= MAX_LEVEL {
            return Err(AssetsError::MaxLevel);
        }
        let next = current + 1;
        let cost = LEVEL_COST[next as usize];
        if self.world.club(club).cash < cost {
            return Err(AssetsError::Cash { cost });
        }
        let kind = if pillar == Pillar::Arena { 7 } else { 8 };
        self.world.club_payment(club, cost.min(i32::MAX as i64) as i32, kind);
        self.club_assets(club).set_level(pillar, next);
        if pillar == Pillar::Arena
            && let Some(s) = self.world.club(club).stadium
        {
            self.world.stadium_mut(s).condition = 0;
        }
        if pillar == Pillar::Academy {
            self.youth_trials(club, true);
        }
        let inauguration = match pillar {
            Pillar::Arena => next >= 3,
            _ => next >= 4,
        };
        if inauguration {
            self.fans_event(3, FanEvent::Inauguration { pillar, level: next });
        }
        // Jornal.noticia: the inauguration's news is not ported
        Ok(())
    }

    /// `Patrimonio.evoluirFinanciadoPor(ah, ah, Pilar)`: the parent pays the next level of a
    /// satellite's pillar.
    pub(crate) fn upgrade_financed_by(&mut self, parent: ClubId, satellite: ClubId, pillar: Pillar) -> Result<(), AssetsError> {
        self.load_assets();
        let current = self.pillar_level(satellite, pillar);
        if current >= MAX_LEVEL {
            return Err(AssetsError::MaxLevel);
        }
        let next = current + 1;
        let cost = LEVEL_COST[next as usize];
        if self.world.club(parent).cash < cost {
            return Err(AssetsError::Cash { cost });
        }
        let kind = if pillar == Pillar::Arena { 7 } else { 8 };
        self.world.club_payment(parent, cost.min(i32::MAX as i64) as i32, kind);
        self.club_assets(satellite).set_level(pillar, next);
        if pillar == Pillar::Arena
            && let Some(s) = self.world.club(satellite).stadium
        {
            self.world.stadium_mut(s).condition = 0;
        }
        if pillar == Pillar::Academy {
            self.youth_trials(satellite, true);
        }
        // Jornal.noticia: the investment's news is not ported
        Ok(())
    }

    /// `Patrimonio.propostasNaming(ah)`: three naming-rights offers, fixed per club, year and
    /// levels.
    pub fn naming_offers(&mut self, club: ClubId) -> Vec<NamingOffer> {
        if self.world.club(club).stadium.is_none() {
            return Vec::new();
        }
        self.load_assets();
        let e = self.club_assets(club).clone();
        if e.arena < 3 || e.marketing < 2 {
            return Vec::new();
        }
        let base = self.stadium_base_name(club);
        let yearly = self.naming_base_value(club);
        let seed = 131 * self.world.club(club).id as i64 + 997 * year(self.world) as i64 + e.arena as i64 * 17 + e.marketing as i64 * 31;
        let mut r = JavaRandom::new(seed);
        let mut order: Vec<usize> = (0..brands::NAMING_BRANDS.len()).collect();
        r.shuffle(&mut order);
        let mut offer = |brand: usize, years: i32, low: f64, spread: f64, fee: f64| {
            let yearly = java_round(yearly * (low + r.next_double() * spread));
            let stadium_name = brands::sponsored_stadium(Some(&base), brands::NAMING_BRANDS[brand]).unwrap_or_default();
            NamingOffer { brand, stadium_name, signing_fee: java_round(yearly as f64 * fee), yearly, years }
        };
        vec![offer(order[0], 2, 0.95, 0.1, 0.35), offer(order[1], 3, 1.12, 0.12, 0.50), offer(order[2], 4, 0.90, 0.1, 1.15)]
    }

    /// `Patrimonio.assinarNaming(ah, PropostaNaming)`
    pub fn sign_naming(&mut self, club: ClubId, offer: &NamingOffer) -> Result<(), AssetsError> {
        if self.world.club(club).stadium.is_none() {
            return Err(AssetsError::NoStadium);
        }
        self.load_assets();
        let year = year(self.world);
        let e = self.club_assets(club).clone();
        if !e.naming_brand.is_empty() && year < e.naming_until {
            return Err(AssetsError::NamingActive { until: e.naming_until });
        }
        if e.arena < 3 || e.marketing < 2 {
            return Err(AssetsError::NamingNeedsLevels);
        }
        if e.stadium_base_name.is_empty() {
            let name = self.world.club(club).stadium.map(|s| self.world.stadium(s).name.clone()).unwrap_or_default();
            self.club_assets(club).stadium_base_name = name;
        }
        let a = self.club_assets(club);
        a.naming_brand = brands::NAMING_BRANDS[offer.brand].to_owned();
        a.naming_yearly = offer.yearly;
        a.naming_until = year + offer.years;
        self.sync_stadium_name(club);
        if offer.signing_fee > 0 {
            self.world.club_income(club, offer.signing_fee.min(i32::MAX as i64) as i32, 6);
        }
        if e.marketing < 5 {
            self.fans_event(-3, FanEvent::NamingRightsSold { stadium: offer.stadium_name.clone() });
        }
        Ok(())
    }

    /// `Patrimonio.multaRescisaoNaming(ah)`
    pub fn naming_fine(&mut self, club: ClubId) -> i64 {
        self.load_assets();
        let e = self.club_assets(club).clone();
        if e.naming_brand.is_empty() {
            return 0;
        }
        let left = (e.naming_until - year(self.world)).max(1);
        java_round(e.naming_yearly as f64 * left as f64 * 0.30)
    }

    /// `Patrimonio.rescindirNaming(ah)`
    pub fn end_naming(&mut self, club: ClubId) -> Result<(), AssetsError> {
        if self.world.club(club).stadium.is_none() {
            return Err(AssetsError::NoStadium);
        }
        self.load_assets();
        if self.club_assets(club).naming_brand.is_empty() {
            return Err(AssetsError::NoNaming);
        }
        let fine = self.naming_fine(club);
        if self.world.club(club).cash < fine {
            return Err(AssetsError::Cash { cost: fine });
        }
        self.world.club_payment(club, fine.min(i32::MAX as i64) as i32, 8);
        let e = self.club_assets(club);
        e.naming_brand.clear();
        e.naming_yearly = 0;
        e.naming_until = 0;
        self.sync_stadium_name(club);
        Ok(())
    }

    /// `Patrimonio.realizarCampanha(ah, int)`: a season's campaign (1), international tour (2)
    /// or world tour (3).
    pub fn run_campaign(&mut self, club: ClubId, kind: i32) -> Result<(), AssetsError> {
        if !(1..=3).contains(&kind) {
            return Err(AssetsError::BadCampaign);
        }
        self.load_assets();
        let season = self.world.season();
        let e = self.club_assets(club).clone();
        if e.campaign_season == season {
            return Err(AssetsError::CampaignDone { kind: e.campaign_kind });
        }
        let needed = CAMPAIGN_MARKETING[kind as usize];
        if e.marketing < needed {
            return Err(AssetsError::CampaignNeedsMarketing { level: needed });
        }
        let cost = CAMPAIGN_COST[kind as usize];
        if self.world.club(club).cash < cost {
            return Err(AssetsError::Cash { cost });
        }
        self.world.club_payment(club, cost.min(i32::MAX as i64) as i32, 8);
        let a = self.club_assets(club);
        a.campaign_season = season;
        a.campaign_kind = kind;
        match kind {
            1 => self.fans_event(5, FanEvent::Campaign { kind }),
            _ => {
                let (min, max, value, fans) = if kind == 2 { (35 * MI, 85 * MI, 0.06, 8) } else { (75 * MI, 175 * MI, 0.10, 12) };
                let gain = self.tour_return(club, min, max);
                self.world.club_income(club, gain.min(i32::MAX as i64) as i32, 6);
                self.raise_squad_value(club, value);
                self.fans_event(fans, FanEvent::Campaign { kind });
            }
        }
        // Jornal.noticia: the campaign's news is not ported
        Ok(())
    }

    /// `Patrimonio.calcularRetornoTour(ah, Estrutura, long, long)`
    fn tour_return(&mut self, club: ClubId, min: i64, max: i64) -> i64 {
        let reputation = self.world.club(club).reputation.max(1);
        let marketing = self.club_assets(club).marketing;
        let stars = self.world.club(club).squad.iter().filter(|&&p| self.world.player(p).world_class || self.world.player(p).strength >= 82).count();
        let draw = sorte!(self, "Patrimonio").next_double() * 0.15;
        let factor = (reputation as f64 * 0.12 + marketing as f64 * 0.06 + (stars as f64 * 0.03).min(0.25) + draw).min(1.0);
        java_round(min as f64 + (max - min) as f64 * factor)
    }

    /// `Patrimonio.valorizarElenco(ah, double)`: the best players of 70 or more not yet stars
    /// become stars (one, or two for the world tour).
    fn raise_squad_value(&mut self, club: ClubId, share: f64) {
        let mut squad: Vec<PlayerId> = self.world.club(club).squad.clone();
        squad.sort_by(|&a, &b| self.world.player(b).strength.cmp(&self.world.player(a).strength));
        let mut stars = if share >= 0.09 { 2 } else { 1 };
        for p in squad {
            if stars > 0 && !self.world.player(p).star && self.world.player(p).strength >= 70 {
                self.world.player_mut(p).set_star(true);
                stars -= 1;
            }
        }
    }

    /// `Patrimonio.manutencaoMensalTotal(ah)`
    pub fn monthly_upkeep(&mut self, club: ClubId) -> i64 {
        self.load_assets();
        let e = self.club_assets(club).clone();
        Pillar::ALL.iter().map(|&p| upkeep(p)[at(e.level(p))]).sum()
    }
}
