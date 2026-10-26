//! The Youth Academy & U-20 League screen (`bfmod.BaseDialog`, "Base e Campeonato Sub-20"):
//! manage junior prospects, follow the national youth league standings, and scout gems.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_systems::awards::season::player_key;
use brasfoot_systems::youth_league::{self, TableRow};
use eframe::egui::{self, Color32, Stroke, vec2};

use crate::game_files::GameFiles;
use crate::glyphs;
use crate::screens::career::CareerScreen;
use crate::screens::modkit::{GOLD, GOLD_DARK};
use crate::screens::widgets;
use crate::{motion, theme};

pub struct YouthState {
    pub club: ClubId,
    pub tab: usize,
    pub selected_player: Option<PlayerId>,
    pub status: Option<(String, Color32)>,
}

impl YouthState {
    pub fn new(club: ClubId) -> Self {
        YouthState {
            club,
            tab: 0,
            selected_player: None,
            status: None,
        }
    }
}

const TABS: [&str; 3] = [
    "youth-tab-academy",
    "youth-tab-championship",
    "youth-tab-scouting",
];

pub fn window(
    s: &mut CareerScreen,
    opt_state: &mut Option<YouthState>,
    ctx: &egui::Context,
    t: &Translator,
    files: &GameFiles,
    open: &mut bool,
) {
    let frame = egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, GOLD_DARK))
        .inner_margin(16);

    motion::dialog(ctx, "youth_dialog", open, vec2(1000.0, 660.0), frame, |ui, open| {
        body(s, opt_state, ui, t, files, open);
    });
}

fn body(
    s: &mut CareerScreen,
    opt_state: &mut Option<YouthState>,
    ui: &mut egui::Ui,
    t: &Translator,
    files: &GameFiles,
    open: &mut bool,
) {
    let state = match opt_state.as_mut() {
        Some(st) => st,
        None => {
            *open = false;
            return;
        }
    };

    let club = s.career.world.club(state.club);
    let youth_count = club.youth_squad.len();
    let is_own_club = s.career.human_club() == Some(state.club);

    if widgets::dialog_title_with(ui, files, Some(glyphs::YOUTH), &t.text("youth-title", None), |ui| {
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(t.text("youth-count", Some(&args([("count", youth_count.to_string())]))))
                .font(theme::regular(12.0))
                .color(theme::MUTED),
        );
    }) {
        *open = false;
    }

    ui.add_space(8.0);

    // Tab buttons
    ui.horizontal(|ui| {
        for (i, &tab_key) in TABS.iter().enumerate() {
            let active = state.tab == i;
            let label = t.text(tab_key, None);
            let btn = egui::Button::new(
                egui::RichText::new(&label)
                    .font(if active { theme::bold(13.0) } else { theme::regular(13.0) })
                    .color(if active { Color32::WHITE } else { theme::MUTED }),
            )
            .fill(if active { theme::PANEL_HIGH } else { Color32::TRANSPARENT })
            .stroke(if active { Stroke::new(1.0, GOLD) } else { Stroke::NONE })
            .corner_radius(4.0)
            .min_size(vec2(120.0, 28.0));

            if ui.add(btn).clicked() {
                state.tab = i;
            }
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    match state.tab {
        0 => tab_academy(s, state, ui, t, files, is_own_club),
        1 => tab_championship(s, ui, t),
        _ => tab_scouting(s, ui, t),
    }

    ui.add_space(8.0);

    // Status message if any
    if let Some((msg, color)) = &state.status {
        ui.label(egui::RichText::new(msg).font(theme::bold(12.0)).color(*color));
        ui.add_space(4.0);
    }
}

/// Tab 0: Academia (Juniores)
fn tab_academy(
    s: &mut CareerScreen,
    state: &mut YouthState,
    ui: &mut egui::Ui,
    t: &Translator,
    _files: &GameFiles,
    is_own_club: bool,
) {
    let squad_copy = s.career.world.club(state.club).youth_squad.clone();
    let youth_count = squad_copy.len();

    // Toolbar inside tab 0
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("Jovens no elenco: {youth_count}/20"))
                .font(theme::bold(13.0))
                .color(Color32::WHITE),
        );

        if is_own_club {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Promote button
                let has_selection = state.selected_player.is_some();
                let promote_btn = egui::Button::new(
                    egui::RichText::new(t.text("youth-button-promote", None))
                        .font(theme::bold(12.0))
                        .color(Color32::WHITE),
                )
                .fill(theme::ACCENT);

                if ui.add_enabled(has_selection, promote_btn).clicked() {
                    if let Some(p) = state.selected_player {
                        let c = s.career.world.club_mut(state.club);
                        if let Some(pos) = c.youth_squad.iter().position(|&id| id == p) {
                            c.youth_squad.remove(pos);
                            c.squad.push(p);
                            state.selected_player = None;
                            state.status = Some((t.text("youth-promoted-success", None), theme::GOOD));
                        }
                    }
                }

                ui.add_space(6.0);

                // Release button
                let release_btn = egui::Button::new(
                    egui::RichText::new(t.text("youth-button-release", None))
                        .font(theme::regular(12.0))
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(120, 45, 45));

                if ui.add_enabled(has_selection, release_btn).clicked() {
                    if let Some(p) = state.selected_player {
                        let youth = &mut s.career.world.club_mut(state.club).youth_squad;
                        if let Some(pos) = youth.iter().position(|&id| id == p) {
                            youth.remove(pos);
                            state.selected_player = None;
                            state.status = Some((t.text("youth-released-success", None), theme::WARN));
                        }
                    }
                }
            });
        }
    });

    ui.add_space(6.0);

    // Junior players table
    egui::Frame::new()
        .fill(theme::PANEL_HIGH)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .inner_margin(6)
        .corner_radius(4)
        .show(ui, |ui| {
            if youth_count == 0 {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(t.text("youth-empty", None)).font(theme::regular(13.0)).color(theme::MUTED));
                });
                ui.add_space(30.0);
            } else {
                table_header(ui, t);
                ui.separator();

                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    for player_id in squad_copy {
                        player_row(s, state, ui, t, player_id);
                    }
                });
            }
        });
}

