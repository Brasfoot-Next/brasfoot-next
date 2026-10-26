//! The line-up screen (`a.bz`, "Escalação"): the pitch with its 25 places, the bench (26 to 36)
//! and the players left out (37 on). A player is dragged from one place to another (`bz.H`,
//! rules in `LineupScreen::move_player`) or clicked and then clicked on the place it goes to.
//! The right button on the pitch sets the captain and the set-piece takers (`bz.AW`). The pitch
//! is drawn by the app in the theme instead of `aicons/campo.png`.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, MatchId};
use brasfoot_model::world::World;
use brasfoot_simulation::human::lineup::{FIRST_BENCH_PLACE, FIRST_SPARE_PLACE, LAST_BENCH_PLACE, LineupScreen};
use eframe::egui::{self, Color32, Rect, Stroke, pos2, vec2};

use crate::career::CareerError;
use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::menu;
use crate::screens::pitch::{TOKEN, paint_pitch, slot_center, token};
use crate::screens::widgets::{self, crest};
use crate::{motion, theme, view};

/// `aq.rB`: the automatic selection combo.
const FORMATION_KEYS: [&str; 14] = [
    "lineup-formation-select",
    "lineup-formation-reset",
    "lineup-formation-541",
    "lineup-formation-532",
    "lineup-formation-451",
    "lineup-formation-442",
    "lineup-formation-442-def",
    "lineup-formation-442-att",
    "lineup-formation-433",
    "lineup-formation-433-def",
    "lineup-formation-352",
    "lineup-formation-343",
    "lineup-formation-4231",
    "lineup-formation-4231-wings",
];
/// `aq.pb`: the pitch's condition (`I.dX()`).
const CONDITION_KEYS: [&str; 4] = ["lineup-pitch-excellent", "lineup-pitch-very-good", "lineup-pitch-bad", "lineup-pitch-poor"];


pub struct LineupState {
    pub lineup: LineupScreen,
    /// A place clicked, waiting for the place it goes to.
    pub selected_slot: Option<usize>,
    /// The index of the formation combo; 0 is "Selecione opção".
    pub selected_formation: usize,
    pub error: Option<String>,
    /// The place a player is being dragged from (`bz.AP`).
    dragging: Option<usize>,
}

impl LineupState {
    pub fn new(world: &mut World, club: ClubId, next_match: MatchId) -> Self {
        let lineup = LineupScreen::open(world, club, next_match);
        LineupState { lineup, selected_slot: None, selected_formation: 0, error: None, dragging: None }
    }

    /// A press, a drag or a click on a place.
    fn handle(&mut self, slot: usize, response: &egui::Response) {
        let has_player = self.lineup.grid.get(slot).is_some_and(|p| p.player.is_some());
        if response.drag_started() && has_player {
            self.dragging = Some(slot);
            self.selected_slot = None;
        } else if response.clicked() {
            match self.selected_slot {
                Some(from) if from != slot => {
                    self.lineup.move_player(from, slot);
                    self.selected_slot = None;
                }
                Some(_) => self.selected_slot = None,
                None if has_player => self.selected_slot = Some(slot),
                None => {}
            }
        }
    }
}

/// The places drawn this frame and where, for the drop.
type Targets = Vec<(usize, Rect)>;

pub fn window(s: &mut CareerScreen, state: &mut Option<LineupState>, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "lineup", open, vec2(1180.0, 700.0), frame, |ui, open| {
        body(s, state, ui, t, files, open);
    });
}

