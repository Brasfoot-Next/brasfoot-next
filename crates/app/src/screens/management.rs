//! The club management (`bfmod.GestaoDialog`, "Gestão do clube"): the performance analysis and the
//! coaching staff (`bfmod.DesempenhoPainel`), the director of football's targets, the scouts, the
//! club's loanees, the master sponsor and the fans.

use brasfoot_i18n::{Translator, args, labels};
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::mods::{StaffMember, StaffRole};
use brasfoot_model::world::World;
use brasfoot_systems::performance::{Analysis, Item, Kind};
use brasfoot_systems::scouts::{self, ScoutError};
use eframe::egui::{self, Color32, Stroke, vec2};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::findings;
use crate::screens::form::{Question, Reply};
use crate::screens::modkit::{self, Cell, GOLD, GOLD_DARK};
use crate::screens::widgets;
use crate::{format, motion, theme};
use crate::screens::controls;

const TABS: [&str; 7] = [
    "management-tab-performance",
    "management-tab-staff",
    "management-tab-director",
    "management-tab-scouts",
    "management-tab-loans",
    "management-tab-sponsorship",
    "management-tab-fans",
];
const PROBLEM: Color32 = Color32::from_rgb(0xe6, 0x67, 0x67);
const ATTENTION: Color32 = Color32::from_rgb(0xfa, 0xb2, 0x19);
const STRONG: Color32 = Color32::from_rgb(0x4c, 0xc3, 0x8a);
const INFO: Color32 = Color32::from_rgb(0x39, 0x87, 0xe5);
const DRAW: Color32 = Color32::from_rgb(0xc8, 0xcc, 0xd6);
const ROLES: [StaffRole; 3] = [StaffRole::Analyst, StaffRole::Assistant, StaffRole::FitnessCoach];

#[derive(Clone, Copy)]
enum Pending {
    Hire(usize),
    Fire(StaffRole),
}

pub struct ManagementState {
    tab: usize,
    analysis: Option<Analysis>,
    candidates: Option<Vec<StaffMember>>,
    candidate: Option<usize>,
    targets: Option<Vec<brasfoot_systems::director::Target>>,
    target: Option<usize>,
    /// The scouts' filters and the last search.
    position: usize,
    max_age: i64,
    min_value: i64,
    max_price: i64,
    found: Option<Vec<scouts::Finding>>,
    found_selected: Option<usize>,
    question: Option<Question>,
    pending: Option<Pending>,
    /// "Buscar patrocinadores": the finances window on its sponsorship tab.
    pub open_sponsors: bool,
}

impl ManagementState {
    pub fn new() -> Self {
        ManagementState {
            tab: 0,
            analysis: None,
            candidates: None,
            candidate: None,
            targets: None,
            target: None,
            position: 0,
            max_age: 23,
            min_value: 55,
            max_price: 30,
            found: None,
            found_selected: None,
            question: None,
            pending: None,
            open_sponsors: false,
        }
    }

    pub fn select(&mut self, tab: usize) {
        self.tab = tab.min(TABS.len() - 1);
    }

    /// `atualizar()`: what the tabs show is worked out again.
    fn refresh(&mut self) {
        self.analysis = None;
        self.candidates = None;
        self.targets = None;
        self.found = None;
    }
}

impl Default for ManagementState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn window(s: &mut CareerScreen, state: &mut ManagementState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(16, 12));
    let mut offer: Option<PlayerId> = None;
    motion::dialog(ctx, "management", open, vec2(980.0, 700.0), frame, |ui, open| {
        // the header
        let world = &s.career.world;
        modkit::title(ui, &t.text("management-header", None), 15.0);
        if let Some(me) = s.career.human_club() {
            ui.add_space(6.0);
            let line = t.text("management-subtitle", Some(&args([("club", world.club(me).name.clone()), ("cash", format::money(t, world.club(me).cash)), ("fans", brasfoot_systems::fans::confidence(world).to_string())])));
            modkit::strong(ui, &line, 16.0, Color32::WHITE);
        }
        ui.add_space(8.0);
        let names: Vec<String> = TABS.iter().map(|k| t.text(k, None)).collect();
        if let Some(i) = controls::tabs(ui, &names, state.tab) {
            state.tab = i;
        }
        ui.add_space(8.0);
        let body = ui.available_height() - 40.0;
        ui.allocate_ui(vec2(ui.available_width(), body), |ui| {
            ui.set_min_height(body);
            let Some(me) = s.career.human_club() else {
                modkit::text(ui, &t.text("management-no-club", None), 13.0, theme::MUTED);
                return;
            };
            match state.tab {
                0 => performance_tab(ui, s, state, me, t, files),
                1 => staff_tab(ui, s, state, me, t, files),
                2 => offer = director_tab(ui, s, state, me, t, files),
                3 => offer = scouts_tab(ui, s, state, t, files),
                4 => loans_tab(ui, s, me, t, files),
                5 => sponsorship_tab(ui, s, state, t, files),
                _ => fans_tab(ui, s, t, files),
            }
        });
        ui.add_space(6.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if controls::button(ui, &t.text("management-button-close", None), controls::Kind::Secondary, true).clicked() {
                *open = false;
            }
        });
    });
    // Diretor.propor: the game's offer screen for the player
    if let Some(p) = offer {
        let world = &s.career.world;
        if world.player(p).club == s.career.human_club() {
            state.question = Some(modkit::message(String::new(), t.text("management-already-yours", None)));
        } else if world.player(p).on_loan {
            state.question = Some(modkit::message(String::new(), t.text("management-on-loan", None)));
        } else {
            s.offer_state = Some(crate::screens::offer::OfferState::new(world, p));
            s.offer_open = true;
            state.refresh();
        }
    }
    if let Some(reply) = modkit::question_overlay(ctx, t, "management", &mut state.question) {
        let pending = state.pending.take();
        if reply == Reply::Yes {
            let career = &mut s.career;
            match pending {
                Some(Pending::Hire(i)) => {
                    if let Some(x) = state.candidates.as_ref().and_then(|c| c.get(i)).cloned() {
                        hire(s, state, t, i, &x);
                    }
                }
                Some(Pending::Fire(role)) => {
                    career.systems.with(&mut career.world, &mut career.runtime, |m| m.fire_staff(role));
                    state.refresh();
                }
                None => {}
            }
        }
    }
}