fn table_header(ui: &mut egui::Ui, t: &Translator) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.set_min_height(22.0);
        ui.label(egui::RichText::new("★").font(theme::bold(12.0)).color(theme::MUTED));
        ui.add_space(8.0);
        ui.label(egui::RichText::new(t.text("squad-column-name", None)).font(theme::bold(12.0)).color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(12.0);
            ui.label(egui::RichText::new(t.text("squad-column-strength", None)).font(theme::bold(12.0)).color(theme::MUTED));
            ui.add_space(20.0);
            ui.label(egui::RichText::new(t.text("squad-column-age", None)).font(theme::bold(12.0)).color(theme::MUTED));
            ui.add_space(20.0);
            ui.label(egui::RichText::new(t.text("squad-column-position", None)).font(theme::bold(12.0)).color(theme::MUTED));
            ui.add_space(10.0);
        });
    });
}

fn player_row(
    s: &mut CareerScreen,
    state: &mut YouthState,
    ui: &mut egui::Ui,
    _t: &Translator,
    player_id: PlayerId,
) {
    let (name, strength, age, position) = {
        let p = s.career.world.player(player_id);
        (p.name.clone(), p.strength, p.age, p.position)
    };
    let key = player_key(&s.career.world, player_id);
    let is_priority = s.career.world.mods.youth_league.priority_starters.contains(&key);
    let selected = state.selected_player == Some(player_id);

    let mut toggle = false;

    let frame = egui::Frame::new()
        .fill(if selected { theme::SELECTION } else { Color32::TRANSPARENT })
        .inner_margin(egui::Margin::symmetric(4, 3))
        .corner_radius(2);

    let resp = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Star priority toggle
            let star = if is_priority { "★" } else { "☆" };
            let star_col = if is_priority { GOLD } else { theme::MUTED };
            if ui.small_button(egui::RichText::new(star).color(star_col)).clicked() {
                toggle = true;
            }

            ui.add_space(4.0);
            ui.label(egui::RichText::new(&name).font(theme::regular(13.0)).color(if selected { Color32::WHITE } else { theme::TEXT }));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(12.0);
                ui.label(egui::RichText::new(format!("{}", strength)).font(theme::bold(13.0)).color(Color32::WHITE));
                ui.add_space(24.0);
                ui.label(egui::RichText::new(format!("{}", age)).font(theme::regular(13.0)).color(theme::TEXT));
                ui.add_space(24.0);
                let pos_str = match position {
                    1 => "GOL",
                    2 => "LAT",
                    3 => "ZAG",
                    4 => "VOL",
                    5 => "MEI",
                    _ => "ATA",
                };
                ui.label(egui::RichText::new(pos_str).font(theme::bold(12.0)).color(theme::MUTED));
            });
        });
    });

    if toggle {
        if is_priority {
            s.career.world.mods.youth_league.priority_starters.retain(|k| k != &key);
        } else {
            s.career.world.mods.youth_league.priority_starters.push(key);
        }
    }

    if resp.response.interact(egui::Sense::click()).clicked() {
        state.selected_player = Some(player_id);
    }
}

