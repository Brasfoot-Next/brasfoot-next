//! The Transfer History screen (`a.hC`, "Histórico de transferências"): view all
//! transfers made across the season.

use brasfoot_i18n::Translator;
use eframe::egui::{self, Color32, Stroke};

use crate::format;
use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::widgets;
use crate::{motion, theme};

pub fn window(
    s: &mut CareerScreen,
    ctx: &egui::Context,
    t: &Translator,
    files: &GameFiles,
    open: &mut bool,
) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, Color32::BLACK)).inner_margin(12);
    motion::dialog(ctx, "history_dialog", open, egui::vec2(780.0, 520.0), frame, |ui, open| {
        body(s, ui, t, files, open);
    });
}

fn body(
    s: &mut CareerScreen,
    ui: &mut egui::Ui,
    t: &Translator,
    files: &GameFiles,
    open: &mut bool,
) {
    let world = &s.career.world;
    let transfers = &world.game.transfers;

    if widgets::dialog_title(ui, files, Some(glyphs::HISTORY), &t.text("history-title", None)) {
        *open = false;
    }

    ui.add_space(8.0);

    // Transfers Table
    egui::Frame::new()
        .fill(theme::PANEL_HIGH)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .inner_margin(6)
        .corner_radius(4)
        .show(ui, |ui| {
            if transfers.is_empty() {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(t.text("history-empty", None)).font(theme::regular(13.0)).color(theme::MUTED));
                });
                ui.add_space(20.0);
            } else {
                table_header(ui, t);
                ui.separator();

                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for tr in transfers.iter().rev() {
                        let date_str = format!("{:02}/{:02}/{}", tr.day, tr.month, tr.year);
                        let player_name = tr
                            .player
                            .map(|p| world.player(p).name.clone())
                            .unwrap_or_else(|| tr.player_number.to_string());

                        let from_name = world
                            .game
                            .clubs
                            .iter()
                            .find(|&&c| world.club(c).id == tr.from_club)
                            .map(|&c| world.club(c).name.clone())
                            .unwrap_or_else(|| "--".to_string());

                        let to_name = world
                            .game
                            .clubs
                            .iter()
                            .find(|&&c| world.club(c).id == tr.to_club)
                            .map(|&c| world.club(c).name.clone())
                            .unwrap_or_else(|| "--".to_string());

                        let fee_str = format::money(t, tr.fee as i64);

                        let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
                        if response.hovered() {
                            ui.painter().rect_filled(rect, 2.0, theme::HOVER);
                        }

                        let p = ui.painter();
                        p.text(egui::pos2(rect.left() + 10.0, rect.center().y), egui::Align2::LEFT_CENTER, date_str, theme::regular(11.0), theme::MUTED);
                        p.text(egui::pos2(rect.left() + 100.0, rect.center().y), egui::Align2::LEFT_CENTER, player_name, theme::bold(11.0), Color32::WHITE);
                        p.text(egui::pos2(rect.left() + 300.0, rect.center().y), egui::Align2::LEFT_CENTER, from_name, theme::regular(11.0), theme::TEXT);
                        p.text(egui::pos2(rect.left() + 480.0, rect.center().y), egui::Align2::LEFT_CENTER, to_name, theme::regular(11.0), theme::TEXT);
                        p.text(egui::pos2(rect.right() - 15.0, rect.center().y), egui::Align2::RIGHT_CENTER, fee_str, theme::bold(11.0), Color32::from_rgb(255, 235, 120));
                    }
                });
            }
        });

    ui.add_space(8.0);

    // Bottom action
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(t.text("history-button-close", None)).clicked() {
                *open = false;
            }
        });
    });
}

fn table_header(ui: &mut egui::Ui, t: &Translator) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
    let p = ui.painter();
    p.text(egui::pos2(rect.left() + 10.0, rect.center().y), egui::Align2::LEFT_CENTER, t.text("history-col-date", None), theme::bold(11.0), theme::MUTED);
    p.text(egui::pos2(rect.left() + 100.0, rect.center().y), egui::Align2::LEFT_CENTER, t.text("history-col-player", None), theme::bold(11.0), theme::MUTED);
    p.text(egui::pos2(rect.left() + 300.0, rect.center().y), egui::Align2::LEFT_CENTER, t.text("history-col-from", None), theme::bold(11.0), theme::MUTED);
    p.text(egui::pos2(rect.left() + 480.0, rect.center().y), egui::Align2::LEFT_CENTER, t.text("history-col-to", None), theme::bold(11.0), theme::MUTED);
    p.text(egui::pos2(rect.right() - 15.0, rect.center().y), egui::Align2::RIGHT_CENTER, t.text("history-col-fee", None), theme::bold(11.0), theme::MUTED);
}
