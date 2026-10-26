//! The Teams screen (`a.iA`, "Ver time"): browse any club in the world, its squad, coach,
//! confidence, trophies and players with their career history.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::countries;
use brasfoot_model::ids::{ClubId, CompetitionId, PlayerId};
use brasfoot_model::player::attribute;
use brasfoot_model::world::World;
use eframe::egui::{self, Color32, Stroke};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::{contract, menu, offer, youth};
use crate::screens::widgets::{self, crest, flag};
use crate::{format, motion, theme};

const BRAZILIAN_STATES: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE",
    "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

const REPUTATIONS: [&str; 6] = [
    "teams-reputation-0",
    "teams-reputation-1",
    "teams-reputation-2",
    "teams-reputation-3",
    "teams-reputation-4",
    "teams-reputation-5",
];

pub fn window(s: &mut CareerScreen, ctx: &egui::Context, t: &Translator, files: &GameFiles, open: &mut bool) {
    let frame = egui::Frame::new().fill(theme::PANEL).stroke(Stroke::new(1.0, Color32::BLACK)).inner_margin(12);
    motion::dialog(ctx, "teams", open, egui::vec2(1200.0, 740.0), frame, |ui, open| {
        body(s, ui, t, files, open);
    });
}

fn body(s: &mut CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, open: &mut bool) {
    let viewed_club_id = s.viewed_club.unwrap_or(s.club);
    let human_club_id = s.career.human_club();

    // Top title bar with close button
    if widgets::dialog_title(ui, files, Some(glyphs::TEAMS), &t.text("teams-title", None)) {
        *open = false;
    }
    ui.add_space(10.0);

    ui.horizontal_top(|ui| {
        // Left Column: Club Tree, Selected Player Card, Cash
        ui.vertical(|ui| {
            ui.set_width(310.0);
            left_column(s, ui, t, files, viewed_club_id, human_club_id);
        });

        ui.add_space(12.0);

        // Right Column: Club Header, Toolbar, Squad Table, Bottom Actions
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            right_column(s, ui, t, files, viewed_club_id, human_club_id, open);
        });
    });
}

fn left_column(
    s: &mut CareerScreen,
    ui: &mut egui::Ui,
    t: &Translator,
    files: &GameFiles,
    viewed: ClubId,
    human_club_id: Option<ClubId>,
) {
    // Search filter input
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut s.teams_search)
                .hint_text(t.text("teams-search-placeholder", None))
                .desired_width(ui.available_width()),
        );
    });
    ui.add_space(6.0);

    // Club Tree in scroll area
    let mut picked_club = None;
    egui::Frame::new().fill(theme::OUTER).inner_margin(6).corner_radius(4).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::vertical().id_salt("teams-tree").max_height(280.0).show(ui, |ui| {
            if let Some(c) = club_tree(ui, t, files, &s.career.world, &s.teams_search, viewed) {
                picked_club = Some(c);
            }
        });
    });
    if let Some(c) = picked_club {
        s.viewed_club = Some(c);
        s.teams_selected_player = None;
    }

    ui.add_space(8.0);

    // Selected Player Card
    let squad = listed_squad(&s.career.world, viewed);
    let selected_player_id = s.teams_selected_player.filter(|p| squad.contains(p)).or_else(|| squad.first().copied());

    if let Some(player_id) = selected_player_id {
        player_card(ui, t, files, &s.career.world, &s.career.runtime.clock, &mut s.teams_player_tab, player_id);
    }

    // Cash label at bottom
    if let Some(human_club) = human_club_id {
        let cash = s.career.world.club(human_club).cash;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if let Some(uri) = files.jar_image(ui.ctx(), "aicons/iconmoneys.png") {
                ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(16.0, 16.0)));
            }
            ui.label(
                egui::RichText::new(t.text("teams-cash", Some(&args([("amount", format::money(t, cash))]))))
                    .font(theme::bold(12.0))
                    .color(theme::GOOD),
            );
        });
    }
}

