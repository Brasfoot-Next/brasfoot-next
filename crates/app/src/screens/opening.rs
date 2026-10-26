//! The opening screen (`a.gq`): new game, saved game and exit, in the app's own look instead of
//! the game's picture (`aiconsj/tela_inicial.jpg`). The editor is not ported yet and stays out.
//! A shortcut to the latest career of this app comes first, and the data sources (Patches) come
//! after the saved games; the original has neither.

use std::path::PathBuf;

use brasfoot_i18n::{Translator, args};
use brasfoot_persistence::native::SaveMeta;
use eframe::egui;

use crate::Navigation;
use crate::career::{Career, CareerError, own_saves};
use crate::format;
use crate::game_files::GameFiles;
use crate::screens::menu::{self, Glyph};
use crate::theme;

pub struct OpeningScreen {
    /// This app's latest career, read once when the screen opens.
    latest: Option<(PathBuf, SaveMeta)>,
    error: Option<String>,
}

impl OpeningScreen {
    pub fn new() -> Self {
        OpeningScreen { latest: own_saves().into_iter().next(), error: None }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> Navigation {
        let mut navigation = Navigation::Stay;
        menu::page(ui, 460.0, |ui| {
            let top = (ui.available_height() - 520.0).max(0.0) / 2.0;
            ui.add_space(top.max(32.0));
            menu::brand(ui, t, Some(&t.text("opening-subtitle", None)));
            ui.add_space(28.0);
            if let Some(e) = &self.error {
                menu::error(ui, e);
            }
            if let Some((path, meta)) = &self.latest {
                let detail = t.text(
                    "opening-continue-detail",
                    Some(&args([("club", meta.club.clone()), ("season", meta.season.to_string()), ("date", format::iso_date_medium(t, &meta.date))])),
                );
                if menu::row(ui, Glyph::Play, &t.text("opening-continue", None), &detail, true, true).clicked() {
                    match Career::open(path, files) {
                        Ok(c) => navigation = Navigation::OpenCareer(c),
                        Err(CareerError::Read(why)) => self.error = Some(t.text("start-error-read", Some(&args([("reason", why)])))),
                        Err(CareerError::NotPortedYet(what)) => self.error = Some(what),
                    }
                }
            }
            if menu::row(ui, Glyph::Ball, &t.text("opening-new-game", None), &t.text("opening-new-game-detail", None), self.latest.is_none(), true).clicked() {
                navigation = Navigation::NewGame;
            }
            if menu::row(ui, Glyph::Folder, &t.text("opening-saved-game", None), &t.text("opening-saved-game-detail", None), false, true).clicked() {
                navigation = Navigation::SavedGame;
            }
            if menu::row(ui, Glyph::Layers, &t.text("opening-patches", None), &t.text("opening-patches-detail", None), false, true).clicked() {
                navigation = Navigation::Patches;
            }
            #[cfg(not(target_arch = "wasm32"))]
            if menu::row(ui, Glyph::Power, &t.text("opening-exit", None), "", false, true).clicked() {
                navigation = Navigation::Exit;
            }
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                let version = t.text("opening-version", Some(&args([("version", env!("CARGO_PKG_VERSION").to_owned())])));
                ui.label(egui::RichText::new(version).font(theme::regular(11.0)).color(theme::MUTED));
            });
        });
        navigation
    }
}
