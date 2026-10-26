//! The satellite clubs (`bfmod.SatelitesDialog`, "Clubes Satélites e Rede Multiclubes"): the
//! human club's satellites (policy, tactic, cash, partnership or ownership, and the moves of
//! players and juniors between them), the search for a new satellite and the world's networks
//! with their history. Every action is `Satelites`'s (`systems::satellites`).

use brasfoot_i18n::{Translator, args, labels};
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::mods::{LoanRole, Pillar, SatelliteEvent, SatelliteLink, SatellitePolicy, SatelliteTactic};
use brasfoot_model::world::World;
use brasfoot_systems::academy::{self, YouthLabel};
use brasfoot_systems::assets::{LEVEL_COST, MAX_LEVEL};
use brasfoot_systems::satellites::{self as sys, SatelliteBlock, SatelliteError};
use eframe::egui::{self, Color32, vec2};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::form::{Question, Reply};
use crate::screens::modkit::{self, Cell, Choice, GOLD, SOFT};
use crate::screens::new_game::country_name;
use crate::screens::widgets;
use crate::{format, motion, theme};
use crate::screens::controls;

const GREEN: Color32 = Color32::from_rgb(90, 210, 130);
const RED: Color32 = Color32::from_rgb(235, 95, 95);
const TABS: [&str; 3] = ["satellites-tab-mine", "satellites-tab-search", "satellites-tab-networks"];
const POLICIES: [SatellitePolicy; 3] = [SatellitePolicy::YoungPlayers, SatellitePolicy::Promotion, SatellitePolicy::Profit];
const STYLES: [&str; 3] = ["lineup-style-balanced", "lineup-style-all-out-attack", "lineup-style-counter-attack"];
const MARKINGS: [&str; 3] = ["lineup-marking-light", "lineup-marking-heavy", "lineup-marking-very-heavy"];
const SIDES: [&str; 2] = ["lineup-attack-center", "lineup-attack-flanks"];
/// `btInjetar`, `btRetirar`: the amounts offered.
const INJECTIONS: [i64; 5] = [5_000_000, 15_000_000, 30_000_000, 60_000_000, 100_000_000];
const WITHDRAWALS: [i64; 3] = [5_000_000, 15_000_000, 30_000_000];
const ROLES: [LoanRole; 3] = [LoanRole::Starter, LoanRole::Regular, LoanRole::Reserve];

/// Which action a warning came from: the original words the same refusal differently in each.
#[derive(Clone, Copy)]
enum Action {
    Partnership,
    Renew,
    End,
    Buy,
    Sell,
    Inject,
    Withdraw,
    Invest,
    Lend,
    Recall,
    SignFrom,
    SendTo,
    PullYouth,
}

/// What a confirmation or a choice is for.
#[derive(Clone, Copy)]
enum Pending {
    Buy(ClubId),
    End(ClubId),
    Sell(ClubId),
    Inject(ClubId),
    Withdraw(ClubId, i64),
    Lend(PlayerId, ClubId),
    Recall(PlayerId),
    SendTo(PlayerId, ClubId),
    SignFrom(PlayerId, ClubId),
    Partnership(ClubId),
    BuyNew(ClubId),
    Invest(ClubId, Pillar),
}

pub struct SatellitesState {
    tab: usize,
    satellite: Option<ClubId>,
    left: Option<usize>,
    pros: Option<usize>,
    juniors: Option<usize>,
    /// The search's filters and its rows, worked out again when a filter or the world changes.
    country: i32,
    reputation: usize,
    eligible_only: bool,
    order: usize,
    found: Option<Vec<ClubId>>,
    found_selected: Option<usize>,
    question: Option<Question>,
    choice: Option<Choice>,
    pending: Option<Pending>,
    /// The satellite's assets dialog (`abrirDialogoPatrimonio`).
    assets_of: Option<ClubId>,
}

impl SatellitesState {
    /// `SatelitesDialog.mostrar`: `Satelites.tick()` and `Base.tick()` first.
    pub fn new(s: &mut CareerScreen) -> Self {
        let career = &mut s.career;
        career.systems.with(&mut career.world, &mut career.runtime, |m| m.open_satellites());
        SatellitesState {
            tab: 0,
            satellite: None,
            left: None,
            pros: None,
            juniors: None,
            country: -1,
            reputation: 0,
            eligible_only: true,
            order: 0,
            found: None,
            found_selected: None,
            question: None,
            choice: None,
            pending: None,
            assets_of: None,
        }
    }

    pub fn select(&mut self, tab: usize) {
        self.tab = tab.min(TABS.len() - 1);
    }

    /// After an action: the lists are read again.
    fn changed(&mut self) {
        self.found = None;
    }
}

fn money(t: &Translator, v: i64) -> String {
    format::money(t, v)
}

fn block_text(t: &Translator, target: &str, b: &SatelliteBlock) -> String {
    let club = || args([("club", target.to_owned())]);
    match b {
        SatelliteBlock::Invalid => t.text("satellites-block-invalid", None),
        SatelliteBlock::Itself => t.text("satellites-block-itself", None),
        SatelliteBlock::HumanOrNationalTeam => t.text("satellites-block-human", None),
        SatelliteBlock::ParentIsSatellite => t.text("satellites-block-parent-is-satellite", None),
        SatelliteBlock::TargetIsParent => t.text("satellites-block-target-is-parent", Some(&club())),
        SatelliteBlock::SatelliteOfAnother { parent } => {
            let parent = parent.clone().unwrap_or_else(|| t.text("satellites-block-another-club", None));
            t.text("satellites-block-satellite-of", Some(&args([("club", target.to_owned()), ("parent", parent)])))
        }
        SatelliteBlock::AlreadyLinked => t.text("satellites-block-already-linked", Some(&club())),
        SatelliteBlock::AlreadyOwned => t.text("satellites-block-already-owned", Some(&club())),
        SatelliteBlock::Reputation { target: rt, parent } => t.text("satellites-block-reputation", Some(&args([("target", *rt), ("parent", *parent)]))),
        SatelliteBlock::GiantNotForSale => t.text("satellites-block-giant", None),
        SatelliteBlock::OwnedByInvestor => t.text("satellites-block-investor", Some(&club())),
    }
}

