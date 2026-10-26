//! The game's offer screen (`a.jm`, "Oferta de compra" / "Proposta de empréstimo") for the
//! offers and loan requests the mod's windows make for the human squad
//! (`Mercado.mostrarPropostas`), one after another. Selling or lending rebuilds the main screen,
//! which runs the mod's clock again (`best.S.aY(2)`).

use brasfoot_i18n::{Translator, args};
use eframe::egui;

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::{form, widgets};
use crate::{format, glyphs, motion};

/// The first offer still waiting, if any, as a dialog.
pub fn window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles) {
    // Mercado.mostrar: an offer for a player no longer at a human club is not shown
    while let Some(&offer) = s.career.systems.squad_offers.first() {
        let shown = s.career.world.player(offer.player).club.is_some_and(|c| brasfoot_systems::economy::is_human(&s.career.world, c));
        if shown {
            break;
        }
        s.career.systems.squad_offers.remove(0);
    }
    let Some(&offer) = s.career.systems.squad_offers.first() else { return };
    let mut open = true;
    let mut answer = None;
    motion::dialog(ctx, "squad-offer", &mut open, egui::vec2(form::WIDTH, 400.0), form::frame(), |ui, open| {
        let title = if offer.loan { "squad-offer-title-loan" } else { "squad-offer-title-sale" };
        if widgets::dialog_title(ui, files, Some(glyphs::MARKET), &t.text(title, None)) {
            *open = false;
        }
        let world = &s.career.world;
        let club = world.club(offer.buyer).name.clone();
        let amount = format::money(t, offer.value);
        let line = match (offer.loan, offer.value > 0) {
            (false, _) => t.text("squad-offer-sale", Some(&args([("club", club), ("amount", amount)]))),
            (true, true) => t.text("squad-offer-loan-fee", Some(&args([("club", club), ("amount", amount)]))),
            (true, false) => t.text("squad-offer-loan", Some(&args([("club", club)]))),
        };
        form::line(ui, &line);
        ui.add_space(8.0);
        form::player_card(ui, t, files, world, offer.player);
        ui.add_space(12.0);
        let main = if offer.loan { "squad-offer-button-accept" } else { "squad-offer-button-sell" };
        let (refuse, accept) = form::footer(ui, &t.text("squad-offer-button-refuse", None), &t.text(main, None));
        if refuse {
            answer = Some(false);
        }
        if accept {
            answer = Some(true);
        }
    });
    if !open && answer.is_none() {
        answer = Some(false);
    }
    if let Some(accept) = answer {
        s.career.systems.squad_offers.remove(0);
        s.career.answer_squad_offer(offer, accept);
    }
}
