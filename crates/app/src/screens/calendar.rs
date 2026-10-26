//! The club's matches (`a.L`, "Jogos do seu time"): date, competition, venue, opponent, score.

use brasfoot_i18n::Translator;
use eframe::egui::{self, Color32, Stroke};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::widgets::{self, crest};
use crate::{format, motion, theme, view};

pub fn window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, Color32::BLACK)).inner_margin(12);
    motion::dialog(ctx, "calendar", open, egui::vec2(736.0, 576.0), frame, |ui, open| {
            if widgets::dialog_title(ui, files, None, &t.text("calendar-title", None)) {
                *open = false;
            }
            ui.add_space(8.0);
            let world = &s.career.world;
            ui.spacing_mut().item_spacing.y = 0.0;
            let header = [
                (t.text("calendar-column-date", None), 90.0),
                (t.text("calendar-column-competition", None), 230.0),
                (t.text("calendar-column-venue", None), 60.0),
                (t.text("calendar-column-opponent", None), 290.0),
                (t.text("calendar-column-score", None), 60.0),
            ];
            let (rect, _) = ui.allocate_exact_size(egui::vec2(736.0, 26.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
            let mut x = rect.left() + 6.0;
            for (text, w) in &header {
                ui.painter().text(egui::pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, text, theme::bold(12.0), theme::TEXT);
                x += w;
            }
            egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                for (i, m) in view::club_matches(world, s.club).into_iter().enumerate() {
                    let g = world.game_match(m);
                    let home = g.home == Some(s.club);
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(736.0, 26.0), egui::Sense::hover());
                    let hover = motion::toward(ui.ctx(), response.id, response.hovered(), motion::HOVER);
                    let base = if i % 2 == 0 { theme::PANEL } else { theme::ROW_ALT };
                    ui.painter().rect_filled(rect, 0.0, motion::mix(base, theme::HOVER, hover));
                    let mut x = rect.left() + 6.0;
                    let y = rect.center().y;
                    let date = world.game.days.get(g.day as usize).map(|d| d.date).unwrap_or(0);
                    ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_CENTER, format::date(t, &s.career.runtime.clock, date), theme::regular(12.0), theme::TEXT);
                    x += header[0].1;
                    if let Some(c) = g.competition {
                        ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_CENTER, widgets::competition_name(t, world, c), theme::regular(12.0), theme::TEXT);
                    }
                    x += header[1].1;
                    ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_CENTER, t.text(if home { "calendar-home" } else { "calendar-away" }, None), theme::regular(12.0), theme::MUTED);
                    x += header[2].1;
                    if let Some(r) = if home { g.away } else { g.home } {
                        if let Some(path) = files.crest(&world.club(r).file_ref) {
                            egui::Image::new(format!("file://{}", path.display())).paint_at(ui, egui::Rect::from_min_size(egui::pos2(x, rect.top() + 3.0), egui::vec2(20.0, 20.0)));
                        }
                        ui.painter().text(egui::pos2(x + 24.0, y), egui::Align2::LEFT_CENTER, &world.club(r).name, theme::bold(12.0), theme::TEXT);
                    }
                    x += header[3].1;
                    if g.played {
                        let (mine, theirs) = if home { (g.home_goals, g.away_goals) } else { (g.away_goals, g.home_goals) };
                        let color = if mine > theirs {
                            theme::TEXT_GREEN
                        } else if mine < theirs {
                            theme::TEXT_RED
                        } else {
                            theme::TEXT
                        };
                        ui.painter().text(egui::pos2(x, y), egui::Align2::LEFT_CENTER, widgets::score(world, m), theme::bold(12.0), color);
                    }
                }
            });
            let _ = crest;
        });
}