/// The original's warning for a refusal of `action`; `club` and `player` name the satellite and
/// the player involved.
fn error_text(t: &Translator, action: Action, e: &SatelliteError, club: &str, player: &str) -> String {
    use SatelliteError as E;
    let key = match (action, e) {
        (_, E::Blocked(b)) => return block_text(t, club, b),
        (_, E::Suspended) => "satellites-error-suspended",
        (_, E::InvalidAmount) => "satellites-error-invalid-amount",
        (Action::Partnership, E::NotEnoughCash { needed }) => return t.text("satellites-error-cash-partnership", Some(&args([("amount", money(t, *needed))]))),
        (Action::End, E::NotEnoughCash { needed }) => return t.text("satellites-error-cash-fine", Some(&args([("amount", money(t, *needed))]))),
        (Action::Buy, E::NotEnoughCash { needed }) => return t.text("satellites-error-cash-buy", Some(&args([("club", club.to_owned()), ("amount", money(t, *needed))]))),
        (Action::Inject, E::NotEnoughCash { needed }) => return t.text("satellites-error-cash-inject", Some(&args([("amount", money(t, *needed))]))),
        (Action::SignFrom, E::NotEnoughCash { needed }) => return t.text("satellites-error-cash-sign", Some(&args([("player", player.to_owned()), ("amount", money(t, *needed))]))),
        (Action::PullYouth, E::NotEnoughCash { needed }) => return t.text("satellites-error-cash-youth", Some(&args([("player", player.to_owned()), ("amount", money(t, *needed))]))),
        (_, E::WorkingCapital { reserve }) => return t.text("satellites-error-working-capital", Some(&args([("amount", money(t, *reserve))]))),
        (_, E::ParentCash { cost, cash }) => return t.text("satellites-error-parent-cash", Some(&args([("cost", format::mi(t, *cost)), ("cash", format::mi(t, *cash))]))),
        (_, E::AssetsMaxLevel { pillar }) => return t.text("satellites-error-max-level", Some(&args([("pillar", labels::pillar(t, *pillar))]))),
        (_, E::AcademyFull { capacity }) => return t.text("satellites-error-academy-full", Some(&args([("capacity", *capacity)]))),
        (Action::Renew, E::NoPartnership) => "satellites-error-no-active-partnership",
        (Action::End, E::NoPartnership) => "satellites-error-no-partnership",
        (Action::Renew, E::ParentMissing) => "satellites-error-parent-missing",
        (Action::Lend, E::ParentMissing) => "satellites-error-not-your-satellite-selected",
        (_, E::ParentMissing | E::NoPartnership) => "satellites-error-not-your-satellite",
        (Action::Sell | Action::Invest, E::NotInGroup) | (Action::Sell, E::NotOwned) => "satellites-error-not-in-group",
        (Action::Inject, E::NotOwned) => "satellites-error-owned-inject",
        (Action::Withdraw, E::NotOwned) => "satellites-error-owned-withdraw",
        (Action::Invest, E::NotOwned) => "satellites-error-owned-invest",
        (_, E::NotOwned) => "satellites-error-owned-send",
        (Action::Lend, E::NotTheParentsPlayer) => "satellites-error-not-parent-squad",
        (_, E::NotTheParentsPlayer) => "satellites-error-not-parent-player",
        (Action::PullYouth, E::NotTheSatellitesPlayer) => "satellites-error-not-satellite-junior",
        (_, E::NotTheSatellitesPlayer) => "satellites-error-not-satellite-player",
        (_, E::LoanFlag) => "satellites-error-ending-career",
        (Action::SignFrom, E::OnLoan) => "satellites-error-on-loan-recall",
        (Action::SendTo, E::OnLoan) => "satellites-error-on-loan-definitive",
        (_, E::OnLoan) => "satellites-error-on-loan",
        (_, E::NotOnLoan) => "satellites-error-not-on-loan",
        (_, E::ParentSquadTooSmall) => "satellites-error-parent-too-small",
        (Action::Lend, E::SatelliteSquadFull) => return t.text("satellites-error-satellite-full-named", Some(&args([("club", club.to_owned())]))),
        (_, E::SatelliteSquadFull) => "satellites-error-satellite-full",
        (Action::Recall, E::ParentSquadFull) => "satellites-error-no-room",
        (Action::PullYouth, E::ParentSquadFull) => "satellites-error-parent-pro-full",
        (_, E::ParentSquadFull) => "satellites-error-parent-full",
        (_, E::SatelliteSquadTooSmall) => "satellites-error-satellite-too-small",
        (_, E::NotEnoughCash { needed }) => return t.text("satellites-error-cash-inject", Some(&args([("amount", money(t, *needed))]))),
        (_, E::NotInGroup) => "satellites-error-not-in-group",
    };
    t.text(key, None)
}

/// A league's name and division for a club (`Tecnicos.campanhas()`), or its country.
fn league_of(t: &Translator, world: &World, campaigns: &std::collections::HashMap<ClubId, brasfoot_systems::awards::coaches::Campaign>, club: ClubId, long: bool) -> String {
    match campaigns.get(&club) {
        Some(k) => {
            let key = if long { "satellites-league-division-long" } else { "satellites-league-division" };
            t.text(key, Some(&args([("league", labels::league(t, &k.league)), ("division", k.division.to_string())])))
        }
        None => country_name(t, world.club(club).country),
    }
}

pub(crate) fn policy_key(p: SatellitePolicy) -> &'static str {
    match p {
        SatellitePolicy::YoungPlayers => "satellite-policy-young-players",
        SatellitePolicy::Promotion => "satellite-policy-promotion",
        SatellitePolicy::Profit => "satellite-policy-profit",
    }
}

fn role_key(r: LoanRole) -> &'static str {
    match r {
        LoanRole::Starter => "satellite-role-starter",
        LoanRole::Regular => "satellite-role-regular",
        LoanRole::Reserve => "satellite-role-reserve",
    }
}

pub fn window(s: &mut CareerScreen, state: &mut SatellitesState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(16, 12));
    motion::dialog(ctx, "satellites", open, vec2(1060.0, 710.0), frame, |ui, open| {
        header(ui, s, t);
        ui.add_space(8.0);
        let names: Vec<String> = TABS.iter().map(|k| t.text(k, None)).collect();
        if let Some(i) = controls::tabs(ui, &names, state.tab) {
            state.tab = i;
        }
        ui.add_space(8.0);
        let body = ui.available_height() - 40.0;
        ui.allocate_ui(vec2(ui.available_width(), body), |ui| {
            ui.set_min_height(body);
            match state.tab {
                0 => mine_tab(ui, s, state, t, files),
                1 => search_tab(ui, s, state, t, files),
                _ => networks_tab(ui, s, t, files),
            }
        });
        ui.add_space(6.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if controls::button(ui, &t.text("satellites-button-close", None), controls::Kind::Secondary, true).clicked() {
                *open = false;
            }
        });
    });
    if let Some(club) = state.assets_of {
        let mut open = true;
        assets_window(s, state, club, ctx, t, &mut open);
        if !open {
            state.assets_of = None;
        }
    }
    if let Some(i) = modkit::choice_overlay(ctx, t, "satellites", &mut state.choice) {
        match state.pending.take() {
            Some(Pending::Inject(sat)) => run(s, state, t, Action::Inject, sat, None, |m| m.inject_cash(sat, INJECTIONS[i])),
            Some(Pending::Withdraw(sat, available)) => {
                let amount = WITHDRAWALS.get(i).map_or(available, |&v| v.min(available));
                run(s, state, t, Action::Withdraw, sat, None, |m| m.withdraw_profit(sat, amount));
            }
            Some(Pending::Lend(p, sat)) => run(s, state, t, Action::Lend, sat, Some(p), |m| m.lend_to_satellite(p, sat, ROLES[i])),
            _ => {}
        }
    } else if state.choice.is_none() && matches!(state.pending, Some(Pending::Inject(_) | Pending::Withdraw(..) | Pending::Lend(..))) {
        state.pending = None;
    }
    if let Some(reply) = modkit::question_overlay(ctx, t, "satellites", &mut state.question) {
        let pending = state.pending.take();
        if reply == Reply::Yes {
            let mine = s.career.human_club();
            match pending {
                Some(Pending::Buy(sat) | Pending::BuyNew(sat)) => {
                    if let Some(me) = mine {
                        run(s, state, t, Action::Buy, sat, None, |m| m.buy_satellite(me, sat));
                        if matches!(pending, Some(Pending::BuyNew(_))) && state.question.is_none() {
                            state.tab = 0;
                            state.satellite = Some(sat);
                        }
                    }
                }
                Some(Pending::Partnership(sat)) => {
                    if let Some(me) = mine {
                        run(s, state, t, Action::Partnership, sat, None, |m| m.sign_partnership(me, sat));
                        if state.question.is_none() {
                            state.tab = 0;
                            state.satellite = Some(sat);
                        }
                    }
                }
                Some(Pending::End(sat)) => run(s, state, t, Action::End, sat, None, |m| m.end_partnership(sat)),
                Some(Pending::Sell(sat)) => run(s, state, t, Action::Sell, sat, None, |m| m.sell_satellite(sat)),
                Some(Pending::Recall(p)) => {
                    let sat = s.career.world.player(p).club.unwrap_or(ClubId(0));
                    run(s, state, t, Action::Recall, sat, Some(p), |m| m.recall(p));
                }
                Some(Pending::SendTo(p, sat)) => run(s, state, t, Action::SendTo, sat, Some(p), |m| m.send_to_satellite(p, sat)),
                Some(Pending::SignFrom(p, sat)) => run(s, state, t, Action::SignFrom, sat, Some(p), |m| m.sign_from_satellite(p, sat)),
                Some(Pending::Invest(sat, pillar)) => {
                    if let Some(me) = mine {
                        run(s, state, t, Action::Invest, sat, None, |m| m.invest_satellite_assets(me, sat, pillar));
                    }
                }
                _ => {}
            }
        }
    }
}

