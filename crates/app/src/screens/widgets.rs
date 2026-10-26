//! Small pieces the screens share.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, CompetitionId, MatchId};
use brasfoot_model::world::World;
use eframe::egui;

use crate::game_files::GameFiles;
use crate::theme;

/// A club's crest, or its initial on the theme's card when the game has no picture.
pub fn crest(ui: &mut egui::Ui, files: &GameFiles, world: &World, club: ClubId, size: f32) {
    let c = world.club(club);
    if let Some(uri) = files.crest(ui.ctx(), &c.file_ref) {
        ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(size, size)));
    } else {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), size / 2.0, theme::PANEL_HIGH);
        ui.painter().circle_stroke(rect.center(), size / 2.0 - 0.5, egui::Stroke::new(1.0, theme::GRID));
        let initial = c.name.chars().next().unwrap_or(' ').to_string();
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, initial, theme::bold(size * 0.45), theme::TEXT);
    }
}

/// `text` cut with "…" so that it fits in `max_width` with `font`.
pub fn fit_text(painter: &egui::Painter, text: &str, font: egui::FontId, max_width: f32) -> String {
    let width = |s: &str| painter.layout_no_wrap(s.to_owned(), font.clone(), egui::Color32::WHITE).size().x;
    if width(text) <= max_width {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut keep = chars.len();
    while keep > 0 {
        keep -= 1;
        let cut = format!("{}…", chars[..keep].iter().collect::<String>().trim_end());
        if width(&cut) <= max_width {
            return cut;
        }
    }
    String::new()
}

/// A country's flag from the game jar (`/aflags/{id}.png`).
pub fn flag(ui: &mut egui::Ui, files: &GameFiles, id: i32, size: f32) {
    if let Some(uri) = files.jar_image(ui.ctx(), &format!("aflags/{id}.png")) {
        ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(size, size)));
    } else {
        ui.add_space(size);
    }
}

/// A competition's name: the saved one, or composed from the catalog for the competitions this
/// app created (the original composed them in Portuguese when it created them).
pub fn competition_name(t: &Translator, world: &World, c: CompetitionId) -> String {
    brasfoot_i18n::labels::competition(t, &world.competition_label(c))
}

/// The country of a national competition (league, cup, super cup, state or regional
/// championship), for its flag; none for the continental and world ones.
pub fn competition_country(world: &World, c: CompetitionId) -> Option<i32> {
    use brasfoot_model::saved::CompetitionKind;
    let comp = world.competition(c);
    let country = match comp.class {
        CompetitionKind::NationalSuperCup => Some(comp.country),
        CompetitionKind::StateChampionship | CompetitionKind::RegionalCup => Some(brasfoot_datapack::codes::BRAZIL),
        _ => comp.country_ref.map(|k| world.country(k).country),
    };
    country.filter(|&id| id >= 0)
}

/// A competition's flag, or its room when it has no country.
pub fn competition_flag(ui: &mut egui::Ui, files: &GameFiles, world: &World, c: CompetitionId, size: f32) {
    match competition_country(world, c) {
        Some(country) => flag(ui, files, country, size),
        None => {
            ui.add_space(size);
        }
    }
}

/// A drop-down of competitions with their flags: `shown` is the text when none is chosen. The
/// competition picked, if any.
#[allow(clippy::too_many_arguments)]
pub fn competition_picker(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, id: &str, width: f32, current: Option<CompetitionId>, shown: String, choices: &[CompetitionId]) -> Option<CompetitionId> {
    let mut picked = None;
    ui.horizontal(|ui| {
        if let Some(c) = current {
            competition_flag(ui, files, world, c, 16.0);
        }
        egui::ComboBox::from_id_salt(id).width(width).height(400.0).selected_text(shown).show_ui(ui, |ui| {
            for &c in choices {
                ui.horizontal(|ui| {
                    competition_flag(ui, files, world, c, 16.0);
                    if ui.selectable_label(current == Some(c), competition_name(t, world, c)).clicked() {
                        picked = Some(c);
                    }
                });
            }
        });
    });
    picked
}

