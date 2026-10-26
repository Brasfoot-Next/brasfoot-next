//! The club's finances (`bfmod.CaixaDialog`, "Finanças do clube"): the summary with its charts,
//! the monthly statement (`bfmod.Caixa`), the wage bill, the sponsorship slots, the transfers,
//! the bank and the owner, and the comparison with the league; the game's own finances screen
//! (`a.cW`) from its button.

use brasfoot_i18n::{Translator, args, labels};
use brasfoot_model::ids::ClubId;
use brasfoot_model::mods::SponsorSlot;
use brasfoot_model::world::World;
use brasfoot_simulation::human::finances as quick_loan;
use brasfoot_systems::cash::{self, EXPENSE_LINES, Flow, INCOME_LINES};
use brasfoot_systems::config::Switch;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, pos2, vec2};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::form::{Question, Reply};
use crate::screens::modkit::{self, Cell, GOLD, GOLD_DARK};
use crate::screens::widgets;
use crate::{format, motion, theme};

const TABS: [&str; 7] = ["finances-tab-summary", "finances-tab-statement", "finances-tab-wages", "finances-tab-sponsors", "finances-tab-transfers", "finances-tab-bank", "finances-tab-comparison"];
const BLUE: Color32 = Color32::from_rgb(0x39, 0x87, 0xe5);
const ORANGE: Color32 = Color32::from_rgb(0xd9, 0x59, 0x26);
const GRID: Color32 = Color32::from_rgba_premultiplied(24, 24, 24, 24);
const POSITIVE: Color32 = Color32::from_rgb(0x4c, 0xc3, 0x8a);
const NEGATIVE: Color32 = Color32::from_rgb(0xe6, 0x67, 0x67);
/// The charts' margins (`ESQ`, `DIR`, `TOPO`, `BASE`).
const LEFT: f32 = 62.0;
const RIGHT: f32 = 14.0;
const TOP: f32 = 34.0;
const BOTTOM: f32 = 24.0;
/// The bank's amounts and terms (`opcoesValores`, `prazosMeses`).
const AMOUNTS: [i64; 5] = [2_000_000, 5_000_000, 10_000_000, 20_000_000, 50_000_000];
const TERMS: [i32; 3] = [12, 24, 36];

#[derive(Clone, Copy)]
enum Pending {
    EndMaster,
    EndSlot(usize),
    RepayLoan(i32),
    Borrow(i64, i32),
}

pub struct FinancesState {
    tab: usize,
    offer: Option<usize>,
    loan: Option<usize>,
    amount: usize,
    term: usize,
    question: Option<Question>,
    pending: Option<Pending>,
    history_open: bool,
    /// "Tela original do jogo"
    classic: bool,
}

impl FinancesState {
    /// `CaixaDialog.mostrar` (tab 0) or `mostrarPatrocinios` (tab 3).
    pub fn new(tab: usize) -> Self {
        FinancesState { tab: tab.min(TABS.len() - 1), offer: Some(0), loan: None, amount: 0, term: 0, question: None, pending: None, history_open: false, classic: false }
    }
}

/// `CaixaDialog.dinheiro(long)`
fn money(t: &Translator, v: i64) -> String {
    format::money_short(t, v)
}

/// `CaixaDialog.sinal(long)`: "+" on the positives.
fn signed(t: &Translator, v: i64) -> String {
    if v > 0 { format!("+{}", money(t, v)) } else { money(t, v) }
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

fn line_name(t: &Translator, i: usize) -> String {
    t.text(&format!("finances-line-{i}"), None)
}

/// `Caixa.nomeDoMes(int)`
fn month_name(t: &Translator, month: i32) -> String {
    let m = month % 100;
    if (1..=12).contains(&m) { t.text(&format!("finances-month-{m}"), None) } else { "?".to_owned() }
}

/// `Caixa.formatar(int)`: dd/MM/yyyy of a yyyymmdd date.
fn date_of(day: i32) -> String {
    format!("{:02}/{:02}/{}", day % 100, day / 100 % 100, day / 10000)
}

pub fn window(s: &mut CareerScreen, state: &mut FinancesState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let Some(me) = s.career.human_club() else {
        *open = false;
        return;
    };
    // Caixa.tick(): the statement is up to date before the window reads it
    let career = &mut s.career;
    career.systems.with(&mut career.world, &mut career.runtime, |m| m.cash_tick());
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(16, 12));
    motion::dialog(ctx, "finances", open, vec2(1060.0, 730.0), frame, |ui, open| {
        header(ui, s, me, t, files);
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
                0 => summary_tab(ui, s, me, t),
                1 => statement_tab(ui, s, t, files),
                2 => wages_tab(ui, s, me, t, files),
                3 => sponsors_tab(ui, s, state, me, t, files),
                4 => transfers_tab(ui, s, me, t, files),
                5 => bank_tab(ui, s, state, me, t, files),
                _ => comparison_tab(ui, s, me, t, files),
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            modkit::text(ui, &t.text("finances-footnote", None), 11.0, theme::MUTED);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if modkit::button(ui, &t.text("finances-button-close", None), false, true).clicked() {
                    *open = false;
                }
                if modkit::button(ui, &t.text("finances-button-classic", None), false, true).clicked() {
                    state.classic = true;
                }
            });
        });
    });
    if state.classic {
        let mut keep = true;
        classic_window(s, me, ctx, t, files, &mut keep);
        state.classic = keep;
    }
    if state.history_open {
        let mut keep = true;
        history_window(s, ctx, t, files, &mut keep);
        state.history_open = keep;
    }
    if let Some(reply) = modkit::question_overlay(ctx, t, "finances", &mut state.question) {
        let pending = state.pending.take();
        if reply == Reply::Yes {
            let career = &mut s.career;
            match pending {
                Some(Pending::EndMaster) => career.systems.with(&mut career.world, &mut career.runtime, |m| {
                    m.end_master_sponsor();
                    m.open_master_offers();
                }),
                Some(Pending::EndSlot(i)) => career.systems.with(&mut career.world, &mut career.runtime, |m| m.end_sponsor_slot(i)),
                Some(Pending::RepayLoan(id)) => {
                    if !career.systems.with(&mut career.world, &mut career.runtime, |m| m.repay_early(me, id)) {
                        state.question = Some(modkit::message(String::new(), t.text("finances-bank-repay-no-cash", None)));
                    }
                }
                Some(Pending::Borrow(amount, months)) => {
                    if career.systems.with(&mut career.world, &mut career.runtime, |m| m.borrow(me, amount, months)).is_none() {
                        state.question = Some(modkit::message(String::new(), t.text("finances-bank-borrow-failed", None)));
                    }
                }
                None => {}
            }
        }
    }
}

fn header(ui: &mut egui::Ui, s: &CareerScreen, me: ClubId, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let year = brasfoot_systems::economy::year(world);
            ui.label(egui::RichText::new(t.text("finances-header", Some(&args([("year", year.to_string())])))).font(theme::bold(15.0)).color(GOLD));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                widgets::crest(ui, files, world, me, 18.0);
                ui.label(egui::RichText::new(&world.club(me).name).font(theme::regular(13.0)).color(theme::MUTED));
            });
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.vertical(|ui| {
                let cash = world.club(me).cash;
                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                    ui.label(egui::RichText::new(format::money(t, cash)).font(theme::bold(22.0)).color(if cash < 0 { NEGATIVE } else { Color32::WHITE }));
                    ui.label(egui::RichText::new(t.text("finances-in-cash", None)).font(theme::regular(11.0)).color(theme::MUTED));
                });
            });
        });
    });
}

/// `cartao(titulo, valor, detalhe, cor)`
fn kpi(ui: &mut egui::Ui, title: &str, value: &str, detail: &str, color: Option<Color32>) {
    modkit::item(None, None).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(title.to_uppercase()).font(theme::bold(10.0)).color(theme::MUTED));
        ui.add_space(3.0);
        ui.label(egui::RichText::new(value).font(theme::bold(17.0)).color(color.unwrap_or(Color32::WHITE)));
        ui.add_space(2.0);
        let mut job = egui::text::LayoutJob::simple_singleline(detail.to_owned(), theme::regular(11.0), theme::MUTED);
        job.wrap = egui::text::TextWrapping::truncate_at_width(ui.available_width());
        ui.label(job).on_hover_text(detail);
    });
}

// Summary

