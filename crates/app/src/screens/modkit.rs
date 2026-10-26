//! The look the mod's own windows share (`bfmod.CerimoniaDialog`'s gold over navy): tables,
//! tabs, cards, buttons and the messages of their `JOptionPane`s, drawn inside the app.

use brasfoot_i18n::Translator;
use eframe::egui::{self, Color32, Stroke, pos2, vec2};

use crate::game_files::GameFiles;
use crate::screens::form::{Question, Reply};
use crate::screens::widgets;
use crate::theme;

/// `OURO`, `OURO_ESCURO`
pub const GOLD: Color32 = Color32::from_rgb(232, 190, 80);
pub const GOLD_DARK: Color32 = Color32::from_rgb(150, 115, 30);
/// `VERDE`, `AZUL_CLARO`, the mod's red (`0xe5484d`)
pub const GREEN: Color32 = Color32::from_rgb(90, 200, 120);
pub const LIGHT_BLUE: Color32 = Color32::from_rgb(110, 180, 245);
pub const RED: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);
/// `#b8c0cc`, the soft lines inside the cards
pub const SOFT: Color32 = Color32::from_rgb(0xb8, 0xc0, 0xcc);

/// A cell: its text and, for a name, the country's flag (`Bandeira`).
#[derive(Clone, Default)]
pub struct Cell {
    pub text: String,
    pub flag: Option<i32>,
    /// A color of its own (the mod's green and red numbers).
    pub color: Option<Color32>,
    pub bold: bool,
    /// Aligned to the right (`direita(tabela, colunas)`).
    pub right: bool,
}
impl Cell {
    pub fn colored(text: impl Into<String>, color: Color32) -> Self {
        Cell { text: text.into(), color: Some(color), ..Default::default() }
    }

    pub fn strong(text: impl Into<String>) -> Self {
        Cell { text: text.into(), bold: true, ..Default::default() }
    }

    /// The same cell aligned to the right.
    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }


    pub fn flagged(text: impl Into<String>, country: i32) -> Self {
        Cell { text: text.into(), flag: Some(country), ..Default::default() }
    }
}
impl<T: ToString> From<T> for Cell {
    fn from(v: T) -> Self {
        Cell { text: v.to_string(), ..Default::default() }
    }
}
/// `Renderizador`: numbers (`[×-]?[\d.,]*`) centered, the rest on the left.
pub fn is_number(text: &str) -> bool {
    let rest = text.strip_prefix('×').or_else(|| text.strip_prefix('-')).unwrap_or(text);
    rest.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',')
}
/// `tabela(model, destacarPrimeiro, larguras...)`: the original's widths spread over the space,
/// rows of 24 in alternating shades, the first row in gold when it is the winner.
pub fn table(ui: &mut egui::Ui, files: &GameFiles, id: &str, columns: &[(String, f32)], rows: &[Vec<Cell>], highlight_first: bool) {
    let width = ui.available_width();
    let total: f32 = columns.iter().map(|c| c.1).sum();
    let widths: Vec<f32> = columns.iter().map(|c| c.1 * width / total).collect();
    let (header, _) = ui.allocate_exact_size(vec2(width, 26.0), egui::Sense::hover());
    ui.painter().rect_filled(header, 6.0, theme::HEADER);
    let mut x = header.left();
    for (j, ((title, _), w)) in columns.iter().zip(&widths).enumerate() {
        let cell = egui::Rect::from_min_size(pos2(x, header.top()), vec2(*w, header.height()));
        // a column of numbers has its title centered too
        let numeric = rows.first().and_then(|r| r.get(j)).is_some_and(|c| c.flag.is_none() && !c.text.is_empty() && is_number(&c.text));
        let painter = ui.painter().with_clip_rect(cell);
        if numeric {
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, title, theme::bold(12.0), GOLD);
        } else {
            painter.text(pos2(cell.left() + 6.0, cell.center().y), egui::Align2::LEFT_CENTER, title, theme::bold(12.0), GOLD);
        }
        x += w;
    }
    ui.add_space(2.0);
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]).show_rows(ui, 24.0, rows.len(), |ui, range| {
        for i in range {
            let row = &rows[i];
            let (rect, response) = ui.allocate_exact_size(vec2(width, 24.0), egui::Sense::hover());
            let base = if i % 2 == 1 { theme::ROW_ALT } else { theme::PANEL };
            ui.painter().rect_filled(rect, 4.0, if response.hovered() { theme::HOVER } else { base });
            let winner = highlight_first && i == 0;
            let mut x = rect.left();
            for (cell, w) in row.iter().zip(&widths) {
                let area = egui::Rect::from_min_size(pos2(x, rect.top()), vec2(*w, rect.height()));
                x += w;
                let mut left = area.left() + 6.0;
                if let Some(country) = cell.flag {
                    let r = egui::Rect::from_center_size(pos2(left + 9.0, area.center().y), vec2(18.0, 18.0));
                    widgets::paint_jar_image(ui, files, &format!("aflags/{country}.png"), r);
                    left += 24.0;
                }
                let font = if winner || cell.bold { theme::bold(13.0) } else { theme::regular(13.0) };
                let color = cell.color.unwrap_or(if winner { GOLD } else { theme::TEXT });
                // a text wider than its column ends in an ellipsis, as Swing's labels do
                let mut job = egui::text::LayoutJob::simple_singleline(cell.text.clone(), font, color);
                job.wrap = egui::text::TextWrapping::truncate_at_width((area.right() - left - 4.0).max(8.0));
                let galley = ui.painter().layout_job(job);
                let at = if cell.flag.is_none() && is_number(&cell.text) {
                    pos2(area.center().x - galley.size().x / 2.0, area.center().y - galley.size().y / 2.0)
                } else {
                    pos2(left, area.center().y - galley.size().y / 2.0)
                };
                ui.painter().galley(at, galley, color);
            }
        }
    });
}

