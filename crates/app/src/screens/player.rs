//! A player's history (`a.dk`, "Histórico do jogador"): who he is, and the two tables of the
//! original, his career by season and club (`F.gr()`, `b.k`, `b.ae`) with the career total,
//! and this season's matches with his rating (`F.gv()`, `b.T`, `b.aN`).

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::PlayerId;
use brasfoot_model::player::attribute;
use brasfoot_model::world::World;
use eframe::egui::{self, Stroke};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::widgets;
use crate::{format, motion, theme};

/// `b.aN`: "sn" under 2 (no rating), "10" at 10, else one decimal.
fn rating_text(t: &Translator, r: f64) -> String {
    if r < 2.0 {
        t.text("player-history-no-rating", None)
    } else if r >= 10.0 {
        "10".to_owned()
    } else {
        format::rating(t, r)
    }
}

pub fn window(s: &CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles, p: PlayerId, open: &mut bool) {
    let world = &s.career.world;
    let x = world.player(p);
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(12).inner_margin(16);
    motion::dialog(ctx, "player-history", open, egui::vec2(980.0, 640.0), frame, |ui, open| {
        let title = t.text("player-history-title", Some(&args([("name", x.name.clone()), ("position", widgets::position_short(t, x.position))])));
        ui.horizontal(|ui| {
            widgets::flag(ui, files, x.country, 22.0);
            ui.add_space(4.0);
            if widgets::dialog_title(ui, files, None, &title) {
                *open = false;
            }
        });
        // who he is
        egui::Frame::new().fill(theme::CARD).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(10).inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(360.0);
                    ui.label(egui::RichText::new(t.text("player-line", Some(&args([("position", widgets::position_name(t, x.position)), ("side", widgets::side_short(t, x.side)), ("age", x.age.to_string())])))).font(theme::bold(14.0)));
                    ui.horizontal(|ui| match x.club {
                        Some(c) => {
                            widgets::crest(ui, files, world, c, 18.0);
                            ui.label(egui::RichText::new(&world.club(c).name).color(theme::MUTED));
                        }
                        None => {
                            ui.label(egui::RichText::new(t.text("player-no-club", None)).color(theme::MUTED));
                        }
                    });
                    ui.add_space(6.0);
                    egui::Grid::new("player-info").num_columns(4).spacing([14.0, 4.0]).show(ui, |ui| {
                        let cell = |ui: &mut egui::Ui, key: &str, value: String| {
                            ui.label(egui::RichText::new(t.text(key, None)).color(theme::MUTED));
                            ui.label(egui::RichText::new(value).font(theme::bold(12.0)));
                        };
                        cell(ui, "player-strength", x.strength.to_string());
                        cell(ui, "player-rating-mod", brasfoot_engine::tactics::best_value(world, p).to_string());
                        ui.end_row();
                        cell(ui, "player-value", format::money(t, x.market_value as i64));
                        cell(ui, "player-salary", format::money(t, x.salary as i64));
                        ui.end_row();
                        cell(ui, "player-contract", format::date(t, &s.career.runtime.clock, x.contract_until));
                        cell(ui, "player-traits", format!("{} / {}", widgets::trait_short(t, x.trait1), widgets::trait_short(t, x.trait2)));
                        ui.end_row();
                    });
                });
                if world.game.individual_skills {
                    ui.add_space(16.0);
                    egui::Grid::new("player-attributes").num_columns(4).spacing([10.0, 4.0]).show(ui, |ui| {
                        for (i, (key, a)) in [
                            ("squad-column-goalkeeping", attribute::GOALKEEPING),
                            ("squad-column-defending", attribute::DEFENDING),
                            ("squad-column-playmaking", attribute::PLAYMAKING),
                            ("squad-column-finishing", attribute::FINISHING),
                            ("squad-column-speed", attribute::SPEED),
                            ("squad-column-technique", attribute::TECHNIQUE),
                            ("squad-column-passing", attribute::PASSING),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            ui.label(egui::RichText::new(t.text(key, None)).color(theme::MUTED));
                            widgets::bar(ui, x.attributes[a], 120.0);
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                }
            });
        });
        ui.add_space(12.0);
        let height = ui.available_height();
        ui.columns(2, |cols| {
            career(&mut cols[0], t, files, world, p, height);
            season_matches(&mut cols[1], t, files, s, p, height);
        });
    });
}

