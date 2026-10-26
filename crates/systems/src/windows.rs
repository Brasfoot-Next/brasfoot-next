// Port of the part of bfmod.Janelas the purchase screens ask: which window is open today
// (janelaDoDia, aberta) and why the player may not buy (bloqueioCompra, proximaAbertura).

use brasfoot_model::clock::Clock;
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::world::World;

use crate::{Mod, sorte};
use crate::config::{ModConfig, Switch};

/// `Janelas.janelaDoDia(Calendar)`: 1 in January or in a late pre-season window, 2 in July and
/// August, 0 otherwise.
pub fn window_of_day(world: &World, clock: &Clock, millis: i64) -> i32 {
    let f = clock.fields(millis);
    let month = f.month + 1;
    if month == 7 || month == 8 {
        return 2;
    }
    if month == 1 {
        return 1;
    }
    let day = clock.day_of_year(millis);
    match world.mods.windows.late_preseason {
        Some((year, first, last)) if year == f.year && day >= first && day <= last => 1,
        _ => 0,
    }
}

/// `Janelas.aberta()`
pub fn is_open(world: &World, clock: &Clock) -> bool {
    window_of_day(world, clock, world.today_millis()) > 0
}

/// When the next window opens (`proximaAbertura()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextWindow {
    /// "em 01/07"
    July,
    /// "na pré-temporada de <year>"
    Preseason(i32),
}

/// `Janelas.proximaAbertura()`
pub fn next_opening(world: &World, clock: &Clock) -> NextWindow {
    let f = clock.fields(world.today_millis());
    if f.month + 1 < 7 { NextWindow::July } else { NextWindow::Preseason(f.year + 1) }
}

/// Why the player's club may not buy now (`Janelas.bloqueioCompra()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurchaseBlock {
    /// The bank's transfer ban (`Credito.temTransferBan`).
    TransferBan,
    /// The window is closed and opens then.
    WindowClosed(NextWindow),
}

/// `Janelas.bloqueioCompra()`
pub fn purchase_block(world: &World, clock: &Clock, config: &ModConfig) -> Option<PurchaseBlock> {
    if let Some(club) = crate::staff::my_club(world)
        && crate::credit::has_transfer_ban(world, config, club)
    {
        return Some(PurchaseBlock::TransferBan);
    }
    if !config.on(Switch::UserWindow) || is_open(world, clock) {
        return None;
    }
    Some(PurchaseBlock::WindowClosed(next_opening(world, clock)))
}

fn add_notice(world: &mut World, notice: String) -> bool {
    let notices = &mut world.mods.windows.notices;
    if notices.contains(&notice) {
        return false;
    }
    notices.push(notice);
    true
}

/// `Janelas.preTemporadaAtrasada(Calendar)`: the year's first visible day falls between
/// February and June and no window was seen this year: the pre-season window opens today, for
/// 30 days and at most to the 30th of June.
fn late_preseason(world: &mut World, clock: &Clock, millis: i64) {
    let f = clock.fields(millis);
    let month = f.month + 1;
    if !(2..=6).contains(&month) || world.mods.windows.late_preseason.is_some_and(|(y, _, _)| y == f.year) {
        return;
    }
    let prefix = format!("{}|", f.year);
    if world.mods.windows.notices.iter().any(|a| a.starts_with(&prefix)) {
        return;
    }
    let first = clock.day_of_year(millis);
    let june_30 = clock.day_of_year(clock.date_at(millis, f.year, 5, 30));
    world.mods.windows.late_preseason = Some((f.year, first, (first + PRESEASON_DAYS - 1).min(june_30)));
}

/// `DIAS_PRE_TEMPORADA`
const PRESEASON_DAYS: i32 = 30;