/// A table's column titles from their keys and the original's widths.
pub fn columns(t: &Translator, specs: &[(&str, f32)]) -> Vec<(String, f32)> {
    specs.iter().map(|(key, w)| (t.text(key, None), *w)).collect()
}

/// `card()`: a panel with the border of the mod's cards.
pub fn card() -> egui::Frame {
    egui::Frame::new().fill(theme::CARD).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(8).inner_margin(egui::Margin::symmetric(12, 10))
}

/// An item inside a card (`FUNDO_TABELA` with a thin border); `border` lights it.
pub fn item(border: Option<Color32>, fill: Option<Color32>) -> egui::Frame {
    egui::Frame::new()
        .fill(fill.unwrap_or(theme::PANEL))
        .stroke(Stroke::new(1.0, border.unwrap_or(theme::BORDER)))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 7))
}

/// A card's title in gold.
pub fn title(ui: &mut egui::Ui, text: &str, size: f32) {
    ui.add(egui::Label::new(egui::RichText::new(text).font(theme::bold(size)).color(GOLD)).wrap());
}

/// A line of text that wraps.
pub fn text(ui: &mut egui::Ui, text: &str, size: f32, color: Color32) {
    ui.add(egui::Label::new(egui::RichText::new(text).font(theme::regular(size)).color(color)).wrap());
}

/// A line in bold that wraps.
pub fn strong(ui: &mut egui::Ui, text: &str, size: f32, color: Color32) {
    ui.add(egui::Label::new(egui::RichText::new(text).font(theme::bold(size)).color(color)).wrap());
}



/// A number field with `-` and `+` (`JSpinner` with a `SpinnerNumberModel`). True when changed.
pub fn spinner(ui: &mut egui::Ui, value: &mut i64, min: i64, max: i64, step: i64, enabled: bool) -> bool {
    let before = *value;
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if crate::screens::controls::stepper(ui, "−", enabled).clicked() {
                *value = (*value - step).max(min);
            }
            ui.add(egui::DragValue::new(value).range(min..=max).speed(step as f64 / 10.0).min_decimals(0));
            if crate::screens::controls::stepper(ui, "+", enabled).clicked() {
                *value = (*value + step).min(max);
            }
        });
    });
    *value = (*value).clamp(min, max);
    *value != before
}

/// A `JOptionPane` of a mod window over it: a card with the title, the text and its buttons.
/// The reply once a button is clicked; the question is gone then.
pub fn question_overlay(ctx: &egui::Context, t: &Translator, id: &str, question: &mut Option<Question>) -> Option<Reply> {
    let q = question.as_ref()?;
    let mut reply = None;
    let screen = ctx.content_rect();
    // a click outside answers as Esc does
    let outside = egui::Area::new(egui::Id::unique((id, "question-backdrop"))).order(egui::Order::Tooltip).fixed_pos(screen.min).interactable(true).show(ctx, |ui| {
        let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
        ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(90));
        response.clicked()
    });
    egui::Area::new(egui::Id::unique((id, "question"))).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16).show(ui, |ui| {
            ui.set_width(440.0);
            ui.set_min_height(150.0);
            reply = crate::screens::form::question(ui, t, q);
        });
    });
    if reply.is_none() && (outside.inner || ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
        reply = Some(if q.yes_no { Reply::No } else { Reply::Ok });
    }
    if reply.is_some() {
        *question = None;
    }
    reply
}

/// A message (`showMessageDialog`) to show over a mod window.
pub fn message(title: String, text: String) -> Question {
    Question { title, text, yes_no: false }
}

/// A confirmation (`showConfirmDialog` with Sim and Não).
pub fn confirm(title: String, text: String) -> Question {
    Question { title, text, yes_no: true }
}

