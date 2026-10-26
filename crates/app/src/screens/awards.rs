//! The awards window (`bfmod.CerimoniaDialog`): the season's Ballon d'Or and the other awards,
//! the team of the year, the winners of every season, the clubs' history and the Hall of Fame.
//! It opens by itself when a season ends (`BolaDeOuro.fimDeTemporada`, button "Continuar") and
//! from the toolbar's "Prêmios" (`BolaDeOuro.mostrarHistorico`, on the winners).

use std::collections::HashMap;

use brasfoot_i18n::{Translator, args, labels};
use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::{AwardExtra, AwardKind, ClubSeason, Idol, RecordKind, TeamSlot};
use brasfoot_model::world::World;
use brasfoot_systems::awards::coaches::{self, CoachCandidate};
use brasfoot_systems::awards::season::{Candidate, Season};
use brasfoot_systems::awards::{clubs, hall_of_fame, history};
use eframe::egui::{self, Color32, Stroke, pos2, vec2};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::modkit::{Cell, GOLD, GOLD_DARK, tab_button, table};
use crate::screens::{menu, widgets};
use crate::{format, motion, theme};

/// `LARGURA`, `ALTURA`
const SIZE: egui::Vec2 = vec2(980.0, 700.0);
/// The header's gradient starts in this dark gold.
const HEADER_GOLD: Color32 = Color32::from_rgb(48, 38, 12);

/// The tabs, in their order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    BallonDor,
    GoldenShoe,
    GoldenGlove,
    GoldenBoy,
    Coach,
    TeamOfTheYear,
    Winners,
    Clubs,
    HallOfFame,
    SuperMundial,
}

const TABS: [(Tab, &str); 10] = [
    (Tab::BallonDor, "awards-ceremony-tab-ballon-dor"),
    (Tab::GoldenShoe, "awards-ceremony-tab-golden-shoe"),
    (Tab::GoldenGlove, "awards-ceremony-tab-golden-glove"),
    (Tab::GoldenBoy, "awards-ceremony-tab-golden-boy"),
    (Tab::Coach, "awards-ceremony-tab-coach"),
    (Tab::TeamOfTheYear, "awards-ceremony-tab-team-of-year"),
    (Tab::Winners, "awards-ceremony-tab-winners"),
    (Tab::Clubs, "awards-ceremony-tab-clubs"),
    (Tab::HallOfFame, "awards-ceremony-tab-hall-of-fame"),
    (Tab::SuperMundial, "awards-ceremony-tab-super-world-cup"),
];

/// The Hall of Fame's tabs; the last two need systems not ported yet (`Joias`, `Motor`).
const HALL_TABS: [&str; 6] = [
    "awards-ceremony-hall-tab-gallery",
    "awards-ceremony-hall-tab-scorers",
    "awards-ceremony-hall-tab-legends",
    "awards-ceremony-hall-tab-records",
    "awards-ceremony-hall-tab-gems",
    "awards-ceremony-hall-tab-squads",
];
const HALL_TABS_PORTED: usize = 4;

/// What the Clubes tab shows: the human coach's career or a club.
#[derive(Clone, Copy, PartialEq)]
enum ClubsChoice {
    Career,
    Club(ClubId),
}

pub struct AwardsState {
    season: Season,
    coaches: Vec<CoachCandidate>,
    /// Opened at the season's end: the button says "Continuar".
    announcing: bool,
    tab: Tab,
    hall_tab: usize,
    /// The season of the team of the year: None for the one shown.
    team_year: Option<i32>,
    clubs: Option<ClubsChoice>,
}

impl AwardsState {
    /// `CerimoniaDialog.mostrar(owner, t, anuncio, aba)`
    pub fn new(world: &World, season: Season, announcing: bool, tab: Tab) -> Self {
        let coaches = coaches::ranking(world, &season);
        AwardsState { season, coaches, announcing, tab, hall_tab: 0, team_year: None, clubs: None }
    }

    /// The tab and the Hall of Fame's tab by their index.
    pub fn select(&mut self, tab: usize, hall_tab: usize) {
        self.tab = TABS.get(tab).map_or(self.tab, |t| t.0);
        self.hall_tab = hall_tab.min(HALL_TABS_PORTED - 1);
    }
}

