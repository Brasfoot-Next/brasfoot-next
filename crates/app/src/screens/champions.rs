//! The champions (`a.R`, "Campeões"): a competition's past editions, newest first, one per
//! season (`R.s(at)`), with the champion, the runner-up, the top scorer and the champion's
//! coach (`b.j`, `b.ad`).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, CompetitionId};
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::widgets;
use crate::{motion, theme, view};

pub struct ChampionsState {
    competition: Option<CompetitionId>,
}

impl ChampionsState {
    /// `R.cB(0)`: the first competition of the list.
    pub fn new(world: &World) -> Self {
        ChampionsState { competition: view::competitions(world).first().copied() }
    }
}

pub fn window(s: &mut CareerScreen, state: &mut ChampionsState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let world = &s.career.world;
    motion::dialog(ctx, "champions", open, egui::vec2(900.0, 640.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::HISTORY), &t.text("champions-title", None)) {
            *open = false;
        }
        let shown = state.competition.map(|c| widgets::competition_name(t, world, c)).unwrap_or_default();
        egui::ComboBox::from_id_salt("champions-competition").width(320.0).height(400.0).selected_text(shown).show_ui(ui, |ui| {
            for c in view::competitions(world) {
                ui.selectable_value(&mut state.competition, Some(c), widgets::competition_name(t, world, c));
            }
        });
        ui.add_space(10.0);
        if let Some(c) = state.competition {
            table(ui, t, files, world, c);
        }
    });
}

fn table(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, c: CompetitionId) {
    // R.s(at): newest first, the first record of each season
    let mut editions = Vec::new();
    let mut last = -1;
    for e in world.competition(c).titles.iter().rev() {
        if e.season != last {
            editions.push(e);
            last = e.season;
        }
    }
    let width = ui.available_width();
    let columns = [("champions-column-year", 0.08), ("champions-column-champion", 0.22), ("champions-column-runner-up", 0.22), ("champions-column-top-scorer", 0.28), ("champions-column-coach", 0.20)];
    let widths: Vec<f32> = columns.iter().map(|c| width * c.1).collect();
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
    ui.painter().rect_filled(header, 6.0, theme::HEADER);
    let mut x = header.left();
    for ((key, _), w) in columns.iter().zip(&widths) {
        ui.painter().text(egui::pos2(x + 8.0, header.center().y), egui::Align2::LEFT_CENTER, t.text(key, None), theme::bold(11.0), theme::MUTED);
        x += w;
    }
    ui.add_space(2.0);
    if editions.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| ui.label(egui::RichText::new(t.text("champions-empty", None)).color(theme::MUTED)));
        return;
    }
    let base_year = world.game.start_year - 1;
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical().id_salt("champions-rows").auto_shrink([false, false]).show_rows(ui, 28.0, editions.len(), |ui, range| {
        for i in range {
            let e = editions[i];
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::hover());
            if i % 2 == 1 {
                ui.painter().rect_filled(rect, 4.0, theme::ROW_ALT);
            }
            let mut x = rect.left();
            let mut cell = |ui: &mut egui::Ui, w: f32, club: Option<ClubId>, text: String, bold: bool| {
                let r = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(w, rect.height()));
                x += w;
                let mut left = r.left() + 8.0;
                if let Some(c) = club {
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(egui::pos2(left + 9.0, r.center().y), egui::vec2(18.0, 18.0))));
                    widgets::crest(&mut child, files, world, c, 18.0);
                    left += 24.0;
                }
                let font = if bold { theme::bold(13.0) } else { theme::regular(13.0) };
                ui.painter().with_clip_rect(r.shrink2(egui::vec2(2.0, 0.0))).text(egui::pos2(left, r.center().y), egui::Align2::LEFT_CENTER, text, font, theme::TEXT);
            };
            let champion = world.club_by_game_id(e.champion);
            let runner_up = world.club_by_game_id(e.runner_up);
            let scorer_club = world.club_by_game_id(e.top_scorer_club);
            let name = |c: Option<ClubId>| c.map(|c| world.club(c).name.clone()).unwrap_or_default();
            // b.ad: "<scorer>(<club>) - <goals> g"
            let scorer = match &e.top_scorer_name {
                Some(n) => t.text("champions-top-scorer", Some(&args([("player", n.clone()), ("goals", e.top_scorer_goals.to_string())]))),
                None => String::new(),
            };
            let coach = world.coach_by_game_id(e.champion_coach).map(|c| world.coach(c).name.clone()).unwrap_or_default();
            cell(ui, widths[0], None, (e.season + base_year).to_string(), true);
            cell(ui, widths[1], champion, name(champion), true);
            cell(ui, widths[2], runner_up, name(runner_up), false);
            cell(ui, widths[3], scorer_club, scorer, false);
            cell(ui, widths[4], None, coach, false);
        }
    });
}
