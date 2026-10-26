// Port of bfmod.Credito: the bank. Human and AI clubs borrow by their repayment capacity and
// credit rating (AAA to D); a missed instalment brings interest, a lower rating, then a transfer
// ban with 30% of the income seized, then points lost in the league and the best player sold.

use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::mods::{BankLoan, CreditStage, CreditState, CreditStatus, SatellitePolicy};
use brasfoot_model::rng::java_round;

use crate::config::Switch;
use crate::economy::{ai_clubs, clubs_by_id, is_human, year};
use crate::{Mod, sorte};

const MI: i64 = 1_000_000;

/// `Credito.Rating`, in its order: the base monthly rate and the leverage factor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
    D,
}

impl Rating {
    const ALL: [Rating; 8] = [Rating::AAA, Rating::AA, Rating::A, Rating::BBB, Rating::BB, Rating::B, Rating::CCC, Rating::D];

    /// `taxaBase`
    pub fn base_rate(self) -> f64 {
        [0.006, 0.008, 0.011, 0.015, 0.022, 0.032, 0.045, 0.060][self as usize]
    }

    /// `fatorAlavancagem`
    pub fn leverage(self) -> f64 {
        [1.20, 1.00, 0.80, 0.60, 0.40, 0.25, 0.12, 0.00][self as usize]
    }

    fn down(self, steps: usize) -> Rating {
        Rating::ALL[(self as usize + steps).min(Rating::ALL.len() - 1)]
    }
}

/// What the bank seizes a share of (`reterReceita`'s origin); the news word it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncomeSource {
    PlayerSale,
    SponsorBonus,
}