/// Port of `f.s.getNome()`: the league's name without "Campeonato ...", and the division.
pub fn stage_name(t: &Translator, world: &World, stage: brasfoot_model::ids::StageId) -> String {
    brasfoot_i18n::labels::stage(t, &world.stage_label(stage))
}

/// `aq.rI`: G, L, Z, M, A.
pub fn position_short(t: &Translator, position: i32) -> String {
    t.text(&format!("position-short-{position}"), None)
}

pub fn position_name(t: &Translator, position: i32) -> String {
    t.text(&format!("position-{position}"), None)
}

/// `aq.rK`: D, E.
pub fn side_short(t: &Translator, side: i32) -> String {
    t.text(&format!("side-short-{side}"), None)
}

/// `aq.qN`: the traits' abbreviations.
pub fn trait_short(t: &Translator, id: i32) -> String {
    t.text(&format!("trait-short-{id}"), None)
}

/// A value from 0 to 100 as a bar with the number on it.
pub fn bar(ui: &mut egui::Ui, value: i32, width: f32) {
    ui.add(egui::ProgressBar::new(value.clamp(0, 100) as f32 / 100.0).desired_width(width).fill(theme::ACCENT).text(value.to_string()));
}

/// "2 x 1", or "x" when not played (`I.t(false)`).
pub fn score(world: &World, m: MatchId) -> String {
    let g = world.game_match(m);
    if g.played { format!("{} x {}", g.home_goals, g.away_goals) } else { "x".to_owned() }
}

/// A rating colored like the game's NM column: red below 5, green above 7.
pub fn rating_color(rating: f64) -> egui::Color32 {
    if rating < 5.0 {
        theme::TEXT_RED
    } else if rating > 7.0 {
        theme::TEXT_GREEN
    } else {
        theme::TEXT
    }
}

/// A picture of the game's jar at a size.
pub fn jar_icon(ui: &mut egui::Ui, files: &GameFiles, path: &str, size: f32) {
    if let Some(uri) = files.jar_image(ui.ctx(), path) {
        ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(size, size)));
    } else {
        ui.add_space(size);
    }
}

/// A clickable button with a jar image.
pub fn jar_button(ui: &mut egui::Ui, files: &GameFiles, path: &str, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter().rect_filled(rect, 3.0, theme::HOVER);
    }
    if let Some(uri) = files.jar_image(ui.ctx(), path) {
        egui::Image::new(uri).fit_to_exact_size(egui::vec2(size, size)).paint_at(ui, rect);
    }
    response
}

/// The confidence bars of the club panel: a dark track filled with the club's text color and
/// the percentage on it (`Visual.barra`).
pub fn club_bar(ui: &mut egui::Ui, value: i32, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(122.0), 14.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::HEADER);
    let fill = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * value.clamp(0, 100) as f32 / 100.0, rect.height()));
    ui.painter().rect_filled(fill, 0.0, color);
    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, color), egui::StrokeKind::Inside);
    let light = color.r() as u32 + color.g() as u32 + color.b() as u32 > 380;
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{value}%"), theme::regular(9.0), if light { theme::HEADER } else { theme::TEXT });
}

/// The energy column: a slim bar colored by what is left, with the percentage.
pub fn energy_bar(ui: &mut egui::Ui, value: i32) {
    paint_energy_bar(ui.painter(), ui.max_rect(), value);
}

pub fn paint_energy_bar(painter: &egui::Painter, cell: egui::Rect, value: i32) {
    let rect = egui::Rect::from_center_size(cell.center(), egui::vec2((cell.width() - 12.0).max(20.0), 13.0));
    painter.rect_filled(rect, 1.0, theme::HEADER);
    let fill = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * value.clamp(0, 100) as f32 / 100.0, rect.height()));
    painter.rect_filled(fill, 1.0, theme::energy_color(value));
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, format!("{value}%"), theme::regular(9.0), egui::Color32::WHITE);
}