/// `table` whose rows are picked by a click (`JTable` with a single selection); the selected row
/// is lit. `height` bounds it; the rows scroll inside.
pub fn select_table(ui: &mut egui::Ui, files: &GameFiles, id: &str, columns: &[(String, f32)], rows: &[Vec<Cell>], selected: &mut Option<usize>, height: f32) {
    let width = ui.available_width();
    let total: f32 = columns.iter().map(|c| c.1).sum();
    let widths: Vec<f32> = columns.iter().map(|c| c.1 * width / total).collect();
    let (header, _) = ui.allocate_exact_size(vec2(width, 24.0), egui::Sense::hover());
    ui.painter().rect_filled(header, 6.0, theme::HEADER);
    let mut x = header.left();
    for (title, w) in columns.iter().map(|c| &c.0).zip(&widths) {
        let cell = egui::Rect::from_min_size(pos2(x, header.top()), vec2(*w, header.height()));
        ui.painter().with_clip_rect(cell).text(pos2(cell.left() + 6.0, cell.center().y), egui::Align2::LEFT_CENTER, title, theme::bold(11.0), GOLD);
        x += w;
    }
    if selected.is_some_and(|i| i >= rows.len()) {
        *selected = None;
    }
    ui.allocate_ui(vec2(width, (height - 24.0).max(40.0)), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]).show_rows(ui, 22.0, rows.len(), |ui, range| {
            for i in range {
                let row = &rows[i];
                let (rect, response) = ui.allocate_exact_size(vec2(width, 22.0), egui::Sense::click());
                let base = if i % 2 == 1 { theme::ROW_ALT } else { theme::PANEL };
                let fill = if *selected == Some(i) {
                    theme::SELECTION
                } else if response.hovered() {
                    theme::HOVER
                } else {
                    base
                };
                ui.painter().rect_filled(rect, 3.0, fill);
                if response.clicked() {
                    *selected = Some(i);
                }
                let mut x = rect.left();
                for (cell, w) in row.iter().zip(&widths) {
                    let area = egui::Rect::from_min_size(pos2(x, rect.top()), vec2(*w, rect.height()));
                    x += w;
                    let mut left = area.left() + 6.0;
                    if let Some(country) = cell.flag {
                        let r = egui::Rect::from_center_size(pos2(left + 8.0, area.center().y), vec2(16.0, 16.0));
                        widgets::paint_jar_image(ui, files, &format!("aflags/{country}.png"), r);
                        left += 21.0;
                    }
                    let font = if cell.bold { theme::bold(12.0) } else { theme::regular(12.0) };
                    let color = cell.color.unwrap_or(theme::TEXT);
                    let mut job = egui::text::LayoutJob::simple_singleline(cell.text.clone(), font, color);
                    job.wrap = egui::text::TextWrapping::truncate_at_width((area.right() - left - 4.0).max(8.0));
                    let galley = ui.painter().layout_job(job);
                    let x = if cell.right { area.right() - 6.0 - galley.size().x } else { left };
                    ui.painter().galley(pos2(x, area.center().y - galley.size().y / 2.0), galley, color);
                }
            }
        });
    });
}

/// A choice among options (`showInputDialog` with a list, `showOptionDialog`).
#[derive(Clone, Debug)]
pub struct Choice {
    pub title: String,
    pub text: String,
    pub options: Vec<String>,
}

/// The choice over a mod window: one button per option and "Cancelar". The option picked, once
/// clicked; the choice is gone then (also on cancel).
pub fn choice_overlay(ctx: &egui::Context, t: &Translator, id: &str, choice: &mut Option<Choice>) -> Option<usize> {
    let c = choice.as_ref()?;
    let mut picked = None;
    let mut cancel = false;
    let screen = ctx.content_rect();
    // a click outside cancels, as Esc does
    let outside = egui::Area::new(egui::Id::unique((id, "choice-backdrop"))).order(egui::Order::Tooltip).fixed_pos(screen.min).interactable(true).show(ctx, |ui| {
        let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
        ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(90));
        response.clicked()
    });
    egui::Area::new(egui::Id::unique((id, "choice"))).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16).show(ui, |ui| {
            ui.set_width(480.0);
            ui.label(egui::RichText::new(&c.title).font(theme::bold(14.0)).color(theme::ACCENT));
            ui.add_space(6.0);
            for line in c.text.lines() {
                ui.add(egui::Label::new(egui::RichText::new(line).font(theme::regular(13.0))).wrap());
            }
            ui.add_space(10.0);
            for (i, option) in c.options.iter().enumerate() {
                if crate::screens::controls::wide_button(ui, option, crate::screens::controls::Kind::Secondary, true).clicked() {
                    picked = Some(i);
                }
                ui.add_space(4.0);
            }
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::screens::controls::button(ui, &t.text("dialog-cancel", None), crate::screens::controls::Kind::Secondary, true).clicked() {
                    cancel = true;
                }
            });
        });
    });
    if outside.inner || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        cancel = true;
    }
    if picked.is_some() || cancel {
        *choice = None;
    }
    picked
}
