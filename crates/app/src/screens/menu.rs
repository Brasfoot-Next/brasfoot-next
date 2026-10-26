//! The look the menus outside a career share (opening screen, saved games): a navy backdrop,
//! one centered column, and large rows to click with a drawn icon.

use std::f32::consts::TAU;

use brasfoot_i18n::Translator;
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

use crate::theme;

/// The icons of the menu rows, drawn in code.
#[derive(Clone, Copy)]
pub enum Glyph {
    Ball,
    Play,
    Folder,
    Import,
    File,
    Power,
    /// Stacked sheets: the layers of data sources.
    Layers,
}

/// Fills the screen with the backdrop and runs `add_contents` in a centered column `width` wide.
pub fn page<R>(ui: &mut egui::Ui, width: f32, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(theme::OUTER))
        .show(ui, |ui| {
            backdrop(ui.painter(), ui.max_rect());
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let full = ui.available_width();
                    let width = width.min(full - 32.0);
                    ui.horizontal_top(|ui| {
                        ui.add_space((full - width) / 2.0);
                        ui.vertical(|ui| {
                            ui.set_width(width);
                            add_contents(ui)
                        })
                        .inner
                    })
                    .inner
                })
                .inner
        })
        .inner
}

/// A soft vertical gradient with a faint glow of the accent at the top.
fn backdrop(p: &Painter, rect: Rect) {
    let top = Color32::from_rgb(0x15, 0x20, 0x37);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), theme::OUTER);
    mesh.colored_vertex(rect.left_bottom(), theme::OUTER);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(Shape::mesh(mesh));
    let center = pos2(rect.center().x, rect.top() + 40.0);
    // many faint rings so the glow has no visible edge
    for i in 0..16 {
        let radius = 560.0 - i as f32 * 30.0;
        p.circle_filled(center, radius, Color32::from_rgba_unmultiplied(0x3b, 0x82, 0xf6, 2));
    }
}

/// The app's name with its mark, centered.
pub fn brand(ui: &mut egui::Ui, t: &Translator, subtitle: Option<&str>) {
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 32.0, theme::ACCENT.gamma_multiply(0.18));
        glyph(ui.painter(), rect.shrink(14.0), Glyph::Ball, theme::TEXT);
        ui.add_space(10.0);
        ui.label(egui::RichText::new(t.text("app-title", None)).font(theme::bold(34.0)).color(theme::TEXT));
        if let Some(s) = subtitle {
            ui.add_space(2.0);
            ui.label(egui::RichText::new(s).font(theme::regular(14.0)).color(theme::MUTED));
        }
    });
}

/// A section title over a group of rows.
pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(18.0);
    ui.label(egui::RichText::new(text.to_uppercase()).font(theme::bold(11.0)).color(theme::MUTED));
    ui.add_space(6.0);
}

/// A large row to click: icon, title and a line of detail. `primary` paints it in the accent.
pub fn row(ui: &mut egui::Ui, glyph_kind: Glyph, title: &str, detail: &str, primary: bool, enabled: bool) -> egui::Response {
    let height = if detail.is_empty() { 52.0 } else { 64.0 };
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let hover = ui.ctx().animate_bool_with_time(response.id, enabled && response.hovered(), 0.12);
    let (base, lit) = if primary { (theme::ACCENT, Color32::from_rgb(0x52, 0x93, 0xf8)) } else { (theme::CARD, theme::CARD_HOVER) };
    let fill = if enabled { lerp(base, lit, hover) } else { theme::PANEL };
    let p = ui.painter();
    p.rect_filled(rect, 10.0, fill);
    if !primary {
        p.rect_stroke(rect, 10.0, Stroke::new(1.0, lerp(theme::BORDER, theme::GRID, hover)), egui::StrokeKind::Inside);
    }
    let ink = if enabled { if primary { Color32::WHITE } else { theme::TEXT } } else { theme::MUTED };
    let badge = Rect::from_center_size(pos2(rect.left() + 34.0, rect.center().y), vec2(36.0, 36.0));
    let badge_fill = if primary { Color32::from_white_alpha(36) } else { theme::ACCENT.gamma_multiply(if enabled { 0.22 } else { 0.08 }) };
    p.rect_filled(badge, 8.0, badge_fill);
    glyph(p, badge.shrink(9.0), glyph_kind, ink);
    let x = badge.right() + 14.0;
    let room = rect.right() - 16.0 - x;
    if detail.is_empty() {
        one_line(p, pos2(x, rect.center().y), title, theme::bold(15.0), ink, room);
    } else {
        one_line(p, pos2(x, rect.center().y - 9.0), title, theme::bold(15.0), ink, room);
        let detail_color = if primary { Color32::from_white_alpha(200) } else { theme::MUTED };
        one_line(p, pos2(x, rect.center().y + 11.0), detail, theme::regular(12.0), detail_color, room);
    }
    ui.add_space(8.0);
    if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
}