fn hire(s: &mut CareerScreen, state: &mut ManagementState, t: &Translator, i: usize, x: &StaffMember) {
    let career = &mut s.career;
    match career.systems.hire_staff(&mut career.world, &mut career.runtime, i) {
        Ok(()) => {}
        Err(brasfoot_systems::staff::HireError::Unavailable) => state.question = Some(modkit::message(String::new(), t.text("management-staff-unavailable", None))),
        Err(brasfoot_systems::staff::HireError::Reputation { minimum }) => {
            let text = t.text("management-staff-reputation", Some(&args([("name", brasfoot_model::brands::staff(x.name)), ("reputation", minimum.to_string())])));
            state.question = Some(modkit::message(String::new(), text));
        }
    }
    state.refresh();
}

// Performance

fn kind_style(t: &Translator, kind: Kind) -> (Color32, String) {
    match kind {
        Kind::Problem => (PROBLEM, t.text("management-tag-problem", None)),
        Kind::Attention => (ATTENTION, t.text("management-tag-attention", None)),
        Kind::Strong => (STRONG, t.text("management-tag-strong", None)),
        Kind::Locked => (theme::MUTED, t.text("management-tag-locked", None)),
        Kind::Info => (INFO, t.text("management-tag-info", None)),
    }
}

/// `achado(Achado)`: the colored bar, the tag, the title and the text.
fn finding_card(ui: &mut egui::Ui, t: &Translator, kind: Kind, title: &str, text: &str) {
    let (color, tag) = kind_style(t, kind);
    let locked = kind == Kind::Locked;
    let fill = if locked { theme::PANEL } else { theme::ROW_ALT };
    let response = egui::Frame::new().fill(fill).inner_margin(egui::Margin { left: 11, right: 8, top: 6, bottom: 6 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let mut job = egui::text::LayoutJob::default();
        // the original's lock emoji, as the app's icon
        if locked {
            job.append(crate::glyphs::LOCK, 0.0, egui::TextFormat::simple(theme::icon_font(12.0), color));
            job.append(" ", 0.0, egui::TextFormat::simple(theme::bold(12.0), color));
        }
        job.append(&tag, 0.0, egui::TextFormat::simple(theme::bold(12.0), color));
        job.append(title, 10.0, egui::TextFormat::simple(theme::bold(12.0), if locked { theme::MUTED } else { Color32::WHITE }));
        ui.add(egui::Label::new(job).wrap());
        ui.add_space(2.0);
        modkit::text(ui, text, 11.0, if locked { theme::MUTED } else { theme::TEXT });
    });
    let r = response.response.rect;
    ui.painter().rect_filled(egui::Rect::from_min_size(r.min, vec2(3.0, r.height())), 0.0, color);
    ui.add_space(6.0);
}

fn section(ui: &mut egui::Ui, s: &CareerScreen, t: &Translator, title: &str, items: &[Item]) {
    if ui.min_rect().height() > 1.0 {
        ui.add_space(12.0);
    }
    ui.label(egui::RichText::new(title).font(theme::bold(12.0)).color(GOLD));
    ui.add_space(6.0);
    for i in items {
        let (title, text) = findings::text(t, &s.career.world, &s.career.runtime.clock, &i.finding);
        finding_card(ui, t, i.kind, &title, &text);
    }
}

fn kpi(ui: &mut egui::Ui, title: &str, value: egui::text::LayoutJob, detail: &str) {
    modkit::item(None, None).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(title).font(theme::bold(10.0)).color(theme::MUTED));
        ui.add_space(3.0);
        ui.label(value);
        ui.add_space(2.0);
        ui.label(egui::RichText::new(detail).font(theme::regular(11.0)).color(theme::MUTED));
    });
}