fn body(s: &mut CareerScreen, opt_state: &mut Option<LineupState>, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, open: &mut bool) {
    let Some(human_club) = s.career.human_club() else {
        *open = false;
        return;
    };
    let Some(next_match) = view::next_match(&s.career.world, human_club) else {
        *open = false;
        return;
    };
    if opt_state.as_ref().is_none_or(|st| st.lineup.club != human_club) {
        *opt_state = Some(LineupState::new(&mut s.career.world, human_club, next_match));
    }
    let Some(state) = opt_state.as_mut() else { return };

    let starters = state.lineup.starters();
    let close = widgets::dialog_title_with(ui, files, Some(glyphs::LINEUP), &t.text("lineup-title", None), |ui| {
        ui.add_space(8.0);
        let color = if starters == 11 { theme::GOOD } else { theme::WARN };
        egui::Frame::new().fill(color.gamma_multiply(0.18)).corner_radius(10).inner_margin(egui::Margin::symmetric(8, 2)).show(ui, |ui| {
            ui.label(egui::RichText::new(t.text("lineup-starters-count", Some(&args([("count", starters)])))).font(theme::bold(12.0)).color(color));
        });
    });
    if close {
        *open = false;
    }

    let mut targets: Targets = Vec::new();
    let mut play = false;
    // the pitch takes the height there is, under the opponent's card, at the original's 2:3
    let pitch_height = (ui.available_height() - 96.0).clamp(300.0, 600.0);
    let pitch_size = vec2(pitch_height * 2.0 / 3.0, pitch_height);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(pitch_size.x);
            // bz.u, x, v: ah.k(), ah.j(), ah.n() at once, as the original
            if let Some((which, p)) = pitch(state, ui, t, &s.career.world, &mut targets, pitch_size) {
                let club = s.career.world.club_mut(human_club);
                match which {
                    0 => club.free_kick_taker = Some(p),
                    1 => club.captain = Some(p),
                    _ => club.corner_taker = Some(p),
                }
            }
            ui.add_space(10.0);
            match_card(ui, t, files, &s.career.world, human_club, next_match);
        });
        ui.add_space(16.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            bench(state, ui, t, &s.career.world, &mut targets);
            ui.add_space(10.0);
            // the list takes what the tactics and the buttons leave
            let list_height = (ui.available_height() - 230.0).max(64.0);
            spares(state, ui, t, &s.career.world, &mut targets, list_height);
            ui.add_space(10.0);
            tactics(state, ui, t, &mut s.career.world);
            ui.add_space(12.0);
            if let Some(err) = &state.error {
                menu::error(ui, err);
            }
            ui.horizontal(|ui| {
                if menu::secondary_button(ui, &t.text("lineup-button-back", None), true).clicked() {
                    *open = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    play = menu::primary_button(ui, &t.text("lineup-button-play", None), true).clicked();
                });
            });
        });
    });
    drop_dragged(state, ui, &s.career.world, &targets);
    // bz: the space bar plays, like the button
    if ui.input(|i| i.key_pressed(egui::Key::Space)) {
        play = true;
    }

    if play {
        match state.lineup.confirm(&mut s.career.world) {
            Ok(()) => {
                state.error = None;
                *open = false;
                match s.career.start_live_round() {
                    Ok(lr) => {
                        s.round_state = Some(crate::screens::round::RoundState::new(lr));
                        s.round_open = true;
                    }
                    Err(CareerError::NotPortedYet(what)) => s.status = Some((what, theme::WARN)),
                    Err(CareerError::Read(e)) => s.status = Some((e, theme::BAD)),
                }
            }
            Err(_) => state.error = Some(t.text("lineup-error-eleven", None)),
        }
    }
}

/// The dragged player follows the pointer; on release it goes to the place under it.
fn drop_dragged(state: &mut LineupState, ui: &egui::Ui, world: &World, targets: &Targets) {
    let Some(from) = state.dragging else { return };
    let (pointer, down, released) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.pointer.any_released()));
    if let Some(pos) = pointer
        && down
        && let Some(p) = state.lineup.grid.get(from).and_then(|place| place.player)
    {
        let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("lineup-drag")));
        let x = world.player(p);
        painter.circle_filled(pos + vec2(0.0, 3.0), TOKEN + 2.0, Color32::from_black_alpha(90));
        token(&painter, pos, x.position, x.strength, &x.name, true, false);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    if released || !down {
        if let Some(pos) = pointer
            && let Some(&(to, _)) = targets.iter().find(|(_, r)| r.contains(pos))
            && to != from
        {
            state.lineup.move_player(from, to);
        }
        state.dragging = None;
    }
}

