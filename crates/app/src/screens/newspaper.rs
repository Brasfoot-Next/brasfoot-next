//! The newspaper screen (`bfmod.JornalDialog`, "Gazeta do Brasfoot"): headlines by
//! section on the left, the article on a sheet of newspaper print on the right.

use brasfoot_i18n::Translator;
use brasfoot_model::mods::Article;
use eframe::egui::{self, Color32, Stroke, vec2};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::modkit::{GOLD, GOLD_DARK};
use crate::screens::widgets;
use crate::{motion, theme};

const PAPER: Color32 = Color32::from_rgb(0xf4, 0xef, 0xe4);
const INK: Color32 = Color32::from_rgb(0x22, 0x20, 0x1c);
const INK_MUTED: Color32 = Color32::from_rgb(0x6b, 0x65, 0x5b);

pub struct NewspaperState {
    pub selected_section: usize,
    pub selected_article_id: Option<i32>,
}

impl Default for NewspaperState {
    fn default() -> Self {
        NewspaperState {
            selected_section: 0,
            selected_article_id: None,
        }
    }
}

const SECTIONS: [&str; 6] = [
    "newspaper-section-all",
    "newspaper-section-market",
    "newspaper-section-your-club",
    "newspaper-section-records",
    "newspaper-section-youth",
    "newspaper-section-world",
];

const SECTION_NAMES: [&str; 6] = [
    "Todas",
    "Mercado",
    "Seu clube",
    "Craques e recordes",
    "Base",
    "Mundo",
];

fn section_color(section: &str) -> Color32 {
    match section {
        "Mercado" => Color32::from_rgb(0x3b, 0x82, 0xf6),
        "Seu clube" => Color32::from_rgb(0xe8, 0xbe, 0x50),
        "Craques e recordes" => Color32::from_rgb(0xa7, 0x8b, 0xfa),
        "Base" => Color32::from_rgb(0x4c, 0xc3, 0x8a),
        _ => Color32::from_rgb(0x94, 0xa3, 0xb8),
    }
}

pub fn window(
    s: &mut CareerScreen,
    state: &mut NewspaperState,
    ctx: &egui::Context,
    t: &Translator,
    files: &GameFiles,
    open: &mut bool,
) {
    let frame = egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, GOLD_DARK))
        .inner_margin(16);

    motion::dialog(ctx, "newspaper-dialog", open, vec2(1000.0, 660.0), frame, |ui, open| {
        body(s, state, ui, t, files, open);
    });
}

