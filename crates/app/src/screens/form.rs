//! The pieces of the game's small form dialogs (an offer, a new contract): the player they are
//! about, label and value lines, a money field, and the question the original asks in a
//! `JOptionPane`, shown inside the dialog.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::PlayerId;
use brasfoot_model::world::World;
use eframe::egui::{self, Color32, vec2};

use crate::game_files::GameFiles;
use crate::screens::widgets;
use crate::theme;
use crate::screens::controls;

/// The width of a form dialog.
pub const WIDTH: f32 = 420.0;

/// The dialog's frame.
pub fn frame() -> egui::Frame {
    egui::Frame::new().fill(theme::PANEL).inner_margin(18)
}

/// The player the form is about: his club's crest, his name, position, age and strength.
pub fn player_card(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, p: PlayerId) {
    let x = world.player(p);
    egui::Frame::new().fill(theme::CARD).corner_radius(10).inner_margin(12).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            if let Some(club) = x.club {
                widgets::crest(ui, files, world, club, 40.0);
                ui.add_space(8.0);
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.label(egui::RichText::new(&x.name).font(theme::bold(17.0)).color(Color32::WHITE));
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(22.0, 18.0), egui::Sense::hover());
                    widgets::paint_position_badge(ui.painter(), t, rect.center(), x.position);
                    ui.add_space(2.0);
                    let age = t.text("teams-player-age", Some(&args([("age", x.age.to_string())])));
                    let strength = t.text("teams-player-strength", Some(&args([("strength", x.strength.to_string())])));
                    ui.label(egui::RichText::new(age).font(theme::regular(12.0)).color(theme::MUTED));
                    ui.label(egui::RichText::new("·").font(theme::regular(12.0)).color(theme::MUTED));
                    ui.label(egui::RichText::new(strength).font(theme::bold(12.0)).color(theme::TEXT));
                });
                if let Some(club) = x.club {
                    ui.label(egui::RichText::new(&world.club(club).name).font(theme::regular(12.0)).color(theme::MUTED));
                }
            });
        });
    });
}

/// A line of the form: the text as the original writes it, "Label: value".
pub fn line(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(theme::regular(13.0)).color(theme::TEXT));
}

/// A money field in units of `unit` (1000 for the original's "em milhares"), with − and + that
/// step it by `step`; the digits only.
pub fn money_field(ui: &mut egui::Ui, id: &str, text: &mut String, step: Option<i64>, bounds: (i64, i64)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let stepper = |ui: &mut egui::Ui, glyph: &str| crate::screens::controls::stepper(ui, glyph, true).clicked();
        if let Some(step) = step
            && stepper(ui, "−")
        {
            let v = text.trim().parse::<i64>().unwrap_or(bounds.0);
            *text = (v - step).clamp(bounds.0, bounds.1).to_string();
        }
        let edit = egui::TextEdit::singleline(text)
            .id_salt(id)
            .font(theme::bold(15.0))
            .desired_width(if step.is_some() { ui.available_width() - 40.0 } else { ui.available_width() })
            .margin(vec2(10.0, 8.0));
        let response = ui.add(edit);
        if response.changed() {
            text.retain(|c| c.is_ascii_digit());
        }
        if let Some(step) = step
            && stepper(ui, "+")
        {
            let v = text.trim().parse::<i64>().unwrap_or(bounds.0);
            *text = (v + step).clamp(bounds.0, bounds.1).to_string();
        }
    });
}

/// The original's feedback label under a field, in red when it is a refusal.
pub fn feedback(ui: &mut egui::Ui, text: &str, problem: bool) {
    let color = if problem { theme::TEXT_RED } else { theme::TEXT };
    ui.add(egui::Label::new(egui::RichText::new(text).font(theme::bold(13.0)).color(color)).wrap());
}

/// The buttons of the form at the dialog's bottom right: the secondary then the main one.
/// Returns (secondary clicked, main clicked).
pub fn footer(ui: &mut egui::Ui, secondary: &str, main: &str) -> (bool, bool) {
    let mut clicked = (false, false);
    let rest = ui.available_height() - 36.0;
    if rest > 0.0 {
        ui.add_space(rest);
    }
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        clicked.1 = controls::button(ui, main, controls::Kind::Primary, true).clicked();
        ui.add_space(4.0);
        clicked.0 = controls::button(ui, secondary, controls::Kind::Secondary, true).clicked();
    });
    clicked
}

/// A question or a message of the original's `JOptionPane`, inside the dialog.
#[derive(Clone, Debug)]
pub struct Question {
    pub title: String,
    /// The message; a line break of the original is a new line here.
    pub text: String,
    /// "Sim" and "Não" (`showConfirmDialog`), or "OK" alone (`showMessageDialog`).
    pub yes_no: bool,
}

/// What the human answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    Yes,
    No,
    Ok,
}

/// The question in place of the form: a card with its title and text, and its buttons at the
/// bottom. `None` until a button is clicked.
pub fn question(ui: &mut egui::Ui, t: &Translator, q: &Question) -> Option<Reply> {
    let accent = if q.yes_no { theme::WARN } else { theme::ACCENT };
    egui::Frame::new().fill(theme::CARD).corner_radius(10).inner_margin(egui::Margin { left: 16, right: 14, top: 12, bottom: 12 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let top = ui.cursor().min;
        ui.label(egui::RichText::new(&q.title).font(theme::bold(14.0)).color(accent));
        ui.add_space(6.0);
        for line in q.text.lines() {
            ui.add(egui::Label::new(egui::RichText::new(line).font(theme::regular(13.0)).color(theme::TEXT)).wrap());
        }
        let bottom = ui.cursor().min.y;
        // the accent strip on the card's left
        ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(top.x - 10.0, top.y), egui::pos2(top.x - 7.0, bottom)), 2.0, accent);
    });
    let rest = ui.available_height() - 36.0;
    if rest > 0.0 {
        ui.add_space(rest);
    }
    let mut reply = None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if q.yes_no {
            if controls::button(ui, &t.text("dialog-yes", None), controls::Kind::Primary, true).clicked() {
                reply = Some(Reply::Yes);
            }
            ui.add_space(4.0);
            if controls::button(ui, &t.text("dialog-no", None), controls::Kind::Secondary, true).clicked() {
                reply = Some(Reply::No);
            }
        } else if controls::button(ui, &t.text("dialog-ok", None), controls::Kind::Primary, true).clicked() {
            reply = Some(Reply::Ok);
        }
    });
    reply
}