fn plain(text: &str) -> egui::text::LayoutJob {
    egui::text::LayoutJob::simple_singleline(text.to_owned(), theme::bold(18.0), Color32::WHITE)
}

fn performance_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut ManagementState, me: ClubId, t: &Translator, files: &GameFiles) {
    if state.analysis.is_none() {
        let career = &mut s.career;
        state.analysis = Some(career.systems.with(&mut career.world, &mut career.runtime, |m| m.analyse_performance()));
    }
    let Some(a) = state.analysis.clone() else { return };
    let _ = me;
    let recent: Vec<&brasfoot_systems::performance::Game> = a.window.iter().take(5).collect();
    ui.columns(4, |cols| {
        let mut form = egui::text::LayoutJob::default();
        for g in recent.iter().rev() {
            let r = g.result();
            let color = match r {
                'V' => STRONG,
                'E' => DRAW,
                _ => PROBLEM,
            };
            form.append(&format!("{} ", t.text(&format!("management-result-{r}"), None)), 0.0, egui::TextFormat::simple(theme::bold(18.0), color));
        }
        if form.text.is_empty() {
            form = plain(" ");
        }
        kpi(&mut cols[0], &t.text("management-kpi-form", Some(&args([("games", recent.len().to_string())]))), form, &t.text("management-kpi-form-detail", None));
        let before = if a.ppg_before >= 0.0 { t.text("management-kpi-ppg-before", Some(&args([("before", format::decimal(t, a.ppg_before, 1))]))) } else { t.text("management-kpi-ppg-first", None) };
        kpi(&mut cols[1], &t.text("management-kpi-ppg", None), plain(&format::decimal(t, a.ppg_recent, 1)), &before);
        let (scored, conceded): (i32, i32) = a.window.iter().fold((0, 0), |(f, c), g| (f + g.scored, c + g.conceded));
        let diff = scored - conceded;
        kpi(
            &mut cols[2],
            &t.text("management-kpi-goals", Some(&args([("games", a.window.len().to_string())]))),
            plain(&t.text("management-kpi-goals-value", Some(&args([("for", scored.to_string()), ("against", conceded.to_string())])))),
            &t.text("management-kpi-goals-detail", Some(&args([("sign", if diff >= 0 { "+" } else { "" }.to_owned()), ("difference", diff.to_string())]))),
        );
        if a.expected >= 0.0 {
            kpi(
                &mut cols[3],
                &t.text("management-kpi-expected", None),
                plain(&t.text("management-kpi-expected-value", Some(&args([("points", a.expected_points.to_string()), ("expected", format::decimal(t, a.expected, 0))])))),
                &t.text("management-kpi-expected-detail", Some(&args([("games", a.expected_games.to_string())]))),
            );
        } else {
            kpi(&mut cols[3], &t.text("management-kpi-expected", None), plain("?"), &t.text("management-kpi-expected-locked", None));
        }
    });
    ui.add_space(4.0);
    let note = t.text("management-staff-note", Some(&args([("analyst", a.analyst.to_string()), ("assistant", a.assistant.to_string()), ("physio", a.physio.to_string())])));
    modkit::text(ui, &note, 11.0, theme::MUTED);
    ui.add_space(6.0);
    let games_height = 170.0;
    let middle = (ui.available_height() - games_height - 10.0).max(160.0);
    ui.allocate_ui(vec2(ui.available_width(), middle), |ui| {
        ui.columns(2, |cols| {
            egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).inner_margin(egui::Margin::symmetric(10, 8)).show(&mut cols[0], |ui| {
                egui::ScrollArea::vertical().id_salt("management-diagnosis").max_height(middle - 18.0).auto_shrink([false, false]).show(ui, |ui| {
                    section(ui, s, t, &t.text("management-section-diagnosis", None), &a.diagnosis);
                });
            });
            egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).inner_margin(egui::Margin::symmetric(10, 8)).show(&mut cols[1], |ui| {
                egui::ScrollArea::vertical().id_salt("management-right").max_height(middle - 18.0).auto_shrink([false, false]).show(ui, |ui| {
                    if let Some(tactic) = &a.tactic {
                        section(ui, s, t, &t.text("management-section-tactic", None), std::slice::from_ref(tactic));
                    }
                    section(ui, s, t, &t.text("management-section-suggestions", None), &a.suggestions);
                    section(ui, s, t, &t.text("management-section-medical", None), &a.medical);
                    next_rival(ui, s, t, &a);
                });
            });
        });
    });
    ui.add_space(6.0);
    games_table(ui, s, t, files, &a);
}

