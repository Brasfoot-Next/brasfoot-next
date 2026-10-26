//! The player search (`a.gx` with the filter panel `components.O`): filters on the left, the
//! players found on the right (`b.D`). Double click opens the player's club in Teams
//! (`best.S.a`), with the player picked.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::countries;
use brasfoot_model::ids::PlayerId;
use brasfoot_model::player::attribute;
use brasfoot_model::world::World;
use brasfoot_simulation::player_search::{PlayerFilter, VALUE_BANDS};
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::menu;
use crate::screens::new_game::{collate, country_name};
use crate::screens::widgets;
use crate::{format, motion, theme};
use crate::screens::controls;

const STRENGTH: (i32, i32) = (1, 100);
/// `QC`: the age slider; its top shows "45+" and means up to 100 (`gx.nD`).
const AGE: (i32, i32) = (16, 45);
/// The attributes' sliders in the order of `components.O` mode 2, with their labels.
const ATTRIBUTES: [(usize, &str); 7] = [
    (attribute::GOALKEEPING, "player-search-label-goalkeeping"),
    (attribute::DEFENDING, "player-search-label-tackling"),
    (attribute::SPEED, "player-search-label-speed"),
    (attribute::FINISHING, "player-search-label-finishing"),
    (attribute::PLAYMAKING, "player-search-label-playmaking"),
    (attribute::TECHNIQUE, "player-search-label-technique"),
    (attribute::PASSING, "player-search-label-passing"),
];
/// The table's attribute columns (`b.D`, mode 2): Gol, Des, Arm, Fin, Vel, Tec, Pas.
const ATTRIBUTE_COLUMNS: [(usize, &str); 7] = [
    (attribute::GOALKEEPING, "squad-column-goalkeeping"),
    (attribute::DEFENDING, "squad-column-defending"),
    (attribute::PLAYMAKING, "squad-column-playmaking"),
    (attribute::FINISHING, "squad-column-finishing"),
    (attribute::SPEED, "squad-column-speed"),
    (attribute::TECHNIQUE, "squad-column-technique"),
    (attribute::PASSING, "squad-column-passing"),
];
const TRAITS: i32 = 14;

/// The controls of `components.O` and what the last search found.
pub struct SearchState {
    name: String,
    position: Option<i32>,
    side: Option<i32>,
    strength: (i32, i32),
    age: (i32, i32),
    attributes: [(i32, i32); attribute::COUNT],
    value_band: Option<usize>,
    trait1: Option<i32>,
    trait2: Option<i32>,
    nationality: Option<i32>,
    playing_in: Option<usize>,
    star: bool,
    world_class: bool,
    for_sale: bool,
    for_loan: bool,
    /// `uK`; `None` before the first search.
    results: Option<Vec<PlayerId>>,
    selected: Option<PlayerId>,
    /// The column the table is sorted by and whether it goes down.
    sort: Option<(Column, bool)>,
}

impl Default for SearchState {
    fn default() -> Self {
        SearchState {
            name: String::new(),
            position: None,
            side: None,
            strength: STRENGTH,
            age: AGE,
            attributes: [STRENGTH; attribute::COUNT],
            value_band: None,
            trait1: None,
            trait2: None,
            nationality: None,
            playing_in: None,
            star: false,
            world_class: false,
            for_sale: false,
            for_loan: false,
            results: None,
            selected: None,
            sort: None,
        }
    }
}

impl SearchState {
    /// The screen with a search already made with no filter (the development shortcut
    /// `BF_TAB=search`).
    pub fn searched(world: &World) -> Self {
        let mut state = SearchState::default();
        state.search(world);
        state
    }

    /// `gx.rR()`: every control back to "any"; the results stay.
    fn reset(&mut self) {
        let results = self.results.take();
        let selected = self.selected.take();
        *self = SearchState { results, selected, ..SearchState::default() };
    }