fn heading(ui: &mut egui::Ui, key: &str, t: &Translator) {
    ui.label(egui::RichText::new(t.text(key, None)).font(theme::bold(14.0)));
    ui.add_space(4.0);
}

fn header_row(ui: &mut egui::Ui, cells: &[(String, f32)]) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 6.0, theme::HEADER);
    let mut x = rect.left();
    for (i, (text, w)) in cells.iter().enumerate() {
        let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(width * w, rect.height()));
        x += width * w;
        let (pos, align) = if i == 1 { (egui::pos2(cell.left() + 6.0, cell.center().y), egui::Align2::LEFT_CENTER) } else { (cell.center(), egui::Align2::CENTER_CENTER) };
        ui.painter().text(pos, align, text, theme::bold(11.0), theme::MUTED);
    }
}

/// `dk.Fl` (`b.k`, `b.ae`): year, club with its crest, J, G, CA, CV, Ct., A, NM; newest first
/// and the career total last.
fn career(ui: &mut egui::Ui, t: &Translator, files: &GameFiles, world: &World, p: PlayerId, height: f32) {
    heading(ui, "player-history-career-heading", t);
    let shares = [0.10, 0.36, 0.07, 0.07, 0.07, 0.07, 0.07, 0.07, 0.12];
    let keys = [
        "player-history-column-year",
        "player-history-column-team",
        "player-history-column-games",
        "player-history-column-goals",
        "player-history-column-yellow",
        "player-history-column-red",
        "player-history-column-injuries",
        "player-history-column-assists",
        "player-history-column-average-rating",
    ];
    header_row(ui, &keys.iter().map(|k| t.text(k, None)).zip(shares).collect::<Vec<_>>());
    let x = world.player(p);
    let base_year = world.game.start_year - 1;
    let width = ui.available_width();
    let average = |sum: f64, rated: i32| if rated > 0 { format::rating_two(t, sum / rated as f64) } else { "--".to_owned() };
    let mut totals = (0, 0, 0, 0, 0, 0, 0.0, 0);
    egui::ScrollArea::vertical().id_salt(("player-career", p)).max_height(height - 40.0).auto_shrink([false, true]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (i, k) in x.season_stats.iter().rev().enumerate() {
            totals = (totals.0 + k.games, totals.1 + k.goals, totals.2 + k.yellow_cards, totals.3 + k.red_cards, totals.4 + k.injuries, totals.5 + k.assists, totals.6 + k.rating_sum, totals.7 + k.rated_games);
            let club = world.club_by_game_id(k.club_id);
            let cells = [
                (k.season + base_year).to_string(),
                club.map(|c| world.club(c).name.clone()).unwrap_or_default(),
                k.games.to_string(),
                k.goals.to_string(),
                k.yellow_cards.to_string(),
                k.red_cards.to_string(),
                k.injuries.to_string(),
                k.assists.to_string(),
                average(k.rating_sum, k.rated_games),
            ];
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
            if i % 2 == 1 {
                ui.painter().rect_filled(rect, 4.0, theme::ROW_ALT);
            }
            table_row(ui, rect, &cells, &shares, club, files, world, false);
        }
        // H() == -1: "Total carreira:"
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, theme::CARD);
        let cells = [
            String::new(),
            t.text("player-history-career-total", None),
            totals.0.to_string(),
            totals.1.to_string(),
            totals.2.to_string(),
            totals.3.to_string(),
            totals.4.to_string(),
            totals.5.to_string(),
            average(totals.6, totals.7),
        ];
        table_row(ui, rect, &cells, &shares, None, files, world, true);
    });
}