fn body(
    s: &mut CareerScreen,
    state: &mut NewspaperState,
    ui: &mut egui::Ui,
    t: &Translator,
    files: &GameFiles,
    open: &mut bool,
) {
    if widgets::dialog_title_with(ui, files, Some(glyphs::NEWSPAPER), &t.text("newspaper-title", None), |ui| {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(t.text("newspaper-subtitle", None)).font(theme::regular(12.0)).color(theme::MUTED));
    }) {
        *open = false;
    }

    ui.add_space(10.0);

    // Section bar
    ui.horizontal(|ui| {
        for (i, &sec_key) in SECTIONS.iter().enumerate() {
            let active = state.selected_section == i;
            let label = t.text(sec_key, None);
            let btn = egui::Button::new(
                egui::RichText::new(&label)
                    .font(if active { theme::bold(13.0) } else { theme::regular(13.0) })
                    .color(if active { Color32::WHITE } else { theme::MUTED }),
            )
            .fill(if active { theme::PANEL_HIGH } else { Color32::TRANSPARENT })
            .stroke(if active { Stroke::new(1.0, GOLD) } else { Stroke::NONE })
            .corner_radius(4.0)
            .min_size(vec2(70.0, 26.0));

            if ui.add(btn).clicked() {
                state.selected_section = i;
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    let articles: Vec<Article> = s.career.world.mods.articles.clone();
    let current_filter = SECTION_NAMES[state.selected_section];

    let filtered: Vec<&Article> = articles
        .iter()
        .rev()
        .filter(|a| state.selected_section == 0 || a.section == current_filter)
        .collect();

    if state.selected_article_id.is_none() && !filtered.is_empty() {
        state.selected_article_id = Some(filtered[0].id);
    }

    // Mark active article as read
    if let Some(id) = state.selected_article_id {
        if let Some(pos) = s.career.world.mods.articles.iter().position(|a| a.id == id) {
            s.career.world.mods.articles[pos].read = true;
        }
    }

    let avail_height = ui.available_height() - 8.0;

    ui.horizontal(|ui| {
        // Left Column: Headlines
        ui.allocate_ui_with_layout(vec2(360.0, avail_height), egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("newspaper-headlines")
                .show(ui, |ui| {
                    if filtered.is_empty() {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(t.text("newspaper-empty", None)).color(theme::MUTED));
                        return;
                    }

                    for a in &filtered {
                        let selected = state.selected_article_id == Some(a.id);
                        let col = section_color(&a.section);
                        let bg = if selected {
                            theme::PANEL_HIGH
                        } else {
                            Color32::from_rgb(0x1a, 0x23, 0x33)
                        };

                        let item_frame = egui::Frame::new()
                            .fill(bg)
                            .stroke(if selected { Stroke::new(1.0, GOLD) } else { Stroke::new(1.0, theme::BORDER) })
                            .corner_radius(4.0)
                            .inner_margin(8.0);

                        let resp = item_frame.show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                // Section badge
                                ui.label(egui::RichText::new(&a.section).font(theme::bold(10.0)).color(col));
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(&a.date).font(theme::regular(10.0)).color(theme::MUTED));
                                if !a.read {
                                    ui.add_space(4.0);
                                    ui.label(egui::RichText::new("●").color(GOLD).font(theme::bold(10.0)));
                                }
                            });
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(&a.title)
                                    .font(if a.read { theme::regular(13.0) } else { theme::bold(13.0) })
                                    .color(if selected { Color32::WHITE } else { theme::TEXT }),
                            );
                        });

                        if resp.response.interact(egui::Sense::click()).clicked() {
                            state.selected_article_id = Some(a.id);
                        }

                        ui.add_space(4.0);
                    }
                });
        });

        ui.add_space(8.0);

        // Right Column: Paper sheet article view
        let paper_frame = egui::Frame::new()
            .fill(PAPER)
            .corner_radius(4.0)
            .stroke(Stroke::new(1.0, Color32::from_rgb(0xd8, 0xd0, 0xc0)))
            .inner_margin(24.0);

        paper_frame.show(ui, |ui| {
            ui.set_min_size(vec2(ui.available_width(), avail_height));
            egui::ScrollArea::vertical()
                .id_salt("newspaper-body")
                .show(ui, |ui| {
                    if let Some(id) = state.selected_article_id {
                        if let Some(a) = filtered.iter().find(|art| art.id == id) {
                            render_article(ui, a);
                        } else {
                            ui.label(egui::RichText::new(t.text("newspaper-no-selection", None)).color(INK_MUTED));
                        }
                    } else {
                        ui.label(egui::RichText::new(t.text("newspaper-no-selection", None)).color(INK_MUTED));
                    }
                });
        });
    });
}

fn render_article(ui: &mut egui::Ui, a: &Article) {
    let col = section_color(&a.section);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&a.section).font(theme::bold(12.0)).color(col));
        ui.label(egui::RichText::new("•").color(INK_MUTED));
        ui.label(egui::RichText::new(&a.date).font(theme::regular(12.0)).color(INK_MUTED));
    });

    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(&a.title)
            .font(theme::bold(20.0))
            .color(INK),
    );

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(10.0);

    // Simple HTML parsing for <b>, <br>, etc.
    let text = a.body.replace("<br><br>", "\n\n").replace("<br>", "\n");
    let parts: Vec<&str> = text.split("\n\n").collect();

    for paragraph in parts {
        let lines: Vec<&str> = paragraph.split('\n').collect();
        for line in lines {
            let clean = line.replace("<b>", "").replace("</b>", "");
            let is_bullet = clean.trim_start().starts_with('•');

            if line.contains("<b>") && !is_bullet {
                ui.label(
                    egui::RichText::new(clean.trim())
                        .font(theme::bold(14.0))
                        .color(INK),
                );
            } else if is_bullet {
                ui.label(
                    egui::RichText::new(clean.trim())
                        .font(theme::regular(13.0))
                        .color(INK),
                );
            } else {
                ui.label(
                    egui::RichText::new(clean.trim())
                        .font(theme::regular(13.5))
                        .color(INK),
                );
            }
        }
        ui.add_space(8.0);
    }
}
