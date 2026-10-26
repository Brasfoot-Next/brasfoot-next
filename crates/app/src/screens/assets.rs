//! The stadium and the club's assets (`bfmod.PatrimonioDialog`, "Estádio e Patrimônio"), the
//! mod's screen behind the toolbar's "Estádio" when `PATRIMONIO` is on: the stadium's sectors and
//! their expansion, the ticket prices, the four pillars (arena, training centre, academy,
//! marketing) and their levels, the arena's naming rights, the season's campaign and the ranking
//! of stadiums and structures. Every action is `Patrimonio`'s (`systems::assets`).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::{ClubAssets, Pillar};
use brasfoot_model::world::World;
use brasfoot_systems::assets::{self as sys, AssetsError, CAMPAIGN_COST, CAMPAIGN_MARKETING, LEVEL_COST, MAX_LEVEL, NamingOffer};
use brasfoot_systems::config::ModConfig;
use brasfoot_systems::economy::is_national_team;
use eframe::egui::{self, Color32, vec2};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::form::{Question, Reply};
use crate::screens::modkit::{self, Cell, GOLD, GOLD_DARK, GREEN, SOFT};
use crate::{format, motion, theme};
use crate::screens::controls;

/// `nomesSetores`
const SECTORS: [&str; 4] = ["assets-sector-general", "assets-sector-stand", "assets-sector-chairs", "assets-sector-box"];
/// The spinners' highest ticket price per sector.
const MAX_PRICE: [i64; 4] = [200, 300, 500, 1000];
const TABS: [&str; 4] = ["assets-tab-stadium", "assets-tab-infra", "assets-tab-marketing", "assets-tab-ranking"];

/// What a confirmation is about.
#[derive(Clone, Copy)]
enum Pending {
    EndNaming,
}

pub struct AssetsState {
    tab: usize,
    /// The expansion's spinners.
    add: [i64; 4],
    /// The ticket prices' spinners and the check box.
    prices: [i64; 4],
    auto: bool,
    scope: usize,
    sort: usize,
    question: Option<Question>,
    pending: Option<Pending>,
    /// The academy's button closes this and opens the youth panel.
    pub open_youth: bool,
    /// "Tela Clássica do Jogo" closes this and opens the game's stadium screen.
    pub open_classic: bool,
}

impl AssetsState {
    /// `PatrimonioDialog.mostrar`: `Patrimonio.tick()` first, then the first tab.
    pub fn new(s: &mut CareerScreen, club: ClubId) -> Self {
        let career = &mut s.career;
        career.systems.with(&mut career.world, &mut career.runtime, |m| m.assets_tick());
        let mut state = AssetsState { tab: 0, add: [0; 4], prices: [0; 4], auto: true, scope: 0, sort: 0, question: None, pending: None, open_youth: false, open_classic: false };
        state.reset_prices(&career.world, club);
        state
    }

    /// `mostrar(..., abaInicial)`
    pub fn select(&mut self, tab: usize) {
        self.tab = tab.min(TABS.len() - 1);
    }

    /// The prices card as `painelEstadio` builds it: the suggested ones when automatic.
    fn reset_prices(&mut self, world: &World, club: ClubId) {
        let Some(stadium) = world.club(club).stadium else { return };
        let st = world.stadium(stadium);
        let suggested = brasfoot_engine::stadium::default_prices(world, 1, club);
        self.auto = st.automatic_prices;
        for s in 0..4 {
            let shown = if st.automatic_prices { suggested[s] } else { st.ticket_prices.get(s).copied().unwrap_or(suggested[s]) };
            self.prices[s] = (shown as i64).clamp(1, MAX_PRICE[s]);
        }
    }
}

/// `Diretor.meuClube()` with a stadium: the screen opens only then.
pub fn available(world: &World, club: Option<ClubId>) -> bool {
    club.is_some_and(|c| world.club(c).stadium.is_some())
}

/// Which action a refusal came from: the original words the missing cash differently in each.
#[derive(Clone, Copy)]
enum Action {
    Works,
    Upgrade(Pillar),
    Naming,
    Campaign,
}