pub fn window(s: &mut CareerScreen, state: &mut AwardsState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(0);
    let world = &s.career.world;
    motion::dialog(ctx, "awards", open, SIZE, frame, |ui, open| {
        header(ui, t, files, world, state);
        egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 10, bottom: 0 }).show(ui, |ui| {
            tabs(ui, t, state);
            ui.add_space(8.0);
            let body = ui.available_height() - 52.0;
            ui.allocate_ui(vec2(ui.available_width(), body), |ui| {
                ui.set_min_height(body);
                match state.tab {
                    Tab::BallonDor => {
                        let rows = ballon_rows(t, world, &state.season.ballon_dor(30));
                        table(ui, files, "awards-ballon", &columns(t, BALLON_COLUMNS), &rows, true);
                    }
                    Tab::GoldenShoe => {
                        let rows = shoe_rows(t, world, &state.season.golden_shoe(20));
                        table(ui, files, "awards-shoe", &columns(t, SHOE_COLUMNS), &rows, true);
                    }
                    Tab::GoldenGlove => {
                        let rows = glove_rows(t, world, &state.season.golden_glove(world, 15));
                        table(ui, files, "awards-glove", &columns(t, GLOVE_COLUMNS), &rows, true);
                    }
                    Tab::GoldenBoy => {
                        let rows = ballon_rows(t, world, &state.season.golden_boy(15));
                        table(ui, files, "awards-golden-boy", &columns(t, BALLON_COLUMNS), &rows, true);
                    }
                    Tab::Coach => coach_tab(ui, t, files, world, state),
                    Tab::TeamOfTheYear => team_tab(ui, t, files, world, state),
                    Tab::Winners => {
                        let rows = winners_rows(t, world);
                        table(ui, files, "awards-winners", &columns(t, WINNERS_COLUMNS), &rows, false);
                    }
                    Tab::Clubs => clubs_tab(ui, t, files, world, state),
                    Tab::HallOfFame => hall_tab(ui, t, files, world, state),
                    Tab::SuperMundial => {}
                }
            });
            // the bottom bar: Continuar or Fechar on the right
            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let key = if state.announcing { "awards-ceremony-button-continue" } else { "awards-ceremony-button-close" };
                if menu::action_button(ui, &t.text(key, None), true, true).clicked() {
                    *open = false;
                }
            });
        });
        if ui.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter)) {
            *open = false;
        }
    });
}

/// `cabecalho(Temporada, boolean)`: the Ballon d'Or of the season, the other winners below.
fn header(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, state: &AwardsState) {
    let season = &state.season;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 176.0), egui::Sense::hover());
    let painter = ui.painter();
    // the gradient from the dark gold to the background, a gold line at the bottom
    let mut mesh = egui::Mesh::default();
    let i = mesh.vertices.len() as u32;
    mesh.colored_vertex(rect.left_top(), HEADER_GOLD);
    mesh.colored_vertex(rect.right_top(), theme::PANEL);
    mesh.colored_vertex(rect.right_bottom(), theme::PANEL);
    mesh.colored_vertex(rect.left_bottom(), motion::mix(HEADER_GOLD, theme::PANEL, 0.5));
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
    painter.add(egui::Shape::mesh(mesh));
    painter.rect_filled(egui::Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.right_bottom()), 0.0, GOLD_DARK);
    let ball = egui::Rect::from_min_size(rect.min + vec2(20.0, 26.0), vec2(120.0, 120.0));
    widgets::paint_jar_image(ui, files, "aicons/bolaouro.png", ball);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(pos2(ball.right() + 18.0, rect.top() + 14.0), rect.right_bottom() - vec2(20.0, 14.0))));
    let ui = &mut child;
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.label(egui::RichText::new(t.text("awards-ceremony-header-title", Some(&args([("year", season.year.to_string())])))).font(theme::bold(15.0)).color(GOLD));
    let subtitle = if state.announcing { "awards-ceremony-header-announce" } else { "awards-ceremony-header-leader" };
    ui.label(egui::RichText::new(t.text(subtitle, None)).font(theme::regular(12.0)).color(theme::MUTED));
    let Some(v) = season.ballon_dor(1).into_iter().next() else {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(t.text("awards-ceremony-header-nobody", None)).font(theme::regular(13.0)).color(theme::TEXT));
        return;
    };
    let p = world.player(v.player);
    ui.label(egui::RichText::new(&p.name).font(theme::bold(28.0)).color(Color32::WHITE));
    ui.horizontal(|ui| {
        widgets::flag(ui, files, p.country, 18.0);
        ui.add_space(4.0);
        let line = args([("club", club_name(t, world, v)), ("position", widgets::position_name(t, p.position)), ("age", v.age.to_string())]);
        ui.label(egui::RichText::new(t.text("awards-ceremony-header-club-line", Some(&line))).font(theme::regular(14.0)).color(theme::TEXT));
    });
    let stats = args([
        ("games", v.games.to_string()),
        ("goals", v.goals.to_string()),
        ("assists", v.assists.to_string()),
        ("rating", format::rating_two(t, v.rating)),
    ]);
    ui.label(egui::RichText::new(t.text("awards-ceremony-header-stats", Some(&stats))).font(theme::bold(13.0)).color(GOLD));
    let mut others = Vec::new();
    if let Some(c) = season.golden_shoe(1).first() {
        others.push(t.text("awards-ceremony-header-golden-shoe", Some(&args([("player", world.player(c.player).name.clone()), ("goals", c.goals.to_string())]))));
    }
    if let Some(c) = season.golden_glove(world, 1).first() {
        others.push(t.text("awards-ceremony-header-golden-glove", Some(&args([("player", world.player(c.player).name.clone())]))));
    }
    if let Some(c) = season.golden_boy(1).first() {
        others.push(t.text("awards-ceremony-header-golden-boy", Some(&args([("player", world.player(c.player).name.clone())]))));
    }
    if let Some(c) = state.coaches.first() {
        others.push(t.text("awards-ceremony-header-coach", Some(&args([("name", world.coach(c.coach).name.clone())]))));
    }
    if !others.is_empty() {
        ui.add(egui::Label::new(egui::RichText::new(others.join("   ·   ")).font(theme::regular(12.0)).color(theme::MUTED)).wrap());
    }
}

