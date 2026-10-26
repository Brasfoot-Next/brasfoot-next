//! The main screen (`a.eg`): on the left the club card, next match, `Escalar Time` and the
//! selected player's card `components.m`; on the right the squad table (`b.Q`, with the mod's
//! Rating column). The original paints the left side in the club's colors (`ah.kB()`,
//! `ah.kC()`) and lets the player change them (`J-a-bb`); here the whole app keeps one theme.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::PlayerId;
use brasfoot_model::player::attribute;
use eframe::egui::{self, Color32, Stroke};

use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::{contract, player_actions};
use crate::screens::widgets::{self, crest};
use crate::{format, glyphs, motion, theme, view};

const LEFT: f32 = 300.0;

pub fn show(s: &mut CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) {
    let height = ui.available_height();
    egui::Frame::new().fill(theme::BACKGROUND).inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 0 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_height(height);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.vertical(|ui| {
                ui.add_space(6.0);
                egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(8).inner_margin(10).show(ui, |ui| {
                    ui.set_width(LEFT - 36.0);
                    ui.set_max_width(LEFT - 36.0);
                    ui.set_height(height - 38.0);
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    club_panel(s, ui, t, files, theme::TEXT, theme::HEADER);
                });
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.add_space(6.0);
                egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(8).show(ui, |ui| {
                    ui.set_width(ui.available_width() - 2.0);
                    ui.set_height(height - 16.0);
                    ui.vertical(|ui| {
                        egui::ScrollArea::horizontal().id_salt("squad-scroll").show(ui, |ui| squad_table(s, ui, t, files));
                    });
                });
            });
        });
    });
}

/// The squad as the main screen lists it (`h.y`, `h.a.abe`): position, style, strength, stars
/// first. The original sorts the player's squad in place each time it builds this screen
/// (`eg.qJ`); that comes with the human club's day.
fn listed_squad(world: &brasfoot_model::world::World, club: brasfoot_model::ids::ClubId) -> Vec<PlayerId> {
    let mut squad = world.club(club).squad.clone();
    squad.sort_by(|&a, &b| {
        let (x, y) = (world.player(a), world.player(b));
        x.position.cmp(&y.position).then(x.style.cmp(&y.style)).then(y.strength.cmp(&x.strength)).then(y.star.cmp(&x.star))
    });
    squad
}