/// The original's warnings (`Patrimonio`'s returned strings).
fn error_text(t: &Translator, world: &World, club: ClubId, action: Action, e: AssetsError) -> String {
    let money = |v: i64| format::mi(t, v);
    match e {
        AssetsError::Cash { cost } => match action {
            Action::Works => t.text("assets-error-no-cash-works", Some(&args([("cost", money(cost))]))),
            Action::Upgrade(_) => t.text("assets-error-no-cash-upgrade", Some(&args([("cost", money(cost)), ("cash", money(world.club(club).cash))]))),
            Action::Naming => t.text("assets-error-no-cash-fine", Some(&args([("fine", money(cost))]))),
            Action::Campaign => t.text("assets-error-no-cash-campaign", Some(&args([("cost", money(cost))]))),
        },
        AssetsError::MaxLevel => {
            let pillar = if let Action::Upgrade(p) = action { t.text(pillar_key(p), None) } else { String::new() };
            t.text("assets-error-max-level", Some(&args([("pillar", pillar)])))
        }
        AssetsError::NoStadium => t.text(if matches!(action, Action::Works) { "assets-error-no-stadium" } else { "assets-error-invalid-club" }, None),
        AssetsError::WorkInProgress => t.text("assets-error-works-ongoing", None),
        AssetsError::NoSeats => t.text("assets-error-no-seats", None),
        AssetsError::NamingActive { until } => t.text("assets-error-naming-active", Some(&args([("year", until)]))),
        AssetsError::NamingNeedsLevels => t.text("assets-error-naming-requirements", None),
        AssetsError::NoNaming => t.text("assets-error-no-naming", None),
        AssetsError::BadCampaign => t.text("assets-error-invalid-campaign", None),
        AssetsError::CampaignDone { kind } => t.text("assets-error-campaign-done", Some(&args([("name", t.text(&format!("assets-campaign-{kind}"), None))]))),
        AssetsError::CampaignNeedsMarketing { level } => t.text("assets-error-campaign-level", Some(&args([("level", level)]))),
    }
}

/// The catalog's stem of a pillar (`Pilar.titulo`, `nomes`, `descricoes`).
fn pillar_key(p: Pillar) -> &'static str {
    match p {
        Pillar::Arena => "assets-pillar-arena",
        Pillar::TrainingCentre => "assets-pillar-training-centre",
        Pillar::Academy => "assets-pillar-academy",
        Pillar::Marketing => "assets-pillar-marketing",
    }
}

/// `Pilar.manutencao`, read through the system.
fn upkeep(p: Pillar, level: i32) -> i64 {
    sys::pillar_upkeep(p, level)
}

pub fn window(s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(18, 14));
    motion::dialog(ctx, "assets", open, vec2(1020.0, 690.0), frame, |ui, open| {
        let career = &mut s.career;
        let data = career.systems.with(&mut career.world, &mut career.runtime, |m| m.assets_data(club));
        let upkeep_total = career.systems.with(&mut career.world, &mut career.runtime, |m| m.monthly_upkeep(club));
        let world = &career.world;
        let Some(stadium) = world.club(club).stadium else { return };
        let st = world.stadium(stadium);
        // the header: the title, and the summary on the right (below it when there is no room)
        let name = world.club(club).name.to_uppercase();
        let capacity: i32 = st.capacity.iter().sum();
        let summary = t.text(
            "assets-summary",
            Some(&args([
                ("cash", format::money(t, world.club(club).cash)),
                ("stadium", st.name.clone()),
                ("capacity", format::thousands(t, capacity as i64)),
                ("limit", format::thousands(t, sys::total_limit(world) as i64)),
                ("upkeep", format::mi(t, upkeep_total)),
            ])),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(t.text("assets-header", Some(&args([("club", name)])))).font(theme::bold(18.0)).color(GOLD));
            ui.add_space(12.0);
            ui.label(egui::RichText::new(summary).font(theme::regular(12.0)).color(theme::MUTED));
        });
        ui.add_space(8.0);
        let names: Vec<String> = TABS.iter().map(|k| t.text(k, None)).collect();
        if let Some(i) = controls::tabs(ui, &names, state.tab) {
            state.tab = i;
        }
        ui.add_space(10.0);
        let footer = 34.0;
        let body = ui.available_height() - footer;
        ui.allocate_ui(vec2(ui.available_width(), body), |ui| {
            ui.set_min_height(body);
            // the ranking scrolls its own table
            if state.tab == 3 {
                ranking_tab(ui, s, state, club, t, files);
                return;
            }
            egui::ScrollArea::vertical().id_salt(("assets-body", state.tab)).auto_shrink([false, false]).show(ui, |ui| match state.tab {
                0 => stadium_tab(ui, s, state, club, &data, t),
                1 => infra_tab(ui, s, state, club, t),
                _ => marketing_tab(ui, s, state, club, &data, t),
            });
        });
        ui.horizontal(|ui| {
            modkit::text(ui, &t.text("assets-tip", None), 11.0, theme::MUTED);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if controls::button(ui, &t.text("assets-button-close", None), controls::Kind::Secondary, true).clicked() {
                    *open = false;
                }
            });
        });
        if state.open_youth || state.open_classic {
            *open = false;
        }
    });
    if let Some(reply) = modkit::question_overlay(ctx, t, "assets", &mut state.question)
        && reply == Reply::Yes
        && let Some(Pending::EndNaming) = state.pending.take()
    {
        let career = &mut s.career;
        if let Err(e) = career.systems.with(&mut career.world, &mut career.runtime, |m| m.end_naming(club)) {
            state.question = Some(modkit::message(t.text("assets-naming-error-title", None), error_text(t, &career.world, club, Action::Naming, e)));
        }
    }
}

