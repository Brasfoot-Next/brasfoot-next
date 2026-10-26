//! Finances and the transfer market (`bfmod.FinancasDialog`, "Finanças e Mercado"): who is rich
//! as the press sees it, the master sponsors, the season's deals, the biggest spenders, and the
//! owners and investors.

use brasfoot_i18n::{Translator, args, labels};
use brasfoot_model::ids::ClubId;
use brasfoot_model::world::World;
use eframe::egui::{self, Color32, vec2};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::modkit::{self, Cell};
use crate::{format, motion, theme};

const TABS: [&str; 5] = ["market-tab-fortunes", "market-tab-sponsors", "market-tab-deals", "market-tab-spenders", "market-tab-owners"];

pub struct MarketState {
    tab: usize,
}

impl MarketState {
    pub fn new(tab: usize) -> Self {
        MarketState { tab: tab.min(TABS.len() - 1) }
    }
}

/// `Economia.faixa(long)`
fn wealth(t: &Translator, v: i64) -> String {
    const MI: i64 = 1_000_000;
    let key = match v {
        _ if v >= 1000 * MI => "finances-wealth-billionaire",
        _ if v >= 300 * MI => "finances-wealth-very-rich",
        _ if v >= 100 * MI => "finances-wealth-rich",
        _ if v >= 30 * MI => "finances-wealth-stable",
        0.. => "finances-wealth-tight",
        _ => "finances-wealth-crisis",
    };
    t.text(key, None)
}

/// The league of each club (`Tecnicos.campanhas()`'s `liga`).
fn leagues(t: &Translator, world: &World) -> std::collections::HashMap<ClubId, String> {
    brasfoot_systems::awards::coaches::campaigns(world).into_iter().map(|(c, k)| (c, labels::league(t, &k.league))).collect()
}

fn name_with_mine(t: &Translator, world: &World, club: ClubId) -> Cell {
    let c = world.club(club);
    let name = if brasfoot_systems::economy::is_human(world, club) { t.text("market-mine", Some(&args([("club", c.name.clone())]))) } else { c.name.clone() };
    Cell::flagged(name, c.country)
}

/// `Mercado.transferenciasDaTemporada()`: this season's deals with a fee, the biggest first.
struct Deal {
    date: String,
    player: Option<brasfoot_model::ids::PlayerId>,
    from: Option<ClubId>,
    to: Option<ClubId>,
    fee: i64,
}

fn season_deals(world: &World) -> Vec<Deal> {
    let year = brasfoot_systems::economy::year(world);
    let by_id = brasfoot_systems::economy::clubs_by_id(world);
    let mut out: Vec<Deal> = world
        .game
        .transfers
        .iter()
        .filter(|x| x.fee > 0 && x.year == year)
        .map(|x| Deal { date: format!("{}/{}/{}", x.day, x.month, x.year), player: x.player, from: by_id.get(&x.from_club).copied(), to: by_id.get(&x.to_club).copied(), fee: x.fee as i64 })
        .collect();
    // List.sort is stable: equal fees keep the history's order
    out.sort_by(|a, b| b.fee.cmp(&a.fee));
    out
}

pub fn window(s: &mut CareerScreen, state: &mut MarketState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(16, 12));
    motion::dialog(ctx, "market", open, vec2(980.0, 700.0), frame, |ui, open| {
        let world = &s.career.world;
        modkit::title(ui, &t.text("market-header", Some(&args([("year", brasfoot_systems::economy::year(world).to_string())]))), 15.0);
        ui.add_space(6.0);
        modkit::text(ui, &t.text("market-note", None), 12.0, theme::MUTED);
        if let Some(me) = s.career.human_club() {
            ui.add_space(8.0);
            modkit::strong(ui, &t.text("market-cash", Some(&args([("club", world.club(me).name.clone()), ("cash", format::money(t, world.club(me).cash))]))), 16.0, Color32::WHITE);
        }
        ui.add_space(8.0);
        let names: Vec<String> = TABS.iter().map(|k| t.text(k, None)).collect();
        if let Some(i) = modkit::tabs(ui, &names, state.tab) {
            state.tab = i;
        }
        ui.add_space(8.0);
        let body = ui.available_height() - 40.0;
        ui.allocate_ui(vec2(ui.available_width(), body), |ui| {
            ui.set_min_height(body);
            match state.tab {
                0 => fortunes(ui, s, t, files),
                1 => sponsors(ui, s, t, files),
                2 => deals(ui, s, t, files),
                3 => spenders(ui, s, t, files),
                _ => owners(ui, s, t, files),
            }
        });
        ui.add_space(6.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if modkit::button(ui, &t.text("market-button-close", None), false, true).clicked() {
                *open = false;
            }
        });
    });
}