/// `best.aq` round names, as catalog keys (`ps`, `px`, `py`, `pz`, `pA`, `pB`, `pC`, `pD`,
/// `pX`, `abY`).
const ROUNDS_PS: [&str; 7] = ["preliminary-1", "preliminary-2", "pre-round-of-16", "round-of-16", "quarterfinal", "semifinal", "final"];
const ROUNDS_PX: [&str; 7] = ["preliminary-stage", "first-stage", "pre-round-of-16", "round-of-16", "quarterfinal", "semifinal", "final"];
const ROUNDS_FROM_FIRST_STAGE: [&str; 6] = ["first-stage", "pre-round-of-16", "round-of-16", "quarterfinal", "semifinal", "final"];

fn default_round_names(phase_code: i32, last_round: i32) -> &'static [&'static str] {
    match (phase_code, last_round) {
        (1068, _) => &["playoff-decider"],
        (6006, _) => &["pre-round-of-16"],
        (_, 6) => &ROUNDS_PX,
        (_, 5) => &ROUNDS_FROM_FIRST_STAGE,
        (_, 4) => &ROUNDS_FROM_FIRST_STAGE[1..],
        (_, 3) => &ROUNDS_FROM_FIRST_STAGE[2..],
        (_, 2) => &ROUNDS_FROM_FIRST_STAGE[3..],
        (_, 1) => &ROUNDS_FROM_FIRST_STAGE[4..],
        (_, 0) => &["decider"],
        _ => &ROUNDS_PS,
    }
}

/// `L.io()`: the round a stage is at, "3ª rodada" for a table (`f.s`) or the knockout round with
/// its leg (`f.z`, names from `zB()`; `at.b(L)` never gives its own).
pub fn round_text(t: &Translator, world: &World, stage: brasfoot_model::ids::StageId) -> String {
    let s = world.stage(stage);
    if s.class == brasfoot_model::saved::StageKind::League {
        return t.text("round-league", Some(&args([("round", s.round.to_string())])));
    }
    // the original writes ZG back from 7 to 6 here, a getter with a side effect; the screen only
    // reads, so it uses 6 without storing it
    let last = if s.last_round == 7 { 6 } else { s.last_round };
    let names = default_round_names(s.phase_code, last);
    let round = s.round;
    let index = (names.len() as i32 - 1 - last + round).clamp(0, names.len() as i32 - 1) as usize;
    let mut key = names[index];
    if last == 0 && round == 0 && (s.competition_kind == 4 || s.competition_kind == 5) {
        key = names[0];
    }
    if last == 1 && s.competition_kind == 4 && round == 0 {
        key = "preliminary-r1";
    }
    if last == 1 && s.competition_kind == 4 && round == 1 {
        key = "preliminary-r2";
    }
    let name = t.text(&format!("round-name-{key}"), None);
    let two_legged = usize::try_from(round).ok().and_then(|r| s.rounds.get(r)).is_some_and(|r| r.two_legged);
    if two_legged { t.text("round-leg", Some(&args([("name", name), ("leg", s.leg.to_string())]))) } else { name }
}

/// `I.hI()`: the competition and the round of a match.
pub fn match_round_text(t: &Translator, world: &World, m: brasfoot_model::ids::MatchId) -> String {
    let g = world.game_match(m);
    let competition = g.competition.map(|c| competition_name(t, world, c)).unwrap_or_default();
    match g.stage {
        Some(st) => t.text("match-round", Some(&args([("competition", competition), ("round", round_text(t, world, st))]))),
        None => competition,
    }
}

/// The top of a dialog: an icon (`glyphs`), the title, and a close button on the right. True
/// when the close button was clicked.
pub fn dialog_title(ui: &mut egui::Ui, files: &GameFiles, icon: Option<&str>, title: &str) -> bool {
    dialog_title_with(ui, files, icon, title, |_| {})
}

/// `dialog_title` with more after the title (a counter, a status).
pub fn dialog_title_with(ui: &mut egui::Ui, files: &GameFiles, icon: Option<&str>, title: &str, after: impl FnOnce(&mut egui::Ui)) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        let _ = files;
        if let Some(glyph) = icon {
            icon_glyph(ui, glyph, 24.0, theme::ACCENT);
            ui.add_space(6.0);
        }
        ui.label(egui::RichText::new(title).font(theme::bold(18.0)).color(theme::TEXT));
        after(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close = close_button(ui);
        });
    });
    ui.add_space(10.0);
    close
}

