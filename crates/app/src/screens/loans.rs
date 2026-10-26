//! The loans (`a.bs`, "Jogadores emprestados"): the players on loan at the club (`SR.e(ah)`,
//! up to 4) and the club's players on loan elsewhere (`SR.f(ah)`, up to 10). Cancelling a loan
//! (`SR.d(F)`) is not ported yet.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::widgets;
use crate::{format, motion, theme};

/// `bs.An`, `bs.Ao`: the fixed limits.
const BORROWED_MAX: usize = 4;
const LENT_MAX: usize = 10;

/// A loan as the lists show it: the player, the other club, the end.
struct Row {
    player: PlayerId,
    club: Option<ClubId>,
    until: i64,
}

fn rows(world: &World, club: ClubId) -> (Vec<Row>, Vec<Row>) {
    let mut borrowed = Vec::new();
    let mut lent = Vec::new();
    for l in &world.game.loans {
        let Some(p) = l.player else { continue };
        let playing_at = world.player(p).club;
        if playing_at == Some(club) {
            borrowed.push(Row { player: p, club: l.owner, until: l.until });
        } else if l.owner == Some(club) {
            lent.push(Row { player: p, club: playing_at, until: l.until });
        }
    }
    (borrowed, lent)
}

pub fn window(s: &mut CareerScreen, club: ClubId, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let mut info = None;
    motion::dialog(ctx, "loans", open, egui::vec2(780.0, 620.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::LOANS), &t.text("loans-panel-tooltip-button", None)) {
            *open = false;
        }
        let world = &s.career.world;
        let clock = &s.career.runtime.clock;
        let (borrowed, lent) = rows(world, club);
        for (title, count_key, list, max, owner_header) in [
            ("loans-panel-title-borrowed", "loans-panel-slots", &borrowed, BORROWED_MAX, "loans-panel-header-owner"),
            ("loans-panel-title-lent", "loans-panel-limit", &lent, LENT_MAX, "loans-panel-header-playing-at"),
        ] {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t.text(title, None)).font(theme::bold(15.0)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(t.text(count_key, Some(&args([("used", list.len()), ("max", max)])))).font(theme::bold(12.0)).color(theme::STAR));
                });
            });
            ui.add_space(6.0);
            let width = ui.available_width();
            let columns = [(t.text("loans-panel-header-player", None), 0.36), (t.text(owner_header, None), 0.32), (t.text("loans-panel-header-end-date", None), 0.20)];
            let (header, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
            ui.painter().rect_filled(header, 6.0, theme::HEADER);
            let mut x = header.left();
            for (text, share) in &columns {
                ui.painter().text(egui::pos2(x + 8.0, header.center().y), egui::Align2::LEFT_CENTER, text, theme::bold(11.0), theme::MUTED);
                x += width * share;
            }
            ui.add_space(2.0);
            if list.is_empty() {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(t.text("loans-panel-none", None)).color(theme::MUTED));
            }
            for r in list.iter() {
                let x = world.player(r.player);
                ui.horizontal(|ui| {
                    ui.set_height(36.0);
                    let start = ui.cursor().left();
                    ui.allocate_ui(egui::vec2(width * 0.36, 30.0), |ui| {
                        ui.horizontal(|ui| {
                            widgets::paint_position_badge(ui.painter(), t, ui.cursor().left_center() + egui::vec2(12.0, 0.0), x.position);
                            ui.add_space(28.0);
                            ui.label(egui::RichText::new(&x.name).font(theme::bold(13.0)));
                        });
                    });
                    ui.add_space(start + width * 0.36 - ui.cursor().left());
                    ui.allocate_ui(egui::vec2(width * 0.32, 30.0), |ui| {
                        ui.horizontal(|ui| {
                            if let Some(c) = r.club {
                                widgets::crest(ui, files, world, c, 20.0);
                                ui.label(egui::RichText::new(&world.club(c).name).font(theme::regular(13.0)));
                            }
                        });
                    });
                    ui.add_space(start + width * 0.68 - ui.cursor().left());
                    ui.label(egui::RichText::new(format::date(t, clock, r.until)).font(theme::regular(13.0)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // the cancel button (r_eli): not ported yet
                        widgets::icon_button(ui, glyphs::ELIMINATED, 16.0, theme::TEXT_RED, false).on_hover_text(t.text("toolbar-not-ported", None));
                        // the info button: the player's card
                        if widgets::icon_button(ui, glyphs::CAREER, 16.0, theme::ACCENT, true).on_hover_text(t.text("teams-tab-career", None)).clicked() {
                            info = Some(r.player);
                        }
                    });
                });
            }
            ui.add_space(18.0);
        }
    });
    if info.is_some() {
        s.open_player = info;
    }
}
