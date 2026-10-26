//! The coach ranking (`a.gX`): every coach of the game (`SR.L()`) or those of one country, by
//! points (the list's own order) or by reputation (`a.gY`), and the picked coach's seasons
//! (`best.j`, newest first).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, CoachId};
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::menu;
use crate::screens::new_game::country_name;
use crate::screens::widgets;
use crate::{motion, theme};

pub struct CoachesState {
    /// `ue`: an index of the league countries; `None` is "Internacional".
    country: Option<usize>,
    /// `KK`: 0 points, 1 reputation.
    mode: usize,
    /// `Es`
    selected: Option<CoachId>,
    /// Scroll to the selected row on the next frame.
    scroll_to_selected: bool,
}

impl CoachesState {
    /// The screen as `a.gX` opens it: the country of the player's coach and that coach picked.
    pub fn new(world: &World, own: Option<CoachId>) -> Self {
        let country = own.and_then(|c| {
            let code = coach_country(world, c);
            world.game.league_countries.iter().position(|&k| world.country(k).country == code)
        });
        CoachesState { country, mode: 0, selected: own, scroll_to_selected: true }
    }
}

/// `fg().getPais()`, or `bz()` for a coach without a club.
fn coach_country(world: &World, c: CoachId) -> i32 {
    let coach = world.coach(c);
    match coach.club_id {
        Some(club) => world.club(club).country,
        None => coach.previous_country,
    }
}

/// `gX.AR()`: the coaches of the country (all for "Internacional"), sorted by reputation in that
/// mode; a stable sort, as `Collections.sort`.
fn listed(world: &World, state: &CoachesState) -> Vec<CoachId> {
    let code = state.country.and_then(|i| world.game.league_countries.get(i)).map(|&k| world.country(k).country);
    let mut list: Vec<CoachId> = world.game.coaches.iter().copied().filter(|&c| code.is_none_or(|code| coach_country(world, c) == code)).collect();
    if state.mode == 1 {
        list.sort_by(|&a, &b| world.coach(b).reputation.cmp(&world.coach(a).reputation));
    }
    list
}

fn reputation_name(t: &Translator, reputation: i32) -> String {
    if (0..6).contains(&reputation) { t.text(&format!("teams-reputation-{reputation}"), None) } else { String::new() }
}

pub fn window(s: &mut CareerScreen, state: &mut Option<CoachesState>, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let own = s.career.world.club(s.club).coach;
    let state = state.get_or_insert_with(|| CoachesState::new(&s.career.world, own));
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let mut gallery = None;
    motion::dialog(ctx, "coach-ranking", open, egui::vec2(1000.0, 660.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::COACHES), &t.text("coach-ranking-title", None)) {
            *open = false;
        }
        let world = &s.career.world;
        let mut changed = false;
        ui.horizontal(|ui| {
            let shown = state.country.map(|i| country_name(t, world.country(world.game.league_countries[i]).country)).unwrap_or_else(|| t.text("coach-ranking-combo-international", None));
            egui::ComboBox::from_id_salt("coach-country").width(220.0).height(400.0).selected_text(shown).show_ui(ui, |ui| {
                changed |= ui.selectable_value(&mut state.country, None, t.text("coach-ranking-combo-international", None)).clicked();
                for (i, &k) in world.game.league_countries.iter().enumerate() {
                    let code = world.country(k).country;
                    ui.horizontal(|ui| {
                        widgets::flag(ui, files, code, 16.0);
                        changed |= ui.selectable_value(&mut state.country, Some(i), country_name(t, code)).clicked();
                    });
                }
            });
            ui.add_space(12.0);
            let modes = [t.text("coach-ranking-mode-points", None), t.text("coach-ranking-mode-reputation", None)];
            if let Some(m) = menu::segmented(ui, &modes, state.mode) {
                state.mode = m;
                changed = true;
            }
        });
        ui.add_space(10.0);
        let list = listed(world, state);
        // AR() picks the first row after a change of country or mode
        if changed {
            state.selected = list.first().copied();
            state.scroll_to_selected = true;
        }
        if state.selected.is_none_or(|c| !list.contains(&c)) && !changed {
            state.selected = state.selected.filter(|c| list.contains(c)).or(list.first().copied());
        }
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(470.0);
                ranking(world, state, &list, ui, t);
            });
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                if let Some(c) = state.selected
                    && details(world, c, ui, t, files)
                {
                    gallery = Some(c);
                }
            });
        });
    });
    if gallery.is_some() {
        s.coach_trophies = gallery;
    }
}