fn club_tree(
    ui: &mut egui::Ui,
    t: &Translator,
    files: &GameFiles,
    world: &World,
    search_query: &str,
    viewed: ClubId,
) -> Option<ClubId> {
    let mut clicked_club = None;
    let query = search_query.trim().to_lowercase();

    for &country_id in &world.game.league_countries {
        let country = world.country(country_id);
        let code = countries::country(country.country).map(|c| c.code.to_lowercase()).unwrap_or_default();
        let country_name = t.text(&format!("country-{code}"), None);

        // Check if country or any club matches search query
        let matches_country = query.is_empty() || country_name.to_lowercase().contains(&query);

        let mut matching_divisions: Vec<(CompetitionId, Vec<ClubId>)> = Vec::new();
        for &league_id in &country.divisions {
            let comp = world.competition(league_id);
            if let Some(stage_id) = comp.league {
                let clubs = world.stage(stage_id).clubs.clone();
                let filtered: Vec<ClubId> = clubs
                    .into_iter()
                    .filter(|&c| matches_country || world.club(c).name.to_lowercase().contains(&query))
                    .collect();
                if !filtered.is_empty() {
                    matching_divisions.push((league_id, filtered));
                }
            }
        }

        let matching_regionals: Vec<ClubId> = country
            .list_hw
            .iter()
            .copied()
            .filter(|&c| matches_country || world.club(c).name.to_lowercase().contains(&query))
            .collect();

        if matching_divisions.is_empty() && matching_regionals.is_empty() {
            continue;
        }

        let id = ui.make_persistent_id(format!("country-node-{}", country_id.0));
        let open_by_default = !query.is_empty() || country.clubs.contains(&viewed);

        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open_by_default)
            .show_header(ui, |ui| {
                flag(ui, files, country.country, 14.0);
                ui.label(egui::RichText::new(&country_name).font(theme::bold(12.0)));
            })
            .body(|ui| {
                for (league_id, mut clubs) in matching_divisions {
                    let div_name = widgets::competition_name(t, world, league_id);
                    let div_id = ui.make_persistent_id(format!("league-node-{}", league_id.0));
                    let div_open = !query.is_empty() || clubs.contains(&viewed);

                    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), div_id, div_open)
                        .show_header(ui, |ui| {
                            ui.label(egui::RichText::new(&div_name).font(theme::regular(11.0)).color(theme::MUTED));
                        })
                        .body(|ui| {
                            clubs.sort_by(|&a, &b| world.club(a).name.cmp(&world.club(b).name));
                            for c in clubs {
                                if club_row(ui, files, world, c, viewed) {
                                    clicked_club = Some(c);
                                }
                            }
                        });
                }

                if !matching_regionals.is_empty() {
                    let reg_id = ui.make_persistent_id(format!("reg-node-{}", country_id.0));
                    let reg_open = !query.is_empty() || matching_regionals.contains(&viewed);
                    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), reg_id, reg_open)
                        .show_header(ui, |ui| {
                            ui.label(egui::RichText::new(t.text("teams-regionals", None)).font(theme::regular(11.0)).color(theme::MUTED));
                        })
                        .body(|ui| {
                            let mut regs = matching_regionals.clone();
                            regs.sort_by(|&a, &b| world.club(a).name.cmp(&world.club(b).name));
                            for c in regs {
                                if club_row(ui, files, world, c, viewed) {
                                    clicked_club = Some(c);
                                }
                            }
                        });
                }
            });
    }

    clicked_club
}

fn club_row(ui: &mut egui::Ui, files: &GameFiles, world: &World, club: ClubId, viewed: ClubId) -> bool {
    let name = &world.club(club).name;
    let selected = club == viewed;
    let mut clicked = false;

    ui.horizontal(|ui| {
        crest(ui, files, world, club, 16.0);
        if ui.selectable_label(selected, name).clicked() {
            clicked = true;
        }
    });

    clicked
}