/// `proximo(JPanel, Analise)`
fn next_rival(ui: &mut egui::Ui, s: &CareerScreen, t: &Translator, a: &Analysis) {
    ui.add_space(12.0);
    ui.label(egui::RichText::new(t.text("management-section-next", None)).font(theme::bold(12.0)).color(GOLD));
    ui.add_space(6.0);
    let world = &s.career.world;
    let Some(px) = &a.next else {
        finding_card(ui, t, Kind::Info, &t.text("management-next-none-title", None), &t.text("management-next-none-text", None));
        return;
    };
    let f = s.career.runtime.clock.fields(px.date);
    let competition = match (px.stage, px.competition) {
        (Some(st), _) => labels::collapse_spaces(&widgets::stage_name(t, world, st)),
        (None, Some(c)) => widgets::competition_name(t, world, c),
        _ => String::new(),
    };
    let place = t.text(if px.home { "management-next-home" } else { "management-next-away" }, None);
    let when = t.text("management-next-when", Some(&args([("date", format!("{:02}/{:02}", f.day, f.month + 1)), ("place", place), ("competition", competition)])));
    let rival = world.club(px.rival).name.clone();
    if a.analyst < 2 {
        finding_card(ui, t, Kind::Info, &rival, &when);
        let (title, text) = findings::locked(t, StaffRole::Analyst, 2, &t.text("management-next-report", None));
        finding_card(ui, t, Kind::Locked, &title, &text);
        return;
    }
    let mut lines = vec![when];
    if !px.rival_form.is_empty() {
        let form: String = px.rival_form.iter().rev().map(|g| t.text(&format!("management-result-{}", g.result()), None)).collect();
        let (scored, conceded) = px.rival_form.iter().fold((0, 0), |(f, c), g| (f + g.scored, c + g.conceded));
        lines.push(t.text("management-next-form", Some(&args([("form", form), ("for", scored.to_string()), ("against", conceded.to_string()), ("games", px.rival_form.len().to_string())]))));
    }
    if let Some((name, goals)) = &px.rival_scorer {
        let mut a = brasfoot_i18n::Args::new();
        a.set("player", name.clone());
        a.set("goals", *goals as i64);
        lines.push(t.text("management-next-scorer", Some(&a)));
    }
    if a.analyst >= 3 {
        let key = if world.game.individual_skills { "management-next-strength-pitch" } else { "management-next-strength" };
        lines.push(t.text(key, Some(&args([("rival", format::decimal(t, px.rival_strength, 0)), ("mine", format::decimal(t, px.strength, 0))]))));
    }
    if a.analyst >= 5 {
        let pct = |v: f64| format::decimal(t, v * 100.0, 0);
        lines.push(t.text("management-next-chances", Some(&args([("win", pct(px.chances[0])), ("draw", pct(px.chances[1])), ("loss", pct(px.chances[2]))]))));
    }
    finding_card(ui, t, Kind::Info, &rival, &lines.join("\n"));
}

/// `tabelaJogos(Analise)`
fn games_table(ui: &mut egui::Ui, s: &CareerScreen, t: &Translator, files: &GameFiles, a: &Analysis) {
    let world = &s.career.world;
    let strength = a.analyst >= 3;
    let mut specs: Vec<(&str, f32)> = vec![
        ("management-games-date", 60.0),
        ("management-games-competition", 200.0),
        ("management-games-opponent", 220.0),
        ("management-games-place", 60.0),
        ("management-games-score", 70.0),
        ("management-games-blank", 30.0),
        ("management-games-possession", 60.0),
        ("management-games-shots", 110.0),
    ];
    if strength {
        specs.push((if world.game.individual_skills { "management-games-strength-pitch" } else { "management-games-strength" }, 120.0));
    }
    let mut columns = modkit::columns(t, &specs);
    columns[5].0 = String::new();
    let rows: Vec<Vec<Cell>> = a
        .window
        .iter()
        .map(|g| {
            let f = s.career.runtime.clock.fields(g.date);
            let competition = match (g.stage, g.competition) {
                (Some(st), _) => labels::collapse_spaces(&widgets::stage_name(t, world, st)),
                (None, Some(c)) => widgets::competition_name(t, world, c),
                _ => String::new(),
            };
            let opponent = g.opponent.map(|c| Cell::flagged(&world.club(c).name, world.club(c).country)).unwrap_or_default();
            let r = g.result();
            let color = match r {
                'V' => STRONG,
                'E' => DRAW,
                _ => PROBLEM,
            };
            let mut row = vec![
                Cell::from(format!("{:02}/{:02}", f.day, f.month + 1)),
                Cell::from(competition),
                opponent,
                Cell::from(t.text(if g.home { "management-games-home" } else { "management-games-away" }, None)),
                Cell::from(format!("{} x {}", g.scored, g.conceded)),
                Cell::colored(t.text(&format!("management-result-{r}"), None), color),
                Cell::from(t.text("management-games-percent", Some(&args([("value", g.possession.to_string())])))),
                Cell::from(t.text("management-games-shots-value", Some(&args([("shots", g.shots.to_string()), ("target", g.on_target.to_string())])))),
            ];
            if strength {
                row.push(Cell::from(t.text("management-games-strengths", Some(&args([("mine", format::decimal(t, g.strength, 0)), ("theirs", format::decimal(t, g.opponent_strength, 0))])))));
            }
            row
        })
        .collect();
    let mut none = None;
    modkit::select_table(ui, files, "management-games", &columns, &rows, &mut none, ui.available_height());
}