/// The pitch, its lines and the 25 places. Returns a role picked in the right button's menu:
/// 0 free kicks, 1 captain, 2 corners, and the player.
fn pitch(state: &mut LineupState, ui: &mut egui::Ui, t: &Translator, world: &World, targets: &mut Targets, size: egui::Vec2) -> Option<(usize, brasfoot_model::ids::PlayerId)> {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    paint_pitch(ui.painter(), rect);
    let pointer = ui.input(|i| i.pointer.latest_pos());
    let club = world.club(state.lineup.club);
    let (captain, free_kicks, corners) = (club.captain, club.free_kick_taker, club.corner_taker);
    let dragging_on_pitch = state.dragging.is_some_and(|d| (2..=25).contains(&d));
    let mut set = None;
    for slot in 1..=25 {
        let center = slot_center(rect, slot);
        let area = Rect::from_center_size(center, vec2(64.0, 52.0));
        targets.push((slot, area));
        let response = ui.interact(area, ui.make_persistent_id(("lineup-place", slot)), egui::Sense::click_and_drag());
        state.handle(slot, &response);
        let place = state.lineup.grid[slot];
        let hovered_target = state.dragging.is_some() && pointer.is_some_and(|p| area.contains(p));
        let painter = ui.painter();
        match place.player {
            Some(p) if state.dragging != Some(slot) => {
                let x = world.player(p);
                let selected = state.selected_slot == Some(slot) || hovered_target;
                token(painter, center, x.position, x.strength, &x.name, selected, captain == Some(p));
                if response.hovered() && state.dragging.is_none() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                // bz.AW: the right button's menu
                response.context_menu(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (key, which, current) in [
                        ("lineup-context-free-kick", 0, free_kicks),
                        ("lineup-context-captain", 1, captain),
                        ("lineup-context-corner", 2, corners),
                    ] {
                        let text = t.text(key, None);
                        let text = if current == Some(p) { t.text("lineup-context-current", Some(&args([("what", text)]))) } else { text };
                        if widgets::menu_entry(ui, &text, true, "") {
                            set = Some((which, p));
                            ui.close();
                        }
                    }
                });
            }
            Some(_) => {}
            None => {
                if place.shirt {
                    painter.circle_filled(center, TOKEN - 2.0, Color32::from_white_alpha(18));
                    painter.circle_stroke(center, TOKEN - 2.0, Stroke::new(1.5, Color32::from_white_alpha(if hovered_target { 220 } else { 90 })));
                } else if dragging_on_pitch {
                    painter.circle_filled(center, if hovered_target { 9.0 } else { 4.0 }, Color32::from_white_alpha(if hovered_target { 200 } else { 70 }));
                }
            }
        }
    }
    set
}

/// The bench (places 26 to 36): three columns of chips.
fn bench(state: &mut LineupState, ui: &mut egui::Ui, t: &Translator, world: &World, targets: &mut Targets) {
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        menu::panel_title(ui, &t.text("lineup-bench-title", None), |_| {});
        let width = (ui.available_width() - 16.0) / 3.0;
        let pointer = ui.input(|i| i.pointer.latest_pos());
        for row in 0..4 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for column in 0..3 {
                    let slot = FIRST_BENCH_PLACE + row * 3 + column;
                    if slot > LAST_BENCH_PLACE {
                        continue;
                    }
                    let (rect, response) = ui.allocate_exact_size(vec2(width, 30.0), egui::Sense::click_and_drag());
                    targets.push((slot, rect));
                    state.handle(slot, &response);
                    let hovered_target = state.dragging.is_some() && pointer.is_some_and(|p| rect.contains(p));
                    player_row(ui, t, world, state, slot, rect, &response, hovered_target);
                }
            });
            ui.add_space(4.0);
        }
    });
}

/// A row of the bench or of the list: position, name, energy and strength.
#[allow(clippy::too_many_arguments)]
fn player_row(ui: &egui::Ui, t: &Translator, world: &World, state: &LineupState, slot: usize, rect: Rect, response: &egui::Response, hovered_target: bool) {
    let place = state.lineup.grid[slot];
    let hover = motion::toward(ui.ctx(), response.id, response.hovered() && state.dragging.is_none(), motion::HOVER);
    let selected = state.selected_slot == Some(slot) || hovered_target;
    let fill = if selected { theme::SELECTION } else { motion::mix(theme::CARD, theme::HOVER, hover) };
    let painter = ui.painter_at(rect);
    match place.player {
        Some(p) if state.dragging != Some(slot) => {
            painter.rect_filled(rect, 6.0, fill);
            let x = world.player(p);
            widgets::paint_position_badge(&painter, t, pos2(rect.left() + 16.0, rect.center().y), x.position);
            let right = rect.right() - 10.0;
            painter.text(pos2(right, rect.center().y), egui::Align2::RIGHT_CENTER, x.strength.to_string(), theme::bold(12.0), theme::TEXT);
            let energy = Rect::from_min_size(pos2(right - 74.0, rect.center().y - 3.0), vec2(42.0, 6.0));
            painter.rect_filled(energy, 3.0, theme::HEADER);
            painter.rect_filled(Rect::from_min_size(energy.min, vec2(energy.width() * x.fitness.clamp(0, 100) as f32 / 100.0, energy.height())), 3.0, theme::energy_color(x.fitness));
            let name_room = Rect::from_x_y_ranges(rect.left() + 32.0..=energy.left() - 6.0, rect.y_range());
            ui.painter().with_clip_rect(name_room).text(pos2(name_room.left(), rect.center().y), egui::Align2::LEFT_CENTER, &x.name, theme::regular(12.0), theme::TEXT);
            if response.hovered() && state.dragging.is_none() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }
        _ => {
            let stroke = Stroke::new(1.0, if hovered_target { theme::ACCENT } else { theme::BORDER });
            painter.rect_stroke(rect.shrink(0.5), 6.0, stroke, egui::StrokeKind::Inside);
            if place.shirt {
                painter.text(rect.center(), egui::Align2::CENTER_CENTER, t.text("lineup-bench-empty", None), theme::regular(11.0), theme::MUTED);
            }
        }
    }
}

