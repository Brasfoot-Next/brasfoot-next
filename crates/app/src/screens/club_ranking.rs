//! The club ranking (`a.hc`, "Ranking Clubes"): every club of the game (`SR.P()`) or of one
//! country, by reputation, by all-time league points (`ah.ld()`) or by placing points
//! (`ah.lb()`), with the picked club's points by kind of competition (`hc.rW()`).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::countries;
use brasfoot_model::ids::ClubId;
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::new_game::country_name;
use crate::screens::{menu, widgets};
use crate::{motion, theme};
use crate::screens::controls;

pub struct ClubRankingState {
    /// `ue`: an index of the league countries; `None` is "Internacional".
    pub country: Option<usize>,
    /// `KK`: 0 reputation, 1 all-time points, 2 placing points (the one it opens with).
    pub mode: usize,
    pub selected: Option<ClubId>,
    pub national: bool,
}

impl Default for ClubRankingState {
    fn default() -> Self {
        ClubRankingState { country: None, mode: 2, selected: None, national: false }
    }
}

/// `components.ae.Re`: what a club is ranked by.
fn numbers(world: &World, club: ClubId, mode: usize) -> [i32; 3] {
    let c = world.club(club);
    match mode {
        0 => [c.reputation, 0, 0],
        // ah.ld(): PG, GP, GC summed over the kinds 1 to 14
        1 => {
            let mut sum = [0; 3];
            for kind in 1..=14 {
                if let Some(r) = c.all_time_record.get(kind) {
                    for (i, v) in r.iter().take(3).enumerate() {
                        sum[i] += v;
                    }
                }
            }
            sum
        }
        // ah.lb()
        _ => [(1..=14).filter_map(|k| c.ranking_points.get(k)).sum(), 0, 0],
    }
}

/// `hc.dw(int)`: the clubs of the country (all for "Internacional"), sorted by `hd` (the first
/// number, more first) or, for the all-time points, by `he`, which breaks ties by the third
/// number (goals against) going down, as the original does.
fn listed(world: &World, state: &ClubRankingState) -> Vec<(ClubId, [i32; 3])> {
    let mut list: Vec<(ClubId, [i32; 3])> = if state.national {
        world.game.countries.iter().filter_map(|&k| world.country(k).national_team).map(|c| (c, numbers(world, c, state.mode))).collect()
    } else {
        let code = state.country.and_then(|i| world.game.league_countries.get(i)).map(|&k| world.country(k).country);
        world.game.clubs.iter().copied().filter(|&c| code.is_none_or(|code| world.club(c).country == code)).map(|c| (c, numbers(world, c, state.mode))).collect()
    };
    if state.mode == 1 {
        list.sort_by(|a, b| b.1[0].cmp(&a.1[0]).then(b.1[2].cmp(&a.1[2])));
    } else {
        list.sort_by(|a, b| b.1[0].cmp(&a.1[0]));
    }
    list
}

/// `hc.dy(int)`: whether the panel lists a kind of competition for the club.
fn kind_shown(world: &World, club: ClubId, kind: usize) -> bool {
    let c = world.club(club);
    let confederation = countries::confederation(c.country);
    if kind == 3 && (!world.game.plays_state || c.country != 29) {
        return false;
    }
    if [4, 5, 6, 8, 12].contains(&kind) {
        if !world.game.plays_international_clubs {
            return false;
        }
        if (kind == 6 || kind == 8) && confederation > 1 {
            return false;
        }
        if kind == 12 && confederation > 0 {
            return false;
        }
    }
    // aq.fs(int)
    ![7, 9, 13, 14].contains(&kind)
}

/// `aq.y(int, int)`: the name of a kind of competition, the international ones by the club's
/// confederation.
fn kind_name(t: &Translator, kind: usize, country: i32) -> String {
    let confederation = countries::confederation(country).max(0);
    match kind {
        4 => t.text(&format!("club-ranking-kind-4-{confederation}"), None),
        6 => t.text(&format!("club-ranking-kind-6-{confederation}"), None),
        12 if confederation == 0 => t.text("club-ranking-kind-12", None),
        12 => String::new(),
        _ => t.text(&format!("club-ranking-kind-{kind}"), None),
    }
}

pub fn window(s: &mut CareerScreen, state: &mut ClubRankingState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let mut gallery = None;
    motion::dialog(ctx, "club-ranking", open, egui::vec2(960.0, 640.0), frame, |ui, open| {
        let world = &s.career.world;
        let title = if state.national { "Ranking Seleções" } else { &t.text("club-ranking-title", None) };
        if widgets::dialog_title(ui, files, None, title) {
            *open = false;
        }
        let mut changed = false;
        ui.horizontal(|ui| {
            if !state.national {
                let shown = state.country.map(|i| country_name(t, world.country(world.game.league_countries[i]).country)).unwrap_or_else(|| t.text("coach-ranking-combo-international", None));
                egui::ComboBox::from_id_salt("club-ranking-country").width(220.0).height(400.0).selected_text(shown).show_ui(ui, |ui| {
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
            }
            let modes = [t.text("club-ranking-mode-reputation", None), t.text("club-ranking-mode-points", None), t.text("club-ranking-mode-placing", None)];
            if let Some(m) = controls::segmented(ui, &modes, state.mode) {
                state.mode = m;
                changed = true;
            }
        });
        ui.add_space(10.0);
        let list = listed(world, state);
        // dw() picks the first row
        if changed || state.selected.is_none_or(|c| !list.iter().any(|(x, _)| *x == c)) {
            state.selected = list.first().map(|(c, _)| *c);
        }
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(460.0);
                ranking(ui, t, files, world, state, &list, height);
            });
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                if let Some(c) = state.selected
                    && details(ui, t, files, world, state.mode, c)
                {
                    gallery = Some(c);
                }
            });
        });
    });
    // hc.rw(): the club's trophies (best.S.v)
    if let Some(c) = gallery {
        s.trophies_club = Some(c);
        s.trophies_open = true;
    }
}