// Staff

fn stars(level: i32) -> String {
    (1..=5).map(|i| if i <= level { '★' } else { '☆' }).collect()
}

fn role_key(role: StaffRole) -> &'static str {
    labels::staff_role_key(role)
}

fn staff_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut ManagementState, me: ClubId, t: &Translator, files: &GameFiles) {
    let career = &mut s.career;
    if !career.systems.config.on(brasfoot_systems::config::Switch::Staff) {
        modkit::text(ui, &t.text("management-staff-off", None), 13.0, theme::MUTED);
        return;
    }
    if state.candidates.is_none() {
        state.candidates = Some(career.systems.staff_candidates(&mut career.world, &mut career.runtime));
    }
    let candidates = state.candidates.clone().unwrap_or_default();
    let world = &career.world;
    let mut fire = None;
    ui.columns(3, |cols| {
        for (i, role) in ROLES.iter().enumerate() {
            let hired = world.mods.staff.hired(*role).cloned();
            let level = hired.as_ref().map_or(0, |x| x.level);
            let border = if hired.is_some() { GOLD_DARK } else { theme::BORDER };
            modkit::item(Some(border), None).show(&mut cols[i], |ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new(t.text(role_key(*role), None).to_uppercase()).font(theme::bold(11.0)).color(GOLD));
                ui.add_space(4.0);
                match &hired {
                    Some(x) => {
                        ui.label(egui::RichText::new(brasfoot_model::brands::staff(x.name)).font(theme::bold(15.0)).color(Color32::WHITE));
                        ui.label(egui::RichText::new(format!("{}   {}", stars(x.level), labels::staff_profile(t, x.profile))).font(theme::regular(12.0)).color(GOLD));
                        ui.label(egui::RichText::new(t.text("management-staff-since", Some(&args([("salary", format::money_short(t, x.salary)), ("year", x.since.to_string())])))).font(theme::regular(11.0)).color(theme::MUTED));
                    }
                    None => {
                        ui.label(egui::RichText::new(t.text("management-staff-vacant", None)).font(theme::bold(15.0)).color(theme::MUTED));
                        ui.label(egui::RichText::new(stars(0)).font(theme::regular(12.0)).color(theme::MUTED));
                        ui.label(egui::RichText::new(" ").font(theme::regular(11.0)));
                    }
                }
                ui.add_space(8.0);
                let mut job = egui::text::LayoutJob::default();
                job.wrap.max_width = ui.available_width();
                let bold = egui::TextFormat::simple(theme::bold(11.0), theme::TEXT);
                let normal = egui::TextFormat::simple(theme::regular(11.0), theme::TEXT);
                job.append(&t.text("management-staff-today", None), 0.0, bold.clone());
                job.append(&t.text(&format!("{}-level-{level}", role_key(*role)), None), 4.0, normal.clone());
                if level < 5 {
                    job.append("\n", 0.0, normal.clone());
                    job.append(&t.text("management-staff-next", Some(&args([("level", (level + 1).to_string())]))), 0.0, bold);
                    job.append(&t.text(&format!("{}-level-{}", role_key(*role), level + 1), None), 4.0, normal);
                }
                ui.add(egui::Label::new(job).wrap());
                ui.add_space(8.0);
                if let Some(x) = &hired
                    && controls::button(ui, &t.text("management-staff-fire", None), controls::Kind::Secondary, true).clicked()
                {
                    fire = Some((*role, brasfoot_model::brands::staff(x.name)));
                }
            });
        }
    });
    ui.add_space(10.0);
    let reputation = world.club(me).reputation;
    ui.label(egui::RichText::new(t.text("management-staff-candidates", Some(&args([("wages", format::money_short(t, brasfoot_systems::staff::staff_wages(world)))])))).font(theme::bold(12.0)).color(GOLD));
    ui.add_space(4.0);
    let rows: Vec<Vec<Cell>> = candidates
        .iter()
        .map(|x| {
            let minimum = brasfoot_systems::staff::minimum_reputation(x.level);
            let accepts = if reputation >= minimum { t.text("management-staff-accepts", None) } else { t.text("management-staff-refuses", Some(&args([("reputation", minimum.to_string())]))) };
            vec![
                Cell::from(t.text(role_key(x.role), None)),
                Cell::from(brasfoot_model::brands::staff(x.name)),
                Cell::from(stars(x.level)),
                Cell::from(labels::staff_profile(t, x.profile)),
                Cell::from(format::money_short(t, x.salary)),
                Cell::from(accepts),
            ]
        })
        .collect();
    let columns = modkit::columns(
        t,
        &[
            ("management-staff-column-role", 190.0),
            ("management-staff-column-name", 200.0),
            ("management-staff-column-level", 90.0),
            ("management-staff-column-profile", 160.0),
            ("management-staff-column-salary", 110.0),
            ("management-staff-column-accepts", 160.0),
        ],
    );
    let height = ui.available_height() - 40.0;
    modkit::select_table(ui, files, "management-candidates", &columns, &rows, &mut state.candidate, height);
    ui.add_space(4.0);
    if controls::button(ui, &t.text("management-staff-hire", None), controls::Kind::Secondary, true).clicked() {
        match state.candidate.and_then(|i| candidates.get(i).map(|x| (i, x.clone()))) {
            None => state.question = Some(modkit::message(String::new(), t.text("management-staff-pick", None))),
            Some((i, x)) => match world.mods.staff.hired(x.role) {
                Some(current) => {
                    let text = t.text(
                        "management-staff-swap",
                        Some(&args([("current", brasfoot_model::brands::staff(current.name)), ("current_level", current.level.to_string()), ("new", brasfoot_model::brands::staff(x.name)), ("new_level", x.level.to_string())])),
                    );
                    state.question = Some(modkit::confirm(t.text("management-staff-title", None), text));
                    state.pending = Some(Pending::Hire(i));
                }
                None => hire(s, state, t, i, &x),
            },
        }
    }
    if let Some((role, name)) = fire {
        state.question = Some(modkit::confirm(t.text("management-staff-title", None), t.text("management-staff-fire-confirm", Some(&args([("name", name)])))));
        state.pending = Some(Pending::Fire(role));
    }
}

