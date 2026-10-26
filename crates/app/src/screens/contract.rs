//! The new contract (`a.fy`, "Novo contrato"): the salary of the spinner (from the current one,
//! 1 to 100,000,000 in steps of 1,000) and the duration of the combo, offered to the player
//! (`fy.rE()`, `brasfoot_simulation::human::contract`). A refusal asks "Salário recusado" with the
//! salary he wants; a salary at the limit says so ("Salário").

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::PlayerId;
use brasfoot_model::world::World;
use brasfoot_simulation::human::contract::{self, Answer};
use eframe::egui;

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::form::{self, Question, Reply};
use crate::screens::{menu, widgets};
use crate::{format, glyphs, motion, theme};

/// `fy.Bf`: "6 meses", "1 ano", "2 anos", "3 anos".
const DURATION_KEYS: [&str; 4] = [
    "contract-renewal-duration-6-months",
    "contract-renewal-duration-1-year",
    "contract-renewal-duration-2-years",
    "contract-renewal-duration-3-years",
];

/// `fy.IE`'s bounds and step.
const SALARY_BOUNDS: (i64, i64) = (1, 100_000_000);
const SALARY_STEP: i64 = 1000;

/// What the open question does on "Sim".
#[derive(Clone, Copy)]
enum Pending {
    /// "Salário recusado": signs the salary he wants.
    Refused(i32),
    /// "Salário": nothing.
    OverLimit,
}

pub struct ContractState {
    pub player: PlayerId,
    /// `fy.Bf.getSelectedIndex()`, 0 at first.
    duration: usize,
    salary: String,
    question: Option<(Question, Pending)>,
}

impl ContractState {
    pub fn new(world: &World, player: PlayerId) -> Self {
        ContractState { player, duration: 0, salary: world.player(player).salary.to_string(), question: None }
    }
}

pub fn window(s: &mut CareerScreen, state: &mut Option<ContractState>, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let Some(st) = state.as_mut() else {
        *open = false;
        return;
    };
    motion::dialog(ctx, "contract", open, egui::vec2(form::WIDTH, 410.0), form::frame(), |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::CONTRACT), &t.text("contract-renewal-title", None)) {
            *open = false;
        }
        let world = &s.career.world;
        form::player_card(ui, t, files, world, st.player);
        ui.add_space(12.0);
        if let Some((q, pending)) = st.question.clone() {
            match form::question(ui, t, &q) {
                Some(Reply::Yes) => {
                    if let Pending::Refused(asked) = pending {
                        contract::sign(&mut s.career.world, st.player, asked, st.duration);
                        *open = false;
                    }
                    st.question = None;
                }
                Some(_) => st.question = None,
                None => {}
            }
            return;
        }
        let x = world.player(st.player);
        let until = format::date(t, &s.career.runtime.clock, x.contract_until);
        form::line(ui, &t.text("contract-renewal-contract-until", Some(&args([("date", until)]))));
        form::line(ui, &t.text("contract-renewal-current-salary", Some(&args([("amount", format::money(t, x.salary as i64))]))));
        ui.add_space(12.0);
        menu::field_label(ui, &t.text("contract-renewal-label-new-salary", None));
        form::money_field(ui, "contract-salary", &mut st.salary, Some(SALARY_STEP), SALARY_BOUNDS);
        let salary = st.salary.trim().parse::<i64>().ok().filter(|v| (SALARY_BOUNDS.0..=SALARY_BOUNDS.1).contains(v));
        if let Some(v) = salary {
            ui.label(egui::RichText::new(format::money(t, v)).font(theme::regular(12.0)).color(theme::MUTED));
        }
        ui.add_space(10.0);
        menu::field_label(ui, &t.text("contract-renewal-label-duration", None));
        egui::ComboBox::from_id_salt("contract-duration").width(ui.available_width() - 8.0).selected_text(t.text(DURATION_KEYS[st.duration], None)).show_ui(ui, |ui| {
            for (i, key) in DURATION_KEYS.iter().enumerate() {
                ui.selectable_value(&mut st.duration, i, t.text(key, None));
            }
        });
        let (cancel, offer) = form::footer(ui, &t.text("contract-renewal-button-cancel", None), &t.text("contract-renewal-button-offer", None));
        if cancel {
            *open = false;
        }
        // the spinner never holds a value outside its bounds
        if offer && let Some(v) = salary {
            match contract::offer(&mut s.career.world, st.player, v as i32, st.duration) {
                Answer::Signed => *open = false,
                Answer::Asks(asked) => {
                    let text = t.text("contract-renewal-refused-message", Some(&args([("amount", format::money(t, asked as i64))])));
                    st.question = Some((Question { title: t.text("contract-renewal-refused-title", None), text, yes_no: true }, Pending::Refused(asked)));
                }
                Answer::OverLimit(limit) => {
                    let text = t.text("contract-renewal-limit-message", Some(&args([("amount", format::money(t, limit as i64))])));
                    st.question = Some((Question { title: t.text("contract-renewal-limit-title", None), text, yes_no: false }, Pending::OverLimit));
                }
                Answer::Nothing => {}
            }
        }
    });
}
