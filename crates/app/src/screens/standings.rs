//! Tables (`a.ay`, 833x735): the competition and its stage, the table or the knockout rounds,
//! and on the right the statistics cards or the rounds.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, MatchId, StageId};
use brasfoot_model::saved::StageKind;
use brasfoot_model::world::World;
use eframe::egui::{self, Color32, Stroke};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::widgets::{self, crest};
use crate::{format, motion, theme, view};

pub fn window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, Color32::BLACK));
    motion::dialog(ctx, "tables", open, egui::vec2(833.0, 735.0), frame, |ui, open| body(s, ui, t, files, open));
}

fn body(s: &mut CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, open: &mut bool) {
    let world = &s.career.world;
    ui.add_space(20.0);
    // the top bar: the category buttons, the year and the X
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        egui::Frame::new().fill(theme::BACKGROUND).show(ui, |ui| {
            ui.set_width(810.0);
            ui.set_height(48.0);
            ui.horizontal_centered(|ui| {
                if let Some(uri) = files.jar_image(ui.ctx(), "aiconsb/b0s.png") {
                    ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(79.0, 45.0)));
                }
                ui.add_space(500.0);
                let year = s.career.runtime.clock.fields(world.today_millis()).year;
                ui.label(egui::RichText::new(year.to_string()).font(theme::bold(14.0)).color(Color32::WHITE));
                ui.add_space(80.0);
                if widgets::close_button(ui) {
                    *open = false;
                }
            });
        });
    });
    ui.add_space(20.0);
    let competitions = view::competitions(world);
    if competitions.is_empty() {
        ui.label(t.text("tables-nothing-yet", None));
        return;
    }
    let current = s
        .standings_competition
        .filter(|c| competitions.contains(c))
        .or_else(|| view::league_place(world, s.club).map(|(c, _)| c))
        .unwrap_or(competitions[0]);
    let stages = view::stages_of(world, current);
    let mut show_rounds = s.standings_rounds;
    let mut chosen = current;
    let mut stage_index = s.standings_stage.min(stages.len().saturating_sub(1));
    ui.horizontal_top(|ui| {
        ui.add_space(14.0);
        // the left column (490 wide)
        ui.vertical(|ui| {
            ui.set_width(490.0);
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("competition").width(290.0).selected_text(widgets::competition_name(t, world, current)).show_ui(ui, |ui| {
                    for &c in &competitions {
                        ui.selectable_value(&mut chosen, c, widgets::competition_name(t, world, c));
                    }
                });
                if stages.len() > 1 {
                    egui::ComboBox::from_id_salt("stage").width(180.0).selected_text(t.text("tables-stage", Some(&args([("number", stage_index + 1)])))).show_ui(ui, |ui| {
                        for i in 0..stages.len() {
                            ui.selectable_value(&mut stage_index, i, t.text("tables-stage", Some(&args([("number", i + 1)]))));
                        }
                    });
                }
            });
            ui.add_space(14.0);
            egui::ScrollArea::vertical().id_salt("table").max_height(570.0).show(ui, |ui| {
                if let Some(&stage) = stages.get(stage_index) {
                    match world.stage(stage).class {
                        StageKind::League => league(s, ui, t, files, stage),
                        StageKind::Knockout => knockout(world, ui, files, t, stage),
                    }
                } else {
                    ui.label(t.text("tables-nothing-yet", None));
                }
            });
        });
        ui.add_space(14.0);
        // the right column (300 wide): statistics or rounds, and the two tabs
        ui.vertical(|ui| {
            ui.set_width(300.0);
            let league_stage = stages.get(stage_index).filter(|&&st| world.stage(st).class == StageKind::League).copied();
            let rounds = show_rounds && league_stage.is_some();
            egui::ScrollArea::vertical().id_salt("right").max_height(580.0).show(ui, |ui| {
                ui.set_min_height(580.0);
                if rounds {
                    rounds_list(world, ui, t, league_stage.expect("checked"));
                } else {
                    statistics(s, ui, t, files, current);
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if tab(ui, &t.text("tables-tab-statistics", None), !rounds).clicked() {
                    show_rounds = false;
                }
                if league_stage.is_some() && tab(ui, &t.text("tables-rounds", None), rounds).clicked() {
                    show_rounds = true;
                }
            });
        });
    });
    s.standings_rounds = show_rounds;
    if chosen != current {
        s.standings_competition = Some(chosen);
        s.standings_stage = 0;
    } else {
        s.standings_competition = Some(current);
        s.standings_stage = stage_index;
    }
}

