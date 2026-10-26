//! The club's history (`a.hw`, "Retrospecto"): each season and competition of `ah.le()`
//! (`best.aj`), newest first, with the placing (`aj.d(Z)`), the record and a total row per year.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::ClubId;
use brasfoot_model::saved::{CompetitionKind, CompetitionRecord};
use brasfoot_model::world::World;
use eframe::egui::{self, Color32, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::new_game::country_name;
use crate::screens::widgets;
use crate::{motion, theme};

/// The marks `aj.d(Z)` leaves on the record (`nP`, `nQ`, `nR`) and `lw()`.
#[derive(Default, Clone, Copy)]
struct Marks {
    champion: bool,
    runner_up: bool,
    eliminated: bool,
}

/// `aj.d(Z)`: the placing's text. `cup_rounds` is the country's cup's list (`yh()`).
fn placement(t: &Translator, world: &World, r: &CompetitionRecord, cup_rounds: &str) -> (String, Marks) {
    let mut marks = Marks::default();
    let list = match r.competition_kind {
        2 => cup_rounds,
        4 => match r.competition_subkind {
            0..=3 => "pa",
            4 => "pb",
            5 => "pc",
            _ => "py",
        },
        6 | 12 => {
            if r.competition_subkind == 0 {
                "pz"
            } else {
                "pa"
            }
        }
        _ => "py",
    };
    let count = (0..)
        .take_while(|i| {
            let key = format!("round-list-{list}-{i}");
            t.text(&key, None) != key
        })
        .count();
    // Collections.reverse: index 1 is the round before the final
    let round = |i: usize| t.text(&format!("round-list-{list}-{}", count - 1 - i), None);
    if r.competition_kind == 9 {
        let key = if r.division == 1 { "placement-qualified" } else { "placement-not-qualified" };
        return (t.text(key, None), marks);
    }
    if r.round_reached >= 0 {
        marks.eliminated = true;
        let code = r.round_reached;
        let fixed = match code {
            1090 => {
                marks.eliminated = false;
                Some(t.text("placement-won-playoff", None))
            }
            1091 => Some(t.text("placement-lost-playoff", None)),
            1001 => Some(t.text("placement-group-stage", None)),
            4101 | 4102 | 4103 | 6000 | 6001 | 6002 | 6003 | 6006 | 6004 | 6100 | 4000 | 4001 | 4002 | 4003 | 4004 | 12001 | 12002 | 12003 | 12004 | 12006 => {
                Some(t.text(&format!("placement-code-{code}"), None))
            }
            _ => None,
        };
        if let Some(text) = fixed {
            return (text, marks);
        }
        if code == 0 {
            match r.position {
                1 => {
                    marks = Marks { champion: true, ..Default::default() };
                    return (t.text("placement-champion", None), marks);
                }
                2 => {
                    marks = Marks { runner_up: true, ..Default::default() };
                    return (t.text("placement-runner-up", None), marks);
                }
                3 | 4 if r.competition_kind == 7 || r.competition_kind == 14 => {
                    marks.eliminated = false;
                    let key = if r.position == 3 { "placement-third" } else { "placement-fourth" };
                    return (t.text(key, None), marks);
                }
                _ => {}
            }
        } else if code >= 1 && (code as usize) < count {
            return (round(code as usize), marks);
        }
        return (String::new(), marks);
    }
    if r.position > 0 {
        return match r.position {
            1 => (t.text("placement-champion", None), Marks { champion: true, ..Default::default() }),
            2 => (t.text("placement-runner-up", None), Marks { runner_up: true, ..Default::default() }),
            _ if r.competition_kind == 7 => (t.text("placement-groups-short", None), Marks { eliminated: true, ..Default::default() }),
            place => (t.text("placement-position", Some(&args([("place", place)]))), marks),
        };
    }
    if r.season < world.season() {
        match r.competition_kind {
            1 => return (t.text("placement-first-phase", None), Marks { eliminated: true, ..Default::default() }),
            14 => {
                const LETTERS: [&str; 5] = ["A", "A", "B", "C", "D"];
                return match usize::try_from(r.division).ok().and_then(|d| LETTERS.get(d)) {
                    Some(l) => (t.text("placement-league-letter", Some(&args([("letter", *l)]))), marks),
                    None => (t.text("placement-groups", None), marks),
                };
            }
            7 => return (t.text("placement-groups-short", None), Marks { eliminated: true, ..Default::default() }),
            _ => {}
        }
    }
    (String::new(), marks)
}

/// `hw.h(int, int, int)`: the competition's name in the row.
fn competition(t: &Translator, world: &World, club: ClubId, r: &CompetitionRecord) -> String {
    let country = world.club(club).country;
    let (kind, id, division) = (r.competition_kind, r.competition_subkind, r.division);
    match kind {
        9 if id == 70 => t.text("club-history-qualifier-euro", None),
        1 if id == 1098 => t.text("relegation-playoff-name", None),
        // ad.bu(): "Campeonato <adjective>", or the country's name
        1 => {
            let key = format!("country-adjective-{country}");
            let adjective = t.text(&key, None);
            if adjective != key {
                t.text("club-history-league", Some(&args([("adjective", adjective), ("division", division.to_string())])))
            } else {
                t.text("club-history-league-country", Some(&args([("country", country_name(t, country)), ("division", division.to_string())])))
            }
        }
        // "Copa " + ad.bq() + " " + ac.bl()
        2 => {
            let article = brasfoot_i18n::labels::cup_article(t, country);
            t.text("club-history-cup", Some(&args([("article", article), ("country", country_name(t, country))])))
        }
        3 if (0..27).contains(&id) => t.text("club-history-state", Some(&args([("state", t.text(&format!("state-adjective-{id}"), None)), ("division", division.to_string())]))),
        3 => t.text("club-history-kind-division", Some(&args([("kind", t.text("head-to-head-kind-3", None)), ("division", division.to_string())]))),
        // SR.c(kind, id): the game's competition of that kind
        4 | 6 | 12 => world
            .competitions
            .iter()
            .enumerate()
            .find(|(_, c)| c.kind == kind && c.subkind == id)
            .map(|(i, _)| widgets::competition_name(t, world, brasfoot_model::ids::CompetitionId(i as u32)))
            .unwrap_or_else(|| t.text(&format!("head-to-head-kind-{kind}"), None)),
        10 => t.text(&format!("regional-name-{id}"), None),
        7 => t.text(&format!("national-teams-cup-{id}"), None),
        k if (0..16).contains(&k) => t.text(&format!("head-to-head-kind-{k}"), None),
        _ => String::new(),
    }
}

/// `f.f.yh()`: the cup's rounds by its number of clubs.
fn cup_rounds(world: &World, club: ClubId) -> &'static str {
    let code = world.club(club).country;
    let cup = world
        .game
        .league_countries
        .iter()
        .find(|&&k| world.country(k).country == code)
        .and_then(|&k| world.country(k).cup)
        .filter(|&c| world.competition(c).class == CompetitionKind::NationalCup);
    match cup.map(|c| world.competition(c).team_count) {
        Some(128) => "px",
        Some(64) => "py",
        Some(32) => "pz",
        Some(16) => "pa",
        Some(8) => "pb",
        Some(4) => "pc",
        Some(2) => "pd",
        _ => "ps",
    }
}