fn player_card(
    ui: &mut egui::Ui,
    t: &Translator,
    files: &GameFiles,
    world: &World,
    clock: &brasfoot_model::clock::Clock,
    tab: &mut usize,
    player_id: PlayerId,
) {
    let p = world.player(player_id);

    egui::Frame::new().fill(theme::PANEL).inner_margin(8).corner_radius(4).stroke(Stroke::new(1.0, theme::BORDER)).show(ui, |ui| {
        // Player header
        ui.horizontal(|ui| {
            flag(ui, files, p.country, 14.0);
            let pos = widgets::position_short(t, p.position);
            ui.label(egui::RichText::new(format!("{} - {pos}", p.name)).font(theme::bold(12.0)));
            if p.world_class {
                if let Some(uri) = files.jar_image(ui.ctx(), "aicons/estrelared.png") {
                    ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(12.0, 12.0)));
                }
            } else if p.star {
                if let Some(uri) = files.jar_image(ui.ctx(), "aicons/estrela.png") {
                    ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(12.0, 12.0)));
                }
            }
        });
        ui.add_space(4.0);

        // Tabs: Info | Carreira
        ui.horizontal(|ui| {
            if ui.selectable_label(*tab == 0, t.text("teams-tab-info", None)).clicked() {
                *tab = 0;
            }
            if ui.selectable_label(*tab == 1, t.text("teams-tab-career", None)).clicked() {
                *tab = 1;
            }
        });
        ui.separator();

        if *tab == 0 {
            // Tab Info
            egui::Grid::new("teams-player-info").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                if !world.game.individual_skills {
                    ui.label(egui::RichText::new(t.text("teams-player-strength", Some(&args([("strength", p.strength.to_string())])))).font(theme::bold(11.0)));
                } else {
                    ui.label("");
                }
                ui.label(egui::RichText::new(t.text("teams-player-age", Some(&args([("age", p.age.to_string())])))).font(theme::bold(11.0)));
                ui.end_row();

                ui.label(t.text("teams-player-games", Some(&args([("games", p.career_games.to_string())]))));
                ui.label(t.text("teams-player-goals", Some(&args([("goals", p.career_goals.to_string())]))));
                ui.end_row();

                ui.label(t.text("teams-player-pass", Some(&args([("value", format::money(t, p.market_value as i64))]))));
                ui.label(t.text("teams-player-salary", Some(&args([("salary", format::money(t, p.salary as i64))]))));
                ui.end_row();

                let traits = format!("{} / {}", widgets::trait_short(t, p.trait1), widgets::trait_short(t, p.trait2));
                ui.label(traits);
                ui.label(widgets::side_short(t, p.side));
                ui.end_row();
            });

            if p.injured_until > world.today_millis() {
                ui.add_space(4.0);
                let date = format::date(t, clock, p.injured_until);
                ui.label(egui::RichText::new(t.text("teams-player-injury-end", Some(&args([("date", date)])))).color(theme::BAD));
            }
        } else {
            // Tab Carreira
            egui::ScrollArea::vertical().id_salt("teams-player-career").max_height(140.0).show(ui, |ui| {
                egui::Grid::new("teams-career-table").striped(true).spacing([6.0, 2.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new(t.text("teams-career-col-year", None)).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(t.text("teams-career-col-club", None)).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(t.text("teams-career-col-games", None)).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(t.text("teams-career-col-goals", None)).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(t.text("teams-career-col-yellow", None)).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(t.text("teams-career-col-red", None)).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(t.text("teams-career-col-injury", None)).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(t.text("teams-career-col-assists", None)).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(t.text("teams-career-col-average", None)).font(theme::bold(10.0)));
                    ui.end_row();

                    let mut tot_games = 0;
                    let mut tot_goals = 0;
                    let mut tot_yellow = 0;
                    let mut tot_red = 0;
                    let mut tot_injuries = 0;
                    let mut tot_assists = 0;
                    let mut tot_rating_sum = 0.0;
                    let mut tot_rated_games = 0;

                    for stat in p.season_stats.iter().rev().take(10) {
                        tot_games += stat.games;
                        tot_goals += stat.goals;
                        tot_yellow += stat.yellow_cards;
                        tot_red += stat.red_cards;
                        tot_injuries += stat.injuries;
                        tot_assists += stat.assists;
                        tot_rating_sum += stat.rating_sum;
                        tot_rated_games += stat.rated_games;

                        ui.label(stat.season.to_string());
                        let club_name = if stat.club_id >= 0 && (stat.club_id as usize) < world.clubs.len() {
                            world.club(ClubId(stat.club_id as u32)).name.clone()
                        } else {
                            t.text("teams-career-promoted", None)
                        };
                        ui.label(club_name);
                        ui.label(stat.games.to_string());
                        ui.label(stat.goals.to_string());
                        ui.label(stat.yellow_cards.to_string());
                        ui.label(stat.red_cards.to_string());
                        ui.label(stat.injuries.to_string());
                        ui.label(stat.assists.to_string());
                        let avg = if stat.rated_games > 0 { format!("{:.1}", stat.rating_sum / stat.rated_games as f64) } else { "-".to_owned() };
                        ui.label(avg);
                        ui.end_row();
                    }

                    // Total carreira row
                    ui.label(egui::RichText::new(t.text("teams-career-total", None)).font(theme::bold(10.0)));
                    ui.label("");
                    ui.label(egui::RichText::new(tot_games.to_string()).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(tot_goals.to_string()).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(tot_yellow.to_string()).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(tot_red.to_string()).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(tot_injuries.to_string()).font(theme::bold(10.0)));
                    ui.label(egui::RichText::new(tot_assists.to_string()).font(theme::bold(10.0)));
                    let tot_avg = if tot_rated_games > 0 { format!("{:.1}", tot_rating_sum / tot_rated_games as f64) } else { "-".to_owned() };
                    ui.label(egui::RichText::new(tot_avg).font(theme::bold(10.0)));
                    ui.end_row();
                });
            });
        }
    });
}

