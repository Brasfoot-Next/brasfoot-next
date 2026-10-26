//! The message center (`a.bn`, "Central de Mensagens"): the coach's messages, newest first
//! (`b.u`, the mod's `b.ar` renderer), the one picked on the right with the board's question
//! when it may fine the player.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::CoachId;
use brasfoot_model::saved::Message;
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::{menu, widgets};
use crate::{format, motion, theme};

/// `components.as.vm()` of the mod's newspaper messages (`b.ar`, `bfmod.Jornal`).
const NEWSPAPER: i32 = 77;

#[derive(Default)]
pub struct MessagesState {
    /// The index in the coach's list of the message shown (`bn.zX`).
    selected: Option<usize>,
    /// "Multado!" after a fine, for the message it was given on.
    fined: Option<usize>,
}

/// `best.ar.tP[vm()] + " " + vp()`: the subject column.
fn subject(t: &Translator, world: &World, m: &Message) -> String {
    let text1 = text1(t, world, m);
    if m.sender == NEWSPAPER {
        return t.text("message-center-newspaper", Some(&args([("text", text1)])));
    }
    let base = t.text(&format!("message-subject-{}", m.sender), None);
    if text1.is_empty() { base } else { t.text("message-center-subject", Some(&args([("subject", base), ("text", text1)]))) }
}

/// `vp()`, composed by the screens for the messages the port keeps as data (the stage or the
/// competition of a prize).
fn text1(t: &Translator, world: &World, m: &Message) -> String {
    if let Some(stage) = m.stage {
        widgets::stage_name(t, world, stage)
    } else if let Some(c) = m.competition {
        widgets::competition_name(t, world, c)
    } else {
        m.text1.clone()
    }
}

/// `vq()`, with the prize the port keeps as a number and the continental cup it keeps as a
/// confederation.
fn text2(t: &Translator, m: &Message) -> String {
    match (m.money, m.confederation) {
        (Some(money), _) => format::money(t, money),
        (None, Some(confederation)) => t.text(&format!("message-continental-title-{confederation}"), None),
        (None, None) => m.text2.clone(),
    }
}

/// `bn.a(as)`: the text, `$1` and `$2` filled in for the kinds that have them, else followed by
/// `vq()`.
fn body(t: &Translator, world: &World, m: &Message) -> String {
    let (p1, p2) = (text2(t, m), text1(t, world, m));
    let key = format!("message-body-{}", m.message_id);
    match m.sender {
        24 | 28 | 29 | 34 => t.text(&key, Some(&args([("p1", p1), ("p2", String::new())]))),
        26 => t.text(&key, Some(&args([("p1", p1), ("p2", p2)]))),
        _ => {
            let text = if m.sender == NEWSPAPER { String::new() } else { t.text(&key, Some(&args([("p1", String::new()), ("p2", String::new())]))) };
            if p1.is_empty() { text } else { format!("{text} {p1}") }
        }
    }
}

fn date(s: &CareerScreen, t: &Translator, m: &Message) -> String {
    match m.day {
        Some(day) => format::date(t, &s.career.runtime.clock, day),
        None => m.date_text.clone(),
    }
}

pub fn window(s: &mut CareerScreen, state: &mut MessagesState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let Some(club) = s.career.human_club() else {
        *open = false;
        return;
    };
    let Some(coach) = s.career.world.club(club).coach else {
        *open = false;
        return;
    };
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "messages", open, egui::vec2(940.0, 560.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::MESSAGES), &t.text("message-center-title", None)) {
            *open = false;
        }
        let count = s.career.world.coach(coach).messages.len();
        if count == 0 {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| ui.label(egui::RichText::new(t.text("message-center-empty", None)).font(theme::regular(14.0)).color(theme::MUTED)));
            return;
        }
        // the first row (the newest) is picked when the window opens
        let selected = *state.selected.get_or_insert(count - 1);
        if state.selected == Some(selected) && !s.career.world.coach(coach).messages[selected].read {
            s.career.read_message(coach, selected);
        }
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(380.0);
                list(s, state, ui, t, coach, height);
            });
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                if let Some(i) = state.selected {
                    shown(s, state, ui, t, coach, i, height);
                }
            });
        });
    });
}