fn ranking(world: &World, state: &mut CoachesState, list: &[CoachId], ui: &mut egui::Ui, t: &Translator) {
    let by_reputation = state.mode == 1;
    let columns: Vec<(String, f32, bool)> = if by_reputation {
        vec![
            (String::new(), 40.0, false),
            (t.text("coach-ranking-column-coach", None), 160.0, true),
            (t.text("coach-ranking-column-current-team", None), 150.0, true),
            (t.text("coach-ranking-column-reputation", None), 110.0, false),
        ]
    } else {
        vec![
            (String::new(), 40.0, false),
            (t.text("coach-ranking-column-coach", None), 160.0, true),
            (t.text("coach-ranking-column-current-team", None), 150.0, true),
            (t.text("coach-ranking-column-points", None), 60.0, false),
            (t.text("coach-ranking-column-titles", None), 50.0, false),
        ]
    };
    let total: f32 = columns.iter().map(|c| c.1).sum();
    header(ui, &columns, total);
    let ctx = ui.ctx().clone();
    let target = if state.scroll_to_selected { state.selected.and_then(|c| list.iter().position(|&x| x == c)) } else { None };
    let mut scroll = egui::ScrollArea::vertical().id_salt("coach-ranking-rows").auto_shrink([false, false]);
    if let Some(row) = target {
        scroll = scroll.vertical_scroll_offset(((row as f32) - 8.0).max(0.0) * 24.0);
        state.scroll_to_selected = false;
    }
    // show_rows counts the spacing between rows: none here, so a row is 24 high
    ui.spacing_mut().item_spacing.y = 0.0;
    scroll.show_rows(ui, 24.0, list.len(), |ui, rows| {
        for i in rows {
            let c = list[i];
            let coach = world.coach(c);
            let (rect, response) = ui.allocate_exact_size(egui::vec2(total, 24.0), egui::Sense::click());
            if response.clicked() {
                state.selected = Some(c);
            }
            let hover = motion::toward(&ctx, response.id, response.hovered(), motion::HOVER);
            let base = if i % 2 == 1 { theme::ROW_ALT } else { theme::PANEL };
            let fill = if state.selected == Some(c) { theme::SELECTION } else { motion::mix(base, theme::HOVER, hover) };
            let p = ui.painter();
            p.rect_filled(rect, 4.0, fill);
            let club = coach.club_id.map(|k| world.club(k).name.clone()).unwrap_or_default();
            let mut cells = vec![(i + 1).to_string(), coach.name.clone(), club];
            if by_reputation {
                cells.push(reputation_name(t, coach.reputation));
            } else {
                cells.push(coach.points.to_string());
                cells.push(coach.titles.to_string());
            }
            let mut x = rect.left();
            for ((_, w, left), text) in columns.iter().zip(cells) {
                let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*w, rect.height()));
                x += w;
                let clipped = p.with_clip_rect(cell.shrink2(egui::vec2(2.0, 0.0)));
                if *left {
                    clipped.text(egui::pos2(cell.left() + 4.0, cell.center().y), egui::Align2::LEFT_CENTER, text, theme::regular(12.0), theme::TEXT);
                } else {
                    clipped.text(cell.center(), egui::Align2::CENTER_CENTER, text, theme::regular(12.0), theme::TEXT);
                }
            }
            if response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
    });
}

fn header(ui: &mut egui::Ui, columns: &[(String, f32, bool)], total: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total, 26.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 6.0, theme::HEADER);
    let mut x = rect.left();
    for (title, w, left) in columns {
        let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*w, rect.height()));
        x += w;
        if *left {
            ui.painter().text(egui::pos2(cell.left() + 4.0, cell.center().y), egui::Align2::LEFT_CENTER, title, theme::bold(11.0), theme::MUTED);
        } else {
            ui.painter().text(cell.center(), egui::Align2::CENTER_CENTER, title, theme::bold(11.0), theme::MUTED);
        }
    }
    ui.add_space(2.0);
}