fn right_column(
    s: &mut CareerScreen,
    ui: &mut egui::Ui,
    t: &Translator,
    files: &GameFiles,
    viewed: ClubId,
    human_club_id: Option<ClubId>,
    open: &mut bool,
) {
    // Club Header Panel (`wf`)
    egui::Frame::new().fill(theme::PANEL_HIGH).inner_margin(12).corner_radius(6).show(ui, |ui| {
        let world = &s.career.world;
        let club = world.club(viewed);
        ui.horizontal(|ui| {
            crest(ui, files, world, viewed, 56.0);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&club.name).font(theme::bold(18.0)).color(Color32::WHITE));
                let coach_name = club.coach.map(|c| world.coach(c).name.clone()).unwrap_or_else(|| t.text("teams-coach-interim", None));
                ui.label(egui::RichText::new(format!("{} {coach_name}", t.text("teams-coach", None))).font(theme::regular(12.0)).color(theme::MUTED));

                ui.horizontal(|ui| {
                    let rep_key = REPUTATIONS[club.reputation.clamp(0, 5) as usize];
                    ui.label(egui::RichText::new(t.text(rep_key, None)).font(theme::bold(11.0)).color(theme::ACCENT));
                    ui.label("•");

                    let country_id = find_club_country(world, viewed);
                    flag(ui, files, country_id, 14.0);
                    let code = countries::country(country_id).map(|c| c.code.to_lowercase()).unwrap_or_default();
                    let c_name = t.text(&format!("country-{code}"), None);
                    let display_country = if country_id == 29 && (club.state as usize) < BRAZILIAN_STATES.len() {
                        format!("{c_name} ({})", BRAZILIAN_STATES[club.state as usize])
                    } else {
                        c_name
                    };
                    ui.label(egui::RichText::new(display_country).font(theme::regular(11.0)));
                });

                let squad_value: i64 = club.squad.iter().map(|&p| world.player(p).market_value as i64).sum();
                ui.label(
                    egui::RichText::new(t.text("teams-squad-value", Some(&args([("value", format::money(t, squad_value))]))))
                        .font(theme::bold(11.0))
                        .color(theme::GOOD),
                );
            });

            // Confidence Progress Bars on the right of header
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.vertical(|ui| {
                    let (board, fans) = if let Some(coach) = club.coach {
                        (world.coach(coach).board_confidence, world.coach(coach).fan_support)
                    } else {
                        (90, 80)
                    };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(t.text("teams-board-confidence", None)).font(theme::regular(11.0)));
                        ui.add(egui::ProgressBar::new(board as f32 / 100.0).desired_width(120.0).text(format!("{board}%")).fill(theme::ACCENT));
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(t.text("teams-fan-confidence", None)).font(theme::regular(11.0)));
                        ui.add(egui::ProgressBar::new(fans as f32 / 100.0).desired_width(120.0).text(format!("{fans}%")).fill(theme::ACCENT));
                    });
                });
            });
        });
    });

    ui.add_space(6.0);

    // Club Toolbar (`wg`)
    ui.horizontal(|ui| {
        let mut btn = |glyph: &str, tooltip: &str, enabled: bool| widgets::icon_button(ui, glyph, 20.0, theme::TEXT, enabled).on_hover_text(tooltip);

        if btn(glyphs::CALENDAR, &t.text("calendar-title", None), true).clicked() {
            s.club = viewed;
            s.calendar_open = true;
        }
        if btn(glyphs::TROPHY, &t.text("trophies-title", None), true).clicked() {
            s.trophies_club = Some(viewed);
            s.trophies_open = true;
        }
        btn(glyphs::CLUB_RECORD, &t.text("home-club-record", None), false);
        btn(glyphs::MARKET, &t.text("toolbar-market", None), false);
        if btn(glyphs::YOUTH, &t.text("toolbar-youth", None), true).clicked() {
            s.youth_state = Some(youth::YouthState::new(viewed));
            s.youth_open = true;
        }

        ui.add_space(16.0);
        if ui.button(t.text("teams-button-view-main", None)).clicked() {
            s.club = viewed;
            s.selected_player = None;
            *open = false;
        }
    });

    ui.add_space(6.0);

    // Squad Table (`b.U`)
    // its own scope: the table takes the spacing away for its rows only
    ui.scope(|ui| squad_table(s, ui, t, files, viewed));

    ui.add_space(8.0);

    // Bottom Action Buttons: Empréstimo, Comprar, Fazer oferta, Fechar (or Renovar if own club)
    ui.horizontal(|ui| {
        // the squad table above takes the spacing away; the buttons get it back
        ui.spacing_mut().item_spacing.x = 10.0;
        let is_own_club = human_club_id == Some(viewed);
        let selected_player = s.teams_selected_player.map(|p| s.career.world.player(p));

        if is_own_club {
            let renew_enabled = s.teams_selected_player.is_some();
            if menu::action_button(ui, &t.text("contract-title", None), renew_enabled, false).clicked() {
                if let Some(p) = s.teams_selected_player {
                    s.contract_state = Some(contract::ContractState::new(&s.career.world, p));
                    s.contract_open = true;
                }
            }
        } else {
            let loan_enabled = selected_player.map(|p| p.listed_for_loan).unwrap_or(false);
            let buy_enabled = selected_player.map(|p| p.listed_for_sale).unwrap_or(false);
            let offer_enabled = selected_player.is_some();

            if menu::action_button(ui, &t.text("teams-button-loan", None), loan_enabled, false).on_hover_text(t.text("teams-action-loan-tooltip", None)).clicked() {
                s.status = Some((t.text("teams-action-loan-tooltip", None), theme::ACCENT));
            }

            if menu::action_button(ui, &t.text("teams-button-buy", None), buy_enabled, false).on_hover_text(t.text("teams-action-buy-tooltip", None)).clicked() {
                if let Some(p) = s.teams_selected_player {
                    s.offer_state = Some(offer::OfferState::new(&s.career.world, p));
                    s.offer_open = true;
                }
            }

            if menu::action_button(ui, &t.text("teams-button-offer", None), offer_enabled, true).on_hover_text(t.text("teams-action-offer-tooltip", None)).clicked() {
                if let Some(p) = s.teams_selected_player {
                    s.offer_state = Some(offer::OfferState::new(&s.career.world, p));
                    s.offer_open = true;
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if menu::action_button(ui, &t.text("teams-button-close", None), true, false).clicked() {
                *open = false;
            }
        });
    });
}

