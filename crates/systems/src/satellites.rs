// Port of bfmod.Satelites: multi-club networks, for the player and the AI. A satellite is tied to
// its parent club by a partnership (three seasons, a yearly fee of ~15% of its wages: unlimited
// loans, 25% off its players, 20% of its sales to third parties) or by ownership (free moves both
// ways, cash in and out, its policy and tactic set by the parent). Parent and satellite in the
// same division of a season suspend the moves between them.

use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::mods::{LoanRole, Pillar, SatelliteEvent, SatelliteLink, SatelliteNote, SatellitePolicy, SatelliteTactic, SatellitesState};
use brasfoot_model::rng::java_round;
use brasfoot_model::world::World;

use crate::awards::season::player_key;
use crate::config::Switch;
use crate::economy::{ai_clubs, clubs_by_id, is_human, is_national_team, year};
use crate::loans::{loan_of, season_numbers};
use crate::market::{SQUAD_TARGET, scale, starters_average, value, worst_starter};
use crate::{Mod, sorte};

const MI: i64 = 1_000_000;
/// `DURACAO_PARCERIA`
const PARTNERSHIP_YEARS: i32 = 3;
/// `DESCONTO_PARCERIA`: a partner pays 75% of the market value.
const PARTNER_PRICE: f64 = 0.75;
/// `PARTICIPA_VENDA`
const SELL_ON: f64 = 0.20;
/// `IMPOSTO_RETIRADA`
const WITHDRAWAL_TAX: f64 = 0.15;
/// `FATOR_REVENDA`
const RESALE: f64 = 0.90;
/// `MAX_HISTORICO`
const MAX_HISTORY: usize = 120;

/// Why a club cannot become a satellite (`motivoBloqueio`); the screens word it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SatelliteBlock {
    Invalid,
    Itself,
    HumanOrNationalTeam,
    ParentIsSatellite,
    TargetIsParent,
    SatelliteOfAnother { parent: Option<String> },
    AlreadyLinked,
    AlreadyOwned,
    Reputation { target: i32, parent: i32 },
    GiantNotForSale,
    OwnedByInvestor,
}

/// Why an action of the satellites' screen did not happen; the screens word it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SatelliteError {
    Blocked(SatelliteBlock),
    NotEnoughCash { needed: i64 },
    NoPartnership,
    NotOwned,
    ParentMissing,
    Suspended,
    NotTheParentsPlayer,
    NotTheSatellitesPlayer,
    OnLoan,
    /// The player's loan flag (`F.gl()`) is on; the mod says he is ending his career.
    LoanFlag,
    NotOnLoan,
    ParentSquadTooSmall,
    SatelliteSquadFull,
    ParentSquadFull,
    SatelliteSquadTooSmall,
    AcademyFull { capacity: i32 },
    InvalidAmount,
    WorkingCapital { reserve: i64 },
    /// The parent does not own this satellite (`investirPatrimonio`).
    NotInGroup,
    /// The satellite's pillar is at level 5 already.
    AssetsMaxLevel { pillar: Pillar },
    /// "Caixa da matriz insuficiente: a obra custa {cost} e a matriz tem {cash}."
    ParentCash { cost: i64, cash: i64 },
}

/// How a loanee at a satellite is doing against his role (`statusMinutagem`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinutesStatus {
    pub role: LoanRole,
    pub played: i32,
    pub total: i32,
    pub percent: i64,
    /// Below the role's target by more than 15 points after five games.
    pub below: bool,
}

/// `Satelites.mesmaDivisao(ah, ah)`: both clubs play the same national division this season.
pub fn same_division(world: &World, a: ClubId, b: ClubId) -> bool {
    if a == b || world.club(a).country != world.club(b).country {
        return false;
    }
    for &country in &world.game.league_countries {
        for &league in &world.country(country).divisions {
            if let Some(table) = world.competition(league).league {
                let clubs = &world.stage(table).clubs;
                if clubs.contains(&a) && clubs.contains(&b) {
                    return true;
                }
            }
        }
    }
    false
}

/// `Satelites.taxaParceria(ah)`: ~15% of a year of the satellite's wages, at least 2 million,
/// rounded to 100 thousand.
pub fn partnership_fee(world: &World, satellite: ClubId) -> i64 {
    let yearly = world.club_wages(satellite).max(0) * 12;
    let gross = (2 * MI).max(java_round(yearly as f64 * 0.15));
    java_round(gross as f64 / 100_000.0) * 100_000
}

/// `Satelites.valorElenco(ah)`
pub fn squad_market_value(world: &World, club: ClubId) -> i64 {
    world.club(club).squad.iter().map(|&p| (world.player(p).market_value as i64).max(0)).sum()
}

/// `Satelites.vistoPorSatelite(F)`: a junior of a satellite of the human's club.
pub fn seen_by_satellite(world: &World, config: &crate::config::ModConfig, p: PlayerId) -> bool {
    if !config.on(Switch::Satellites) {
        return false;
    }
    let (Some(club), Some(mine)) = (world.player(p).club, crate::staff::my_club(world)) else { return false };
    let id = world.club(club).id;
    world.mods.satellites.as_ref().and_then(|s| s.link(id)).is_some_and(|l| l.parent == world.club(mine).id)
}