impl Mod<'_> {
    /// `Janelas.tick()`: the window of the day opens, moves on (the AI's deals, spread with a
    /// rush in the last week, and the bloated squads trimmed) and closes.
    pub fn windows_tick(&mut self) {
        let clock = self.runtime.clock.clone();
        let today = self.world.today_millis();
        late_preseason(self.world, &clock, today);
        let number = window_of_day(self.world, &clock, today);
        let id = format!("{}|{number}", clock.fields(today).year);
        if number > 0 {
            let (first, last) = bounds(self.world, &clock, today, number);
            let day = days_between(first, today) + 1;
            let last_day = days_between(first, last) + 1;
            if self.world.mods.windows.current.as_deref() != Some(id.as_str()) {
                if self.world.mods.windows.current.is_some() {
                    close(self.world);
                }
                self.open_window(&id, number, day, last_day);
            }
            self.progress(day, last_day);
            // a week ahead (the news and the notice on screen are not ported)
            if last_day - day <= 7 {
                add_notice(self.world, format!("{id}|semana"));
            }
        } else if self.world.mods.windows.current.is_some() {
            close(self.world);
        }
    }

    /// `Janelas.abrir`: the buyers' plan; a window this save already closed (July before the
    /// window reached August) plans only the days left.
    fn open_window(&mut self, id: &str, number: i32, day: i32, last_day: i32) {
        let w = &mut self.world.mods.windows;
        w.current = Some(id.to_owned());
        let closed = format!("{id}|fechou");
        let reopened = w.notices.iter().position(|a| *a == closed).map(|i| w.notices.remove(i)).is_some();
        w.plan.clear();
        w.planned = 0;
        w.executed = 0;
        w.offers = 0;
        w.requests = 0;
        w.deals.clear();
        self.sys.traded.clear();
        let left = if reopened { 1.0 - target_share(day, last_day) } else { 1.0 };
        let mut extra = 0i64;
        self.ai_credit_decisions();
        for (club, signings, budget) in self.plan_window(number) {
            let n = (signings as f64 * left).round() as i64;
            if n > 0 {
                self.world.mods.windows.plan.push((club, n, (budget as f64 * left).round() as i64));
                extra += n;
            }
        }
        let w = &mut self.world.mods.windows;
        if reopened {
            // the pace goes on: what was already done counts as executed (the news is not ported)
            w.planned = if left > 0.0 { (extra as f64 / left).round() as i32 } else { extra as i32 };
            w.executed = w.planned - extra as i32;
            return;
        }
        w.planned = extra as i32;
        if self.sys.config.on(Switch::AiLoans) {
            self.window_loans(number);
        }
        self.satellites_window();
        // the news, the director's report and the notice on screen are not ported
        add_notice(self.world, format!("{id}|abriu"));
    }

    /// `Janelas.novoProjeto(ah)`: a club just bought by a fund joins the open window with its
    /// spree (the next window plans it otherwise).
    pub fn new_project(&mut self, club: brasfoot_model::ids::ClubId) {
        if self.world.mods.windows.current.is_none() {
            return;
        }
        let (id, signings, budget) = self.project_plan(club);
        let w = &mut self.world.mods.windows;
        let before = match w.plan.iter_mut().find(|p| p.0 == id) {
            Some(p) => std::mem::replace(p, (id, signings, budget)).1,
            None => {
                w.plan.push((id, signings, budget));
                0
            }
        };
        w.planned += (signings - before) as i32;
    }

    /// `Janelas.progredir(int, int, int)`
    fn progress(&mut self, day: i32, last_day: i32) {
        let target = (self.world.mods.windows.planned as f64 * target_share(day, last_day)).ceil() as i32;
        let clubs = crate::economy::clubs_by_id(self.world);
        let mut any = true;
        while self.world.mods.windows.executed < target && any {
            any = false;
            // one deal per club per pass: new owners first, everyone else in a random order
            let mut order: Vec<i32> = self.world.mods.windows.plan.iter().map(|p| p.0).collect();
            crate::sorte!(self, "Janelas").shuffle(&mut order);
            let mut project = std::collections::HashMap::new();
            for &id in &order {
                let in_project = clubs.get(&id).is_some_and(|&c| self.in_project(c));
                project.insert(id, in_project);
            }
            order.sort_by_key(|id| !project[id]);
            for id in order {
                if self.world.mods.windows.executed >= target {
                    break;
                }
                let i = self.world.mods.windows.plan.iter().position(|p| p.0 == id).expect("planned");
                let (_, signings, budget) = self.world.mods.windows.plan[i];
                let Some(&buyer) = clubs.get(&id) else { continue };
                if signings <= 0 {
                    continue;
                }
                match self.sign_one(buyer, budget) {
                    None => {
                        // nobody fits this buyer any more: his slots count as done
                        let w = &mut self.world.mods.windows;
                        w.executed += signings as i32;
                        w.plan[i].1 = 0;
                    }
                    Some((p, from, fee)) => {
                        let names = (self.world.player(p).name.clone(), self.world.club(from).name.clone(), self.world.club(buyer).name.clone());
                        let w = &mut self.world.mods.windows;
                        w.plan[i].1 -= 1;
                        w.plan[i].2 -= fee;
                        w.executed += 1;
                        any = true;
                        w.deals.push((fee, names.0, names.1, names.2));
                    }
                }
            }
        }
        for t in crate::economy::ai_clubs(self.world) {
            if self.world.club(t).squad.len() > crate::market::SQUAD_TARGET {
                self.trim_squad(t, 2);
            }
        }
        let mut offers = Vec::new();
        if self.sys.config.on(Switch::Offers) && self.world.mods.windows.offers < 2 && sorte!(self, "Janelas").next_int_bounded(100) < 30 {
            let found = self.offers_for_humans(1);
            self.world.mods.windows.offers += found.len() as i32;
            offers.extend(found);
        }
        if self.sys.config.on(Switch::LoanRequests) && self.world.mods.windows.requests < 2 && sorte!(self, "Janelas").next_int_bounded(100) < 25 {
            let found = self.loan_requests_for_humans();
            if let Some(&first) = found.first() {
                offers.push(first);
                self.world.mods.windows.requests += 1;
            }
        }
        self.sys.squad_offers.extend(offers);
    }

    /// `Mercado.mostrar(List, int)`: the human's answer to an offer of the game's offer screen
    /// (`a.jm`): a sale for the value, or a loan with its fee (`Emprestimos.aoEmprestar`). An
    /// offer for a player no longer at a human club is not shown.
    /// Returns whether a sale or loan was made: the game then rebuilds the main screen
    /// (`best.S.aY(2)`), whose card runs the mod's clock again ([`crate::ModSystems::main_screen_tick`]).
    pub fn answer_squad_offer(&mut self, offer: SquadOffer, accept: bool) -> bool {
        let Some(owner) = self.world.player(offer.player).club else { return false };
        if !crate::economy::is_human(self.world, owner) || !accept {
            return false;
        }
        let value = offer.value.min(i32::MAX as i64) as i32;
        if offer.loan {
            self.lend(offer.player, offer.buyer);
            self.pay_loan_fee(offer.buyer, owner, value as i64);
        } else {
            brasfoot_simulation::Sim { world: self.world, runtime: self.runtime, hooks: &mut brasfoot_engine::hooks::NoSystems::default(), human: None }
                .transfer(offer.player, offer.buyer, value);
        }
        true
    }
}

