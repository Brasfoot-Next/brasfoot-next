//! The tabs and buttons every screen uses, one look for the whole app. The awards ceremony is the
//! reference: the active tab in gold, the main action in the accent, the others as cards. Text is
//! always the same weight, so nothing changes width when a tab is picked or hovered.

use eframe::egui::{self, Color32, Stroke, vec2};

use crate::screens::modkit::GOLD;
use crate::theme;

/// What a button is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The main action of a window or page (Fechar, Continuar, Iniciar Jogo).
    Primary,
    /// Any other action.
    Secondary,
}

/// A button: `Primary` in the accent, `Secondary` as a card with a border.
pub fn button(ui: &mut egui::Ui, text: &str, kind: Kind, enabled: bool) -> egui::Response {
    button_sized(ui, text, kind, enabled, 120.0)
}

/// A button as wide as the space it is in (the options of a choice).
pub fn wide_button(ui: &mut egui::Ui, text: &str, kind: Kind, enabled: bool) -> egui::Response {
    let width = ui.available_width();
    button_sized(ui, text, kind, enabled, width)
}

/// The − and + of a number field, as small secondary buttons.
pub fn stepper(ui: &mut egui::Ui, glyph: &str, enabled: bool) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(glyph).font(theme::bold(15.0)).color(if enabled { theme::TEXT } else { theme::MUTED }))
        .fill(theme::CARD)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(8)
        .min_size(vec2(32.0, 32.0));
    let response = ui.add_enabled(enabled, button);
    if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
}

fn button_sized(ui: &mut egui::Ui, text: &str, kind: Kind, enabled: bool, width: f32) -> egui::Response {
    let primary = kind == Kind::Primary;
    let color = match (enabled, primary) {
        (false, _) => theme::MUTED,
        (true, true) => Color32::WHITE,
        (true, false) => theme::TEXT,
    };
    let button = egui::Button::new(egui::RichText::new(text).font(theme::bold(13.0)).color(color))
        .fill(if primary && enabled { theme::ACCENT } else { theme::CARD })
        .stroke(if primary && enabled { Stroke::NONE } else { Stroke::new(1.0, theme::BORDER) })
        .corner_radius(8)
        .min_size(vec2(width, 32.0));
    let response = ui.add_enabled(enabled, button);
    if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
}

/// A tab: gold when active. A disabled one explains why on hover (`not_ported`).
pub fn tab(ui: &mut egui::Ui, text: &str, active: bool, enabled: bool, not_ported: &str) -> egui::Response {
    tab_sized(ui, text, active, enabled, not_ported, 0.0)
}

fn tab_sized(ui: &mut egui::Ui, text: &str, active: bool, enabled: bool, not_ported: &str, width: f32) -> egui::Response {
    let (fill, color) = if active { (GOLD, theme::HEADER) } else { (theme::CARD, if enabled { theme::TEXT } else { theme::MUTED }) };
    let button = egui::Button::new(egui::RichText::new(text).font(theme::bold(12.0)).color(color)).fill(fill).corner_radius(6).min_size(vec2(width, 28.0));
    let response = ui.add_enabled(enabled, button);
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else if not_ported.is_empty() {
        response
    } else {
        response.on_disabled_hover_text(not_ported)
    }
}

/// A row of tabs; the index clicked, if any.
pub fn tabs(ui: &mut egui::Ui, names: &[String], active: usize) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, name) in names.iter().enumerate() {
            if tab(ui, name, i == active, true, "").clicked() && i != active {
                clicked = Some(i);
            }
        }
    });
    clicked
}

/// Every choice of `segmented` is as wide as the widest, so the row keeps its size.
fn segment_width(ui: &egui::Ui, choices: &[String]) -> f32 {
    let widest = choices.iter().map(|c| ui.painter().layout_no_wrap(c.clone(), theme::bold(12.0), Color32::WHITE).size().x).fold(0.0, f32::max);
    (widest + 20.0).max(96.0)
}

/// The width `segmented` takes with these choices.
pub fn segmented_width(ui: &egui::Ui, choices: &[String]) -> f32 {
    segment_width(ui, choices) * choices.len() as f32 + 4.0 * (choices.len() as f32 - 1.0)
}

/// Choices side by side, one on (an option of a form): tabs of equal width. The index clicked.
pub fn segmented(ui: &mut egui::Ui, choices: &[String], selected: usize) -> Option<usize> {
    let mut clicked = None;
    let width = segment_width(ui, choices);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, text) in choices.iter().enumerate() {
            if tab_sized(ui, text, i == selected, true, "", width).clicked() && i != selected {
                clicked = Some(i);
            }
        }
    });
    clicked
}