/// Runs an action of `Patrimonio`; a refusal shows its warning under `title`.
fn act(s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, t: &Translator, title: String, kind: Action, action: impl FnOnce(&mut brasfoot_systems::Mod) -> Result<(), AssetsError>) {
    let career = &mut s.career;
    if let Err(e) = career.systems.with(&mut career.world, &mut career.runtime, action) {
        state.question = Some(modkit::message(title, error_text(t, &career.world, club, kind, e)));
    }
}

// Tab 1: the stadium, its sectors and prices, the arena and the naming rights

fn stadium_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, data: &ClubAssets, t: &Translator) {
    ui.columns(2, |cols| {
        stadium_card(&mut cols[0], s, state, club, data, t);
        cols[0].add_space(10.0);
        prices_card(&mut cols[0], s, state, club, t);
        pillar_card(&mut cols[1], s, state, club, Pillar::Arena, data, t);
        cols[1].add_space(10.0);
        naming_card(&mut cols[1], s, state, club, data, t);
    });
}

fn stadium_card(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, data: &ClubAssets, t: &Translator) {
    let world = &s.career.world;
    let Some(stadium) = world.club(club).stadium else { return };
    let st = world.stadium(stadium);
    let current: Vec<i32> = (0..4).map(|i| st.capacity.get(i).copied().unwrap_or(0)).collect();
    let limits = sys::sector_limits(world);
    let work = world.game.stadium_works.iter().find(|w| w.stadium == Some(stadium)).map(|w| w.until);
    let mut start = false;
    modkit::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        modkit::title(ui, &st.name.to_uppercase(), 15.0);
        ui.add_space(4.0);
        let mut pitch = t.text(&format!("assets-pitch-{}", st.condition.clamp(0, 3)), None);
        if data.arena >= 1 {
            pitch = t.text("assets-pitch-hybrid", Some(&args([("pitch", pitch)])));
        }
        let capacity: i32 = current.iter().sum();
        modkit::text(
            ui,
            &t.text("assets-capacity", Some(&args([("capacity", format::thousands(t, capacity as i64)), ("limit", format::thousands(t, sys::total_limit(world) as i64)), ("pitch", pitch)]))),
            12.0,
            theme::TEXT,
        );
        let year = brasfoot_systems::economy::year(world);
        if !data.naming_brand.is_empty() && year <= data.naming_until {
            ui.add_space(3.0);
            let line = t.text(
                "assets-naming-line",
                Some(&args([("brand", data.naming_brand.clone()), ("amount", format::mi(t, data.naming_yearly)), ("year", data.naming_until.to_string()), ("original", data.stadium_base_name.clone())])),
            );
            modkit::strong(ui, &line, 11.0, GREEN);
        }
        ui.add_space(10.0);
        modkit::title(ui, &t.text("assets-expansion-title", None), 12.0);
        ui.add_space(6.0);
        if let Some(until) = work {
            let date = format::date_medium(t, &s.career.runtime.clock, until);
            modkit::strong(ui, &t.text("assets-works-ongoing", Some(&args([("date", date)]))), 12.0, GREEN);
            ui.add_space(6.0);
        }
        let mut available = 0;
        egui::Grid::new("assets-sectors").num_columns(3).spacing(vec2(10.0, 6.0)).show(ui, |ui| {
            for sector in 0..4 {
                let room = (limits[sector] - current[sector]).max(0) as i64;
                available += room;
                let line = t.text(
                    "assets-sector-line",
                    Some(&args([("sector", t.text(SECTORS[sector], None)), ("current", format::thousands(t, current[sector] as i64)), ("limit", format::thousands(t, limits[sector] as i64))])),
                );
                ui.label(egui::RichText::new(line).font(theme::regular(12.0)));
                ui.label(egui::RichText::new(t.text("assets-sector-max", Some(&args([("n", format::thousands(t, room))])))).font(theme::regular(11.0)).color(theme::MUTED));
                let step = if sector == 3 { 50 } else { 500 };
                state.add[sector] = state.add[sector].min(room);
                modkit::spinner(ui, &mut state.add[sector], 0, room, step.min(room.max(1)), work.is_none() && room > 0);
                ui.end_row();
            }
        });
        ui.add_space(8.0);
        let seats: i64 = state.add.iter().sum();
        let budget = if seats <= 0 {
            t.text("assets-budget-empty", None)
        } else {
            let cost: i64 = 100_000 + (0..4).map(|i| sys::sector_cost(i, current[i], state.add[i] as i32)).sum::<i64>();
            t.text("assets-budget", Some(&args([("seats", format::thousands(t, seats)), ("cost", format::money(t, cost)), ("days", sys::work_days(seats as i32).to_string())])))
        };
        modkit::text(ui, &budget, 11.0, theme::MUTED);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if controls::button(ui, &t.text("assets-button-fill-max", None), controls::Kind::Secondary, work.is_none() && available > 0).clicked() {
                for sector in 0..4 {
                    state.add[sector] = (limits[sector] - current[sector]).max(0) as i64;
                }
            }
            if controls::button(ui, &t.text("assets-button-start-works", None), controls::Kind::Primary, work.is_none() && available > 0).clicked() {
                start = true;
            }
        });
    });
    if start {
        let add = state.add.map(|v| v as i32);
        act(s, state, club, t, t.text("assets-works-error-title", None), Action::Works, |m| m.start_stadium_expansion(club, add));
        if state.question.is_none() {
            state.add = [0; 4];
        }
    }
}

