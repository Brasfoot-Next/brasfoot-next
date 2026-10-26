//! The training (`a.iQ`, "Treino"): the club's weekly focus, main or secondary skills
//! (`ah.bZ(int)`), and whether the assistant coach picks it each week (`ah.M(boolean)`). Both are
//! written at once, the original has no save button.

use brasfoot_i18n::Translator;
use brasfoot_model::ids::ClubId;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::{menu, widgets};
use crate::{motion, theme};

/// `eg.qU()`: only with individual skills, for the human's club.
pub fn available(world: &brasfoot_model::world::World, club: Option<ClubId>) -> bool {
    world.game.individual_skills && club.is_some()
}

pub fn window(s: &mut CareerScreen, club: ClubId, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "training", open, egui::vec2(460.0, 230.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::TRAINING), &t.text("training-title", None)) {
            *open = false;
        }
        let c = s.career.world.club_mut(club);
        menu::field_label(ui, &t.text("training-label-focus", None));
        ui.add_space(4.0);
        // iQ.sI(): the radio picked is the focus
        if menu::radio(ui, c.training_mode == 0, &t.text("training-radio-main", None), Some(&t.text("training-hint-main", None))) {
            c.training_mode = 0;
        }
        ui.add_space(4.0);
        if menu::radio(ui, c.training_mode != 0, &t.text("training-radio-secondary", None), Some(&t.text("training-hint-secondary", None))) {
            c.training_mode = 1;
        }
        ui.add_space(14.0);
        let mut auto = c.auto_training;
        menu::check_row(ui, &mut auto, &t.text("training-checkbox-assistant", None), true);
        c.auto_training = auto;
    });
}