fn summary_tab(ui: &mut egui::Ui, s: &mut CareerScreen, me: ClubId, t: &Translator) {
    let career = &mut s.career;
    let world = &career.world;
    let season = cash::season_totals(world, world.season());
    let income: i64 = season[..INCOME_LINES].iter().sum();
    let expenses: i64 = season[INCOME_LINES..].iter().sum();
    let months = cash::months_elapsed(world, &career.runtime.clock);
    let wages = cash::monthly_wages(world, me);
    let recurring = brasfoot_model::rng::java_round((season[0] + season[3]) as f64 / months);
    let interest = world.club(me).finances.bank_interest as i64;
    let other_costs = brasfoot_model::rng::java_round((season[6] + season[10]) as f64 / months);
    let left = (12.0 - months).max(0.0);
    let cash_now = world.club(me).cash;
    let projection = cash_now + brasfoot_model::rng::java_round(left * (recurring - wages - interest - other_costs) as f64);
    let sponsorship = brasfoot_systems::sponsors::slots_yearly(world) + world.mods.master_sponsor.active().map_or(0, |k| k.yearly);
    let squad_value: i64 = world.club(me).squad.iter().map(|&p| world.player(p).market_value as i64).sum();
    let config = career.systems.config.clone();
    let members = brasfoot_systems::fans::monthly_members(world, &config);
    let squad_size = world.club(me).squad.len();
    let (debt, rating, owner) = career.systems.with(&mut career.world, &mut career.runtime, |m| (m.active_debt(me), m.credit_rating(me), m.club(me).clone()));
    let world = &career.world;
    let mut rows = [Vec::new(), Vec::new()];
    rows[0].push((t.text("finances-kpi-cash", None), format::money(t, cash_now), wealth(t, cash_now), (cash_now < 0).then_some(NEGATIVE)));
    rows[0].push((
        t.text("finances-kpi-result", None),
        signed(t, income - expenses),
        t.text("finances-kpi-result-detail", Some(&args([("income", money(t, income)), ("expenses", money(t, expenses))]))),
        Some(if income - expenses < 0 { NEGATIVE } else { POSITIVE }),
    ));
    let wage_detail = if recurring > 0 {
        t.text("finances-kpi-wages-detail", Some(&args([("percent", brasfoot_model::rng::java_round(100.0 * wages as f64 / recurring as f64).to_string())])))
    } else {
        t.text("finances-kpi-wages-none", None)
    };
    rows[0].push((t.text("finances-kpi-wages", None), money(t, wages), wage_detail, (recurring > 0 && wages > recurring).then_some(NEGATIVE)));
    rows[0].push((t.text("finances-kpi-projection", None), money(t, projection), t.text("finances-kpi-projection-detail", None), (projection < 0).then_some(NEGATIVE)));
    rows[1].push((t.text("finances-kpi-sponsorship", None), money(t, sponsorship), t.text("finances-kpi-sponsorship-detail", Some(&args([("members", money(t, members))]))), None));
    rows[1].push((t.text("finances-kpi-squad", None), money(t, squad_value), t.text("finances-kpi-squad-detail", Some(&args([("count", squad_size.to_string())]))), None));
    let debt_state = t.text(if debt > 0 { "finances-kpi-debt-committed" } else { "finances-kpi-debt-none" }, None);
    rows[1].push((
        t.text("finances-kpi-debt", None),
        money(t, debt),
        t.text("finances-kpi-debt-detail", Some(&args([("rating", rating_name(rating)), ("state", debt_state)]))),
        (debt > 0).then_some(NEGATIVE),
    ));
    let (owner_name, owner_detail) = if owner.has_owner {
        (owner.owner_name.map(brasfoot_model::brands::owner).unwrap_or_default(), t.text("finances-kpi-owner-detail", Some(&args([("amount", money(t, owner.owner_funding))]))))
    } else {
        (t.text("finances-kpi-no-owner", None), t.text("finances-kpi-no-owner-detail", None))
    };
    rows[1].push((t.text("finances-kpi-owner", None), owner_name, owner_detail, None));
    for row in &rows {
        ui.columns(4, |cols| {
            for (i, (title, value, detail, color)) in row.iter().enumerate() {
                kpi(&mut cols[i], title, value, detail, *color);
            }
        });
        ui.add_space(8.0);
    }
    let height = ((ui.available_height() - 8.0) / 2.0).max(120.0);
    let flows = cash::flows(world);
    let days: Vec<(i32, i64)> = world.mods.cash_book.days.iter().map(|(&d, &v)| (d, v)).collect();
    ui.columns(2, |cols| {
        chart_frame(&mut cols[0], height, |ui, rect| cash_chart(ui, rect, t, &days));
        chart_frame(&mut cols[1], height, |ui, rect| months_chart(ui, rect, t, &flows));
    });
    ui.add_space(8.0);
    ui.columns(2, |cols| {
        let names: Vec<String> = (0..INCOME_LINES).map(|i| line_name(t, i)).collect();
        chart_frame(&mut cols[0], height, |ui, rect| composition(ui, rect, t, &t.text("finances-chart-income", None), &names, &season[..INCOME_LINES], BLUE));
        let names: Vec<String> = (0..EXPENSE_LINES).map(|i| line_name(t, INCOME_LINES + i)).collect();
        chart_frame(&mut cols[1], height, |ui, rect| composition(ui, rect, t, &t.text("finances-chart-expenses", None), &names, &season[INCOME_LINES..], ORANGE));
    });
}

/// `Credito.Rating.nome`
fn rating_name(r: brasfoot_systems::credit::Rating) -> String {
    format!("{r:?}").to_uppercase()
}

/// `moldura(grafico)`: a framed panel the chart paints into.
fn chart_frame(ui: &mut egui::Ui, height: f32, paint: impl FnOnce(&mut egui::Ui, Rect)) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    ui.painter().rect(rect, 4.0, theme::PANEL, Stroke::new(1.0, theme::BORDER), egui::StrokeKind::Inside);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    paint(&mut child, rect);
}

fn chart_title(ui: &egui::Ui, rect: Rect, title: &str) {
    ui.painter().text(rect.min + vec2(12.0, 20.0), egui::Align2::LEFT_BOTTOM, title, theme::bold(12.0), theme::TEXT);
}

fn empty_chart(ui: &egui::Ui, rect: Rect, text: &str) {
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, theme::regular(12.0), theme::MUTED);
}

/// `Grafico.escala(min, max)`: a rounded scale, {min, max, step}.
fn scale(min: f64, max: f64) -> [f64; 3] {
    let max = if max <= min { min + 1.0 } else { max };
    let raw = (max - min) / 4.0;
    let mag = 10f64.powf(raw.log10().floor());
    let r = raw / mag;
    let step = (if r <= 1.0 {
        1.0
    } else if r <= 2.0 {
        2.0
    } else if r <= 2.5 {
        2.5
    } else if r <= 5.0 {
        5.0
    } else {
        10.0
    }) * mag;
    [(min / step).floor() * step, (max / step).ceil() * step, step]
}

/// `Grafico.eixo(double)`
fn axis(t: &Translator, v: f64) -> String {
    let a = v.abs();
    let sign = if v < 0.0 { "-" } else { "" };
    let trim = |x: f64| if x == x.round() { format::decimal(t, x, 0) } else { format::decimal(t, x, 1) };
    if a >= 1e9 {
        format!("{sign}{} {}", trim(a / 1e9), t.text("finances-axis-bi", None))
    } else if a >= 1e6 {
        format!("{sign}{} {}", trim(a / 1e6), t.text("finances-axis-mi", None))
    } else if a >= 1e3 {
        format!("{sign}{} {}", trim(a / 1e3), t.text("finances-axis-thousand", None))
    } else {
        format!("{sign}{}", trim(a))
    }
}

fn y_of(rect: Rect, v: f64, s: [f64; 3]) -> f32 {
    let h = rect.height() - TOP - BOTTOM;
    rect.top() + TOP + (h as f64 - (v - s[0]) / (s[1] - s[0]) * h as f64) as f32
}

fn grid(ui: &egui::Ui, rect: Rect, t: &Translator, s: [f64; 3]) {
    let p = ui.painter();
    let mut v = s[0];
    while v <= s[1] + s[2] / 2.0 {
        let y = y_of(rect, v, s);
        let color = if v.abs() < s[2] / 2.0 { Color32::from_white_alpha(70) } else { GRID };
        p.line_segment([pos2(rect.left() + LEFT, y), pos2(rect.right() - RIGHT, y)], Stroke::new(1.0, color));
        p.text(pos2(rect.left() + LEFT - 6.0, y), egui::Align2::RIGHT_CENTER, axis(t, v), theme::regular(10.0), theme::MUTED);
        v += s[2];
    }
}

/// `Grafico.dica`: the box near the cursor, kept inside the chart.
fn tip(ui: &egui::Ui, rect: Rect, at: Pos2, lines: &[String]) {
    let p = ui.painter();
    let w = lines.iter().map(|l| p.layout_no_wrap(l.clone(), theme::regular(11.0), theme::TEXT).size().x).fold(0.0, f32::max) + 16.0;
    let h = lines.len() as f32 * 15.0 + 10.0;
    let x = if at.x + 12.0 + w > rect.right() - 4.0 { at.x - 12.0 - w } else { at.x + 12.0 };
    let y = (at.y - h / 2.0).clamp(rect.top() + 4.0, rect.bottom() - h - 4.0);
    let b = Rect::from_min_size(pos2(x, y), vec2(w, h));
    p.rect(b, 8.0, Color32::from_rgba_unmultiplied(12, 14, 20, 235), Stroke::new(1.0, GOLD_DARK), egui::StrokeKind::Inside);
    for (i, l) in lines.iter().enumerate() {
        let font = if i == 0 { theme::bold(11.0) } else { theme::regular(11.0) };
        p.text(pos2(b.left() + 8.0, b.top() + 12.0 + i as f32 * 15.0), egui::Align2::LEFT_CENTER, l, font, if i == 0 { Color32::WHITE } else { theme::TEXT });
    }
}