    /// `gx.nD()`: the filter the controls make.
    fn filter(&self, skills: bool) -> PlayerFilter {
        let mut f = PlayerFilter {
            name: self.name.clone(),
            position: self.position,
            side: self.side,
            value_band: self.value_band,
            trait1: self.trait1,
            trait2: self.trait2,
            nationality: self.nationality,
            playing_in: self.playing_in,
            star: self.star,
            world_class: self.world_class,
            for_loan: self.for_loan,
            for_sale: self.for_sale,
            ..PlayerFilter::default()
        };
        if skills {
            for (i, &(lo, hi)) in self.attributes.iter().enumerate() {
                if lo > STRENGTH.0 || hi < STRENGTH.1 {
                    f.attributes[i] = Some((lo, hi));
                }
            }
        } else if self.strength != STRENGTH {
            f.strength = Some(self.strength);
        }
        if self.age != AGE {
            f.age = Some((self.age.0, if self.age.1 == AGE.1 { 100 } else { self.age.1 }));
        }
        f
    }

    fn search(&mut self, world: &World) {
        let found = self.filter(world.game.individual_skills).search(world);
        self.selected = found.first().copied();
        self.results = Some(found);
        self.sort = None;
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Column {
    Position,
    Name,
    Club,
    Side,
    Strength,
    Attribute(usize),
    Salary,
    Value,
    CareerGoals,
    Age,
}

pub fn window(s: &mut CareerScreen, state: &mut Option<SearchState>, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let state = state.get_or_insert_with(SearchState::default);
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "player-search", open, egui::vec2(1280.0, 720.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::SEARCH), &t.text("player-search-title", None)) {
            *open = false;
        }
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(300.0);
                filters(s, state, ui, t, files);
            });
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                results(s, state, ui, t, files, open);
            });
        });
    });
}

fn combo<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, t: &Translator, value: &mut Option<T>, items: &[(T, String)]) {
    let any = t.text("player-search-combo-any", None);
    let shown = value.and_then(|v| items.iter().find(|(x, _)| *x == v)).map(|(_, s)| s.clone()).unwrap_or(any.clone());
    egui::ComboBox::from_id_salt(id).width(ui.available_width() - 8.0).height(360.0).selected_text(shown).show_ui(ui, |ui| {
        ui.selectable_value(value, None, any);
        for (v, text) in items {
            ui.selectable_value(value, Some(*v), text);
        }
    });
}

fn range_label(t: &Translator, range: (i32, i32), top_plus: Option<i32>) -> String {
    let max = match top_plus {
        Some(top) if range.1 == top => t.text("player-search-age-max", Some(&args([("age", top)]))),
        _ => range.1.to_string(),
    };
    t.text("player-search-slider-range", Some(&args([("min", range.0.to_string()), ("max", max)])))
}

fn slider_row(ui: &mut egui::Ui, t: &Translator, label: &str, range: &mut (i32, i32), bounds: (i32, i32), top_plus: Option<i32>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).font(theme::bold(12.0)).color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(range_label(t, *range, top_plus)).font(theme::regular(12.0)));
        });
    });
    widgets::range_slider(ui, range, bounds.0, bounds.1, ui.available_width() - 8.0);
}

