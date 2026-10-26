//! The game's stadium (`a.cl`, "Estádio"): the capacity of each sector, the ticket prices
//! (manual or the suggested ones) and the expansion (`a.cq`, "Expansão do estádio") with its
//! quote and works (`simulation::human::stadium`). With the mod's `PATRIMONIO` on, the toolbar
//! opens the mod's window instead and this one is its "Tela Clássica do Jogo".

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::ClubId;
use brasfoot_model::world::World;
use brasfoot_simulation::human::stadium::{self as expansion, Quote};
use eframe::egui::{self, Stroke, vec2};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::form::Reply;
use crate::screens::{menu, modkit, widgets};
use crate::{format, motion, theme};

/// The spinners' highest price per sector (`cl.Cl`, `Ci`, `Cj`, `Ck`).
const MAX_PRICE: [i64; 4] = [200, 300, 500, 1000];
const SECTORS: [&str; 4] = ["stadium-sector-general", "stadium-sector-stand", "stadium-sector-chair", "stadium-sector-box"];

pub struct StadiumState {
    prices: [i64; 4],
    auto: bool,
    /// `a.cq` while it is open.
    expansion: Option<ExpansionState>,
}

struct ExpansionState {
    add: [i64; 4],
    quote: Option<Quote>,
    question: Option<crate::screens::form::Question>,
}

impl StadiumState {
    /// `cl.oA()`: the check box and the spinners as saved, the suggested prices when automatic.
    pub fn new(world: &World, club: ClubId) -> Self {
        let mut state = StadiumState { prices: [0; 4], auto: true, expansion: None };
        if let Some(stadium) = world.club(club).stadium {
            let st = world.stadium(stadium);
            state.auto = st.automatic_prices;
            let shown = if st.automatic_prices { suggested(world, club) } else { std::array::from_fn(|i| st.ticket_prices.get(i).copied().unwrap_or(0)) };
            state.prices = std::array::from_fn(|i| (shown[i] as i64).clamp(1, MAX_PRICE[i]));
        }
        state
    }
}

/// `best.v.a(eg.tR, club)`: the main screen's competition kind is the league's (1).
fn suggested(world: &World, club: ClubId) -> [i32; 4] {
    brasfoot_engine::stadium::default_prices(world, 1, club)
}

pub fn window(s: &mut CareerScreen, state: &mut StadiumState, club: ClubId, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let was_open = *open;
    motion::dialog(ctx, "stadium", open, vec2(640.0, 340.0), frame, |ui, open| {
        let world = &s.career.world;
        let Some(stadium) = world.club(club).stadium else { return };
        let st = world.stadium(stadium);
        let capacity: i32 = st.capacity.iter().sum();
        let title = t.text("stadium-title", Some(&args([("name", st.name.clone()), ("seats", format::thousands(t, capacity as i64))])));
        if widgets::dialog_title(ui, files, Some(glyphs::STADIUM), &title) {
            *open = false;
        }
        let work = world.game.stadium_works.iter().find(|w| w.stadium == Some(stadium)).map(|w| w.until);
        let sugg = suggested(world, club);
        let mut toggled = false;
        ui.columns(2, |cols| {
            menu::panel().show(&mut cols[0], |ui| {
                ui.set_width(ui.available_width());
                menu::panel_title(ui, &t.text("stadium-current-capacity", None), |_| {});
                egui::Grid::new("stadium-capacity").num_columns(2).spacing(vec2(16.0, 8.0)).show(ui, |ui| {
                    for (i, key) in SECTORS.iter().enumerate() {
                        ui.label(egui::RichText::new(t.text(key, None)).font(theme::regular(13.0)).color(theme::MUTED));
                        ui.label(egui::RichText::new(format::thousands(t, st.capacity.get(i).copied().unwrap_or(0) as i64)).font(theme::bold(14.0)));
                        ui.end_row();
                    }
                });
            });
            menu::panel().show(&mut cols[1], |ui| {
                ui.set_width(ui.available_width());
                menu::panel_title(ui, &t.text("stadium-ticket-prices", None), |ui| {
                    // cl.oD(): the suggested prices into the spinners, not saved
                    if widgets::icon_button(ui, glyphs::FOR_SALE, 14.0, theme::ACCENT, !state.auto).on_hover_text(t.text("stadium-apply-suggested", None)).clicked() {
                        state.prices = std::array::from_fn(|i| sugg[i] as i64);
                    }
                });
                egui::Grid::new("stadium-prices").num_columns(2).spacing(vec2(16.0, 6.0)).show(ui, |ui| {
                    for (i, key) in SECTORS.iter().enumerate() {
                        ui.label(egui::RichText::new(t.text(key, None)).font(theme::regular(13.0)).color(theme::MUTED));
                        modkit::spinner(ui, &mut state.prices[i], 1, MAX_PRICE[i], 1, !state.auto);
                        ui.end_row();
                    }
                });
                ui.add_space(6.0);
                let line = t.text("stadium-suggested", Some(&args([("general", sugg[0]), ("stand", sugg[1]), ("chair", sugg[2]), ("box", sugg[3])])));
                ui.label(egui::RichText::new(format!("{} {line}", t.text("stadium-suggested-prices", None))).font(theme::regular(12.0)).color(theme::MUTED));
                ui.add_space(4.0);
                toggled = menu::check_row(ui, &mut state.auto, &t.text("stadium-check-use-suggested", None), true);
            });
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if let Some(until) = work {
                ui.label(egui::RichText::new(t.text("stadium-expansion-ends", Some(&args([("date", format::date_medium(t, &s.career.runtime.clock, until))])))).font(theme::bold(13.0)).color(theme::GOOD));
            } else if menu::action_button(ui, &t.text("stadium-button-expand", None), true, true).clicked() {
                state.expansion = Some(ExpansionState { add: [0; 4], quote: None, question: None });
            }
        });
        if toggled {
            // cl.oC(): the stadium switches at once; the spinners show the suggested or the saved prices
            let st = s.career.world.stadium_mut(stadium);
            st.automatic_prices = state.auto;
            let shown = if state.auto { sugg } else { std::array::from_fn(|i| st.ticket_prices.get(i).copied().unwrap_or(0)) };
            state.prices = std::array::from_fn(|i| (shown[i] as i64).clamp(1, MAX_PRICE[i]));
        }
    });
    // cl.oF() on "X": the manual prices are saved
    if was_open && !*open && !state.auto
        && let Some(stadium) = s.career.world.club(club).stadium
    {
        s.career.world.stadium_mut(stadium).ticket_prices = state.prices.iter().map(|&p| p as i32).collect();
    }
    if let Some(mut e) = state.expansion.take() {
        let mut open = true;
        expansion_window(s, &mut e, club, ctx, t, files, &mut open);
        if open {
            state.expansion = Some(e);
        }
    }
}

