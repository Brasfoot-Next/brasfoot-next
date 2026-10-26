//! The round screen (`a.dV`): the day's matches minute by minute. On top the match picked (the
//! human's first, `dV.Hr`), under it every match of the group (`dV.Hs`). A click picks a match
//! for the top; a double click, the button, Space or Enter opens the line-up and substitutions
//! window (`a.cD`), as the game does at half time (its option) and after a red card or an
//! injury of the human's club. The round can't be left before it ends.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

use brasfoot_engine::simulate::event_kind;
use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, MatchId};
use brasfoot_model::saved::MatchEvent;
use brasfoot_model::world::World;
use brasfoot_simulation::human::round::LiveRound;
use brasfoot_simulation::human::substitutions::{FIRST_MATCH_BENCH, MATCH_PLACES, MatchGrid, formation, match_grid};
use eframe::egui::{self, Color32, Rect, Stroke, pos2, vec2};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::menu::{self, Glyph};
use crate::screens::pitch::{paint_pitch, slot_center, token};
use crate::screens::widgets::{self, crest};
use crate::{motion, theme};
use crate::screens::controls;

/// `aq.rz`: milliseconds between ticks with a human club playing, by speed (`getVelocidade()`).
const HUMAN_SPEEDS: [u64; 6] = [2000, 1200, 700, 450, 220, 100];
/// `aq.rA`: the same without one (`getVelocidadeNH()`).
const OTHER_SPEEDS: [u64; 6] = [1200, 700, 300, 100, 70, 30];


pub struct RoundState {
    pub round: LiveRound,
    pub paused: bool,
    pub last_tick: Instant,
    /// The match on top: the row's index.
    featured: Option<usize>,
    /// The window `a.cD` when open.
    substitutions: Option<SubstitutionsWindow>,
    /// `dV.GV`: the window of the human's match is asked for.
    pub window_asked: bool,
}

/// `a.cD`: the two sides of a match, the one shown and the human's moves.
struct SubstitutionsWindow {
    game_match: MatchId,
    grids: [MatchGrid; 2],
    /// `cD.Di` - 1
    tab: usize,
    /// The side the human may change, if one is his.
    editable: Option<usize>,
    /// `cD.Dg`
    half_time: bool,
    dragging: Option<usize>,
    selected: Option<usize>,
    /// Opened this frame: the key that opened it does not close it.
    fresh: bool,
}

impl RoundState {
    pub fn new(round: LiveRound) -> Self {
        let featured = round.rows().iter().position(|r| r.human);
        RoundState { round, paused: false, last_tick: Instant::now(), featured, substitutions: None, window_asked: false }
    }
}

pub fn window(s: &mut CareerScreen, opt_state: &mut Option<RoundState>, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let Some(state) = opt_state.as_mut() else {
        *open = false;
        return;
    };
    // the round can't be left halfway: Esc and the backdrop do nothing until it ends
    let mut still_open = true;
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "live-round", &mut still_open, vec2(1220.0, 700.0), frame, |ui, _| {
        body(s, state, ui, t, files, open);
    });
    if let Some(mut window) = state.substitutions.take() {
        let mut keep = true;
        substitutions_window(s, state, &mut window, ctx, t, &mut keep);
        if keep {
            state.substitutions = Some(window);
        } else {
            state.last_tick = Instant::now();
        }
    }
}

fn open_substitutions(s: &CareerScreen, state: &mut RoundState, m: MatchId, half_time: bool) {
    let world = &s.career.world;
    let g = world.game_match(m);
    let editable = [g.home, g.away].iter().position(|c| c.is_some_and(|c| world.club(c).human));
    state.substitutions = Some(SubstitutionsWindow {
        game_match: m,
        grids: [match_grid(world, m, 0), match_grid(world, m, 1)],
        tab: editable.unwrap_or(0),
        editable,
        half_time,
        dragging: None,
        selected: None,
        fresh: true,
    });
}

fn speed_index(world: &World, state: &RoundState) -> usize {
    let human = state.round.rows().iter().any(|r| r.human);
    let speed = if human { world.game.speed } else { world.game.speed_non_human };
    speed.clamp(0, 5) as usize
}

