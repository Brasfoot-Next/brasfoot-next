//! The leaders: top scorers (`a.o`: a competition's scorers, `at.ms()` sorted by `au`, or the
//! best of all time, `o.mR()`) and assists (`a.y`: the season's assists of a country's players
//! or of all, `y.mT()`). Both list at most 401 (`b.d`, `b.e`).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, CompetitionId};
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::new_game::country_name;
use crate::screens::widgets;
use crate::{motion, theme, view};
use crate::screens::controls;

/// `o.mR()` and `y.mT()` stop after the 401st.
const SHOWN: usize = 401;

#[derive(Clone, Copy, PartialEq)]
pub enum Board {
    Scorers,
    Assists,
}

pub struct LeadersState {
    pub board: Board,
    /// Scorers: the competition of the combo; `None` is "Melhores de sempre".
    competition: Option<CompetitionId>,
    /// Assists: an index of the league countries (`SR.N()`); `None` is "Internacional".
    country: Option<usize>,
}

impl LeadersState {
    /// `new o(...)` picks the first competition; `new y(dialog, club)` the club's country
    /// (`y.cD(ah.getPais())`).
    pub fn new(world: &World, board: Board, club: ClubId) -> Self {
        let competition = view::competitions(world).first().copied();
        let code = world.club(club).country;
        let country = world.game.league_countries.iter().position(|&k| world.country(k).country == code);
        LeadersState { board, competition, country }
    }
}

/// A row: name, club, the number counted and games (`best.d`).
struct Row {
    name: String,
    club: Option<ClubId>,
    count: i32,
    games: i32,
}

/// `a.p`, `a.z`, `best.au`: more first, then fewer games; Java's sort keeps the rest in order.
fn sorted(mut rows: Vec<Row>) -> Vec<Row> {
    rows.sort_by(|a, b| b.count.cmp(&a.count).then(a.games.cmp(&b.games)));
    rows.truncate(SHOWN);
    rows
}

fn rows(world: &World, state: &LeadersState) -> Vec<Row> {
    match state.board {
        Board::Scorers => match state.competition {
            Some(c) => view::top_scorers(world, c, usize::MAX)
                .into_iter()
                .map(|(p, goals, games)| Row { name: world.player(p).name.clone(), club: world.player(p).club, count: goals, games })
                .collect(),
            // o.mR(): the players with career goals (`gy()`, `gz()`), then the retired (`SR.bJ()`)
            None => sorted(
                world
                    .game
                    .players
                    .iter()
                    .map(|&p| world.player(p))
                    .filter(|x| x.career_goals > 0)
                    .map(|x| Row { name: x.name.clone(), club: x.club, count: x.career_goals, games: x.career_games })
                    .chain(world.game.retired_idols.iter().map(|r| Row { name: r.name.clone(), club: None, count: r.goals, games: r.games }))
                    .collect(),
            ),
        },
        Board::Assists => {
            let code = state.country.and_then(|i| world.game.league_countries.get(i)).map(|&k| world.country(k).country);
            let season = world.season();
            sorted(
                world
                    .game
                    .players
                    .iter()
                    .map(|&p| world.player(p))
                    .filter(|x| code.is_none_or(|code| x.club.is_some_and(|c| world.club(c).country == code)))
                    .filter_map(|x| {
                        // F.gs(), F.gu(): the season's assists and games
                        let (assists, _, games) = x.season_totals(season);
                        (assists > 0).then(|| Row { name: x.name.clone(), club: x.club, count: assists, games })
                    })
                    .collect(),
            )
        }
    }
}

