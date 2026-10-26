//! Retire a player (`a.ji`, "Aposentar jogador"): the player leaves now and a youth player may
//! take his place, with the name and the position the human picks (`ji.qW()`).

use brasfoot_i18n::Translator;
use brasfoot_model::ids::PlayerId;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::{menu, widgets};
use crate::{motion, theme};

pub struct RetireState {
    pub player: PlayerId,
    /// `Nr`
    name: String,
    /// `Nq`: a position 0..4, or 5 "Aleatório".
    position: usize,
}

impl RetireState {
    /// `new ji(dialog, club, player)`: the player's own position picked.
    pub fn new(world: &brasfoot_model::world::World, player: PlayerId) -> Self {
        RetireState { player, name: String::new(), position: world.player(player).position.clamp(0, 4) as usize }
    }
}

pub fn window(s: &mut CareerScreen, state: &mut RetireState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let mut retire = false;
    motion::dialog(ctx, "retire", open, egui::vec2(460.0, 420.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::RETIRE), &t.text("retire-title", None)) {
            *open = false;
        }
        let world = &s.career.world;
        ui.label(egui::RichText::new(&world.player(state.player).name).font(theme::bold(16.0)).color(theme::STAR));
        ui.add_space(8.0);
        ui.label(egui::RichText::new(t.text("retire-explanation", None)).font(theme::regular(13.0)));
        ui.add_space(4.0);
        ui.label(egui::RichText::new(t.text("retire-no-room", None)).font(theme::regular(12.0)).color(theme::MUTED));
        ui.add_space(14.0);
        menu::field_label(ui, &t.text("retire-label-name", None));
        ui.add(egui::TextEdit::singleline(&mut state.name).hint_text(t.text("retire-name-hint", None)).desired_width(ui.available_width()).char_limit(40).margin(egui::vec2(8.0, 6.0)));
        ui.add_space(10.0);
        menu::field_label(ui, &t.text("retire-label-position", None));
        let positions: Vec<String> = (0..5).map(|p| widgets::position_name(t, p)).chain([t.text("retire-position-random", None)]).collect();
        egui::ComboBox::from_id_salt("retire-position").width(200.0).selected_text(positions[state.position].clone()).show_ui(ui, |ui| {
            for (i, p) in positions.iter().enumerate() {
                ui.selectable_value(&mut state.position, i, p);
            }
        });
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if menu::action_button(ui, &t.text("retire-button", None), true, true).clicked() {
                retire = true;
                *open = false;
            }
        });
    });
    if retire {
        // ji.qW(): a name of 2 to 34 letters, else the game picks one; "Aleatório" is -1
        let name = state.name.trim().to_owned();
        let length = name.chars().count();
        let name = (length > 1 && length < 35).then_some(name);
        let position = if state.position < 5 { state.position as i32 } else { -1 };
        if s.career.retire_player(state.player, name, position) {
            if s.selected_player == Some(state.player) {
                s.selected_player = None;
            }
            s.status = Some((t.text("retire-done", None), theme::GOOD));
        }
    }
}