fn prices_card(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, t: &Translator) {
    let world = &s.career.world;
    let suggested = brasfoot_engine::stadium::default_prices(world, 1, club);
    let mut save = false;
    let mut classic = false;
    let mut auto_changed = false;
    modkit::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        modkit::title(ui, &t.text("assets-prices-title", None), 13.0);
        ui.add_space(4.0);
        let line = t.text(
            "assets-prices-suggested",
            Some(&args([("general", suggested[0].to_string()), ("stand", suggested[1].to_string()), ("chairs", suggested[2].to_string()), ("box", suggested[3].to_string())])),
        );
        modkit::text(ui, &line, 11.0, theme::MUTED);
        ui.add_space(6.0);
        if crate::screens::menu::check_row(ui, &mut state.auto, &t.text("assets-check-auto-prices", None), true) {
            auto_changed = true;
        }
        ui.add_space(6.0);
        egui::Grid::new("assets-prices").num_columns(4).spacing(vec2(8.0, 4.0)).show(ui, |ui| {
            for key in SECTORS {
                ui.label(egui::RichText::new(t.text(key, None)).font(theme::bold(11.0)));
            }
            ui.end_row();
            for sector in 0..4 {
                modkit::spinner(ui, &mut state.prices[sector], 1, MAX_PRICE[sector], 1, !state.auto);
            }
            ui.end_row();
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            save = controls::button(ui, &t.text("assets-button-save-prices", None), controls::Kind::Secondary, true).clicked();
            classic = controls::button(ui, &t.text("assets-button-classic", None), controls::Kind::Secondary, true).clicked();
        });
    });
    let Some(stadium) = s.career.world.club(club).stadium else { return };
    // chkSugeridos: the stadium switches at once, and the spinners show the suggested prices
    if auto_changed {
        s.career.world.stadium_mut(stadium).automatic_prices = state.auto;
        if state.auto {
            for sector in 0..4 {
                state.prices[sector] = suggested[sector] as i64;
            }
        }
    }
    if save {
        let st = s.career.world.stadium_mut(stadium);
        st.automatic_prices = state.auto;
        if !state.auto {
            st.ticket_prices = state.prices.iter().map(|&p| p as i32).collect();
        }
        state.question = Some(modkit::message(t.text("assets-prices-saved-title", None), t.text("assets-prices-saved", None)));
    }
    if classic {
        state.open_classic = true;
    }
}