/// The tab strip: the selected tab in gold. The Super Mundial is not ported yet.
fn tabs(ui: &mut egui::Ui, t: &Translator, state: &mut AwardsState) {
    let not_ported = t.text("toolbar-not-ported", None);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (tab, key) in TABS {
            let enabled = tab != Tab::SuperMundial;
            if tab_button(ui, &t.text(key, None), state.tab == tab, enabled, &not_ported).clicked() {
                state.tab = tab;
            }
        }
    });
}


/// A table column: its title's key and the original's preferred width.
type ColumnSpec = (&'static str, f32);

const BALLON_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-rank", 34.0),
    ("awards-ceremony-column-player", 210.0),
    ("awards-ceremony-column-position", 90.0),
    ("awards-ceremony-column-age", 50.0),
    ("awards-ceremony-column-club", 190.0),
    ("awards-ceremony-column-games-short", 40.0),
    ("awards-ceremony-column-goals-short", 40.0),
    ("awards-ceremony-column-assists-short", 40.0),
    ("awards-ceremony-column-rating", 55.0),
    ("awards-ceremony-column-points", 60.0),
];
const SHOE_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-rank", 34.0),
    ("awards-ceremony-column-player", 230.0),
    ("awards-ceremony-column-age", 50.0),
    ("awards-ceremony-column-club", 210.0),
    ("awards-ceremony-column-league-weight", 90.0),
    ("awards-ceremony-column-games-short", 45.0),
    ("awards-ceremony-column-goals", 50.0),
    ("awards-ceremony-column-points", 70.0),
];
const GLOVE_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-rank", 34.0),
    ("awards-ceremony-column-player", 250.0),
    ("awards-ceremony-column-age", 50.0),
    ("awards-ceremony-column-club", 250.0),
    ("awards-ceremony-column-games-short", 45.0),
    ("awards-ceremony-column-rating", 60.0),
    ("awards-ceremony-column-points", 60.0),
];
const COACH_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-rank", 34.0),
    ("awards-ceremony-column-coach", 190.0),
    ("awards-ceremony-column-club", 170.0),
    ("awards-ceremony-column-campaign", 190.0),
    ("awards-ceremony-column-finish", 45.0),
    ("awards-ceremony-column-expected", 65.0),
    ("awards-ceremony-column-titles", 200.0),
    ("awards-ceremony-column-points", 55.0),
];
const WINNERS_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-year", 45.0),
    ("awards-ceremony-column-winner-ballon-dor", 180.0),
    ("awards-ceremony-column-winner-golden-shoe", 180.0),
    ("awards-ceremony-column-winner-golden-glove", 170.0),
    ("awards-ceremony-column-winner-golden-boy", 170.0),
    ("awards-ceremony-column-winner-coach", 180.0),
];
const CLUB_HISTORY_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-year", 50.0),
    ("awards-ceremony-column-league", 180.0),
    ("awards-ceremony-column-finish", 45.0),
    ("awards-ceremony-column-titles", 290.0),
    ("awards-ceremony-column-top-scorer", 180.0),
    ("awards-ceremony-column-coach", 160.0),
];
const CAREER_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-year", 50.0),
    ("awards-ceremony-column-club", 190.0),
    ("awards-ceremony-column-league", 180.0),
    ("awards-ceremony-column-finish", 45.0),
    ("awards-ceremony-column-titles", 260.0),
    ("awards-ceremony-column-top-scorer", 180.0),
];
const GALLERY_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-player", 240.0),
    ("awards-ceremony-column-balls", 60.0),
    ("awards-ceremony-column-boots", 75.0),
    ("awards-ceremony-column-gloves", 60.0),
    ("awards-ceremony-column-golden-boys", 90.0),
    ("awards-ceremony-column-teams-of-year", 70.0),
    ("awards-ceremony-column-status", 230.0),
];
const SCORERS_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-rank", 34.0),
    ("awards-ceremony-column-player", 220.0),
    ("awards-ceremony-column-position", 80.0),
    ("awards-ceremony-column-games-short", 50.0),
    ("awards-ceremony-column-goals", 55.0),
    ("awards-ceremony-column-assists-short", 50.0),
    ("awards-ceremony-column-goals-per-game", 70.0),
    ("awards-ceremony-column-status", 230.0),
];
const LEGENDS_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-player", 200.0),
    ("awards-ceremony-column-position", 75.0),
    ("awards-ceremony-column-retired-year", 70.0),
    ("awards-ceremony-column-last-club", 170.0),
    ("awards-ceremony-column-games-short", 45.0),
    ("awards-ceremony-column-goals-short", 45.0),
    ("awards-ceremony-column-assists-short", 45.0),
    ("awards-ceremony-column-rating", 50.0),
    ("awards-ceremony-column-awards", 200.0),
];
const RECORDS_COLUMNS: &[ColumnSpec] = &[
    ("awards-ceremony-column-record", 320.0),
    ("awards-ceremony-column-rank", 30.0),
    ("awards-ceremony-column-player", 220.0),
    ("awards-ceremony-column-club", 200.0),
    ("awards-ceremony-column-year", 55.0),
    ("awards-ceremony-column-mark", 60.0),
];

