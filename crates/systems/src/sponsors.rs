// Port of bfmod.Patrocinio and bfmod.Cotas: the human club's master sponsor on top of the game's
// fixed sponsor quota (three offers when a contract ends, monthly payments, season-end bonuses)
// and the other slots (sleeve, back, shorts, kit supplier, stadium naming rights, official
// partner), each found by the commercial department and signed from up to three offers.

use std::collections::HashSet;

use brasfoot_model::brands;
use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::{MasterBonusReason, MasterContract, MasterKind, MasterNote, SlotContract, SlotKind, SlotNote, SponsorSlot};
use brasfoot_model::rng::java_round;
use brasfoot_model::world::World;

use crate::awards::coaches::campaigns;
use crate::awards::season::Season;
use crate::config::Switch;
use crate::economy::{league_weights, year};
use crate::staff::my_club;
use crate::{Mod, sorte};

/// `Cotas.VALIDADE_DIAS`
const OFFER_DAYS: i32 = 30;

/// Why the commercial department could not start a search (`Cotas.buscar`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchError {
    /// "Sem clube."
    NoClub,
    /// "Essa cota já tem patrocinador."
    Taken,
    /// "O departamento comercial já está buscando."
    Searching,
}

/// `Patrocinio.base(ah)` without its last factor (`Patrimonio.fatorPatrocinio`): what the market
/// pays a master sponsor of this club per year by size, league, reputation and fan mood.
pub fn master_base_before_factor(world: &World, club: ClubId) -> f64 {
    let yearly_wages = world.club_wages(club) as f64 * 12.0;
    let reputation = world.club(club).reputation.max(1);
    let weight = league_weights(world).get(&club).copied().unwrap_or(1.0);
    let fans = crate::fans::confidence(world) as f64 / 100.0;
    yearly_wages * (0.15 + 0.05 * reputation as f64) * (weight / 1.5) * (0.7 + 0.6 * fans)
}

/// `Patrocinio.multaRescisao()`, `Cotas.multa(Contrato)`: a quarter of what is left.
pub fn early_end_fine(yearly: i64, years: i32) -> i64 {
    java_round(yearly as f64 * years as f64 * 0.25)
}

/// `Cotas.totalAnual()`
pub fn slots_yearly(world: &World) -> i64 {
    world.mods.sponsor_slots.deals.iter().map(|k| k.yearly).sum()
}