fn table(ui: &mut egui::Ui, files: &GameFiles, id: &str, t: &Translator, specs: &[(&str, f32)], rows: &[Vec<Cell>]) {
    let mut none = None;
    modkit::select_table(ui, files, id, &modkit::columns(t, specs), rows, &mut none, ui.available_height());
}

fn fortunes(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator, files: &GameFiles) {
    let career = &mut s.career;
    let clubs = brasfoot_systems::awards::clubs::league_clubs(&career.world);
    let mut list: Vec<(ClubId, i64, bool, Option<ClubId>)> = career.systems.with(&mut career.world, &mut career.runtime, |m| {
        clubs
            .iter()
            .map(|&c| {
                let parent = m.satellite_link(c).filter(|l| l.owned).and_then(|_| m.parent_of(c));
                (c, m.estimate(c), m.has_owner(c), parent)
            })
            .collect()
    });
    list.sort_by(|a, b| b.1.cmp(&a.1));
    let world = &career.world;
    let leagues = leagues(t, world);
    let rows: Vec<Vec<Cell>> = list
        .iter()
        .take(150)
        .enumerate()
        .map(|(i, &(c, v, owner, parent))| {
            let mine = brasfoot_systems::economy::is_human(world, c);
            let owner = match parent {
                Some(p) => t.text("market-group", Some(&args([("club", world.club(p).name.clone())]))),
                None if owner => t.text("market-investor", None),
                None => String::new(),
            };
            vec![
                Cell::from(i + 1),
                name_with_mine(t, world, c),
                Cell::from(leagues.get(&c).cloned().unwrap_or_default()),
                Cell::from(wealth(t, v)),
                Cell::from(if mine { format::money(t, v) } else { format::approximate(t, v) }),
                Cell::from(owner),
            ]
        })
        .collect();
    table(
        ui,
        files,
        "market-fortunes",
        t,
        &[("market-column-rank", 40.0), ("market-column-club", 250.0), ("market-column-league", 190.0), ("market-column-wealth", 100.0), ("market-column-estimate", 120.0), ("market-column-owner", 130.0)],
        &rows,
    );
}

