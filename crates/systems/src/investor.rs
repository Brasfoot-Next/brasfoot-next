// Port of bfmod.Investidor: investors who want to buy the player's club. Once a year a 15% chance
// that an offer arrives on a day between February and November (while the club has no owner).
// Accepting brings money now and a yearly injection, but the owner wants the squad to deliver: a
// season well below what the squad promises cuts the next injection, two in a row and he gives
// up. Ownership itself lives in Economia (it stays with the club if the player leaves).

use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::{InvestorOffer, InvestorState};
use brasfoot_model::rng::java_round;

use crate::awards::coaches::campaigns;
use crate::awards::season::Season;
use crate::config::Switch;
use crate::economy::{MI, year};
use crate::staff::my_club;
use crate::{Mod, sorte};

/// `CHANCE`: the yearly chance (%) of an offer for a club without an owner.
pub const CHANCE: i32 = 15;

impl Mod<'_> {
    /// `Investidor.carregar()`: true on the first load of a save with no state. A save without a
    /// name never loads (`Objects.equals(null, null)`), and its state stays as the statics start.
    fn load_investor(&mut self) -> bool {
        if self.world.game.save_name.is_none() || self.sys.investor_loaded {
            return false;
        }
        self.sys.investor_loaded = true;
        self.world.mods.investor == InvestorState::default()
    }

    /// `Investidor.tick()`
    pub(crate) fn investor_tick(&mut self) {
        let Some(club) = my_club(self.world) else { return };
        if !self.sys.config.on(Switch::Investor) {
            return;
        }
        let first = self.load_investor();
        let year = year(self.world);
        let today = self.today_date();
        let id = self.world.club(club).id;
        let state = &mut self.world.mods.investor;
        if id != state.club {
            state.club = id;
            state.complaints = 0;
        }
        if first {
            // a save that never had this feature: this year's injection counts as paid
            state.year_paid = year;
        }
        if self.club(club).has_owner && self.world.mods.investor.year_paid < year {
            self.world.mods.investor.year_paid = year;
            self.pay_owner_injection(club);
        }
        if self.world.mods.investor.year_drawn < year {
            self.world.mods.investor.year_drawn = year;
            self.world.mods.investor.offer = None;
            if !self.club(club).has_owner && sorte!(self, "Investidor").next_int_bounded(100) < CHANCE {
                self.schedule_investor(club, year, today);
            }
        }
        if std::mem::take(&mut self.sys.investor_test_request) {
            let offer = self.investor_offer(club, today);
            self.world.mods.investor.offer = Some(offer);
        }
        if let Some(offer) = self.world.mods.investor.offer.clone()
            && today >= offer.day
            && self.world.mods.investor.shown.is_none()
        {
            self.world.mods.investor.offer = None;
            if !self.club(club).has_owner {
                // the offer screen (oferecer), answered with accept_investor or refuse_investor
                self.world.mods.investor.shown = Some(offer);
            }
        }
    }

    /// `Investidor.agendar(ah, int, int)`: a day between February and November (from today
    /// on), and the offer.
    fn schedule_investor(&mut self, club: ClubId, year: i32, today: i32) {
        let clock = &self.runtime.clock;
        let mut start = clock.midnight(year, 1, 1);
        let end = clock.midnight(year, 10, 30);
        let now = self.world.today_millis();
        if now > start {
            start = now;
        }
        if start > end {
            return;
        }
        let days = (end - start) / 86_400_000;
        let add = (sorte!(self, "Investidor").next_double() * (days + 1) as f64) as i32;
        let day = self.runtime.clock.add_days(start, add);
        let f = self.runtime.clock.fields(day);
        let date = f.year * 10_000 + (f.month + 1) * 100 + f.day;
        let offer = self.investor_offer(club, today.max(date));
        self.world.mods.investor.offer = Some(offer);
    }

    /// `Investidor.proposta(ah)`: the offer grows with the club's reputation.
    fn investor_offer(&mut self, club: ClubId, day: i32) -> InvestorOffer {
        let factor = 0.5 + 0.12 * self.world.club(club).reputation.clamp(0, 6) as f64;
        let investor = self.new_owner_name();
        let now = java_round((100 + sorte!(self, "Investidor").next_int_bounded(201)) as f64 * MI as f64 * factor);
        let yearly = java_round((40 + sorte!(self, "Investidor").next_int_bounded(81)) as f64 * MI as f64 * factor);
        InvestorOffer { day, investor, now, yearly }
    }

    /// `Investidor.pagarAporte(ah)`: the owner's yearly injection, 40% less after a season of
    /// complaints.
    fn pay_owner_injection(&mut self, club: ClubId) {
        let year = year(self.world);
        let k = self.club(club).clone();
        if k.owner_funding <= 0 || k.owner_since >= year {
            return;
        }
        let value = if self.world.mods.investor.complaints == 1 { java_round(k.owner_funding as f64 * 0.6) } else { k.owner_funding };
        self.world.club_income(club, value.min(i32::MAX as i64) as i32, 9);
        // Jornal.noticia: the injection's news is not ported
    }

    /// The mod's test command "investidor" (`pedidoDeTeste`): an offer on the next tick.
    pub fn request_investor_offer(&mut self) {
        self.sys.investor_test_request = true;
    }

    /// The player accepts the offer on the screen (`Investidor.aceitar`).
    pub fn accept_investor(&mut self) {
        let Some(offer) = self.world.mods.investor.shown.take() else { return };
        let Some(club) = my_club(self.world) else { return };
        if self.club(club).has_owner {
            return;
        }
        self.sell_human_club(club, offer.investor, offer.now, offer.yearly);
        // the purchase year already got its money: the yearly injection starts next season
        self.load_investor();
        let year = year(self.world);
        let id = self.world.club(club).id;
        let state = &mut self.world.mods.investor;
        state.year_paid = year;
        state.complaints = 0;
        state.club = id;
        // Jornal.noticia: the news of the purchase is not ported
        self.fans_event(4, brasfoot_model::mods::FanEvent::NewOwner { investor: offer.investor });
    }

    /// The player turns the offer down (the screen's "Recusar"): only news.
    pub fn refuse_investor(&mut self) {
        self.world.mods.investor.shown = None;
    }

    /// `Investidor.fimDeTemporada(Temporada)`: the owner compares the final place with the one
    /// the squad promised (`Tecnicos.Campanha.prevista`); the season of the purchase does not
    /// count. `Economia.temDono` comes before the switch, so the player's club joins the
    /// economy at every season end.
    pub(crate) fn investor_season_end(&mut self, t: &Season) {
        let Some(club) = my_club(self.world) else { return };
        if !self.club(club).has_owner || !self.sys.config.on(Switch::Investor) {
            return;
        }
        if self.judge_owner(club, t) {
            self.owner_leaves(club);
            // Jornal.noticia: the news is not ported
            self.fans_event(-6, brasfoot_model::mods::FanEvent::OwnerGaveUp);
        }
    }

    /// `Investidor.avaliar(ah, Temporada)`: the owner's patience; true when he gives up.
    fn judge_owner(&mut self, club: ClubId, t: &Season) -> bool {
        self.load_investor();
        let since = self.club(club).owner_since;
        let Some(campaign) = campaigns(self.world).get(&club).cloned() else { return false };
        let state = &self.world.mods.investor;
        if since >= t.year || state.year_judged >= t.year {
            return false;
        }
        let title = t.league_champion(self.world, club) || t.continental_champion(self.world, club);
        let state = &mut self.world.mods.investor;
        state.year_judged = t.year;
        let mut gives_up = false;
        if title || campaign.position <= campaign.expected {
            // the owner's praise is news only
            state.complaints = 0;
        } else if campaign.position > campaign.expected + 2 {
            state.complaints += 1;
            if state.complaints >= 2 {
                gives_up = true;
                state.complaints = 0;
            }
        }
        gives_up
    }
}