impl Mod<'_> {
    /// `Patrocinio.base(ah)`: the factor last, as the original evaluates it (its first call can
    /// start `Patrimonio`, which makes youth players).
    pub fn master_base(&mut self, club: ClubId) -> f64 {
        let before = master_base_before_factor(self.world, club);
        before * self.sponsorship_factor(club)
    }

    /// `Patrocinio.tick()`: offers a new deal when there is none.
    pub(crate) fn master_sponsor_tick(&mut self) {
        if !self.sys.config.on(Switch::Sponsorship) || my_club(self.world).is_none() {
            return;
        }
        let state = &self.world.mods.master_sponsor;
        if state.active().is_none() && state.offers.is_empty() {
            self.world.mods.master_sponsor.offers = self.master_offers();
        }
    }

    /// `Patrocinio.abrirPropostas()`: three offers to pick from now, when the club has no deal
    /// and none is waiting.
    pub fn open_master_offers(&mut self) {
        if my_club(self.world).is_none() {
            return;
        }
        let state = &self.world.mods.master_sponsor;
        if state.active().is_none() && state.offers.is_empty() {
            self.world.mods.master_sponsor.offers = self.master_offers();
        }
    }

    /// `Patrocinio.ofertas()`: fixed, mixed and risky.
    fn master_offers(&mut self) -> Vec<MasterContract> {
        let club = my_club(self.world).expect("the player's club");
        let base = self.master_base(club);
        let mut order: Vec<u16> = (0..brands::MASTER_BRANDS.len() as u16).collect();
        sorte!(self, "Patrocinio").shuffle(&mut order);
        vec![
            self.master_contract(order[0], MasterKind::Fixed, base, 0.0, 0.0, 0.0, 3),
            self.master_contract(order[1], MasterKind::Mixed, base * 0.7, base * 0.3, base * 0.4, base * 0.6, 2),
            self.master_contract(order[2], MasterKind::Risky, base * 0.4, base * 0.6, base * 0.9, base * 1.2, 1),
        ]
    }

    /// `Patrocinio.contrato(String, String, double, double, double, double, int)`
    #[allow(clippy::too_many_arguments)]
    fn master_contract(&mut self, brand: u16, kind: MasterKind, yearly: f64, top4: f64, league: f64, continental: f64, years: i32) -> MasterContract {
        let noise = 0.9 + sorte!(self, "Patrocinio").next_double() * 0.2;
        MasterContract {
            brand,
            kind,
            yearly: java_round(yearly * noise),
            bonus_top4: java_round(top4 * noise),
            bonus_league: java_round(league * noise),
            bonus_continental: java_round(continental * noise),
            years,
        }
    }

    /// `Patrocinio.assinar(Contrato)`: the player picks one of the offers.
    pub fn sign_master_sponsor(&mut self, offer: usize) {
        let state = &mut self.world.mods.master_sponsor;
        let Some(k) = state.offers.get(offer).cloned() else { return };
        state.offers.clear();
        let year = year(self.world);
        let state = &mut self.world.mods.master_sponsor;
        state.history.push(MasterNote::Signed { year, contract: k.clone() });
        state.contract = Some(k);
        // Jornal.noticia: the news of the signing is not ported
    }

    /// `Patrocinio.rescindir()`: ends the master deal (the club pays the fine); the three new
    /// offers come with the next tick.
    pub fn end_master_sponsor(&mut self) {
        let Some(k) = self.world.mods.master_sponsor.active().cloned() else { return };
        let Some(club) = my_club(self.world) else { return };
        let fine = early_end_fine(k.yearly, k.years);
        self.world.club_payment(club, fine.min(i32::MAX as i64) as i32, 8);
        let year = year(self.world);
        let state = &mut self.world.mods.master_sponsor;
        state.history.push(MasterNote::Ended { year, brand: k.brand, fine });
        state.contract = None;
    }

    /// `Patrocinio.pagarMes()`: the monthly instalment of the fixed part.
    pub(crate) fn pay_master_sponsor_month(&mut self) {
        let Some(k) = self.world.mods.master_sponsor.active() else { return };
        let Some(club) = my_club(self.world) else { return };
        if !self.sys.config.on(Switch::Sponsorship) {
            return;
        }
        let month = (k.yearly / 12).min(i32::MAX as i64) as i32;
        self.world.club_income(club, month, 6);
    }

    /// `Patrocinio.fimDeTemporada(Temporada)`: the bonuses, and one season less on the contract.
    pub(crate) fn master_sponsor_season_end(&mut self, t: &Season) {
        let Some(k) = self.world.mods.master_sponsor.active().cloned() else { return };
        let Some(club) = my_club(self.world) else { return };
        let mut bonus = 0;
        let mut reasons = Vec::new();
        if let Some(c) = campaigns(self.world).get(&club)
            && c.division == 1
            && c.position <= 4
            && k.bonus_top4 > 0
        {
            bonus += k.bonus_top4;
            reasons.push(MasterBonusReason::Place(c.position));
        }
        if t.league_champion(self.world, club) && k.bonus_league > 0 {
            bonus += k.bonus_league;
            reasons.push(MasterBonusReason::LeagueTitle);
        }
        if t.continental_champion(self.world, club) && k.bonus_continental > 0 {
            bonus += k.bonus_continental;
            reasons.push(MasterBonusReason::ContinentalTitle);
        }
        if bonus > 0 {
            let net = bonus - self.withhold_income(club, bonus, crate::credit::IncomeSource::SponsorBonus);
            if net > 0 {
                self.world.club_income(club, net.min(i32::MAX as i64) as i32, 6);
            }
            self.world.mods.master_sponsor.history.push(MasterNote::Bonus { year: t.year, amount: bonus, reasons });
        }
        if let Some(k) = &mut self.world.mods.master_sponsor.contract {
            k.years -= 1;
        }
    }

    /// `Cotas.tick()`: offers past their date go, a club change drops the deals, searches
    /// that are due bring their offers.
    pub fn sponsor_slots_tick(&mut self) {
        let Some(club) = my_club(self.world) else { return };
        if !self.sys.config.on(Switch::Sponsorship) {
            return;
        }
        let today = self.today_date();
        let id = self.world.club(club).id;
        let state = &mut self.world.mods.sponsor_slots;
        state.offers.retain(|o| o.valid_until >= today);
        if id != state.club {
            if state.club >= 0 {
                // the deals were the old club's
                state.deals.clear();
                state.offers.clear();
                state.searches.clear();
            }
            state.club = id;
        }
        for (slot, day) in self.world.mods.sponsor_slots.searches.clone() {
            if today >= day {
                self.world.mods.sponsor_slots.searches.retain(|s| s.0 != slot);
                self.slot_search_result(club, slot);
            }
        }
    }

    /// `Cotas.buscar(Cota)`: sends the commercial department after a sponsor.
    pub fn search_sponsor_slot(&mut self, slot: SponsorSlot) -> Result<(), SearchError> {
        if my_club(self.world).is_none() {
            return Err(SearchError::NoClub);
        }
        let state = &self.world.mods.sponsor_slots;
        if state.deals.iter().any(|k| k.slot == slot) {
            return Err(SearchError::Taken);
        }
        if state.searches.iter().any(|s| s.0 == slot) {
            return Err(SearchError::Searching);
        }
        self.world.mods.sponsor_slots.offers.retain(|o| o.slot != slot);
        let days = 5 + sorte!(self, "Cotas").next_int_bounded(8);
        let day = self.date_in(days);
        let searches = &mut self.world.mods.sponsor_slots.searches;
        searches.push((slot, day));
        searches.sort_by_key(|s| s.0);
        Ok(())
    }

    /// `Cotas.assinar(Contrato)`: the offer at that index of the slots' offers.
    pub fn sign_sponsor_slot(&mut self, offer: usize) {
        if my_club(self.world).is_none() {
            return;
        }
        let state = &mut self.world.mods.sponsor_slots;
        let Some(mut k) = state.offers.get(offer).cloned() else { return };
        if state.deals.iter().any(|d| d.slot == k.slot) {
            return;
        }
        state.offers.remove(offer);
        state.offers.retain(|o| o.slot != k.slot);
        k.valid_until = 0;
        state.deals.push(k.clone());
        let year = year(self.world);
        self.world.mods.sponsor_slots.history.push(SlotNote::Signed { year, contract: k });
        // Jornal.noticia: the news of the signing is not ported
    }

    /// `Cotas.recusar(Contrato)`
    pub fn refuse_sponsor_slot(&mut self, offer: usize) {
        let offers = &mut self.world.mods.sponsor_slots.offers;
        if offer < offers.len() {
            offers.remove(offer);
        }
    }

    /// `Cotas.rescindir(Contrato)`: the deal at that index ends, the club pays the fine.
    pub fn end_sponsor_slot(&mut self, deal: usize) {
        let Some(club) = my_club(self.world) else { return };
        let deals = &mut self.world.mods.sponsor_slots.deals;
        if deal >= deals.len() {
            return;
        }
        let k = deals.remove(deal);
        let fine = early_end_fine(k.yearly, k.years);
        self.world.club_payment(club, fine.min(i32::MAX as i64) as i32, 8);
        let year = year(self.world);
        self.world.mods.sponsor_slots.history.push(SlotNote::Ended { year, slot: k.slot, brand: k.brand, fine });
    }

    /// `Cotas.pagarMes()`
    pub(crate) fn pay_sponsor_slots_month(&mut self) {
        let Some(club) = my_club(self.world) else { return };
        if !self.sys.config.on(Switch::Sponsorship) {
            return;
        }
        let month = slots_yearly(self.world) / 12;
        if month > 0 {
            self.world.club_income(club, month.min(i32::MAX as i64) as i32, 6);
        }
    }

    /// `Cotas.fimDeTemporada(Temporada)`: the title bonuses, and one season less on each deal.
    pub(crate) fn sponsor_slots_season_end(&mut self, t: &Season) {
        let Some(club) = my_club(self.world) else { return };
        let title = t.league_champion(self.world, club) || t.continental_champion(self.world, club);
        let mut bonus = 0;
        let deals = &mut self.world.mods.sponsor_slots.deals;
        for k in deals.iter_mut() {
            if title && k.bonus > 0 {
                bonus += k.bonus;
            }
            k.years -= 1;
        }
        // the ended deals' news is not ported
        deals.retain(|k| k.years > 0);
        if bonus > 0 {
            self.world.club_income(club, bonus.min(i32::MAX as i64) as i32, 6);
            self.world.mods.sponsor_slots.history.push(SlotNote::TitleBonus { year: t.year, amount: bonus });
        }
    }

    /// `Cotas.valorDeMercado(ah, Cota)`: what the market would pay for a slot this season.
    pub fn slot_market_value(&mut self, club: ClubId, slot: SponsorSlot) -> i64 {
        let mut v = self.master_base(club) * slot.weight();
        if self.world.season() > 1 {
            let last = Season::new(self.world, self.world.season() - 1);
            v *= 1.0 + (last.title_points(club) / 60.0).min(0.3);
        }
        java_round(v)
    }

    /// `Cotas.resultado(ah, Cota)`: up to three offers, fewer for small clubs or an unhappy
    /// crowd.
    fn slot_search_result(&mut self, club: ClubId, slot: SponsorSlot) {
        let fans = crate::fans::confidence(self.world);
        let reputation = self.world.club(club).reputation;
        let r = sorte!(self, "Cotas");
        let mut n = 1;
        if r.next_int_bounded(100) < fans {
            n += 1;
        }
        if reputation >= 3 && r.next_int_bounded(100) < 60 {
            n += 1;
        }
        if fans < 40 && r.next_int_bounded(100) < 30 {
            n = 0;
        }
        let n = n.min(3);
        if n == 0 {
            // the news that nobody came is not ported
            return;
        }
        let value = self.slot_market_value(club, slot);
        let mut kinds = vec![SlotKind::Fixed, SlotKind::Short, SlotKind::WithBonus];
        sorte!(self, "Cotas").shuffle(&mut kinds);
        let mut taken: HashSet<String> = self.world.mods.sponsor_slots.deals.iter().map(|k| brands::slot_brand(k.brand)).collect();
        if let Some(k) = self.world.mods.master_sponsor.active() {
            taken.insert(brands::MASTER_BRANDS[k.brand as usize].to_owned());
        }
        let valid_until = self.date_in(OFFER_DAYS);
        for &kind in kinds.iter().take(n as usize) {
            let brand = loop {
                // Economia.novaMarca() without its last word, and a line of business
                let (first, _) = self.new_brand();
                let line = sorte!(self, "Cotas").next_int_bounded(brands::SLOT_LINES.len() as i32) as u16;
                if taken.insert(brands::slot_brand((first, line))) {
                    break (first, line);
                }
            };
            let noise = 0.85 + sorte!(self, "Cotas").next_double() * 0.3;
            let v = value as f64;
            let (yearly, bonus, years) = match kind {
                SlotKind::Short => (java_round(v * 1.15 * noise), 0, 1),
                SlotKind::WithBonus => (java_round(v * 0.75 * noise), java_round(v * 0.6 * noise), 2),
                SlotKind::Fixed => (java_round(v * noise), 0, 3),
            };
            self.world.mods.sponsor_slots.offers.push(SlotContract { slot, brand, kind, yearly, bonus, years, valid_until });
        }
        // Jornal.noticia: the offers' news is not ported
    }

    /// `Caixa.daquiA(int)`: the date (yyyymmdd) that many days from today.
    pub fn date_in(&self, days: i32) -> i32 {
        let millis = self.runtime.clock.add_days(self.world.today_millis(), days);
        let f = self.runtime.clock.fields(millis);
        f.year * 10_000 + (f.month + 1) * 100 + f.day
    }
}