fn columns(t: &Translator, specs: &[ColumnSpec]) -> Vec<(String, f32)> {
    specs.iter().map(|(key, w)| (t.text(key, None), *w)).collect()
}






/// `nomeClube(Candidato)`
fn club_name(t: &Translator, world: &World, c: &Candidate) -> String {
    c.club.map(|k| world.club(k).name.clone()).unwrap_or_else(|| t.text("awards-ceremony-no-club", None))
}

/// `inteiro(double)`: `%.0f`.
fn whole(v: f64) -> String {
    format!("{v:.0}")
}

fn ordinal(t: &Translator, n: i32) -> String {
    t.text("awards-ceremony-ordinal", Some(&args([("n", n.to_string())])))
}

/// `tabelaBola`
fn ballon_rows(t: &Translator, world: &World, list: &[&Candidate]) -> Vec<Vec<Cell>> {
    list.iter()
        .enumerate()
        .map(|(i, c)| {
            let p = world.player(c.player);
            vec![
                Cell::from(i + 1),
                Cell::flagged(&p.name, p.country),
                widgets::position_name(t, p.position).into(),
                c.age.into(),
                club_name(t, world, c).into(),
                c.games.into(),
                c.goals.into(),
                c.assists.into(),
                format::rating_two(t, c.rating).into(),
                whole(c.points).into(),
            ]
        })
        .collect()
}

/// `tabelaChuteira`
fn shoe_rows(t: &Translator, world: &World, list: &[&Candidate]) -> Vec<Vec<Cell>> {
    list.iter()
        .enumerate()
        .map(|(i, c)| {
            let p = world.player(c.player);
            vec![
                Cell::from(i + 1),
                Cell::flagged(&p.name, p.country),
                c.age.into(),
                club_name(t, world, c).into(),
                t.text("awards-ceremony-league-weight", Some(&args([("weight", format::rating(t, c.league_weight))]))).into(),
                c.games.into(),
                c.goals.into(),
                format::rating(t, c.weighted_goals()).into(),
            ]
        })
        .collect()
}

/// `tabelaLuva`
fn glove_rows(t: &Translator, world: &World, list: &[&Candidate]) -> Vec<Vec<Cell>> {
    list.iter()
        .enumerate()
        .map(|(i, c)| {
            let p = world.player(c.player);
            vec![
                Cell::from(i + 1),
                Cell::flagged(&p.name, p.country),
                c.age.into(),
                club_name(t, world, c).into(),
                c.games.into(),
                format::rating_two(t, c.rating).into(),
                whole(c.points).into(),
            ]
        })
        .collect()
}