/// `GraficoCaixa`: the cash day by day.
fn cash_chart(ui: &mut egui::Ui, rect: Rect, t: &Translator, days: &[(i32, i64)]) {
    chart_title(ui, rect, &t.text("finances-chart-cash", None));
    if days.len() < 2 {
        empty_chart(ui, rect, &t.text("finances-chart-cash-empty", None));
        return;
    }
    let (min, max) = days.iter().fold((0.0f64, 0.0f64), |(a, b), &(_, v)| (a.min(v as f64), b.max(v as f64)));
    let s = scale(min, max);
    grid(ui, rect, t, s);
    let serial = |d: i32| {
        let (y, m, dd) = (d / 10000, d / 100 % 100, d % 100);
        // days since an epoch, for the spacing only
        (y as i64) * 372 + (m as i64) * 31 + dd as i64
    };
    let d0 = serial(days[0].0);
    let d1 = serial(days[days.len() - 1].0).max(d0 + 1);
    let width = rect.width() - LEFT - RIGHT;
    let xs: Vec<f32> = days.iter().map(|&(d, _)| rect.left() + LEFT + ((serial(d) - d0) as f64 / (d1 - d0) as f64 * width as f64) as f32).collect();
    let p = ui.painter();
    let (mut last_x, mut last_month) = (-100.0f32, -1);
    for (i, &(d, _)) in days.iter().enumerate() {
        let month = d / 100;
        if month != last_month && xs[i] - last_x > 46.0 {
            let mut label = month_name(t, month);
            if month % 100 == 1 {
                label = format!("{label}/{:02}", month / 100 % 100);
            }
            p.text(pos2(xs[i], rect.bottom() - 8.0), egui::Align2::LEFT_BOTTOM, label, theme::regular(10.0), theme::MUTED);
            last_x = xs[i];
        }
        last_month = month;
    }
    let points: Vec<Pos2> = days.iter().enumerate().map(|(i, &(_, v))| pos2(xs[i], y_of(rect, v as f64, s))).collect();
    let base = y_of(rect, s[0].max(0.0), s);
    // the area under the line, fading down
    let mut mesh = egui::Mesh::default();
    for (i, &pt) in points.iter().enumerate() {
        mesh.colored_vertex(pt, Color32::from_rgba_unmultiplied(57, 135, 229, 70));
        mesh.colored_vertex(pos2(pt.x, base), Color32::from_rgba_unmultiplied(57, 135, 229, 0));
        if i > 0 {
            let k = (i * 2) as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k + 1, k);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    p.add(egui::Shape::line(points.clone(), Stroke::new(2.0, BLUE)));
    let marker = |p: &egui::Painter, at: Pos2| {
        p.circle_filled(at, 6.0, theme::PANEL);
        p.circle_filled(at, 4.0, BLUE);
    };
    let last = points.len() - 1;
    marker(p, points[last]);
    let hover = ui.input(|i| i.pointer.hover_pos()).filter(|h| rect.contains(*h) && h.x >= rect.left() + LEFT);
    if let Some(h) = hover {
        let best = (0..xs.len()).min_by(|&a, &b| (xs[a] - h.x).abs().total_cmp(&(xs[b] - h.x).abs())).unwrap_or(0);
        p.line_segment([pos2(xs[best], rect.top() + TOP), pos2(xs[best], rect.bottom() - BOTTOM)], Stroke::new(1.0, Color32::from_white_alpha(60)));
        marker(p, points[best]);
        tip(ui, rect, points[best], &[date_of(days[best].0), t.text("finances-chart-cash-tip", Some(&args([("amount", money(t, days[best].1))])))]);
    } else {
        let text = money(t, days[last].1);
        let w = p.layout_no_wrap(text.clone(), theme::bold(11.0), theme::TEXT).size().x;
        let x = (points[last].x - w / 2.0).min(rect.right() - RIGHT - w);
        let y = (points[last].y - 10.0).max(rect.top() + TOP + 10.0);
        p.text(pos2(x, y), egui::Align2::LEFT_BOTTOM, text, theme::bold(11.0), theme::TEXT);
    }
}

/// `GraficoMeses`: money in and out of the last twelve complete months.
fn months_chart(ui: &mut egui::Ui, rect: Rect, t: &Translator, flows: &[Flow]) {
    chart_title(ui, rect, &t.text("finances-chart-months", None));
    let p = ui.painter();
    // the legend
    let names = [(t.text("finances-chart-income-legend", None), BLUE), (t.text("finances-chart-expenses-legend", None), ORANGE)];
    let widths: Vec<f32> = names.iter().map(|(n, _)| p.layout_no_wrap(n.clone(), theme::regular(11.0), theme::TEXT).size().x).collect();
    let mut x = rect.right() - RIGHT - widths.iter().sum::<f32>() - 44.0;
    for ((name, color), w) in names.iter().zip(&widths) {
        p.rect_filled(Rect::from_min_size(pos2(x, rect.top() + 11.0), vec2(10.0, 10.0)), 3.0, *color);
        p.text(pos2(x + 14.0, rect.top() + 20.0), egui::Align2::LEFT_BOTTOM, name, theme::regular(11.0), theme::TEXT);
        x += 14.0 + w + 16.0;
    }
    let mut months: Vec<&Flow> = flows.iter().filter(|f| !f.partial).collect();
    while months.len() > 12 {
        months.remove(0);
    }
    if months.is_empty() {
        empty_chart(ui, rect, &t.text("finances-chart-months-empty", None));
        return;
    }
    let max = months.iter().map(|f| f.income().max(f.expenses()) as f64).fold(0.0, f64::max);
    let s = scale(0.0, max);
    grid(ui, rect, t, s);
    let p = ui.painter();
    let width = rect.width() - LEFT - RIGHT;
    let group = width / months.len() as f32;
    let bar = ((group - 10.0) / 2.0).clamp(3.0, 26.0);
    let zero = y_of(rect, 0.0, s);
    let hover = ui.input(|i| i.pointer.hover_pos()).filter(|h| rect.contains(*h) && h.x >= rect.left() + LEFT).map(|h| ((h.x - rect.left() - LEFT) / group) as usize);
    for (i, f) in months.iter().enumerate() {
        let centre = rect.left() + LEFT + group * i as f32 + group / 2.0;
        if hover == Some(i) {
            p.rect_filled(Rect::from_min_max(pos2(rect.left() + LEFT + group * i as f32, rect.top() + TOP), pos2(rect.left() + LEFT + group * (i + 1) as f32, zero)), 0.0, Color32::from_white_alpha(14));
        }
        for (value, left, color) in [(f.income(), centre - 1.0 - bar, BLUE), (f.expenses(), centre + 1.0, ORANGE)] {
            let top = y_of(rect, value as f64, s);
            if zero - top > 0.0 {
                p.rect_filled(Rect::from_min_max(pos2(left, top), pos2(left + bar, zero)), egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 }, color);
            }
        }
        p.text(pos2(centre, rect.bottom() - 8.0), egui::Align2::CENTER_BOTTOM, month_name(t, f.month), theme::regular(10.0), theme::MUTED);
    }
    if let Some(i) = hover.filter(|&i| i < months.len()) {
        let f = months[i];
        let centre = rect.left() + LEFT + group * i as f32 + group / 2.0;
        let lines = [
            format!("{}/{}", month_name(t, f.month), f.month / 100),
            t.text("finances-chart-tip-income", Some(&args([("amount", money(t, f.income()))]))),
            t.text("finances-chart-tip-expenses", Some(&args([("amount", money(t, f.expenses()))]))),
            t.text("finances-chart-tip-balance", Some(&args([("amount", signed(t, f.income() - f.expenses()))]))),
        ];
        tip(ui, rect, pos2(centre, rect.top() + TOP + 40.0), &lines);
    }
}

/// `Composicao`: each line's share of the total, the biggest first.
fn composition(ui: &mut egui::Ui, rect: Rect, t: &Translator, title: &str, names: &[String], values: &[i64], color: Color32) {
    chart_title(ui, rect, title);
    let mut order: Vec<usize> = (0..names.len()).collect();
    order.sort_by(|&a, &b| values[b].cmp(&values[a]));
    let total: i64 = values.iter().map(|&v| v.max(0)).sum();
    if total <= 0 {
        empty_chart(ui, rect, &t.text("finances-chart-composition-empty", None));
        return;
    }
    let p = ui.painter();
    let x0 = rect.left() + 12.0 + 140.0;
    let width = rect.width() - 140.0 - 120.0 - 24.0;
    let row = ((rect.height() - TOP - 8.0) / names.len() as f32).clamp(16.0, 26.0);
    let max = values[order[0]].max(1) as f32;
    for (k, &i) in order.iter().enumerate() {
        let y = rect.top() + TOP + k as f32 * row + row / 2.0;
        let v = values[i].max(0);
        p.text(pos2(rect.left() + 12.0, y), egui::Align2::LEFT_CENTER, &names[i], theme::regular(11.0), theme::TEXT);
        p.rect_filled(Rect::from_min_size(pos2(x0, y - 5.0), vec2(width, 10.0)), 4.0, GRID);
        if v > 0 {
            p.rect_filled(Rect::from_min_size(pos2(x0, y - 5.0), vec2((width * v as f32 / max).max(4.0), 10.0)), 4.0, color);
        }
        let label = format!("{}  {}%", money(t, v), brasfoot_model::rng::java_round(100.0 * v as f64 / total as f64));
        p.text(pos2(rect.right() - 12.0, y), egui::Align2::RIGHT_CENTER, label, theme::regular(11.0), if v > 0 { theme::TEXT } else { theme::MUTED });
    }
}

// Statement