impl Mod<'_> {
    /// The networks' state, loaded on first use in this run (`carregar()`): a save without it
    /// starts with this season.
    fn satellites(&mut self) -> &mut SatellitesState {
        if self.world.mods.satellites.is_none() {
            let season = self.world.season();
            self.world.mods.satellites = Some(SatellitesState { last_season: season, ..Default::default() });
        }
        self.sys.satellites_loaded = true;
        self.world.mods.satellites.as_mut().expect("just made")
    }

    fn note(&mut self, parent: Option<ClubId>, satellite: Option<ClubId>, event: SatelliteEvent) {
        let year = year(self.world);
        let parent = parent.map(|c| self.world.club(c).name.clone());
        let satellite = satellite.map(|c| self.world.club(c).name.clone());
        let history = &mut self.satellites().history;
        history.push(SatelliteNote { year, parent, satellite, event });
        while history.len() > MAX_HISTORY {
            history.remove(0);
        }
    }

    /// `Satelites.tick()` (from `Economia.tick`): the AI's first networks once per save, the new
    /// season once per season.
    pub(crate) fn satellites_tick(&mut self) {
        if !self.sys.config.on(Switch::Satellites) {
            return;
        }
        self.satellites();
        if !self.satellites().ai_started {
            self.satellites().ai_started = true;
            self.expand_ai_networks(true);
        }
        let season = self.world.season();
        if season > self.satellites().last_season {
            self.satellites().last_season = season;
            self.satellites_new_season();
        }
    }

    /// `SatelitesDialog.mostrar`: `Satelites.tick()` and `Base.tick()` before the window.
    pub fn open_satellites(&mut self) {
        self.satellites_tick();
        self.academy_tick();
    }

    /// `Satelites.vinculoDe(ah)`
    pub fn satellite_link(&mut self, satellite: ClubId) -> Option<SatelliteLink> {
        let id = self.world.club(satellite).id;
        self.satellites().link(id).cloned()
    }

    /// `Satelites.matrizDe(ah)`
    pub fn parent_of(&mut self, satellite: ClubId) -> Option<ClubId> {
        let link = self.satellite_link(satellite)?;
        clubs_by_id(self.world).get(&link.parent).copied()
    }

    /// `Satelites.satelitesDe(ah)`
    pub fn satellites_of(&mut self, parent: ClubId) -> Vec<ClubId> {
        let id = self.world.club(parent).id;
        let clubs = clubs_by_id(self.world);
        self.satellites().links.iter().filter(|l| l.parent == id).filter_map(|l| clubs.get(&l.satellite).copied()).collect()
    }

    /// `Satelites.suspenso(ah)`: parent and satellite in the same division this season.
    pub fn suspended(&mut self, satellite: ClubId) -> bool {
        match self.parent_of(satellite) {
            Some(parent) => same_division(self.world, parent, satellite),
            None => false,
        }
    }

    /// `Satelites.politicaDe(ah)`: the policy of an owned satellite.
    pub fn satellite_policy(&mut self, club: ClubId) -> Option<SatellitePolicy> {
        self.satellite_link(club).filter(|l| l.owned).map(|l| l.policy)
    }

    /// `Satelites.precoCompra(ah)`: the squad's value, 1.2 times the press estimate of the cash,
    /// a premium by reputation and 8 million per level of infrastructure, rounded to 500
    /// thousand; at least 8 million.
    pub fn buy_price(&mut self, satellite: ClubId) -> i64 {
        let squad = squad_market_value(self.world, satellite);
        let cash = self.estimate(satellite).max(0);
        let reputation = self.world.club(satellite).reputation.clamp(0, 6);
        let premium = match reputation {
            0 => 8 * MI,
            1 => 15 * MI,
            2 => 28 * MI,
            3 => 60 * MI,
            4 => 125 * MI,
            _ => 260 * MI,
        };
        let levels = self.pillar_level(satellite, Pillar::Arena)
            + self.pillar_level(satellite, Pillar::TrainingCentre)
            + self.pillar_level(satellite, Pillar::Academy)
            + self.pillar_level(satellite, Pillar::Marketing);
        let total = squad + java_round(cash as f64 * 1.2) + premium + levels as i64 * 8 * MI;
        (8 * MI).max(java_round(total as f64 / 500_000.0) * 500_000)
    }

    /// `Satelites.valorRevenda(ah)`
    pub fn resale_value(&mut self, satellite: ClubId) -> i64 {
        java_round(self.buy_price(satellite) as f64 * RESALE)
    }

    /// `Satelites.motivoBloqueio(ah, ah, boolean)`: why `target` cannot become a satellite of
    /// `parent`, if it cannot.
    pub fn block_reason(&mut self, parent: ClubId, target: ClubId, buy: bool) -> Option<SatelliteBlock> {
        self.satellites();
        if parent == target {
            return Some(SatelliteBlock::Itself);
        }
        if is_national_team(self.world, target) || self.world.club(target).human || is_human(self.world, target) {
            return Some(SatelliteBlock::HumanOrNationalTeam);
        }
        let parent_id = self.world.club(parent).id;
        if self.satellites().link(parent_id).is_some() {
            return Some(SatelliteBlock::ParentIsSatellite);
        }
        if !self.satellites_of(target).is_empty() {
            return Some(SatelliteBlock::TargetIsParent);
        }
        let target_id = self.world.club(target).id;
        if let Some(existing) = self.satellites().link(target_id).cloned() {
            if existing.parent != parent_id {
                let name = clubs_by_id(self.world).get(&existing.parent).map(|&c| self.world.club(c).name.clone());
                return Some(SatelliteBlock::SatelliteOfAnother { parent: name });
            }
            if !buy {
                return Some(SatelliteBlock::AlreadyLinked);
            }
            if existing.owned {
                return Some(SatelliteBlock::AlreadyOwned);
            }
        }
        let (rp, rt) = (self.world.club(parent).reputation, self.world.club(target).reputation);
        if rt >= rp && !(rp <= 2 && rt <= 1) {
            return Some(SatelliteBlock::Reputation { target: rt, parent: rp });
        }
        if buy {
            if rt >= 5 {
                return Some(SatelliteBlock::GiantNotForSale);
            }
            if self.has_owner(target) && self.fortune(target) >= 0 {
                return Some(SatelliteBlock::OwnedByInvestor);
            }
        }
        None
    }

    /// `Satelites.cobrar(ah, long, int)`: a human club pays in the game's books, an AI club from
    /// its fortune (and the game's cash follows it).
    fn charge(&mut self, club: ClubId, amount: i64, kind: i32) {
        if amount <= 0 {
            return;
        }
        if is_human(self.world, club) {
            self.world.club_payment(club, amount.min(i32::MAX as i64) as i32, kind);
        } else {
            self.spend(club, amount);
            let fortune = self.fortune(club);
            self.world.club_mut(club).cash = fortune;
        }
    }

    /// `Satelites.creditar(ah, long, int)`
    fn credit(&mut self, club: ClubId, amount: i64, kind: i32) {
        if amount <= 0 {
            return;
        }
        if is_human(self.world, club) {
            self.world.club_income(club, amount.min(i32::MAX as i64) as i32, kind);
        } else {
            self.spend(club, -amount);
            let fortune = self.fortune(club);
            self.world.club_mut(club).cash = fortune;
        }
    }

    /// `Satelites.firmarParceria(ah, ah)`: three seasons, the first fee paid now.
    pub fn sign_partnership(&mut self, parent: ClubId, satellite: ClubId) -> Result<(), SatelliteError> {
        if let Some(b) = self.block_reason(parent, satellite, false) {
            return Err(SatelliteError::Blocked(b));
        }
        let fee = partnership_fee(self.world, satellite);
        if self.fortune(parent) < fee {
            return Err(SatelliteError::NotEnoughCash { needed: fee });
        }
        self.charge(parent, fee, 8);
        self.credit(satellite, fee, 9);
        let since = year(self.world);
        let link = SatelliteLink {
            satellite: self.world.club(satellite).id,
            parent: self.world.club(parent).id,
            owned: false,
            policy: SatellitePolicy::YoungPlayers,
            yearly_fee: fee,
            since,
            until: since + PARTNERSHIP_YEARS,
            price_paid: 0,
        };
        self.satellites().put(link);
        self.note(Some(parent), Some(satellite), SatelliteEvent::Partnership { fee });
        Ok(())
    }

    /// `Satelites.renovarParceria(ah)`
    pub fn renew_partnership(&mut self, satellite: ClubId) -> Result<(), SatelliteError> {
        let link = self.satellite_link(satellite).filter(|l| !l.owned).ok_or(SatelliteError::NoPartnership)?;
        let parent = self.parent_of(satellite).ok_or(SatelliteError::ParentMissing)?;
        let fee = partnership_fee(self.world, satellite);
        let until = link.until.max(year(self.world)) + PARTNERSHIP_YEARS;
        let l = self.satellites().link_mut(link.satellite).expect("found above");
        l.yearly_fee = fee;
        l.until = until;
        self.note(Some(parent), Some(satellite), SatelliteEvent::Renewal { until, fee });
        Ok(())
    }

    /// `Satelites.romperParceria(ah)`: a fee of fine before the end.
    pub fn end_partnership(&mut self, satellite: ClubId) -> Result<(), SatelliteError> {
        let link = self.satellite_link(satellite).filter(|l| !l.owned).ok_or(SatelliteError::NoPartnership)?;
        let parent = self.parent_of(satellite);
        let fine = if link.until > year(self.world) { link.yearly_fee } else { 0 };
        if let Some(parent) = parent
            && fine > 0
        {
            if self.fortune(parent) < fine {
                return Err(SatelliteError::NotEnoughCash { needed: fine });
            }
            self.charge(parent, fine, 8);
            self.credit(satellite, fine, 9);
        }
        self.satellites().remove(link.satellite);
        self.note(parent, Some(satellite), SatelliteEvent::Termination { fine });
        Ok(())
    }

    /// `Satelites.comprarClube(ah, ah)`
    pub fn buy_satellite(&mut self, parent: ClubId, satellite: ClubId) -> Result<(), SatelliteError> {
        if let Some(b) = self.block_reason(parent, satellite, true) {
            return Err(SatelliteError::Blocked(b));
        }
        let price = self.buy_price(satellite);
        if self.fortune(parent) < price {
            return Err(SatelliteError::NotEnoughCash { needed: price });
        }
        self.charge(parent, price, 8);
        let id = self.world.club(satellite).id;
        let mut link = self.satellites().link(id).cloned().unwrap_or_default();
        link.satellite = id;
        link.parent = self.world.club(parent).id;
        link.owned = true;
        link.policy = SatellitePolicy::YoungPlayers;
        link.yearly_fee = 0;
        link.until = 0;
        link.since = year(self.world);
        link.price_paid = price;
        self.satellites().put(link);
        self.note(Some(parent), Some(satellite), SatelliteEvent::Acquisition { price });
        // Jornal.noticia: the purchase's news is not ported
        Ok(())
    }

    /// `Satelites.venderClube(ah)`: 90% of the price.
    pub fn sell_satellite(&mut self, satellite: ClubId) -> Result<(), SatelliteError> {
        let link = self.satellite_link(satellite).filter(|l| l.owned).ok_or(SatelliteError::NotOwned)?;
        let parent = self.parent_of(satellite);
        let price = self.resale_value(satellite);
        if let Some(parent) = parent {
            self.credit(parent, price, 9);
        }
        self.satellites().remove(link.satellite);
        self.note(parent, Some(satellite), SatelliteEvent::ClubSale { price });
        Ok(())
    }

    /// `Satelites.definirPolitica(ah, Politica)`
    pub fn set_satellite_policy(&mut self, satellite: ClubId, policy: SatellitePolicy) {
        let id = self.world.club(satellite).id;
        if let Some(l) = self.satellites().link_mut(id)
            && l.owned
        {
            l.policy = policy;
        }
    }

    /// `Satelites.injetarCaixa(ah, long)`
    pub fn inject_cash(&mut self, satellite: ClubId, amount: i64) -> Result<(), SatelliteError> {
        if amount <= 0 {
            return Err(SatelliteError::InvalidAmount);
        }
        self.satellite_link(satellite).filter(|l| l.owned).ok_or(SatelliteError::NotOwned)?;
        let parent = self.parent_of(satellite).ok_or(SatelliteError::NotEnoughCash { needed: amount })?;
        if self.fortune(parent) < amount {
            return Err(SatelliteError::NotEnoughCash { needed: amount });
        }
        self.charge(parent, amount, 8);
        self.credit(satellite, amount, 9);
        self.note(Some(parent), Some(satellite), SatelliteEvent::CashInjection { amount });
        Ok(())
    }

    /// `Satelites.retirarLucro(ah, long)`: the satellite keeps 5 million; 15% taxes.
    pub fn withdraw_profit(&mut self, satellite: ClubId, amount: i64) -> Result<(), SatelliteError> {
        if amount <= 0 {
            return Err(SatelliteError::InvalidAmount);
        }
        self.satellite_link(satellite).filter(|l| l.owned).ok_or(SatelliteError::NotOwned)?;
        let parent = self.parent_of(satellite);
        let reserve = 5 * MI;
        if self.fortune(satellite) - amount < reserve {
            return Err(SatelliteError::WorkingCapital { reserve });
        }
        let net = java_round(amount as f64 * (1.0 - WITHDRAWAL_TAX));
        self.charge(satellite, amount, 8);
        if let Some(parent) = parent {
            self.credit(parent, net, 9);
        }
        self.note(parent, Some(satellite), SatelliteEvent::ProfitWithdrawal { amount, net });
        Ok(())
    }

    /// `Satelites.taticaDe(ah)`: an owned satellite's tactic (kept once asked).
    pub fn satellite_tactic(&mut self, satellite: ClubId) -> SatelliteTactic {
        let id = self.world.club(satellite).id;
        if let Some(&(_, t)) = self.satellites().tactics.iter().find(|(s, _)| *s == id) {
            return t;
        }
        let t = SatelliteTactic::default();
        if self.satellite_link(satellite).is_some_and(|l| l.owned) {
            self.satellites().tactics.push((id, t));
        }
        t
    }

    /// `Satelites.definirTatica(ah, boolean, int, int, int)`
    pub fn set_satellite_tactic(&mut self, satellite: ClubId, tactic: SatelliteTactic) {
        if !self.satellite_link(satellite).is_some_and(|l| l.owned) {
            return;
        }
        self.satellite_tactic(satellite);
        let t = SatelliteTactic { mirror: tactic.mirror, style: tactic.style.clamp(0, 2), marking: tactic.marking.clamp(0, 2), side: tactic.side.clamp(0, 1) };
        let id = self.world.club(satellite).id;
        if let Some(e) = self.satellites().tactics.iter_mut().find(|(s, _)| *s == id) {
            e.1 = t;
        }
        self.apply_satellite_tactic(satellite);
    }

    /// `Satelites.diretrizTatica(ah)`: the style, marking and side an owned satellite plays: its
    /// parent's when mirroring.
    pub fn satellite_directive(&mut self, satellite: ClubId) -> Option<[i32; 3]> {
        if !self.sys.config.on(Switch::Satellites) {
            return None;
        }
        let link = self.satellite_link(satellite).filter(|l| l.owned)?;
        let id = link.satellite;
        let t = self.satellites().tactics.iter().find(|(s, _)| *s == id).map(|&(_, t)| t).unwrap_or_default();
        if t.mirror
            && let Some(parent) = self.parent_of(satellite)
        {
            let k = &self.world.club(parent).tactics;
            if k.len() >= 4 {
                return Some([k[1].clamp(0, 2), k[2].clamp(0, 2), k[3].clamp(0, 1)]);
            }
        }
        Some([t.style, t.marking, t.side])
    }

    /// `Satelites.aplicarTatica(ah)`
    pub fn apply_satellite_tactic(&mut self, satellite: ClubId) {
        if let Some(d) = self.satellite_directive(satellite) {
            let k = &mut self.world.club_mut(satellite).tactics;
            if k.len() >= 4 {
                k[1] = d[0];
                k[2] = d[1];
                k[3] = d[2];
            }
        }
    }

    /// `Satelites.papelDe(F)`
    pub fn loan_role(&mut self, p: PlayerId) -> LoanRole {
        let key = player_key(self.world, p);
        self.satellites().roles.iter().find(|(k, _)| *k == key).map(|&(_, r)| r).unwrap_or_default()
    }

    /// `Satelites.definirPapel(F, PapelEmprestimo)`
    pub fn set_loan_role(&mut self, p: PlayerId, role: Option<LoanRole>) {
        let key = player_key(self.world, p);
        let roles = &mut self.satellites().roles;
        match (roles.iter_mut().find(|(k, _)| *k == key), role) {
            (Some(e), Some(r)) => e.1 = r,
            (None, Some(r)) => roles.push((key, r)),
            (_, None) => roles.retain(|(k, _)| *k != key),
        }
    }

    /// `Satelites.emprestarParaSatelite(F, ah, PapelEmprestimo)`
    pub fn lend_to_satellite(&mut self, p: PlayerId, satellite: ClubId, role: LoanRole) -> Result<(), SatelliteError> {
        let parent = self.parent_of(satellite).ok_or(SatelliteError::ParentMissing)?;
        if self.world.player(p).club != Some(parent) {
            return Err(SatelliteError::NotTheParentsPlayer);
        }
        if self.suspended(satellite) {
            return Err(SatelliteError::Suspended);
        }
        if self.world.player(p).on_loan {
            return Err(SatelliteError::LoanFlag);
        }
        if loan_of(self.world, p).is_some() {
            return Err(SatelliteError::OnLoan);
        }
        if self.world.club(parent).squad.len() <= 16 {
            return Err(SatelliteError::ParentSquadTooSmall);
        }
        if self.world.club(satellite).squad.len() >= 34 {
            return Err(SatelliteError::SatelliteSquadFull);
        }
        self.lend(p, satellite);
        self.set_loan_role(p, Some(role));
        let (player, age, strength) = (self.world.player(p).name.clone(), self.world.player(p).age, self.world.player(p).strength);
        self.note(Some(parent), Some(satellite), SatelliteEvent::Loan { player, age, strength, role });
        Ok(())
    }

    /// `Satelites.custoContratarDoSatelite(F, ah)`: nothing if owned, 75% of the market value
    /// for a partner.
    pub fn satellite_signing_cost(&mut self, p: PlayerId, satellite: ClubId) -> i64 {
        match self.satellite_link(satellite) {
            None => self.world.player(p).market_value as i64,
            Some(l) if l.owned => 0,
            Some(_) => java_round(self.world.player(p).market_value as f64 * PARTNER_PRICE),
        }
    }

    /// `Satelites.contratarDoSatelite(F, ah)`
    pub fn sign_from_satellite(&mut self, p: PlayerId, satellite: ClubId) -> Result<(), SatelliteError> {
        let parent = self.parent_of(satellite).ok_or(SatelliteError::ParentMissing)?;
        if self.world.player(p).club != Some(satellite) {
            return Err(SatelliteError::NotTheSatellitesPlayer);
        }
        if self.suspended(satellite) {
            return Err(SatelliteError::Suspended);
        }
        if loan_of(self.world, p).is_some() {
            return Err(SatelliteError::OnLoan);
        }
        if self.world.club(parent).squad.len() >= 35 {
            return Err(SatelliteError::ParentSquadFull);
        }
        if self.world.club(satellite).squad.len() <= 16 {
            return Err(SatelliteError::SatelliteSquadTooSmall);
        }
        let cost = self.satellite_signing_cost(p, satellite);
        if cost > 0 && self.fortune(parent) < cost {
            return Err(SatelliteError::NotEnoughCash { needed: cost });
        }
        self.set_loan_role(p, None);
        self.transfer_now(p, parent, cost.min(i32::MAX as i64) as i32);
        self.account_transfers();
        let (player, strength) = (self.world.player(p).name.clone(), self.world.player(p).strength);
        self.note(Some(parent), Some(satellite), SatelliteEvent::ToParent { player, strength, cost });
        Ok(())
    }

    /// `Satelites.enviarDefinitivoParaSatelite(F, ah)`
    pub fn send_to_satellite(&mut self, p: PlayerId, satellite: ClubId) -> Result<(), SatelliteError> {
        self.satellite_link(satellite).filter(|l| l.owned).ok_or(SatelliteError::NotOwned)?;
        let parent = self.parent_of(satellite).ok_or(SatelliteError::ParentMissing)?;
        if self.world.player(p).club != Some(parent) {
            return Err(SatelliteError::NotTheParentsPlayer);
        }
        if self.suspended(satellite) {
            return Err(SatelliteError::Suspended);
        }
        if loan_of(self.world, p).is_some() {
            return Err(SatelliteError::OnLoan);
        }
        if self.world.club(parent).squad.len() <= 16 {
            return Err(SatelliteError::ParentSquadTooSmall);
        }
        if self.world.club(satellite).squad.len() >= 34 {
            return Err(SatelliteError::SatelliteSquadFull);
        }
        self.set_loan_role(p, None);
        self.transfer_now(p, satellite, 0);
        self.account_transfers();
        let (player, strength) = (self.world.player(p).name.clone(), self.world.player(p).strength);
        self.note(Some(parent), Some(satellite), SatelliteEvent::Definitive { player, strength });
        Ok(())
    }

    /// `Satelites.investirPatrimonio(ah, ah, Pilar)`: the parent pays the next level of an owned
    /// satellite's infrastructure.
    pub fn invest_satellite_assets(&mut self, parent: ClubId, satellite: ClubId, pillar: Pillar) -> Result<(), SatelliteError> {
        let link = self.satellite_link(satellite).filter(|l| l.owned).ok_or(SatelliteError::NotOwned)?;
        if link.parent != self.world.club(parent).id {
            return Err(SatelliteError::NotInGroup);
        }
        match self.upgrade_financed_by(parent, satellite, pillar) {
            Ok(()) => {}
            Err(crate::assets::AssetsError::Cash { cost }) => return Err(SatelliteError::ParentCash { cost, cash: self.world.club(parent).cash }),
            Err(_) => return Err(SatelliteError::AssetsMaxLevel { pillar }),
        }
        let level = self.pillar_level(satellite, pillar);
        self.note(Some(parent), Some(satellite), SatelliteEvent::Assets { pillar, level });
        Ok(())
    }

    /// `Satelites.chamarDeVolta(F)`: a loanee goes back to his club at once.
    pub fn recall(&mut self, p: PlayerId) -> Result<(), SatelliteError> {
        let i = loan_of(self.world, p).ok_or(SatelliteError::NotOnLoan)?;
        let parent = self.world.game.loans[i].owner.ok_or(SatelliteError::NotOnLoan)?;
        let satellite = self.world.player(p).club;
        if self.world.club(parent).squad.len() >= 35 {
            return Err(SatelliteError::ParentSquadFull);
        }
        // components.t.tN(): the player goes back (a human owner has room, checked above), then
        // c.a.SR.d(F) drops the loan
        brasfoot_simulation::Sim { world: self.world, runtime: self.runtime, hooks: &mut brasfoot_engine::hooks::NoSystems::default(), human: None }.return_from_loan(p, parent);
        if let Some(i) = loan_of(self.world, p) {
            self.world.game.loans.remove(i);
        }
        self.set_loan_role(p, None);
        let player = self.world.player(p).name.clone();
        self.note(Some(parent), satellite, SatelliteEvent::Return { player });
        Ok(())
    }

    /// `Satelites.puxarJuniorDoSatelite(F, ah, boolean)`: a junior of the satellite's academy
    /// joins the parent's academy or first team.
    pub fn pull_youth(&mut self, p: PlayerId, satellite: ClubId, professional: bool) -> Result<(), SatelliteError> {
        let link = self.satellite_link(satellite).ok_or(SatelliteError::NoPartnership)?;
        let parent = clubs_by_id(self.world).get(&link.parent).copied();
        let Some(parent) = parent.filter(|_| self.world.player(p).club == Some(satellite)) else {
            return Err(SatelliteError::NotTheSatellitesPlayer);
        };
        if self.suspended(satellite) {
            return Err(SatelliteError::Suspended);
        }
        if professional {
            if self.world.club(parent).squad.len() >= 35 {
                return Err(SatelliteError::ParentSquadFull);
            }
        } else {
            let capacity = 10 + 2 * self.pillar_level(parent, Pillar::Academy);
            if self.world.club(parent).youth_squad.len() as i32 >= capacity {
                return Err(SatelliteError::AcademyFull { capacity });
            }
        }
        let cost = self.satellite_signing_cost(p, satellite);
        if cost > 0 && self.fortune(parent) < cost {
            return Err(SatelliteError::NotEnoughCash { needed: cost });
        }
        let fee = cost.min(i32::MAX as i64) as i32;
        let mut sim = brasfoot_simulation::Sim { world: self.world, runtime: self.runtime, hooks: &mut brasfoot_engine::hooks::NoSystems::default(), human: None };
        if professional {
            // K.a(false, jr, satellite), Base.aoPromover, then F.a(parent, cost, false, false, false)
            sim.promote_from_youth_list(p, satellite);
            self.on_promoted(p);
            self.transfer_now(p, parent, fee);
        } else {
            sim.transfer_youth(p, parent, fee);
        }
        self.account_transfers();
        let (player, age) = (self.world.player(p).name.clone(), self.world.player(p).age);
        self.note(Some(parent), Some(satellite), SatelliteEvent::YouthPull { player, age, professional, cost });
        Ok(())
    }

    /// `Satelites.statusMinutagem(F)`: the loanee's games against his role this season.
    pub fn minutes_status(&mut self, p: PlayerId) -> Option<MinutesStatus> {
        let club = self.world.player(p).club?;
        let role = self.loan_role(p);
        let total = crate::performance::played_matches(self.world, club).len() as i32;
        let played = season_numbers(self.world, p).0;
        let percent = if total > 0 { java_round(played as f64 * 100.0 / total as f64) } else { 100 };
        let below = total >= 5 && role != LoanRole::Reserve && percent < java_round((role.target() - 0.15) * 100.0);
        Some(MinutesStatus { role, played, total, percent, below })
    }

    /// `Satelites.cumpreMinutagem(F)`
    pub fn meets_minutes(&mut self, p: PlayerId) -> bool {
        let Some(club) = self.world.player(p).club else { return true };
        let role = self.loan_role(p);
        if role == LoanRole::Reserve {
            return true;
        }
        let total = crate::performance::played_matches(self.world, club).len() as i32;
        if total < 5 {
            return true;
        }
        let played = season_numbers(self.world, p).0;
        played as f64 / total as f64 >= role.target() - 0.15
    }

    /// `F.a(ah, int, false, false, false)`
    fn transfer_now(&mut self, p: PlayerId, to: ClubId, fee: i32) {
        brasfoot_simulation::Sim { world: self.world, runtime: self.runtime, hooks: &mut brasfoot_engine::hooks::NoSystems::default(), human: None }.transfer(p, to, fee);
    }

    /// `Satelites.fatorEvolucaoEmprestado(F)`: a loanee of the parent at an owned satellite
    /// develops 25% faster, 40% with the mirrored tactic.
    pub fn loanee_development_factor(&mut self, p: PlayerId) -> f64 {
        if !self.sys.config.on(Switch::Satellites) {
            return 1.0;
        }
        let Some(club) = self.world.player(p).club else { return 1.0 };
        let Some(link) = self.satellite_link(club).filter(|l| l.owned) else { return 1.0 };
        if let Some(i) = loan_of(self.world, p)
            && let Some(owner) = self.world.game.loans[i].owner
            && self.world.club(owner).id == link.parent
        {
            let mirror = self.satellite_tactic(club).mirror;
            return if mirror { 1.40 } else { 1.25 };
        }
        1.0
    }

    /// `Satelites.processarVendaTerceiros(ah, ah, long, String)` (from the transfers' accounts):
    /// a partner satellite selling to anyone but its parent pays it 20%.
    pub(crate) fn satellite_sell_on(&mut self, from: ClubId, to: Option<ClubId>, amount: i64, player: Option<String>) {
        if amount <= 0 || !self.sys.satellites_loaded || !self.sys.config.on(Switch::Satellites) {
            return;
        }
        let Some(link) = self.satellite_link(from).filter(|l| !l.owned) else { return };
        if to.is_some_and(|t| self.world.club(t).id == link.parent) {
            return;
        }
        let Some(parent) = clubs_by_id(self.world).get(&link.parent).copied() else { return };
        let share = java_round(amount as f64 * SELL_ON);
        if share <= 0 {
            return;
        }
        self.charge(from, share, 8);
        self.credit(parent, share, 9);
        let buyer = to.map(|t| self.world.club(t).name.clone());
        self.note(Some(parent), Some(from), SatelliteEvent::SellOn { amount: share, player, buyer });
        // Jornal.noticia: the human parent's news is not ported
    }

    /// `Satelites.novaTemporada()`
    fn satellites_new_season(&mut self) {
        let clubs = clubs_by_id(self.world);
        let year = year(self.world);
        // 1. the loanees playing at the owned satellites grow at the season's end
        for i in 0..self.world.game.loans.len() {
            let loan = self.world.game.loans[i].clone();
            let (Some(p), Some(owner)) = (loan.player, loan.owner) else { continue };
            let Some(club) = self.world.player(p).club else { continue };
            let Some(link) = self.satellite_link(club) else { continue };
            if link.owned && link.parent == self.world.club(owner).id && self.world.player(p).age <= 24 {
                let (games, _, _) = season_numbers(self.world, p);
                if games >= 10 {
                    let mirror = self.satellite_tactic(club).mirror;
                    let key = player_key(self.world, p);
                    self.highlight_bonus(&key, if games >= 20 { 1 } else { 0 }, if mirror { 0.45 } else { 0.35 });
                }
            }
        }
        // 2. fees, expirations
        let mut remove = Vec::new();
        for link in self.satellites().links.clone() {
            let (Some(&satellite), Some(&parent)) = (clubs.get(&link.satellite), clubs.get(&link.parent)) else {
                remove.push(link.satellite);
                continue;
            };
            if !link.owned {
                let fee = partnership_fee(self.world, satellite);
                if let Some(l) = self.satellites().link_mut(link.satellite) {
                    l.yearly_fee = fee;
                }
                if year >= link.until {
                    if is_human(self.world, parent) {
                        if self.fortune(parent) >= fee {
                            self.charge(parent, fee, 8);
                            self.credit(satellite, fee, 9);
                            if let Some(l) = self.satellites().link_mut(link.satellite) {
                                l.until = year + PARTNERSHIP_YEARS;
                            }
                        } else {
                            remove.push(link.satellite);
                        }
                    } else if self.fortune(parent) >= (80 * MI).max(fee * 6) {
                        self.charge(parent, fee, 8);
                        self.credit(satellite, fee, 9);
                        if let Some(l) = self.satellites().link_mut(link.satellite) {
                            l.until = year + PARTNERSHIP_YEARS;
                        }
                    } else {
                        remove.push(link.satellite);
                    }
                } else if self.fortune(parent) >= fee {
                    self.charge(parent, fee, 8);
                    self.credit(satellite, fee, 9);
                } else if !is_human(self.world, parent) {
                    remove.push(link.satellite);
                }
            } else if !is_human(self.world, parent) {
                // an AI parent rescues its owned satellite in crisis, or takes the profit
                let cash = self.fortune(satellite);
                if cash < 0 && self.fortune(parent) >= cash.abs() * 3 {
                    let rescue = cash.abs() + 10 * MI;
                    self.charge(parent, rescue, 8);
                    self.credit(satellite, rescue, 9);
                } else if link.policy == SatellitePolicy::Profit && cash >= 45 * MI {
                    let out = java_round((cash - 20 * MI) as f64 * 0.5);
                    let net = java_round(out as f64 * (1.0 - WITHDRAWAL_TAX));
                    self.charge(satellite, out, 8);
                    self.credit(parent, net, 9);
                }
            }
            // Jornal.noticia: the suspension's news to a human parent is not ported
        }
        for id in remove {
            self.satellites().remove(id);
        }
        // 3. the AI's networks grow
        self.expand_ai_networks(false);
    }

    /// `Satelites.expandirRedesIA(boolean)`: rich AI clubs of reputation 4 or more (220 million,
    /// or an owner) take smaller clubs: by purchase when very rich, else by partnership; six
    /// links at the start, at most two a season (each parent with a chance of 40%).
    fn expand_ai_networks(&mut self, first: bool) {
        let mut parents = Vec::new();
        for t in ai_clubs(self.world) {
            let id = self.world.club(t).id;
            if self.world.club(t).reputation >= 4 && self.satellites().link(id).is_none() {
                let fortune = self.fortune(t);
                if fortune >= 220 * MI || self.has_owner(t) {
                    parents.push(t);
                }
            }
        }
        sorte!(self, "Satelites").shuffle(&mut parents);
        let limit = if first { 6 } else { 2 };
        let mut made = 0;
        for parent in parents {
            if made >= limit {
                break;
            }
            let current = self.satellites_of(parent).len();
            let wanted = if self.has_owner(parent) || self.fortune(parent) >= 600 * MI { 3 } else { 2 };
            if current >= wanted {
                continue;
            }
            if !first && sorte!(self, "Satelites").next_int_bounded(100) >= 40 {
                continue;
            }
            let buy = (self.has_owner(parent) && self.fortune(parent) >= 380 * MI) || self.fortune(parent) >= 550 * MI;
            let mut candidates = Vec::new();
            for target in ai_clubs(self.world) {
                let r = self.world.club(target).reputation;
                if (1..=3).contains(&r) && !same_division(self.world, parent, target) && self.block_reason(parent, target, buy).is_none() {
                    candidates.push(target);
                }
            }
            if candidates.is_empty() {
                continue;
            }
            sorte!(self, "Satelites").shuffle(&mut candidates);
            candidates.sort_by(|&a, &b| self.world.club(b).reputation.cmp(&self.world.club(a).reputation));
            let n = candidates.len().min(8) as i32;
            let chosen = candidates[sorte!(self, "Satelites").next_int_bounded(n) as usize];
            if buy {
                let price = self.buy_price(chosen);
                if self.fortune(parent) as f64 >= price as f64 * 2.2 {
                    self.charge(parent, price, 8);
                    let policy = match sorte!(self, "Satelites").next_int_bounded(3) {
                        0 => SatellitePolicy::YoungPlayers,
                        1 => SatellitePolicy::Promotion,
                        _ => SatellitePolicy::Profit,
                    };
                    let link = SatelliteLink {
                        satellite: self.world.club(chosen).id,
                        parent: self.world.club(parent).id,
                        owned: true,
                        policy,
                        since: year(self.world),
                        price_paid: price,
                        ..Default::default()
                    };
                    self.satellites().put(link);
                    self.note(Some(parent), Some(chosen), SatelliteEvent::AiAcquisition { price, policy });
                    // Jornal.noticia (after the start): the purchase's news is not ported
                    made += 1;
                    continue;
                }
            }
            let fee = partnership_fee(self.world, chosen);
            if self.fortune(parent) >= fee * 8 {
                self.charge(parent, fee, 8);
                self.credit(chosen, fee, 9);
                let since = year(self.world);
                let link = SatelliteLink {
                    satellite: self.world.club(chosen).id,
                    parent: self.world.club(parent).id,
                    owned: false,
                    policy: SatellitePolicy::YoungPlayers,
                    yearly_fee: fee,
                    since,
                    until: since + PARTNERSHIP_YEARS,
                    price_paid: 0,
                };
                self.satellites().put(link);
                self.note(Some(parent), Some(chosen), SatelliteEvent::AiPartnership { fee });
                made += 1;
            }
        }
    }

    /// `Satelites.janelaIA(Set, int)`: in a window, each AI parent lends up to two benched
    /// youngsters (23 or under) to its satellite, and takes the satellite's standout if he would
    /// start for it.
    pub(crate) fn satellites_window(&mut self) {
        if !self.sys.config.on(Switch::Satellites) {
            return;
        }
        if self.satellites().links.is_empty() {
            return;
        }
        let clubs = clubs_by_id(self.world);
        for link in self.satellites().links.clone() {
            let (Some(&satellite), Some(&parent)) = (clubs.get(&link.satellite), clubs.get(&link.parent)) else { continue };
            if is_human(self.world, parent) || self.suspended(satellite) {
                continue;
            }
            // A. up to two benched youngsters on loan
            if self.world.club(parent).squad.len() > 21 && self.world.club(satellite).squad.len() < 31 {
                let world = &*self.world;
                let mut squad = world.club(parent).squad.clone();
                squad.sort_by(|&a, &b| value(world, b).cmp(&value(world, a)));
                let mut lent = 0;
                for &young in squad.iter().skip(16) {
                    if lent >= 2 {
                        break;
                    }
                    let y = self.world.player(young);
                    if y.age <= 23 && !y.on_loan && !self.sys.traded.contains(&young) && loan_of(self.world, young).is_none() {
                        let position = y.position as usize;
                        if value(self.world, young) as f64 >= starters_average(self.world, satellite, position) - scale(self.world, 4) as f64 {
                            self.lend(young, satellite);
                            self.sys.traded.insert(young);
                            lent += 1;
                            let (player, age) = (self.world.player(young).name.clone(), self.world.player(young).age);
                            self.note(Some(parent), Some(satellite), SatelliteEvent::AiLoan { player, age: Some(age) });
                        }
                    }
                }
            }
            // B. the satellite's standout to the parent
            if self.world.club(satellite).squad.len() > 19 && self.world.club(parent).squad.len() < SQUAD_TARGET + 1 {
                let mut standout: Option<PlayerId> = None;
                for &p in &self.world.club(satellite).squad.clone() {
                    let x = self.world.player(p);
                    let position = x.position;
                    if !(0..5).contains(&position) || x.age > 27 || x.on_loan || self.sys.traded.contains(&p) || loan_of(self.world, p).is_some() {
                        continue;
                    }
                    let v = value(self.world, p);
                    let worst = worst_starter(self.world, parent, position as usize);
                    if v >= worst + scale(self.world, 2) && x.strength >= 70 && standout.is_none_or(|s| v > value(self.world, s)) {
                        standout = Some(p);
                    }
                }
                if let Some(s) = standout {
                    let cost = if link.owned { 0 } else { java_round(self.world.player(s).market_value as f64 * PARTNER_PRICE) };
                    if self.fortune(parent) >= cost * 2 {
                        self.transfer_now(s, parent, cost.min(i32::MAX as i64) as i32);
                        self.account_transfers();
                        self.sys.traded.insert(s);
                        let (player, strength) = (self.world.player(s).name.clone(), self.world.player(s).strength);
                        self.note(Some(parent), Some(satellite), SatelliteEvent::AiPromotion { player, strength, cost });
                    }
                }
            }
        }
    }

    /// `Satelites.absorverExcedente(ah, F, Set)` (from `Mercado.abrirVaga`): a surplus player goes
    /// to a satellite of the club with room where he would be useful: on loan if young, free to
    /// an owned one otherwise.
    pub(crate) fn absorb_surplus(&mut self, parent: ClubId, p: PlayerId) -> bool {
        if !self.sys.config.on(Switch::Satellites) {
            return false;
        }
        for satellite in self.satellites_of(parent) {
            if self.suspended(satellite) || self.world.club(satellite).squad.len() >= SQUAD_TARGET - 1 {
                continue;
            }
            let position = self.world.player(p).position as usize;
            if (value(self.world, p) as f64) < starters_average(self.world, satellite, position) - scale(self.world, 6) as f64 {
                continue;
            }
            let Some(link) = self.satellite_link(satellite) else { continue };
            if self.world.player(p).age <= 23 && loan_of(self.world, p).is_none() {
                self.lend(p, satellite);
                self.sys.traded.insert(p);
                let player = self.world.player(p).name.clone();
                self.note(Some(parent), Some(satellite), SatelliteEvent::AiLoan { player, age: None });
                return true;
            }
            if link.owned {
                self.transfer_now(p, satellite, 0);
                self.account_transfers();
                self.sys.traded.insert(p);
                let player = self.world.player(p).name.clone();
                self.note(Some(parent), Some(satellite), SatelliteEvent::AiCession { player });
                return true;
            }
        }
        false
    }

    /// `Satelites.aoIniciarPartida(I)`: at kick-off, the owned satellites play their directed
    /// tactic, and the parent's loanees with a role start in place of a player of their position
    /// (a regular one three times in four, and not one playing badly half the time).
    pub(crate) fn satellites_kickoff(&mut self, m: brasfoot_model::ids::MatchId) {
        if !self.sys.config.on(Switch::Satellites) {
            return;
        }
        let g = self.world.game_match(m);
        let (Some(home), Some(away)) = (g.home, g.away) else { return };
        self.apply_satellite_tactic(home);
        self.apply_satellite_tactic(away);
        self.start_loanees(m, home, true);
        self.start_loanees(m, away, false);
    }

    /// `Satelites.escalarEmprestados(I, ah, List, List, List)`: the starters (`fF`/`fG`), the
    /// players on the pitch (`fJ`/`fK`) and the bench (`fH`/`fI`) of one side.
    fn start_loanees(&mut self, m: brasfoot_model::ids::MatchId, team: ClubId, home: bool) {
        let g = self.world.game_match(m);
        if (if home { &g.home_starters } else { &g.away_starters }).is_empty() {
            return;
        }
        let Some(parent) = self.parent_of(team) else { return };
        let parent_id = self.world.club(parent).id;
        for p in self.world.club(team).squad.clone() {
            let Some(i) = loan_of(self.world, p) else { continue };
            if self.world.game.loans[i].owner.is_none_or(|o| self.world.club(o).id != parent_id) {
                continue;
            }
            let role = self.loan_role(p);
            if role == LoanRole::Reserve {
                continue;
            }
            if !available_for(self.world, p, m) {
                continue;
            }
            let (games, _, average) = season_numbers(self.world, p);
            if games >= 5 && average > 0.0 && average < 5.5 && sorte!(self, "Satelites").next_boolean() {
                continue;
            }
            if role == LoanRole::Regular && sorte!(self, "Satelites").next_int_bounded(100) >= 75 {
                continue;
            }
            let g = self.world.game_match(m);
            let starters = if home { g.home_starters.clone() } else { g.away_starters.clone() };
            if starters.contains(&p) {
                continue;
            }
            let position = self.world.player(p).position;
            let not_loanee = |w: &World, s: PlayerId| loan_of(w, s).is_none();
            let mut displaced = starters.iter().copied().find(|&s| not_loanee(self.world, s) && self.world.player(s).position == position);
            if displaced.is_none() {
                displaced = starters.iter().copied().find(|&s| not_loanee(self.world, s) && ((position == 0) == (self.world.player(s).position == 0)));
            }
            let Some(d) = displaced else { continue };
            let g = self.world.game_match_mut(m);
            let (st, pl, bench) = if home { (&mut g.home_starters, &mut g.home_played, &mut g.home_bench) } else { (&mut g.away_starters, &mut g.away_played, &mut g.away_bench) };
            if let Some(k) = st.iter().position(|&x| x == d) {
                st[k] = p;
            }
            if let Some(k) = pl.iter().position(|&x| x == d) {
                pl[k] = p;
            }
            bench.retain(|&x| x != p);
            if !bench.contains(&d) {
                bench.push(d);
            }
            let slot = self.world.player(d).slot;
            let x = self.world.player_mut(p);
            x.slot = slot;
            x.started_recently = true;
        }
    }
}