/// `gX.rV()`: the coach, the reputation, the totals and the seasons. True when the trophies
/// button (`Ie`) was clicked.
fn details(world: &World, c: CoachId, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> bool {
    let coach = world.coach(c);
    let mut gallery = false;
    menu::panel().fill(theme::CARD).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            widgets::flag(ui, files, coach.country, 22.0);
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&coach.name).font(theme::bold(18.0)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                gallery = widgets::icon_button(ui, glyphs::TROPHY, 20.0, theme::STAR, true).on_hover_text(t.text("coaches-menu-trophies", None)).clicked();
            });
        });
        ui.add_space(2.0);
        let reputation = t.text("coach-ranking-detail-reputation", Some(&args([("reputation", reputation_name(t, coach.reputation))])));
        ui.label(egui::RichText::new(reputation).font(theme::regular(13.0)).color(theme::MUTED));
        ui.add_space(8.0);
        let sum = |f: fn(&brasfoot_model::saved::CoachSeason) -> i32| coach.seasons.iter().map(f).sum::<i32>();
        let summary = t.text(
            "coach-ranking-detail-summary",
            Some(&args([
                ("played", sum(|s| s.games)),
                ("wins", sum(|s| s.wins)),
                ("losses", sum(|s| s.losses)),
                ("points", sum(|s| s.points)),
                ("titles", sum(|s| s.titles)),
            ])),
        );
        ui.label(egui::RichText::new(summary).font(theme::regular(13.0)));
    });
    ui.add_space(10.0);
    let columns: Vec<(String, f32, bool)> = vec![
        (t.text("coach-ranking-column-year", None), 52.0, false),
        (t.text("coach-ranking-column-team", None), 170.0, true),
        (t.text("coach-ranking-column-played", None), 40.0, false),
        (t.text("coach-ranking-column-wins", None), 40.0, false),
        (t.text("coach-ranking-column-losses", None), 40.0, false),
        (t.text("coach-ranking-column-win-pct", None), 50.0, false),
        (t.text("coach-ranking-column-pr", None), 40.0, false),
        (t.text("coach-ranking-column-title-short", None), 40.0, false),
    ];
    let total: f32 = columns.iter().map(|c| c.1).sum();
    header(ui, &columns, total);
    // the year is `H() + SR.iU()`, the first season being the start year
    let base_year = world.game.start_year - 1;
    egui::ScrollArea::vertical().id_salt("coach-history").auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for (i, season) in coach.seasons.iter().rev().enumerate() {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(total, 22.0), egui::Sense::hover());
            if i % 2 == 1 {
                ui.painter().rect_filled(rect, 4.0, theme::ROW_ALT);
            }
            // `j.ck()`: the club by its game id, clubs and national teams alike
            let club = world.club_by_game_id(season.club_id).map(|k| world.club(k).name.clone()).unwrap_or_default();
            let pct = if season.games > 0 { season.wins * 100 / season.games } else { 0 };
            let cells = [
                (season.season + base_year).to_string(),
                club,
                season.games.to_string(),
                season.wins.to_string(),
                season.losses.to_string(),
                t.text("coach-ranking-percent", Some(&args([("value", pct)]))),
                season.points.to_string(),
                season.titles.to_string(),
            ];
            let mut x = rect.left();
            for ((_, w, left), text) in columns.iter().zip(cells) {
                let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*w, rect.height()));
                x += w;
                let p = ui.painter().with_clip_rect(cell.shrink2(egui::vec2(2.0, 0.0)));
                if *left {
                    p.text(egui::pos2(cell.left() + 4.0, cell.center().y), egui::Align2::LEFT_CENTER, text, theme::regular(12.0), theme::TEXT);
                } else {
                    p.text(cell.center(), egui::Align2::CENTER_CENTER, text, theme::regular(12.0), theme::TEXT);
                }
            }
        }
    });
    gallery
}

/// The latest coach changes window (`a.hC`).
pub struct AddRemoveCoachState {
    pub name: String,
    pub selected_club: Option<ClubId>,
    pub status: Option<(String, egui::Color32)>,
}

impl Default for AddRemoveCoachState {
    fn default() -> Self {
        AddRemoveCoachState {
            name: String::new(),
            selected_club: None,
            status: None,
        }
    }
}