pub fn window(s: &mut CareerScreen, club: ClubId, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    let world = &s.career.world;
    motion::dialog(ctx, "club-history", open, egui::vec2(860.0, 680.0), frame, |ui, open| {
        let title = t.text("club-history-title", Some(&args([("club", world.club(club).name.clone())])));
        if widgets::dialog_title(ui, files, Some(glyphs::CLUB_RECORD), &title) {
            *open = false;
        }
        let records = &world.club(club).competition_records;
        if records.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| ui.label(egui::RichText::new(t.text("club-history-empty", None)).color(theme::MUTED)));
            return;
        }
        let width = ui.available_width();
        let fixed = [60.0, 0.0, 200.0, 40.0, 40.0, 40.0, 40.0, 44.0, 44.0];
        let flexible = width - fixed.iter().sum::<f32>();
        let widths: Vec<f32> = fixed.iter().map(|w| if *w == 0.0 { flexible } else { *w }).collect();
        let headers = [
            "club-history-column-year",
            "club-history-column-competition",
            "club-history-column-placement",
            "club-history-column-played",
            "club-history-column-won",
            "club-history-column-drawn",
            "club-history-column-lost",
            "club-history-column-goals-for",
            "club-history-column-goals-against",
        ];
        let (header, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
        ui.painter().rect_filled(header, 6.0, theme::HEADER);
        let mut x = header.left();
        for (i, (key, w)) in headers.iter().zip(&widths).enumerate() {
            let cell = egui::Rect::from_min_size(egui::pos2(x, header.top()), egui::vec2(*w, header.height()));
            x += w;
            let (pos, align) = if i == 1 || i == 2 { (egui::pos2(cell.left() + 8.0, cell.center().y), egui::Align2::LEFT_CENTER) } else { (cell.center(), egui::Align2::CENTER_CENTER) };
            ui.painter().text(pos, align, t.text(key, None), theme::bold(11.0), theme::MUTED);
        }
        ui.add_space(2.0);
        let base_year = world.game.start_year - 1;
        let rounds = cup_rounds(world, club);
        egui::ScrollArea::vertical().id_salt("club-history-rows").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let mut totals = [0i32; 5];
            // hw: newest first, a total row after the last record of each year
            for i in (0..records.len()).rev() {
                let r = &records[i];
                let (text, marks) = placement(t, world, r, rounds);
                let draws = r.games - r.wins - r.losses;
                for (slot, v) in totals.iter_mut().zip([r.games, r.wins, r.losses, r.goals_for, r.goals_against]) {
                    *slot += v;
                }
                let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 4.0, theme::CARD);
                let cells = [
                    (r.season + base_year).to_string(),
                    competition(t, world, club, r),
                    text,
                    r.games.to_string(),
                    r.wins.to_string(),
                    draws.to_string(),
                    r.losses.to_string(),
                    r.goals_for.to_string(),
                    r.goals_against.to_string(),
                ];
                row(ui, rect, &widths, &cells, Some((marks, r.league_move)), false);
                let year_ends = i == 0 || records[i - 1].season != r.season;
                if year_ends {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
                    let label = t.text("club-history-total-year", Some(&args([("year", (r.season + base_year).to_string())])));
                    let [games, wins, losses, gf, ga] = totals;
                    let cells = [String::new(), String::new(), label, games.to_string(), wins.to_string(), (games - wins - losses).to_string(), losses.to_string(), gf.to_string(), ga.to_string()];
                    row(ui, rect, &widths, &cells, None, true);
                    totals = [0; 5];
                    ui.add_space(6.0);
                }
            }
        });
    });
}