fn filters(s: &mut CareerScreen, state: &mut SearchState, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let skills = world.game.individual_skills;
    let mut run = false;
    egui::ScrollArea::vertical().id_salt("search-filters").max_height(574.0).auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        menu::field_label(ui, &t.text("player-search-label-name", None));
        let edit = ui.add(egui::TextEdit::singleline(&mut state.name).desired_width(ui.available_width() - 8.0).margin(egui::vec2(8.0, 6.0)));
        run |= edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.add_space(6.0);
        ui.columns(2, |cols| {
            menu::field_label(&mut cols[0], &t.text("player-search-label-position", None));
            let positions: Vec<(i32, String)> = (0..5).map(|p| (p, widgets::position_name(t, p))).collect();
            combo(&mut cols[0], "search-position", t, &mut state.position, &positions);
            menu::field_label(&mut cols[1], &t.text("player-search-label-side", None));
            let sides: Vec<(i32, String)> = (0..2).map(|x| (x, t.text(&format!("side-{x}"), None))).collect();
            combo(&mut cols[1], "search-side", t, &mut state.side, &sides);
        });
        ui.add_space(8.0);
        if skills {
            for (index, key) in ATTRIBUTES {
                slider_row(ui, t, &t.text(key, None), &mut state.attributes[index], STRENGTH, None);
            }
        } else {
            slider_row(ui, t, &t.text("player-search-label-strength", None), &mut state.strength, STRENGTH, None);
        }
        slider_row(ui, t, &t.text("player-search-label-age", None), &mut state.age, AGE, Some(AGE.1));
        ui.add_space(6.0);
        menu::field_label(ui, &t.text("player-search-label-value", None));
        let bands: Vec<(usize, String)> = (0..VALUE_BANDS.len()).map(|i| (i, t.text(&format!("player-search-value-range-{}", i + 1), None))).collect();
        combo(ui, "search-value", t, &mut state.value_band, &bands);
        ui.add_space(6.0);
        let traits: Vec<(i32, String)> = (0..TRAITS).map(|i| (i, t.text(&format!("trait-{i}"), None))).collect();
        ui.columns(2, |cols| {
            menu::field_label(&mut cols[0], &t.text("player-search-label-trait1", None));
            combo(&mut cols[0], "search-trait1", t, &mut state.trait1, &traits);
            menu::field_label(&mut cols[1], &t.text("player-search-label-trait2", None));
            combo(&mut cols[1], "search-trait2", t, &mut state.trait2, &traits);
        });
        ui.add_space(6.0);
        menu::field_label(ui, &t.text("player-search-label-nationality", None));
        let mut nations: Vec<(i32, String)> = countries::COUNTRIES.iter().map(|c| (c.id, country_name(t, c.id))).collect();
        nations.sort_by(|a, b| collate(&a.1).cmp(&collate(&b.1)));
        flag_combo(ui, files, "search-nationality", t, &mut state.nationality, &nations);
        ui.add_space(6.0);
        menu::field_label(ui, &t.text("player-search-label-playing-in", None));
        let leagues: Vec<(usize, String)> = world.game.league_countries.iter().enumerate().map(|(i, &c)| (i, country_name(t, world.country(c).country))).collect();
        combo(ui, "search-playing-in", t, &mut state.playing_in, &leagues);
        ui.add_space(8.0);
        ui.columns(2, |cols| {
            toggle(&mut cols[0], &t.text("player-search-check-star", None), &mut state.star);
            toggle(&mut cols[1], &t.text("player-search-check-world-top", None), &mut state.world_class);
            toggle(&mut cols[0], &t.text("player-search-check-for-sale", None), &mut state.for_sale);
            toggle(&mut cols[1], &t.text("player-search-check-loan", None), &mut state.for_loan);
        });
    });
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if controls::button(ui, &t.text("player-search-button-search", None), controls::Kind::Primary, true).clicked() {
            run = true;
        }
        ui.add_space(6.0);
        if controls::button(ui, &t.text("player-search-button-reset-filters", None), controls::Kind::Secondary, true).clicked() {
            state.reset();
        }
    });
    if run {
        state.search(&s.career.world);
    }
}

/// A combo of countries with their flags (`a.aJ`).
fn flag_combo(ui: &mut egui::Ui, files: &GameFiles, id: &str, t: &Translator, value: &mut Option<i32>, items: &[(i32, String)]) {
    let any = t.text("player-search-combo-any", None);
    let shown = value.and_then(|v| items.iter().find(|(x, _)| *x == v)).map(|(_, s)| s.clone()).unwrap_or(any.clone());
    egui::ComboBox::from_id_salt(id).width(ui.available_width() - 8.0).height(360.0).selected_text(shown).show_ui(ui, |ui| {
        ui.selectable_value(value, None, any);
        for (v, text) in items {
            ui.horizontal(|ui| {
                widgets::flag(ui, files, *v, 16.0);
                ui.selectable_value(value, Some(*v), text);
            });
        }
    });
}

fn toggle(ui: &mut egui::Ui, text: &str, value: &mut bool) {
    ui.horizontal(|ui| {
        let r = menu::check(ui, *value, true);
        let label = ui.add(egui::Label::new(egui::RichText::new(text).size(13.0)).sense(egui::Sense::click()));
        if r.clicked() || label.clicked() {
            *value = !*value;
        }
    });
}