/// Runs an action of `Satelites`; a refusal shows its warning.
fn run(s: &mut CareerScreen, state: &mut SatellitesState, t: &Translator, action: Action, club: ClubId, player: Option<PlayerId>, f: impl FnOnce(&mut brasfoot_systems::Mod) -> Result<(), SatelliteError>) {
    let career = &mut s.career;
    let club_name = career.world.club(club).name.clone();
    let player_name = player.map(|p| career.world.player(p).name.clone()).unwrap_or_default();
    match career.systems.with(&mut career.world, &mut career.runtime, f) {
        Ok(()) => state.changed(),
        Err(e) => {
            let title = match action {
                Action::Buy => t.text("satellites-title-buy-failed", None),
                Action::Partnership => t.text("satellites-title-partnership-failed", None),
                _ => t.text("satellites-title-warning", None),
            };
            state.question = Some(modkit::message(title, error_text(t, action, &e, &club_name, &player_name)));
        }
    }
}

/// The header: the title, the club and its satellites, the fees and the world's links.
fn header(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator) {
    let career = &mut s.career;
    let mine = career.human_club();
    let (subtitle, summary) = match mine {
        None => (t.text("satellites-no-club", None), String::new()),
        Some(me) => career.systems.with(&mut career.world, &mut career.runtime, |m| {
            let sats = m.satellites_of(me);
            let (mut owned, mut partners, mut fees) = (0, 0, 0i64);
            for &c in &sats {
                if let Some(l) = m.satellite_link(c) {
                    if l.owned {
                        owned += 1;
                    } else {
                        partners += 1;
                        fees += l.yearly_fee;
                    }
                }
            }
            let links = m.world.mods.satellites.as_ref().map_or(0, |x| x.links.len());
            let w = &*m.world;
            (
                t.text(
                    "satellites-subtitle",
                    Some(&args([("club", w.club(me).name.clone()), ("cash", money(t, w.club(me).cash)), ("count", sats.len().to_string()), ("owned", owned.to_string()), ("partners", partners.to_string())])),
                ),
                t.text("satellites-summary", Some(&args([("fees", money(t, fees)), ("links", links.to_string())]))),
            )
        }),
    };
    modkit::title(ui, &t.text("satellites-header", None), 14.0);
    ui.add_space(3.0);
    modkit::strong(ui, &subtitle, 14.0, Color32::WHITE);
    if !summary.is_empty() {
        modkit::text(ui, &summary, 12.0, theme::MUTED);
    }
}

// Tab 1: my satellites