fn club_panel(s: &mut CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, text: Color32, back: Color32) {
    let mut open_messages = false;
    let (mut open_record, mut open_loans) = (false, false);
    let world = &s.career.world;
    let club = world.club(s.club);
    // `Fq`, `ug`, `a_`: the crest, the club's name and the coach's
    ui.horizontal(|ui| {
        crest(ui, files, world, s.club, 60.0);
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(&club.name).font(theme::bold(16.0)).color(text));
            if let Some(coach) = club.coach {
                ui.label(egui::RichText::new(&world.coach(coach).name).font(theme::regular(12.0)).color(theme::MUTED));
            }
        });
    });
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        widgets::icon_glyph(ui, glyphs::CASH, 18.0, theme::GOOD);
        ui.label(egui::RichText::new(format::money(t, club.cash)).font(theme::regular(11.0)).color(text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // ze: the unread messages; a click opens the message center (eg.rm())
            let unread = club.coach.map(|c| world.coach(c).messages.iter().filter(|m| !m.read).count()).unwrap_or(0);
            let label = ui.add(egui::Label::new(egui::RichText::new(unread.to_string()).font(theme::bold(12.0)).color(if unread > 0 { theme::TEXT } else { theme::MUTED })).sense(egui::Sense::click()));
            let icon = widgets::icon_button(ui, glyphs::MESSAGES, 18.0, if unread > 0 { theme::ACCENT } else { theme::MUTED }, true);
            if (label.clicked() || icon.clicked()) && s.career.human_club().is_some() {
                open_messages = true;
            }
        });
    });
    // `uh`: the next match's competition, with the club's place from the second round on
    if let Some(m) = view::next_match(world, s.club)
        && let Some(c) = world.game_match(m).competition
    {
        let name = widgets::competition_name(t, world, c);
        let line = match view::shown_place(world, world.game_match(m).stage, s.club) {
            Some(place) => t.text("home-league-place", Some(&args([("competition", name), ("place", place.to_string())]))),
            None => name,
        };
        ui.label(egui::RichText::new(line).font(theme::regular(12.0)).color(text));
    }
    ui.add_space(8.0);
    if let Some(coach) = club.coach {
        let c = world.coach(coach);
        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new(t.text("home-board-confidence", None)).font(theme::bold(11.0)).color(text));
            widgets::club_bar(&mut cols[0], c.board_confidence, theme::ACCENT);
            cols[1].label(egui::RichText::new(t.text("home-fan-confidence", None)).font(theme::bold(11.0)).color(text));
            widgets::club_bar(&mut cols[1], c.fan_support, theme::ACCENT);
        });
    }
    if open_messages {
        s.messages_state = Default::default();
        s.messages_open = true;
    }
    ui.add_space(14.0);
    if next_match(s, ui, t, files, text, back) {
        s.open_head_to_head();
    }
    ui.add_space(14.0);
    s.play_button(ui, t, files);
    ui.add_space(6.0);
    let opt_p = s.selected_player.or_else(|| listed_squad(&s.career.world, s.club).first().copied());
    if let Some(p) = opt_p {
        player_card(s, ui, t, files, p, text, back);
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        // `Iv`: trophy gallery, the club's record, loaned players; and the squad count. The
        // first button of the original, the screen's colors, has no place with one theme.
        ui.horizontal(|ui| {
            if widgets::icon_button(ui, glyphs::TROPHY, 24.0, theme::STAR, true).on_hover_text(t.text("trophies-title", None)).clicked() {
                s.trophies_club = Some(s.club);
                s.trophies_open = true;
            }
            ui.add_space(10.0);
            if widgets::icon_button(ui, glyphs::CLUB_RECORD, 24.0, theme::ACCENT, true).on_hover_text(t.text("home-club-record", None)).clicked() {
                open_record = true;
            }
            ui.add_space(10.0);
            if widgets::icon_button(ui, glyphs::LOANS, 24.0, theme::GOOD, true).on_hover_text(t.text("home-loaned-players", None)).clicked() {
                open_loans = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                ui.label(egui::RichText::new(t.text("squad-count", Some(&args([("count", s.career.world.club(s.club).squad.len())])))).font(theme::regular(11.0)).color(text));
                widgets::icon_glyph(ui, glyphs::USERS, 14.0, theme::MUTED);
            });
        });
    });
    // HY: qP, the club's history (J-a-hw); Ic: qO, the loans (J-a-bs)
    if open_record {
        s.club_history = Some(s.club);
    }
    if open_loans {
        s.loans = Some(s.club);
    }
}

/// The `Próximo Jogo` strip, the rival and the round (`vz`, `yt`, `vw`, `Bj`, `zb`).
/// True when the head-to-head icon (`zb`) was clicked.
fn next_match(s: &CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, text: Color32, back: Color32) -> bool {
    let mut open_head_to_head = false;
    let world = &s.career.world;
    let Some(m) = view::next_match(world, s.club) else {
        ui.label(egui::RichText::new(t.text("home-no-next-match", None)).color(text));
        return false;
    };
    let g = world.game_match(m);
    let home = g.home == Some(s.club);
    let derby = brasfoot_simulation::matchday_rivalry(world, m);
    let key = match (home, derby) {
        (true, false) => "home-next-home",
        (false, false) => "home-next-away",
        (true, true) => "home-next-home-derby",
        (false, true) => "home-next-away-derby",
    };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 27.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, back);
    ui.painter().text(rect.left_center() + egui::vec2(8.0, 0.0), egui::Align2::LEFT_CENTER, t.text(key, None), theme::bold(11.0), text);
    ui.add_space(8.0);
    let rival = if home { g.away } else { g.home };
    ui.horizontal(|ui| {
        if let Some(r) = rival {
            if files.crest(&world.club(r).file_ref).is_some() {
                crest(ui, files, world, r, 50.0);
            }
            ui.vertical(|ui| {
                let name = match view::shown_place(world, g.stage, r) {
                    Some(p) => t.text("home-rival-place", Some(&args([("club", world.club(r).name.clone()), ("place", p.to_string())]))),
                    None => world.club(r).name.clone(),
                };
                ui.label(egui::RichText::new(name).font(theme::regular(13.0)).color(text));
                ui.label(egui::RichText::new(widgets::match_round_text(t, world, m)).font(theme::regular(12.0)).color(text));
            });
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            // zb: the head-to-head with this opponent
            if widgets::icon_button(ui, glyphs::HEAD_TO_HEAD, 18.0, theme::ACCENT, true).on_hover_text(t.text("history-menu-head-to-head", None)).clicked() {
                open_head_to_head = true;
            }
        });
    });
    open_head_to_head
}