/// `cardPilar`: the pillar's level, the next one and its button.
fn pillar_card(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, p: Pillar, data: &ClubAssets, t: &Translator) {
    let level = data.level(p);
    let key = pillar_key(p);
    let pillar_title = t.text(key, None);
    let mut upgrade = false;
    modkit::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        modkit::title(ui, &t.text("assets-pillar-header", Some(&args([("pillar", pillar_title.to_uppercase()), ("level", level.to_string())]))), 14.0);
        ui.add_space(4.0);
        let current = t.text("assets-pillar-current", Some(&args([("name", t.text(&format!("{key}-level-{level}"), None)), ("upkeep", format::mi(t, upkeep(p, level)))])));
        modkit::text(ui, &current, 12.0, theme::TEXT);
        modkit::text(ui, &t.text(&format!("{key}-effect-{level}"), None), 12.0, SOFT);
        ui.add_space(8.0);
        if level < MAX_LEVEL {
            let next = level + 1;
            let cost = LEVEL_COST[next as usize];
            modkit::strong(ui, &t.text("assets-pillar-next", Some(&args([("level", next.to_string()), ("name", t.text(&format!("{key}-level-{next}"), None))]))), 11.0, theme::TEXT);
            modkit::text(ui, &t.text("assets-pillar-next-cost", Some(&args([("cost", format::mi(t, cost)), ("upkeep", format::mi(t, upkeep(p, next)))]))), 11.0, theme::TEXT);
            modkit::text(ui, &t.text(&format!("{key}-effect-{next}"), None), 11.0, Color32::from_rgb(0x80, 0xd8, 0x98));
            ui.add_space(8.0);
            upgrade = controls::button(ui, &t.text("assets-pillar-button-upgrade", Some(&args([("level", next.to_string()), ("cost", format::mi(t, cost))]))), controls::Kind::Primary, true).clicked();
        } else {
            modkit::strong(ui, &t.text("assets-pillar-max", None), 12.0, GREEN);
        }
    });
    if upgrade {
        act(s, state, club, t, pillar_title, Action::Upgrade(p), |m| m.upgrade_pillar(club, p));
    }
}

/// `cardPilarDetalhado`: every level of the pillar with its state.
fn pillar_detail_card(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, p: Pillar, t: &Translator) {
    let career = &mut s.career;
    let level = career.systems.with(&mut career.world, &mut career.runtime, |m| m.pillar_level(club, p));
    let key = pillar_key(p);
    let pillar_title = t.text(key, None);
    let mut upgrade = false;
    modkit::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        modkit::title(ui, &t.text("assets-pillar-header", Some(&args([("pillar", pillar_title.to_uppercase()), ("level", level.to_string())]))), 15.0);
        ui.add_space(3.0);
        let current = t.text("assets-pillar-current-short", Some(&args([("name", t.text(&format!("{key}-level-{level}"), None)), ("upkeep", format::mi(t, upkeep(p, level)))])));
        modkit::text(ui, &current, 12.0, theme::MUTED);
        ui.add_space(8.0);
        for i in 1..=MAX_LEVEL {
            let border = if i == level { GREEN } else if i == level + 1 { GOLD_DARK } else { theme::BORDER };
            let fill = if i == level { Some(Color32::from_rgb(45, 58, 42)) } else { None };
            modkit::item(Some(border), fill).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (status, color) = if i < level {
                        (t.text("assets-level-done", None), GREEN)
                    } else if i == level {
                        (t.text("assets-level-current", None), GREEN)
                    } else {
                        (t.text("assets-level-cost", Some(&args([("cost", format::mi(t, LEVEL_COST[i as usize]))]))), if i == level + 1 { GOLD } else { theme::MUTED })
                    };
                    let status_width = 110.0;
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - status_width);
                        modkit::strong(ui, &t.text("assets-level-item", Some(&args([("level", i.to_string()), ("name", t.text(&format!("{key}-level-{i}"), None))]))), 11.0, theme::TEXT);
                        let line = t.text("assets-level-description", Some(&args([("description", t.text(&format!("{key}-effect-{i}"), None)), ("upkeep", format::mi(t, upkeep(p, i)))])));
                        modkit::text(ui, &line, 11.0, SOFT);
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(status).font(theme::bold(11.0)).color(color));
                    });
                });
            });
            ui.add_space(5.0);
        }
        if level < MAX_LEVEL {
            let next = level + 1;
            let label = t.text(
                "assets-pillar-button-build",
                Some(&args([("level", next.to_string()), ("name", t.text(&format!("{key}-level-{next}"), None)), ("cost", format::mi(t, LEVEL_COST[next as usize]))])),
            );
            upgrade = controls::button(ui, &label, controls::Kind::Primary, true).clicked();
        } else {
            modkit::strong(ui, &t.text("assets-pillar-max-infra", None), 12.0, GREEN);
        }
    });
    if upgrade {
        act(s, state, club, t, pillar_title, Action::Upgrade(p), |m| m.upgrade_pillar(club, p));
    }
}