impl Mod<'_> {
    /// The bank's state, loaded on first use in this run (`carregar()`).
    fn bank(&mut self) -> &mut CreditState {
        self.sys.credit_loaded = true;
        self.world.mods.credit.get_or_insert_with(CreditState::default)
    }

    fn loans_of_id(&mut self, id: i32) -> Option<&mut Vec<BankLoan>> {
        self.bank().loans.iter_mut().find(|(c, _)| *c == id).map(|(_, l)| l)
    }

    /// `Credito.statusDe(ah)`: created on first use.
    pub fn credit_status(&mut self, club: ClubId) -> CreditStatus {
        let id = self.world.club(club).id;
        self.status_mut(id).clone()
    }

    fn status_mut(&mut self, id: i32) -> &mut CreditStatus {
        let bank = self.bank();
        if !bank.statuses.iter().any(|s| s.club == id) {
            bank.statuses.push(CreditStatus { club: id, ..Default::default() });
        }
        bank.statuses.iter_mut().find(|s| s.club == id).expect("just added")
    }

    /// `Credito.temTransferBan(ah)`: with the bank on, a club in the ban or the intervention.
    pub fn transfer_ban(&mut self, club: ClubId) -> bool {
        if self.sys.config.on(Switch::Credit) {
            self.sys.credit_loaded = true;
        }
        transfer_ban_in(self.world, &self.sys.config, club)
    }

    /// `Credito.emprestimosDe(ah)`
    pub fn bank_loans(&mut self, club: ClubId) -> Vec<BankLoan> {
        let id = self.world.club(club).id;
        self.loans_of_id(id).cloned().unwrap_or_default()
    }

    /// `Credito.receitaAnualEstimada(ah)`: TV money by division, the master sponsor, the gate of
    /// twenty home games at 60% and the members.
    pub fn estimated_revenue(&mut self, club: ClubId) -> i64 {
        let tv = match self.world.club(club).division {
            1 => 60 * MI,
            2 => 22 * MI,
            3 => 8 * MI,
            _ => 3 * MI,
        };
        let mut sponsor = 0;
        if is_human(self.world, club) {
            if let Some(k) = &self.world.mods.master_sponsor.contract
                && k.years > 0
            {
                sponsor = k.yearly;
            }
        } else {
            let c = self.club(club);
            if c.brand.is_some() {
                sponsor = c.sponsorship;
            }
        }
        if sponsor <= 0 {
            sponsor = java_round(self.master_base(club));
        }
        let capacity = self.stadium_capacity(club);
        let ticket = (25 + self.world.club(club).reputation * 12).clamp(25, 85) as i64;
        let gate = java_round(capacity as f64 * 0.60 * ticket as f64 * 20.0);
        let members = if is_human(self.world, club) { crate::fans::monthly_members(self.world, &self.sys.config) * 12 } else { capacity * 180 };
        (2 * MI).max(tv + sponsor + gate + members)
    }

    /// `ah.ev().dW()`, 10 thousand without a stadium.
    fn stadium_capacity(&self, club: ClubId) -> i64 {
        match self.world.club(club).stadium {
            Some(s) => self.world.stadium(s).capacity.iter().take(4).map(|&c| c as i64).sum(),
            None => 10_000,
        }
    }

    /// `Credito.valorAtivos(ah)`: the squad's value, the stadium and the infrastructure.
    pub fn asset_value(&mut self, club: ClubId) -> i64 {
        let squad: i64 = self.world.club(club).squad.iter().map(|&p| self.world.player(p).market_value as i64).sum();
        let stadium = self.stadium_capacity(club) * 120;
        let e = self.assets_data(club);
        let infrastructure = (e.arena + e.training_centre + e.academy + e.marketing) as i64 * 2_500_000;
        (3 * MI).max(squad + stadium + infrastructure)
    }

    /// `Credito.dividaAtiva(ah)`: the bank's balances and, for a human club, the game's own loan.
    pub fn active_debt(&mut self, club: ClubId) -> i64 {
        let id = self.world.club(club).id;
        let mut debt: i64 = self.loans_of_id(id).map(|l| l.iter().map(|e| e.balance).sum()).unwrap_or(0);
        if is_human(self.world, club) {
            debt += self.world.club(club).finances.bank_loan as i64;
        }
        debt
    }

    /// `Credito.calcularRating(ah)`: D from six months late, CCC from three; else by the debt
    /// over the capacity (revenue and 15% of the assets) by division, two steps down when late.
    pub fn credit_rating(&mut self, club: ClubId) -> Rating {
        let id = self.world.club(club).id;
        let late = self.loans_of_id(id).map(|l| l.iter().map(|e| e.months_late).max().unwrap_or(0)).unwrap_or(0);
        if late >= 6 {
            return Rating::D;
        }
        if late >= 3 {
            return Rating::CCC;
        }
        let revenue = self.estimated_revenue(club);
        let assets = self.asset_value(club);
        let debt = self.active_debt(club);
        let capacity = revenue as f64 + 0.15 * assets as f64;
        let leverage = debt as f64 / (1_000_000.0f64).max(capacity);
        let division = self.world.club(club).division;
        let reputation = self.world.club(club).reputation;
        let base = if division <= 1 {
            if leverage < 0.20 && reputation >= 4 {
                Rating::AAA
            } else if leverage < 0.40 {
                Rating::AA
            } else if leverage < 0.70 {
                Rating::A
            } else if leverage < 1.05 {
                Rating::BBB
            } else if leverage < 1.45 {
                Rating::BB
            } else if leverage < 1.90 {
                Rating::B
            } else if leverage < 2.50 {
                Rating::CCC
            } else {
                Rating::D
            }
        } else if division == 2 {
            if leverage < 0.20 && reputation >= 3 {
                Rating::AA
            } else if leverage < 0.50 {
                Rating::A
            } else if leverage < 0.85 {
                Rating::BBB
            } else if leverage < 1.25 {
                Rating::BB
            } else if leverage < 1.70 {
                Rating::B
            } else if leverage < 2.30 {
                Rating::CCC
            } else {
                Rating::D
            }
        } else if leverage < 0.30 {
            Rating::BBB
        } else if leverage < 0.70 {
            Rating::BB
        } else if leverage < 1.15 {
            Rating::B
        } else if leverage < 1.70 {
            Rating::CCC
        } else {
            Rating::D
        };
        if late >= 1 { base.down(2) } else { base }
    }

    /// `Credito.limiteTotal(ah)`
    pub fn credit_limit(&mut self, club: ClubId) -> i64 {
        let r = self.credit_rating(club);
        if r == Rating::D {
            return 0;
        }
        let revenue = self.estimated_revenue(club);
        let assets = self.asset_value(club);
        let base = java_round((revenue as f64 * 0.75 + assets as f64 * 0.12) * r.leverage());
        let minimum = match self.world.club(club).division {
            1 => 15 * MI,
            2 => 6 * MI,
            3 => 2_500_000,
            _ => 1_200_000,
        };
        java_round(minimum as f64 * r.leverage()).max(base)
    }

    /// `Credito.limiteDisponivel(ah)`
    pub fn available_credit(&mut self, club: ClubId) -> i64 {
        if self.credit_rating(club) == Rating::D || self.transfer_ban(club) {
            return 0;
        }
        let total = self.credit_limit(club);
        let active = self.active_debt(club);
        (total - active).max(0)
    }

    /// `Credito.taxaJuros(ah, int)`: the rating's rate, more for 24 and 36 months.
    pub fn interest_rate(&mut self, club: ClubId, months: i32) -> f64 {
        let mut rate = self.credit_rating(club).base_rate();
        if months == 24 {
            rate += 0.0015;
        } else if months == 36 {
            rate += 0.0035;
        }
        java_round(rate * 10000.0) as f64 / 10000.0
    }

    /// `Credito.podeContratar(ah, long, int)`
    pub fn can_borrow(&mut self, club: ClubId, amount: i64, months: i32) -> bool {
        if amount <= 0 || !matches!(months, 12 | 24 | 36) {
            return false;
        }
        if self.transfer_ban(club) || self.credit_rating(club) == Rating::D {
            return false;
        }
        amount <= self.available_credit(club)
    }

    /// `Credito.contratar(ah, long, int)`: the money goes into the club's cash (category 9 for a
    /// human club).
    pub fn borrow(&mut self, club: ClubId, amount: i64, months: i32) -> Option<BankLoan> {
        if !self.can_borrow(club, amount, months) {
            return None;
        }
        let rate = self.interest_rate(club, months);
        let month = self.runtime.clock.fields(self.world.today_millis()).month + 1;
        let id = self.world.club(club).id;
        let year = year(self.world);
        let next = self.bank().next_id;
        self.bank().next_id += 1;
        let loan = BankLoan {
            id: next,
            club: id,
            original: amount,
            balance: amount,
            instalments: months,
            paid: 0,
            instalment: instalment(amount, months, rate),
            monthly_rate: rate,
            months_late: 0,
            year,
            month,
        };
        if is_human(self.world, club) {
            self.inject_human(club, amount);
        } else {
            let c = self.club(club);
            c.fortune += amount;
            let fortune = c.fortune;
            self.world.club_mut(club).cash = fortune;
        }
        match self.loans_of_id(id) {
            Some(l) => l.push(loan.clone()),
            None => self.bank().loans.push((id, vec![loan.clone()])),
        }
        // Jornal.noticia: the loan's news is not ported
        Some(loan)
    }

    /// `Credito.quitarAntecipado(ah, int)`: the balance paid off now.
    pub fn repay_early(&mut self, club: ClubId, loan_id: i32) -> bool {
        let id = self.world.club(club).id;
        let Some(loan) = self.loans_of_id(id).and_then(|l| l.iter().find(|e| e.id == loan_id).cloned()) else { return false };
        let cost = loan.balance;
        let enough = if is_human(self.world, club) { self.world.club(club).cash >= cost } else { self.fortune(club) >= cost };
        if !enough {
            return false;
        }
        if is_human(self.world, club) {
            self.debit_human(club, cost);
        } else {
            self.spend(club, cost);
            let fortune = self.fortune(club);
            self.world.club_mut(club).cash = fortune;
        }
        if let Some(l) = self.loans_of_id(id) {
            l.retain(|e| e.id != loan_id);
        }
        self.bank().loans.retain(|(c, l)| *c != id || !l.is_empty());
        self.reassess_credit(club);
        // Jornal.noticia: the payoff's news is not ported
        true
    }

    /// `Credito.pagarMes()` (from `Mensal`): each club pays its instalments; a club that cannot
    /// falls behind: interest of 1.5%, then the ban and 30% of its cash seized (from three
    /// months), then points lost once a year and its best player sold (from six).
    pub(crate) fn pay_bank_month(&mut self) {
        if !self.sys.config.on(Switch::Credit) {
            return;
        }
        if self.bank().loans.is_empty() {
            self.ai_bank_emergency();
            return;
        }
        let clubs = clubs_by_id(self.world);
        let ids: Vec<i32> = self.bank().loans.iter().map(|(c, _)| *c).collect();
        for id in ids {
            let Some(&club) = clubs.get(&id) else { continue };
            let Some(loans) = self.loans_of_id(id).cloned() else { continue };
            if loans.is_empty() {
                continue;
            }
            let mut done = Vec::new();
            for (i, _) in loans.iter().enumerate() {
                let e = self.loans_of_id(id).expect("the club's loans")[i].clone();
                if e.balance <= 0 || e.paid >= e.instalments {
                    done.push(i);
                    continue;
                }
                let can_pay = if is_human(self.world, club) { self.world.club(club).cash >= e.instalment } else { self.fortune(club) >= e.instalment };
                if can_pay {
                    if is_human(self.world, club) {
                        self.debit_human(club, e.instalment);
                    } else {
                        self.spend(club, e.instalment);
                        let fortune = self.fortune(club);
                        self.world.club_mut(club).cash = fortune;
                    }
                    let l = &mut self.loans_of_id(id).expect("the club's loans")[i];
                    let interest = java_round(l.balance as f64 * l.monthly_rate);
                    let amortization = (l.instalment - interest).max(0);
                    l.balance = (l.balance - amortization).max(0);
                    l.paid += 1;
                    if l.months_late > 0 {
                        l.months_late -= 1;
                    }
                    if l.balance <= 0 || l.paid >= l.instalments {
                        done.push(i);
                    }
                } else {
                    let l = &mut self.loans_of_id(id).expect("the club's loans")[i];
                    l.months_late += 1;
                    l.balance += java_round(l.instalment as f64 * 0.015);
                    let (late, instalment) = (l.months_late, l.instalment);
                    if (1..=2).contains(&late) {
                        self.status_mut(id).stage = CreditStage::Warning;
                        // Jornal.noticia: the human club's warning is not ported
                    } else if (3..=5).contains(&late) {
                        self.status_mut(id).stage = CreditStage::TransferBan;
                        // Jornal.noticia: the ban's news is not ported
                        let cash = if is_human(self.world, club) { self.world.club(club).cash } else { self.fortune(club) };
                        if cash > 500_000 {
                            let seized = java_round(cash as f64 * 0.30).min(instalment * late as i64);
                            if seized > 0 {
                                if is_human(self.world, club) {
                                    self.debit_human(club, seized);
                                } else {
                                    self.spend(club, seized);
                                    let fortune = self.fortune(club);
                                    self.world.club_mut(club).cash = fortune;
                                }
                                let l = &mut self.loans_of_id(id).expect("the club's loans")[i];
                                l.balance = (l.balance - seized).max(0);
                                self.status_mut(id).withheld += seized;
                            }
                        }
                    } else if late >= 6 {
                        self.status_mut(id).stage = CreditStage::Intervention;
                        let year = year(self.world);
                        if self.status_mut(id).last_deduction_year != year {
                            let points = if late >= 9 { 6 } else { 3 };
                            if self.deduct_points(club, points) {
                                let s = self.status_mut(id);
                                s.last_deduction_year = year;
                                s.points_deducted += points;
                                // Jornal.noticia: the deduction's news is not ported
                            }
                        }
                        self.forced_sale(club, id, i);
                    }
                }
            }
            if let Some(l) = self.loans_of_id(id) {
                let mut k = 0;
                l.retain(|_| {
                    let keep = !done.contains(&k);
                    k += 1;
                    keep
                });
            }
            self.bank().loans.retain(|(c, l)| *c != id || !l.is_empty());
            self.reassess_credit(club);
        }
        self.ai_bank_emergency();
    }

    /// `Credito.reavaliarStatus(ah)`: the stage by the months the club is late.
    fn reassess_credit(&mut self, club: ClubId) {
        let id = self.world.club(club).id;
        let late = self.loans_of_id(id).map(|l| l.iter().map(|e| e.months_late).max().unwrap_or(0)).unwrap_or(0);
        let s = self.status_mut(id);
        s.stage = match late {
            0 => CreditStage::Normal,
            1..=2 => CreditStage::Warning,
            3..=5 => CreditStage::TransferBan,
            _ => CreditStage::Intervention,
        };
        // Jornal.noticia: the end of the intervention is not ported
    }

    /// `Credito.aplicarPerdaDePontos(ah, int)`: points off the club's record in its league's
    /// table, which is sorted again.
    fn deduct_points(&mut self, club: ClubId, points: i32) -> bool {
        let id = self.world.club(club).country;
        let Some(country) = self.world.game.countries.iter().copied().find(|&c| self.world.country(c).country == id) else { return false };
        for league in self.world.country(country).divisions.clone() {
            let Some(table) = self.world.competition(league).league else { continue };
            if !self.world.stage(table).clubs.contains(&club) {
                continue;
            }
            let Some(r) = self.world.club_mut(club).stage_records.iter_mut().find(|r| r.stage == Some(table)) else { return false };
            r.points -= points;
            // h.a.abn reads the table of f.s.Zl, the last league stage whose round ended, not
            // this one: the original's bug, kept (a club of another table, or none, leaves the
            // order as it was until this league's next round)
            if let Some(read) = self.runtime.table_stage {
                let mut clubs = self.world.stage(table).clubs.clone();
                clubs.sort_by(|&a, &b| brasfoot_simulation::sorting::standings(self.world, read, a, b));
                self.world.stage_mut(table).clubs = clubs;
            }
            return true;
        }
        false
    }

    /// `Credito.executarVendaForcada(ah, Emprestimo)`: with more than 16 players, the most
    /// valuable (a million or more, not on loan) goes at 75% to the first AI club that can pay
    /// 1.5 times that (or any without a ban); the money pays the loan and the rest goes to the
    /// club.
    fn forced_sale(&mut self, club: ClubId, id: i32, loan: usize) {
        if self.world.club(club).squad.len() <= 16 {
            return;
        }
        let mut best: Option<PlayerId> = None;
        for &p in &self.world.club(club).squad {
            let x = self.world.player(p);
            if !x.on_loan && x.market_value > 0 && best.is_none_or(|b| x.market_value > self.world.player(b).market_value) {
                best = Some(p);
            }
        }
        let Some(p) = best else { return };
        if (self.world.player(p).market_value as i64) < MI {
            return;
        }
        let price = java_round(self.world.player(p).market_value as f64 * 0.75);
        let mut buyer = None;
        for t in ai_clubs(self.world) {
            if t != club && !self.transfer_ban(t) && self.fortune(t) as f64 >= price as f64 * 1.5 {
                buyer = Some(t);
                break;
            }
        }
        if buyer.is_none() {
            for t in ai_clubs(self.world) {
                if t != club && !self.transfer_ban(t) {
                    buyer = Some(t);
                    break;
                }
            }
        }
        let Some(buyer) = buyer else { return };
        brasfoot_simulation::Sim { world: self.world, runtime: self.runtime, hooks: &mut brasfoot_engine::hooks::NoSystems::default(), human: None }.transfer(p, buyer, price.min(i32::MAX as i64) as i32);
        self.account_transfers();
        let l = &mut self.loans_of_id(id).expect("the club's loans")[loan];
        let paid = price.min(l.balance);
        l.balance = (l.balance - paid).max(0);
        let instalments = if l.instalment > 0 { paid / l.instalment } else { 1 }.max(1);
        l.months_late = (l.months_late as i64 - instalments).max(0) as i32;
        let surplus = price - paid;
        if surplus > 0 {
            if is_human(self.world, club) {
                self.inject_human(club, surplus);
            } else {
                let c = self.club(club);
                c.fortune += surplus;
                let fortune = c.fortune;
                self.world.club_mut(club).cash = fortune;
            }
        }
        // Jornal.noticia: the forced sale's news is not ported
    }

    /// `Credito.reterReceita(ah, long, String)`: a club in the ban or the intervention loses 30%
    /// of an income to its late loans; the amount withheld.
    pub(crate) fn withhold_income(&mut self, club: ClubId, amount: i64, _source: IncomeSource) -> i64 {
        if amount <= 0 || !self.sys.config.on(Switch::Credit) {
            return 0;
        }
        let id = self.world.club(club).id;
        let stage = self.status_mut(id).stage;
        if !matches!(stage, CreditStage::TransferBan | CreditStage::Intervention) {
            return 0;
        }
        let Some(loans) = self.loans_of_id(id) else { return 0 };
        if loans.is_empty() {
            return 0;
        }
        let withheld = java_round(amount as f64 * 0.30);
        let mut paid = 0;
        for e in loans.iter_mut() {
            if e.months_late > 0 && e.balance > 0 {
                let part = (withheld - paid).min(e.balance);
                if part > 0 {
                    e.balance -= part;
                    paid += part;
                    let instalments = if e.instalment > 0 { part / e.instalment } else { 1 }.max(1);
                    e.months_late = (e.months_late as i64 - instalments).max(0) as i32;
                    if paid >= withheld {
                        break;
                    }
                }
            }
        }
        if paid > 0 {
            self.status_mut(id).withheld += paid;
            self.reassess_credit(club);
            // Jornal.noticia: the seizure's news is not ported
        }
        paid
    }

    /// `Credito.verificarIAEmergencia()`: an AI club in the red borrows its deficit and 4 million
    /// over 36 months, as far as its limit goes (at least 2 million).
    fn ai_bank_emergency(&mut self) {
        for t in ai_clubs(self.world) {
            if is_human(self.world, t) || self.transfer_ban(t) {
                continue;
            }
            let fortune = self.fortune(t);
            if fortune < 0 {
                let deficit = fortune.abs() + 4 * MI;
                let available = self.available_credit(t);
                let amount = deficit.min(available);
                if amount >= 2 * MI && self.credit_rating(t) != Rating::D {
                    self.borrow(t, amount, 36);
                }
            }
        }
    }

    /// `Credito.decidirCreditoIA()` (from a window): an ambitious AI club without debt and with
    /// a top rating borrows 10 to 40 million, one time in five or so.
    pub(crate) fn ai_credit_decisions(&mut self) {
        if !self.sys.config.on(Switch::Credit) {
            return;
        }
        self.bank();
        for t in ai_clubs(self.world) {
            if is_human(self.world, t) || self.transfer_ban(t) {
                continue;
            }
            if self.active_debt(t) > 0 {
                continue;
            }
            let r = self.credit_rating(t);
            if matches!(r, Rating::AAA | Rating::AA | Rating::A) {
                let ambitious = self.world.club(t).reputation >= 3 || self.satellite_policy(t) == Some(SatellitePolicy::Promotion);
                if ambitious && sorte!(self, "Credito").next_int_bounded(100) < 22 {
                    let available = self.available_credit(t);
                    let amount = available.min((10 + sorte!(self, "Credito").next_int_bounded(31) as i64) * MI);
                    if amount >= 5 * MI {
                        let months = if sorte!(self, "Credito").next_boolean() { 24 } else { 36 };
                        self.borrow(t, amount, months);
                    }
                }
            }
        }
    }

    /// `Credito.injetarCaixaHumano(ah, long)`: into the game's books in parts of an int.
    fn inject_human(&mut self, club: ClubId, amount: i64) {
        let mut left = amount;
        while left > 0 {
            let part = left.min(i32::MAX as i64);
            self.world.club_income(club, part as i32, 9);
            left -= part;
        }
    }

    /// `Credito.debitarCaixaHumano(ah, long)`
    fn debit_human(&mut self, club: ClubId, amount: i64) {
        let mut left = amount;
        while left > 0 {
            let part = left.min(i32::MAX as i64);
            self.world.club_payment(club, part as i32, 8);
            left -= part;
        }
    }
}