/// `tabelaTecnicos`: the top 20; a past season shows the recorded coach of the year.
fn coach_tab(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, state: &AwardsState) {
    let mut rows: Vec<Vec<Cell>> = state
        .coaches
        .iter()
        .take(20)
        .enumerate()
        .map(|(i, c)| {
            let club = world.club(c.club);
            let k = c.campaign.as_ref();
            vec![
                Cell::from(i + 1),
                Cell::flagged(&world.coach(c.coach).name, club.country),
                club.name.clone().into(),
                k.map(|k| labels::league(t, &k.league)).unwrap_or_default().into(),
                k.map(|k| ordinal(t, k.position)).unwrap_or_default().into(),
                k.map(|k| ordinal(t, k.expected)).unwrap_or_default().into(),
                labels::season_titles(t, &c.titles).into(),
                whole(c.points).into(),
            ]
        })
        .collect();
    if rows.is_empty() {
        for a in history::of_kind(&world.mods.awards, AwardKind::CoachOfTheYear) {
            if a.year == state.season.year {
                let extra = match &a.extra {
                    AwardExtra::Coach(summary) => labels::coach_summary(t, summary),
                    _ => String::new(),
                };
                rows.push(vec![Cell::from(1), Cell::flagged(&a.name, a.country), a.club.clone().unwrap_or_default().into(), extra.into()]);
            }
        }
    }
    if rows.is_empty() {
        rows.push(vec![Cell::default(), t.text("awards-ceremony-coach-unavailable", None).into()]);
    }
    table(ui, files, "awards-coach", &columns(t, COACH_COLUMNS), &rows, true);
}

/// `tabelaPremiados`: one row per year, newest first; a kind won twice in a year shows the last.
fn winners_rows(t: &Translator, world: &World) -> Vec<Vec<Cell>> {
    let mut by_year: std::collections::BTreeMap<i32, HashMap<AwardKind, &brasfoot_model::mods::Award>> = Default::default();
    for a in &world.mods.awards.awards {
        if a.kind != AwardKind::TeamOfTheYear {
            by_year.entry(a.year).or_default().insert(a.kind, a);
        }
    }
    let cell = |a: Option<&&brasfoot_model::mods::Award>| match a {
        Some(a) => {
            let text = t.text("awards-ceremony-winner-cell", Some(&args([("name", a.name.clone()), ("club", a.club.clone().unwrap_or_else(|| "-".to_owned()))])));
            Cell::flagged(text, a.country)
        }
        None => Cell::from("-"),
    };
    by_year
        .iter()
        .rev()
        .map(|(year, x)| {
            let mut row = vec![Cell::from(year)];
            for kind in [AwardKind::BallonDor, AwardKind::GoldenShoe, AwardKind::GoldenGlove, AwardKind::GoldenBoy, AwardKind::CoachOfTheYear] {
                row.push(cell(x.get(&kind)));
            }
            row
        })
        .collect()
}

/// A player of the team of the year on the pitch (`Escalado`).
struct Picked {
    slot: TeamSlot,
    name: String,
    club: String,
    country: i32,
}

/// `selecao(Temporada)`: the season shown (partial while it runs) and the past ones recorded.
fn team_tab(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, state: &mut AwardsState) {
    let season = &state.season;
    let mut years: Vec<i32> = history::of_kind(&world.mods.awards, AwardKind::TeamOfTheYear).iter().map(|a| a.year).filter(|&y| y != season.year).collect();
    years.sort_by(|a, b| b.cmp(a));
    years.dedup();
    let label = |year: Option<i32>| match year {
        None if season.current => t.text("awards-ceremony-team-partial", Some(&args([("year", season.year.to_string())]))),
        None => season.year.to_string(),
        Some(y) => y.to_string(),
    };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t.text("awards-ceremony-team-season-label", None)).font(theme::bold(12.0)).color(GOLD));
        egui::ComboBox::from_id_salt("awards-team-year").width(140.0).selected_text(label(state.team_year)).show_ui(ui, |ui| {
            ui.selectable_value(&mut state.team_year, None, label(None));
            for &y in &years {
                ui.selectable_value(&mut state.team_year, Some(y), label(Some(y)));
            }
        });
    });
    ui.add_space(6.0);
    let team: Vec<Picked> = match state.team_year {
        None => season
            .team_of_the_year(world)
            .into_iter()
            .map(|(slot, c)| {
                let p = world.player(c.player);
                Picked { slot, name: p.name.clone(), club: club_name(t, world, c), country: p.country }
            })
            .collect(),
        Some(year) => history::of_kind(&world.mods.awards, AwardKind::TeamOfTheYear)
            .into_iter()
            .filter(|a| a.year == year)
            .filter_map(|a| match a.extra {
                AwardExtra::Slot(slot) => Some(Picked { slot, name: a.name.clone(), club: a.club.clone().unwrap_or_else(|| "-".into()), country: a.country }),
                _ => None,
            })
            .collect(),
    };
    let height = ui.available_height().max(200.0);
    let size = vec2((height * 1.45).min(ui.available_width()), height);
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        paint_team(ui, files, rect, &team);
    });
}