/// A slider with two handles over `min..=max`; the handle nearest the press moves. Returns true
/// when the range changed.
pub fn range_slider(ui: &mut egui::Ui, range: &mut (i32, i32), min: i32, max: i32, width: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 22.0), egui::Sense::click_and_drag());
    let track = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width() - 16.0, 4.0));
    let to_x = |v: i32| track.left() + track.width() * (v - min) as f32 / (max - min).max(1) as f32;
    let to_v = |x: f32| (min as f32 + ((x - track.left()) / track.width()).clamp(0.0, 1.0) * (max - min) as f32).round() as i32;
    let before = *range;
    if let Some(pos) = response.interact_pointer_pos() {
        let id = response.id.with("handle");
        // the handle picked when the press starts stays picked through the drag
        let high = if response.drag_started() || response.clicked() || !ui.memory(|m| m.data.get_temp::<bool>(id).is_some()) {
            let pick = (pos.x - to_x(range.1)).abs() < (pos.x - to_x(range.0)).abs() || (range.0 == range.1 && pos.x > to_x(range.1));
            ui.memory_mut(|m| m.data.insert_temp(id, pick));
            pick
        } else {
            ui.memory(|m| m.data.get_temp::<bool>(id)).unwrap_or(false)
        };
        let v = to_v(pos.x);
        if high {
            range.1 = v.max(range.0);
        } else {
            range.0 = v.min(range.1);
        }
    }
    if response.drag_stopped() {
        ui.memory_mut(|m| m.data.remove::<bool>(response.id.with("handle")));
    }
    let p = ui.painter();
    p.rect_filled(track, 2.0, theme::HEADER);
    let filled = egui::Rect::from_x_y_ranges(to_x(range.0)..=to_x(range.1), track.y_range());
    p.rect_filled(filled, 2.0, theme::ACCENT);
    for v in [range.0, range.1] {
        let c = egui::pos2(to_x(v), rect.center().y);
        p.circle_filled(c, 7.0, theme::TEXT);
        p.circle_stroke(c, 7.0, egui::Stroke::new(2.0, theme::ACCENT));
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    *range != before
}

/// A picture of the game's jar painted into `rect`.
pub fn paint_jar_image(ui: &mut egui::Ui, files: &GameFiles, path: &str, rect: egui::Rect) {
    if let Some(uri) = files.jar_image(ui.ctx(), path) {
        egui::Image::new(uri).fit_to_exact_size(rect.size()).paint_at(ui, rect);
    }
}

/// An entry of a toolbar menu (the game's popup menus): a row the menu's width, lit under the
/// mouse. Entries not ported yet are greyed out and say so. True when clicked.
pub fn menu_entry(ui: &mut egui::Ui, text: &str, enabled: bool, not_ported: &str) -> bool {
    let width = ui.available_width().max(240.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let hover = crate::motion::toward(ui.ctx(), response.id, enabled && response.hovered(), crate::motion::HOVER);
    if hover > 0.0 {
        ui.painter().rect_filled(rect, 6.0, crate::motion::mix(egui::Color32::TRANSPARENT, theme::SELECTION, hover));
    }
    let color = if enabled { theme::TEXT } else { theme::MUTED };
    ui.painter().text(rect.left_center() + egui::vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, text, theme::regular(13.0), color);
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    } else {
        response.on_hover_text(not_ported);
        false
    }
}

/// The thin line between the groups of a menu.
pub fn menu_separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 7.0), egui::Sense::hover());
    ui.painter().line_segment([rect.left_center(), rect.right_center()], egui::Stroke::new(1.0, theme::BORDER));
}

