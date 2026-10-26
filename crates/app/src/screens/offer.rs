//! The offer for a player of another club (`a.cz`, "Proposta pelo jogador"): the amount in
//! thousands, written out as it is typed (`cz.oK()`), then `cz.oL()`: the mod's window check
//! (`Janelas.bloqueioCompra`), the cash, another human's player (he is asked), a listed player
//! at his price (`l.b`), or the negotiation (`l.a(F, ah, int)`, with the player's salary and
//! the club's counter offer asked as the original asks them).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::world::World;
use brasfoot_simulation::Sim;
use brasfoot_simulation::human::purchase::{self, ListedAnswer, OfferAnswer};
use brasfoot_systems::windows::{self, NextWindow, PurchaseBlock};
use eframe::egui;

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::form::{self, Question, Reply};
use crate::screens::{menu, widgets};
use crate::{format, glyphs, motion};

/// What the open question does when answered.
#[derive(Clone, Copy)]
enum Pending {
    /// "Proposta de compra": the other human sells on "Sim".
    SaleBetweenHumans(i32),
    /// The answer of `l.b`; the dialog closes after it when he was bought.
    Listed(bool),
    /// A message of `l.a` (0 to 5); "Compra realizada" signs him after it.
    Message(OfferAnswer, i32),
    /// "Deseja aumento de salário": the amount and the salary.
    Salary(i32, i32),
    /// "Contra-proposta": the club's price and the salary if any.
    Counter(i32, Option<i32>),
}

pub struct OfferState {
    pub player: PlayerId,
    /// `uE`: thousands.
    amount: String,
    /// `uH` after "Enviar": why the offer did not go, until the amount changes.
    feedback: Option<String>,
    question: Option<(Question, Pending)>,
    /// Development: send at the next frame.
    send_now: bool,
}

impl OfferState {
    pub fn new(_world: &World, player: PlayerId) -> Self {
        OfferState { player, amount: String::new(), feedback: None, question: None, send_now: false }
    }

    /// The amount as if typed (development: `BF_TAB=offer-<thousands>`).
    pub fn type_amount(&mut self, thousands: &str, send: bool) {
        self.amount = thousands.to_owned();
        self.send_now = send;
    }
}

pub fn window(s: &mut CareerScreen, state: &mut Option<OfferState>, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let (Some(st), Some(buyer)) = (state.as_mut(), s.career.human_club()) else {
        *open = false;
        return;
    };
    motion::dialog(ctx, "offer", open, egui::vec2(form::WIDTH, 370.0), form::frame(), |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::MARKET), &t.text("transfer-offer-title", None)) {
            *open = false;
        }
        form::player_card(ui, t, files, &s.career.world, st.player);
        ui.add_space(12.0);
        if let Some((q, pending)) = st.question.clone() {
            if let Some(reply) = form::question(ui, t, &q) {
                st.question = None;
                if answered(s, st.player, buyer, pending, reply) {
                    *open = false;
                }
            }
            return;
        }
        let x = s.career.world.player(st.player);
        form::line(ui, &t.text("transfer-offer-fee", Some(&args([("amount", format::money(t, x.market_value as i64))]))));
        ui.add_space(12.0);
        menu::field_label(ui, &t.text("transfer-offer-prompt", None));
        let before = st.amount.clone();
        form::money_field(ui, "offer-amount", &mut st.amount, None, (0, i64::MAX));
        if st.amount != before {
            st.feedback = None;
        }
        ui.add_space(4.0);
        if let Some(text) = &st.feedback {
            form::feedback(ui, text, true);
        } else if !st.amount.is_empty() {
            // cz.oK(): what was typed, written out
            match thousands(&st.amount) {
                Some(v) => form::feedback(ui, &format::money(t, v as i64), false),
                None => form::feedback(ui, &t.text("transfer-offer-invalid-value", None), true),
            }
        }
        let (cancel, send) = form::footer(ui, &t.text("transfer-offer-button-cancel", None), &t.text("transfer-offer-button-send", None));
        if cancel {
            *open = false;
        }
        if send || std::mem::take(&mut st.send_now) {
            send_offer(s, st, buyer, t);
        }
    });
}

/// The text plus "000" as an `int`, as `cz.B(String)` reads it.
fn thousands(text: &str) -> Option<i32> {
    format!("{text}000").parse::<i32>().ok().filter(|v| *v >= 0)
}