fn statement_tab(ui: &mut egui::Ui, s: &mut CareerScreen, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let season = world.season();
    let flows: Vec<Flow> = cash::flows(world).into_iter().filter(|f| f.season == season).collect();
    let last = season > 1 && cash::has_season(world, season - 1);
    let mut columns: Vec<(String, f32)> = vec![(String::new(), 170.0)];
    for f in &flows {
        let name = month_name(t, f.month);
        columns.push((if f.partial { t.text("finances-statement-until", Some(&args([("month", name)]))) } else { name }, 80.0));
    }
    columns.push((t.text("finances-statement-season", None), 80.0));
    if last {
        columns.push((t.text("finances-statement-last-year", None), 80.0));
    }
    let total = cash::season_totals(world, season);
    let before = last.then(|| cash::season_totals(world, season - 1));
    let n = columns.len();
    let title = |key: &str| {
        let mut row = vec![Cell::colored(t.text(key, None), GOLD)];
        row[0].bold = true;
        row.resize(n, Cell::default());
        row
    };
    let line = |name: String, i: usize, strong: bool| {
        let mut row = vec![if strong { Cell::strong(name) } else { Cell::from(name) }];
        for f in &flows {
            row.push(Cell::from(if f.values[i] == 0 { "-".to_owned() } else { money(t, f.values[i]) }).right());
        }
        row.push(Cell::strong(money(t, total[i])).right());
        if let Some(b) = &before {
            row.push(Cell::from(money(t, b[i])).right());
        }
        row
    };
    let sum = |v: &[i64], a: usize, b: usize| v[a..b].iter().sum::<i64>();
    let totals = |key: &str, a: usize, b: usize| {
        let mut row = vec![Cell::strong(t.text(key, None))];
        for f in &flows {
            row.push(Cell::strong(money(t, sum(&f.values, a, b))).right());
        }
        row.push(Cell::strong(money(t, sum(&total, a, b))).right());
        if let Some(bv) = &before {
            row.push(Cell::strong(money(t, sum(bv, a, b))).right());
        }
        row
    };
    let all = INCOME_LINES + EXPENSE_LINES;
    let mut rows = vec![title("finances-statement-income")];
    for i in 0..INCOME_LINES {
        rows.push(line(line_name(t, i), i, false));
    }
    rows.push(totals("finances-statement-total-income", 0, INCOME_LINES));
    rows.push(title("finances-statement-expenses"));
    for i in INCOME_LINES..all {
        rows.push(line(line_name(t, i), i, false));
    }
    rows.push(totals("finances-statement-total-expenses", INCOME_LINES, all));
    let mut balance = vec![Cell::strong(t.text("finances-statement-balance", None))];
    for f in &flows {
        balance.push(Cell::strong(signed(t, f.income() - f.expenses())).right());
    }
    balance.push(Cell::strong(signed(t, sum(&total, 0, INCOME_LINES) - sum(&total, INCOME_LINES, all))).right());
    if let Some(b) = &before {
        balance.push(Cell::strong(signed(t, sum(b, 0, INCOME_LINES) - sum(b, INCOME_LINES, all))).right());
    }
    rows.push(balance);
    modkit::text(ui, &t.text("finances-statement-note", None), 11.0, theme::MUTED);
    ui.add_space(6.0);
    let mut none = None;
    modkit::select_table(ui, files, "finances-statement", &columns, &rows, &mut none, ui.available_height());
}

// Wage bill

fn wages_tab(ui: &mut egui::Ui, s: &mut CareerScreen, me: ClubId, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let clock = &s.career.runtime.clock;
    let total = world.club_wages(me);
    let year = brasfoot_systems::economy::year(world);
    let today = world.today_millis();
    let (mut expiring, mut expiring_wages) = (0, 0i64);
    let mut list: Vec<(brasfoot_model::ids::PlayerId, bool)> = Vec::new();
    for (pro, squad) in [(true, &world.club(me).squad), (false, &world.club(me).youth_squad)] {
        for &p in squad {
            list.push((p, pro));
        }
    }
    list.sort_by(|a, b| world.player(b.0).salary.cmp(&world.player(a.0).salary));
    let rows: Vec<Vec<Cell>> = list
        .iter()
        .map(|&(p, pro)| {
            let x = world.player(p);
            let days = x.contract_days_left(today);
            let end = clock.add_days(today, days.max(0) as i32);
            let ends = days <= 0 || clock.fields(end).year <= year;
            if ends {
                expiring += 1;
                expiring_wages += x.salary as i64;
            }
            let contract = if days <= 0 {
                t.text("finances-wages-expired", None)
            } else {
                let d = format::date(t, clock, end);
                if ends { t.text("finances-wages-ends", Some(&args([("date", d)]))) } else { d }
            };
            vec![
                Cell::flagged(&x.name, x.country),
                Cell::from(widgets::position_name(t, x.position)),
                Cell::from(x.age),
                Cell::from(x.strength),
                Cell::from(money(t, x.salary as i64)).right(),
                Cell::from(if total > 0 { format!("{}%", format::decimal(t, 100.0 * x.salary as f64 / total as f64, 1)) } else { "-".to_owned() }).right(),
                Cell::from(contract),
                Cell::from(money(t, x.market_value as i64)).right(),
                Cell::from(t.text(if pro { "finances-wages-pro" } else { "finances-wages-youth" }, None)),
            ]
        })
        .collect();
    let league = league_clubs(world, me);
    let mut bills: Vec<i64> = league.iter().map(|&c| world.club_wages(c)).collect();
    bills.sort_by(|a, b| b.cmp(a));
    let rank = bills.iter().position(|&w| w == total).map(|i| i + 1);
    let monthly = cash::monthly_wages(world, me);
    ui.columns(4, |cols| {
        kpi(&mut cols[0], &t.text("finances-wages-monthly", None), &money(t, monthly), &t.text("finances-wages-yearly", Some(&args([("amount", money(t, monthly * 12))]))), None);
        kpi(
            &mut cols[1],
            &t.text("finances-wages-league", None),
            &rank.map(|r| t.text("finances-wages-rank", Some(&args([("rank", r.to_string())])))).unwrap_or_else(|| "-".into()),
            &t.text("finances-wages-of", Some(&args([("count", bills.len().to_string())]))),
            None,
        );
        let (top, top_salary) = list.first().map(|&(p, _)| (world.player(p).name.clone(), t.text("finances-wages-per-month", Some(&args([("amount", money(t, world.player(p).salary as i64))]))))).unwrap_or(("-".into(), String::new()));
        kpi(&mut cols[2], &t.text("finances-wages-top", None), &top, &top_salary, None);
        kpi(
            &mut cols[3],
            &t.text("finances-wages-expiring", None),
            &t.text("finances-wages-expiring-count", Some(&args([("count", expiring.to_string())]))),
            &t.text("finances-wages-expiring-detail", Some(&args([("year", year.to_string()), ("amount", money(t, expiring_wages))]))),
            (expiring > 0).then_some(GOLD),
        );
    });
    ui.add_space(8.0);
    let period = if world.game.monthly_salary { "finances-wages-column-salary-month" } else { "finances-wages-column-salary-week" };
    let columns = modkit::columns(
        t,
        &[
            ("finances-wages-column-player", 220.0),
            ("finances-wages-column-position", 80.0),
            ("finances-wages-column-age", 50.0),
            ("finances-wages-column-strength", 50.0),
            (period, 100.0),
            ("finances-wages-column-share", 80.0),
            ("finances-wages-column-contract", 150.0),
            ("finances-wages-column-value", 115.0),
            ("finances-wages-column-squad", 95.0),
        ],
    );
    let mut none = None;
    modkit::select_table(ui, files, "finances-wages", &columns, &rows, &mut none, ui.available_height());
}

/// `clubesDaLiga()`: the clubs of the same league and division (`Tecnicos.campanhas()`).
fn league_clubs(world: &World, me: ClubId) -> Vec<ClubId> {
    let campaigns = brasfoot_systems::awards::coaches::campaigns(world);
    let Some(mine) = campaigns.get(&me) else { return Vec::new() };
    let mut out: Vec<ClubId> = campaigns.values().filter(|k| k.league == mine.league && k.division == mine.division).map(|k| k.club).collect();
    out.sort();
    out
}

// Sponsorship

