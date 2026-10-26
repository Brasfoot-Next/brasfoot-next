//! A career on screen: the main toolbar (`a.eg` as rearranged by `bfmod.BarraPrincipal`) over
//! the main screen, and the game's dialogs as windows above it.

use brasfoot_i18n::{Translator, args};
use brasfoot_model::ids::{ClubId, CompetitionId, PlayerId};
use brasfoot_systems::config::Switch;
use eframe::egui;

use crate::Navigation;
use crate::career::{Career, CareerError};
use crate::game_files::GameFiles;
use crate::screens::{
    assets, awards, calendar, champions, finances, club_history, club_ranking, coaches, contract, head_to_head, history, home, leaders, lineup, loans, management, market, messages, newspaper, offer, player, player_actions, preferences, proposals, retire, round, satellites, search, squad_offer, stadium, standings, team_of_round, teams, training, trophies, widgets, youth,
};
use crate::{format, glyphs, motion, theme, view};

pub struct CareerScreen {
    pub career: Career,
    /// The club on the main screen: the player's, or one picked in `Times`.
    pub club: ClubId,
    pub selected_player: Option<PlayerId>,
    /// The squad table's sort (`eg.zj`'s sorter): the column's key and whether descending.
    pub squad_sort: Option<(&'static str, bool)>,
    pub open_player: Option<PlayerId>,
    pub tables_open: bool,
    pub calendar_open: bool,
    pub teams_open: bool,
    pub lineup_open: bool,
    pub lineup_state: Option<lineup::LineupState>,
    pub round_open: bool,
    pub round_state: Option<round::RoundState>,
    pub contract_open: bool,
    pub contract_state: Option<contract::ContractState>,
    pub offer_open: bool,
    pub offer_state: Option<offer::OfferState>,
    pub youth_open: bool,
    pub youth_state: Option<youth::YouthState>,
    pub history_open: bool,
    pub search_open: bool,
    pub preferences_open: bool,
    pub training_open: bool,
    pub newspaper_open: bool,
    pub newspaper_state: Option<newspaper::NewspaperState>,
    pub assets: Option<assets::AssetsState>,
    pub stadium: Option<stadium::StadiumState>,
    pub satellites: Option<satellites::SatellitesState>,
    pub management: Option<management::ManagementState>,
    pub finances: Option<finances::FinancesState>,
    pub market: Option<market::MarketState>,
    pub messages_open: bool,
    pub leaders: Option<leaders::LeadersState>,
    pub champions: Option<champions::ChampionsState>,
    pub club_ranking: Option<club_ranking::ClubRankingState>,
    pub head_to_head: Option<head_to_head::HeadToHeadState>,
    pub team_of_round: Option<team_of_round::TeamOfRoundState>,
    pub retire: Option<retire::RetireState>,
    pub club_history: Option<ClubId>,
    pub loans: Option<ClubId>,
    pub sell: Option<player_actions::SellState>,
    pub messages_state: messages::MessagesState,
    pub coaches_open: bool,
    pub coach_changes_open: bool,
    pub add_coach_open: bool,
    pub add_coach_state: coaches::AddRemoveCoachState,
    pub coaches_state: Option<coaches::CoachesState>,
    pub search_state: Option<search::SearchState>,
    pub trophies_open: bool,
    pub trophies_club: Option<ClubId>,
    pub coach_trophies: Option<brasfoot_model::ids::CoachId>,
    pub viewed_club: Option<ClubId>,
    pub teams_selected_player: Option<PlayerId>,
    pub teams_player_tab: usize,
    pub teams_search: String,
    pub standings_competition: Option<CompetitionId>,
    pub standings_stage: usize,
    pub standings_rounds: bool,
    pub status: Option<(String, egui::Color32)>,
    pub awards: Option<awards::AwardsState>,
    /// Whether the season was over at the last frame: the awards open when it ends.
    season_over_seen: bool,
    /// The season and day the mod's tick last ran for.
    ticked_at: Option<(i32, i32)>,
    tick_warned: bool,
}

/// The width of the game's window (`a.eg`).
const GAME_WIDTH: f32 = 1280.0;

impl CareerScreen {
    pub fn new(career: Career) -> Self {
        let club = career
            .human_club()
            .or_else(|| {
                let league = view::competitions(&career.world).into_iter().next()?;
                let stage = career.world.competition(league).league?;
                career.world.stage(stage).clubs.first().copied()
            })
            .unwrap_or(ClubId(0));
        CareerScreen {
            club,
            selected_player: None,
            squad_sort: None,
            open_player: None,
            tables_open: false,
            calendar_open: false,
            teams_open: false,
            lineup_open: false,
            lineup_state: None,
            round_open: false,
            round_state: None,
            contract_open: false,
            contract_state: None,
            offer_open: false,
            offer_state: None,
            youth_open: false,
            youth_state: None,
            history_open: false,
            search_open: false,
            preferences_open: false,
            training_open: false,
            newspaper_open: false,
            newspaper_state: None,
            assets: None,
            stadium: None,
            satellites: None,
            management: None,
            finances: None,
            market: None,
            messages_open: false,
            leaders: None,
            champions: None,
            club_ranking: None,
            head_to_head: None,
            team_of_round: None,
            retire: None,
            club_history: None,
            loans: None,
            sell: None,
            messages_state: Default::default(),
            coaches_open: false,
            coach_changes_open: false,
            add_coach_open: false,
            add_coach_state: Default::default(),
            coaches_state: None,
            search_state: None,
            trophies_open: false,
            trophies_club: None,
            coach_trophies: None,
            viewed_club: None,
            teams_selected_player: None,
            teams_player_tab: 0,
            teams_search: String::new(),
            standings_competition: None,
            standings_stage: 0,
            standings_rounds: false,
            status: None,
            awards: None,
            season_over_seen: career.world.game.season_over,
            ticked_at: None,
            tick_warned: false,
            career,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> Navigation {
        let mut navigation = Navigation::Stay;
        self.open_awards_at_season_end();
        self.mod_tick();
        // the game is 1280 wide, centered in a bigger window
        let side = ((ui.available_width() - GAME_WIDTH) / 2.0).max(0.0) as i8;
        let toolbar = egui::Frame::new().fill(theme::HEADER).inner_margin(egui::Margin::symmetric(6, 2)).outer_margin(egui::Margin { left: side, right: side, top: 4, bottom: 0 });
        egui::Panel::top("toolbar").frame(egui::Frame::new().fill(theme::OUTER)).show(ui, |ui| {
            toolbar.show(ui, |ui| navigation = self.toolbar(ui, t, files));
        });
        if let Some((text, color)) = self.status.clone() {
            egui::Panel::bottom("status").frame(egui::Frame::new().fill(theme::HEADER).inner_margin(6)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(color, text);
                    if widgets::close_button(ui) {
                        self.status = None;
                    }
                });
            });
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::OUTER).inner_margin(egui::Margin { left: side, right: side, top: 0, bottom: 8 })).show(ui, |ui| {
            home::show(self, ui, t, files);
        });
        let ctx = ui.ctx().clone();
        // the dialogs are drawn while they fade out too
        let mut open = self.tables_open;
        standings::window(self, &ctx, t, files, &mut open);
        self.tables_open = open;
        let mut open = self.calendar_open;
        calendar::window(self, &ctx, t, files, &mut open);
        self.calendar_open = open;
        let mut open = self.teams_open;
        teams::window(self, &ctx, t, files, &mut open);
        self.teams_open = open;
        let mut open = self.lineup_open;
        if open {
            let mut state = self.lineup_state.take();
            lineup::window(self, &mut state, &ctx, t, files, &mut open);
            self.lineup_state = state;
            self.lineup_open = open;
        }
        let mut open = self.round_open;
        if open {
            let mut state = self.round_state.take();
            round::window(self, &mut state, &ctx, t, files, &mut open);
            self.round_state = state;
            self.round_open = open;
        }
        let mut open = self.contract_open;
        if open {
            let mut state = self.contract_state.take();
            contract::window(self, &mut state, &ctx, t, files, &mut open);
            self.contract_state = state;
            self.contract_open = open;
        }
        let mut open = self.offer_open;
        if open {
            let mut state = self.offer_state.take();
            offer::window(self, &mut state, &ctx, t, files, &mut open);
            self.offer_state = state;
            self.offer_open = open;
        }
        squad_offer::window(self, &ctx, t, files);
        let mut open = self.youth_open;
        if open {
            let mut state = self.youth_state.take();
            youth::window(self, &mut state, &ctx, t, files, &mut open);
            self.youth_state = state;
            self.youth_open = open;
        }
        let mut open = self.history_open;
        if open {
            history::window(self, &ctx, t, files, &mut open);
            self.history_open = open;
        }
        let mut open = self.search_open;
        if open {
            let mut state = self.search_state.take();
            search::window(self, &mut state, &ctx, t, files, &mut open);
            self.search_state = state;
            self.search_open = open;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) && !self.round_open {
            self.preferences_open = true;
        }
        if let Some(club) = self.club_history {
            let mut open = true;
            club_history::window(self, club, &ctx, t, files, &mut open);
            if !open {
                self.club_history = None;
            }
        }
        if let Some(club) = self.loans {
            let mut open = true;
            loans::window(self, club, &ctx, t, files, &mut open);
            if !open {
                self.loans = None;
            }
        }
        if let Some(mut state) = self.retire.take() {
            let mut open = true;
            retire::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.retire = Some(state);
            }
        }
        if let Some(mut state) = self.sell.take() {
            let mut open = true;
            player_actions::sell_window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.sell = Some(state);
            }
        }
        if let Some(mut state) = self.team_of_round.take() {
            let mut open = true;
            team_of_round::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.team_of_round = Some(state);
            }
        }
        if let Some(mut state) = self.head_to_head.take() {
            let mut open = true;
            head_to_head::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.head_to_head = Some(state);
            }
        }
        if let Some(mut state) = self.club_ranking.take() {
            let mut open = true;
            club_ranking::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.club_ranking = Some(state);
            }
        }
        if let Some(mut state) = self.champions.take() {
            let mut open = true;
            champions::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.champions = Some(state);
            }
        }
        if let Some(mut state) = self.leaders.take() {
            let mut open = true;
            leaders::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.leaders = Some(state);
            }
        }
        let mut open = self.messages_open;
        if open {
            let mut state = std::mem::take(&mut self.messages_state);
            messages::window(self, &mut state, &ctx, t, files, &mut open);
            self.messages_state = state;
            self.messages_open = open;
        }
        let mut open = self.training_open;
        if open && let Some(club) = self.career.human_club() {
            training::window(self, club, &ctx, t, files, &mut open);
            self.training_open = open;
        }
        if let Some(club) = self.career.human_club() {
            if let Some(mut state) = self.assets.take() {
                let mut open = true;
                assets::window(self, &mut state, club, &ctx, t, files, &mut open);
                if state.open_classic {
                    self.stadium = Some(stadium::StadiumState::new(&self.career.world, club));
                } else if state.open_youth {
                    self.youth_state = Some(youth::YouthState::new(club));
                    self.youth_open = true;
                } else if open {
                    self.assets = Some(state);
                }
            }
            if let Some(mut state) = self.stadium.take() {
                let mut open = true;
                stadium::window(self, &mut state, club, &ctx, t, files, &mut open);
                if open {
                    self.stadium = Some(state);
                }
            }
        }
        if let Some(mut state) = self.satellites.take() {
            let mut open = true;
            satellites::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.satellites = Some(state);
            }
        }
        if let Some(mut state) = self.management.take() {
            let mut open = true;
            management::window(self, &mut state, &ctx, t, files, &mut open);
            if state.open_sponsors {
                // CaixaDialog.mostrarPatrocinios
                state.open_sponsors = false;
                self.finances = Some(finances::FinancesState::new(3));
            }
            if open {
                self.management = Some(state);
            }
        }
        if let Some(mut state) = self.finances.take() {
            let mut open = true;
            finances::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.finances = Some(state);
            }
        }
        if let Some(mut state) = self.market.take() {
            let mut open = true;
            market::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.market = Some(state);
            }
        }
        proposals::master_window(self, &ctx, t, files);
        proposals::investor_window(self, &ctx, t, files);
        let mut open = self.newspaper_open;
        if open {
            let mut state = self.newspaper_state.take().unwrap_or_default();
            newspaper::window(self, &mut state, &ctx, t, files, &mut open);
            self.newspaper_state = Some(state);
            self.newspaper_open = open;
        }
        let mut open = self.preferences_open;
        if open {
            preferences::window(self, &ctx, t, files, &mut open);
            self.preferences_open = open;
        }
        let mut open = self.coaches_open;
        if open {
            let mut state = self.coaches_state.take();
            coaches::window(self, &mut state, &ctx, t, files, &mut open);
            self.coaches_state = state;
            self.coaches_open = open;
        }
        let mut open = self.coach_changes_open;
        if open {
            coaches::changes_window(self, &ctx, t, files, &mut open);
            self.coach_changes_open = open;
        }
        let mut open = self.add_coach_open;
        if open {
            let mut state = std::mem::take(&mut self.add_coach_state);
            coaches::add_remove_window(self, &mut state, &ctx, t, files, &mut open);
            self.add_coach_state = state;
            self.add_coach_open = open;
        }
        let mut open = self.trophies_open;
        if open {
            let club = self.trophies_club.unwrap_or(self.club);
            trophies::window(self, &ctx, t, files, club, &mut open);
            self.trophies_open = open;
        }
        if let Some(coach) = self.coach_trophies {
            let mut open = true;
            trophies::coach_window(self, &ctx, t, files, coach, &mut open);
            if !open {
                self.coach_trophies = None;
            }
        }
        if let Some(p) = self.open_player {
            let mut open = true;
            player::window(self, &ctx, t, files, p, &mut open);
            if !open {
                self.open_player = None;
            }
        }
        if let Some(mut state) = self.awards.take() {
            let mut open = true;
            awards::window(self, &mut state, &ctx, t, files, &mut open);
            if open {
                self.awards = Some(state);
            }
        }
        navigation
    }

    /// The patched season-end screen `a.cR` hands the season to the mod, which shows its awards
    /// (`BolaDeOuro.fimDeTemporada`, when someone played enough to win the Ballon d'Or).
    fn open_awards_at_season_end(&mut self) {
        let over = self.career.world.game.season_over;
        if over && !self.season_over_seen {
            let season = brasfoot_systems::awards::season::Season::current(&self.career.world);
            if !season.ballon_dor(1).is_empty() {
                self.awards = Some(awards::AwardsState::new(&self.career.world, season, true, awards::Tab::BallonDor));
            }
        }
        self.season_over_seen = over;
    }

    /// `components.m` calls `BolaDeOuro.tick` each time the main screen is built: here once per
    /// day shown. At a season's end the game shows its season-end screen instead.
    fn mod_tick(&mut self) {
        let world = &self.career.world;
        let today = (world.season(), world.game.day);
        if world.game.season_over || self.ticked_at == Some(today) {
            return;
        }
        self.ticked_at = Some(today);
        // what is not ported yet is said once, not every day
        if let Err(CareerError::NotPortedYet(what)) = self.career.mod_tick()
            && !self.tick_warned
        {
            self.tick_warned = true;
            eprintln!("mod tick not ported yet: {what}");
        }
    }

    /// The toolbar's "Estádio": the mod's window with `PATRIMONIO` on, else the game's.
    pub fn open_stadium(&mut self) {
        let Some(club) = self.career.human_club() else { return };
        if !assets::available(&self.career.world, Some(club)) {
            return;
        }
        if self.career.systems.config.on(Switch::Assets) {
            self.assets = Some(assets::AssetsState::new(self, club));
        } else {
            self.stadium = Some(stadium::StadiumState::new(&self.career.world, club));
        }
    }

    /// `BolaDeOuro.mostrarHistorico`: the awards window from the toolbar, on the winners.
    pub fn open_awards_history(&mut self) {
        let season = self.career.systems.open_awards(&mut self.career.world);
        self.awards = Some(awards::AwardsState::new(&self.career.world, season, false, awards::Tab::Winners));
    }

    /// `eg.rh()`: the head-to-head of the club and its next opponent.
    pub fn open_head_to_head(&mut self) {
        let world = &self.career.world;
        let rival = view::next_match(world, self.club).and_then(|m| {
            let g = world.game_match(m);
            if g.home == Some(self.club) { g.away } else { g.home }
        });
        self.head_to_head = Some(head_to_head::HeadToHeadState::new(self.club, rival));
    }

    /// The toolbar in the mod's order: the club, the football world, the newspaper, the game.
    fn toolbar(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> Navigation {
        let mut navigation = Navigation::Stay;
        let config = self.career.systems.config.clone();
        let has_club = self.career.human_club().is_some();
        ui.horizontal(|ui| {
            // the buttons sit side by side, 64 apart, closer on a narrow window so all fit
            ui.spacing_mut().item_spacing.x = 0.0;
            let width = ((ui.available_width() - 140.0 - 4.0 * 17.0) / 18.0).clamp(44.0, 64.0);
            ui.data_mut(|d| d.insert_temp(egui::Id::unique(TOOL_WIDTH), width));
            ui.add_space(6.0);
            let not_yet = t.text("toolbar-not-ported", None);
            // group 1: the club
            if tool_button(ui, t, files, "toolbar-agenda", glyphs::CALENDAR, true, "").clicked() {
                self.calendar_open = true;
            }
            // eg: the training button only with individual skills and a club
            if training::available(&self.career.world, self.career.human_club()) && tool_button(ui, t, files, "toolbar-training", glyphs::TRAINING, true, "").clicked() {
                self.training_open = true;
            }
            let youth = if config.on(Switch::Youth) { "toolbar-academy" } else { "toolbar-youth" };
            if tool_button(ui, t, files, youth, glyphs::YOUTH, true, "").clicked() {
                self.youth_state = Some(youth::YouthState::new(self.club));
                self.youth_open = true;
            }
            // eg.ra(), or PatrimonioDialog.mostrar with PATRIMONIO on; only with a stadium
            let stadium_club = self.career.human_club().filter(|_| assets::available(&self.career.world, self.career.human_club()));
            if tool_button(ui, t, files, "toolbar-stadium", glyphs::STADIUM, stadium_club.is_some(), &not_yet).clicked() {
                self.open_stadium();
            }
            if config.on(Switch::Satellites) {
                if tool_button(ui, t, files, "toolbar-satellites", glyphs::SATELLITES, true, "").clicked() {
                    self.satellites = Some(satellites::SatellitesState::new(self));
                }
            }
            if tool_button(ui, t, files, "toolbar-management", glyphs::MANAGEMENT, true, "").clicked() {
                self.management = Some(management::ManagementState::new());
            }
            if has_club {
                if tool_button(ui, t, files, "toolbar-finances", glyphs::FINANCES, true, "").clicked() {
                    self.finances = Some(finances::FinancesState::new(0));
                }
            }
            divider(ui);
            // group 2: the football world
            if tool_button(ui, t, files, "toolbar-tables", glyphs::TABLES, true, "").clicked() {
                self.tables_open = true;
            }
            if tool_button(ui, t, files, "toolbar-teams", glyphs::TEAMS, true, "").clicked() {
                self.teams_open = true;
            }
            if tool_button(ui, t, files, "toolbar-search", glyphs::SEARCH, true, "").clicked() {
                self.search_open = true;
            }
            // eg.Hz: the coaches' menu
            let coaches_button = tool_button(ui, t, files, "toolbar-coaches", glyphs::COACHES, true, "");
            egui::Popup::menu(&coaches_button).show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if widgets::menu_entry(ui, &t.text("coaches-menu-ranking", None), true, "") {
                    self.coaches_state = None;
                    self.coaches_open = true;
                    ui.close();
                }
                widgets::menu_separator(ui);
                if widgets::menu_entry(ui, &t.text("coaches-menu-add-remove", None), true, "") {
                    self.add_coach_open = true;
                    ui.close();
                }
                widgets::menu_separator(ui);
                if widgets::menu_entry(ui, &t.text("coaches-menu-latest-changes", None), true, "") {
                    self.coach_changes_open = true;
                    ui.close();
                }
                widgets::menu_separator(ui);
                // eg.rx(): the trophies of the club's coach
                let coach = self.career.world.club(self.club).coach;
                if widgets::menu_entry(ui, &t.text("coaches-menu-trophies", None), coach.is_some(), &not_yet) {
                    self.coach_trophies = coach;
                    ui.close();
                }
            });
            if tool_button(ui, t, files, "toolbar-market", glyphs::MARKET, true, "").clicked() {
                self.market = Some(market::MarketState::new(0));
            }
            // Prêmios is the mod's awards and Hall of Fame (BolaDeOuro.mostrarHistorico), not the
            // club's trophies, which open from the main screen
            if tool_button(ui, t, files, "toolbar-awards", glyphs::AWARDS, true, "").clicked() {
                self.open_awards_history();
            }
            divider(ui);
            if tool_button(ui, t, files, "toolbar-newspaper", glyphs::NEWSPAPER, true, "").clicked() {
                self.newspaper_open = true;
            }
            divider(ui);
            // eg.HA: the history menu, a separator between every entry
            let history_button = tool_button(ui, t, files, "toolbar-history", glyphs::HISTORY, true, "");
            egui::Popup::menu(&history_button).show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let world = &self.career.world;
                let mut entries: Vec<(&str, Option<HistoryEntry>)> = vec![
                    ("history-menu-champions", Some(HistoryEntry::Champions)),
                    ("history-menu-club-ranking", Some(HistoryEntry::ClubRanking)),
                ];
                if world.game.plays_all_national_teams {
                    entries.push(("history-menu-national-ranking", Some(HistoryEntry::NationalRanking)));
                }
                entries.push(("history-menu-top-scorers", Some(HistoryEntry::Scorers)));
                entries.push(("history-menu-assists", Some(HistoryEntry::Assists)));
                if has_club {
                    entries.push(("history-menu-trophies", Some(HistoryEntry::Trophies)));
                }
                entries.push(("history-menu-team-of-round", Some(HistoryEntry::TeamOfRound)));
                entries.push(("history-menu-team-of-year", Some(HistoryEntry::TeamOfYear)));
                entries.push(("history-menu-golden-ball", Some(HistoryEntry::GoldenBall)));
                entries.push(("history-menu-transfers", Some(HistoryEntry::Transfers)));
                entries.push(("history-menu-head-to-head", Some(HistoryEntry::HeadToHead)));
                for (i, (key, entry)) in entries.into_iter().enumerate() {
                    if i > 0 {
                        widgets::menu_separator(ui);
                    }
                    if widgets::menu_entry(ui, &t.text(key, None), entry.is_some(), &not_yet) {
                        match entry {
                            Some(HistoryEntry::Trophies) => {
                                self.trophies_club = self.career.human_club();
                                self.trophies_open = true;
                            }
                            Some(HistoryEntry::Transfers) => self.history_open = true,
                            Some(HistoryEntry::HeadToHead) => self.open_head_to_head(),
                            Some(HistoryEntry::TeamOfRound) => self.team_of_round = Some(team_of_round::TeamOfRoundState::new(&self.career.world, self.club, false)),
                            Some(HistoryEntry::TeamOfYear) => self.team_of_round = Some(team_of_round::TeamOfRoundState::new(&self.career.world, self.club, true)),
                            Some(HistoryEntry::ClubRanking) => self.club_ranking = Some(Default::default()),
                            Some(HistoryEntry::NationalRanking) => self.club_ranking = Some(club_ranking::ClubRankingState { national: true, ..Default::default() }),
                            Some(HistoryEntry::Champions) => self.champions = Some(champions::ChampionsState::new(&self.career.world)),
                            Some(HistoryEntry::GoldenBall) => self.open_awards_history(),
                            Some(HistoryEntry::Scorers) => self.leaders = Some(leaders::LeadersState::new(&self.career.world, leaders::Board::Scorers, self.club)),
                            Some(HistoryEntry::Assists) => self.leaders = Some(leaders::LeadersState::new(&self.career.world, leaders::Board::Assists, self.club)),
                            None => {}
                        }
                        ui.close();
                    }
                }
            });
            let options = tool_button(ui, t, files, "toolbar-options", glyphs::OPTIONS, true, "");
            egui::Popup::menu(&options).show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                // eg.Hy: Preferências (F3)
                if widgets::menu_entry(ui, &t.text("options-preferences", None), true, "") {
                    self.preferences_open = true;
                    ui.close();
                }
                widgets::menu_separator(ui);
                if widgets::menu_entry(ui, &t.text("options-save", None), true, "") {
                    self.status = Some(match self.career.save(None) {
                        Ok(()) => (t.text("status-saved", None), theme::GOOD),
                        Err(e) => (t.text("status-save-failed", Some(&args([("reason", e)]))), theme::BAD),
                    });
                    ui.close();
                }
                widgets::menu_separator(ui);
                if widgets::menu_entry(ui, &t.text("options-close-career", None), true, "") {
                    navigation = Navigation::CloseCareer;
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(14.0);
                let today = format::date(t, &self.career.runtime.clock, self.career.world.today_millis());
                ui.label(egui::RichText::new(today).font(theme::bold(13.0)).color(egui::Color32::WHITE));
            });
        });
        navigation
    }



    /// The day's button: `Escalar Time` for the player's club, or a day of the AI's world.
    pub fn play_button(&mut self, ui: &mut egui::Ui, t: &Translator, _files: &GameFiles) {
        let human = self.career.human_club().is_some();
        let key = if human { "home-pick-team" } else { "toolbar-advance" };
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::click());
        let enabled = self.career.can_advance() || human;
        let hover = motion::toward(ui.ctx(), response.id, response.hovered(), motion::HOVER);
        let fill = motion::mix(theme::BUTTON, theme::PANEL_HIGH, hover);
        let stroke = motion::mix(theme::BORDER, theme::ACCENT, hover);
        ui.painter().rect(rect, 4.0, fill, egui::Stroke::new(1.0, stroke), egui::StrokeKind::Inside);
        if response.hovered() {
            ui.ctx().set_cursor_icon(if enabled { egui::CursorIcon::PointingHand } else { egui::CursorIcon::NotAllowed });
        }
        let text = t.text(key, None);
        let galley = ui.painter().layout_no_wrap(text, theme::regular(12.0), theme::TEXT);
        let icon = 26.0;
        let total = icon + 6.0 + galley.size().x;
        let start = rect.center().x - total / 2.0;
        widgets::paint_icon(ui.painter(), egui::Rect::from_center_size(egui::pos2(start + icon / 2.0, rect.center().y), egui::vec2(icon, icon) * 0.8), glyphs::LINEUP, theme::ACCENT);
        ui.painter().galley(egui::pos2(start + icon + 6.0, rect.center().y - galley.size().y / 2.0), galley, theme::TEXT);
        let response = if enabled { response } else { response.on_hover_text(t.text("advance-human-not-ported", None)) };
        if enabled && response.clicked() {
            if human {
                self.lineup_open = true;
            } else {
                self.status = match self.career.advance() {
                    Ok(()) => None,
                    Err(CareerError::NotPortedYet(what)) => Some((t.text("status-not-ported", Some(&args([("what", what)]))), theme::WARN)),
                    Err(CareerError::Read(e)) => Some((e, theme::BAD)),
                };
            }
        }
    }
}