fn sponsors(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator, files: &GameFiles) {
    let career = &mut s.career;
    let clubs = brasfoot_systems::awards::clubs::league_clubs(&career.world);
    // Economia.patrocinador(ah): brand, kind, yearly, until
    struct Deal {
        club: ClubId,
        brand: String,
        kind: String,
        estimate: i64,
        until: i32,
    }
    let year = brasfoot_systems::economy::year(&career.world);
    let mut list: Vec<Deal> = career.systems.with(&mut career.world, &mut career.runtime, |m| {
        clubs
            .iter()
            .filter_map(|&c| {
                if brasfoot_systems::economy::is_human(m.world, c) {
                    let k = m.world.mods.master_sponsor.active()?.clone();
                    return Some(Deal { club: c, brand: brasfoot_model::brands::MASTER_BRANDS[k.brand as usize].to_owned(), kind: labels::master_kind(t, k.kind), estimate: k.yearly, until: year + k.years });
                }
                let e = m.club(c).clone();
                let brand = e.brand?;
                let estimate = m.sponsorship_estimate(c, e.sponsorship);
                Some(Deal { club: c, brand: brasfoot_model::brands::brand(brand), kind: labels::sponsorship_kind(t, e.sponsorship_kind), estimate, until: e.sponsorship_until })
            })
            .collect()
    });
    list.sort_by(|a, b| b.estimate.cmp(&a.estimate));
    let world = &career.world;
    let leagues = leagues(t, world);
    let mut mine_at = None;
    let mut rows = Vec::new();
    for (i, d) in list.iter().enumerate() {
        let mine = brasfoot_systems::economy::is_human(world, d.club);
        if mine {
            mine_at = Some(i + 1);
        }
        if i < 150 || mine {
            rows.push(vec![
                Cell::from(i + 1),
                name_with_mine(t, world, d.club),
                Cell::from(leagues.get(&d.club).cloned().unwrap_or_default()),
                Cell::from(&d.brand),
                Cell::from(&d.kind),
                Cell::from(if mine { format::mi(t, d.estimate) } else { format::approximate(t, d.estimate) }),
                Cell::from(d.until),
            ]);
        }
    }
    if rows.is_empty() {
        modkit::text(ui, &t.text("market-sponsors-none", None), 13.0, theme::MUTED);
        return;
    }
    let sub = match mine_at {
        Some(n) => t.text("market-sponsors-mine", Some(&args([("rank", n.to_string())]))),
        None => t.text("market-sponsors-no-master", None),
    };
    modkit::text(ui, &sub, 12.0, theme::MUTED);
    ui.add_space(6.0);
    table(
        ui,
        files,
        "market-sponsors",
        t,
        &[
            ("market-column-rank", 45.0),
            ("market-column-club", 240.0),
            ("market-column-league", 190.0),
            ("market-column-sponsor", 170.0),
            ("market-column-kind", 110.0),
            ("market-column-yearly", 100.0),
            ("market-column-until", 55.0),
        ],
        &rows,
    );
}

fn deals(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let list = season_deals(world);
    if list.is_empty() {
        modkit::text(ui, &t.text("market-deals-none", None), 13.0, theme::MUTED);
        return;
    }
    let name = |c: Option<ClubId>| c.map(|c| world.club(c).name.clone()).unwrap_or_else(|| "-".into());
    let rows: Vec<Vec<Cell>> = list
        .iter()
        .take(200)
        .map(|d| {
            let player = d.player.map(|p| Cell::flagged(&world.player(p).name, world.player(p).country)).unwrap_or_else(|| Cell::from("-"));
            vec![Cell::from(&d.date), player, Cell::from(name(d.from)), Cell::from(name(d.to)), Cell::from(format::mi(t, d.fee))]
        })
        .collect();
    table(ui, files, "market-deals", t, &[("market-column-date", 90.0), ("market-column-player", 220.0), ("market-column-from", 220.0), ("market-column-to", 220.0), ("market-column-fee", 120.0)], &rows);
}

fn spenders(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let mut spent: Vec<(ClubId, i64, i64)> = Vec::new();
    for d in season_deals(world) {
        if let Some(to) = d.to.filter(|&c| !brasfoot_systems::economy::is_national_team(world, c)) {
            match spent.iter_mut().find(|x| x.0 == to) {
                Some(x) => {
                    x.1 += d.fee;
                    x.2 += 1;
                }
                None => spent.push((to, d.fee, 1)),
            }
        }
    }
    spent.sort_by(|a, b| b.1.cmp(&a.1));
    if spent.is_empty() {
        modkit::text(ui, &t.text("market-spenders-none", None), 13.0, theme::MUTED);
        return;
    }
    let rows: Vec<Vec<Cell>> = spent
        .iter()
        .take(50)
        .enumerate()
        .map(|(i, &(c, fee, n))| vec![Cell::from(i + 1), Cell::flagged(&world.club(c).name, world.club(c).country), Cell::from(n), Cell::from(format::mi(t, fee))])
        .collect();
    table(ui, files, "market-spenders", t, &[("market-column-rank", 40.0), ("market-column-club", 320.0), ("market-column-signings", 120.0), ("market-column-spent", 160.0)], &rows);
}