fn sponsors_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut FinancesState, me: ClubId, t: &Translator, files: &GameFiles) {
    let career = &mut s.career;
    if !career.systems.config.on(Switch::Sponsorship) {
        modkit::text(ui, &t.text("finances-sponsors-off", None), 13.0, theme::MUTED);
        return;
    }
    // Cotas.tick() when the tab is built, and the market values
    let (markets, master_market) = career.systems.with(&mut career.world, &mut career.runtime, |m| {
        m.sponsor_slots_tick();
        let markets: Vec<i64> = SponsorSlot::ALL.iter().map(|&slot| m.slot_market_value(me, slot)).collect();
        (markets, brasfoot_model::rng::java_round(m.master_base(me)))
    });
    let world = &career.world;
    let config = career.systems.config.clone();
    let master = world.mods.master_sponsor.active().cloned();
    let total = brasfoot_systems::sponsors::slots_yearly(world) + master.as_ref().map_or(0, |k| k.yearly);
    let mut action: Option<SponsorAction> = None;
    ui.horizontal(|ui| {
        let line = t.text("finances-sponsors-total", Some(&args([("yearly", money(t, total)), ("monthly", money(t, total / 12)), ("members", money(t, brasfoot_systems::fans::monthly_members(world, &config)))])));
        ui.label(egui::RichText::new(line).font(theme::bold(14.0)).color(Color32::WHITE));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if modkit::button(ui, &t.text("finances-sponsors-history", None), false, true).clicked() {
                action = Some(SponsorAction::History);
            }
            if modkit::button(ui, &t.text("finances-sponsors-search-all", None), false, true).clicked() {
                action = Some(SponsorAction::SearchAll);
            }
        });
    });
    ui.add_space(8.0);
    let slots = &world.mods.sponsor_slots;
    let mut cards = vec![Card::Master { contract: master.clone(), market: master_market }];
    for (si, &slot) in SponsorSlot::ALL.iter().enumerate() {
        cards.push(Card::Slot {
            slot,
            deal: slots.deals.iter().position(|k| k.slot == slot).map(|i| (i, slots.deals[i].clone())),
            searching: slots.searches.iter().find(|x| x.0 == slot).map(|x| x.1),
            offers: slots.offers.iter().filter(|o| o.slot == slot).count(),
            market: markets[si],
        });
    }
    cards.push(Card::How);
    for row in cards.chunks(4) {
        ui.columns(4, |cols| {
            for (card, col) in row.iter().zip(cols.iter_mut()) {
                if let Some(a) = draw_card(col, t, card) {
                    action = Some(a);
                }
            }
        });
        ui.add_space(8.0);
    }
    // the offers on the table
    let world = &s.career.world;
    let offers = world.mods.sponsor_slots.offers.clone();
    ui.label(egui::RichText::new(t.text("finances-sponsors-table", None)).font(theme::bold(12.0)).color(GOLD));
    ui.add_space(4.0);
    if offers.is_empty() {
        modkit::text(ui, &t.text("finances-sponsors-none", None), 13.0, theme::MUTED);
    } else {
        let rows: Vec<Vec<Cell>> = offers
            .iter()
            .map(|k| {
                vec![
                    Cell::from(labels::sponsor_slot(t, k.slot, false)),
                    Cell::from(brasfoot_model::brands::slot_brand(k.brand)),
                    Cell::from(labels::slot_kind(t, k.kind)),
                    Cell::from(money(t, k.yearly)),
                    Cell::from(k.years),
                    Cell::from(if k.bonus > 0 { money(t, k.bonus) } else { "-".into() }),
                    Cell::from(date_of(k.valid_until)),
                ]
            })
            .collect();
        let columns = modkit::columns(
            t,
            &[
                ("finances-sponsors-column-slot", 150.0),
                ("finances-sponsors-column-company", 200.0),
                ("finances-sponsors-column-kind", 100.0),
                ("finances-sponsors-column-yearly", 100.0),
                ("finances-sponsors-column-seasons", 90.0),
                ("finances-sponsors-column-bonus", 120.0),
                ("finances-sponsors-column-until", 100.0),
            ],
        );
        let height = ui.available_height() - 40.0;
        modkit::select_table(ui, files, "finances-offers", &columns, &rows, &mut state.offer, height);
        ui.horizontal(|ui| {
            if modkit::button(ui, &t.text("finances-sponsors-sign", None), false, true).clicked()
                && let Some(i) = state.offer
            {
                action = Some(SponsorAction::Sign(i));
            }
            if modkit::button(ui, &t.text("finances-sponsors-refuse", None), false, true).clicked()
                && let Some(i) = state.offer
            {
                action = Some(SponsorAction::Refuse(i));
            }
        });
    }
    let career = &mut s.career;
    match action {
        Some(SponsorAction::History) => state.history_open = true,
        Some(SponsorAction::SearchAll) => {
            let n = career.systems.with(&mut career.world, &mut career.runtime, |m| {
                let mut n = 0;
                for slot in SponsorSlot::ALL {
                    let st = &m.world.mods.sponsor_slots;
                    let free = !st.deals.iter().any(|k| k.slot == slot) && !st.searches.iter().any(|x| x.0 == slot) && !st.offers.iter().any(|o| o.slot == slot);
                    if free && m.search_sponsor_slot(slot).is_ok() {
                        n += 1;
                    }
                }
                n
            });
            let text = if n == 0 { t.text("finances-sponsors-search-all-none", None) } else { t.text("finances-sponsors-search-all-done", Some(&args([("count", n as i64)]))) };
            state.question = Some(modkit::message(String::new(), text));
        }
        Some(SponsorAction::OpenMaster) => career.systems.with(&mut career.world, &mut career.runtime, |m| m.open_master_offers()),
        Some(SponsorAction::EndMaster(fine)) => {
            let brand = master.map(|k| brasfoot_model::brands::MASTER_BRANDS[k.brand as usize].to_owned()).unwrap_or_default();
            state.question = Some(modkit::confirm(t.text("finances-confirm", None), t.text("finances-sponsors-end-master-confirm", Some(&args([("brand", brand), ("fine", money(t, fine))])))));
            state.pending = Some(Pending::EndMaster);
        }
        Some(SponsorAction::EndSlot(i, brand, fine)) => {
            state.question = Some(modkit::confirm(t.text("finances-confirm", None), t.text("finances-sponsors-end-slot-confirm", Some(&args([("brand", brand), ("fine", money(t, fine))])))));
            state.pending = Some(Pending::EndSlot(i));
        }
        Some(SponsorAction::Search(slot)) => {
            if let Err(e) = career.systems.with(&mut career.world, &mut career.runtime, |m| m.search_sponsor_slot(slot)) {
                let key = match e {
                    brasfoot_systems::sponsors::SearchError::NoClub => "finances-sponsors-error-no-club",
                    brasfoot_systems::sponsors::SearchError::Taken => "finances-sponsors-error-taken",
                    brasfoot_systems::sponsors::SearchError::Searching => "finances-sponsors-error-searching",
                };
                state.question = Some(modkit::message(String::new(), t.text(key, None)));
            }
        }
        Some(SponsorAction::Sign(i)) => career.systems.with(&mut career.world, &mut career.runtime, |m| m.sign_sponsor_slot(i)),
        Some(SponsorAction::Refuse(i)) => career.systems.with(&mut career.world, &mut career.runtime, |m| m.refuse_sponsor_slot(i)),
        None => {}
    }
}

/// A card of the sponsorship grid.
enum Card {
    Master { contract: Option<brasfoot_model::mods::MasterContract>, market: i64 },
    Slot { slot: SponsorSlot, deal: Option<(usize, brasfoot_model::mods::SlotContract)>, searching: Option<i32>, offers: usize, market: i64 },
    How,
}

/// `cartaoMaster`, `cartaoCota`, `cartaoTexto`: the card and what its button asks.
fn draw_card(ui: &mut egui::Ui, t: &Translator, card: &Card) -> Option<SponsorAction> {
    const HEIGHT: f32 = 118.0;
    let mut action = None;
    match card {
        Card::Master { contract, market } => {
            let brand = contract.as_ref().map(|k| brasfoot_model::brands::MASTER_BRANDS[k.brand as usize].to_owned()).unwrap_or_else(|| t.text("finances-sponsors-vacant", None));
            card_base(ui, &t.text("finances-sponsors-master", None), &brand, contract.is_some(), HEIGHT, |ui| match contract {
                Some(k) => {
                    ui.label(egui::RichText::new(t.text("finances-sponsors-per-year", Some(&args([("amount", money(t, k.yearly)), ("kind", labels::master_kind(t, k.kind))])))).font(theme::regular(12.0)));
                    ui.label(egui::RichText::new(years_left(t, k.years)).font(theme::regular(11.0)).color(theme::MUTED));
                    let fine = brasfoot_systems::sponsors::early_end_fine(k.yearly, k.years);
                    if modkit::button(ui, &t.text("finances-sponsors-end", Some(&args([("fine", money(t, fine))]))), false, true).clicked() {
                        action = Some(SponsorAction::EndMaster(fine));
                    }
                }
                None => {
                    ui.label(egui::RichText::new(t.text("finances-sponsors-market", Some(&args([("amount", money(t, *market))])))).font(theme::regular(12.0)));
                    ui.label(" ");
                    if modkit::button(ui, &t.text("finances-sponsors-see-offers", None), false, true).clicked() {
                        action = Some(SponsorAction::OpenMaster);
                    }
                }
            });
        }
        Card::Slot { slot, deal, searching, offers, market } => {
            let brand = deal.as_ref().map(|(_, k)| brasfoot_model::brands::slot_brand(k.brand)).unwrap_or_else(|| t.text("finances-sponsors-vacant", None));
            card_base(ui, &labels::sponsor_slot(t, *slot, false), &brand, deal.is_some(), HEIGHT, |ui| match deal {
                Some((i, k)) => {
                    ui.label(egui::RichText::new(t.text("finances-sponsors-per-year", Some(&args([("amount", money(t, k.yearly)), ("kind", labels::slot_kind(t, k.kind))])))).font(theme::regular(12.0)));
                    let mut line = years_left(t, k.years);
                    if k.bonus > 0 {
                        line += &t.text("finances-sponsors-bonus", Some(&args([("amount", money(t, k.bonus))])));
                    }
                    ui.label(egui::RichText::new(line).font(theme::regular(11.0)).color(theme::MUTED));
                    let fine = brasfoot_systems::sponsors::early_end_fine(k.yearly, k.years);
                    if modkit::button(ui, &t.text("finances-sponsors-end", Some(&args([("fine", money(t, fine))]))), false, true).clicked() {
                        action = Some(SponsorAction::EndSlot(*i, brasfoot_model::brands::slot_brand(k.brand), fine));
                    }
                }
                None => {
                    ui.label(egui::RichText::new(t.text("finances-sponsors-market", Some(&args([("amount", money(t, *market))])))).font(theme::regular(12.0)));
                    if let Some(day) = searching {
                        ui.label(egui::RichText::new(t.text("finances-sponsors-searching", Some(&args([("date", date_of(*day)[..5].to_owned())])))).font(theme::regular(11.0)).color(GOLD));
                    } else if *offers > 0 {
                        ui.label(egui::RichText::new(t.text("finances-sponsors-offers", Some(&args([("count", *offers as i64)])))).font(theme::bold(11.0)).color(GOLD));
                    } else {
                        ui.label(" ");
                    }
                    let key = if *offers == 0 { "finances-sponsors-search" } else { "finances-sponsors-search-more" };
                    if modkit::button(ui, &t.text(key, None), false, searching.is_none()).clicked() {
                        action = Some(SponsorAction::Search(*slot));
                    }
                }
            });
        }
        Card::How => card_base(ui, &t.text("finances-sponsors-how", None), "", false, HEIGHT, |ui| modkit::text(ui, &t.text("finances-sponsors-how-text", None), 10.0, theme::MUTED)),
    }
    action
}

