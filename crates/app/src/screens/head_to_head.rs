//! The head-to-head (`a.aL`, "Histórico de confrontos"): the record of two clubs (`best.T`),
//! games, wins, draws and goals of each, each side's biggest win, and their matches this
//! season. Club B is one of club A's opponents with a record (`T.y(ah)`), by name.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::countries;
use brasfoot_model::ids::{ClubId, MatchId};
use brasfoot_model::saved::HeadToHead;
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::new_game::collate;
use crate::screens::{menu, widgets};
use crate::{format, motion, theme};

pub struct HeadToHeadState {
    club_a: ClubId,
    club_b: Option<ClubId>,
}

impl HeadToHeadState {
    /// `new aL(dialog, a, b, false)`.
    pub fn new(club_a: ClubId, club_b: Option<ClubId>) -> Self {
        HeadToHeadState { club_a, club_b }
    }
}

/// `T.a(ah, ah)`: the record of the pair, either way round.
fn record(world: &World, a: ClubId, b: ClubId) -> Option<&HeadToHead> {
    let (ia, ib) = (world.club(a).id, world.club(b).id);
    world.game.head_to_head.iter().find(|r| (r.club1 == ia && r.club2 == ib) || (r.club1 == ib && r.club2 == ia))
}

/// `aL.R(boolean)`: club A's opponents with a record, by name (`h.a.VS`).
fn opponents(world: &World, a: ClubId) -> Vec<ClubId> {
    let ia = world.club(a).id;
    let mut list: Vec<ClubId> = world
        .game
        .head_to_head
        .iter()
        .filter_map(|r| if r.club1 == ia { world.club_by_game_id(r.club2) } else if r.club2 == ia { world.club_by_game_id(r.club1) } else { None })
        .collect();
    list.sort_by_key(|&c| collate(&world.club(c).name));
    list
}

/// `T.bh(int)`: the kind of the competition of a biggest win, named as the game names it.
fn kind_name(t: &Translator, world: &World, club: ClubId, kind: i32) -> String {
    let c = world.club(club);
    match kind {
        1 => t.text("head-to-head-kind-national", None),
        // "Camp. " + aq.rZ[estado]
        3 => t.text("head-to-head-kind-state", Some(&args([("state", t.text(&format!("state-adjective-{}", c.state), None))]))),
        // aq.z(kind, confederation): the international ones by the club's confederation
        4 | 6 => t.text(&format!("club-ranking-kind-{kind}-{}", countries::confederation(c.country).max(0)), None),
        k if (0..16).contains(&k) => t.text(&format!("head-to-head-kind-{k}"), None),
        _ => String::new(),
    }
}

/// `T.iP()` / `iQ()`: "<year> - <competition>: AxB" and "(<kind>)" of a side's biggest win.
fn biggest_win(t: &Translator, world: &World, r: &HeadToHead, first: bool, club: ClubId) -> Option<(String, String)> {
    let (kind, season, scored, conceded) =
        if first { (r.best_win1_kind, r.best_win1_season, r.best_win1_for, r.best_win1_against) } else { (r.best_win2_kind, r.best_win2_season, r.best_win2_for, r.best_win2_against) };
    if kind < 0 {
        return None;
    }
    let year = world.game.start_year - 1 + season;
    let line = t.text("head-to-head-biggest-win-line", Some(&args([("year", year.to_string()), ("competition", kind_name(t, world, club, kind)), ("home", scored.to_string()), ("away", conceded.to_string())])));
    let kind_text = t.text("head-to-head-kind-paren", Some(&args([("kind", t.text(&format!("head-to-head-kind-{kind}"), None))])));
    Some((line, kind_text))
}

/// `aL.yj`: the pair's matches in this season's calendar.
fn season_matches(world: &World, a: ClubId, b: ClubId) -> Vec<MatchId> {
    world
        .game
        .days
        .iter()
        .flat_map(|d| d.matches.iter().copied())
        .filter(|&m| {
            let g = world.game_match(m);
            (g.home == Some(a) && g.away == Some(b)) || (g.home == Some(b) && g.away == Some(a))
        })
        .collect()
}

