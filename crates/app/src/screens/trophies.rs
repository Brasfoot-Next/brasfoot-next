//! The trophy galleries: the club's (`best.S.v` = `a.di`, a card per kind of title with the
//! trophy, how many and the years) and the coach's (`best.S.e` = `a.dg`, three columns of titles:
//! international, national, state and regional, under a row of the trophies won).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, CoachId, CompetitionId};
use brasfoot_model::world::World;
use brasfoot_simulation::trophy::{self, GENERIC};
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::widgets;
use crate::{motion, theme};

/// `a.di.W(list)`: at most 23 years, then "...".
const YEARS_SHOWN: usize = 23;

fn frame() -> egui::Frame {
    egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16)
}

/// `H() + SR.iU()`
fn year(world: &World, season: i32) -> i32 {
    season + world.game.start_year - 1
}

fn picture(world: &World, files: &GameFiles, c: CompetitionId) -> (String, Option<std::path::PathBuf>) {
    let name = trophy::trophy_picture(world, c, &|n| files.trophy(n).is_some());
    let path = files.trophy(&name);
    (name, path)
}

pub fn window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles, club: ClubId, open: &mut bool) {
    let world = &s.career.world;
    let cards = trophy::club_gallery(world, club);
    // a.di grows by rows of four cards: 219, 399, 579 or 762 high
    let rows = cards.len().div_ceil(4).max(1) as f32;
    motion::dialog(ctx, "club-trophies", open, egui::vec2(900.0, 90.0 + rows * 200.0), frame(), |ui, open| {
        let title = t.text("trophies-title-club", Some(&args([("club", world.club(club).name.clone())])));
        ui.horizontal(|ui| {
            widgets::crest(ui, files, world, club, 28.0);
            ui.add_space(4.0);
            if widgets::dialog_title(ui, files, None, &title) {
                *open = false;
            }
        });
        if cards.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| ui.label(egui::RichText::new(t.text("trophies-empty", None)).font(theme::regular(14.0)).color(theme::MUTED)));
            return;
        }
        let width = (ui.available_width() - 3.0 * 12.0) / 4.0;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for row in cards.chunks(4) {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for card in row {
                        club_card(ui, t, world, files, card, width);
                    }
                });
                ui.add_space(12.0);
            }
        });
    });
}

/// `components.k`: the trophy, the competition, the years and the count.
fn club_card(ui: &mut egui::Ui, t: &Translator, world: &World, files: &GameFiles, card: &trophy::GalleryCard, width: f32) {
    egui::Frame::new().fill(theme::CARD).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(10).inner_margin(10).show(ui, |ui| {
        ui.set_width(width - 20.0);
        ui.set_height(168.0);
        // the row of cards is horizontal; the card itself goes down
        ui.vertical(|ui| {
        ui.horizontal_top(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 96.0), egui::Sense::hover());
            match card.competition.map(|c| picture(world, files, c)) {
                Some((_, Some(path))) => {
                    ui.put(rect, egui::Image::new(format!("file://{}", path.display())).max_size(rect.size()));
                }
                _ => widgets::paint_jar_image(ui, files, "aicons/trgen.png", rect),
            }
            ui.vertical(|ui| {
                let name = card.competition.map(|c| widgets::competition_name(t, world, c)).unwrap_or_default();
                ui.label(egui::RichText::new(name).font(theme::bold(13.0)));
                ui.add_space(4.0);
                let (badge, _) = ui.allocate_exact_size(egui::vec2(34.0, 24.0), egui::Sense::hover());
                ui.painter().rect_filled(badge, 6.0, theme::ACCENT);
                ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, card.count.to_string(), theme::bold(14.0), egui::Color32::WHITE);
            });
        });
        ui.add_space(6.0);
        let mut years: Vec<String> = card.seasons.iter().take(YEARS_SHOWN).map(|&s| year(world, s).to_string()).collect();
        if card.seasons.len() > YEARS_SHOWN {
            years.push(t.text("trophies-more", None));
        }
        ui.label(egui::RichText::new(years.join(", ")).font(theme::regular(11.0)).color(theme::MUTED));
        });
    });
}

pub fn coach_window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles, coach: CoachId, open: &mut bool) {
    let world = &s.career.world;
    let columns = trophy::coach_gallery(world, coach);
    motion::dialog(ctx, "coach-trophies", open, egui::vec2(980.0, 660.0), frame(), |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::TROPHY), &t.text("coach-trophy-gallery-title", Some(&args([("coach", world.coach(coach).name.clone())])))) {
            *open = false;
        }
        // a.dg.pz(): one trophy of each kind won, at most thirteen, the generic ones left out
        let mut shown: Vec<(String, std::path::PathBuf)> = Vec::new();
        for title in columns.iter().flatten() {
            let (name, path) = picture(world, files, title.competition);
            if let Some(path) = path
                && !GENERIC.contains(&name.as_str())
                && !shown.iter().any(|(n, _)| *n == name)
                && shown.len() < 13
            {
                shown.push((name, path));
            }
        }
        if !shown.is_empty() {
            ui.horizontal(|ui| {
                for (_, path) in &shown {
                    ui.add(egui::Image::new(format!("file://{}", path.display())).fit_to_exact_size(egui::vec2(43.0, 60.0)));
                    ui.add_space(6.0);
                }
            });
            ui.add_space(10.0);
        }
        let headers = [
            t.text("coach-trophy-gallery-column-international", None),
            t.text("coach-trophy-gallery-column-national", None),
            if world.game.plays_regionals { t.text("coach-trophy-gallery-column-state-regional", None) } else { t.text("coach-trophy-gallery-column-state", None) },
        ];
        ui.columns(3, |cols| {
            for (i, ui) in cols.iter_mut().enumerate() {
                egui::Frame::new().fill(theme::CARD).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(10).inner_margin(10).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(ui.available_height() - 24.0);
                    ui.label(egui::RichText::new(&headers[i]).font(theme::bold(14.0)));
                    ui.add_space(6.0);
                    if columns[i].is_empty() {
                        ui.label(egui::RichText::new(t.text("coach-trophy-gallery-none", None)).color(theme::MUTED));
                    }
                    egui::ScrollArea::vertical().id_salt(("coach-trophies", i)).auto_shrink([false, false]).show(ui, |ui| {
                        for title in &columns[i] {
                            coach_line(ui, t, world, files, title);
                        }
                    });
                });
            }
        });
    });
}

/// `components.aC`: the trophy and competition, the club with its crest, the year.
fn coach_line(ui: &mut egui::Ui, t: &Translator, world: &World, files: &GameFiles, title: &trophy::CoachTitle) {
    ui.horizontal(|ui| {
        match picture(world, files, title.competition).1 {
            Some(path) => {
                ui.add(egui::Image::new(format!("file://{}", path.display())).fit_to_exact_size(egui::vec2(18.0, 18.0)));
            }
            None => ui.add_space(18.0),
        }
        ui.label(egui::RichText::new(widgets::competition_name(t, world, title.competition)).font(theme::regular(12.0)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(year(world, title.season).to_string()).font(theme::bold(12.0)));
        });
    });
    if let Some(club) = title.club {
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            widgets::crest(ui, files, world, club, 16.0);
            ui.label(egui::RichText::new(&world.club(club).name).font(theme::regular(11.0)).color(theme::MUTED));
        });
    }
    ui.add_space(6.0);
}