enum SponsorAction {
    History,
    SearchAll,
    OpenMaster,
    EndMaster(i64),
    EndSlot(usize, String, i64),
    Search(SponsorSlot),
    Sign(usize),
    Refuse(usize),
}

fn years_left(t: &Translator, years: i32) -> String {
    t.text("finances-sponsors-years-left", Some(&args([("years", years as i64)])))
}

/// `cartaoBase(cota, marca, ocupada)`
fn card_base(ui: &mut egui::Ui, slot: &str, brand: &str, taken: bool, height: f32, add: impl FnOnce(&mut egui::Ui)) {
    modkit::item(Some(if taken { GOLD_DARK } else { theme::BORDER }), None).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(height - 14.0);
        ui.label(egui::RichText::new(slot.to_uppercase()).font(theme::bold(10.0)).color(GOLD));
        ui.add_space(2.0);
        if !brand.is_empty() {
            ui.label(egui::RichText::new(brand).font(theme::bold(14.0)).color(if taken { Color32::WHITE } else { theme::MUTED }));
            ui.add_space(4.0);
        }
        add(ui);
    });
}

/// "Histórico": the master's and the slots' notes, newest text first.
fn history_window(s: &CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let world = &s.career.world;
    let millions = |v: i64| format::mi(t, v);
    let mut lines: Vec<String> = world.mods.master_sponsor.history.iter().map(|n| labels::master_note(t, n, &millions)).collect();
    lines.extend(world.mods.sponsor_slots.history.iter().map(|n| labels::slot_note(t, n, &millions)));
    lines.sort_by(|a, b| b.encode_utf16().cmp(a.encode_utf16()));
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(16);
    motion::dialog(ctx, "sponsor-history", open, vec2(760.0, 460.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, None, &t.text("finances-sponsors-history-title", None)) {
            *open = false;
        }
        let rows: Vec<Vec<Cell>> = lines.iter().map(|l| vec![Cell::from(l)]).collect();
        let columns = vec![(t.text("finances-sponsors-history-title", None), 700.0)];
        let mut none = None;
        modkit::select_table(ui, files, "sponsor-history", &columns, &rows, &mut none, ui.available_height());
    });
}

// Transfers

fn transfers_tab(ui: &mut egui::Ui, s: &mut CareerScreen, me: ClubId, t: &Translator, files: &GameFiles) {
    let world = &s.career.world;
    let id = world.club(me).id;
    let by_id = brasfoot_systems::economy::clubs_by_id(world);
    let mut by_year: std::collections::BTreeMap<std::cmp::Reverse<i32>, [i64; 4]> = std::collections::BTreeMap::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for x in world.game.transfers.iter().rev() {
        let buy = x.to_club == id;
        let sell = x.from_club == id;
        if (!buy && !sell) || x.fee <= 0 {
            continue;
        }
        let other = by_id.get(&if buy { x.from_club } else { x.to_club }).map(|&c| world.club(c).name.clone());
        let e = by_year.entry(std::cmp::Reverse(x.year)).or_insert([0; 4]);
        e[if buy { 0 } else { 2 }] += x.fee as i64;
        e[if buy { 1 } else { 3 }] += 1;
        rows.push(vec![
            // ap.f(): day, month and year without zeros
            Cell::from(format!("{}/{}/{}", x.day, x.month, x.year)),
            x.player.map(|p| Cell::flagged(&world.player(p).name, world.player(p).country)).unwrap_or_else(|| Cell::from("-")),
            Cell::from(t.text(if buy { "finances-transfers-buy" } else { "finances-transfers-sale" }, None)),
            Cell::from(other.unwrap_or_else(|| "-".into())),
            Cell::colored(money(t, x.fee as i64), if buy { NEGATIVE } else { POSITIVE }),
        ]);
    }
    if rows.is_empty() {
        modkit::text(ui, &t.text("finances-transfers-none", None), 13.0, theme::MUTED);
        return;
    }
    let years: Vec<Vec<Cell>> = by_year
        .iter()
        .map(|(y, v)| {
            vec![
                Cell::from(y.0),
                Cell::from(v[1]),
                Cell::from(money(t, v[0])),
                Cell::from(v[3]),
                Cell::from(money(t, v[2])),
                Cell::colored(signed(t, v[2] - v[0]), if v[2] - v[0] < 0 { NEGATIVE } else { POSITIVE }),
            ]
        })
        .collect();
    ui.label(egui::RichText::new(t.text("finances-transfers-by-year", None)).font(theme::bold(12.0)).color(GOLD));
    ui.add_space(4.0);
    let columns = modkit::columns(
        t,
        &[
            ("finances-transfers-column-year", 80.0),
            ("finances-transfers-column-buys", 80.0),
            ("finances-transfers-column-spent", 120.0),
            ("finances-transfers-column-sales", 80.0),
            ("finances-transfers-column-raised", 120.0),
            ("finances-transfers-column-balance", 120.0),
        ],
    );
    let mut none = None;
    modkit::select_table(ui, files, "finances-years", &columns, &years, &mut none, 150.0);
    ui.add_space(8.0);
    ui.label(egui::RichText::new(t.text("finances-transfers-all", None)).font(theme::bold(12.0)).color(GOLD));
    ui.add_space(4.0);
    let columns = modkit::columns(
        t,
        &[
            ("finances-transfers-column-date", 100.0),
            ("finances-transfers-column-player", 260.0),
            ("finances-transfers-column-operation", 90.0),
            ("finances-transfers-column-club", 260.0),
            ("finances-transfers-column-fee", 120.0),
        ],
    );
    let mut none = None;
    modkit::select_table(ui, files, "finances-transfers", &columns, &rows, &mut none, ui.available_height());
}

// Bank and credit