// Director of football

fn director_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut ManagementState, me: ClubId, t: &Translator, files: &GameFiles) -> Option<PlayerId> {
    let career = &mut s.career;
    if state.targets.is_none() {
        state.targets = Some(career.systems.with(&mut career.world, &mut career.runtime, |m| m.director_targets(15)));
    }
    let targets = state.targets.clone().unwrap_or_default();
    let world = &career.world;
    let slot = findings::slot_name(t, brasfoot_systems::director::target_kind(world, me));
    modkit::text(ui, &t.text("management-director-top", Some(&args([("slot", slot)]))), 12.0, theme::MUTED);
    ui.add_space(6.0);
    if targets.is_empty() {
        modkit::text(ui, &t.text("management-director-none", None), 13.0, theme::MUTED);
        return None;
    }
    let rows: Vec<Vec<Cell>> = targets
        .iter()
        .map(|x| {
            let p = world.player(x.player);
            let e = scouts::estimate_in(world, x.player, x.slot);
            vec![
                Cell::flagged(&p.name, p.country),
                Cell::from(p.age),
                Cell::from(&world.club(x.club).name),
                Cell::from(pitch_range(t, world, x.player, e)),
                Cell::from(p.strength),
                Cell::from(format::mi(t, x.price)),
                Cell::from(t.text("management-percent", Some(&args([("value", x.chance.to_string())])))),
            ]
        })
        .collect();
    let columns = modkit::columns(
        t,
        &[
            ("management-director-column-player", 230.0),
            ("management-director-column-age", 55.0),
            ("management-director-column-club", 200.0),
            ("management-director-column-pitch", 75.0),
            ("management-director-column-screen", 50.0),
            ("management-director-column-price", 110.0),
            ("management-director-column-chance", 120.0),
        ],
    );
    let height = ui.available_height() - 40.0;
    modkit::select_table(ui, files, "management-director", &columns, &rows, &mut state.target, height);
    ui.add_space(4.0);
    if controls::button(ui, &t.text("management-button-offer", None), controls::Kind::Secondary, true).clicked() {
        match state.target.and_then(|i| targets.get(i)) {
            None => state.question = Some(modkit::message(String::new(), t.text("management-pick-player", None))),
            Some(x) => return Some(x.player),
        }
    }
    None
}

/// `Olheiros.faixa(F, int)`: the value, or a range for a player the club does not know.
fn pitch_range(t: &Translator, world: &World, p: PlayerId, estimate: i32) -> String {
    let (low, high) = scouts::range(world, p, estimate);
    if low == high { low.to_string() } else { t.text("management-range", Some(&args([("low", low), ("high", high)]))) }
}

// Scouts