/// Going back, at the top left.
pub fn back(ui: &mut egui::Ui, t: &Translator) -> bool {
    crate::screens::controls::button(ui, &t.text("menu-back", None), crate::screens::controls::Kind::Secondary, true).clicked()
}

/// An error line in the theme's red, on a tinted strip.
pub fn error(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(theme::BAD.gamma_multiply(0.15))
        .stroke(Stroke::new(1.0, theme::BAD.gamma_multiply(0.5)))
        .corner_radius(8)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(text).color(theme::TEXT_RED));
        });
    ui.add_space(8.0);
}

/// Text on one line, vertically centered on `left_center`, cut with "…" past `width`.
fn one_line(p: &Painter, left_center: Pos2, text: &str, font: egui::FontId, color: Color32, width: f32) {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(0.0));
    let galley = p.layout_job(job);
    p.galley(left_center - vec2(0.0, galley.size().y / 2.0), galley, color);
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
}

/// Draws an icon inside `rect` (a square) with `color`.
pub fn glyph(p: &Painter, rect: Rect, kind: Glyph, color: Color32) {
    let c = rect.center();
    let r = rect.width() / 2.0;
    let line = Stroke::new((r * 0.16).max(1.5), color);
    match kind {
        Glyph::Ball => {
            p.circle_stroke(c, r, line);
            let pentagon: Vec<Pos2> = (0..5).map(|i| c + polar(r * 0.42, -TAU / 4.0 + i as f32 * TAU / 5.0)).collect();
            for &corner in &pentagon {
                p.line_segment([corner, c + (corner - c) * 2.35], Stroke::new(line.width * 0.7, color));
            }
            p.add(Shape::convex_polygon(pentagon, color, Stroke::NONE));
        }
        Glyph::Play => {
            let points = vec![c + vec2(-r * 0.55, -r * 0.8), c + vec2(r * 0.85, 0.0), c + vec2(-r * 0.55, r * 0.8)];
            p.add(Shape::convex_polygon(points, color, Stroke::NONE));
        }
        Glyph::Folder => {
            let body = Rect::from_min_max(c + vec2(-r, -r * 0.45), c + vec2(r, r * 0.8));
            let tab = Rect::from_min_max(c + vec2(-r, -r * 0.8), c + vec2(-r * 0.1, -r * 0.3));
            p.rect_filled(tab, 2.0, color);
            p.rect_filled(body, 3.0, color);
        }
        Glyph::Import => {
            p.line_segment([c + vec2(0.0, -r), c + vec2(0.0, r * 0.3)], line);
            p.line_segment([c + vec2(-r * 0.45, -r * 0.1), c + vec2(0.0, r * 0.35)], line);
            p.line_segment([c + vec2(r * 0.45, -r * 0.1), c + vec2(0.0, r * 0.35)], line);
            let tray = [c + vec2(-r, r * 0.25), c + vec2(-r, r * 0.9), c + vec2(r, r * 0.9), c + vec2(r, r * 0.25)];
            p.add(Shape::line(tray.to_vec(), line));
        }
        Glyph::File => {
            let fold = r * 0.45;
            let outline = vec![
                c + vec2(-r * 0.75, -r),
                c + vec2(r * 0.75 - fold, -r),
                c + vec2(r * 0.75, -r + fold),
                c + vec2(r * 0.75, r),
                c + vec2(-r * 0.75, r),
            ];
            p.add(Shape::closed_line(outline, line));
            p.add(Shape::line(vec![c + vec2(r * 0.75 - fold, -r), c + vec2(r * 0.75 - fold, -r + fold), c + vec2(r * 0.75, -r + fold)], line));
        }
        Glyph::Layers => {
            // three diamonds, the top one filled
            for (i, dy) in [r * 0.45, 0.0, -r * 0.45].into_iter().enumerate() {
                let center = c + vec2(0.0, dy);
                let diamond = vec![center + vec2(-r, 0.0), center + vec2(0.0, -r * 0.45), center + vec2(r, 0.0), center + vec2(0.0, r * 0.45)];
                if i == 2 {
                    p.add(Shape::convex_polygon(diamond, color, Stroke::NONE));
                } else {
                    p.add(Shape::closed_line(diamond, line));
                }
            }
        }
        Glyph::Power => {
            let arc: Vec<Pos2> = (0..=24).map(|i| c + polar(r * 0.85, -TAU / 4.0 + TAU * 0.12 + i as f32 * TAU * 0.76 / 24.0)).collect();
            p.add(Shape::line(arc, line));
            p.line_segment([c + vec2(0.0, -r), c + vec2(0.0, -r * 0.1)], line);
        }
    }
}

