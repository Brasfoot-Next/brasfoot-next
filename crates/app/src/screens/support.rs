//! The support and feedback screen: guidance on reporting bugs and suggesting features,
//! with direct links to the GitHub issues pages.

use brasfoot_i18n::Translator;
use eframe::egui::{self, Color32, RichText};

use crate::Navigation;
use crate::screens::controls;
use crate::screens::menu;
use crate::theme;

const ISSUES_URL: &str = "https://github.com/Brasfoot-Next/brasfoot-next/issues";
const DATABASE_ISSUES_URL: &str = "https://github.com/Brasfoot-Next/database/issues";

#[derive(Default)]
pub struct SupportScreen;

impl SupportScreen {
    pub fn new() -> Self {
        SupportScreen
    }

    pub fn show(&mut self, ui: &mut egui::Ui, t: &Translator) -> Navigation {
        let mut navigation = Navigation::Stay;
        menu::page(ui, 680.0, |ui| {
            if menu::header(ui, t, &t.text("support-title", None), &t.text("support-subtitle", None), true) {
                navigation = Navigation::Opening;
            }

            ui.add_space(4.0);
            ui.label(
                RichText::new(t.text("support-intro", None))
                    .font(theme::regular(14.0))
                    .color(theme::TEXT),
            );

            menu::section(ui, &t.text("support-section-types", None));

            self.card(
                ui,
                "🐛",
                &t.text("support-type-bug-title", None),
                &t.text("support-type-bug-desc", None),
                theme::BAD,
            );
            ui.add_space(8.0);

            self.card(
                ui,
                "💡",
                &t.text("support-type-feature-title", None),
                &t.text("support-type-feature-desc", None),
                theme::ACCENT,
            );
            ui.add_space(8.0);

            self.card(
                ui,
                "📊",
                &t.text("support-type-data-title", None),
                &t.text("support-type-data-desc", None),
                Color32::from_rgb(0x10, 0xb9, 0x81),
            );

            menu::section(ui, &t.text("support-section-guidelines", None));

            menu::panel().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 10.0;

                self.guide_item(ui, "1", &t.text("support-guide-1", None));
                self.guide_item(ui, "2", &t.text("support-guide-2", None));
                self.guide_item(ui, "3", &t.text("support-guide-3", None));
                self.guide_item(ui, "4", &t.text("support-guide-4", None));
            });

            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

                let btn_main = controls::button(
                    ui,
                    &t.text("support-open-issues", None),
                    controls::Kind::Primary,
                    true,
                );
                if btn_main.clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(ISSUES_URL));
                }

                let btn_data = controls::button(
                    ui,
                    &t.text("support-open-database-issues", None),
                    controls::Kind::Secondary,
                    true,
                );
                if btn_data.clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(DATABASE_ISSUES_URL));
                }
            });

            ui.add_space(28.0);
        });

        navigation
    }

    fn card(&self, ui: &mut egui::Ui, icon: &str, title: &str, desc: &str, accent: Color32) {
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).font(theme::bold(18.0)));
                ui.add_space(4.0);
                ui.label(RichText::new(title).font(theme::bold(15.0)).color(accent));
            });
            ui.add_space(4.0);
            ui.label(RichText::new(desc).font(theme::regular(13.0)).color(theme::MUTED));
        });
    }

    fn guide_item(&self, ui: &mut egui::Ui, step: &str, text: &str) {
        ui.horizontal_top(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 10.0, theme::ACCENT.gamma_multiply(0.25));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                step,
                theme::bold(11.0),
                theme::TEXT,
            );
            ui.add_space(8.0);
            ui.label(RichText::new(text).font(theme::regular(13.0)).color(theme::TEXT));
        });
    }
}