#[allow(clippy::too_many_arguments)]
fn table_row(ui: &mut egui::Ui, rect: egui::Rect, cells: &[String], shares: &[f32], club: Option<brasfoot_model::ids::ClubId>, files: &GameFiles, world: &World, bold: bool) {
    let mut x = rect.left();
    for (i, (text, share)) in cells.iter().zip(shares).enumerate() {
        let w = rect.width() * share;
        let cell = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(w, rect.height()));
        x += w;
        let font = if bold || i == 0 { theme::bold(12.0) } else { theme::regular(12.0) };
        if i == 1 {
            let mut left = cell.left() + 6.0;
            if let Some(c) = club {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(egui::pos2(left + 8.0, cell.center().y), egui::vec2(16.0, 16.0))));
                widgets::crest(&mut child, files, world, c, 16.0);
                left += 22.0;
            }
            ui.painter().with_clip_rect(cell).text(egui::pos2(left, cell.center().y), egui::Align2::LEFT_CENTER, text, font, theme::TEXT);
        } else {
            ui.painter().text(cell.center(), egui::Align2::CENTER_CENTER, text, font, theme::TEXT);
        }
    }
}

/// `dk.vN` (`b.T`, `b.aN`): this season's matches, the date, the clubs, the score and his
/// rating (red under 5, green over 7).
fn season_matches(ui: &mut egui::Ui, t: &Translator, _files: &GameFiles, s: &CareerScreen, p: PlayerId, height: f32) {
    let world = &s.career.world;
    heading(ui, "player-history-season-heading", t);
    let shares = [0.20, 0.30, 0.12, 0.26, 0.12];
    let header = [String::new(), String::new(), String::new(), String::new(), t.text("player-history-column-rating", None)];
    header_row(ui, &header.into_iter().zip(shares).collect::<Vec<_>>());
    let width = ui.available_width();
    let x = world.player(p);
    egui::ScrollArea::vertical().id_salt(("player-matches", p)).max_height(height - 40.0).auto_shrink([false, true]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        if x.ratings.is_empty() {
            ui.label(egui::RichText::new(t.text("player-history-no-matches", None)).color(theme::MUTED));
        }
        for (i, r) in x.ratings.iter().enumerate() {
            // best.J: the day of the calendar and the match of that day
            let day = usize::try_from(r.day).ok().and_then(|d| world.game.days.get(d));
            let m = day.and_then(|d| usize::try_from(r.match_index).ok().and_then(|i| d.matches.get(i))).copied();
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
            if i % 2 == 1 {
                ui.painter().rect_filled(rect, 4.0, theme::ROW_ALT);
            }
            let mut x0 = rect.left();
            let mut next = |share: f32| {
                let cell = egui::Rect::from_min_size(egui::pos2(x0, rect.top()), egui::vec2(rect.width() * share, rect.height()));
                x0 += rect.width() * share;
                cell
            };
            let date = next(shares[0]);
            if let Some(d) = day {
                ui.painter().text(egui::pos2(date.left() + 6.0, date.center().y), egui::Align2::LEFT_CENTER, format::date(t, &s.career.runtime.clock, d.date), theme::regular(12.0), theme::MUTED);
            }
            let (home, score, away) = (next(shares[1]), next(shares[2]), next(shares[3]));
            if let Some(m) = m {
                let g = world.game_match(m);
                if let Some(h) = g.home {
                    ui.painter().with_clip_rect(home).text(egui::pos2(home.right() - 4.0, home.center().y), egui::Align2::RIGHT_CENTER, &world.club(h).name, theme::regular(12.0), theme::TEXT);
                }
                ui.painter().text(score.center(), egui::Align2::CENTER_CENTER, widgets::score(world, m), theme::bold(12.0), theme::TEXT);
                if let Some(a) = g.away {
                    ui.painter().with_clip_rect(away).text(egui::pos2(away.left() + 4.0, away.center().y), egui::Align2::LEFT_CENTER, &world.club(a).name, theme::regular(12.0), theme::TEXT);
                }
            }
            let cell = next(shares[4]);
            ui.painter().text(cell.center(), egui::Align2::CENTER_CENTER, rating_text(t, r.rating), theme::bold(12.0), widgets::rating_color(r.rating));
        }
    });
}