fn bank_tab(ui: &mut egui::Ui, s: &mut CareerScreen, state: &mut FinancesState, me: ClubId, t: &Translator, files: &GameFiles) {
    let career = &mut s.career;
    if !career.systems.config.on(Switch::Credit) {
        modkit::text(ui, &t.text("finances-bank-off", None), 13.0, theme::MUTED);
        return;
    }
    struct Bank {
        rating: brasfoot_systems::credit::Rating,
        limit: i64,
        debt: i64,
        available: i64,
        status: brasfoot_model::mods::CreditStatus,
        loans: Vec<brasfoot_model::mods::BankLoan>,
        ban: bool,
        owner: brasfoot_model::mods::EconomyClub,
        rates: [f64; 3],
    }
    let b = career.systems.with(&mut career.world, &mut career.runtime, |m| Bank {
        rating: m.credit_rating(me),
        limit: m.credit_limit(me),
        debt: m.active_debt(me),
        available: m.available_credit(me),
        status: m.credit_status(me),
        loans: m.bank_loans(me),
        ban: m.transfer_ban(me),
        owner: m.club(me).clone(),
        rates: [m.interest_rate(me, 12), m.interest_rate(me, 24), m.interest_rate(me, 36)],
    });
    let world = &career.world;
    let config = career.systems.config.clone();
    let mut quick: Option<bool> = None;
    let mut repay: Option<usize> = None;
    let mut borrow: Option<(i64, i32)> = None;
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        // rating and capacity
        panel(ui, &t.text("finances-bank-rating-title", None), |ui| {
            ui.horizontal(|ui| {
                let color = rating_color(b.rating);
                let name = rating_name(b.rating);
                egui::Frame::new().fill(color).inner_margin(egui::Margin::symmetric(8, 2)).show(ui, |ui| {
                    ui.label(egui::RichText::new(name).font(theme::bold(16.0)).color(Color32::BLACK));
                });
                ui.label(egui::RichText::new(t.text(&format!("credit-rating-{}", format!("{:?}", b.rating).to_lowercase()), None)).font(theme::bold(12.0)).color(color));
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(t.text("finances-bank-limit", Some(&args([("amount", money(t, b.limit))])))).font(theme::regular(12.0)));
            ui.label(egui::RichText::new(t.text("finances-bank-debt", Some(&args([("amount", money(t, b.debt))])))).font(theme::bold(12.0)).color(if b.debt > 0 { NEGATIVE } else { theme::TEXT }));
            ui.label(egui::RichText::new(t.text("finances-bank-available", Some(&args([("amount", money(t, b.available))])))).font(theme::bold(12.0)).color(if b.available > 0 { POSITIVE } else { theme::MUTED }));
            ui.add_space(6.0);
            usage_bar(ui, t, b.debt, b.limit.max(1));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t.text("finances-bank-status", None)).font(theme::bold(12.0)));
                let (key, color) = stage(b.status.stage);
                ui.label(egui::RichText::new(t.text(key, None)).font(theme::bold(12.0)).color(color));
            });
            modkit::text(ui, &t.text(&format!("{}-description", stage(b.status.stage).0), None), 11.0, theme::MUTED);
        });
        ui.add_space(8.0);
        // the owner
        panel(ui, &t.text("finances-bank-owner-title", None), |ui| {
            if b.owner.has_owner {
                let complaints = world.mods.investor.complaints;
                ui.label(egui::RichText::new(b.owner.owner_name.map(brasfoot_model::brands::owner).unwrap_or_default()).font(theme::bold(16.0)).color(Color32::WHITE));
                ui.label(egui::RichText::new(t.text("finances-bank-owner-since", Some(&args([("year", b.owner.owner_since.to_string())])))).font(theme::regular(11.0)).color(theme::MUTED));
                ui.add_space(6.0);
                ui.label(egui::RichText::new(t.text("finances-bank-owner-yearly", Some(&args([("amount", money(t, b.owner.owner_funding))])))).font(theme::bold(13.0)));
                let next = if complaints == 1 { brasfoot_model::rng::java_round(b.owner.owner_funding as f64 * 0.6) } else { b.owner.owner_funding };
                ui.label(egui::RichText::new(t.text("finances-bank-owner-next", Some(&args([("amount", money(t, next))])))).font(theme::regular(12.0)));
                ui.add_space(6.0);
                let (key, color) = if complaints == 0 { ("finances-bank-owner-calm", POSITIVE) } else { ("finances-bank-owner-warned", NEGATIVE) };
                ui.label(egui::RichText::new(t.text(key, None)).font(theme::bold(12.0)).color(color));
            } else {
                ui.label(egui::RichText::new(t.text("finances-kpi-no-owner", None)).font(theme::bold(16.0)).color(Color32::WHITE));
                ui.add_space(6.0);
                let key = if config.on(Switch::Investor) { "finances-bank-no-owner" } else { "finances-bank-no-owner-off" };
                modkit::text(ui, &t.text(key, Some(&args([("chance", brasfoot_systems::investor::CHANCE.to_string())]))), 11.0, theme::TEXT);
            }
        });
        ui.add_space(8.0);
        // the game's quick loan
        panel(ui, &t.text("finances-bank-quick-title", None), |ui| {
            let loan = world.club(me).finances.bank_loan as i64;
            ui.label(egui::RichText::new(t.text("finances-bank-quick", Some(&args([("amount", money(t, loan))])))).font(theme::regular(12.0)).color(if loan > 0 { NEGATIVE } else { theme::TEXT }));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if modkit::button(ui, &t.text("finances-bank-quick-take", None), false, true).clicked() {
                    quick = Some(true);
                }
                if modkit::button(ui, &t.text("finances-bank-quick-pay", None), false, true).clicked() {
                    quick = Some(false);
                }
            });
        });
        let ui = &mut cols[1];
        panel(ui, &t.text("finances-bank-loans-title", None), |ui| {
            if b.loans.is_empty() {
                ui.label(egui::RichText::new(t.text("finances-bank-loans-none", None)).font(theme::regular(12.0)).italics().color(theme::MUTED));
                ui.add_space(14.0);
            } else {
                let rows: Vec<Vec<Cell>> = b
                    .loans
                    .iter()
                    .map(|l| {
                        let late = if l.months_late == 0 { t.text("finances-bank-on-time", None) } else { t.text("finances-bank-late", Some(&args([("months", l.months_late.to_string())]))) };
                        vec![Cell::from(format!("#{}", l.id)), Cell::from(money(t, l.original)), Cell::from(money(t, l.balance)), Cell::from(format!("{}/{}", l.paid, l.instalments)), Cell::from(money(t, l.instalment)), Cell::from(late)]
                    })
                    .collect();
                let columns = modkit::columns(
                    t,
                    &[
                        ("finances-bank-column-id", 40.0),
                        ("finances-bank-column-original", 90.0),
                        ("finances-bank-column-balance", 100.0),
                        ("finances-bank-column-instalments", 70.0),
                        ("finances-bank-column-monthly", 90.0),
                        ("finances-bank-column-late", 80.0),
                    ],
                );
                modkit::select_table(ui, files, "finances-loans", &columns, &rows, &mut state.loan, 110.0);
                ui.add_space(8.0);
                if modkit::button(ui, &t.text("finances-bank-repay", None), false, true).clicked() {
                    repay = Some(state.loan.unwrap_or(usize::MAX));
                }
            }
        });
        ui.add_space(8.0);
        panel(ui, &t.text("finances-bank-simulator-title", None), |ui| {
            if b.ban {
                ui.label(egui::RichText::new(t.text("finances-bank-ban", None)).font(theme::bold(13.0)).color(NEGATIVE));
                ui.add_space(4.0);
                modkit::text(ui, &t.text("finances-bank-ban-note", None), 11.0, theme::MUTED);
            } else if b.available <= 0 {
                ui.label(egui::RichText::new(t.text("finances-bank-exhausted", Some(&args([("rating", rating_name(b.rating))])))).font(theme::bold(13.0)).color(NEGATIVE));
                ui.add_space(4.0);
                modkit::text(ui, &t.text("finances-bank-exhausted-note", None), 11.0, theme::MUTED);
            } else {
                let mut values: Vec<i64> = AMOUNTS.iter().copied().chain(std::iter::once(b.available)).filter(|&v| v <= b.available).collect();
                values.dedup();
                let labels: Vec<String> = values.iter().map(|&v| if v == b.available { t.text("finances-bank-maximum", Some(&args([("amount", money(t, v))]))) } else { money(t, v) }).collect();
                state.amount = state.amount.min(values.len() - 1);
                ui.columns(2, |c| {
                    c[0].label(egui::RichText::new(t.text("finances-bank-amount", None)).font(theme::bold(11.0)));
                    egui::ComboBox::from_id_salt("finances-amount").width(c[0].available_width() - 8.0).selected_text(labels[state.amount].clone()).show_ui(&mut c[0], |ui| {
                        for (i, l) in labels.iter().enumerate() {
                            ui.selectable_value(&mut state.amount, i, l);
                        }
                    });
                    c[1].label(egui::RichText::new(t.text("finances-bank-term", None)).font(theme::bold(11.0)));
                    let terms: Vec<String> = TERMS.iter().map(|m| t.text("finances-bank-months", Some(&args([("months", m.to_string())])))).collect();
                    egui::ComboBox::from_id_salt("finances-term").width(c[1].available_width() - 8.0).selected_text(terms[state.term].clone()).show_ui(&mut c[1], |ui| {
                        for (i, l) in terms.iter().enumerate() {
                            ui.selectable_value(&mut state.term, i, l);
                        }
                    });
                });
                ui.add_space(8.0);
                let amount = values[state.amount];
                let months = TERMS[state.term];
                let rate = b.rates[state.term];
                let instalment = brasfoot_systems::credit::instalment(amount, months, rate);
                let paid = instalment * months as i64;
                ui.label(egui::RichText::new(t.text("finances-bank-rate", Some(&args([("rate", format::decimal(t, rate * 100.0, 2)), ("rating", rating_name(b.rating))])))).font(theme::regular(12.0)));
                ui.label(egui::RichText::new(t.text("finances-bank-instalment", Some(&args([("amount", money(t, instalment))])))).font(theme::bold(14.0)).color(GOLD));
                ui.label(egui::RichText::new(t.text("finances-bank-total", Some(&args([("total", money(t, paid)), ("interest", money(t, paid - amount))])))).font(theme::regular(12.0)).color(theme::MUTED));
                ui.add_space(8.0);
                if modkit::button(ui, &t.text("finances-bank-borrow", None), false, true).clicked() {
                    borrow = Some((amount, months));
                }
            }
        });
    });
    match quick {
        Some(true) => {
            if !quick_loan::borrow_emergency(&mut s.career.world, me) {
                state.question = Some(modkit::message(String::new(), t.text("finances-bank-quick-limit", None)));
            }
        }
        Some(false) => {
            let world = &mut s.career.world;
            if !quick_loan::repay_emergency(world, me) {
                let key = if world.club(me).finances.bank_loan <= 0 { "finances-bank-quick-no-debt" } else { "finances-bank-quick-no-cash" };
                state.question = Some(modkit::message(String::new(), t.text(key, None)));
            }
        }
        None => {}
    }
    if let Some(row) = repay {
        match b.loans.get(row) {
            None => state.question = Some(modkit::message(String::new(), t.text("finances-bank-repay-pick", None))),
            Some(l) => {
                state.question = Some(modkit::confirm(t.text("finances-confirm", None), t.text("finances-bank-repay-confirm", Some(&args([("id", l.id.to_string()), ("amount", money(t, l.balance))])))));
                state.pending = Some(Pending::RepayLoan(l.id));
            }
        }
    }
    if let Some((amount, months)) = borrow {
        let instalment = brasfoot_systems::credit::instalment(amount, months, b.rates[state.term]);
        state.question = Some(modkit::confirm(
            t.text("finances-confirm", None),
            t.text("finances-bank-borrow-confirm", Some(&args([("amount", money(t, amount)), ("months", months.to_string()), ("instalment", money(t, instalment))]))),
        ));
        state.pending = Some(Pending::Borrow(amount, months));
    }
}

/// `painel(titulo)`
fn panel(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    modkit::item(None, None).inner_margin(egui::Margin::symmetric(16, 14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new(title).font(theme::bold(12.0)).color(GOLD));
        ui.add_space(10.0);
        add(ui);
    });
}

