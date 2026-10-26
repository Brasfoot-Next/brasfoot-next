//! The pitch the app draws for the line-up (`a.bz`) and the substitutions window (`a.cD`), in
//! the theme instead of `aicons/campo.png`, and the players on it.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, pos2, vec2};

use crate::theme;

pub const TOKEN: f32 = 18.0;

/// The places 1..=25 on the original's 434x650 pitch (`components.ao.RJ`), top left of the
/// player's figure; the figure's center is 30 right and 25 down.
pub const PITCH_SLOTS: [(f32, f32); 26] = [
    (0.0, 0.0),
    (190.0, 565.0),
    (35.0, 425.0),
    (100.0, 485.0),
    (190.0, 485.0),
    (285.0, 485.0),
    (100.0, 405.0),
    (190.0, 405.0),
    (285.0, 405.0),
    (345.0, 425.0),
    (35.0, 265.0),
    (100.0, 315.0),
    (190.0, 315.0),
    (285.0, 315.0),
    (100.0, 205.0),
    (190.0, 205.0),
    (285.0, 205.0),
    (345.0, 265.0),
    (35.0, 55.0),
    (100.0, 115.0),
    (190.0, 115.0),
    (285.0, 115.0),
    (100.0, 35.0),
    (190.0, 30.0),
    (285.0, 35.0),
    (345.0, 55.0),
];

/// The center of a place (1 to 25) on a pitch drawn in `rect`.
pub fn slot_center(rect: Rect, slot: usize) -> Pos2 {
    let (x, y) = PITCH_SLOTS[slot];
    rect.min + vec2((x + 30.0) / 434.0 * rect.width(), (y + 25.0) / 650.0 * rect.height())
}

/// The pitch as the app draws it: stripes and the lines.
pub fn paint_pitch(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 12.0, theme::PITCH_DARK);
    let inner = rect.shrink(6.0);
    let stripes = 12;
    let h = inner.height() / stripes as f32;
    for i in (0..stripes).step_by(2) {
        painter.rect_filled(Rect::from_min_size(pos2(inner.left(), inner.top() + i as f32 * h), vec2(inner.width(), h)), 0.0, theme::PITCH_LIGHT);
    }
    let line = Stroke::new(1.5, Color32::from_white_alpha(130));
    let field = rect.shrink(16.0);
    painter.rect_stroke(field, 2.0, line, egui::StrokeKind::Middle);
    painter.line_segment([field.left_center(), field.right_center()], line);
    painter.circle_stroke(field.center(), field.width() * 0.15, line);
    painter.circle_filled(field.center(), 3.0, Color32::from_white_alpha(150));
    for top in [true, false] {
        let (edge, dir) = if top { (field.top(), 1.0) } else { (field.bottom(), -1.0) };
        let area = |w: f32, h: f32| {
            let y2 = edge + dir * h;
            Rect::from_x_y_ranges(field.center().x - w / 2.0..=field.center().x + w / 2.0, edge.min(y2)..=edge.max(y2))
        };
        painter.rect_stroke(area(field.width() * 0.58, field.height() * 0.15), 0.0, line, egui::StrokeKind::Middle);
        painter.rect_stroke(area(field.width() * 0.28, field.height() * 0.055), 0.0, line, egui::StrokeKind::Middle);
        painter.circle_filled(pos2(field.center().x, edge + dir * field.height() * 0.105), 2.5, Color32::from_white_alpha(150));
    }
}

/// A player on the pitch: a disc in the position's color with the strength, the name under it.
pub fn token(painter: &egui::Painter, center: Pos2, position: i32, strength: i32, name: &str, selected: bool, captain: bool) {
    painter.circle_filled(center, TOKEN, theme::position_color(position));
    let ring = if selected { Stroke::new(3.0, Color32::WHITE) } else { Stroke::new(1.5, Color32::from_white_alpha(170)) };
    painter.circle_stroke(center, TOKEN, ring);
    painter.text(center, egui::Align2::CENTER_CENTER, strength.to_string(), theme::bold(13.0), Color32::WHITE);
    let mut job = egui::text::LayoutJob::simple_singleline(name.to_owned(), theme::bold(10.0), Color32::WHITE);
    job.wrap = egui::text::TextWrapping::truncate_at_width(86.0);
    let galley = painter.layout_job(job);
    let pill = Rect::from_center_size(center + vec2(0.0, TOKEN + 10.0), galley.size() + vec2(10.0, 3.0));
    painter.rect_filled(pill, 7.0, Color32::from_black_alpha(165));
    painter.galley(pill.min + vec2(5.0, 1.5), galley, Color32::WHITE);
    if captain {
        let badge = center + vec2(TOKEN * 0.8, -TOKEN * 0.8);
        painter.circle_filled(badge, 7.5, theme::STAR);
        painter.text(badge, egui::Align2::CENTER_CENTER, "C", theme::bold(9.0), theme::HEADER);
    }
}