pub fn window(s: &mut CareerScreen, state: &mut HeadToHeadState, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let world = &s.career.world;
    let clock = &s.career.runtime.clock;
    motion::dialog(ctx, "head-to-head", open, egui::vec2(760.0, 620.0), frame, |ui, open| {
        if widgets::dialog_title(ui, files, Some(glyphs::HEAD_TO_HEAD), &t.text("head-to-head-title", None)) {
            *open = false;
        }
        // yr, ys: the two clubs
        ui.horizontal(|ui| {
            let mut a = state.club_a;
            egui::ComboBox::from_id_salt("h2h-a").width(260.0).height(400.0).selected_text(world.club(a).name.clone()).show_ui(ui, |ui| {
                let mut clubs = world.game.clubs.clone();
                clubs.sort_by_key(|&c| collate(&world.club(c).name));
                for c in clubs {
                    ui.selectable_value(&mut a, c, &world.club(c).name);
                }
            });
            if a != state.club_a {
                state.club_a = a;
                state.club_b = opponents(world, a).first().copied();
            }
            ui.label(egui::RichText::new(t.text("head-to-head-versus", None)).font(theme::bold(14.0)).color(theme::MUTED));
            let list = opponents(world, state.club_a);
            let shown = state.club_b.map(|b| world.club(b).name.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt("h2h-b").width(260.0).height(400.0).selected_text(shown).show_ui(ui, |ui| {
                for c in list {
                    ui.selectable_value(&mut state.club_b, Some(c), &world.club(c).name);
                }
            });
        });
        ui.add_space(14.0);
        let Some(b) = state.club_b.filter(|&b| b != state.club_a) else {
            ui.label(egui::RichText::new(t.text("head-to-head-no-games", None)).color(theme::MUTED));
            return;
        };
        let a = state.club_a;
        let r = record(world, a, b);
        let first_is_a = r.is_some_and(|r| r.club1 == world.club(a).id);
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.columns(3, |cols| {
                for (col, club, first) in [(0usize, a, first_is_a), (2, b, !first_is_a)] {
                    let ui = &mut cols[col];
                    ui.vertical_centered(|ui| {
                        widgets::crest(ui, files, world, club, 56.0);
                        ui.label(egui::RichText::new(&world.club(club).name).font(theme::bold(15.0)));
                        if let Some(r) = r {
                            let (wins, goals) = if first { (r.wins1, r.goals1) } else { (r.wins2, r.goals2) };
                            ui.label(egui::RichText::new(t.text("head-to-head-wins", Some(&args([("count", wins)])))).font(theme::bold(14.0)).color(theme::GOOD));
                            ui.label(egui::RichText::new(t.text("head-to-head-goals", Some(&args([("count", goals)])))).font(theme::regular(13.0)).color(theme::MUTED));
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(t.text("head-to-head-biggest-win", None)).font(theme::bold(12.0)).color(theme::MUTED));
                            match biggest_win(t, world, r, first, club) {
                                Some((line, kind)) => {
                                    ui.label(egui::RichText::new(line).font(theme::regular(12.0)));
                                    ui.label(egui::RichText::new(kind).font(theme::regular(11.0)).color(theme::MUTED));
                                }
                                None => {
                                    ui.label(egui::RichText::new("-").color(theme::MUTED));
                                }
                            }
                        }
                    });
                }
                cols[1].vertical_centered(|ui| {
                    ui.add_space(14.0);
                    match r {
                        Some(r) => {
                            ui.label(egui::RichText::new(t.text("head-to-head-games", Some(&args([("count", r.games)])))).font(theme::bold(20.0)));
                            let draws = r.games - (r.wins1 + r.wins2);
                            ui.label(egui::RichText::new(t.text("head-to-head-draws", Some(&args([("count", draws)])))).font(theme::regular(13.0)).color(theme::MUTED));
                        }
                        None => {
                            ui.label(egui::RichText::new(t.text("head-to-head-no-games", None)).font(theme::bold(18.0)));
                        }
                    }
                });
            });
        });
        ui.add_space(12.0);
        ui.label(egui::RichText::new(t.text("head-to-head-season-games", None)).font(theme::bold(14.0)));
        ui.add_space(6.0);
        let matches = season_matches(world, a, b);
        if matches.is_empty() {
            ui.label(egui::RichText::new(t.text("head-to-head-none-this-season", None)).color(theme::MUTED));
        }
        // b.an: date, home, score or "x", away, competition
        for (i, m) in matches.into_iter().enumerate() {
            let g = world.game_match(m);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::hover());
            if i % 2 == 1 {
                ui.painter().rect_filled(rect, 4.0, theme::ROW_ALT);
            }
            let p = ui.painter();
            let date = world.game.days.iter().find(|d| d.matches.contains(&m)).map(|d| format::date(t, clock, d.date)).unwrap_or_default();
            p.text(egui::pos2(rect.left() + 8.0, rect.center().y), egui::Align2::LEFT_CENTER, date, theme::regular(12.0), theme::MUTED);
            let score = rect.center() + egui::vec2(-40.0, 0.0);
            p.text(score, egui::Align2::CENTER_CENTER, widgets::score(world, m), theme::bold(13.0), theme::TEXT);
            if let Some(h) = g.home {
                let room = score.x - 36.0 - rect.left() - 110.0;
                p.text(score - egui::vec2(36.0, 0.0), egui::Align2::RIGHT_CENTER, widgets::fit_text(p, &world.club(h).name, theme::regular(13.0), room), theme::regular(13.0), theme::TEXT);
            }
            if let Some(v) = g.away {
                let room = 200.0;
                p.text(score + egui::vec2(36.0, 0.0), egui::Align2::LEFT_CENTER, widgets::fit_text(p, &world.club(v).name, theme::regular(13.0), room), theme::regular(13.0), theme::TEXT);
            }
            if let Some(c) = g.competition {
                p.text(egui::pos2(rect.right() - 8.0, rect.center().y), egui::Align2::RIGHT_CENTER, widgets::competition_name(t, world, c), theme::regular(12.0), theme::MUTED);
            }
        }
    });
}