fn body(s: &mut CareerScreen, state: &mut RoundState, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, open: &mut bool) {
    // the timer (`dV.GO`): one tick per interval while nothing else is on screen
    let human = state.round.rows().iter().any(|r| r.human);
    let interval = Duration::from_millis(if human { HUMAN_SPEEDS } else { OTHER_SPEEDS }[speed_index(&s.career.world, state)]);
    if !state.paused && state.round.on && state.substitutions.is_none() {
        if state.last_tick.elapsed() >= interval {
            let before = state.round.rows().len();
            let asked = s.career.step_live_round(&mut state.round);
            state.last_tick = Instant::now();
            if state.round.rows().len() != before || state.featured.is_none_or(|f| f >= state.round.rows().len()) {
                state.featured = state.round.rows().iter().position(|r| r.human).or_else(|| state.round.rows().iter().position(|r| r.game_match.is_some()));
            }
            if let Some((m, half_time)) = asked {
                open_substitutions(s, state, m, half_time);
            }
        }
        ui.ctx().request_repaint_after(interval.saturating_sub(state.last_tick.elapsed()).max(Duration::from_millis(16)));
    }

    header(state, ui, t, files, &s.career.world);
    ui.add_space(12.0);
    let mut open_window = None;
    // the two panels take the height the controls leave
    let panel_height = (ui.available_height() - 72.0).max(240.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() * 0.56);
            if let Some(m) = state.featured.and_then(|i| state.round.rows().get(i)).and_then(|r| r.game_match) {
                let goals = state.featured.and_then(|i| state.round.rows().get(i)).map(|r| r.goals).unwrap_or_default();
                if featured_match(ui, t, files, &s.career.world, state, m, goals, panel_height) {
                    open_window = Some(m);
                }
            }
        });
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            if let Some(m) = match_list(state, ui, t, files, &s.career.world, panel_height) {
                open_window = Some(m);
            }
        });
    });
    ui.add_space(12.0);
    controls(s, state, ui, t, open);
    // dW: Space and Enter ask for the window of the human's match
    if state.substitutions.is_none() && ui.input(|i| i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter)) && state.round.on {
        state.window_asked = true;
    }
    if std::mem::take(&mut state.window_asked) {
        open_window = state.round.human_row().and_then(|r| r.game_match);
    }
    if let Some(m) = open_window
        && state.substitutions.is_none()
    {
        open_substitutions(s, state, m, false);
    }
}

/// `dV.uh` and `dV.Hf`: the time and its bar.
fn time_text(t: &Translator, state: &RoundState) -> String {
    let (minute, half) = (state.round.minute(), state.round.half());
    if !state.round.on {
        t.text("live-round-finished", None)
    } else if state.round.is_over() {
        t.text("live-round-full-time", None)
    } else if half == 2 && minute == 0 {
        t.text("live-round-half-time", None)
    } else if minute >= 45 {
        t.text("live-round-added-time", Some(&args([("half", half)])))
    } else {
        t.text("live-round-clock", Some(&args([("minute", minute), ("half", half)])))
    }
}

fn header(state: &RoundState, ui: &mut egui::Ui, t: &Translator, _files: &GameFiles, world: &World) {
    ui.horizontal(|ui| {
        widgets::icon_glyph(ui, glyphs::SOCCER_BALL, 24.0, theme::ACCENT);
        ui.add_space(6.0);
        ui.label(egui::RichText::new(t.text("live-round-title", None)).font(theme::bold(18.0)));
        // dV.Hg: the competition of the group shown
        if let Some(c) = state.round.rows().iter().find_map(|r| r.competition) {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(widgets::competition_name(t, world, c)).font(theme::regular(14.0)).color(theme::MUTED));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let progress = if state.round.on { (state.round.minute().clamp(0, 45) as f32 / 45.0).clamp(0.0, 1.0) } else { 1.0 };
            let (bar, _) = ui.allocate_exact_size(vec2(160.0, 8.0), egui::Sense::hover());
            ui.painter().rect_filled(bar, 4.0, theme::HEADER);
            ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * progress, bar.height())), 4.0, theme::GOOD);
            ui.add_space(10.0);
            egui::Frame::new().fill(theme::ACCENT.gamma_multiply(0.2)).corner_radius(12).inner_margin(egui::Margin::symmetric(12, 4)).show(ui, |ui| {
                ui.label(egui::RichText::new(time_text(t, state)).font(theme::bold(14.0)).color(theme::TEXT));
            });
        });
    });
}