/// `Campo`: the 4-3-3 attacking upwards, the places as fractions of the field.
fn slot_places(slot: TeamSlot) -> &'static [(f32, f32)] {
    match slot {
        TeamSlot::Goalkeeper => &[(0.5, 0.86)],
        TeamSlot::RightBack => &[(0.85, 0.64)],
        TeamSlot::CentreBack => &[(0.62, 0.70), (0.38, 0.70)],
        TeamSlot::LeftBack => &[(0.15, 0.64)],
        TeamSlot::Midfielder => &[(0.25, 0.43), (0.5, 0.47), (0.75, 0.43)],
        TeamSlot::Forward => &[(0.18, 0.18), (0.5, 0.13), (0.82, 0.18)],
    }
}

fn paint_team(ui: &mut egui::Ui, files: &GameFiles, rect: egui::Rect, team: &[Picked]) {
    crate::screens::pitch::paint_pitch(ui.painter(), rect);
    let field = rect.shrink2(vec2(40.0, 16.0));
    let mut used: HashMap<u8, usize> = HashMap::new();
    for e in team {
        let places = slot_places(e.slot);
        let n = used.entry(e.slot as u8).or_insert(0);
        let Some(&(fx, fy)) = places.get(*n) else { continue };
        *n += 1;
        let center = pos2(field.left() + fx * field.width(), field.top() + fy * field.height());
        let painter = ui.painter();
        // a gold disc with a shadow and the country's flag
        painter.circle_filled(center + vec2(2.0, 3.0), 17.0, Color32::from_black_alpha(90));
        painter.circle_filled(center, 17.0, GOLD_DARK);
        painter.circle_filled(center - vec2(0.0, 3.0), 13.0, Color32::from_rgb(250, 214, 110));
        painter.circle_stroke(center, 17.0, Stroke::new(1.5, GOLD));
        widgets::paint_jar_image(ui, files, &format!("aflags/{}.png", e.country), egui::Rect::from_center_size(center, vec2(18.0, 18.0)));
        let painter = ui.painter();
        for (text, font, color, dy) in [(&e.name, theme::bold(13.0), Color32::WHITE, 32.0), (&e.club, theme::regular(11.0), Color32::from_rgb(225, 235, 225), 47.0)] {
            painter.text(center + vec2(1.0, dy + 1.0), egui::Align2::CENTER_CENTER, text, font.clone(), Color32::from_black_alpha(160));
            painter.text(center + vec2(0.0, dy), egui::Align2::CENTER_CENTER, text, font, color);
        }
    }
}

