//! The saved games screen: careers of this app, the original game's saves to import (`a.dN`,
//! load game), or any file. Same look as the opening screen.

use std::path::PathBuf;

use brasfoot_i18n::{Translator, args};
use brasfoot_persistence::native::SaveMeta;
use eframe::egui;

use crate::Navigation;
use crate::career::{Career, CareerError, own_saves};
use crate::format;
use crate::game_files::GameFiles;
#[cfg(not(target_arch = "wasm32"))]
use crate::game_files::file_stem;
use crate::screens::menu::{self, Glyph};
use crate::theme;

pub struct StartScreen {
    /// This app's careers and the game's saves, read once when the screen opens.
    careers: Vec<(PathBuf, SaveMeta)>,
    #[cfg(not(target_arch = "wasm32"))]
    game_saves: Vec<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    path: String,
    error: Option<String>,
}

impl StartScreen {
    pub fn new(_files: &GameFiles) -> Self {
        StartScreen {
            careers: own_saves(),
            #[cfg(not(target_arch = "wasm32"))]
            game_saves: _files.saves(),
            #[cfg(not(target_arch = "wasm32"))]
            path: String::new(),
            error: None,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> Navigation {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            return Navigation::Opening;
        }
        let mut navigation = Navigation::Stay;
        let mut result: Option<Result<Career, CareerError>> = None;
        menu::page(ui, 640.0, |ui| {
            ui.add_space(20.0);
            if menu::back(ui, t) {
                navigation = Navigation::Opening;
            }
            ui.add_space(4.0);
            ui.label(egui::RichText::new(t.text("opening-saved-game", None)).font(theme::bold(28.0)));
            ui.label(egui::RichText::new(t.text("start-subtitle", None)).font(theme::regular(13.0)).color(theme::MUTED));
            ui.add_space(6.0);
            if let Some(e) = &self.error {
                ui.add_space(8.0);
                menu::error(ui, e);
            }

            menu::section(ui, &t.text("start-own-careers", None));
            if self.careers.is_empty() {
                ui.label(egui::RichText::new(t.text("start-no-careers", None)).color(theme::MUTED));
            }
            for (i, (path, meta)) in self.careers.iter().enumerate() {
                let detail = t.text(
                    "start-career-detail",
                    Some(&args([("coach", meta.coach.clone()), ("season", meta.season.to_string()), ("date", format::iso_date_medium(t, &meta.date))])),
                );
                if menu::row(ui, Glyph::Play, &meta.club, &detail, i == 0, true).clicked() {
                    result = Some(Career::open(path, files));
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                menu::section(ui, &t.text("start-import", None));
                ui.label(egui::RichText::new(t.text("start-import-hint", Some(&args([("folder", files.dir.display().to_string())])))).color(theme::MUTED));
                ui.add_space(8.0);
                for path in &self.game_saves {
                    if menu::row(ui, Glyph::Import, &file_stem(path), &t.text("start-import-detail", None), false, true).clicked() {
                        result = Some(Career::import(path, files));
                    }
                }

                menu::section(ui, &t.text("start-open-file", None));
                ui.horizontal(|ui| {
                    let edit = egui::TextEdit::singleline(&mut self.path)
                        .hint_text(t.text("start-open-file-hint", None))
                        .font(theme::regular(13.0))
                        .margin(egui::vec2(10.0, 9.0))
                        .desired_width(ui.available_width() - 96.0);
                    let response = ui.add(edit);
                    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let button = egui::Button::new(egui::RichText::new(t.text("start-open", None)).font(theme::bold(13.0)))
                        .fill(theme::ACCENT)
                        .corner_radius(8)
                        .min_size(egui::vec2(88.0, 36.0));
                    if (ui.add_enabled(!self.path.trim().is_empty(), button).clicked() || enter) && !self.path.trim().is_empty() {
                        let path = PathBuf::from(self.path.trim());
                        result = Some(if path.extension().is_some_and(|x| x == "bfn") { Career::open(&path, files) } else { Career::import(&path, files) });
                    }
                });
            }
            ui.add_space(32.0);
        });
        match result {
            Some(Ok(c)) => navigation = Navigation::OpenCareer(c),
            Some(Err(CareerError::Read(why))) => self.error = Some(t.text("start-error-read", Some(&args([("reason", why)])))),
            Some(Err(CareerError::NotPortedYet(what))) => self.error = Some(what),
            None => {}
        }
        navigation
    }
}