/// The match on top (`dV.Hr`): the clubs, the score, the events of each side and the numbers.
/// True when its button asks for the window.
fn featured_match(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, state: &RoundState, m: MatchId, goals: [i32; 2], height: f32) -> bool {
    let g = world.game_match(m);
    let (Some(h), Some(a)) = (g.home, g.away) else { return false };
    let human = world.club(h).human || world.club(a).human;
    let mut clicked = false;
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(height - 32.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(widgets::match_round_text(t, world, m)).font(theme::regular(12.0)).color(theme::MUTED));
        });
        ui.add_space(10.0);
        // the clubs and the score
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, 96.0), egui::Sense::hover());
        let score = Rect::from_center_size(rect.center(), vec2(140.0, 64.0));
        ui.painter().rect_filled(score, 12.0, theme::HEADER);
        ui.painter().text(score.center(), egui::Align2::CENTER_CENTER, t.text("live-round-score", Some(&args([("home", goals[0]), ("away", goals[1])]))), theme::bold(34.0), Color32::WHITE);
        for (side, club) in [(0, h), (1, a)] {
            let column = if side == 0 { Rect::from_x_y_ranges(rect.left()..=score.left() - 12.0, rect.y_range()) } else { Rect::from_x_y_ranges(score.right() + 12.0..=rect.right(), rect.y_range()) };
            let crest_rect = Rect::from_center_size(pos2(column.center().x, rect.top() + 28.0), vec2(52.0, 52.0));
            paint_crest(ui, files, world, club, crest_rect);
            let mut job = egui::text::LayoutJob::simple_singleline(world.club(club).name.clone(), theme::bold(15.0), theme::TEXT);
            job.wrap = egui::text::TextWrapping::truncate_at_width(column.width());
            let galley = ui.painter().layout_job(job);
            ui.painter().galley(pos2(column.center().x - galley.size().x / 2.0, rect.top() + 62.0), galley, theme::TEXT);
        }
        // the shoot-out, once it is played (`gi` is {-1, -1} before)
        if let [p0, p1] = g.penalty_shootout[..]
            && p0 >= 0
            && p1 >= 0
        {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(t.text("live-round-penalties", Some(&args([("home", p0), ("away", p1)])))).font(theme::bold(12.0)).color(theme::ACCENT));
            });
        }
        ui.add_space(12.0);
        // the events of each side, as far as the screen has shown them
        ui.columns(2, |cols| {
            for (side, club) in [(0, h), (1, a)] {
                let ui = &mut cols[side];
                let align = if side == 0 { egui::Align::Max } else { egui::Align::Min };
                ui.with_layout(egui::Layout::top_down(align), |ui| {
                    for e in g.events.iter().filter(|e| e.done && e.club == Some(club) && e.kind != event_kind::ASSIST) {
                        event_line(ui, t, world, e, side == 0);
                    }
                });
            }
        });
        ui.add_space(12.0);
        // the numbers
        let stats = [
            ("live-round-stat-possession", g.possession.first().copied().unwrap_or(50), g.possession.get(1).copied().unwrap_or(50), true),
            ("live-round-stat-shots", g.shots.first().copied().unwrap_or(0), g.shots.get(1).copied().unwrap_or(0), false),
            ("live-round-stat-on-target", g.shots_on_target.first().copied().unwrap_or(0), g.shots_on_target.get(1).copied().unwrap_or(0), false),
        ];
        for (key, home, away, percent) in stats {
            stat_bar(ui, &t.text(key, None), home, away, percent);
            ui.add_space(6.0);
        }
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            let key = if human { "live-round-open-substitutions" } else { "live-round-open-lineups" };
            clicked = controls::button(ui, &t.text(key, None), controls::Kind::Secondary, state.round.on).clicked();
        });
    });
    clicked
}