/// `F.d(I)`: not injured, not suspended in the match's competition, and under contract.
fn available_for(world: &World, p: PlayerId, m: brasfoot_model::ids::MatchId) -> bool {
    let today = world.today_millis();
    let x = world.player(p);
    if x.is_injured(today) {
        return false;
    }
    let competition = world.game_match(m).competition;
    if let Some(c) = competition
        && world.competition(c).kind != 0
        && x.competition_stats.iter().any(|h| h.competition_id == Some(c) && h.is_suspended())
    {
        return false;
    }
    x.contract_until >= today
}

/// `Satelites.diretrizTatica(ah)` for the engine (nothing is created): the tactic an owned
/// satellite plays.
pub fn satellite_directive_in(world: &World, config: &crate::config::ModConfig, club: ClubId) -> Option<[i32; 3]> {
    if !config.on(Switch::Satellites) {
        return None;
    }
    let state = world.mods.satellites.as_ref()?;
    let id = world.club(club).id;
    let link = state.link(id).filter(|l| l.owned)?;
    let t = state.tactics.iter().find(|(s, _)| *s == id).map(|&(_, t)| t).unwrap_or_default();
    if t.mirror
        && let Some(&parent) = clubs_by_id(world).get(&link.parent)
    {
        let k = &world.club(parent).tactics;
        if k.len() >= 4 {
            return Some([k[1].clamp(0, 2), k[2].clamp(0, 2), k[3].clamp(0, 1)]);
        }
    }
    Some([t.style, t.marking, t.side])
}