/// `bn.Ad`: date and subject, newest first; the unread ones in bold with a dot.
fn list(s: &mut CareerScreen, state: &mut MessagesState, ui: &mut egui::Ui, t: &Translator, coach: CoachId, height: f32) {
    menu::panel().inner_margin(8).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::vertical().id_salt("messages-list").auto_shrink([false, false]).max_height(height - 20.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let count = s.career.world.coach(coach).messages.len();
            for i in (0..count).rev() {
                let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 46.0), egui::Sense::click());
                if response.clicked() {
                    state.selected = Some(i);
                    s.career.read_message(coach, i);
                }
                let m = &s.career.world.coach(coach).messages[i];
                let hover = motion::toward(ui.ctx(), response.id, response.hovered(), motion::HOVER);
                let fill = if state.selected == Some(i) { theme::SELECTION } else { motion::mix(egui::Color32::TRANSPARENT, theme::HOVER, hover) };
                let p = ui.painter();
                p.rect_filled(rect, 6.0, fill);
                if !m.read {
                    p.circle_filled(egui::pos2(rect.left() + 10.0, rect.top() + 15.0), 4.0, theme::ACCENT);
                }
                let font = if m.read { theme::regular(13.0) } else { theme::bold(13.0) };
                let room = egui::Rect::from_x_y_ranges(rect.left() + 22.0..=rect.right() - 8.0, rect.y_range());
                let mut job = egui::text::LayoutJob::simple_singleline(subject(t, &s.career.world, m), font, theme::TEXT);
                job.wrap = egui::text::TextWrapping::truncate_at_width(room.width());
                let galley = p.layout_job(job);
                p.galley(egui::pos2(room.left(), rect.top() + 6.0), galley, theme::TEXT);
                p.text(egui::pos2(room.left(), rect.bottom() - 12.0), egui::Align2::LEFT_CENTER, date(s, t, m), theme::regular(11.0), theme::MUTED);
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
        });
    });
}

/// `bn.vd`: the message, and the question with "Multar" and "Não Multar" (`bn.Aa`, `zZ`).
fn shown(s: &mut CareerScreen, state: &mut MessagesState, ui: &mut egui::Ui, t: &Translator, coach: CoachId, i: usize, height: f32) {
    let (title, when, text, question) = {
        let world = &s.career.world;
        let m = &world.coach(coach).messages[i];
        (subject(t, world, m), date(s, t, m), body(t, world, m), m.flag_rz)
    };
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(height - 34.0);
        ui.label(egui::RichText::new(title).font(theme::bold(17.0)));
        ui.label(egui::RichText::new(when).font(theme::regular(12.0)).color(theme::MUTED));
        ui.add_space(12.0);
        ui.label(egui::RichText::new(text).font(theme::regular(14.0)).line_height(Some(21.0)));
        ui.add_space(16.0);
        if question {
            ui.label(egui::RichText::new(t.text("message-center-fine-question", None)).font(theme::bold(13.0)));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let club = s.career.human_club();
                if menu::primary_button(ui, &t.text("message-center-button-fine", None), true).clicked()
                    && let Some(club) = club
                    && s.career.answer_fine(club, coach, i, true)
                {
                    state.fined = Some(i);
                }
                ui.add_space(8.0);
                if menu::secondary_button(ui, &t.text("message-center-button-dont-fine", None), true).clicked()
                    && let Some(club) = club
                {
                    s.career.answer_fine(club, coach, i, false);
                }
            });
        } else if state.fined == Some(i) {
            ui.label(egui::RichText::new(t.text("message-center-fined", None)).font(theme::bold(13.0)).color(theme::GOOD));
        }
    });
}