/// The players left out (`bz.AB`, places from 37 on).
fn spares(state: &mut LineupState, ui: &mut egui::Ui, t: &Translator, world: &World, targets: &mut Targets, height: f32) {
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        let count = state.lineup.grid[FIRST_SPARE_PLACE..].iter().filter(|p| p.player.is_some()).count();
        menu::panel_title(ui, &t.text("lineup-available-title", None), |ui| {
            ui.label(egui::RichText::new(count.to_string()).font(theme::bold(12.0)).color(theme::MUTED));
        });
        let pointer = ui.input(|i| i.pointer.latest_pos());
        egui::ScrollArea::vertical().id_salt("lineup-spares").max_height(height).auto_shrink([false, true]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let clip = ui.clip_rect();
            for slot in FIRST_SPARE_PLACE..state.lineup.grid.len() {
                if state.lineup.grid[slot].player.is_none() {
                    continue;
                }
                let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), egui::Sense::click_and_drag());
                targets.push((slot, rect.intersect(clip)));
                state.handle(slot, &response);
                let hovered_target = state.dragging.is_some() && pointer.is_some_and(|p| rect.intersect(clip).contains(p));
                player_row(ui, t, world, state, slot, rect, &response, hovered_target);
            }
        });
    });
}

/// The automatic selection, the style, the marking and where to attack (`bz.vd`).
fn tactics(state: &mut LineupState, ui: &mut egui::Ui, t: &Translator, world: &mut World) {
    let club = state.lineup.club;
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                menu::field_label(ui, &t.text("lineup-auto-selection", None));
                let mut picked = None;
                egui::ComboBox::from_id_salt("lineup-formation").width(170.0).height(420.0).selected_text(t.text(FORMATION_KEYS[state.selected_formation], None)).show_ui(ui, |ui| {
                    for (i, key) in FORMATION_KEYS.iter().enumerate().skip(1) {
                        if ui.selectable_label(state.selected_formation == i, t.text(key, None)).clicked() {
                            picked = Some(i);
                        }
                    }
                });
                // bz.bR: cU(index - 1), 0 being "Resetar tudo"
                if let Some(i) = picked {
                    state.selected_formation = i;
                    state.lineup.pick_formation(world, i - 1);
                    state.selected_slot = None;
                }
                ui.add_space(8.0);
                let mut default = world.game.flag_bd;
                ui.horizontal(|ui| {
                    if menu::check(ui, default, true).clicked() {
                        default = !default;
                    }
                    ui.label(egui::RichText::new(t.text("lineup-default-checkbox", None)).size(12.0));
                });
                // bz.bE: c.a.SR.f(selected)
                world.game.flag_bd = default;
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                // bz.bS, bT, bU
                widgets::tactics_combos(ui, t, "lineup-tactics", &mut world.club_mut(club).tactics);
            });
        });
    });
}

/// `bz.we`: the opponent and the pitch's condition.
fn match_card(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, club: ClubId, m: MatchId) {
    let g = world.game_match(m);
    let opponent = if g.home == Some(club) { g.away } else { g.home };
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            if let Some(o) = opponent {
                crest(ui, files, world, o, 40.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(t.text("lineup-opponent-label", None)).font(theme::regular(11.0)).color(theme::MUTED));
                    ui.label(egui::RichText::new(&world.club(o).name).font(theme::bold(15.0)));
                });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(t.text("lineup-pitch-condition", None)).font(theme::regular(11.0)).color(theme::MUTED));
                    let condition = g.stadium_condition.clamp(0, 3) as usize;
                    let color = [theme::GOOD, theme::GOOD, theme::WARN, theme::BAD][condition];
                    ui.label(egui::RichText::new(t.text(CONDITION_KEYS[condition], None)).font(theme::bold(14.0)).color(color));
                });
            });
        });
    });
}