/// A row: the cells, the placing's mark (`r_ch`, `r_vice`, `r_sobe`, `r_desce`, `r_eli`).
fn row(ui: &egui::Ui, rect: egui::Rect, widths: &[f32], cells: &[String], marks: Option<(Marks, i32)>, total: bool) {
    let mut x = rect.left();
    for (i, (text, w)) in cells.iter().zip(widths).enumerate() {
        let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*w, rect.height()));
        x += w;
        let p = ui.painter().with_clip_rect(cell.shrink2(egui::vec2(2.0, 0.0)));
        let font = if total || i == 0 { theme::bold(12.0) } else { theme::regular(12.0) };
        let color = if total { theme::MUTED } else { theme::TEXT };
        if i == 2 {
            let mut left = cell.left() + 8.0;
            if let Some((m, promoted)) = marks {
                let mark = if m.champion {
                    Some((glyphs::TROPHY, theme::STAR))
                } else if m.runner_up {
                    Some((glyphs::RUNNER_UP, Color32::from_rgb(0xc0, 0xc8, 0xd4)))
                } else if promoted == 1 {
                    Some((glyphs::PROMOTED, theme::GOOD))
                } else if promoted == 0 {
                    Some((glyphs::RELEGATED, theme::BAD))
                } else if m.eliminated {
                    Some((glyphs::ELIMINATED, theme::TEXT_RED))
                } else {
                    None
                };
                if let Some((glyph, tint)) = mark {
                    widgets::paint_icon(&p, egui::Rect::from_center_size(egui::pos2(left + 7.0, cell.center().y), egui::vec2(15.0, 15.0)), glyph, tint);
                }
                left += 20.0;
            }
            let align = if total { egui::Align2::RIGHT_CENTER } else { egui::Align2::LEFT_CENTER };
            let pos = if total { egui::pos2(cell.right() - 8.0, cell.center().y) } else { egui::pos2(left, cell.center().y) };
            p.text(pos, align, text, font, color);
        } else if i == 1 {
            p.text(egui::pos2(cell.left() + 8.0, cell.center().y), egui::Align2::LEFT_CENTER, text, font, color);
        } else {
            p.text(cell.center(), egui::Align2::CENTER_CENTER, text, font, color);
        }
    }
}