/// The tabs of the dialog: the active one light with dark text.
fn tab(ui: &mut egui::Ui, text: &str, active: bool) -> egui::Response {
    let (fill, color) = if active { (Color32::from_rgb(0xd0, 0xd1, 0xd5), theme::HEADER) } else { (theme::CARD, theme::TEXT) };
    ui.add(egui::Button::new(egui::RichText::new(text).color(color).font(theme::regular(11.0))).fill(fill).corner_radius(0.0).min_size(egui::vec2(90.0, 25.0)))
}

/// The row band of a club (`components.e.td()`): promotion, relegation or neutral.
fn band(world: &World, stage: StageId, index: usize, count: usize) -> Color32 {
    let s = world.stage(stage);
    let relegated = brasfoot_simulation::league::relegated_places(world, stage) as usize;
    let promoted = s
        .competition
        .map(|c| {
            let comp = world.competition(c);
            let divisions = comp.country_ref.map(|k| world.country(k).divisions.clone()).unwrap_or_default();
            if comp.division > 1 && (comp.division as usize - 2) < divisions.len() { world.competition(divisions[comp.division as usize - 2]).relegated as usize } else { 0 }
        })
        .unwrap_or(0);
    if index < promoted {
        theme::BAND_GREEN
    } else if relegated > 0 && index >= count.saturating_sub(relegated) {
        theme::BAND_RED
    } else {
        theme::BAND_NEUTRAL
    }
}

fn league(s: &CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, stage: StageId) {
    let world = &s.career.world;
    let groups = view::standings(world, stage);
    let columns = ["tables-column-points", "tables-column-games", "tables-column-wins", "tables-column-draws", "tables-column-losses", "tables-column-for", "tables-column-against", "tables-column-difference"];
    let xs = [210.0, 240.0, 270.0, 300.0, 330.0, 365.0, 400.0, 435.0];
    for (g, rows) in groups.iter().enumerate() {
        // `components.aw`: the group's header
        let (rect, _) = ui.allocate_exact_size(egui::vec2(470.0, 25.0), egui::Sense::hover());
        ui.painter().rect_filled(egui::Rect::from_min_max(egui::pos2(rect.left() + 200.0, rect.top()), rect.max), 0.0, theme::HEADER);
        if groups.len() > 1 {
            ui.painter().text(rect.left_center() + egui::vec2(4.0, 0.0), egui::Align2::LEFT_CENTER, t.text("tables-group-letter", Some(&args([("letter", ((b'A' + g as u8) as char).to_string())]))), theme::bold(12.0), theme::TEXT);
        }
        for (i, key) in columns.iter().enumerate() {
            ui.painter().text(egui::pos2(rect.left() + xs[i] + 15.0, rect.center().y), egui::Align2::CENTER_CENTER, t.text(key, None), theme::bold(12.0), theme::TEXT);
        }
        let n = rows.len();
        for (i, r) in rows.iter().enumerate() {
            // `components.au`: a table row
            let (rect, response) = ui.allocate_exact_size(egui::vec2(470.0, 26.0), egui::Sense::hover());
            let hover = motion::toward(ui.ctx(), response.id, response.hovered(), motion::HOVER);
            let p = ui.painter();
            let tied = i > 0 && rows[i - 1].numbers[0] == r.numbers[0] && rows[i - 1].numbers[2] == r.numbers[2] && rows[i - 1].numbers[7] == r.numbers[7] && rows[i - 1].numbers[5] == r.numbers[5];
            if !tied {
                p.text(egui::pos2(rect.left() + 10.0, rect.center().y), egui::Align2::CENTER_CENTER, (i + 1).to_string(), theme::regular(12.0), theme::TEXT);
            }
            let band_color = if groups.len() == 1 { band(world, stage, i, n) } else { theme::BAND_NEUTRAL };
            p.rect_filled(egui::Rect::from_min_size(egui::pos2(rect.left() + 25.0, rect.top()), egui::vec2(5.0, rect.height() - 1.0)), 0.0, band_color);
            let name_rect = egui::Rect::from_min_max(egui::pos2(rect.left() + 32.0, rect.top()), egui::pos2(rect.left() + 208.0, rect.bottom() - 1.0));
            p.rect_filled(name_rect, 0.0, motion::mix(theme::CARD, theme::CARD_HOVER, hover));
            let numbers_rect = egui::Rect::from_min_max(egui::pos2(rect.left() + 208.0, rect.top()), egui::pos2(rect.right(), rect.bottom() - 1.0));
            p.rect_filled(numbers_rect, 0.0, motion::mix(theme::PANEL_HIGH, theme::CARD_HOVER, hover));
            let mut x = name_rect.left() + 4.0;
            if let Some(path) = files.crest(&world.club(r.club).file_ref) {
                egui::Image::new(format!("file://{}", path.display())).paint_at(ui, egui::Rect::from_min_size(egui::pos2(x, rect.top() + 3.0), egui::vec2(20.0, 20.0)));
                x += 24.0;
            }
            let p = ui.painter();
            p.text(egui::pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, &world.club(r.club).name, theme::bold(12.0), theme::TEXT);
            for (k, v) in r.numbers.iter().enumerate() {
                p.text(egui::pos2(rect.left() + xs[k] + 15.0, rect.center().y), egui::Align2::CENTER_CENTER, v.to_string(), theme::bold(12.0), theme::TEXT);
            }
        }
        ui.add_space(20.0);
    }
}