/// `Credito.temTransferBan(ah)` on the world alone (the status is created on first use, as the
/// original's `statusDe`).
pub fn transfer_ban_in(world: &mut brasfoot_model::world::World, config: &crate::config::ModConfig, club: ClubId) -> bool {
    if !config.on(Switch::Credit) {
        return false;
    }
    let id = world.club(club).id;
    let bank = world.mods.credit.get_or_insert_with(CreditState::default);
    if !bank.statuses.iter().any(|s| s.club == id) {
        bank.statuses.push(CreditStatus { club: id, ..Default::default() });
    }
    let stage = bank.statuses.iter().find(|s| s.club == id).expect("just added").stage;
    matches!(stage, CreditStage::TransferBan | CreditStage::Intervention)
}

/// `Credito.temTransferBan(ah)` for the screens: no state is created.
pub fn has_transfer_ban(world: &brasfoot_model::world::World, config: &crate::config::ModConfig, club: ClubId) -> bool {
    if !config.on(Switch::Credit) {
        return false;
    }
    let id = world.club(club).id;
    world.mods.credit.as_ref().and_then(|b| b.statuses.iter().find(|s| s.club == id)).is_some_and(|s| matches!(s.stage, CreditStage::TransferBan | CreditStage::Intervention))
}

/// `Credito.calcularParcela(long, int, double)`: the instalment of a loan at a monthly rate
/// (Price table).
pub fn instalment(amount: i64, months: i32, rate: f64) -> i64 {
    if amount <= 0 || months <= 0 {
        return 0;
    }
    if rate <= 0.0 {
        return amount / months as i64;
    }
    let factor = (1.0 + rate).powf(months as f64);
    java_round(amount as f64 * (rate * factor) / (factor - 1.0))
}