/// `a.cq`: the seats per sector, the quote and the works.
fn expansion_window(s: &mut CareerScreen, e: &mut ExpansionState, club: ClubId, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let mut start = false;
    motion::dialog(ctx, "stadium-expansion", open, vec2(520.0, 380.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::STADIUM), &t.text("stadium-expansion-title", None)) {
            *open = false;
        }
        let world = &s.career.world;
        let Some(stadium) = world.club(club).stadium else { return };
        let current = world.stadium(stadium).capacity.clone();
        let ceilings = expansion::ceilings(world);
        menu::field_label(ui, &t.text("stadium-expansion-prompt", None));
        ui.add_space(6.0);
        let mut changed = false;
        egui::Grid::new("stadium-expansion-grid").num_columns(3).spacing(vec2(14.0, 8.0)).show(ui, |ui| {
            for (i, key) in SECTORS.iter().enumerate() {
                let now = current.get(i).copied().unwrap_or(0);
                let room = (ceilings[i] - now) as i64;
                ui.label(egui::RichText::new(t.text("stadium-expansion-sector-current", Some(&args([("sector", t.text(key, None)), ("current", now.to_string())])))).font(theme::regular(13.0)));
                changed |= modkit::spinner(ui, &mut e.add[i], 0, room.max(0), 1, room > 0);
                ui.label(egui::RichText::new(t.text("stadium-expansion-max-more", Some(&args([("max", room.to_string())])))).font(theme::regular(12.0)).color(theme::MUTED));
                ui.end_row();
            }
        });
        if changed {
            e.quote = None;
        }
        ui.add_space(10.0);
        if let Some(q) = &e.quote {
            let date = format::date_medium(t, &s.career.runtime.clock, q.until);
            ui.label(egui::RichText::new(t.text("stadium-expansion-quote-cost", Some(&args([("cost", format::money(t, q.cost as i64))])))).font(theme::bold(13.0)).color(theme::GOOD));
            ui.label(egui::RichText::new(t.text("stadium-expansion-quote-end", Some(&args([("date", date)])))).font(theme::bold(13.0)).color(theme::GOOD));
        }
        let rest = ui.available_height() - 36.0;
        if rest > 0.0 {
            ui.add_space(rest);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // "Iniciar construção" only after a quote
            if e.quote.is_some() && menu::action_button(ui, &t.text("stadium-expansion-button-start", None), true, true).clicked() {
                start = true;
            }
            ui.add_space(4.0);
            if menu::action_button(ui, &t.text("stadium-expansion-button-quote", None), true, false).clicked() {
                let add = e.add.map(|v| v as i32);
                e.quote = expansion::quote(&s.career.world, &s.career.runtime.clock, &current, add);
            }
        });
    });
    if start && let Some(q) = &e.quote {
        if expansion::start(&mut s.career.world, club, q) {
            *open = false;
        } else {
            e.question = Some(modkit::message(t.text("stadium-expansion-no-money-title", None), t.text("stadium-expansion-no-money", None)));
        }
    }
    let _: Option<Reply> = modkit::question_overlay(ctx, t, "stadium-expansion", &mut e.question);
}
