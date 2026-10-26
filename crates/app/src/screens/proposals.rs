//! The mod's proposals that wait for the player's answer: the three master sponsor offers
//! (`Patrocinio.escolher`, "Propostas de patrocínio", closed only by signing one) and an
//! investor's offer to buy the club (`Investidor`, "Proposta de compra do clube"). Both open by
//! themselves on the main screen while the system keeps them waiting.

use brasfoot_i18n::{Translator, args, labels};
use eframe::egui::{self, Color32, vec2};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::modkit::{self, GOLD};
use crate::{format, motion, theme};
use crate::screens::controls;

/// The master sponsor offers, while there are any.
pub fn master_window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, _files: &GameFiles) {
    let offers = s.career.world.mods.master_sponsor.offers.clone();
    let mut open = !offers.is_empty() && s.career.human_club().is_some();
    if !open {
        return;
    }
    let club = s.career.human_club().map(|c| s.career.world.club(c).name.clone()).unwrap_or_default();
    let mut signed = None;
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(16);
    // no Esc: the original closes only by signing
    let mut keep = true;
    motion::dialog(ctx, "master-offers", &mut keep, vec2(820.0, 330.0), frame, |ui, _| {
        modkit::title(ui, &t.text("proposals-master-title", None), 15.0);
        modkit::text(ui, &t.text("proposals-master-intro", Some(&args([("club", club.clone())]))), 12.0, theme::MUTED);
        ui.add_space(10.0);
        ui.columns(3, |cols| {
            for (i, k) in offers.iter().enumerate() {
                modkit::item(None, None).inner_margin(12).show(&mut cols[i], |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(egui::RichText::new(labels::master_kind(t, k.kind).to_uppercase()).font(theme::bold(13.0)).color(GOLD));
                    ui.label(egui::RichText::new(brasfoot_model::brands::MASTER_BRANDS[k.brand as usize]).font(theme::bold(16.0)).color(Color32::WHITE));
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(t.text("proposals-master-yearly", Some(&args([("amount", format::mi(t, k.yearly))])))).font(theme::bold(14.0)));
                    ui.label(egui::RichText::new(t.text("proposals-master-years", Some(&args([("years", k.years as i64)])))).font(theme::regular(12.0)).color(theme::MUTED));
                    ui.add_space(4.0);
                    if k.bonus_top4 > 0 || k.bonus_league > 0 || k.bonus_continental > 0 {
                        for (key, v) in [("proposals-master-bonus-top4", k.bonus_top4), ("proposals-master-bonus-league", k.bonus_league), ("proposals-master-bonus-continental", k.bonus_continental)] {
                            ui.label(egui::RichText::new(t.text(key, Some(&args([("amount", format::mi(t, v))])))).font(theme::regular(12.0)));
                        }
                    } else {
                        ui.label(egui::RichText::new(t.text("proposals-master-no-bonus", None)).font(theme::regular(12.0)));
                    }
                    ui.add_space(8.0);
                    if controls::button(ui, &t.text("proposals-master-sign", None), controls::Kind::Primary, true).clicked() {
                        signed = Some(i);
                    }
                });
            }
        });
    });
    if let Some(i) = signed {
        let career = &mut s.career;
        career.systems.with(&mut career.world, &mut career.runtime, |m| m.sign_master_sponsor(i));
        open = false;
    }
    let _ = open;
}

/// The investor's offer, while it waits.
pub fn investor_window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, _files: &GameFiles) {
    let Some(offer) = s.career.world.mods.investor.shown.clone() else { return };
    let Some(me) = s.career.human_club() else { return };
    let club = s.career.world.club(me).name.clone();
    let investor = brasfoot_model::brands::owner(offer.investor);
    let mut answer = None;
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin { left: 18, right: 18, top: 16, bottom: 12 });
    let mut keep = true;
    motion::dialog(ctx, "investor-offer", &mut keep, vec2(600.0, 310.0), frame, |ui, _| {
        modkit::title(ui, &t.text("proposals-investor-title", None), 15.0);
        modkit::text(ui, &t.text("proposals-investor-intro", Some(&args([("investor", investor.clone()), ("club", club.clone())]))), 12.0, theme::MUTED);
        ui.add_space(12.0);
        modkit::item(None, None).inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(&investor).font(theme::bold(16.0)).color(Color32::WHITE));
            ui.add_space(4.0);
            ui.label(egui::RichText::new(t.text("proposals-investor-now", Some(&args([("amount", format::mi(t, offer.now))])))).font(theme::bold(14.0)));
            ui.label(egui::RichText::new(t.text("proposals-investor-yearly", Some(&args([("amount", format::mi(t, offer.yearly))])))).font(theme::bold(14.0)));
            ui.add_space(6.0);
            for key in ["proposals-investor-note-1", "proposals-investor-note-2", "proposals-investor-note-3"] {
                ui.label(egui::RichText::new(t.text(key, None)).font(theme::regular(12.0)).color(theme::MUTED));
            }
        });
        ui.add_space(12.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if controls::button(ui, &t.text("proposals-investor-accept", None), controls::Kind::Primary, true).clicked() {
                answer = Some(true);
            }
            if controls::button(ui, &t.text("proposals-investor-refuse", None), controls::Kind::Secondary, true).clicked() {
                answer = Some(false);
            }
        });
    });
    if let Some(accept) = answer {
        let career = &mut s.career;
        career.systems.with(&mut career.world, &mut career.runtime, |m| if accept { m.accept_investor() } else { m.refuse_investor() });
    }
}