/// Tab 1: Campeonato de Base (Tabela Sub-20)
fn tab_championship(s: &mut CareerScreen, ui: &mut egui::Ui, _t: &Translator) {
    let rows: Vec<TableRow> = youth_league::table(&s.career.world);
    let rounds = youth_league::rounds_played(&s.career.world.mods.youth_league);
    let total = youth_league::total_rounds(&s.career.world.mods.youth_league);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Campeonato Nacional Sub-20").font(theme::bold(15.0)).color(Color32::WHITE));
        ui.add_space(12.0);
        ui.label(egui::RichText::new(format!("Rodada: {rounds} / {total}")).font(theme::regular(13.0)).color(theme::MUTED));
    });

    ui.add_space(8.0);

    // Standings table
    egui::Frame::new()
        .fill(theme::PANEL_HIGH)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .inner_margin(6)
        .corner_radius(4)
        .show(ui, |ui| {
            if rows.is_empty() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Campeonato de Base ainda não iniciado nesta temporada.").font(theme::regular(13.0)).color(theme::MUTED));
                });
                ui.add_space(30.0);
                return;
            }

            ui.horizontal(|ui| {
                ui.set_min_height(22.0);
                ui.label(egui::RichText::new("#").font(theme::bold(12.0)).color(theme::MUTED));
                ui.add_space(12.0);
                ui.label(egui::RichText::new("Clube").font(theme::bold(12.0)).color(theme::MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    for (name, w) in [("SG", 35.0), ("GC", 35.0), ("GP", 35.0), ("D", 30.0), ("E", 30.0), ("V", 30.0), ("J", 30.0), ("P", 35.0)] {
                        ui.allocate_ui_with_layout(vec2(w, 20.0), egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(egui::RichText::new(name).font(theme::bold(12.0)).color(theme::MUTED));
                        });
                    }
                });
            });

            ui.separator();

            let human_club_id = s.career.human_club().map(|c| s.career.world.club(c).id);

            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for (pos, row) in rows.iter().enumerate() {
                    let is_human = human_club_id == Some(row.club);
                    let bg = if is_human { Color32::from_rgb(0x1a, 0x33, 0x22) } else { Color32::TRANSPARENT };

                    egui::Frame::new().fill(bg).inner_margin(egui::Margin::symmetric(4, 3)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let pos_color = match pos {
                                0 => GOLD,
                                1..=3 => Color32::from_rgb(0x3b, 0x82, 0xf6),
                                _ => theme::MUTED,
                            };
                            ui.label(egui::RichText::new(format!("{:2}", pos + 1)).font(theme::bold(12.0)).color(pos_color));
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(&row.name)
                                    .font(if is_human { theme::bold(13.0) } else { theme::regular(13.0) })
                                    .color(if is_human { GOLD } else { Color32::WHITE }),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add_space(8.0);
                                let sg = row.goal_difference();
                                let sg_str = if sg > 0 { format!("+{sg}") } else { format!("{sg}") };
                                for (val, w) in [
                                    (sg_str, 35.0),
                                    (format!("{}", row.goals_against), 35.0),
                                    (format!("{}", row.goals_for), 35.0),
                                    (format!("{}", row.losses), 30.0),
                                    (format!("{}", row.draws), 30.0),
                                    (format!("{}", row.wins), 30.0),
                                    (format!("{}", row.games), 30.0),
                                    (format!("{}", row.points), 35.0),
                                ] {
                                    ui.allocate_ui_with_layout(vec2(w, 20.0), egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                        ui.label(egui::RichText::new(val).font(theme::regular(12.5)).color(Color32::WHITE));
                                    });
                                }
                            });
                        });
                    });
                }
            });
        });
}

/// Tab 2: Destaques e Garimpo
fn tab_scouting(s: &mut CareerScreen, ui: &mut egui::Ui, _t: &Translator) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Joias Reveladas e Histórico").font(theme::bold(15.0)).color(Color32::WHITE));
    });

    ui.add_space(8.0);

    let gems = s.career.world.mods.gems.clone();
    let history = s.career.world.mods.youth_league.history.clone();

    ui.columns(2, |cols| {
        // Left: Joias
        cols[0].group(|ui| {
            ui.label(egui::RichText::new("Maiores Promessas (Joias)").font(theme::bold(14.0)).color(GOLD));
            ui.add_space(6.0);
            if gems.is_empty() {
                ui.label(egui::RichText::new("Nenhuma joia catalogada ainda.").color(theme::MUTED));
            } else {
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for g in gems.iter().rev() {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&g.name).font(theme::bold(13.0)).color(Color32::WHITE));
                            ui.label(egui::RichText::new(format!("({} anos)", g.age_revealed)).color(theme::MUTED));
                        });
                        let club_name = g.club.as_deref().unwrap_or("-");
                        ui.label(egui::RichText::new(format!("Clube: {club_name} | Potencial: {}", g.target)).color(theme::MUTED));
                        ui.separator();
                    }
                });
            }
        });

        // Right: Histórico de Campeões
        cols[1].group(|ui| {
            ui.label(egui::RichText::new("Histórico de Campeões Sub-20").font(theme::bold(14.0)).color(GOLD));
            ui.add_space(6.0);
            if history.is_empty() {
                ui.label(egui::RichText::new("Nenhum histórico registrado.").color(theme::MUTED));
            } else {
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for h in &history {
                        ui.label(egui::RichText::new(format!("Ano {}", h.year)).font(theme::bold(13.0)).color(Color32::WHITE));
                        ui.label(egui::RichText::new(format!("Campeão: {} | Vice: {}", h.champion, h.runner_up)).color(GOLD));
                        if let Some((nome, clube, gols)) = &h.top_scorer {
                            ui.label(egui::RichText::new(format!("Artilheiro: {nome} ({clube}, {gols} gols)")).color(theme::MUTED));
                        }
                        if let Some((nome, clube, nota)) = &h.best_player {
                            ui.label(egui::RichText::new(format!("Craque: {nome} ({clube}, {nota:.2})")).color(theme::MUTED));
                        }
                        ui.separator();
                    }
                });
            }
        });
    });
}