fn scouts_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut ManagementState, t: &Translator, files: &GameFiles) -> Option<PlayerId> {
    let mut search = state.found.is_none();
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(t.text("management-scouts-position", None)).font(theme::bold(12.0)).color(GOLD));
        let positions: Vec<String> = std::iter::once(t.text("management-scouts-any", None)).chain((0..5).map(|p| widgets::position_name(t, p))).collect();
        egui::ComboBox::from_id_salt("management-scouts-position").width(110.0).selected_text(positions[state.position].clone()).show_ui(ui, |ui| {
            for (i, p) in positions.iter().enumerate() {
                ui.selectable_value(&mut state.position, i, p);
            }
        });
        ui.label(egui::RichText::new(t.text("management-scouts-age", None)).font(theme::bold(12.0)).color(GOLD));
        modkit::spinner(ui, &mut state.max_age, 16, 40, 1, true);
        ui.label(egui::RichText::new(t.text("management-scouts-pitch", None)).font(theme::bold(12.0)).color(GOLD));
        modkit::spinner(ui, &mut state.min_value, 1, 99, 1, true);
        ui.label(egui::RichText::new(t.text("management-scouts-price", None)).font(theme::bold(12.0)).color(GOLD));
        modkit::spinner(ui, &mut state.max_price, 0, 500, 5, true);
        if controls::button(ui, &t.text("management-scouts-search", None), controls::Kind::Secondary, true).clicked() {
            search = true;
        }
    });
    let world = &s.career.world;
    let note = t.text("management-scouts-note", Some(&args([("cost", format::mi(t, scouts::cost(world))), ("max", scouts::MAX_AT_ONCE.to_string())])));
    modkit::text(ui, &note, 12.0, theme::MUTED);
    ui.add_space(6.0);
    if search {
        state.found = Some(scouts::search(world, state.position as i32 - 1, state.max_age as i32, state.min_value as i32, state.max_price * 1_000_000));
    }
    let found = state.found.clone().unwrap_or_default();
    let config = s.career.systems.config.clone();
    let rows: Vec<Vec<Cell>> = found
        .iter()
        .map(|a| {
            let p = world.player(a.player);
            let report = if scouts::margin(world, a.player) == 0 || p.age <= 21 { brasfoot_systems::academy::youth_report(world, &config, a.player) } else { None };
            let status = if a.watched {
                t.text("management-scouts-watched", None)
            } else if a.being_watched {
                t.text("management-scouts-watching", None)
            } else {
                String::new()
            };
            vec![
                Cell::flagged(&p.name, p.country),
                Cell::from(p.age),
                Cell::from(widgets::position_name(t, p.position)),
                Cell::from(p.club.map(|c| world.club(c).name.clone()).unwrap_or_default()),
                Cell::from(if a.range.0 == a.range.1 { a.range.0.to_string() } else { t.text("management-range", Some(&args([("low", a.range.0), ("high", a.range.1)]))) }),
                Cell::from(findings::slot_name(t, a.kind)),
                Cell::from(p.strength),
                Cell::from(report.map(|r| t.text("youth-report-range", Some(&args([("low", r.low), ("high", r.high)])))).unwrap_or_default()),
                Cell::from(format::mi(t, p.market_value as i64)),
                Cell::from(status),
            ]
        })
        .collect();
    let columns = modkit::columns(
        t,
        &[
            ("management-scouts-column-player", 200.0),
            ("management-scouts-column-age", 45.0),
            ("management-scouts-column-position", 70.0),
            ("management-scouts-column-club", 170.0),
            ("management-scouts-column-pitch", 75.0),
            ("management-scouts-column-slot", 85.0),
            ("management-scouts-column-screen", 45.0),
            ("management-scouts-column-potential", 80.0),
            ("management-scouts-column-value", 70.0),
            ("management-scouts-column-scout", 100.0),
        ],
    );
    let height = ui.available_height() - 40.0;
    modkit::select_table(ui, files, "management-scouts", &columns, &rows, &mut state.found_selected, height);
    ui.add_space(4.0);
    let mut offer = None;
    ui.horizontal(|ui| {
        if controls::button(ui, &t.text("management-scouts-send", None), controls::Kind::Secondary, true).clicked() {
            match state.found_selected.and_then(|i| found.get(i)) {
                None => state.question = Some(modkit::message(String::new(), t.text("management-pick-player", None))),
                Some(a) => {
                    let p = a.player;
                    let career = &mut s.career;
                    let result = career.systems.with(&mut career.world, &mut career.runtime, |m| m.send_scout(p));
                    let key = match result {
                        Ok(()) => "management-scouts-sent",
                        Err(ScoutError::NoClub) => "management-scouts-error-no-club",
                        Err(ScoutError::AlreadyKnown) => "management-scouts-error-known",
                        Err(ScoutError::AlreadyWatching) => "management-scouts-error-watching",
                        Err(ScoutError::AllBusy) => "management-scouts-error-busy",
                        Err(ScoutError::NoCash) => "management-scouts-error-cash",
                    };
                    state.question = Some(modkit::message(String::new(), t.text(key, None)));
                    state.found = None;
                }
            }
        }
        if controls::button(ui, &t.text("management-button-offer", None), controls::Kind::Secondary, true).clicked() {
            match state.found_selected.and_then(|i| found.get(i)) {
                None => state.question = Some(modkit::message(String::new(), t.text("management-pick-player", None))),
                Some(a) => offer = Some(a.player),
            }
        }
    });
    offer
}

// Loans