fn paint_crest(ui: &mut egui::Ui, files: &GameFiles, world: &World, club: ClubId, rect: Rect) {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    crest(&mut child, files, world, club, rect.width());
}

/// A comparison bar: the home share from the left in the accent, the away share in grey.
fn stat_bar(ui: &mut egui::Ui, label: &str, home: i32, away: i32, percent: bool) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 34.0), egui::Sense::hover());
    let p = ui.painter();
    let text = |v: i32| if percent { format!("{v}%") } else { v.to_string() };
    p.text(pos2(rect.left(), rect.top() + 8.0), egui::Align2::LEFT_CENTER, text(home), theme::bold(13.0), theme::TEXT);
    p.text(pos2(rect.right(), rect.top() + 8.0), egui::Align2::RIGHT_CENTER, text(away), theme::bold(13.0), theme::TEXT);
    p.text(pos2(rect.center().x, rect.top() + 8.0), egui::Align2::CENTER_CENTER, label, theme::regular(12.0), theme::MUTED);
    let bar = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 8.0), vec2(width, 6.0));
    p.rect_filled(bar, 3.0, theme::GRID);
    let total = (home + away).max(1) as f32;
    let share = if home + away == 0 { 0.5 } else { home as f32 / total };
    p.rect_filled(Rect::from_min_size(bar.min, vec2(width * share, 6.0)), 3.0, theme::ACCENT);
}

/// `A.er()`: an event's line, with its icon (`A.es()`) drawn by the app.
fn event_text(t: &Translator, world: &World, e: &MatchEvent) -> String {
    let name = |p: Option<brasfoot_model::ids::PlayerId>| p.map(|p| world.player(p).name.clone()).unwrap_or_default();
    match e.kind {
        event_kind::GOAL => {
            let key = match e.detail {
                2 => "event-goal-own",
                3 => "event-goal-penalty",
                4 => "event-goal-free-kick",
                5 => "event-goal-olympic",
                _ => "event-goal",
            };
            t.text(key, Some(&args([("player", name(e.player)), ("minute", e.minute.to_string())])))
        }
        event_kind::SUBSTITUTION => {
            if e.minute == 0 && e.half == 2 {
                t.text("event-substitution-half-time", Some(&args([("out", name(e.player)), ("in", name(e.other_player))])))
            } else {
                t.text("event-substitution", Some(&args([("out", name(e.player)), ("in", name(e.other_player)), ("minute", e.minute.to_string())])))
            }
        }
        event_kind::MISSED_PENALTY => t.text("event-missed-penalty", Some(&args([("player", name(e.player)), ("minute", e.minute.to_string())]))),
        _ => t.text("event-plain", Some(&args([("player", name(e.player)), ("minute", e.minute.to_string())]))),
    }
}

