//! The preferences (`a.fB`, "Opções"): what the game saves with the career and the round screen
//! follows (speeds, the substitutions window at half time, the shoot-outs without humans), and
//! which leagues and competitions without human clubs it shows. The window theme, the
//! scoreboard color and the list colors of the original are left out: the app has one theme.

use brasfoot_i18n::Translator;
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::menu;
use crate::screens::new_game::country_name;
use crate::screens::widgets;
use crate::{motion, theme};

/// `fB.II`: the items of the combos.
const AUTOSAVE: [&str; 4] = ["preferences-autosave-none", "preferences-autosave-3-games", "preferences-autosave-each-game", "preferences-autosave-year-start"];
const SPEEDS: [&str; 6] = [
    "preferences-speed-slow",
    "preferences-speed-normal",
    "preferences-speed-fast",
    "preferences-speed-very-fast",
    "preferences-speed-super-fast",
    "preferences-speed-ultra-fast",
];
const CONTRACT_WARNING: [&str; 3] = ["preferences-option-no", "preferences-contract-warning-after", "preferences-contract-warning-month"];
/// `fB.fy(int)`: the competitions of the list "sem times humanos", in `verJint` order.
const COMPETITIONS: [&str; 21] = [
    "preferences-comp-world-cup",
    "preferences-comp-national-teams-cup",
    "preferences-comp-qualifiers",
    "preferences-comp-nations-league",
    "preferences-comp-finalissima",
    "preferences-comp-champions-europe",
    "preferences-comp-libertadores",
    "preferences-comp-champions-africa",
    "preferences-comp-champions-asia",
    "preferences-comp-champions-concacaf",
    "preferences-comp-champions-oceania",
    "preferences-comp-europa-league",
    "preferences-comp-sudamericana",
    "preferences-comp-conference-league",
    "preferences-comp-club-world-cup",
    "preferences-comp-recopa",
    "preferences-comp-national-cup",
    "preferences-comp-state",
    "preferences-comp-regional",
    "preferences-comp-super-cup",
    "preferences-comp-friendly",
];

pub fn window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "preferences", open, egui::vec2(1000.0, 680.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::OPTIONS), &t.text("preferences-title", None)) {
            *open = false;
        }
        let world = &mut s.career.world;
        ui.columns(2, |cols| {
            game_options(&mut cols[0], t, world);
            shown_options(&mut cols[1], t, files, world);
        });
    });
}

fn combo(ui: &mut egui::Ui, id: &str, t: &Translator, keys: &[&str], value: &mut i32) {
    let current = (*value).clamp(0, keys.len() as i32 - 1) as usize;
    egui::ComboBox::from_id_salt(id).width(ui.available_width() - 8.0).selected_text(t.text(keys[current], None)).show_ui(ui, |ui| {
        for (i, key) in keys.iter().enumerate() {
            if ui.selectable_label(current == i, t.text(key, None)).clicked() {
                *value = i as i32;
            }
        }
    });
}

fn yes_no(ui: &mut egui::Ui, t: &Translator, value: &mut i32) {
    let choices = [t.text("preferences-option-no", None), t.text("preferences-option-yes", None)];
    if let Some(i) = menu::segmented(ui, &choices, if *value == 1 { 1 } else { 0 }) {
        *value = i as i32;
    }
}

fn toggle(ui: &mut egui::Ui, text: &str, value: &mut bool) {
    ui.horizontal(|ui| {
        let r = menu::check(ui, *value, true);
        ui.add_space(4.0);
        let label = ui.add(egui::Label::new(egui::RichText::new(text).size(13.0)).sense(egui::Sense::click()));
        if r.clicked() || label.clicked() {
            *value = !*value;
        }
    });
}