/// The add / remove human coaches window (`a.ip`).
pub fn add_remove_window(
    s: &mut CareerScreen,
    state: &mut AddRemoveCoachState,
    ctx: &egui::Context,
    t: &Translator,
    files: &GameFiles,
    open: &mut bool,
) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "add-remove-coaches-dialog", open, egui::vec2(600.0, 480.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, None, &t.text("coaches-menu-add-remove", None)) {
            *open = false;
        }
        ui.add_space(8.0);

        ui.label(egui::RichText::new("Técnicos Humanos Atuais").font(theme::bold(14.0)).color(egui::Color32::WHITE));
        ui.add_space(4.0);

        let humans = s.career.world.game.human_coaches.clone();
        egui::Frame::new().fill(theme::PANEL_HIGH).inner_margin(8).corner_radius(4).show(ui, |ui| {
            for &c in &humans {
                let coach = s.career.world.coach(c);
                let club_name = coach.club_id.map(|k| s.career.world.club(k).name.clone()).unwrap_or_else(|| "Sem clube".to_string());
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&coach.name).font(theme::bold(13.0)).color(egui::Color32::WHITE));
                    ui.label(egui::RichText::new(format!("— {club_name}")).color(theme::MUTED));
                });
            }
        });

        ui.add_space(16.0);
        ui.label(egui::RichText::new("Adicionar Novo Técnico").font(theme::bold(14.0)).color(egui::Color32::WHITE));
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label("Nome:");
            ui.add(egui::TextEdit::singleline(&mut state.name).desired_width(220.0));
        });

        ui.add_space(8.0);
        let world = &s.career.world;
        let mut club_options: Vec<(ClubId, String)> = world.game.clubs.iter().copied()
            .filter(|&c| world.club(c).coach.map(|coach_id| !world.game.human_coaches.contains(&coach_id)).unwrap_or(true))
            .take(50)
            .map(|c| (c, world.club(c).name.clone()))
            .collect();
        club_options.sort_by(|a, b| a.1.cmp(&b.1));

        if state.selected_club.is_none() && !club_options.is_empty() {
            state.selected_club = Some(club_options[0].0);
        }

        ui.horizontal(|ui| {
            ui.label("Clube:");
            let current_name = state.selected_club.map(|c| world.club(c).name.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt("add-coach-club")
                .selected_text(current_name)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for (c, name) in club_options {
                        ui.selectable_value(&mut state.selected_club, Some(c), name);
                    }
                });
        });

        ui.add_space(12.0);
        if let Some((msg, col)) = &state.status {
            ui.label(egui::RichText::new(msg).color(*col));
            ui.add_space(6.0);
        }

        if ui.button(egui::RichText::new("Adicionar Técnico").font(theme::bold(13.0))).clicked() {
            if state.name.trim().is_empty() {
                state.status = Some(("Digite um nome para o técnico.".to_string(), theme::BAD));
            } else if let Some(club) = state.selected_club {
                let country = s.career.world.club(club).country;
                s.career.add_human_coach(club, state.name.trim(), country);
                state.status = Some((format!("Técnico {} contratado com sucesso!", state.name.trim()), theme::GOOD));
                state.name.clear();
            }
        }
    });
}

/// The latest coach changes window (`a.hC`).
pub fn changes_window(
    s: &mut CareerScreen,
    ctx: &egui::Context,
    t: &Translator,
    files: &GameFiles,
    open: &mut bool,
) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "coach-changes-dialog", open, egui::vec2(680.0, 480.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, None, &t.text("coach-changes-title", None)) {
            *open = false;
        }
        ui.add_space(8.0);
        let changes = s.career.world.game.coach_changes.clone();
        if changes.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(t.text("coach-changes-empty", None)).font(theme::regular(13.0)).color(theme::MUTED));
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(t.text("coach-changes-column-date", None)).font(theme::bold(12.0)).color(theme::MUTED));
            ui.add_space(30.0);
            ui.label(egui::RichText::new(t.text("coach-changes-column-club", None)).font(theme::bold(12.0)).color(theme::MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(t.text("coach-changes-column-in", None)).font(theme::bold(12.0)).color(theme::MUTED));
                ui.add_space(60.0);
                ui.label(egui::RichText::new(t.text("coach-changes-column-out", None)).font(theme::bold(12.0)).color(theme::MUTED));
            });
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, c) in changes.iter().rev().enumerate() {
                let club_name = s.career.world.club_by_game_id(c.club_id).map(|k| s.career.world.club(k).name.clone()).unwrap_or_default();
                let coach_out = c.coach_out.map(|id| s.career.world.coach(id).name.clone()).unwrap_or_else(|| "-".to_string());
                let coach_in = c.coach_in.map(|id| s.career.world.coach(id).name.clone()).unwrap_or_else(|| "-".to_string());
                let date_str = crate::format::date(t, &s.career.runtime.clock, c.date);

                let bg = if i % 2 == 1 { theme::ROW_ALT } else { egui::Color32::TRANSPARENT };
                egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(4, 4)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(date_str).font(theme::regular(12.0)).color(theme::MUTED));
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(club_name).font(theme::bold(13.0)).color(egui::Color32::WHITE));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new(coach_in).font(theme::regular(12.5)).color(theme::GOOD));
                            ui.add_space(40.0);
                            ui.label(egui::RichText::new(coach_out).font(theme::regular(12.5)).color(theme::WARN));
                        });
                    });
                });
            }
        });
    });
}