/// The icon of an event, drawn: a ball for a goal, the cards, a cross for an injury, two
/// triangles for a substitution, a crossed ball for a missed penalty.
fn paint_event_icon(p: &egui::Painter, rect: Rect, e: &MatchEvent) {
    let c = rect.center();
    match e.kind {
        event_kind::GOAL => menu::glyph(p, rect.shrink(1.0), Glyph::Ball, Color32::WHITE),
        event_kind::YELLOW => {
            p.rect_filled(Rect::from_center_size(c, vec2(9.0, 13.0)), 2.0, Color32::from_rgb(0xf2, 0xc9, 0x1d));
        }
        event_kind::SECOND_YELLOW | event_kind::RED => {
            if e.kind == event_kind::SECOND_YELLOW {
                p.rect_filled(Rect::from_center_size(c + vec2(-3.0, -1.0), vec2(9.0, 13.0)), 2.0, Color32::from_rgb(0xf2, 0xc9, 0x1d));
            }
            p.rect_filled(Rect::from_center_size(c + vec2(2.0, 1.0), vec2(9.0, 13.0)), 2.0, Color32::from_rgb(0xe0, 0x3b, 0x3b));
        }
        event_kind::INJURY => {
            let red = Color32::from_rgb(0xe0, 0x3b, 0x3b);
            p.rect_filled(Rect::from_center_size(c, vec2(12.0, 4.0)), 1.0, red);
            p.rect_filled(Rect::from_center_size(c, vec2(4.0, 12.0)), 1.0, red);
        }
        event_kind::SUBSTITUTION => {
            let green = Color32::from_rgb(0x3f, 0xc0, 0x7a);
            let red = Color32::from_rgb(0xe0, 0x5b, 0x5b);
            p.add(egui::Shape::convex_polygon(vec![c + vec2(-6.0, 0.0), c + vec2(-2.0, -6.0), c + vec2(2.0, 0.0)], green, Stroke::NONE));
            p.add(egui::Shape::convex_polygon(vec![c + vec2(-2.0, 1.0), c + vec2(6.0, 1.0), c + vec2(2.0, 7.0)], red, Stroke::NONE));
        }
        event_kind::MISSED_PENALTY => {
            menu::glyph(p, rect.shrink(1.0), Glyph::Ball, Color32::from_white_alpha(140));
            let s = Stroke::new(2.0, Color32::from_rgb(0xe0, 0x3b, 0x3b));
            p.line_segment([rect.left_top() + vec2(2.0, 2.0), rect.right_bottom() - vec2(2.0, 2.0)], s);
        }
        _ => {}
    }
}

fn event_line(ui: &mut egui::Ui, t: &Translator, world: &World, e: &MatchEvent, icon_last: bool) {
    let text = event_text(t, world, e);
    let draw_icon = |ui: &mut egui::Ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
        paint_event_icon(ui.painter(), rect, e);
    };
    let layout = if icon_last { egui::Layout::right_to_left(egui::Align::Center) } else { egui::Layout::left_to_right(egui::Align::Center) };
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 20.0), layout, |ui| {
        draw_icon(ui);
        ui.add_space(4.0);
        ui.label(egui::RichText::new(text).font(theme::regular(12.0)));
    });
}

/// `dV.Hs`: the matches of the group by competition. A click puts a match on top, a double
/// click opens its window (`dV.ef`). Returns the match to open.
fn match_list(state: &mut RoundState, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, height: f32) -> Option<MatchId> {
    let mut open = None;
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(height - 32.0);
        egui::ScrollArea::vertical().id_salt("round-matches").auto_shrink([false, false]).max_height(height - 32.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for i in 0..state.round.rows().len() {
                let row = state.round.rows()[i].clone();
                let Some(m) = row.game_match else {
                    if let Some(c) = row.competition {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(widgets::competition_name(t, world, c)).font(theme::bold(12.0)).color(theme::MUTED));
                        ui.add_space(2.0);
                    }
                    continue;
                };
                let g = world.game_match(m);
                let (Some(h), Some(a)) = (g.home, g.away) else { continue };
                let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), egui::Sense::click());
                if response.clicked() {
                    state.featured = Some(i);
                }
                if response.double_clicked() && state.round.on {
                    open = Some(m);
                }
                let hover = motion::toward(ui.ctx(), response.id, response.hovered(), motion::HOVER);
                // a goal lights the row for a moment
                let flash = 1.0 - motion::since_change(ui.ctx(), response.id.with("goals"), (row.goals[0] * 100 + row.goals[1]) as u64, 1.2);
                let base = if state.featured == Some(i) { theme::SELECTION } else { motion::mix(theme::CARD, theme::HOVER, hover) };
                let fill = if row.goals != [0, 0] && flash > 0.0 { motion::mix(base, theme::GOOD.gamma_multiply(0.6), flash) } else { base };
                let p = ui.painter();
                p.rect_filled(rect, 8.0, fill);
                if row.human {
                    p.rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 8.0), vec2(3.0, rect.height() - 16.0)), 2.0, theme::ACCENT);
                }
                let score = Rect::from_center_size(pos2(rect.center().x, rect.top() + 16.0), vec2(56.0, 24.0));
                p.rect_filled(score, 6.0, theme::HEADER);
                p.text(score.center(), egui::Align2::CENTER_CENTER, t.text("live-round-score", Some(&args([("home", row.goals[0]), ("away", row.goals[1])]))), theme::bold(14.0), Color32::WHITE);
                for (side, club) in [(0, h), (1, a)] {
                    let crest_center = if side == 0 { pos2(score.left() - 16.0, score.center().y) } else { pos2(score.right() + 16.0, score.center().y) };
                    paint_crest(ui, files, world, club, Rect::from_center_size(crest_center, vec2(20.0, 20.0)));
                    let room = if side == 0 { Rect::from_x_y_ranges(rect.left() + 8.0..=crest_center.x - 16.0, score.y_range()) } else { Rect::from_x_y_ranges(crest_center.x + 16.0..=rect.right() - 8.0, score.y_range()) };
                    let (anchor, align) = if side == 0 { (room.right_center(), egui::Align2::RIGHT_CENTER) } else { (room.left_center(), egui::Align2::LEFT_CENTER) };
                    ui.painter().with_clip_rect(room).text(anchor, align, &world.club(club).name, theme::regular(13.0), theme::TEXT);
                }
                // `v.tV()`: the last event shown, under the score
                if let Some(e) = g.events.iter().rev().find(|e| e.done && e.kind != event_kind::ASSIST && (e.club == Some(h) || e.club == Some(a))) {
                    let text = event_text(t, world, e);
                    let galley = ui.painter().layout_no_wrap(text, theme::regular(11.0), theme::MUTED);
                    let width = galley.size().x + 20.0;
                    let x = rect.center().x - width / 2.0;
                    paint_event_icon(ui.painter(), Rect::from_min_size(pos2(x, rect.bottom() - 18.0), vec2(14.0, 14.0)), e);
                    ui.painter().galley(pos2(x + 20.0, rect.bottom() - 18.0), galley, theme::MUTED);
                }
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
        });
    });
    open
}