fn mine_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut SatellitesState, t: &Translator, files: &GameFiles) {
    let Some(me) = s.career.human_club() else {
        modkit::text(ui, &t.text("satellites-take-a-club", None), 13.0, theme::MUTED);
        return;
    };
    let career = &mut s.career;
    let config = career.systems.config.clone();
    let campaigns = brasfoot_systems::awards::coaches::campaigns(&career.world);
    // what the tables need, read through the system
    struct View {
        sats: Vec<(ClubId, Option<SatelliteLink>)>,
        link: Option<SatelliteLink>,
        suspended: bool,
        tactic: SatelliteTactic,
        fortune: i64,
        resale: i64,
        estimate: i64,
        buy_price: i64,
        loaned: Vec<(PlayerId, Option<sys::MinutesStatus>)>,
        pro_costs: Vec<(PlayerId, i64, bool, bool)>,
        junior_costs: Vec<(PlayerId, i64)>,
    }
    if state.satellite.is_some_and(|c| !career.systems.with(&mut career.world, &mut career.runtime, |m| m.satellites_of(me)).contains(&c)) {
        state.satellite = None;
    }
    let v = career.systems.with(&mut career.world, &mut career.runtime, |m| {
        let sats: Vec<(ClubId, Option<SatelliteLink>)> = m.satellites_of(me).into_iter().map(|c| (c, m.satellite_link(c))).collect();
        let sat = state.satellite.or(sats.first().map(|x| x.0));
        let link = sat.and_then(|c| m.satellite_link(c));
        let mut view = View {
            sats,
            link: link.clone(),
            suspended: false,
            tactic: SatelliteTactic::default(),
            fortune: 0,
            resale: 0,
            estimate: 0,
            buy_price: 0,
            loaned: Vec::new(),
            pro_costs: Vec::new(),
            junior_costs: Vec::new(),
        };
        if let (Some(sat), Some(_)) = (sat, &link) {
            view.suspended = m.suspended(sat);
            view.tactic = m.satellite_tactic(sat);
            view.fortune = m.fortune(sat);
            view.resale = m.resale_value(sat);
            view.estimate = m.estimate(sat);
            view.buy_price = m.buy_price(sat);
            let loans: Vec<PlayerId> = m.world.game.loans.iter().filter(|l| l.owner == Some(me)).filter_map(|l| l.player).filter(|&p| m.world.player(p).club == Some(sat)).collect();
            view.loaned = loans.into_iter().map(|p| (p, m.minutes_status(p))).collect();
            let mut pros = m.world.club(sat).squad.clone();
            pros.sort_by(|&a, &b| m.world.player(b).strength.cmp(&m.world.player(a).strength));
            view.pro_costs = pros
                .into_iter()
                .map(|p| {
                    let loan = brasfoot_systems::loans::loan_of(m.world, p).map(|i| m.world.game.loans[i].owner);
                    (p, m.satellite_signing_cost(p, sat), loan.is_some(), loan.flatten() == Some(me))
                })
                .collect();
            let mut juniors = m.world.club(sat).youth_squad.clone();
            juniors.sort_by(|&a, &b| m.world.player(b).strength.cmp(&m.world.player(a).strength));
            view.junior_costs = juniors.into_iter().map(|p| (p, m.satellite_signing_cost(p, sat))).collect();
        }
        state.satellite = sat;
        view
    });
    let world = &career.world;
    let sat = state.satellite;
    let mut tactic_changed: Option<SatelliteTactic> = None;
    let mut policy_changed: Option<SatellitePolicy> = None;
    let mut button: Option<&'static str> = None;
    modkit::item(None, None).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(t.text("satellites-label-satellite", None)).font(theme::bold(12.0)).color(GOLD));
            let shown = |c: ClubId, l: &Option<SatelliteLink>| {
                let tag = if l.as_ref().is_some_and(|l| l.owned) { t.text("satellites-tag-owned", None) } else { t.text("satellites-tag-partnership", None) };
                t.text("satellites-combo-item", Some(&args([("club", world.club(c).name.clone()), ("tag", tag), ("league", league_of(t, world, &campaigns, c, true))])))
            };
            let selected = v.sats.iter().find(|x| Some(x.0) == sat).map(|x| shown(x.0, &x.1)).unwrap_or_default();
            egui::ComboBox::from_id_salt("satellites-combo").width(300.0).selected_text(selected).show_ui(ui, |ui| {
                for (c, l) in &v.sats {
                    if ui.selectable_label(Some(*c) == sat, shown(*c, l)).clicked() {
                        state.satellite = Some(*c);
                        state.left = None;
                        state.pros = None;
                        state.juniors = None;
                    }
                }
            });
            match &v.link {
                None => {
                    ui.label(egui::RichText::new(t.text("satellites-none", None)).font(theme::bold(12.0)).color(theme::MUTED));
                }
                Some(l) => {
                    let link = if l.owned { t.text("satellites-status-owned", None) } else { t.text("satellites-status-partnership", Some(&args([("year", l.until.to_string())]))) };
                    let (text, color) = if v.suspended {
                        (t.text("satellites-status-suspended", Some(&args([("link", link)]))), RED)
                    } else {
                        (t.text("satellites-status-active", Some(&args([("link", link), ("year", l.since.to_string())]))), if l.owned { GREEN } else { GOLD })
                    };
                    ui.label(egui::RichText::new(text).font(theme::bold(12.0)).color(color));
                    ui.label(egui::RichText::new(t.text("satellites-label-policy", None)).font(theme::bold(12.0)).color(GOLD));
                    ui.add_enabled_ui(l.owned, |ui| {
                        egui::ComboBox::from_id_salt("satellites-policy").width(200.0).selected_text(t.text(policy_key(l.policy), None)).show_ui(ui, |ui| {
                            for p in POLICIES {
                                if ui.selectable_label(p == l.policy, t.text(policy_key(p), None)).clicked() {
                                    policy_changed = Some(p);
                                }
                            }
                        });
                    })
                    .response
                    .on_hover_text(t.text(&format!("{}-description", policy_key(l.policy)), None));
                }
            }
        });
        let Some(l) = &v.link else { return };
        if l.owned {
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(t.text("satellites-label-tactic", None)).font(theme::bold(12.0)).color(GOLD));
                let mut tactic = v.tactic;
                // espelhar: the combos show the parent's tactic and are off
                let parent_tactics = &world.club(me).tactics;
                if tactic.mirror && parent_tactics.len() >= 4 {
                    tactic.style = parent_tactics[1].clamp(0, 2);
                    tactic.marking = parent_tactics[2].clamp(0, 2);
                    tactic.side = parent_tactics[3].clamp(0, 1);
                }
                let mut mirror = tactic.mirror;
                if crate::screens::menu::check_row(ui, &mut mirror, &t.text("satellites-check-mirror", None), true) {
                    tactic.mirror = mirror;
                    tactic_changed = Some(tactic);
                }
                for (label, keys, value, id) in [("satellites-label-style", &STYLES[..], tactic.style, 0), ("satellites-label-marking", &MARKINGS[..], tactic.marking, 1), ("satellites-label-focus", &SIDES[..], tactic.side, 2)] {
                    ui.label(egui::RichText::new(t.text(label, None)).font(theme::regular(12.0)).color(theme::MUTED));
                    ui.add_enabled_ui(!tactic.mirror, |ui| {
                        egui::ComboBox::from_id_salt(("satellites-tactic", id)).width(130.0).selected_text(t.text(keys[value.clamp(0, keys.len() as i32 - 1) as usize], None)).show_ui(ui, |ui| {
                            for (i, k) in keys.iter().enumerate() {
                                if ui.selectable_label(i as i32 == value, t.text(k, None)).clicked() {
                                    let mut changed = tactic;
                                    match id {
                                        0 => changed.style = i as i32,
                                        1 => changed.marking = i as i32,
                                        _ => changed.side = i as i32,
                                    }
                                    tactic_changed = Some(changed);
                                }
                            }
                        });
                    });
                }
            });
        }
        ui.add_space(3.0);
        ui.horizontal_wrapped(|ui| {
            let sat = sat.expect("a link has its club");
            let name = world.club(sat).name.clone();
            let line = if l.owned {
                t.text("satellites-finance-owned", Some(&args([("club", name), ("cash", money(t, v.fortune)), ("resale", money(t, v.resale))])))
            } else {
                t.text("satellites-finance-partnership", Some(&args([("cash", format::approximate(t, v.estimate)), ("fee", money(t, l.yearly_fee)), ("price", money(t, v.buy_price))])))
            };
            ui.label(egui::RichText::new(line).font(theme::regular(12.0)).color(theme::MUTED));
            let buttons: &[(&str, bool)] = &[
                ("satellites-button-assets", l.owned),
                ("satellites-button-inject", l.owned),
                ("satellites-button-withdraw", l.owned),
                ("satellites-button-buy", !l.owned),
                ("satellites-button-renew", !l.owned),
                ("satellites-button-end", !l.owned),
                ("satellites-button-sell", l.owned),
            ];
            for &(key, visible) in buttons {
                if visible && controls::button(ui, &t.text(key, None), controls::Kind::Secondary, true).clicked() {
                    button = Some(key);
                }
            }
            if controls::button(ui, &t.text("satellites-button-new", None), controls::Kind::Secondary, true).clicked() {
                button = Some("satellites-button-new");
            }
        });
    });
    if v.link.is_none() && controls::button(ui, &t.text("satellites-button-new", None), controls::Kind::Secondary, true).clicked() {
        button = Some("satellites-button-new");
    }
    ui.add_space(8.0);
    // the tables
    let sat_name = sat.map(|c| world.club(c).name.clone()).unwrap_or_default();
    let suspended = v.suspended;
    let owned = v.link.as_ref().is_some_and(|l| l.owned);
    let has = v.link.is_some();
    let mut left_list: Vec<PlayerId> = v.loaned.iter().map(|x| x.0).collect();
    let mut left_rows: Vec<Vec<Cell>> = v
        .loaned
        .iter()
        .map(|(p, status)| {
            let x = world.player(*p);
            let st = status.map(|st| {
                let warn = if st.below { t.text("satellites-minutes-warning", None) } else { String::new() };
                t.text("satellites-minutes", Some(&args([("role", t.text(role_key(st.role), None)), ("played", st.played.to_string()), ("total", st.total.to_string()), ("percent", st.percent.to_string()), ("warning", warn)])))
            });
            vec![
                Cell::flagged(&x.name, x.country),
                Cell::from(widgets::position_name(t, x.position)),
                Cell::from(x.age),
                Cell::from(x.strength),
                Cell::from(brasfoot_systems::market::value(world, *p)),
                Cell::from(st.unwrap_or_else(|| "-".to_owned())),
                Cell::from(money(t, x.market_value as i64)),
            ]
        })
        .collect();
    if has {
        let mut squad = world.club(me).squad.clone();
        squad.sort_by(|&a, &b| world.player(a).position.cmp(&world.player(b).position).then(world.player(b).strength.cmp(&world.player(a).strength)));
        for p in squad {
            let x = world.player(p);
            let situation = if brasfoot_systems::loans::loan_of(world, p).is_some() {
                t.text("satellites-situation-loaned-in", None)
            } else if x.age <= 23 {
                t.text("satellites-situation-young", None)
            } else {
                t.text("satellites-situation-squad", None)
            };
            left_list.push(p);
            left_rows.push(vec![
                Cell::flagged(&x.name, x.country),
                Cell::from(widgets::position_name(t, x.position)),
                Cell::from(x.age),
                Cell::from(x.strength),
                Cell::from(brasfoot_systems::market::value(world, p)),
                Cell::from(situation),
                Cell::from(money(t, x.market_value as i64)),
            ]);
        }
    }
    let pro_rows: Vec<Vec<Cell>> = v
        .pro_costs
        .iter()
        .map(|&(p, cost, loan, mine_loan)| {
            let x = world.player(p);
            let price = if loan {
                t.text(if mine_loan { "satellites-cost-your-loanee" } else { "satellites-cost-loanee" }, None)
            } else if owned {
                t.text("satellites-cost-group", None)
            } else {
                t.text("satellites-cost-discount", Some(&args([("amount", money(t, cost))])))
            };
            vec![
                Cell::flagged(&x.name, x.country),
                Cell::from(widgets::position_name(t, x.position)),
                Cell::from(x.age),
                Cell::from(x.strength),
                Cell::from(brasfoot_systems::market::value(world, p)),
                Cell::from(money(t, x.market_value as i64)),
                Cell::from(price),
            ]
        })
        .collect();
    let junior_rows: Vec<Vec<Cell>> = v
        .junior_costs
        .iter()
        .map(|&(p, cost)| {
            let x = world.player(p);
            let report = academy::youth_report(world, &config, p);
            let label = report.map(|r| {
                t.text(
                    match r.label {
                        YouthLabel::Star => "youth-report-star",
                        YouthLabel::Promise => "youth-report-promise",
                        YouthLabel::Squad => "youth-report-squad",
                        YouthLabel::Expendable => "youth-report-expendable",
                    },
                    None,
                )
            });
            let price = if owned { t.text("satellites-cost-group", None) } else { t.text("satellites-cost-discount", Some(&args([("amount", money(t, cost))]))) };
            vec![
                Cell::flagged(&x.name, x.country),
                Cell::from(widgets::position_name(t, x.position)),
                Cell::from(x.age),
                Cell::from(report.map_or(x.strength, |r| r.strength)),
                Cell::from(report.map_or("-".to_owned(), |r| t.text("youth-report-range", Some(&args([("low", r.low), ("high", r.high)]))))),
                Cell::from(label.unwrap_or_else(|| "-".to_owned())),
                Cell::from(price),
            ]
        })
        .collect();
    let height = ui.available_height();
    let mut table_button: Option<&'static str> = None;
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        modkit::strong(ui, &t.text("satellites-left-title", Some(&args([("club", world.club(me).name.clone())]))), 13.0, GOLD);
        let cols_left = modkit::columns(
            t,
            &[
                ("satellites-column-parent-player", 185.0),
                ("satellites-column-position", 50.0),
                ("satellites-column-age", 32.0),
                ("satellites-column-strength", 42.0),
                ("satellites-column-pitch", 46.0),
                ("satellites-column-situation", 110.0),
                ("satellites-column-value", 75.0),
            ],
        );
        modkit::select_table(ui, files, "satellites-left", &cols_left, &left_rows, &mut state.left, height - 70.0);
        ui.horizontal_wrapped(|ui| {
            if controls::button(ui, &t.text("satellites-button-lend", None), controls::Kind::Secondary, has && !suspended).clicked() {
                table_button = Some("lend");
            }
            if controls::button(ui, &t.text("satellites-button-recall", None), controls::Kind::Secondary, has).clicked() {
                table_button = Some("recall");
            }
            if controls::button(ui, &t.text("satellites-button-send", None), controls::Kind::Secondary, owned && !suspended).clicked() {
                table_button = Some("send");
            }
        });
        let ui = &mut cols[1];
        let half = (height - 60.0) / 2.0;
        modkit::strong(ui, &t.text("satellites-pros-title", Some(&args([("club", sat_name.clone()), ("count", pro_rows.len().to_string())]))), 13.0, GOLD);
        let cols_pro = modkit::columns(
            t,
            &[
                ("satellites-column-satellite-pro", 175.0),
                ("satellites-column-position", 48.0),
                ("satellites-column-age", 32.0),
                ("satellites-column-strength", 42.0),
                ("satellites-column-pitch", 46.0),
                ("satellites-column-value", 75.0),
                ("satellites-column-cost-parent", 95.0),
            ],
        );
        modkit::select_table(ui, files, "satellites-pros", &cols_pro, &pro_rows, &mut state.pros, half - 40.0);
        if controls::button(ui, &t.text("satellites-button-sign", None), controls::Kind::Secondary, has && !suspended).clicked() {
            table_button = Some("sign");
        }
        ui.add_space(4.0);
        modkit::strong(ui, &t.text("satellites-juniors-title", Some(&args([("club", sat_name.clone()), ("count", junior_rows.len().to_string())]))), 13.0, GOLD);
        let cols_jr = modkit::columns(
            t,
            &[
                ("satellites-column-junior", 175.0),
                ("satellites-column-position", 48.0),
                ("satellites-column-age", 32.0),
                ("satellites-column-current-strength", 65.0),
                ("satellites-column-potential", 95.0),
                ("satellites-column-rating", 110.0),
                ("satellites-column-cost", 80.0),
            ],
        );
        modkit::select_table(ui, files, "satellites-juniors", &cols_jr, &junior_rows, &mut state.juniors, half - 40.0);
        ui.horizontal(|ui| {
            if controls::button(ui, &t.text("satellites-button-pull-academy", None), controls::Kind::Secondary, has && !suspended).clicked() {
                table_button = Some("pull-academy");
            }
            if controls::button(ui, &t.text("satellites-button-pull-pro", None), controls::Kind::Secondary, has && !suspended).clicked() {
                table_button = Some("pull-pro");
            }
        });
    });
    // the changes and the buttons
    let Some(sat) = sat else {
        if button == Some("satellites-button-new") {
            state.tab = 1;
        }
        return;
    };
    let career = &mut s.career;
    if let Some(p) = policy_changed {
        career.systems.with(&mut career.world, &mut career.runtime, |m| m.set_satellite_policy(sat, p));
    }
    if let Some(tc) = tactic_changed {
        career.systems.with(&mut career.world, &mut career.runtime, |m| m.set_satellite_tactic(sat, tc));
    }
    let world = &career.world;
    let me_name = world.club(me).name.clone();
    let name = world.club(sat).name.clone();
    match button {
        Some("satellites-button-new") => state.tab = 1,
        Some("satellites-button-assets") => state.assets_of = Some(sat),
        Some("satellites-button-inject") => {
            state.choice = Some(Choice {
                title: t.text("satellites-inject-title", None),
                text: t.text("satellites-inject-text", Some(&args([("parent", me_name), ("club", name)]))),
                options: INJECTIONS.iter().map(|&v| t.text("satellites-amount-option", Some(&args([("amount", format::thousands(t, v))])))).collect(),
            });
            state.pending = Some(Pending::Inject(sat));
        }
        Some("satellites-button-withdraw") => {
            let available = (v.fortune - 5_000_000).max(0);
            if available < 2_000_000 {
                state.question = Some(modkit::message(t.text("satellites-title-warning", None), t.text("satellites-withdraw-nothing", None)));
            } else {
                let mut options: Vec<String> = WITHDRAWALS.iter().map(|&v| t.text("satellites-amount-option", Some(&args([("amount", format::thousands(t, v))])))).collect();
                options.push(t.text("satellites-withdraw-all", Some(&args([("amount", money(t, available))]))));
                state.choice = Some(Choice { title: t.text("satellites-withdraw-title", None), text: t.text("satellites-withdraw-text", Some(&args([("club", name)]))), options });
                state.pending = Some(Pending::Withdraw(sat, available));
            }
        }
        Some("satellites-button-buy") => {
            state.question = Some(modkit::confirm(t.text("satellites-buy-title", None), t.text("satellites-buy-text", Some(&args([("club", name), ("price", money(t, v.buy_price))])))));
            state.pending = Some(Pending::Buy(sat));
        }
        Some("satellites-button-renew") => run(s, state, t, Action::Renew, sat, None, |m| m.renew_partnership(sat)),
        Some("satellites-button-end") => {
            let fee = v.link.as_ref().map_or(0, |l| l.yearly_fee);
            state.question = Some(modkit::confirm(t.text("satellites-end-title", None), t.text("satellites-end-text", Some(&args([("club", name), ("fine", money(t, fee))])))));
            state.pending = Some(Pending::End(sat));
        }
        Some("satellites-button-sell") => {
            state.question = Some(modkit::confirm(t.text("satellites-sell-title", None), t.text("satellites-sell-text", Some(&args([("club", name), ("price", money(t, v.resale)), ("parent", me_name)])))));
            state.pending = Some(Pending::Sell(sat));
        }
        _ => {}
    }
    let world = &s.career.world;
    let pick_left = state.left.and_then(|i| left_list.get(i).copied());
    let pick_pro = state.pros.and_then(|i| v.pro_costs.get(i).map(|x| x.0));
    let pick_junior = state.juniors.and_then(|i| v.junior_costs.get(i).map(|x| x.0));
    let info = |key: &str| modkit::message(t.text("satellites-title-warning", None), t.text(key, None));
    match table_button {
        Some("lend") => match pick_left {
            None => state.question = Some(info("satellites-pick-parent-player")),
            Some(p) => {
                state.choice = Some(Choice {
                    title: t.text("satellites-role-title", None),
                    text: t.text("satellites-role-text", Some(&args([("player", world.player(p).name.clone()), ("club", world.club(sat).name.clone())]))),
                    options: ROLES.iter().map(|&r| t.text(&format!("{}-option", role_key(r)), None)).collect(),
                });
                state.pending = Some(Pending::Lend(p, sat));
            }
        },
        Some("recall") => match pick_left.or(pick_pro) {
            None => state.question = Some(info("satellites-pick-loanee")),
            Some(p) => {
                let meets = s.career.systems.with(&mut s.career.world, &mut s.career.runtime, |m| m.meets_minutes(p));
                if meets {
                    run(s, state, t, Action::Recall, sat, Some(p), |m| m.recall(p));
                } else {
                    let world = &s.career.world;
                    let at = world.player(p).club.map(|c| world.club(c).name.clone()).unwrap_or_else(|| t.text("satellites-the-satellite", None));
                    state.question = Some(modkit::confirm(t.text("satellites-recall-title", None), t.text("satellites-recall-text", Some(&args([("player", world.player(p).name.clone()), ("club", at)])))));
                    state.pending = Some(Pending::Recall(p));
                }
            }
        },
        Some("send") => match pick_left {
            None => state.question = Some(info("satellites-pick-parent-player")),
            Some(p) => {
                state.question = Some(modkit::confirm(t.text("satellites-send-title", None), t.text("satellites-send-text", Some(&args([("player", world.player(p).name.clone()), ("club", world.club(sat).name.clone())])))));
                state.pending = Some(Pending::SendTo(p, sat));
            }
        },
        Some("sign") => match pick_pro {
            None => state.question = Some(info("satellites-pick-pro")),
            Some(p) => {
                let cost = v.pro_costs.iter().find(|x| x.0 == p).map_or(0, |x| x.1);
                let price = if cost == 0 { t.text("satellites-sign-free", None) } else { t.text("satellites-sign-discount", Some(&args([("amount", money(t, cost))]))) };
                let text = t.text(
                    "satellites-sign-text",
                    Some(&args([("player", world.player(p).name.clone()), ("strength", world.player(p).strength.to_string()), ("club", world.club(sat).name.clone()), ("parent", world.club(me).name.clone()), ("price", price)])),
                );
                state.question = Some(modkit::confirm(t.text("satellites-sign-title", None), text));
                state.pending = Some(Pending::SignFrom(p, sat));
            }
        },
        Some(which @ ("pull-academy" | "pull-pro")) => match pick_junior {
            None => state.question = Some(info("satellites-pick-junior")),
            Some(p) => {
                let professional = which == "pull-pro";
                run(s, state, t, Action::PullYouth, sat, Some(p), |m| m.pull_youth(p, sat, professional));
            }
        },
        _ => {}
    }
}