fn knockout(world: &World, ui: &mut egui::Ui, files: &GameFiles, t: &Translator, stage: StageId) {
    for (i, round) in view::knockout_rounds(world, stage).iter().enumerate().rev() {
        // `components.ax`: the round's header
        let (rect, _) = ui.allocate_exact_size(egui::vec2(470.0, 25.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, theme::HEADER);
        ui.painter().text(rect.left_center() + egui::vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, t.text("tables-knockout-round", Some(&args([("number", i + 1)]))), theme::bold(12.0), Color32::from_rgb(0xef, 0xef, 0xef));
        for &(first, second) in round {
            let h = if second.is_some() { 55.0 } else { 30.0 };
            let (rect, _) = ui.allocate_exact_size(egui::vec2(470.0, h), egui::Sense::hover());
            ui.painter().rect_filled(rect.shrink2(egui::vec2(0.0, 1.0)), 0.0, theme::CARD);
            tie_line(world, ui, files, egui::Rect::from_min_size(rect.min, egui::vec2(470.0, 27.0)), first);
            if let Some(second) = second {
                tie_line(world, ui, files, egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 27.0), egui::vec2(470.0, 27.0)), second);
            }
        }
        ui.add_space(8.0);
    }
}

/// `components.av`: one leg, the winner in bold, penalties after the score.
fn tie_line(world: &World, ui: &mut egui::Ui, files: &GameFiles, rect: egui::Rect, m: MatchId) {
    let g = world.game_match(m);
    let side = |c: Option<ClubId>| c.map(|c| world.club(c).name.clone()).unwrap_or_default();
    let winner = g.winner.or(if g.home_goals > g.away_goals { g.home } else if g.away_goals > g.home_goals { g.away } else { None });
    let font = |c: Option<ClubId>| if g.played && winner == c && c.is_some() { theme::bold(12.0) } else { theme::regular(12.0) };
    let p = ui.painter();
    p.text(egui::pos2(rect.center().x - 40.0, rect.center().y), egui::Align2::RIGHT_CENTER, side(g.home), font(g.home), theme::TEXT);
    let mut score = widgets::score(world, m);
    if g.went_to_penalties {
        score = format!("{score} P:{}-{}", g.value_gg, g.value_gh);
    }
    p.text(rect.center(), egui::Align2::CENTER_CENTER, score, theme::bold(12.0), theme::TEXT);
    p.text(egui::pos2(rect.center().x + 40.0, rect.center().y), egui::Align2::LEFT_CENTER, side(g.away), font(g.away), theme::TEXT);
    for (c, x) in [(g.home, rect.left() + 4.0), (g.away, rect.right() - 24.0)] {
        if let Some(c) = c
            && let Some(path) = files.crest(&world.club(c).file_ref)
        {
            egui::Image::new(format!("file://{}", path.display())).paint_at(ui, egui::Rect::from_min_size(egui::pos2(x, rect.top() + 3.0), egui::vec2(20.0, 20.0)));
        }
    }
}

fn card_rows(ui: &mut egui::Ui, rows: &[(String, String, String)], empty: &str) {
    let gray = Color32::from_rgb(0x70, 0x78, 0x88);
    let _ = gray;
    for i in 0..5 {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, theme::CARD);
        match rows.get(i) {
            Some((name, a, b)) => {
                ui.painter().text(rect.left_center() + egui::vec2(5.0, 0.0), egui::Align2::LEFT_CENTER, name, theme::bold(11.0), theme::TEXT);
                ui.painter().text(egui::pos2(rect.right() - 54.0, rect.center().y), egui::Align2::CENTER_CENTER, a, theme::bold(12.0), theme::TEXT);
                ui.painter().text(egui::pos2(rect.right() - 18.0, rect.center().y), egui::Align2::CENTER_CENTER, b, theme::bold(12.0), theme::TEXT);
            }
            None => {
                ui.painter().text(rect.left_center() + egui::vec2(5.0, 0.0), egui::Align2::LEFT_CENTER, empty, theme::bold(11.0), theme::TEXT);
            }
        }
    }
}