fn loans_tab(ui: &mut egui::Ui, s: &mut CareerScreen, _me: ClubId, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let config = &s.career.systems.config;
    let humans = brasfoot_systems::economy::humans(world);
    // Emprestimos.meusEmprestados(): the loans of human clubs whose player plays elsewhere
    let rows: Vec<Vec<Cell>> = world
        .game
        .loans
        .iter()
        .filter_map(|l| {
            let p = l.player?;
            let owner = l.owner?;
            (humans.contains(&world.club(owner).id) && world.player(p).club != Some(owner)).then_some((p, l.until))
        })
        .map(|(p, until)| {
            let x = world.player(p);
            let (games, goals, rating) = brasfoot_systems::loans::season_numbers(world, p);
            let report = brasfoot_systems::academy::youth_report(world, config, p);
            vec![
                Cell::flagged(&x.name, x.country),
                Cell::from(x.age),
                Cell::from(x.club.map(|c| world.club(c).name.clone()).unwrap_or_else(|| "-".into())),
                Cell::from(games),
                Cell::from(goals),
                Cell::from(if rating > 0.0 { format::rating_two(t, rating) } else { "-".into() }),
                Cell::from(x.strength),
                Cell::from(report.map(|r| t.text("youth-report-range", Some(&args([("low", r.low), ("high", r.high)])))).unwrap_or_default()),
                Cell::from(format::date(t, &s.career.runtime.clock, until)),
            ]
        })
        .collect();
    if rows.is_empty() {
        modkit::text(ui, &t.text("management-loans-none", None), 13.0, theme::MUTED);
        return;
    }
    let columns = modkit::columns(
        t,
        &[
            ("management-loans-column-player", 220.0),
            ("management-loans-column-age", 50.0),
            ("management-loans-column-club", 200.0),
            ("management-loans-column-games", 55.0),
            ("management-loans-column-goals", 50.0),
            ("management-loans-column-rating", 55.0),
            ("management-loans-column-strength", 55.0),
            ("management-loans-column-potential", 90.0),
            ("management-loans-column-back", 100.0),
        ],
    );
    let mut none = None;
    modkit::select_table(ui, files, "management-loans", &columns, &rows, &mut none, ui.available_height());
}

// Sponsorship

fn sponsorship_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut ManagementState, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let millions = |v: i64| format::mi(t, v);
    match world.mods.master_sponsor.active() {
        Some(k) => {
            let brand = brasfoot_model::brands::MASTER_BRANDS[k.brand as usize].to_owned();
            modkit::strong(ui, &t.text("management-sponsor-master", Some(&args([("brand", brand), ("kind", labels::master_kind(t, k.kind))]))), 16.0, Color32::WHITE);
            ui.add_space(6.0);
            modkit::text(ui, &labels::master_description(t, k, &millions), 13.0, theme::TEXT);
            modkit::text(ui, &t.text("management-sponsor-monthly", Some(&args([("amount", format::mi(t, k.yearly / 12))]))), 12.0, theme::MUTED);
        }
        None => modkit::text(ui, &t.text("management-sponsor-none", None), 14.0, theme::TEXT),
    }
    ui.add_space(8.0);
    if controls::button(ui, &t.text("management-sponsor-search", None), controls::Kind::Secondary, true).clicked() {
        state.open_sponsors = true;
    }
    ui.add_space(8.0);
    let rows: Vec<Vec<Cell>> = world.mods.master_sponsor.history.iter().rev().map(|n| vec![Cell::from(labels::master_note(t, n, &millions))]).collect();
    let columns = modkit::columns(t, &[("management-sponsor-history", 900.0)]);
    let mut none = None;
    modkit::select_table(ui, files, "management-sponsor-history", &columns, &rows, &mut none, ui.available_height());
}

// Fans

fn fans_tab(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let config = &s.career.systems.config;
    modkit::strong(ui, &t.text("management-fans-confidence", Some(&args([("value", brasfoot_systems::fans::confidence(world).to_string())]))), 16.0, Color32::WHITE);
    ui.add_space(6.0);
    modkit::text(ui, &t.text("management-fans-members", Some(&args([("amount", format::mi(t, brasfoot_systems::fans::monthly_members(world, config)))]))), 13.0, theme::TEXT);
    modkit::text(ui, &t.text("management-fans-note", None), 12.0, theme::MUTED);
    ui.add_space(8.0);
    let reasons = &world.mods.fans.reasons;
    if reasons.is_empty() {
        modkit::text(ui, &t.text("management-fans-none", None), 13.0, theme::MUTED);
        return;
    }
    let rows: Vec<Vec<Cell>> = reasons
        .iter()
        .rev()
        .map(|r| {
            let effect = t.text("management-fans-effect", Some(&args([("sign", if r.change > 0 { "+" } else { "" }.to_owned()), ("value", r.change.to_string())])));
            vec![Cell::from(format::date(t, &s.career.runtime.clock, r.day)), Cell::from(effect), Cell::from(labels::fan_event(t, &r.why))]
        })
        .collect();
    let columns = modkit::columns(t, &[("management-fans-column-date", 100.0), ("management-fans-column-effect", 70.0), ("management-fans-column-reason", 700.0)]);
    let mut none = None;
    modkit::select_table(ui, files, "management-fans", &columns, &rows, &mut none, ui.available_height());
}