fn polar(radius: f32, angle: f32) -> egui::Vec2 {
    vec2(angle.cos(), angle.sin()) * radius
}

/// The top of a menu page: back link, title and a line under it. True when back was clicked.
pub fn header(ui: &mut egui::Ui, t: &Translator, title: &str, subtitle: &str, with_back: bool) -> bool {
    ui.add_space(20.0);
    let back = with_back && self::back(ui, t);
    ui.add_space(4.0);
    ui.label(egui::RichText::new(title).font(theme::bold(28.0)));
    if !subtitle.is_empty() {
        ui.label(egui::RichText::new(subtitle).font(theme::regular(13.0)).color(theme::MUTED));
    }
    ui.add_space(16.0);
    back
}

/// A rounded panel for a group of settings.
pub fn panel() -> egui::Frame {
    egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16)
}

/// A panel's title, with room on the right for `add_right` (small actions).
pub fn panel_title(ui: &mut egui::Ui, text: &str, add_right: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text).font(theme::bold(15.0)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add_right);
    });
    ui.add_space(8.0);
}

/// A field's label, above the field.
pub fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(theme::bold(12.0)).color(theme::MUTED));
    ui.add_space(2.0);
}







/// A check box drawn like the rest of the theme.
pub fn check(ui: &mut egui::Ui, value: bool, enabled: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered() && enabled, 0.1);
    let fill = if value { theme::ACCENT } else { lerp(theme::CARD, theme::PANEL_HIGH, hover) };
    let fill = if enabled { fill } else { fill.gamma_multiply(0.45) };
    ui.painter().rect_filled(rect.shrink(1.0), 4.0, fill);
    if !value {
        ui.painter().rect_stroke(rect.shrink(1.0), 4.0, Stroke::new(1.0, theme::GRID), egui::StrokeKind::Inside);
    }
    if value {
        let c = rect.center();
        let points = vec![c + vec2(-4.5, 0.0), c + vec2(-1.5, 3.5), c + vec2(4.5, -3.5)];
        ui.painter().add(Shape::line(points, Stroke::new(2.0, Color32::WHITE)));
    }
    response
}


/// A radio button drawn like the theme's check box, with its text and an optional hint below.
/// True when clicked.
pub fn radio(ui: &mut egui::Ui, on: bool, text: &str, hint: Option<&str>) -> bool {
    let response = ui
        .horizontal(|ui| {
            let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::click());
            let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.1);
            ui.painter().circle_filled(rect.center(), 8.0, if on { theme::ACCENT } else { lerp(theme::CARD, theme::PANEL_HIGH, hover) });
            if on {
                ui.painter().circle_filled(rect.center(), 3.5, Color32::WHITE);
            } else {
                ui.painter().circle_stroke(rect.center(), 8.0, Stroke::new(1.0, theme::GRID));
            }
            ui.add_space(4.0);
            let label = ui.add(egui::Label::new(egui::RichText::new(text).size(13.0)).sense(egui::Sense::click()));
            if let Some(hint) = hint {
                ui.label(egui::RichText::new(hint).size(12.0).color(theme::MUTED));
            }
            response.clicked() || label.clicked()
        })
        .inner;
    response
}

/// A check box with its text; the text clicks it too. True when clicked.
pub fn check_row(ui: &mut egui::Ui, value: &mut bool, text: &str, enabled: bool) -> bool {
    ui.horizontal(|ui| {
        let r = check(ui, *value, enabled);
        ui.add_space(4.0);
        let color = if enabled { theme::TEXT } else { theme::MUTED };
        let label = ui.add(egui::Label::new(egui::RichText::new(text).size(13.0).color(color)).sense(if enabled { egui::Sense::click() } else { egui::Sense::hover() }));
        let clicked = enabled && (r.clicked() || label.clicked());
        if clicked {
            *value = !*value;
        }
        clicked
    })
    .inner
}