/// `clubes()`: the human coach's career, or a league club's seasons and idols.
fn clubs_tab(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, state: &mut AwardsState) {
    let mut list = clubs::league_clubs(world);
    list.sort_by_key(|&c| world.club(c).name.to_lowercase());
    let me = world.game.human_coaches.first().map(|&c| world.coach(c));
    let choice = *state.clubs.get_or_insert_with(|| match me.and_then(|c| c.club_id) {
        Some(club) if list.contains(&club) => ClubsChoice::Club(club),
        _ if me.is_some() => ClubsChoice::Career,
        _ => list.first().map(|&c| ClubsChoice::Club(c)).unwrap_or(ClubsChoice::Career),
    });
    let career_label = me.map(|c| t.text("awards-ceremony-clubs-my-career", Some(&args([("coach", c.name.clone())])))).unwrap_or_default();
    let shown = match choice {
        ClubsChoice::Career => career_label.clone(),
        ClubsChoice::Club(c) => world.club(c).name.clone(),
    };
    let mut picked = choice;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t.text("awards-ceremony-clubs-label", None)).font(theme::bold(12.0)).color(GOLD));
        egui::ComboBox::from_id_salt("awards-clubs").width(320.0).height(480.0).selected_text(shown).show_ui(ui, |ui| {
            if me.is_some() {
                ui.selectable_value(&mut picked, ClubsChoice::Career, &career_label);
            }
            for &c in &list {
                ui.selectable_value(&mut picked, ClubsChoice::Club(c), &world.club(c).name);
            }
        });
    });
    state.clubs = Some(picked);
    ui.add_space(6.0);
    let place = |l: &ClubSeason| if l.position > 0 { ordinal(t, l.position) } else { "-".to_owned() };
    let league = |l: &ClubSeason| l.league.as_ref().map(|k| labels::league(t, k)).unwrap_or_else(|| "-".to_owned());
    let scorer = |l: &ClubSeason| l.top_scorer.as_ref().map(|(n, g)| labels::club_top_scorer(t, n, *g)).unwrap_or_default();
    match picked {
        ClubsChoice::Club(club) => {
            let id = world.club(club).id;
            let seasons = clubs::history(world, id);
            let idols: Vec<&Idol> = world.mods.awards.idols.iter().filter(|i| i.club == id).collect();
            let idols_height = 210.0;
            ui.allocate_ui(vec2(ui.available_width(), ui.available_height() - idols_height - 8.0), |ui| {
                if seasons.is_empty() {
                    empty(ui, &t.text("awards-ceremony-clubs-empty-history", None));
                } else {
                    let rows: Vec<Vec<Cell>> = seasons
                        .iter()
                        .map(|l| vec![l.year.into(), league(l).into(), place(l).into(), labels::season_titles(t, &l.titles).into(), scorer(l).into(), l.coach.clone().unwrap_or_default().into()])
                        .collect();
                    table(ui, files, "awards-club-history", &columns(t, CLUB_HISTORY_COLUMNS), &rows, false);
                }
            });
            ui.add_space(8.0);
            ui.columns(2, |cols| {
                idols_table(&mut cols[0], t, files, &idols, true);
                idols_table(&mut cols[1], t, files, &idols, false);
            });
        }
        ClubsChoice::Career => {
            let seasons = me.map(|c| clubs::coach_career(world, &c.name)).unwrap_or_default();
            if seasons.is_empty() {
                empty(ui, &t.text("awards-ceremony-clubs-empty-career", None));
            } else {
                let rows: Vec<Vec<Cell>> = seasons
                    .iter()
                    .map(|l| vec![l.year.into(), Cell::flagged(&l.name, l.country), league(l).into(), place(l).into(), labels::season_titles(t, &l.titles).into(), scorer(l).into()])
                    .collect();
                table(ui, files, "awards-career", &columns(t, CAREER_COLUMNS), &rows, false);
            }
        }
    }
}

/// `painelIdolos`: the club's top ten by goals or by games.
fn idols_table(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, idols: &[&Idol], goals: bool) {
    let mut list: Vec<&Idol> = idols.to_vec();
    if goals {
        list.sort_by(|a, b| b.goals.cmp(&a.goals));
    } else {
        list.sort_by(|a, b| b.games.cmp(&a.games));
    }
    let title = if goals { "awards-ceremony-idols-scorers" } else { "awards-ceremony-idols-games" };
    ui.label(egui::RichText::new(t.text(title, None)).font(theme::bold(12.0)).color(GOLD));
    let (first, second) = if goals { ("awards-ceremony-column-goals", "awards-ceremony-column-games") } else { ("awards-ceremony-column-games", "awards-ceremony-column-goals") };
    let specs = [("awards-ceremony-column-rank", 30.0), ("awards-ceremony-column-player", 220.0), (first, 55.0), (second, 55.0)];
    let rows: Vec<Vec<Cell>> = list
        .iter()
        .take(clubs::TOP_IDOLS)
        .enumerate()
        .map(|(i, x)| {
            let (a, b) = if goals { (x.goals, x.games) } else { (x.games, x.goals) };
            vec![Cell::from(i + 1), Cell::flagged(&x.name, x.country), a.into(), b.into()]
        })
        .collect();
    ui.allocate_ui(vec2(ui.available_width(), 186.0), |ui| table(ui, files, title, &columns(t, &specs), &rows, false));
}

fn empty(ui: &mut egui::Ui, text: &str) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| ui.label(egui::RichText::new(text).font(theme::regular(13.0)).color(theme::MUTED)));
}