/// `cz.oL()`
fn send_offer(s: &mut CareerScreen, st: &mut OfferState, buyer: ClubId, t: &Translator) {
    let world = &s.career.world;
    if let Some(block) = windows::purchase_block(world, &s.career.runtime.clock, &s.career.systems.config) {
        let text = match block {
            PurchaseBlock::TransferBan => t.text("transfer-offer-transfer-ban", None),
            PurchaseBlock::WindowClosed(next) => {
                let when = match next {
                    NextWindow::July => t.text("transfer-offer-window-july", None),
                    NextWindow::Preseason(year) => t.text("transfer-offer-window-preseason", Some(&args([("year", year.to_string())]))),
                };
                t.text("transfer-offer-window-closed", Some(&args([("when", when)])))
            }
        };
        st.feedback = Some(text);
        return;
    }
    let Some(amount) = thousands(&st.amount).filter(|&v| v > 0) else {
        st.feedback = Some(t.text("transfer-offer-invalid-value", None));
        return;
    };
    if world.club(buyer).cash < amount as i64 {
        st.feedback = Some(t.text("transfer-offer-exceeds-cash", None));
        return;
    }
    let x = world.player(st.player);
    let seller = x.club.expect("an offer is for a player with a club");
    let buy_title = t.text("transfer-offer-buy-title", None);
    if world.club(seller).human {
        let coach = world.club(seller).coach.map(|c| world.coach(c).name.clone()).unwrap_or_default();
        let text = t.text("transfer-offer-accept-body", Some(&args([("coach", coach)])));
        st.question = Some((Question { title: t.text("transfer-offer-accept-title", None), text, yes_no: true }, Pending::SaleBetweenHumans(amount)));
        return;
    }
    if x.listed_for_sale && amount >= x.price {
        // cz.oM(): the purchase at the listed price
        let answer = sim(s).buy_listed(st.player, buyer);
        let key = match answer {
            ListedAnswer::NotForSale => "transfer-offer-result-not-for-sale",
            ListedAnswer::Bought => "transfer-offer-result-done",
            ListedAnswer::OwnPlayer => "transfer-offer-result-own-player",
            ListedAnswer::SquadLimit => "transfer-offer-result-squad-limit-32",
            ListedAnswer::Unwilling => "transfer-offer-result-unwilling",
            ListedAnswer::NoMoney => "transfer-offer-result-no-cash",
        };
        st.question = Some((Question { title: buy_title, text: t.text(key, None), yes_no: false }, Pending::Listed(answer == ListedAnswer::Bought)));
        return;
    }
    let answer = purchase::offer(&s.career.world, st.player, buyer, amount);
    let question = match answer {
        OfferAnswer::WantsSalary(salary) => {
            let text = t.text("transfer-offer-salary-body", Some(&args([("salary", format::money(t, salary as i64))])));
            (Question { title: t.text("transfer-offer-salary-title", None), text, yes_no: true }, Pending::Salary(amount, salary))
        }
        OfferAnswer::Counter { price, salary } => {
            let club = s.career.world.club(seller).name.clone();
            let price_text = format::money(t, price as i64);
            let text = match salary {
                Some(v) => t.text("transfer-offer-counter-body-salary", Some(&args([("club", club), ("price", price_text), ("salary", format::money(t, v as i64))]))),
                None => t.text("transfer-offer-counter-body", Some(&args([("club", club), ("price", price_text)]))),
            };
            (Question { title: t.text("transfer-offer-counter-title", None), text, yes_no: true }, Pending::Counter(price, salary))
        }
        _ => {
            let key = match answer {
                OfferAnswer::Accepted => "transfer-offer-result-done",
                OfferAnswer::OwnPlayer => "transfer-offer-result-own-player",
                OfferAnswer::SquadLimit => "transfer-offer-result-squad-limit-30",
                OfferAnswer::Unwilling => "transfer-offer-result-unwilling",
                OfferAnswer::NoMoney => "transfer-offer-result-no-cash",
                _ => "transfer-offer-result-refused",
            };
            (Question { title: buy_title, text: t.text(key, None), yes_no: false }, Pending::Message(answer, amount))
        }
    };
    st.question = Some(question);
}

/// The answer to the question; true when the dialog closes (`ub.dispose()`).
fn answered(s: &mut CareerScreen, p: PlayerId, buyer: ClubId, pending: Pending, reply: Reply) -> bool {
    let yes = reply == Reply::Yes;
    match pending {
        Pending::SaleBetweenHumans(fee) if yes => sim(s).complete_sale_between_humans(p, buyer, fee),
        Pending::Listed(bought) => return bought,
        // the message box closes either way, then the player signs
        Pending::Message(OfferAnswer::Accepted, fee) => sim(s).complete_purchase(p, buyer, fee, None),
        Pending::Salary(fee, salary) if yes => sim(s).complete_purchase(p, buyer, fee, Some(salary)),
        Pending::Counter(price, salary) if yes => sim(s).complete_purchase(p, buyer, price, salary),
        _ => return false,
    }
    true
}

fn sim(s: &mut CareerScreen) -> Sim<'_> {
    Sim { world: &mut s.career.world, runtime: &mut s.career.runtime, hooks: &mut s.career.systems, human: None }
}