/// A 64x60 toolbar button: the 35-pixel icon above its text (`BarraPrincipal.padronizar`).
/// Where the toolbar keeps its buttons' width for the frame.
const TOOL_WIDTH: &str = "tool-button-width";

fn tool_button(ui: &mut egui::Ui, t: &Translator, _files: &GameFiles, key: &str, glyph: &str, enabled: bool, not_ported: &str) -> egui::Response {
    let width = ui.data(|d| d.get_temp::<f32>(egui::Id::unique(TOOL_WIDTH))).unwrap_or(64.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 60.0), if enabled { egui::Sense::click() } else { egui::Sense::hover() });
    // the modern touch: the button lights up and its icon rises a little under the mouse
    let hover = motion::toward(ui.ctx(), response.id, response.hovered(), motion::HOVER);
    if hover > 0.0 {
        ui.painter().rect_filled(rect, 6.0, theme::PANEL_HIGH.gamma_multiply(hover));
    }
    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let pressed = if enabled && response.is_pointer_button_down_on() { 0.94 } else { 1.0 };
    let icon_size = (width * 0.55).min(35.0);
    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 22.0 - 2.0 * hover), egui::vec2(icon_size, icon_size) * pressed);
    // the icon (Phosphor) in the text color, the accent under the mouse; the screens not ported
    // yet are dimmed and their tooltip says so
    let tint = if enabled { motion::mix(theme::TEXT, theme::ACCENT, hover) } else { theme::MUTED.gamma_multiply(0.7) };
    widgets::paint_icon(ui.painter(), icon_rect.shrink(icon_size * 0.08), glyph, tint);
    let color = if enabled { theme::TEXT } else { theme::MUTED };
    let size = if width < 58.0 { 10.0 } else { 11.0 };
    ui.painter().with_clip_rect(rect).text(egui::pos2(rect.center().x, rect.bottom() - 9.0), egui::Align2::CENTER_CENTER, t.text(key, None), theme::regular(size), color);
    if enabled { response } else { response.on_hover_text(not_ported) }
}

/// The entries of the history menu that open a screen already ported.
enum HistoryEntry {
    Trophies,
    Transfers,
    Scorers,
    Assists,
    Champions,
    ClubRanking,
    HeadToHead,
    TeamOfRound,
    TeamOfYear,
    NationalRanking,
    GoldenBall,
}

/// The 17x44 divider between the groups.
fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(17.0, 44.0), egui::Sense::hover());
    ui.painter().line_segment([rect.center_top(), rect.center_bottom()], egui::Stroke::new(1.0, theme::BORDER));
}