fn controls(s: &mut CareerScreen, state: &mut RoundState, ui: &mut egui::Ui, t: &Translator, open: &mut bool) {
    ui.horizontal(|ui| {
        // the speed is a preference (`a.fB`); F1 to F6 still change it, as in the original
        let human = state.round.rows().iter().any(|r| r.human);
        for (i, key) in [egui::Key::F1, egui::Key::F2, egui::Key::F3, egui::Key::F4, egui::Key::F5, egui::Key::F6].into_iter().enumerate() {
            if ui.input(|input| input.key_pressed(key)) {
                if human {
                    s.career.world.game.speed = i as i32;
                } else {
                    s.career.world.game.speed_non_human = i as i32;
                }
            }
        }
        if state.round.on {
            let key = if state.paused { "live-round-resume" } else { "live-round-pause" };
            if controls::button(ui, &t.text(key, None), controls::Kind::Secondary, true).clicked() {
                state.paused = !state.paused;
                state.last_tick = Instant::now();
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !state.round.on {
                if controls::button(ui, &t.text("live-round-continue", None), controls::Kind::Primary, true).clicked() {
                    *open = false;
                }
            } else if controls::button(ui, &t.text("live-round-speed-instant", None), controls::Kind::Secondary, true).clicked() {
                s.career.finish_live_round(&mut state.round);
            }
        });
    });
}