fn naming_card(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, data: &ClubAssets, t: &Translator) {
    let career = &mut s.career;
    let year = brasfoot_systems::economy::year(&career.world);
    let active = !data.naming_brand.is_empty() && year <= data.naming_until;
    let offers: Vec<NamingOffer> = if active || data.arena < 3 || data.marketing < 2 { Vec::new() } else { career.systems.with(&mut career.world, &mut career.runtime, |m| m.naming_offers(club)) };
    let fine = if active { career.systems.with(&mut career.world, &mut career.runtime, |m| m.naming_fine(club)) } else { 0 };
    let world = &career.world;
    let mut sign = None;
    let mut cancel = false;
    modkit::card().show(ui, |ui| {
        ui.set_width(ui.available_width());
        modkit::title(ui, &t.text("assets-naming-title", None), 14.0);
        ui.add_space(4.0);
        if active {
            let name = world.club(club).stadium.map(|st| world.stadium(st).name.clone()).unwrap_or_default();
            for (key, a) in [
                ("assets-naming-active-1", args([("brand", data.naming_brand.clone()), ("year", data.naming_until.to_string())])),
                ("assets-naming-active-2", args([("name", name), ("original", data.stadium_base_name.clone())])),
                ("assets-naming-active-3", args([("amount", format::mi(t, data.naming_yearly)), ("monthly", format::mi(t, data.naming_yearly / 12))])),
            ] {
                modkit::text(ui, &t.text(key, Some(&a)), 12.0, theme::TEXT);
            }
            ui.add_space(8.0);
            cancel = controls::button(ui, &t.text("assets-naming-cancel-button", Some(&args([("fine", format::mi(t, fine))]))), controls::Kind::Secondary, true).clicked();
            return;
        }
        if data.arena < 3 || data.marketing < 2 {
            modkit::text(ui, &t.text("assets-naming-requirements", None), 12.0, theme::MUTED);
            modkit::text(ui, &t.text("assets-naming-requirement-arena", Some(&args([("level", data.arena)]))), 12.0, theme::MUTED);
            modkit::text(ui, &t.text("assets-naming-requirement-marketing", Some(&args([("level", data.marketing)]))), 12.0, theme::MUTED);
            return;
        }
        modkit::text(ui, &t.text("assets-naming-offers-intro", None), 11.0, theme::MUTED);
        ui.add_space(6.0);
        for (i, offer) in offers.iter().enumerate() {
            modkit::item(None, None).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - 90.0);
                        let brand = brasfoot_model::brands::NAMING_BRANDS[offer.brand];
                        modkit::strong(ui, &t.text("assets-naming-offer-name", Some(&args([("stadium", offer.stadium_name.clone()), ("brand", brand.to_owned())]))), 11.0, theme::TEXT);
                        let profile = t.text(&format!("assets-naming-profile-{}", offer.years), None);
                        let line = t.text("assets-naming-offer-terms", Some(&args([("profile", profile), ("upfront", format::mi(t, offer.signing_fee)), ("annual", format::mi(t, offer.yearly))])));
                        modkit::text(ui, &line, 11.0, SOFT);
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if controls::button(ui, &t.text("assets-button-sign", None), controls::Kind::Primary, true).clicked() {
                            sign = Some(i);
                        }
                    });
                });
            });
            ui.add_space(5.0);
        }
    });
    if cancel {
        let text = t.text("assets-naming-cancel-confirm", Some(&args([("fine", format::mi(t, fine)), ("brand", data.naming_brand.clone()), ("original", data.stadium_base_name.clone())])));
        state.question = Some(modkit::confirm(t.text("assets-naming-cancel-title", None), text));
        state.pending = Some(Pending::EndNaming);
    }
    if let Some(i) = sign {
        let offer = offers[i].clone();
        act(s, state, club, t, t.text("assets-naming-error-title", None), Action::Naming, |m| m.sign_naming(club, &offer));
    }
}

// Tab 2: the training centre and the academy