/// `BarraDeUso`: the debt against the limit.
fn usage_bar(ui: &mut egui::Ui, t: &Translator, used: i64, limit: i64) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(360.0), 30.0), egui::Sense::hover());
    let track = Rect::from_min_size(rect.min + vec2(0.0, 4.0), vec2(rect.width() - 4.0, 12.0));
    ui.painter().rect_filled(track, 6.0, theme::ROW_ALT);
    let f = if limit > 0 { (used as f64 / limit as f64).min(1.0) } else { 0.0 };
    if f > 0.0 {
        let w = (track.width() * f as f32).max(6.0);
        ui.painter().rect_filled(Rect::from_min_size(track.min, vec2(w, 12.0)), 6.0, if f > 0.8 { NEGATIVE } else { GOLD });
    }
    let text = t.text("finances-bank-usage", Some(&args([("percent", (f * 100.0).round().to_string())])));
    ui.painter().text(pos2(rect.left(), rect.top() + 28.0), egui::Align2::LEFT_BOTTOM, text, theme::regular(10.0), theme::MUTED);
}

/// `Credito.Rating.cor`
fn rating_color(r: brasfoot_systems::credit::Rating) -> Color32 {
    use brasfoot_systems::credit::Rating::*;
    match r {
        AAA => Color32::from_rgb(0x38, 0xbd, 0xf8),
        AA => Color32::from_rgb(0x4c, 0xc3, 0x8a),
        A => Color32::from_rgb(0x5e, 0xea, 0xd4),
        BBB => Color32::from_rgb(0xa3, 0xe6, 0x35),
        BB => Color32::from_rgb(0xfa, 0xcc, 0x15),
        B => Color32::from_rgb(0xfb, 0x92, 0x3c),
        CCC => Color32::from_rgb(0xf8, 0x71, 0x71),
        D => Color32::from_rgb(0xef, 0x44, 0x44),
    }
}

/// `Credito.EstagioPenalidade`: its key and color.
fn stage(s: brasfoot_model::mods::CreditStage) -> (&'static str, Color32) {
    use brasfoot_model::mods::CreditStage::*;
    match s {
        Normal => ("credit-stage-normal", POSITIVE),
        Warning => ("credit-stage-warning", Color32::from_rgb(0xfa, 0xcc, 0x15)),
        TransferBan => ("credit-stage-transfer-ban", Color32::from_rgb(0xf8, 0x71, 0x71)),
        Intervention => ("credit-stage-intervention", Color32::from_rgb(0xef, 0x44, 0x44)),
    }
}

// League comparison

fn comparison_tab(ui: &mut egui::Ui, s: &mut CareerScreen, me: ClubId, t: &Translator, files: &GameFiles) {
    let career = &mut s.career;
    let clubs = league_clubs(&career.world, me);
    if clubs.is_empty() {
        modkit::text(ui, &t.text("finances-comparison-none", None), 13.0, theme::MUTED);
        return;
    }
    // {wages, squad value, cash estimate, master sponsor estimate}
    let mut data: Vec<(ClubId, [i64; 4])> = career.systems.with(&mut career.world, &mut career.runtime, |m| {
        clubs
            .iter()
            .map(|&c| {
                let squad: i64 = m.world.club(c).squad.iter().map(|&p| m.world.player(p).market_value as i64).sum();
                let master = m.club_sponsorship(c).map_or(0, |v| m.sponsorship_estimate(c, v));
                (c, [m.world.club_wages(c), squad, m.estimate(c), master])
            })
            .collect()
    });
    data.sort_by(|a, b| b.1[0].cmp(&a.1[0]));
    let mut rank = [0usize; 4];
    for (k, r) in rank.iter_mut().enumerate() {
        let mut order = data.clone();
        order.sort_by(|a, b| b.1[k].cmp(&a.1[k]));
        *r = order.iter().position(|x| x.0 == me).map_or(0, |i| i + 1);
    }
    let world = &career.world;
    let of = data.len();
    ui.columns(4, |cols| {
        for (i, (title, detail)) in [
            ("finances-comparison-wages", "finances-comparison-wages-detail"),
            ("finances-comparison-squad", "finances-comparison-squad-detail"),
            ("finances-comparison-cash", "finances-comparison-press"),
            ("finances-comparison-master", "finances-comparison-press"),
        ]
        .iter()
        .enumerate()
        {
            kpi(&mut cols[i], &t.text(title, None), &t.text("finances-comparison-rank", Some(&args([("rank", rank[i].to_string()), ("of", of.to_string())]))), &t.text(detail, None), None);
        }
    });
    ui.add_space(8.0);
    let rows: Vec<Vec<Cell>> = data
        .iter()
        .enumerate()
        .map(|(i, (c, d))| {
            let mine = *c == me;
            let name = if mine { t.text("finances-comparison-mine", Some(&args([("club", world.club(*c).name.clone())]))) } else { world.club(*c).name.clone() };
            let mut name = Cell::flagged(name, world.club(*c).country);
            name.bold = mine;
            vec![
                Cell::from(i + 1),
                name,
                Cell::from(money(t, d[0])),
                Cell::from(money(t, d[1])),
                Cell::from(if mine { format::money(t, d[2]) } else { format::approximate(t, d[2]) }),
                Cell::from(if d[3] > 0 { money(t, d[3]) } else { "-".into() }),
            ]
        })
        .collect();
    let columns = modkit::columns(
        t,
        &[
            ("finances-comparison-column-rank", 40.0),
            ("finances-comparison-column-club", 280.0),
            ("finances-comparison-column-wages", 130.0),
            ("finances-comparison-column-squad", 140.0),
            ("finances-comparison-column-cash", 140.0),
            ("finances-comparison-column-master", 140.0),
        ],
    );
    let mut none = None;
    modkit::select_table(ui, files, "finances-comparison", &columns, &rows, &mut none, ui.available_height());
}

// The game's own screen

/// `a.cW`: the season's ledger, the wage bill, the cash and the quick loan.
fn classic_window(s: &mut CareerScreen, me: ClubId, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(16);
    let mut quick = None;
    motion::dialog(ctx, "classic-finances", open, vec2(440.0, 640.0), frame, |ui, open| {
        let world = &s.career.world;
        let year = world.season() + world.game.start_year - 1;
        ui.horizontal(|ui| {
            widgets::crest(ui, files, world, me, 22.0);
            ui.label(egui::RichText::new(t.text("club-finances-title", Some(&args([("year", year.to_string())])))).font(theme::bold(14.0)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::close_button(ui) {
                    *open = false;
                }
            });
        });
        ui.add_space(8.0);
        let v = cash::totals(world, me);
        let income: i64 = v[..INCOME_LINES].iter().sum();
        let expenses: i64 = v[INCOME_LINES..].iter().sum();
        let mut rows: Vec<(String, Option<i64>, bool)> = vec![(t.text("club-finances-header-income", None), None, true)];
        for i in 0..INCOME_LINES {
            rows.push((t.text(&format!("club-finances-line-{i}"), None), Some(v[i]), false));
        }
        rows.push((t.text("club-finances-income-total", None), Some(income), true));
        rows.push((t.text("club-finances-header-expenses", None), None, true));
        for i in INCOME_LINES..INCOME_LINES + EXPENSE_LINES {
            rows.push((t.text(&format!("club-finances-line-{i}"), None), Some(v[i]), false));
        }
        rows.push((t.text("club-finances-expense-total", None), Some(expenses), true));
        rows.push((t.text("club-finances-balance", None), Some(income - expenses), true));
        egui::Grid::new("classic-finances-rows").num_columns(2).spacing(vec2(16.0, 4.0)).striped(true).show(ui, |ui| {
            for (name, value, strong) in rows {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(name).font(if strong { theme::bold(12.0) } else { theme::regular(12.0) }));
                });
                ui.label(egui::RichText::new(value.map(|v| format::money(t, v)).unwrap_or_default()).font(if strong { theme::bold(12.0) } else { theme::regular(12.0) }));
                ui.end_row();
            }
        });
        ui.add_space(10.0);
        let key = if world.game.monthly_salary { "club-finances-wages-month" } else { "club-finances-wages-week" };
        ui.label(egui::RichText::new(t.text(key, Some(&args([("amount", format::money(t, world.club_wages(me)))])))).font(theme::regular(12.0)));
        ui.label(egui::RichText::new(t.text("club-finances-cash", Some(&args([("amount", format::money(t, world.club(me).cash))])))).font(theme::bold(12.0)));
        ui.add_space(10.0);
        modkit::item(None, None).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let f = &world.club(me).finances;
            let at_limit = f.bank_loan >= quick_loan::emergency_limit(world, me);
            let title = if at_limit { t.text("club-finances-loan-limit", None) } else { t.text("club-finances-loan", None) };
            ui.label(egui::RichText::new(title).font(theme::bold(12.0)));
            ui.label(egui::RichText::new(t.text("club-finances-loaned", Some(&args([("amount", format::money(t, f.bank_loan as i64))])))).font(theme::regular(12.0)));
            ui.label(egui::RichText::new(t.text("club-finances-interest", Some(&args([("amount", format::money(t, f.bank_interest as i64))])))).font(theme::regular(12.0)));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if crate::screens::menu::action_button(ui, &t.text("finances-bank-quick-take", None), true, false).clicked() {
                    quick = Some(true);
                }
                if crate::screens::menu::action_button(ui, &t.text("finances-bank-quick-pay", None), true, false).clicked() {
                    quick = Some(false);
                }
            });
        });
    });
    match quick {
        Some(true) => {
            quick_loan::borrow_emergency(&mut s.career.world, me);
        }
        Some(false) => {
            quick_loan::repay_emergency(&mut s.career.world, me);
        }
        None => {}
    }
}