/// An offer for a player of a human squad (`Object[]{player, buyer, value, loan}`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SquadOffer {
    pub player: PlayerId,
    pub buyer: ClubId,
    /// The fee offered, or the loan fee.
    pub value: i64,
    pub loan: bool,
}

/// `Janelas.alvo(int, int)`: the share of the window's deals due by `day`: 70% before the last
/// week, the rest in the final rush.
fn target_share(day: i32, last_day: i32) -> f64 {
    let rush = last_day - FINAL_RUSH_DAYS + 1;
    if day < rush { 0.7 * day as f64 / (rush - 1) as f64 } else { (0.7 + 0.3 * (day - rush + 1) as f64 / (last_day - rush + 1) as f64).min(1.0) }
}

/// `DIAS_RETA_FINAL`
const FINAL_RUSH_DAYS: i32 = 7;

/// `Janelas.fechar`: the window's deals are cleared (the news of the biggest is not ported).
fn close(world: &mut World) {
    let w = &mut world.mods.windows;
    if let Some(id) = w.current.take() {
        let closed = format!("{id}|fechou");
        if !w.notices.contains(&closed) {
            w.notices.push(closed);
            w.deals.clear();
            w.plan.clear();
        }
    }
}

/// `Janelas.inicio` and `Janelas.fim`: the first and the last day of window `number` in the
/// year of `millis` (01/01 to 31/01, the late pre-season's days, or 01/07 to 31/08).
fn bounds(world: &World, clock: &Clock, millis: i64, number: i32) -> (i64, i64) {
    let year = clock.fields(millis).year;
    if number == 1
        && let Some((y, first, last)) = world.mods.windows.late_preseason
        && y == year
    {
        return (clock.with_day_of_year(millis, first), clock.with_day_of_year(millis, last));
    }
    if number == 1 {
        (clock.date_at(millis, year, 0, 1), clock.date_at(millis, year, 0, 31))
    } else {
        (clock.date_at(millis, year, 6, 1), clock.date_at(millis, year, 7, 31))
    }
}

/// `Janelas.diasEntre`: whole days, rounded.
fn days_between(from: i64, to: i64) -> i32 {
    ((to - from) as f64 / 86_400_000.0).round() as i32
}

/// The windows' calendar alone (which window is open, the notices), without the deals: what
/// the app keeps when the rest of the mod's tick reaches a part not ported yet.
pub fn calendar_tick(world: &mut World, clock: &Clock) {
    let today = world.today_millis();
    late_preseason(world, clock, today);
    let number = window_of_day(world, clock, today);
    let id = format!("{}|{number}", clock.fields(today).year);
    if number > 0 {
        let (first, last) = bounds(world, clock, today, number);
        if world.mods.windows.current.as_deref() != Some(id.as_str()) {
            if world.mods.windows.current.is_some() {
                close(world);
            }
            world.mods.windows.current = Some(id.clone());
            let closed = format!("{id}|fechou");
            let notices = &mut world.mods.windows.notices;
            match notices.iter().position(|a| *a == closed) {
                Some(i) => {
                    notices.remove(i);
                }
                None => {
                    add_notice(world, format!("{id}|abriu"));
                }
            }
        }
        if days_between(first, last) - days_between(first, today) <= 7 {
            add_notice(world, format!("{id}|semana"));
        }
    } else if world.mods.windows.current.is_some() {
        close(world);
    }
}