fn infra_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, t: &Translator) {
    ui.columns(2, |cols| {
        pillar_detail_card(&mut cols[0], s, state, club, Pillar::TrainingCentre, t);
        pillar_detail_card(&mut cols[1], s, state, club, Pillar::Academy, t);
        cols[1].add_space(10.0);
        let world = &s.career.world;
        // contarJuniores: the youth players of the club
        let count = world.game.youth_players.iter().filter(|&&p| world.player(p).club == Some(club)).count();
        let target = sys::youth_target(world, &s.career.systems.config, club);
        modkit::card().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - 170.0);
                    modkit::strong(ui, &t.text("assets-youth-count", Some(&args([("count", count.to_string()), ("target", target.to_string())]))), 11.0, theme::TEXT);
                    modkit::text(ui, &t.text("assets-youth-note", None), 11.0, SOFT);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if controls::button(ui, &t.text("assets-button-open-youth", None), controls::Kind::Secondary, true).clicked() {
                        state.open_youth = true;
                    }
                });
            });
        });
    });
}

// Tab 3: marketing, the commercial income and the season's campaign

fn marketing_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, data: &ClubAssets, t: &Translator) {
    let mut campaign = None;
    ui.columns(2, |cols| {
        pillar_detail_card(&mut cols[0], s, state, club, Pillar::Marketing, t);
        let world = &s.career.world;
        let config: &ModConfig = &s.career.systems.config;
        modkit::card().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            modkit::title(ui, &t.text("assets-revenue-title", None), 14.0);
            ui.add_space(6.0);
            let contract = match world.mods.master_sponsor.active() {
                Some(k) => t.text(
                    "assets-revenue-contract",
                    Some(&args([
                        ("brand", brasfoot_model::brands::MASTER_BRANDS[k.brand as usize].to_owned()),
                        ("amount", format::mi(t, k.yearly)),
                        ("kind", brasfoot_i18n::labels::master_kind(t, k.kind)),
                    ])),
                ),
                None => t.text("assets-revenue-no-contract", None),
            };
            let bonus = |f: f64| ((f - 1.0) * 100.0).round() as i64;
            let sponsor_bonus = bonus(sys::sponsorship_factor_now(world, config, club));
            modkit::text(ui, &t.text("assets-revenue-master", Some(&args([("contract", contract), ("bonus", sponsor_bonus.to_string())]))), 12.0, theme::TEXT);
            let monthly = brasfoot_systems::fans::monthly_members(world, config);
            let members_bonus = bonus(sys::members_factor_now(world, config, club));
            modkit::text(
                ui,
                &t.text("assets-revenue-members", Some(&args([("monthly", format::mi(t, monthly)), ("annual", format::mi(t, monthly * 12)), ("bonus", members_bonus.to_string())]))),
                12.0,
                theme::TEXT,
            );
            modkit::text(ui, &t.text("assets-revenue-fans", Some(&args([("value", brasfoot_systems::fans::confidence(world))]))), 12.0, theme::TEXT);
        });
        cols[1].add_space(10.0);
        let done = data.campaign_season == world.season();
        modkit::card().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            modkit::title(ui, &t.text("assets-campaigns-title", None), 14.0);
            ui.add_space(4.0);
            if done {
                modkit::strong(ui, &t.text("assets-campaigns-done", Some(&args([("name", t.text(&format!("assets-campaign-{}", data.campaign_kind), None))]))), 12.0, GREEN);
            } else {
                modkit::text(ui, &t.text("assets-campaigns-intro", None), 11.0, theme::MUTED);
            }
            ui.add_space(8.0);
            for kind in 1..=3 {
                let unlocked = data.marketing >= CAMPAIGN_MARKETING[kind] && !done;
                modkit::item(None, None).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width() - 130.0);
                            modkit::strong(ui, &t.text(&format!("assets-campaign-{kind}"), None), 11.0, theme::TEXT);
                            modkit::text(ui, &t.text(&format!("assets-campaign-{kind}-cost"), None), 11.0, SOFT);
                            modkit::text(ui, &t.text(&format!("assets-campaign-{kind}-effect"), None), 11.0, SOFT);
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = if done && data.campaign_kind == kind as i32 {
                                t.text("assets-campaign-button-done", None)
                            } else {
                                t.text("assets-campaign-button-invest", Some(&args([("cost", format::mi(t, CAMPAIGN_COST[kind]))])))
                            };
                            if controls::button(ui, &label, if unlocked { controls::Kind::Primary } else { controls::Kind::Secondary }, unlocked).clicked() {
                                campaign = Some(kind as i32);
                            }
                        });
                    });
                });
                ui.add_space(6.0);
            }
        });
    });
    if let Some(kind) = campaign {
        act(s, state, club, t, t.text("assets-campaign-error-title", None), Action::Campaign, |m| m.run_campaign(club, kind));
    }
}

// Tab 4: the ranking of stadiums and structures