/// The left panel: every combo and check box writes at once (`c.a.vM().set...`).
fn game_options(ui: &mut egui::Ui, t: &Translator, world: &mut World) {
    let g = &mut world.game;
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        menu::panel_title(ui, &t.text("preferences-section-game", None), |_| {});
        menu::field_label(ui, &t.text("preferences-label-autosave", None));
        combo(ui, "pref-autosave", t, &AUTOSAVE, &mut g.autosave);
        ui.add_space(8.0);
        menu::field_label(ui, &t.text("preferences-label-speed", None));
        combo(ui, "pref-speed", t, &SPEEDS, &mut g.speed);
        ui.add_space(8.0);
        menu::field_label(ui, &t.text("preferences-label-speed-no-humans", None));
        combo(ui, "pref-speed-nh", t, &SPEEDS, &mut g.speed_non_human);
        ui.add_space(8.0);
        menu::field_label(ui, &t.text("preferences-label-halftime-subs", None));
        yes_no(ui, t, &mut g.show_substitutions_window);
        ui.add_space(8.0);
        menu::field_label(ui, &t.text("preferences-label-penalties", None));
        yes_no(ui, t, &mut g.show_ai_penalties);
        ui.add_space(8.0);
        menu::field_label(ui, &t.text("preferences-label-coach-changes", None));
        yes_no(ui, t, &mut g.show_coach_changes);
        ui.add_space(8.0);
        menu::field_label(ui, &t.text("preferences-label-contract-warning", None));
        combo(ui, "pref-contract", t, &CONTRACT_WARNING, &mut g.contract_end_warning);
        ui.add_space(10.0);
        toggle(ui, &t.text("preferences-check-auctions", None), &mut g.show_auctions);
        toggle(ui, &t.text("preferences-check-auto-renew", None), &mut g.auto_renew_contracts);
        toggle(ui, &t.text("preferences-check-sounds", None), &mut g.use_sounds);
        toggle(ui, &t.text("preferences-check-bold-home", None), &mut g.bold_home);
    });
}

/// The right panel: the leagues watched without human clubs (`Z.B(boolean)`), the rounds and
/// state leagues shown, and the competitions played on screen or as results only (`verJint`).
fn shown_options(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &mut World) {
    menu::panel().show(ui, |ui| {
        ui.set_width(ui.available_width());
        menu::panel_title(ui, &t.text("preferences-label-watch-leagues", None), |_| {});
        egui::ScrollArea::vertical().id_salt("pref-leagues").max_height(150.0).auto_shrink([false, true]).show(ui, |ui| {
            for i in 0..world.game.league_countries.len() {
                let k = world.game.league_countries[i];
                let code = world.country(k).country;
                let mut on = world.country(k).flag_hc;
                ui.horizontal(|ui| {
                    if menu::check(ui, on, true).clicked() {
                        on = !on;
                    }
                    widgets::flag(ui, files, code, 16.0);
                    ui.label(egui::RichText::new(country_name(t, code)).size(13.0));
                });
                world.country_mut(k).flag_hc = on;
            }
        });
        ui.add_space(8.0);
        let g = &mut world.game;
        toggle(ui, &t.text("preferences-check-skip-leagues", None), &mut g.ignore_leagues);
        toggle(ui, &t.text("preferences-check-ignore-states", None), &mut g.ignore_state);
        toggle(ui, &t.text("preferences-check-group-states", None), &mut g.show_states_grouped);
        ui.add_space(10.0);
        menu::field_label(ui, &t.text("preferences-label-no-human-comps", None));
        if g.show_international.len() < COMPETITIONS.len() {
            g.show_international.resize(COMPETITIONS.len(), true);
        }
        let choices = [t.text("preferences-label-watch-matches", None), t.text("preferences-label-results-only", None)];
        egui::ScrollArea::vertical().id_salt("pref-competitions").max_height(230.0).auto_shrink([false, true]).show(ui, |ui| {
            for (i, key) in COMPETITIONS.iter().enumerate() {
                ui.horizontal(|ui| {
                    // the name in a fixed column, so the switches line up
                    let name_width = (ui.available_width() - menu::segmented_width(ui, &choices) - 12.0).max(80.0);
                    ui.allocate_ui_with_layout(egui::vec2(name_width, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_min_width(name_width);
                        ui.label(egui::RichText::new(t.text(key, None)).size(13.0));
                    });
                    // fB ">>" and "<<": setVerJint(false | true, i)
                    if let Some(c) = menu::segmented(ui, &choices, if g.show_international[i] { 0 } else { 1 }) {
                        g.show_international[i] = c == 0;
                    }
                });
            }
        });
    });
}