// Tab 2: the search

struct Found {
    club: ClubId,
    fee: i64,
    price: i64,
    estimate: i64,
    situation: String,
}

fn search_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut SatellitesState, t: &Translator, files: &GameFiles) {
    let mine = s.career.human_club();
    let world = &s.career.world;
    // the countries of the national leagues, by name
    let mut countries: Vec<(String, i32)> = brasfoot_systems::awards::clubs::league_clubs(world).iter().map(|&c| world.club(c).country).collect::<std::collections::BTreeSet<i32>>().into_iter().map(|k| (country_name(t, k), k)).collect();
    countries.sort();
    countries.dedup();
    let mut changed = false;
    modkit::item(None, None).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(t.text("satellites-filter-country", None)).font(theme::bold(12.0)).color(GOLD));
            let mut options: Vec<(i32, String)> = vec![(-1, t.text("satellites-country-all", None))];
            if let Some(me) = mine {
                let k = world.club(me).country;
                options.push((k, t.text("satellites-country-mine", Some(&args([("country", country_name(t, k))])))));
            }
            options.extend(countries.iter().map(|(n, k)| (*k, n.clone())));
            let shown = options.iter().find(|o| o.0 == state.country).map(|o| o.1.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt("satellites-country").width(190.0).height(400.0).selected_text(shown).show_ui(ui, |ui| {
                for (k, n) in &options {
                    if ui.selectable_label(state.country == *k, n).clicked() {
                        state.country = *k;
                        changed = true;
                    }
                }
            });
            let combos: [(&str, &str, &[&str], &mut usize); 2] = [
                ("satellites-filter-size", "satellites-rep", &["satellites-rep-any", "satellites-rep-0-1", "satellites-rep-2", "satellites-rep-3", "satellites-rep-4"], &mut state.reputation),
                ("satellites-filter-order", "satellites-order", &["satellites-order-reputation", "satellites-order-price", "satellites-order-fee", "satellites-order-academy"], &mut state.order),
            ];
            for (label, id, keys, value) in combos {
                ui.label(egui::RichText::new(t.text(label, None)).font(theme::bold(12.0)).color(GOLD));
                egui::ComboBox::from_id_salt(id).width(190.0).selected_text(t.text(keys[*value], None)).show_ui(ui, |ui| {
                    for (i, k) in keys.iter().enumerate() {
                        if ui.selectable_label(*value == i, t.text(k, None)).clicked() {
                            *value = i;
                            changed = true;
                        }
                    }
                });
                if label == "satellites-filter-size" {
                    ui.label(egui::RichText::new(t.text("satellites-filter-filter", None)).font(theme::bold(12.0)).color(GOLD));
                    let keys = ["satellites-eligible", "satellites-all-clubs"];
                    let current = if state.eligible_only { 0 } else { 1 };
                    egui::ComboBox::from_id_salt("satellites-eligible").width(170.0).selected_text(t.text(keys[current], None)).show_ui(ui, |ui| {
                        for (i, k) in keys.iter().enumerate() {
                            if ui.selectable_label(current == i, t.text(k, None)).clicked() {
                                state.eligible_only = i == 0;
                                changed = true;
                            }
                        }
                    });
                }
            }
        });
    });
    if changed {
        state.found = None;
        state.found_selected = None;
    }
    // popular: the clubs of the leagues by the filters, at most 250 shown
    let career = &mut s.career;
    let rows: Vec<Found> = {
        let (country, reputation, eligible_only, order) = (state.country, state.reputation, state.eligible_only, state.order);
        let cached = state.found.clone();
        career.systems.with(&mut career.world, &mut career.runtime, |m| {
            let list = match cached {
                Some(list) => list,
                None => {
                    let mut list = Vec::new();
                    for c in brasfoot_systems::awards::clubs::league_clubs(m.world) {
                        if Some(c) == mine || brasfoot_systems::economy::is_human(m.world, c) || brasfoot_systems::economy::is_national_team(m.world, c) {
                            continue;
                        }
                        if country != -1 && m.world.club(c).country != country {
                            continue;
                        }
                        let rep = m.world.club(c).reputation;
                        let keep = match reputation {
                            1 => rep <= 1,
                            2 => rep == 2,
                            3 => rep == 3,
                            4 => rep == 4,
                            _ => true,
                        };
                        if !keep {
                            continue;
                        }
                        if eligible_only {
                            let ok = mine.is_some_and(|me| m.block_reason(me, c, false).is_none() || m.block_reason(me, c, true).is_none());
                            if !ok {
                                continue;
                            }
                        }
                        list.push(c);
                    }
                    let keys: std::collections::HashMap<ClubId, (i64, i64, usize, i32)> =
                        list.iter().map(|&c| (c, (m.buy_price(c), sys::partnership_fee(m.world, c), m.world.club(c).youth_squad.len(), m.world.club(c).reputation))).collect();
                    list.sort_by(|a, b| {
                        let (ka, kb) = (keys[a], keys[b]);
                        match order {
                            1 => ka.0.cmp(&kb.0),
                            2 => ka.1.cmp(&kb.1),
                            3 => kb.2.cmp(&ka.2).then(kb.3.cmp(&ka.3)),
                            _ => kb.3.cmp(&ka.3).then(kb.0.cmp(&ka.0)),
                        }
                    });
                    list
                }
            };
            let by_id = brasfoot_systems::economy::clubs_by_id(m.world);
            list.iter()
                .take(250)
                .map(|&c| {
                    let link = m.satellite_link(c);
                    let situation = if let Some(l) = &link {
                        let parent = by_id.get(&l.parent).map(|&p| m.world.club(p).name.clone()).unwrap_or_default();
                        t.text(if l.owned { "satellites-situation-group" } else { "satellites-situation-partner" }, Some(&args([("club", parent)])))
                    } else if !m.satellites_of(c).is_empty() {
                        t.text("satellites-situation-parent", None)
                    } else if m.has_owner(c) {
                        t.text("satellites-situation-billionaire", None)
                    } else if mine.is_some_and(|me| sys::same_division(m.world, me, c)) {
                        t.text("satellites-situation-same-division", None)
                    } else {
                        t.text("satellites-situation-available", None)
                    };
                    Found { club: c, fee: sys::partnership_fee(m.world, c), price: m.buy_price(c), estimate: m.estimate(c), situation }
                })
                .collect()
        })
    };
    if state.found.is_none() {
        state.found = Some(rows.iter().map(|r| r.club).collect());
    }
    let world = &career.world;
    let campaigns = brasfoot_systems::awards::coaches::campaigns(world);
    let cells: Vec<Vec<Cell>> = rows
        .iter()
        .map(|r| {
            let c = world.club(r.club);
            vec![
                Cell::flagged(&c.name, c.country),
                Cell::from(league_of(t, world, &campaigns, r.club, false)),
                Cell::from(c.reputation),
                Cell::from(t.text("satellites-squad-sizes", Some(&args([("pros", c.squad.len().to_string()), ("juniors", c.youth_squad.len().to_string())])))),
                Cell::from(format::approximate(t, r.estimate)),
                Cell::from(t.text("satellites-per-year", Some(&args([("amount", money(t, r.fee))])))),
                Cell::from(money(t, r.price)),
                Cell::from(&r.situation),
            ]
        })
        .collect();
    let columns = modkit::columns(
        t,
        &[
            ("satellites-column-club", 210.0),
            ("satellites-column-league", 175.0),
            ("satellites-column-reputation", 42.0),
            ("satellites-column-squad-academy", 85.0),
            ("satellites-column-cash", 95.0),
            ("satellites-column-fee", 115.0),
            ("satellites-column-price", 115.0),
            ("satellites-column-status", 155.0),
        ],
    );
    ui.add_space(6.0);
    let height = ui.available_height() - 56.0;
    modkit::select_table(ui, files, "satellites-found", &columns, &cells, &mut state.found_selected, height);
    ui.add_space(6.0);
    let mut action = None;
    modkit::item(None, None).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 400.0);
                modkit::text(ui, &t.text("satellites-search-tip", None), 12.0, theme::MUTED);
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if controls::button(ui, &t.text("satellites-button-buy-new", None), controls::Kind::Secondary, true).clicked() {
                    action = Some(true);
                }
                if controls::button(ui, &t.text("satellites-button-partnership", None), controls::Kind::Secondary, true).clicked() {
                    action = Some(false);
                }
            });
        });
    });
    if let Some(buy) = action {
        let picked = state.found_selected.and_then(|i| rows.get(i));
        match (mine, picked) {
            (Some(_), Some(r)) => {
                let name = world.club(r.club).name.clone();
                if buy {
                    state.question = Some(modkit::confirm(t.text("satellites-buy-new-title", None), t.text("satellites-buy-new-text", Some(&args([("club", name), ("price", money(t, r.price))])))));
                    state.pending = Some(Pending::BuyNew(r.club));
                } else {
                    state.question = Some(modkit::confirm(t.text("satellites-partnership-title", None), t.text("satellites-partnership-text", Some(&args([("club", name), ("fee", money(t, r.fee))])))));
                    state.pending = Some(Pending::Partnership(r.club));
                }
            }
            _ => {
                let key = if buy { "satellites-pick-club-buy" } else { "satellites-pick-club-partnership" };
                state.question = Some(modkit::message(t.text("satellites-title-warning", None), t.text(key, None)));
            }
        }
    }
}