/// `components.m`: the selected player.
fn player_card(s: &mut CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, p: PlayerId, text: Color32, back: Color32) {
    let (name, country, position, side, age, contract_until, injured_until, is_injured, market_value, salary, assists, goals, games, career, average, career_average) = {
        let world = &s.career.world;
        let x = world.player(p);
        let today = world.today_millis();
        let (assists, goals, games) = x.season_totals(world.season());
        let career = x.career_totals();
        let average = x.season_average_rating().map(|r| format::rating(t, r)).unwrap_or_else(|| "--".into());
        let career_average = x.career_average_rating().map(|r| format::rating(t, r)).unwrap_or_else(|| "--".into());
        (
            x.name.clone(),
            x.country,
            x.position,
            x.side,
            x.age,
            x.contract_until,
            x.injured_until,
            x.is_injured(today),
            x.market_value,
            x.salary,
            assists,
            goals,
            games,
            career,
            average,
            career_average,
        )
    };
    egui::Frame::new().fill(theme::CARD).stroke(Stroke::new(1.0, theme::BORDER)).corner_radius(6).inner_margin(8).show(ui, |ui| {
        ui.set_width(ui.available_width());
        // another player's card fades in
        let shown = motion::since_change(ui.ctx(), egui::Id::unique("player-card"), egui::Id::unique(p).value(), motion::CONTENT);
        ui.multiply_opacity(0.25 + 0.75 * shown);
        // the name strip with the flag
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width() * 0.78, 20.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, back);
        let mut x0 = rect.left() + 4.0;
        if let Some(uri) = files.jar_image(ui.ctx(), &format!("aflags/{}.png", country)) {
            // the flags are square: drawn square, not squeezed
            egui::Image::new(uri).paint_at(ui, egui::Rect::from_min_size(egui::pos2(x0, rect.top() + 3.0), egui::vec2(14.0, 14.0)));
            x0 += 19.0;
        }
        ui.painter().text(egui::pos2(x0, rect.center().y), egui::Align2::LEFT_CENTER, &name, theme::bold(12.0), text);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            // Vender (qX, a.jc) or, for a player already for sale, Retirar da Venda (qR)
            let listed = s.career.world.player(p).listed_for_sale;
            let key = if listed { "player-action-not-for-sale" } else { "player-action-for-sale" };
            if widgets::icon_button(ui, glyphs::FOR_SALE, 18.0, if listed { theme::GOOD } else { theme::STAR }, true).on_hover_text(t.text(key, None)).clicked() {
                if listed {
                    s.career.world.player_mut(p).listed_for_sale = false;
                } else {
                    player_actions::apply(s, p, player_actions::PlayerAction::Sell);
                }
            }
            ui.add_space(4.0);
            if widgets::icon_button(ui, glyphs::CONTRACT, 18.0, theme::ACCENT, true).on_hover_text(t.text("contract-title", None)).clicked() {
                s.contract_state = Some(contract::ContractState::new(&s.career.world, p));
                s.contract_open = true;
            }
            ui.add_space(4.0);
            // qW: only a player over 32 may retire
            let can = player_actions::can_retire(&s.career.world, p);
            let tip = if can { t.text("player-action-retire", None) } else { t.text("player-action-retire-age", None) };
            if widgets::icon_button(ui, glyphs::RETIRE, 18.0, theme::TEXT_RED, can).on_hover_text(tip).clicked() {
                player_actions::apply(s, p, player_actions::PlayerAction::Retire);
            }
            ui.add_space(4.0);
            if widgets::icon_button(ui, glyphs::CAREER, 18.0, theme::GOOD, true).on_hover_text(t.text("teams-tab-career", None)).clicked() {
                s.open_player = Some(p);
            }
        });
        ui.add_space(4.0);
        let small = |ui: &mut egui::Ui, s: String| {
            ui.label(egui::RichText::new(s).font(theme::regular(11.0)).color(text));
        };
        small(ui, t.text("player-line-long", Some(&args([("position", widgets::position_name(t, position)), ("side", t.text(&format!("side-{}", side), None)), ("age", age.to_string())]))));
        if is_injured {
            small(ui, t.text("player-injured-until", Some(&args([("date", format::date_medium(t, &s.career.runtime.clock, injured_until))]))));
        } else {
            small(ui, t.text("player-contract-until", Some(&args([("date", format::date_medium(t, &s.career.runtime.clock, contract_until))]))));
        }
        ui.add_space(6.0);
        // value and salary wrap on a narrow card
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(t.text("player-value", None)).font(theme::bold(11.0)).color(text));
            ui.label(egui::RichText::new(format::money(t, market_value as i64)).font(theme::regular(11.0)).color(text));
            ui.label(egui::RichText::new(t.text("player-salary", None)).font(theme::bold(11.0)).color(text));
            ui.label(egui::RichText::new(format::money(t, salary as i64)).font(theme::regular(11.0)).color(text));
        });
        ui.add_space(4.0);
        // the stats grid: right-aligned titles, then season, career and current competition
        // the columns follow the card's width: the titles take 30%, the three columns share the
        // rest, and a long column title goes on two lines
        let width = ui.available_width();
        let column = width * 0.70 / 3.0;
        let centers = [width * 0.30 + column * 0.5, width * 0.30 + column * 1.5, width * 0.30 + column * 2.5];
        let header = [t.text("player-card-season", None), t.text("player-card-career", None), t.text("player-card-competition", None)];
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::hover());
        for (i, h) in header.iter().enumerate() {
            let job = egui::text::LayoutJob::simple(h.clone(), theme::regular(9.0), theme::MUTED, column);
            let mut job = job;
            job.halign = egui::Align::Center;
            let galley = ui.painter().layout_job(job);
            let pos = egui::pos2(rect.left() + centers[i], rect.bottom() - galley.size().y);
            ui.painter().galley(pos, galley, theme::MUTED);
        }
        for (key, a, b) in [
            ("player-card-games", games.to_string(), career[0].to_string()),
            ("player-card-goals", goals.to_string(), career[1].to_string()),
            ("player-card-assists", assists.to_string(), career[5].to_string()),
            ("player-card-average", average.clone(), career_average.clone()),
        ] {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::hover());
            let p = ui.painter();
            p.text(egui::pos2(rect.left() + width * 0.29, rect.center().y), egui::Align2::RIGHT_CENTER, t.text(key, None), theme::bold(10.0), text);
            for (i, v) in [&a, &b, &a].into_iter().enumerate() {
                p.text(egui::pos2(rect.left() + centers[i], rect.center().y), egui::Align2::CENTER_CENTER, v, theme::regular(10.0), text);
            }
        }
    });
}

