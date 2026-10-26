//! What can be done with a player of the human's club: the squad's right-button menu
//! (`eg.HB`) and the sell window (`a.jc`, "Vender Jogador"). The auction (`components.an`),
//! the shirt number (`a.ce`) and the loans' cancel and buy are not ported yet.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::PlayerId;
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::{contract, menu, retire, widgets};
use crate::{format, motion, theme};

/// The entries of `eg.HB`.
#[derive(Clone, Copy, PartialEq)]
pub enum PlayerAction {
    /// "Propor contrato" (`qS`)
    Contract,
    /// "Vender Jogador" (`qX`)
    Sell,
    /// "Aposentar Jogador" (`qW`)
    Retire,
    /// "Histórico do jogador" (`qC`)
    History,
    /// "Pôr para empréstimo" / "Não pôr para empréstimo" (`qE`)
    ToggleLoan,
}

/// `qW` is only there for the players over 32.
pub fn can_retire(world: &World, p: PlayerId) -> bool {
    world.player(p).age > 32
}

/// `eg.HB`: the menu, a separator between the groups as the original has them. Returns the
/// entry clicked.
pub fn context_menu(ui: &mut egui::Ui, t: &Translator, world: &World, p: PlayerId) -> Option<PlayerAction> {
    ui.spacing_mut().item_spacing.y = 0.0;
    let not_yet = t.text("toolbar-not-ported", None);
    let x = world.player(p);
    let mut picked = None;
    let mut entry = |ui: &mut egui::Ui, key: &str, action: Option<PlayerAction>, enabled: bool| {
        if widgets::menu_entry(ui, &t.text(key, None), enabled && action.is_some(), &not_yet) {
            picked = action;
            ui.close();
        }
    };
    entry(ui, "player-menu-contract", Some(PlayerAction::Contract), true);
    widgets::menu_separator(ui);
    entry(ui, "player-menu-sell", Some(PlayerAction::Sell), !x.listed_for_sale);
    entry(ui, "player-menu-retire", Some(PlayerAction::Retire), can_retire(world, p));
    widgets::menu_separator(ui);
    entry(ui, "player-menu-number", None, false);
    widgets::menu_separator(ui);
    entry(ui, "player-menu-history", Some(PlayerAction::History), true);
    widgets::menu_separator(ui);
    // the loans the club took: not ported yet
    entry(ui, "player-menu-cancel-loan", None, x.on_loan);
    entry(ui, "player-menu-buy-loaned", None, x.on_loan);
    widgets::menu_separator(ui);
    let loan_key = if x.listed_for_loan { "player-menu-no-loan" } else { "player-menu-loan" };
    entry(ui, loan_key, Some(PlayerAction::ToggleLoan), !x.on_loan);
    picked
}

/// What an entry does.
pub fn apply(s: &mut CareerScreen, p: PlayerId, action: PlayerAction) {
    match action {
        PlayerAction::Contract => {
            s.contract_state = Some(contract::ContractState::new(&s.career.world, p));
            s.contract_open = true;
        }
        PlayerAction::Sell => s.sell = Some(SellState::new(&s.career.world, p)),
        PlayerAction::Retire => s.retire = Some(retire::RetireState::new(&s.career.world, p)),
        PlayerAction::History => s.open_player = Some(p),
        PlayerAction::ToggleLoan => {
            let x = s.career.world.player_mut(p);
            x.listed_for_loan = !x.listed_for_loan;
        }
    }
}

/// `a.jc`: the asked price in thousands (`uE`), the market value to start from.
pub struct SellState {
    pub player: PlayerId,
    price: String,
    error: bool,
}

impl SellState {
    pub fn new(world: &World, p: PlayerId) -> Self {
        SellState { player: p, price: (world.player(p).market_value / 1000).to_string(), error: false }
    }
}

/// `jc.Nl`: the release fee, by the days of contract left (`F.fR()`).
fn release_fee_percent(days: i32) -> i32 {
    match days {
        d if d <= 0 => 0,
        1..=30 => 12,
        31..=60 => 20,
        61..=90 => 22,
        91..=180 => 25,
        181..=722 => 30,
        _ => 35,
    }
}

pub fn sell_window(s: &mut CareerScreen, state: &mut SellState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let mut listed = None;
    motion::dialog(ctx, "sell-player", open, egui::vec2(440.0, 420.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::FOR_SALE), &t.text("sell-player-title", None)) {
            *open = false;
        }
        let world = &s.career.world;
        let x = world.player(state.player);
        ui.label(egui::RichText::new(&x.name).font(theme::bold(16.0)).color(theme::STAR));
        ui.add_space(10.0);
        // Km, Kn: the auction is not ported yet, the fixed price is
        let modes = [t.text("sell-player-radio-auction", None), t.text("sell-player-radio-fixed", None)];
        ui.add_enabled_ui(false, |ui| {
            ui.label(egui::RichText::new(&modes[0]).font(theme::regular(13.0))).on_disabled_hover_text(t.text("toolbar-not-ported", None));
        });
        ui.horizontal(|ui| {
            menu::check(ui, true, true);
            ui.label(egui::RichText::new(&modes[1]).font(theme::regular(13.0)));
        });
        ui.add_space(12.0);
        menu::field_label(ui, &t.text("sell-player-label-fixed-price", None));
        let edit = ui.add(egui::TextEdit::singleline(&mut state.price).desired_width(160.0).margin(egui::vec2(8.0, 6.0)));
        // jc.oK(): the price written out, or "valor inválido"
        let thousands: Option<i64> = state.price.trim().parse::<i64>().ok().filter(|v| *v >= 0);
        let shown = match thousands {
            Some(v) => format::money(t, v * 1000),
            None => t.text("sell-player-invalid-value", None),
        };
        ui.label(egui::RichText::new(shown).font(theme::bold(13.0)).color(if thousands.is_some() { theme::TEXT } else { theme::TEXT_RED }));
        let _ = edit;
        ui.add_space(12.0);
        let days = brasfoot_model::player::days_between(world.today_millis(), x.contract_until);
        ui.label(egui::RichText::new(t.text("sell-player-note-contract-fee", None)).font(theme::regular(12.0)).color(theme::MUTED));
        ui.label(egui::RichText::new(t.text("sell-player-note-estimated-fee", Some(&args([("percent", release_fee_percent(days))])))).font(theme::regular(12.0)).color(theme::MUTED));
        if state.error {
            ui.add_space(6.0);
            menu::error(ui, &t.text("sell-player-error-invalid-price", None));
        }
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if menu::action_button(ui, &t.text("sell-player-button-fix-price", None), true, true).clicked() {
                match thousands.and_then(|v| i32::try_from(v * 1000).ok()) {
                    Some(price) => {
                        listed = Some(price);
                        *open = false;
                    }
                    None => state.error = true,
                }
            }
            if menu::action_button(ui, &t.text("sell-player-button-cancel", None), true, false).clicked() {
                *open = false;
            }
        });
    });
    // jc.dD(): for sale (c(true)) at the asked price (ag(int))
    if let Some(price) = listed {
        let x = s.career.world.player_mut(state.player);
        x.listed_for_sale = true;
        x.price = price;
    }
}