/// `a.cD`: the two sides on the pitch with their benches, the human's moves on his side, the
/// tactics, the events and "Voltar ao jogo".
fn substitutions_window(s: &mut CareerScreen, state: &RoundState, w: &mut SubstitutionsWindow, ctx: &egui::Context, t: &Translator, keep: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let mut open = true;
    motion::dialog(ctx, "match-substitutions", &mut open, vec2(1060.0, 700.0), frame, |ui, _| {
        let m = w.game_match;
        let (h, a) = {
            let g = s.career.world.game_match(m);
            (g.home, g.away)
        };
        // the sides as tabs (`cD.zO`, `zP`): name, goals
        ui.horizontal(|ui| {
            let goals = {
                let g = s.career.world.game_match(m);
                [g.home_goals, g.away_goals]
            };
            for (side, club) in [(0, h), (1, a)] {
                let Some(club) = club else { continue };
                let on = w.tab == side;
                let text = t.text("match-substitutions-tab", Some(&args([("club", s.career.world.club(club).name.clone()), ("goals", goals[side].to_string())])));
                if controls::tab(ui, &text, on, true, "").clicked() {
                    w.tab = side;
                    w.selected = None;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let minute = if w.half_time { t.text("match-substitutions-halftime", None) } else { t.text("live-round-clock", Some(&args([("minute", state.round.minute()), ("half", state.round.half())]))) };
                egui::Frame::new().fill(theme::ACCENT.gamma_multiply(0.2)).corner_radius(12).inner_margin(egui::Margin::symmetric(12, 4)).show(ui, |ui| {
                    ui.label(egui::RichText::new(minute).font(theme::bold(14.0)));
                });
            });
        });
        ui.add_space(12.0);
        let editable = w.editable == Some(w.tab);
        let mut targets: Vec<(usize, Rect)> = Vec::new();
        ui.horizontal_top(|ui| {
            // the pitch
            ui.vertical(|ui| {
                // the pitch takes the height there is, at the original's 2:3
                let height = (ui.available_height() - 30.0).clamp(300.0, 600.0);
                let size = vec2(height * 2.0 / 3.0, height);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                paint_pitch(ui.painter(), rect);
                let pointer = ui.input(|i| i.pointer.latest_pos());
                for slot in 1..=25 {
                    let center = slot_center(rect, slot);
                    let area = Rect::from_center_size(center, vec2(64.0, 52.0));
                    targets.push((slot, area));
                    let response = ui.interact(area, ui.make_persistent_id(("match-place", w.tab, slot)), if editable { egui::Sense::click_and_drag() } else { egui::Sense::hover() });
                    if editable {
                        handle(s, state, w, slot, &response);
                    }
                    let target = w.dragging.is_some() && pointer.is_some_and(|p| area.contains(p));
                    if let Some(p) = w.grids[w.tab].places[slot]
                        && w.dragging != Some(slot)
                    {
                        let x = s.career.world.player(p);
                        token(ui.painter(), center, x.position, x.strength, &x.name, w.selected == Some(slot) || target, false);
                    } else if w.dragging.is_some_and(|d| (2..=25).contains(&d)) {
                        ui.painter().circle_filled(center, if target { 9.0 } else { 4.0 }, Color32::from_white_alpha(if target { 200 } else { 70 }));
                    }
                }
                let lines = formation(&w.grids[w.tab]);
                ui.add_space(6.0);
                ui.label(egui::RichText::new(t.text("match-substitutions-formation", Some(&args([("d", lines[0]), ("m", lines[1]), ("a", lines[2])])))).font(theme::bold(13.0)).color(theme::MUTED));
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                // the bench
                menu::panel().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let left = s.career.world.game_match(m).substitutions_left.get(w.tab).copied().unwrap_or(0);
                    menu::panel_title(ui, &t.text("lineup-bench-title", None), |ui| {
                        ui.label(egui::RichText::new(t.text("match-substitutions-subs-left", Some(&args([("count", left)])))).font(theme::bold(12.0)).color(theme::STAR));
                    });
                    let pointer = ui.input(|i| i.pointer.latest_pos());
                    let width = (ui.available_width() - 8.0) / 2.0;
                    for row in 0..6 {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            for column in 0..2 {
                                let slot = FIRST_MATCH_BENCH + row * 2 + column;
                                if slot >= MATCH_PLACES {
                                    continue;
                                }
                                let (rect, response) = ui.allocate_exact_size(vec2(width, 30.0), if editable { egui::Sense::click_and_drag() } else { egui::Sense::hover() });
                                targets.push((slot, rect));
                                if editable {
                                    handle(s, state, w, slot, &response);
                                }
                                let target = w.dragging.is_some() && pointer.is_some_and(|p| rect.contains(p));
                                bench_chip(ui, t, &s.career.world, w, slot, rect, target);
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
                ui.add_space(10.0);
                // the tactics (`Do`, `Dp`, `Dn`), the human's only
                if editable && let Some(club) = if w.tab == 0 { h } else { a } {
                    menu::panel().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        widgets::tactics_combos(ui, t, "match-tactics", &mut s.career.world.club_mut(club).tactics);
                    });
                    ui.add_space(10.0);
                }
                // the events (`cD.DG`)
                menu::panel().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    menu::panel_title(ui, &t.text("match-substitutions-events", None), |_| {});
                    egui::ScrollArea::vertical().id_salt("match-events").max_height((ui.available_height() - 64.0).max(60.0)).auto_shrink([false, true]).show(ui, |ui| {
                        let world = &s.career.world;
                        for e in world.game_match(m).events.iter().filter(|e| e.done && e.kind != event_kind::ASSIST) {
                            event_line(ui, t, world, e, e.club == a);
                        }
                    });
                });
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if controls::button(ui, &t.text("match-substitutions-button-back", None), controls::Kind::Primary, true).clicked() {
                        *keep = false;
                    }
                });
            });
        });
        drop_in_match(s, state, w, ui, &targets);
    });
    // Esc, Space and Enter go back to the match (`pb()`)
    if !open || (!w.fresh && ctx.input(|i| i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter))) {
        *keep = false;
    }
    w.fresh = false;
}