fn owners(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator, files: &GameFiles) {
    let career = &mut s.career;
    struct Owned {
        club: ClubId,
        parent: Option<(ClubId, brasfoot_model::mods::SatelliteLink)>,
        owner: Option<brasfoot_model::mods::EconomyClub>,
        estimate: i64,
        mine: bool,
    }
    let clubs: Vec<ClubId> = career.world.game.clubs.clone();
    let mut list: Vec<Owned> = career.systems.with(&mut career.world, &mut career.runtime, |m| {
        let mut out = Vec::new();
        for c in clubs {
            if brasfoot_systems::economy::is_national_team(m.world, c) {
                continue;
            }
            let parent = m.satellite_link(c).filter(|l| l.owned).and_then(|l| m.parent_of(c).map(|p| (p, l)));
            let has_owner = m.has_owner(c);
            if has_owner || parent.is_some() {
                let mine = brasfoot_systems::economy::is_human(m.world, c) || parent.as_ref().is_some_and(|(p, _)| brasfoot_systems::economy::is_human(m.world, *p));
                let owner = (parent.is_none()).then(|| m.club(c).clone());
                out.push(Owned { club: c, parent, owner, estimate: m.estimate(c), mine });
            }
        }
        out
    });
    list.sort_by(|a, b| b.mine.cmp(&a.mine).then(b.estimate.cmp(&a.estimate)));
    let world = &career.world;
    let mut names: std::collections::HashSet<String> = list.iter().map(|o| world.club(o.club).name.clone()).collect();
    let mut rows: Vec<Vec<Cell>> = list
        .iter()
        .map(|o| {
            let c = world.club(o.club);
            match &o.parent {
                Some((p, l)) => {
                    let mut group = t.text("market-group", Some(&args([("club", world.club(*p).name.clone())])));
                    if brasfoot_systems::economy::is_human(world, *p) {
                        group = t.text("market-group-mine", Some(&args([("group", group)])));
                    }
                    let situation = t.text(
                        "market-owners-bought",
                        Some(&args([("year", l.since.to_string()), ("price", format::money(t, l.price_paid)), ("policy", t.text(crate::screens::satellites::policy_key(l.policy), None))])),
                    );
                    vec![Cell::flagged(&c.name, c.country), Cell::from(group), Cell::from(situation), Cell::from(wealth(t, o.estimate))]
                }
                None => {
                    let e = o.owner.clone().unwrap_or_default();
                    vec![
                        name_with_mine(t, world, o.club),
                        Cell::from(e.owner_name.map(brasfoot_model::brands::owner).unwrap_or_default()),
                        Cell::from(t.text("market-owners-since", Some(&args([("year", e.owner_since.to_string())])))),
                        Cell::from(wealth(t, o.estimate)),
                    ]
                }
            }
        })
        .collect();
    // the events are history: a club once, its latest event, only if it has no owner now
    if let Some(e) = world.mods.economy.as_ref() {
        for ev in e.events.iter().rev() {
            if names.insert(ev.club.clone()) {
                let key = if ev.arrived { "market-owners-new" } else { "market-owners-left" };
                rows.push(vec![Cell::from(&ev.club), Cell::default(), Cell::from(t.text(key, Some(&args([("year", ev.year.to_string())])))), Cell::default()]);
            }
        }
    }
    if rows.is_empty() {
        modkit::text(ui, &t.text("market-owners-none", None), 13.0, theme::MUTED);
        return;
    }
    table(ui, files, "market-owners", t, &[("market-column-club", 230.0), ("market-column-owner-group", 230.0), ("market-column-situation", 260.0), ("market-column-wealth", 100.0)], &rows);
}