fn statistics(s: &CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, c: brasfoot_model::ids::CompetitionId) {
    let world = &s.career.world;
    let empty = t.text("tables-not-defined", None);
    let owner = |p: brasfoot_model::ids::PlayerId| {
        let x = world.player(p);
        let club = x.club.map(|c| world.club(c).name.clone()).unwrap_or_default();
        format!("{} ({club})", x.name)
    };
    ui.spacing_mut().item_spacing.y = 0.0;
    theme::header_strip(ui, &t.text("tables-top-scorers", None), &[&t.text("tables-goals", None), &t.text("tables-games", None)]);
    let scorers: Vec<(String, String, String)> = view::top_scorers(world, c, 5).into_iter().map(|(p, g, j)| (owner(p), g.to_string(), j.to_string())).collect();
    card_rows(ui, &scorers, &empty);
    ui.add_space(10.0);
    theme::header_strip(ui, &t.text("tables-best-ratings", None), &[&t.text("tables-rating", None), &t.text("tables-games", None)]);
    let ratings: Vec<(String, String, String)> = view::best_ratings(world, c)
        .into_iter()
        .map(|(p, r, j)| {
            let x = world.player(p);
            (format!("{} - {}", widgets::position_short(t, x.position), owner(p)), format::rating_two(t, r), j.to_string())
        })
        .collect();
    card_rows(ui, &ratings, &empty);
    ui.add_space(10.0);
    theme::header_strip(ui, &t.text("tables-history", None), &[]);
    let h = view::competition_history(world, c);
    let mut lines: Vec<(String, String)> = Vec::new();
    let average = if h.games > 0 { format::rating_two(t, h.goals as f64 / h.games as f64) } else { "--".into() };
    lines.push((t.text("tables-goal-average", None), t.text("tables-goal-average-value", Some(&args([("average", average), ("goals", h.goals.to_string()), ("games", h.games.to_string())])))));
    let biggest = h.biggest_win.map(|m| {
        let g = world.game_match(m);
        let name = |c: Option<ClubId>| c.map(|c| world.club(c).name.clone()).unwrap_or_default();
        format!("{} {} x {} {}", name(g.home), g.home_goals, g.away_goals, name(g.away))
    });
    lines.push((t.text("tables-biggest-win", None), biggest.unwrap_or_else(|| t.text("tables-no-big-win", None))));
    lines.push((t.text("tables-last-champion", None), h.last_champion.map(|c| world.club(c).name.clone()).unwrap_or_else(|| empty.clone())));
    let winners = if h.most_titles.is_empty() { empty.clone() } else { h.most_titles.iter().map(|(c, n)| format!("{} ({n})", world.club(*c).name)).collect::<Vec<_>>().join("\n") };
    lines.push((t.text("tables-most-titles", None), winners));
    egui::Frame::new().fill(theme::CARD).inner_margin(5).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 6.0;
        egui::Grid::new("history").num_columns(2).spacing([8.0, 10.0]).show(ui, |ui| {
            for (label, value) in lines {
                ui.label(egui::RichText::new(label).font(theme::bold(11.0)));
                ui.label(egui::RichText::new(value).font(theme::regular(11.0)));
                ui.end_row();
            }
        });
        if let Some(c) = h.last_champion {
            crest(ui, files, world, c, 40.0);
        }
    });
}

fn rounds_list(world: &World, ui: &mut egui::Ui, t: &Translator, stage: StageId) {
    let rounds = view::league_rounds(world, stage);
    for (i, round) in rounds.iter().enumerate() {
        ui.label(egui::RichText::new(t.text("tables-round", Some(&args([("number", i + 1)])))).font(theme::bold(12.0)));
        for &m in round {
            let g = world.game_match(m);
            let name = |c: Option<ClubId>| c.map(|c| world.club(c).name.clone()).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.add_sized([120.0, 20.0], egui::Label::new(egui::RichText::new(name(g.home)).font(theme::regular(11.0))).halign(egui::Align::Max).truncate());
                ui.add_sized([30.0, 20.0], egui::Label::new(egui::RichText::new(if g.played { format!("{}x{}", g.home_goals, g.away_goals) } else { "x".into() }).font(theme::bold(11.0))));
                ui.add_sized([120.0, 20.0], egui::Label::new(egui::RichText::new(name(g.away)).font(theme::regular(11.0))).truncate());
            });
        }
        ui.add_space(8.0);
    }
}