fn results(s: &mut CareerScreen, state: &mut SearchState, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, open: &mut bool) {
    let world = &s.career.world;
    let skills = world.game.individual_skills;
    let count = state.results.as_ref().map_or(0, |r| r.len());
    let label = match (&state.results, count) {
        (None, _) => t.text("player-search-hint", None),
        (Some(_), 0) => t.text("player-search-result-none", None),
        (Some(_), n) => t.text("player-search-result-many", Some(&args([("count", n)]))),
    };
    ui.label(egui::RichText::new(label).font(theme::regular(13.0)).color(theme::MUTED));
    ui.add_space(6.0);

    let mut columns: Vec<(Option<Column>, String, f32)> = vec![
        (Some(Column::Position), t.text("player-search-column-position", None), 26.0),
        (None, String::new(), 24.0),
        (Some(Column::Name), t.text("player-search-column-name", None), 0.0),
        (Some(Column::Club), t.text("player-search-column-club", None), 150.0),
        (Some(Column::Side), t.text("player-search-column-side", None), 24.0),
    ];
    if skills {
        for (index, key) in ATTRIBUTE_COLUMNS {
            columns.push((Some(Column::Attribute(index)), t.text(key, None), 34.0));
        }
    } else {
        columns.push((Some(Column::Strength), t.text("player-search-column-strength", None), 30.0));
    }
    columns.extend([
        (Some(Column::Salary), t.text("player-search-column-salary", None), 62.0),
        (Some(Column::Value), t.text("player-search-column-value", None), 58.0),
        (None, t.text("player-search-column-traits", None), 62.0),
        (Some(Column::CareerGoals), t.text("player-search-column-cards-goals", None), 32.0),
        (Some(Column::Age), t.text("player-search-column-age", None), 38.0),
        (None, t.text("player-search-column-for-sale", None), 22.0),
        (None, t.text("player-search-column-loan", None), 22.0),
    ]);
    // the name takes what is left; on a narrow window the club gives up room first
    let room = ui.available_width() - 12.0;
    let fixed: f32 = columns.iter().map(|c| c.2).sum();
    let club_width = (150.0 - (fixed + 110.0 - room).max(0.0)).max(70.0);
    let name_width = (room - (fixed - 150.0 + club_width)).max(110.0);
    for c in &mut columns {
        match c.0 {
            Some(Column::Name) => c.2 = name_width,
            Some(Column::Club) => c.2 = club_width,
            _ => {}
        }
    }
    let total: f32 = columns.iter().map(|c| c.2).sum();

    // the header: a click sorts by that column, a second click turns the order around
    let (header, _) = ui.allocate_exact_size(egui::vec2(total, 26.0), egui::Sense::hover());
    ui.painter().rect_filled(header, 6.0, theme::HEADER);
    let mut x = header.left();
    for (column, title, w) in &columns {
        let cell = egui::Rect::from_min_size(egui::pos2(x, header.top()), egui::vec2(*w, header.height()));
        x += w;
        let Some(column) = *column else {
            ui.painter().text(cell.center(), egui::Align2::CENTER_CENTER, title, theme::bold(11.0), theme::MUTED);
            continue;
        };
        let response = ui.interact(cell, ui.make_persistent_id(("search-header", title.as_str())), egui::Sense::click());
        let active = state.sort.is_some_and(|(c, _)| c == column);
        let color = if active || response.hovered() { theme::TEXT } else { theme::MUTED };
        ui.painter().text(cell.center(), egui::Align2::CENTER_CENTER, title, theme::bold(11.0), color);
        if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            let down = match state.sort {
                Some((c, down)) if c == column => !down,
                _ => !matches!(column, Column::Name | Column::Club | Column::Position | Column::Side),
            };
            state.sort = Some((column, down));
            if let Some(list) = &mut state.results {
                sort(world, list, column, down);
            }
        }
    }
    ui.add_space(2.0);

    let Some(list) = &state.results else { return };
    let mut opened = None;
    let ctx = ui.ctx().clone();
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical().id_salt("search-results").auto_shrink([false, false]).show_rows(ui, 24.0, list.len(), |ui, rows| {
        for i in rows {
            let p = list[i];
            let pl = world.player(p);
            let (rect, response) = ui.allocate_exact_size(egui::vec2(total, 24.0), egui::Sense::click());
            if response.clicked() {
                state.selected = Some(p);
            }
            if response.double_clicked() {
                opened = Some(p);
            }
            let hover = motion::toward(&ctx, response.id, response.hovered(), motion::HOVER);
            let selected = state.selected == Some(p);
            let base = if i % 2 == 1 { theme::ROW_ALT } else { theme::PANEL };
            let fill = if selected { theme::SELECTION } else { motion::mix(base, theme::HOVER, hover) };
            let painter = ui.painter();
            painter.rect_filled(rect, 4.0, fill);
            let mut x = rect.left();
            let mut cells = columns.iter().map(|c| c.2);
            let mut next = || {
                let w = cells.next().unwrap_or(0.0);
                let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(w, rect.height()));
                x += w;
                cell
            };
            let center = |cell: egui::Rect, text: String| {
                painter.text(cell.center(), egui::Align2::CENTER_CENTER, text, theme::regular(11.0), theme::TEXT);
            };
            center(next(), widgets::position_short(t, pl.position));
            let cell = next();
            if let Some(uri) = files.jar_image(&ctx, &format!("aflags/{}.png", pl.country)) {
                egui::Image::new(uri).paint_at(ui, egui::Rect::from_center_size(cell.center(), egui::vec2(16.0, 16.0)));
            }
            let cell = next();
            let star = if pl.world_class { Some("aicons/estrelared.png") } else if pl.star { Some("aicons/estrela.png") } else { None };
            let mut name_x = cell.left() + 4.0;
            if let Some(star) = star
                && let Some(uri) = files.jar_image(&ctx, star)
            {
                egui::Image::new(uri).paint_at(ui, egui::Rect::from_center_size(egui::pos2(name_x + 6.0, cell.center().y), egui::vec2(12.0, 12.0)));
                name_x += 16.0;
            }
            painter.with_clip_rect(cell.shrink2(egui::vec2(2.0, 0.0))).text(egui::pos2(name_x, cell.center().y), egui::Align2::LEFT_CENTER, &pl.name, theme::regular(12.0), theme::TEXT);
            let cell = next();
            let club = pl.club.map(|c| world.club(c).name.clone()).unwrap_or_default();
            painter.with_clip_rect(cell.shrink2(egui::vec2(2.0, 0.0))).text(egui::pos2(cell.left() + 4.0, cell.center().y), egui::Align2::LEFT_CENTER, club, theme::regular(11.0), theme::MUTED);
            center(next(), widgets::side_short(t, pl.side));
            if skills {
                for (index, _) in ATTRIBUTE_COLUMNS {
                    center(next(), pl.attributes[index].to_string());
                }
            } else {
                center(next(), pl.strength.to_string());
            }
            center(next(), format::compact_money(t, pl.salary as i64));
            center(next(), format::compact_money(t, pl.market_value as i64));
            center(next(), format!("{}/{}", widgets::trait_short(t, pl.trait1), widgets::trait_short(t, pl.trait2)));
            center(next(), pl.career_goals.to_string());
            center(next(), pl.age.to_string());
            for on in [pl.listed_for_sale, pl.listed_for_loan] {
                let cell = next();
                if on && let Some(uri) = files.jar_image(&ctx, "aicons/iconchecked.png") {
                    egui::Image::new(uri).paint_at(ui, egui::Rect::from_center_size(cell.center(), egui::vec2(14.0, 14.0)));
                }
            }
            if response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
    });
    // gx.rQ(): the player's club in Teams
    if let Some(p) = opened
        && let Some(club) = world.player(p).club
    {
        s.viewed_club = Some(club);
        s.teams_selected_player = Some(p);
        s.teams_open = true;
        *open = false;
    }
}

/// The header's sorts (`b.D` is sortable by column; ties keep the list's order).
fn sort(world: &World, list: &mut [PlayerId], column: Column, down: bool) {
    list.sort_by(|&a, &b| {
        let (x, y) = (world.player(a), world.player(b));
        let order = match column {
            Column::Position => x.position.cmp(&y.position),
            Column::Name => collate(&x.name).cmp(&collate(&y.name)),
            Column::Club => {
                let name = |c: Option<brasfoot_model::ids::ClubId>| c.map(|c| collate(&world.club(c).name)).unwrap_or_default();
                name(x.club).cmp(&name(y.club))
            }
            Column::Side => x.side.cmp(&y.side),
            Column::Strength => x.strength.cmp(&y.strength),
            Column::Attribute(i) => x.attributes[i].cmp(&y.attributes[i]),
            Column::Salary => x.salary.cmp(&y.salary),
            Column::Value => x.market_value.cmp(&y.market_value),
            Column::CareerGoals => x.career_goals.cmp(&y.career_goals),
            Column::Age => x.age.cmp(&y.age),
        };
        if down { order.reverse() } else { order }
    });
}