/// `hallDaFama(Temporada)`: its own tabs.
fn hall_tab(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, state: &mut AwardsState) {
    let not_ported = t.text("toolbar-not-ported", None);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, key) in HALL_TABS.iter().enumerate() {
            if tab_button(ui, &t.text(key, None), state.hall_tab == i, i < HALL_TABS_PORTED, &not_ported).clicked() {
                state.hall_tab = i;
            }
        }
    });
    ui.add_space(6.0);
    match state.hall_tab {
        0 => {
            // tabelaGaleria: each player's name and country from his last award
            let situations = hall_of_fame::situations(world);
            let mut last: HashMap<&str, &brasfoot_model::mods::Award> = HashMap::new();
            for a in &world.mods.awards.awards {
                last.insert(a.key.as_str(), a);
            }
            let count = |m: &history::AwardYears, kind| m.get(&kind).map(|y| y.len().to_string()).unwrap_or_default();
            let rows: Vec<Vec<Cell>> = hall_of_fame::gallery(world)
                .iter()
                .filter_map(|(key, m)| {
                    let a = last.get(key.as_str())?;
                    Some(vec![
                        Cell::flagged(&a.name, a.country),
                        count(m, AwardKind::BallonDor).into(),
                        count(m, AwardKind::GoldenShoe).into(),
                        count(m, AwardKind::GoldenGlove).into(),
                        count(m, AwardKind::GoldenBoy).into(),
                        count(m, AwardKind::TeamOfTheYear).into(),
                        situation(t, situations.get(key)).into(),
                    ])
                })
                .collect();
            table(ui, files, "awards-gallery", &columns(t, GALLERY_COLUMNS), &rows, false);
        }
        1 => {
            let rows: Vec<Vec<Cell>> = hall_of_fame::top_scorers(world)
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let status = if c.retired > 0 {
                        t.text("hall-retired-in", Some(&args([("year", c.retired.to_string())])))
                    } else {
                        c.club.clone().unwrap_or_else(|| "-".to_owned())
                    };
                    let per_game = if c.games > 0 { format::rating_two(t, c.goals as f64 / c.games as f64) } else { "-".to_owned() };
                    vec![
                        Cell::from(i + 1),
                        Cell::flagged(&c.name, c.country),
                        widgets::position_name(t, c.position).into(),
                        c.games.into(),
                        c.goals.into(),
                        c.assists.into(),
                        per_game.into(),
                        status.into(),
                    ]
                })
                .collect();
            table(ui, files, "awards-scorers", &columns(t, SCORERS_COLUMNS), &rows, false);
        }
        2 => {
            let awards = history::by_player(&world.mods.awards);
            let mut rows: Vec<Vec<Cell>> = hall_of_fame::legends(world)
                .iter()
                .map(|c| {
                    vec![
                        Cell::flagged(&c.name, c.country),
                        widgets::position_name(t, c.position).into(),
                        c.retired.into(),
                        c.club.clone().unwrap_or_else(|| "-".to_owned()).into(),
                        c.games.into(),
                        c.goals.into(),
                        c.assists.into(),
                        format::rating_two(t, c.rating()).into(),
                        awards.get(&c.key).map(|m| labels::award_counts(t, m)).unwrap_or_default().into(),
                    ]
                })
                .collect();
            if rows.is_empty() {
                rows.push(vec![t.text("awards-ceremony-legends-empty", None).into()]);
            }
            table(ui, files, "awards-legends", &columns(t, LEGENDS_COLUMNS), &rows, false);
        }
        _ => {
            // tabelaRecordes: five per record, the record's name on its first row
            let records = hall_of_fame::records(world);
            let mut rows = Vec::new();
            for kind in RecordKind::ALL {
                for (i, m) in records.iter().filter(|m| m.kind == kind).take(5).enumerate() {
                    let mark = if kind == RecordKind::Rating { format::rating_two(t, m.value) } else { format!("{}", m.value.round() as i64) };
                    rows.push(vec![
                        if i == 0 { labels::record_kind(t, kind) } else { String::new() }.into(),
                        Cell::from(i + 1),
                        Cell::flagged(&m.name, m.country),
                        m.club.clone().unwrap_or_else(|| "-".to_owned()).into(),
                        m.year.into(),
                        mark.into(),
                    ]);
                }
            }
            table(ui, files, "awards-records", &columns(t, RECORDS_COLUMNS), &rows, false);
        }
    }
}

/// `HallDaFama.situacoes()` as text; a key not found is "Aposentado".
fn situation(t: &Translator, s: Option<&hall_of_fame::Situation>) -> String {
    match s {
        Some(hall_of_fame::Situation::RetiredIn(year)) => t.text("hall-retired-in", Some(&args([("year", year.to_string())]))),
        Some(hall_of_fame::Situation::Club(name)) => name.clone(),
        Some(hall_of_fame::Situation::Free) => t.text("hall-free-agent", None),
        Some(hall_of_fame::Situation::Retired) | None => t.text("hall-retired", None),
    }
}