/// A press, drag or click on a place of the human's side.
fn handle(s: &mut CareerScreen, state: &RoundState, w: &mut SubstitutionsWindow, slot: usize, response: &egui::Response) {
    let has_player = w.grids[w.tab].places[slot].is_some();
    if response.drag_started() && has_player {
        w.dragging = Some(slot);
        w.selected = None;
    } else if response.clicked() {
        match w.selected {
            Some(from) if from != slot => {
                move_player(s, state, w, from, slot);
                w.selected = None;
            }
            Some(_) => w.selected = None,
            None if has_player => w.selected = Some(slot),
            None => {}
        }
    }
}

fn move_player(s: &mut CareerScreen, state: &RoundState, w: &mut SubstitutionsWindow, from: usize, to: usize) {
    let tab = w.tab;
    s.career.move_in_match(&mut w.grids[tab], from, to, state.round.half(), state.round.minute(), w.half_time);
}

fn drop_in_match(s: &mut CareerScreen, state: &RoundState, w: &mut SubstitutionsWindow, ui: &egui::Ui, targets: &[(usize, Rect)]) {
    let Some(from) = w.dragging else { return };
    let (pointer, down) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
    if let Some(pos) = pointer
        && down
        && let Some(p) = w.grids[w.tab].places[from]
    {
        let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("match-drag")));
        let x = s.career.world.player(p);
        token(&painter, pos, x.position, x.strength, &x.name, true, false);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if !down {
        if let Some(pos) = pointer
            && let Some(&(to, _)) = targets.iter().find(|(_, r)| r.contains(pos))
            && to != from
        {
            move_player(s, state, w, from, to);
        }
        w.dragging = None;
    }
}

fn bench_chip(ui: &egui::Ui, t: &Translator, world: &World, w: &SubstitutionsWindow, slot: usize, rect: Rect, target: bool) {
    let painter = ui.painter_at(rect);
    match w.grids[w.tab].places[slot] {
        Some(p) if w.dragging != Some(slot) => {
            let selected = w.selected == Some(slot) || target;
            painter.rect_filled(rect, 6.0, if selected { theme::SELECTION } else { theme::CARD });
            let x = world.player(p);
            widgets::paint_position_badge(&painter, t, pos2(rect.left() + 16.0, rect.center().y), x.position);
            painter.text(pos2(rect.right() - 10.0, rect.center().y), egui::Align2::RIGHT_CENTER, x.strength.to_string(), theme::bold(12.0), theme::TEXT);
            let room = Rect::from_x_y_ranges(rect.left() + 32.0..=rect.right() - 36.0, rect.y_range());
            ui.painter().with_clip_rect(room).text(pos2(room.left(), rect.center().y), egui::Align2::LEFT_CENTER, &x.name, theme::regular(12.0), theme::TEXT);
        }
        _ => {
            painter.rect_stroke(rect.shrink(0.5), 6.0, Stroke::new(1.0, if target { theme::ACCENT } else { theme::BORDER }), egui::StrokeKind::Inside);
        }
    }
}