pub fn window(s: &mut CareerScreen, state: &mut LeadersState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let world = &s.career.world;
    motion::dialog(ctx, "leaders", open, egui::vec2(760.0, 640.0), frame, |ui, open| {
        let title = if state.board == Board::Scorers { "top-scorers-title" } else { "assists-title" };
        if widgets::dialog_title(ui, files, None, &t.text(title, None)) {
            *open = false;
        }
        ui.horizontal(|ui| match state.board {
            Board::Scorers => {
                let shown = state.competition.map(|c| widgets::competition_name(t, world, c)).unwrap_or_else(|| t.text("top-scorers-button-all-time", None));
                egui::ComboBox::from_id_salt("scorers-competition").width(320.0).height(400.0).selected_text(shown).show_ui(ui, |ui| {
                    for c in view::competitions(world) {
                        ui.selectable_value(&mut state.competition, Some(c), widgets::competition_name(t, world, c));
                    }
                });
                ui.add_space(10.0);
                // "Melhores de sempre de todas competições"
                if controls::button(ui, &t.text("top-scorers-button-all-time", None), controls::Kind::Secondary, state.competition.is_some()).clicked() {
                    state.competition = None;
                }
            }
            Board::Assists => {
                let shown = state.country.map(|i| country_name(t, world.country(world.game.league_countries[i]).country)).unwrap_or_else(|| t.text("coach-ranking-combo-international", None));
                egui::ComboBox::from_id_salt("assists-country").width(240.0).height(400.0).selected_text(shown).show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.country, None, t.text("coach-ranking-combo-international", None));
                    for (i, &k) in world.game.league_countries.iter().enumerate() {
                        let code = world.country(k).country;
                        ui.horizontal(|ui| {
                            widgets::flag(ui, files, code, 16.0);
                            ui.selectable_value(&mut state.country, Some(i), country_name(t, code));
                        });
                    }
                });
            }
        });
        ui.add_space(10.0);
        let list = rows(world, state);
        let count_key = if state.board == Board::Scorers { "top-scorers-column-goals" } else { "assists-column-assists" };
        table(ui, t, files, world, &list, count_key);
    });
}

fn table(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, list: &[Row], count_key: &str) {
    let width = ui.available_width();
    let columns = [(String::new(), 44.0), (t.text("top-scorers-column-player", None), 0.0), (t.text("top-scorers-column-team", None), 0.0), (t.text(count_key, None), 70.0), (t.text("top-scorers-column-games", None), 70.0)];
    let flexible = (width - 44.0 - 140.0) / 2.0;
    let widths: Vec<f32> = columns.iter().map(|c| if c.1 == 0.0 { flexible } else { c.1 }).collect();
    let (header, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
    ui.painter().rect_filled(header, 6.0, theme::HEADER);
    let mut x = header.left();
    for ((title, _), w) in columns.iter().zip(&widths) {
        let cell = egui::Rect::from_min_size(egui::pos2(x, header.top()), egui::vec2(*w, header.height()));
        ui.painter().text(egui::pos2(cell.left() + 8.0, cell.center().y), egui::Align2::LEFT_CENTER, title, theme::bold(11.0), theme::MUTED);
        x += w;
    }
    ui.add_space(2.0);
    if list.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| ui.label(egui::RichText::new(t.text("leaders-empty", None)).color(theme::MUTED)));
        return;
    }
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical().id_salt("leaders-rows").auto_shrink([false, false]).show_rows(ui, 26.0, list.len(), |ui, range| {
        for i in range {
            let row = &list[i];
            let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
            let base = if i % 2 == 1 { theme::ROW_ALT } else { theme::PANEL };
            ui.painter().rect_filled(rect, 4.0, if response.hovered() { theme::HOVER } else { base });
            let mut x = rect.left();
            let place = t.text("leaders-place", Some(&args([("place", i + 1)])));
            let cells = [place, row.name.clone(), row.club.map(|c| world.club(c).name.clone()).unwrap_or_default(), row.count.to_string(), row.games.to_string()];
            for (j, (text, w)) in cells.iter().zip(&widths).enumerate() {
                let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*w, rect.height()));
                x += w;
                let mut left = cell.left() + 8.0;
                if j == 2
                    && let Some(c) = row.club
                {
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(egui::pos2(left + 9.0, cell.center().y), egui::vec2(18.0, 18.0))));
                    widgets::crest(&mut child, files, world, c, 18.0);
                    left += 24.0;
                }
                let font = if j == 3 { theme::bold(13.0) } else { theme::regular(13.0) };
                let color = if j == 0 || j == 4 { theme::MUTED } else { theme::TEXT };
                ui.painter().with_clip_rect(cell.shrink2(egui::vec2(2.0, 0.0))).text(egui::pos2(left, cell.center().y), egui::Align2::LEFT_CENTER, text, font, color);
            }
        }
    });
}