fn squad_table(s: &mut CareerScreen, ui: &mut egui::Ui, t: &Translator, files: &GameFiles, viewed: ClubId) {
    let world = &s.career.world;
    let skills = world.game.individual_skills;
    let squad = listed_squad(world, viewed);

    let mut columns: Vec<(&str, f32)> = vec![
        ("squad-column-position", 24.0),
        ("", 24.0), // flag
        ("squad-column-name", 160.0),
        ("squad-column-side", 26.0),
    ];

    if skills {
        for key in [
            "squad-column-goalkeeping",
            "squad-column-defending",
            "squad-column-playmaking",
            "squad-column-finishing",
            "squad-column-speed",
            "squad-column-technique",
            "squad-column-passing",
        ] {
            columns.push((key, 32.0));
        }
    } else {
        columns.push(("squad-column-strength", 30.0));
    }

    columns.extend([
        ("squad-column-salary", 80.0),
        ("squad-column-value", 80.0),
        ("squad-column-traits", 55.0),
        ("squad-column-career-goals", 32.0),
        ("squad-column-assists", 30.0),
        ("squad-column-age", 32.0),
        ("squad-column-for-sale", 24.0),
        ("squad-column-for-loan", 24.0),
    ]);

    let fixed: f32 = columns.iter().filter(|(k, _)| *k != "squad-column-name").map(|(_, w)| w).sum();
    let name_width = (ui.available_width() - fixed - 2.0).max(120.0);
    let widths: Vec<f32> = columns.iter().map(|(k, w)| if *k == "squad-column-name" { name_width } else { *w }).collect();
    let total: f32 = widths.iter().sum();

    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    let ctx = ui.ctx().clone();

    // Table Header
    let (header, _) = ui.allocate_exact_size(egui::vec2(total, 24.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(header, 0.0, theme::PANEL);
    painter.line_segment([header.left_bottom(), header.right_bottom()], Stroke::new(1.0, theme::GRID));

    let mut x = header.left();
    for ((key, _), w) in columns.iter().zip(&widths) {
        let cell = egui::Rect::from_min_size(egui::pos2(x, header.top()), egui::vec2(*w, header.height()));
        painter.line_segment([cell.right_top(), cell.right_bottom()], Stroke::new(1.0, theme::GRID));
        if !key.is_empty() {
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, t.text(key, None), theme::regular(11.0), theme::TEXT);
        }
        x += w;
    }

    let mut current = s.teams_selected_player.or_else(|| squad.first().copied());
    let mut clicked = None;
    let mut opened = None;

    egui::ScrollArea::vertical().id_salt("teams-squad-scroll").max_height(340.0).show(ui, |ui| {
        for (i, &p) in squad.iter().enumerate() {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(total, 20.0), egui::Sense::click());
            if response.clicked() {
                clicked = Some(p);
                current = Some(p);
            }
            if response.double_clicked() {
                opened = Some(p);
            }
            if response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if !ui.is_rect_visible(rect) {
                continue;
            }

            let pl = world.player(p);
            let base = if i % 2 == 0 { theme::PANEL } else { theme::ROW_ALT };
            let hover = motion::toward(&ctx, response.id, response.hovered(), motion::HOVER);
            let select = motion::toward(&ctx, response.id.with("sel"), current == Some(p), motion::SELECT);

            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, motion::mix(motion::mix(base, theme::HOVER, hover), theme::SELECTION, select));

            if select > 0.0 {
                let mark = egui::Rect::from_center_size(egui::pos2(rect.left() + 1.5, rect.center().y), egui::vec2(3.0, rect.height() * select));
                painter.rect_filled(mark, 0.0, theme::ACCENT);
            }

            let mut cur_x = rect.left();

            // P (position)
            let w_pos = widths[0];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_pos, rect.height()));
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, widgets::position_short(t, pl.position), theme::bold(10.0), theme::TEXT);
            cur_x += w_pos;

            // Flag
            let w_flag = widths[1];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_flag, rect.height()));
            if let Some(uri) = files.jar_image(&ctx, &format!("aflags/{}.png", pl.country)) {
                let img_rect = egui::Rect::from_center_size(cell.center(), egui::vec2(13.0, 13.0));
                egui::Image::new(uri).paint_at(ui, img_rect);
            }
            cur_x += w_flag;

            // Name
            let w_name = widths[2];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_name, rect.height()));
            let name_color = if pl.listed_for_sale { theme::TEXT_BLUE } else { theme::TEXT };
            painter.text(egui::pos2(cell.left() + 4.0, cell.center().y), egui::Align2::LEFT_CENTER, &pl.name, theme::regular(11.0), name_color);
            if pl.world_class {
                if let Some(uri) = files.jar_image(&ctx, "aicons/estrelared.png") {
                    let star_rect = egui::Rect::from_center_size(egui::pos2(cell.right() - 8.0, cell.center().y), egui::vec2(10.0, 10.0));
                    egui::Image::new(uri).paint_at(ui, star_rect);
                }
            } else if pl.star {
                if let Some(uri) = files.jar_image(&ctx, "aicons/estrela.png") {
                    let star_rect = egui::Rect::from_center_size(egui::pos2(cell.right() - 8.0, cell.center().y), egui::vec2(10.0, 10.0));
                    egui::Image::new(uri).paint_at(ui, star_rect);
                }
            }
            cur_x += w_name;

            // L (Side)
            let w_side = widths[3];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_side, rect.height()));
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, widgets::side_short(t, pl.side), theme::regular(10.0), theme::TEXT);
            cur_x += w_side;

            let mut col_idx = 4;
            if skills {
                for attr_id in [
                    attribute::GOALKEEPING,
                    attribute::DEFENDING,
                    attribute::PLAYMAKING,
                    attribute::FINISHING,
                    attribute::SPEED,
                    attribute::TECHNIQUE,
                    attribute::PASSING,
                ] {
                    let w_attr = widths[col_idx];
                    let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_attr, rect.height()));
                    let val = pl.attributes[attr_id as usize];
                    painter.text(cell.center(), egui::Align2::CENTER_CENTER, val.to_string(), theme::regular(10.0), theme::TEXT);
                    cur_x += w_attr;
                    col_idx += 1;
                }
            } else {
                let w_str = widths[col_idx];
                let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_str, rect.height()));
                painter.text(cell.center(), egui::Align2::CENTER_CENTER, pl.strength.to_string(), theme::bold(11.0), theme::TEXT);
                cur_x += w_str;
                col_idx += 1;
            }

            // Salário
            let w_sal = widths[col_idx];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_sal, rect.height()));
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, format::money(t, pl.salary as i64), theme::regular(10.0), theme::TEXT);
            cur_x += w_sal;
            col_idx += 1;

            // Valor
            let w_val = widths[col_idx];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_val, rect.height()));
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, format::money(t, pl.market_value as i64), theme::regular(10.0), theme::TEXT);
            cur_x += w_val;
            col_idx += 1;

            // Car.
            let w_car = widths[col_idx];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_car, rect.height()));
            let traits_str = format!("{}/{}", widgets::trait_short(t, pl.trait1), widgets::trait_short(t, pl.trait2));
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, traits_str, theme::regular(10.0), theme::TEXT);
            cur_x += w_car;
            col_idx += 1;

            // GC (Career goals)
            let w_gc = widths[col_idx];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_gc, rect.height()));
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, pl.career_goals.to_string(), theme::regular(10.0), theme::TEXT);
            cur_x += w_gc;
            col_idx += 1;

            // A (Assists)
            let w_a = widths[col_idx];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_a, rect.height()));
            let (assists, _, _) = pl.season_totals(world.season());
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, assists.to_string(), theme::regular(10.0), theme::TEXT);
            cur_x += w_a;
            col_idx += 1;

            // Idade
            let w_age = widths[col_idx];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_age, rect.height()));
            painter.text(cell.center(), egui::Align2::CENTER_CENTER, pl.age.to_string(), theme::regular(10.0), theme::TEXT);
            cur_x += w_age;
            col_idx += 1;

            // V (Listed for sale)
            let w_v = widths[col_idx];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_v, rect.height()));
            if pl.listed_for_sale {
                if let Some(uri) = files.jar_image(&ctx, "aicons/iconchecked.png") {
                    let icon_rect = egui::Rect::from_center_size(cell.center(), egui::vec2(12.0, 12.0));
                    egui::Image::new(uri).paint_at(ui, icon_rect);
                }
            }
            cur_x += w_v;
            col_idx += 1;

            // E (Listed for loan)
            let w_e = widths[col_idx];
            let cell = egui::Rect::from_min_size(egui::pos2(cur_x, rect.top()), egui::vec2(w_e, rect.height()));
            if pl.listed_for_loan {
                if let Some(uri) = files.jar_image(&ctx, "aicons/iconchecked.png") {
                    let icon_rect = egui::Rect::from_center_size(cell.center(), egui::vec2(12.0, 12.0));
                    egui::Image::new(uri).paint_at(ui, icon_rect);
                }
            }
        }
    });

    if let Some(p) = clicked {
        s.teams_selected_player = Some(p);
    }
    if let Some(p) = opened {
        s.open_player = Some(p);
    }
}

fn listed_squad(world: &World, club: ClubId) -> Vec<PlayerId> {
    let mut squad = world.club(club).squad.clone();
    squad.sort_by(|&a, &b| {
        let (x, y) = (world.player(a), world.player(b));
        x.position.cmp(&y.position).then(x.style.cmp(&y.style)).then(y.strength.cmp(&x.strength)).then(y.star.cmp(&x.star))
    });
    squad
}

fn find_club_country(world: &World, club: ClubId) -> i32 {
    for &country_id in &world.game.league_countries {
        let country = world.country(country_id);
        if country.clubs.contains(&club) || country.list_hw.contains(&club) {
            return country.country;
        }
    }
    29
}