// Tab 3: the world's networks and the history

fn networks_tab(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let campaigns = brasfoot_systems::awards::coaches::campaigns(world);
    let by_id = brasfoot_systems::economy::clubs_by_id(world);
    let mut links: Vec<SatelliteLink> = world.mods.satellites.as_ref().map(|x| x.links.clone()).unwrap_or_default();
    let human = |l: &SatelliteLink| by_id.get(&l.parent).is_some_and(|&c| brasfoot_systems::economy::is_human(world, c));
    let name = |id: i32| by_id.get(&id).map(|&c| world.club(c).name.clone()).unwrap_or_default();
    // humans first, then by the parent's name (String.compareTo: by UTF-16 units)
    links.sort_by(|a, b| human(b).cmp(&human(a)).then_with(|| name(a.parent).encode_utf16().cmp(name(b.parent).encode_utf16())));
    let rows: Vec<Vec<Cell>> = links
        .iter()
        .filter_map(|l| {
            let parent = *by_id.get(&l.parent)?;
            let sat = *by_id.get(&l.satellite)?;
            let mut parent_name = world.club(parent).name.clone();
            if human(l) {
                parent_name = t.text("satellites-you", Some(&args([("club", parent_name)])));
            }
            let kind = t.text(if l.owned { "satellites-kind-owned" } else { "satellites-kind-partnership" }, None);
            let policy = if l.owned { t.text(policy_key(l.policy), None) } else { t.text("satellites-fee-until", Some(&args([("fee", money(t, l.yearly_fee)), ("year", l.until.to_string())]))) };
            let status = t.text(if sys::same_division(world, parent, sat) { "satellites-network-suspended" } else { "satellites-network-active" }, None);
            Some(vec![
                Cell::flagged(parent_name, world.club(parent).country),
                Cell::flagged(&world.club(sat).name, world.club(sat).country),
                Cell::from(league_of(t, world, &campaigns, sat, false)),
                Cell::from(kind),
                Cell::from(policy),
                Cell::from(l.since),
                Cell::from(status),
            ])
        })
        .collect();
    let history: Vec<Vec<Cell>> = world
        .mods
        .satellites
        .as_ref()
        .map(|x| x.history.iter().rev().map(|n| vec![Cell::from(n.year), Cell::from(t.text(labels::satellite_event_key(&n.event), None)), Cell::from(n.parent.clone().unwrap_or_default()), Cell::from(n.satellite.clone().unwrap_or_default()), Cell::from(event_detail(t, &n.event))]).collect())
        .unwrap_or_default();
    let half = (ui.available_height() - 50.0) / 2.0;
    modkit::strong(ui, &t.text("satellites-networks-title", None), 13.0, GOLD);
    let columns = modkit::columns(
        t,
        &[
            ("satellites-column-parent", 200.0),
            ("satellites-column-satellite", 200.0),
            ("satellites-column-satellite-league", 175.0),
            ("satellites-column-link", 125.0),
            ("satellites-column-policy", 195.0),
            ("satellites-column-since", 55.0),
            ("satellites-column-status", 100.0),
        ],
    );
    let mut none = None;
    modkit::select_table(ui, files, "satellites-networks", &columns, &rows, &mut none, half);
    ui.add_space(8.0);
    modkit::strong(ui, &t.text("satellites-history-title", None), 13.0, GOLD);
    let columns = modkit::columns(t, &[("satellites-column-year", 55.0), ("satellites-column-operation", 135.0), ("satellites-column-parent-short", 185.0), ("satellites-column-satellite-short", 185.0), ("satellites-column-details", 450.0)]);
    let mut none = None;
    modkit::select_table(ui, files, "satellites-history", &columns, &history, &mut none, half);
}