fn ranking(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, state: &mut ClubRankingState, list: &[(ClubId, [i32; 3])], height: f32) {
    let width = ui.available_width();
    let value_key = ["club-ranking-column-reputation", "club-ranking-column-record", "club-ranking-column-points"][state.mode];
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
    ui.painter().rect_filled(header, 6.0, theme::HEADER);
    ui.painter().text(egui::pos2(header.left() + 56.0, header.center().y), egui::Align2::LEFT_CENTER, t.text("club-ranking-column-team", None), theme::bold(11.0), theme::MUTED);
    ui.painter().text(egui::pos2(header.right() - 10.0, header.center().y), egui::Align2::RIGHT_CENTER, t.text(value_key, None), theme::bold(11.0), theme::MUTED);
    ui.add_space(2.0);
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical().id_salt("club-ranking-rows").auto_shrink([false, false]).max_height(height - 30.0).show_rows(ui, 28.0, list.len(), |ui, range| {
        for i in range {
            let (club, n) = list[i];
            let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());
            if response.clicked() {
                state.selected = Some(club);
            }
            let base = if i % 2 == 1 { theme::ROW_ALT } else { theme::PANEL };
            let fill = if state.selected == Some(club) { theme::SELECTION } else if response.hovered() { theme::HOVER } else { base };
            ui.painter().rect_filled(rect, 4.0, fill);
            ui.painter().text(egui::pos2(rect.left() + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, t.text("leaders-place", Some(&args([("place", i + 1)]))), theme::regular(12.0), theme::MUTED);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(egui::pos2(rect.left() + 46.0, rect.center().y), egui::vec2(20.0, 20.0))));
            widgets::crest(&mut child, files, world, club, 20.0);
            let value = match state.mode {
                0 => reputation(t, n[0]),
                1 => t.text("club-ranking-record", Some(&args([("points", n[0]), ("for", n[1]), ("against", n[2])]))),
                _ => n[0].to_string(),
            };
            let name_room = egui::Rect::from_x_y_ranges(rect.left() + 62.0..=rect.right() - 120.0, rect.y_range());
            ui.painter().with_clip_rect(name_room).text(egui::pos2(name_room.left(), rect.center().y), egui::Align2::LEFT_CENTER, &world.club(club).name, theme::regular(13.0), theme::TEXT);
            ui.painter().text(egui::pos2(rect.right() - 10.0, rect.center().y), egui::Align2::RIGHT_CENTER, value, theme::bold(12.0), theme::TEXT);
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
    });
}

fn reputation(t: &Translator, r: i32) -> String {
    if (0..6).contains(&r) { t.text(&format!("teams-reputation-{r}"), None) } else { String::new() }
}

/// `hc.rW()`: the club, its numbers and the kinds of competition with points. True when
/// "troféus e títulos" was clicked.
fn details(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, mode: usize, club: ClubId) -> bool {
    let c = world.club(club);
    let mut gallery = false;
    menu::panel().fill(theme::CARD).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            widgets::crest(ui, files, world, club, 40.0);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&c.name).font(theme::bold(17.0)));
                let line = match mode {
                    0 => reputation(t, c.reputation),
                    1 => {
                        let n = numbers(world, club, 1);
                        t.text("club-ranking-record-line", Some(&args([("points", n[0]), ("for", n[1]), ("against", n[2])])))
                    }
                    _ => t.text("club-ranking-points-line", Some(&args([("points", numbers(world, club, 2)[0])]))),
                };
                ui.label(egui::RichText::new(line).font(theme::regular(13.0)).color(theme::MUTED));
            });
        });
        ui.add_space(8.0);
        gallery = controls::button(ui, &t.text("club-ranking-button-trophies", None), controls::Kind::Secondary, true).clicked();
    });
    if mode == 0 {
        return gallery;
    }
    ui.add_space(10.0);
    let (header, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::hover());
    ui.painter().rect_filled(header, 6.0, theme::HEADER);
    ui.painter().text(egui::pos2(header.left() + 8.0, header.center().y), egui::Align2::LEFT_CENTER, t.text("club-ranking-column-competition", None), theme::bold(11.0), theme::MUTED);
    ui.painter().text(egui::pos2(header.right() - 10.0, header.center().y), egui::Align2::RIGHT_CENTER, t.text("club-ranking-column-points", None), theme::bold(11.0), theme::MUTED);
    for kind in 1..=14usize {
        let value = if mode == 1 {
            let r = c.all_time_record.get(kind).cloned().unwrap_or_default();
            (r.first().copied().unwrap_or(0) > 0).then(|| t.text("club-ranking-record", Some(&args([("points", r[0]), ("for", r.get(1).copied().unwrap_or(0)), ("against", r.get(2).copied().unwrap_or(0))]))))
        } else {
            let v = c.ranking_points.get(kind).copied().unwrap_or(0);
            (v > 0).then(|| v.to_string())
        };
        if let Some(v) = value
            && kind_shown(world, club, kind)
        {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::hover());
            ui.painter().text(egui::pos2(rect.left() + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, kind_name(t, kind, c.country), theme::regular(13.0), theme::TEXT);
            ui.painter().text(egui::pos2(rect.right() - 10.0, rect.center().y), egui::Align2::RIGHT_CENTER, v, theme::bold(12.0), theme::TEXT);
        }
    }
    gallery
}