fn squad_table(s: &mut CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) {
    let skills = s.career.world.game.individual_skills;
    let mut columns: Vec<(&str, f32)> = vec![("", 20.0), ("squad-column-position", 20.0), ("", 20.0), ("squad-column-name", 160.0), ("squad-column-side", 20.0)];
    if skills {
        for key in ["squad-column-goalkeeping", "squad-column-defending", "squad-column-playmaking", "squad-column-finishing", "squad-column-speed", "squad-column-technique", "squad-column-passing"] {
            columns.push((key, 34.0));
        }
    } else {
        columns.push(("squad-column-strength", 26.0));
    }
    let value_key = if skills { "squad-column-value" } else { "squad-column-transfer" };
    columns.extend([
        ("squad-column-energy", 74.0),
        ("squad-column-salary", 64.0),
        (value_key, 64.0),
        ("squad-column-goals", 34.0),
        ("squad-column-traits", 58.0),
        ("squad-column-age", 34.0),
        ("squad-column-career-goals", 34.0),
        ("squad-column-assists", 34.0),
        ("squad-column-average", 34.0),
        ("squad-column-rating", 52.0),
    ]);
    let world = &s.career.world;
    let squad = listed_squad(world, s.club);
    let season = world.season();
    let today = world.today_millis();
    // drawn straight with the painter: one sensed rect per row, so hover and clicks are known
    // before the row is painted, and no child layout per cell
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    let ctx = ui.ctx().clone();
    let fixed: f32 = columns.iter().filter(|(k, _)| *k != "squad-column-name").map(|(_, w)| w).sum();
    let name_width = (ui.available_width() - fixed - 2.0).max(140.0);
    let widths: Vec<f32> = columns.iter().map(|(k, w)| if *k == "squad-column-name" { name_width } else { *w }).collect();
    let total: f32 = widths.iter().sum();
    let small = theme::regular(11.0);
    let (header, _) = ui.allocate_exact_size(egui::vec2(total, 24.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(header, 0.0, theme::PANEL);
    painter.line_segment([header.left_bottom(), header.right_bottom()], Stroke::new(1.0, theme::GRID));
    let mut x = header.left();
    for ((key, _), w) in columns.iter().zip(&widths) {
        let cell = egui::Rect::from_min_size(egui::pos2(x, header.top()), egui::vec2(*w, header.height()));
        painter.line_segment([cell.right_top(), cell.right_bottom()], Stroke::new(1.0, theme::GRID));
        if !key.is_empty() {
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, t.text(key, None), small.clone(), theme::TEXT);
        }
        x += w;
    }
    let mut current = s.selected_player.or_else(|| squad.first().copied());
    let mut clicked = None;
    let mut opened = None;
    let mut action = None;
    let human_squad = s.career.human_club() == Some(s.club);
    egui::ScrollArea::vertical().id_salt("squad-rows").auto_shrink([false, false]).show(ui, |ui| {
        for (i, &p) in squad.iter().enumerate() {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(total, 20.0), egui::Sense::click());
            if response.clicked() {
                clicked = Some(p);
                current = Some(p);
            }
            if response.double_clicked() {
                opened = Some(p);
            }
            // eg.HB: the right button picks the row and shows the player's menu
            if response.secondary_clicked() {
                clicked = Some(p);
                current = Some(p);
            }
            if human_squad {
                response.context_menu(|ui| {
                    if let Some(a) = player_actions::context_menu(ui, t, world, p) {
                        action = Some((p, a));
                    }
                });
            }
            if response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            // the row as a selectable item for screen readers (and the tests)
            response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Row, true, current == Some(p), &world.player(p).name));
            if !ui.is_rect_visible(rect) {
                continue;
            }
            let x = world.player(p);
            let base = if i % 2 == 0 { theme::PANEL } else { theme::ROW_ALT };
            let hover = motion::toward(&ctx, response.id, response.hovered(), motion::HOVER);
            let select = motion::toward(&ctx, response.id.with("selected"), current == Some(p), motion::SELECT);
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, motion::mix(motion::mix(base, theme::HOVER, hover), theme::SELECTION, select));
            // the selected row's accent mark grows from the middle of the left edge
            if select > 0.0 {
                let mark = egui::Rect::from_center_size(egui::pos2(rect.left() + 1.5, rect.center().y), egui::vec2(3.0, rect.height() * select));
                painter.rect_filled(mark, 0.0, theme::ACCENT);
            }
            let (assists, goals, _) = x.season_totals(season);
            let mut cells: Vec<(String, Color32, bool)> = vec![(String::new(), theme::TEXT, false), (widgets::position_short(t, x.position), theme::TEXT, false), (String::new(), theme::TEXT, false)];
            let name_color = if x.listed_for_sale { theme::TEXT_BLUE } else { theme::TEXT };
            cells.push((x.name.clone(), name_color, x.listed_for_sale || x.on_loan));
            cells.push((widgets::side_short(t, x.side), theme::TEXT, false));
            if skills {
                for a in [attribute::GOALKEEPING, attribute::DEFENDING, attribute::PLAYMAKING, attribute::FINISHING, attribute::SPEED, attribute::TECHNIQUE, attribute::PASSING] {
                    cells.push((x.attributes[a].to_string(), theme::TEXT, false));
                }
            } else {
                cells.push((x.strength.to_string(), theme::TEXT, false));
            }
            cells.push((String::new(), theme::TEXT, false));
            cells.push((format::compact_money(t, x.salary as i64), theme::TEXT, false));
            cells.push((format::compact_money(t, x.market_value as i64), if x.listed_for_sale { theme::TEXT_GREEN } else { theme::TEXT }, false));
            cells.push((goals.to_string(), theme::TEXT, false));
            cells.push((format!("{}/{}", widgets::trait_short(t, x.trait1), widgets::trait_short(t, x.trait2)), theme::TEXT, false));
            cells.push((x.age.to_string(), if x.age > 32 { theme::TEXT_RED } else { theme::TEXT_BLUE }, false));
            cells.push((x.career_goals.to_string(), theme::TEXT, false));
            cells.push((assists.to_string(), theme::TEXT, false));
            match x.season_average_rating() {
                Some(r) => cells.push((format::rating(t, r), widgets::rating_color(r), false)),
                None => cells.push(("--".into(), theme::TEXT, false)),
            }
            let v = brasfoot_engine::tactics::best_value(world, p);
            cells.push((v.to_string(), theme::rating_color(v), true));
            let mut left = rect.left();
            for (((key, _), w), (text, color, bold)) in columns.iter().zip(&widths).zip(cells) {
                let cell = egui::Rect::from_min_size(egui::pos2(left, rect.top()), egui::vec2(*w, rect.height()));
                left += w;
                let font = if bold { theme::bold(11.0) } else { small.clone() };
                match *key {
                    "squad-column-name" => {
                        let mut tx = cell.left() + 4.0;
                        let star = if x.world_class {
                            Some("aicons/estrelared.png")
                        } else if x.star {
                            Some("aicons/estrela.png")
                        } else {
                            None
                        };
                        if let Some(icon) = star.and_then(|i| files.jar_image(&ctx, i)) {
                            egui::Image::new(icon).paint_at(ui, egui::Rect::from_min_size(egui::pos2(tx, cell.center().y - 8.0), egui::vec2(16.0, 16.0)));
                            tx += 17.0;
                        }
                        let painter = ui.painter().with_clip_rect(cell.intersect(ui.clip_rect()));
                        painter.text(egui::pos2(tx, cell.center().y), egui::Align2::LEFT_CENTER, text, font, color);
                    }
                    "squad-column-energy" => widgets::paint_energy_bar(ui.painter(), cell, x.fitness),
                    "" if x.is_injured(today) && cell.left() > rect.left() + 30.0 => {
                        if let Some(icon) = files.jar_image(&ctx, "aicons/icontusao.png") {
                            egui::Image::new(icon).paint_at(ui, egui::Rect::from_center_size(cell.center(), egui::vec2(14.0, 14.0)));
                        }
                    }
                    _ => {
                        ui.painter().text(cell.center(), egui::Align2::CENTER_CENTER, text, font, color);
                    }
                }
            }
        }
    });
    if let Some(p) = clicked {
        s.selected_player = Some(p);
        ctx.request_repaint();
    }
    if let Some(p) = opened {
        s.open_player = Some(p);
    }
    if let Some((p, a)) = action {
        player_actions::apply(s, p, a);
    }
}