/// `registrarHistorico`'s detail line.
fn event_detail(t: &Translator, e: &SatelliteEvent) -> String {
    let m = |v: i64| money(t, v);
    let free = |cost: i64, zero: &str| if cost == 0 { t.text(zero, None) } else { m(cost) };
    match e {
        SatelliteEvent::Partnership { fee } => t.text("satellites-detail-partnership", Some(&args([("fee", m(*fee))]))),
        SatelliteEvent::Renewal { until, fee } => t.text("satellites-detail-renewal", Some(&args([("year", until.to_string()), ("fee", m(*fee))]))),
        SatelliteEvent::Termination { fine } => {
            if *fine > 0 {
                t.text("satellites-detail-fine", Some(&args([("fine", m(*fine))])))
            } else {
                t.text("satellites-detail-contract-end", None)
            }
        }
        SatelliteEvent::Acquisition { price } => t.text("satellites-detail-bought", Some(&args([("price", m(*price))]))),
        SatelliteEvent::ClubSale { price } => t.text("satellites-detail-sold", Some(&args([("price", m(*price))]))),
        SatelliteEvent::CashInjection { amount } => t.text("satellites-detail-injection", Some(&args([("amount", m(*amount))]))),
        SatelliteEvent::ProfitWithdrawal { amount, net } => t.text("satellites-detail-withdrawal", Some(&args([("amount", m(*amount)), ("net", m(*net))]))),
        SatelliteEvent::Assets { pillar, level } => t.text("satellites-detail-assets", Some(&args([("pillar", labels::pillar(t, *pillar)), ("level", level.to_string())]))),
        SatelliteEvent::Loan { player, age, strength, role } => {
            t.text("satellites-detail-loan", Some(&args([("player", player.clone()), ("age", age.to_string()), ("strength", strength.to_string()), ("role", t.text(role_key(*role), None))])))
        }
        SatelliteEvent::Return { player } => t.text("satellites-detail-return", Some(&args([("player", player.clone())]))),
        SatelliteEvent::ToParent { player, strength, cost } => {
            let price = if *cost == 0 { t.text("satellites-detail-free-group", None) } else { t.text("satellites-detail-discount", Some(&args([("amount", m(*cost))]))) };
            t.text("satellites-detail-to-parent", Some(&args([("player", player.clone()), ("strength", strength.to_string()), ("price", price)])))
        }
        SatelliteEvent::Definitive { player, strength } => t.text("satellites-detail-definitive", Some(&args([("player", player.clone()), ("strength", strength.to_string())]))),
        SatelliteEvent::YouthPull { player, age, professional, cost } => {
            let to = t.text(if *professional { "satellites-detail-to-pro" } else { "satellites-detail-to-academy" }, None);
            t.text("satellites-detail-youth", Some(&args([("player", player.clone()), ("age", age.to_string()), ("to", to), ("price", free(*cost, "satellites-detail-free-group"))])))
        }
        SatelliteEvent::SellOn { amount, player, buyer } => {
            let player = player.clone().unwrap_or_else(|| t.text("satellites-detail-athlete", None));
            let buyer = buyer.clone().unwrap_or_else(|| t.text("satellites-detail-abroad", None));
            t.text("satellites-detail-sell-on", Some(&args([("amount", m(*amount)), ("player", player), ("buyer", buyer)])))
        }
        SatelliteEvent::AiAcquisition { price, policy } => t.text("satellites-detail-ai-bought", Some(&args([("price", m(*price)), ("policy", t.text(policy_key(*policy), None))]))),
        SatelliteEvent::AiPartnership { fee } => t.text("satellites-detail-ai-partnership", Some(&args([("fee", m(*fee))]))),
        SatelliteEvent::AiLoan { player, age } => match age {
            Some(age) => t.text("satellites-detail-ai-loan-age", Some(&args([("player", player.clone()), ("age", age.to_string())]))),
            None => t.text("satellites-detail-ai-loan", Some(&args([("player", player.clone())]))),
        },
        SatelliteEvent::AiPromotion { player, strength, cost } => {
            t.text("satellites-detail-ai-promotion", Some(&args([("player", player.clone()), ("strength", strength.to_string()), ("price", free(*cost, "satellites-detail-free"))])))
        }
        SatelliteEvent::AiCession { player } => t.text("satellites-detail-ai-cession", Some(&args([("player", player.clone())]))),
    }
}