/// A position's letter on a rounded badge in its color.
pub fn paint_position_badge(painter: &egui::Painter, t: &Translator, center: egui::Pos2, position: i32) {
    let rect = egui::Rect::from_center_size(center, egui::vec2(20.0, 18.0));
    painter.rect_filled(rect, 5.0, theme::position_color(position));
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, position_short(t, position), theme::bold(10.0), egui::Color32::WHITE);
}

/// `aq.rC`, `aq.rE`, `aq.rD`: the style, the marking and where to attack (`ah.kj()[1..=3]`).
const TACTICS: [(&str, &[&str]); 3] = [
    ("lineup-play-style", &["lineup-style-balanced", "lineup-style-all-out-attack", "lineup-style-counter-attack"]),
    ("lineup-marking", &["lineup-marking-light", "lineup-marking-heavy", "lineup-marking-very-heavy"]),
    ("lineup-attack-focus", &["lineup-attack-center", "lineup-attack-flanks"]),
];

/// The three tactics combos side by side (`bz.Bg`, `Bh`, `Bi`; `cD.Do`, `Dp`, `Dn`): each
/// writes its index into `kj()[1..=3]`.
pub fn tactics_combos(ui: &mut egui::Ui, t: &Translator, id: &str, tactics: &mut Vec<i32>) {
    if tactics.len() < 4 {
        tactics.resize(4, 0);
    }
    let width = ((ui.available_width() - 24.0) / 3.0).clamp(110.0, 170.0);
    ui.horizontal_top(|ui| {
        for (i, (label, keys)) in TACTICS.iter().enumerate() {
            let index = i + 1;
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.label(egui::RichText::new(t.text(label, None)).font(theme::bold(12.0)).color(theme::MUTED));
                let current = (tactics[index].max(0) as usize).min(keys.len() - 1);
                egui::ComboBox::from_id_salt((id, index)).width(width - 8.0).selected_text(t.text(keys[current], None)).show_ui(ui, |ui| {
                    for (k, key) in keys.iter().enumerate() {
                        if ui.selectable_label(current == k, t.text(key, None)).clicked() {
                            tactics[index] = k as i32;
                        }
                    }
                });
            });
            ui.add_space(8.0);
        }
    });
}

/// The dialogs' close button: a cross that lights under the mouse. True when clicked.
pub fn close_button(ui: &mut egui::Ui) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
    let hover = crate::motion::toward(ui.ctx(), response.id, response.hovered(), crate::motion::HOVER);
    ui.painter().rect_filled(rect, 6.0, crate::motion::mix(egui::Color32::TRANSPARENT, theme::PANEL_HIGH, hover));
    let c = rect.center();
    let stroke = egui::Stroke::new(1.8, crate::motion::mix(theme::MUTED, theme::TEXT, hover));
    ui.painter().line_segment([c + egui::vec2(-6.0, -6.0), c + egui::vec2(6.0, 6.0)], stroke);
    ui.painter().line_segment([c + egui::vec2(-6.0, 6.0), c + egui::vec2(6.0, -6.0)], stroke);
    response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// An icon (`glyphs`) painted centered in `rect`.
pub fn paint_icon(painter: &egui::Painter, rect: egui::Rect, glyph: &str, color: egui::Color32) {
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, glyph, theme::icon_font(rect.height()), color);
}

/// An icon of `size`.
pub fn icon_glyph(ui: &mut egui::Ui, glyph: &str, size: f32, color: egui::Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    paint_icon(ui.painter(), rect, glyph, color);
    response
}

/// A clickable icon that lights up under the mouse; `color` is its own tint.
pub fn icon_button(ui: &mut egui::Ui, glyph: &str, size: f32, color: egui::Color32, enabled: bool) -> egui::Response {
    let pad = (size * 0.22).round();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size + 2.0 * pad, size + 2.0 * pad), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let hover = crate::motion::toward(ui.ctx(), response.id, enabled && response.hovered(), crate::motion::HOVER);
    ui.painter().rect_filled(rect, 8.0, crate::motion::mix(theme::CARD, theme::PANEL_HIGH, hover));
    let tint = if enabled { color } else { color.gamma_multiply(0.35) };
    paint_icon(ui.painter(), rect.shrink(pad), glyph, tint);
    if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
}