const RANKING_COLUMNS: [(&str, f32); 11] = [
    ("assets-ranking-column-rank", 36.0),
    ("assets-ranking-column-club", 165.0),
    ("assets-ranking-column-reputation", 42.0),
    ("assets-ranking-column-cash", 95.0),
    ("assets-ranking-column-stadium", 195.0),
    ("assets-ranking-column-capacity", 95.0),
    ("assets-ranking-column-arena", 62.0),
    ("assets-ranking-column-training-centre", 62.0),
    ("assets-ranking-column-academy", 70.0),
    ("assets-ranking-column-marketing", 74.0),
    ("assets-ranking-column-total", 62.0),
];

fn ranking_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut AssetsState, club: ClubId, t: &Translator, files: &GameFiles) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t.text("assets-ranking-filter", None)).font(theme::bold(12.0)));
        let scopes = ["assets-ranking-scope-country", "assets-ranking-scope-world", "assets-ranking-scope-all"];
        egui::ComboBox::from_id_salt("assets-scope").width(220.0).selected_text(t.text(scopes[state.scope], None)).show_ui(ui, |ui| {
            for (i, k) in scopes.iter().enumerate() {
                ui.selectable_value(&mut state.scope, i, t.text(k, None));
            }
        });
        ui.add_space(12.0);
        ui.label(egui::RichText::new(t.text("assets-ranking-sort", None)).font(theme::bold(12.0)));
        let sorts = ["assets-ranking-sort-capacity", "assets-ranking-sort-infra", "assets-ranking-sort-cash"];
        egui::ComboBox::from_id_salt("assets-sort").width(240.0).selected_text(t.text(sorts[state.sort], None)).show_ui(ui, |ui| {
            for (i, k) in sorts.iter().enumerate() {
                ui.selectable_value(&mut state.sort, i, t.text(k, None));
            }
        });
    });
    ui.add_space(8.0);
    let career = &mut s.career;
    let me = career.world.club(club).country;
    let clubs: Vec<ClubId> = career
        .world
        .game
        .clubs
        .iter()
        .copied()
        .filter(|&c| !is_national_team(&career.world, c) && career.world.club(c).stadium.is_some())
        .filter(|&c| match state.scope {
            0 => career.world.club(c).country == me,
            1 => career.world.club(c).reputation >= 3 || c == club,
            _ => true,
        })
        .collect();
    // Patrimonio.dados and Economia.estimativa for each, through the systems
    let mut rows: Vec<(ClubId, ClubAssets, i64, i32)> = clubs
        .into_iter()
        .map(|c| {
            let (data, estimate) = career.systems.with(&mut career.world, &mut career.runtime, |m| (m.assets_data(c), m.estimate(c)));
            let capacity: i32 = career.world.club(c).stadium.map(|st| career.world.stadium(st).capacity.iter().sum()).unwrap_or(0);
            (c, data, estimate, capacity)
        })
        .collect();
    let total = |e: &ClubAssets| e.arena + e.training_centre + e.academy + e.marketing;
    let sort = state.sort;
    rows.sort_by(|a, b| match sort {
        0 => b.3.cmp(&a.3).then(total(&b.1).cmp(&total(&a.1))),
        1 => total(&b.1).cmp(&total(&a.1)).then(b.3.cmp(&a.3)),
        _ => b.2.cmp(&a.2),
    });
    let world = &career.world;
    let level = |n: i32| Cell::from(t.text("assets-ranking-level", Some(&args([("level", n)]))));
    let cells: Vec<Vec<Cell>> = rows
        .iter()
        .enumerate()
        .map(|(i, (c, e, estimate, capacity))| {
            let x = world.club(*c);
            let name = if *c == club { Cell::colored(t.text("assets-ranking-mine", Some(&args([("club", x.name.clone())]))), GOLD) } else { Cell::from(&x.name) };
            vec![
                Cell::from(i + 1),
                name,
                Cell::from(x.reputation),
                Cell::from(format::approximate(t, *estimate)),
                Cell::from(x.stadium.map(|st| world.stadium(st).name.clone()).unwrap_or_default()),
                Cell::from(format::thousands(t, *capacity as i64)),
                level(e.arena),
                level(e.training_centre),
                level(e.academy),
                level(e.marketing),
                Cell::from(t.text("assets-ranking-total", Some(&args([("sum", total(e))])))),
            ]
        })
        .collect();
    modkit::table(ui, files, "assets-ranking", &modkit::columns(t, &RANKING_COLUMNS), &cells, false);
}