/// `abrirDialogoPatrimonio`: the parent pays the satellite's infrastructure.
fn assets_window(s: &mut CareerScreen, state: &mut SatellitesState, sat: ClubId, ctx: &egui::Context, t: &Translator, open: &mut bool) {
    let Some(me) = s.career.human_club() else {
        *open = false;
        return;
    };
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(16, 12));
    let career = &mut s.career;
    let levels: Vec<(Pillar, i32)> = career.systems.with(&mut career.world, &mut career.runtime, |m| Pillar::ALL.iter().map(|&p| (p, m.pillar_level(sat, p))).collect());
    let world = &career.world;
    let mut invest = None;
    motion::dialog(ctx, "satellite-assets", open, vec2(840.0, 560.0), frame, |ui, open| {
        modkit::title(ui, &t.text("satellites-assets-header", None), 14.0);
        ui.add_space(4.0);
        modkit::strong(ui, &t.text("satellites-assets-subtitle", Some(&args([("club", world.club(sat).name.clone()), ("parent", world.club(me).name.clone()), ("cash", money(t, world.club(me).cash))]))), 13.0, Color32::WHITE);
        modkit::text(ui, &t.text("satellites-assets-intro", None), 12.0, theme::MUTED);
        ui.add_space(10.0);
        for &(p, level) in &levels {
            modkit::item(None, None).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - 260.0);
                        let name = if level == 0 { t.text("assets-pillar-arena-level-0", None) } else { labels::pillar_level(t, p, level) };
                        let title = t.text("satellites-assets-pillar", Some(&args([("pillar", labels::pillar(t, p)), ("level", level.to_string()), ("name", name)])));
                        modkit::strong(ui, &title, 13.0, if level >= 4 { GOLD } else { theme::TEXT });
                        modkit::text(ui, &labels::pillar_effect(t, p, level), 11.0, SOFT);
                        if level < MAX_LEVEL {
                            let next = t.text("satellites-assets-next", Some(&args([("name", labels::pillar_level(t, p, level + 1)), ("effect", labels::pillar_effect(t, p, level + 1))])));
                            ui.label(egui::RichText::new(next).font(theme::regular(11.0)).italics().color(Color32::from_rgb(160, 200, 240)));
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if level >= MAX_LEVEL {
                            controls::button(ui, &t.text("satellites-assets-max", None), controls::Kind::Secondary, false);
                        } else {
                            let cost = LEVEL_COST[(level + 1) as usize];
                            let label = t.text("satellites-assets-upgrade", Some(&args([("level", (level + 1).to_string()), ("cost", money(t, cost))])));
                            if controls::button(ui, &label, controls::Kind::Secondary, world.club(me).cash >= cost).clicked() {
                                invest = Some((p, level + 1, cost));
                            }
                        }
                    });
                });
            });
            ui.add_space(8.0);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
            if controls::button(ui, &t.text("satellites-button-close", None), controls::Kind::Secondary, true).clicked() {
                *open = false;
            }
        });
    });
    if let Some((p, next, cost)) = invest {
        let text = t.text(
            "satellites-assets-confirm",
            Some(&args([("cost", money(t, cost)), ("parent", world.club(me).name.clone()), ("pillar", labels::pillar(t, p)), ("club", world.club(sat).name.clone()), ("level", next.to_string()), ("name", labels::pillar_level(t, p, next))])),
        );
        state.question = Some(modkit::confirm(t.text("satellites-assets-confirm-title", None), text));
        state.pending = Some(Pending::Invest(sat, p));
    }
}
