//! The desktop app: the start screen, then a career with its screens.

pub mod career;
pub mod format;
pub mod game_files;
pub mod glyphs;
pub mod motion;
pub mod screens;
pub mod theme;
pub mod view;

use brasfoot_i18n::Translator;
use eframe::egui;

use crate::career::Career;
use crate::game_files::GameFiles;
use crate::screens::career::CareerScreen;
use crate::screens::new_game::NewGameScreen;
use crate::screens::opening::OpeningScreen;
use crate::screens::start::StartScreen;

pub struct App {
    pub t: Translator,
    pub files: GameFiles,
    state: State,
    frames: u32,
    /// `BF_FPS`: since when, frames, CPU seconds.
    fps: (f64, u32, f32),
}

impl App {
    /// The app already in a career.
    pub fn with_career(t: Translator, files: GameFiles, career: Career) -> Self {
        App { t, files, state: State::Career(Box::new(CareerScreen::new(career))), frames: 0, fps: (0.0, 0, 0.0) }
    }

    /// The career on screen, if any.
    pub fn career_screen(&mut self) -> Option<&mut CareerScreen> {
        match &mut self.state {
            State::Career(screen) => Some(screen),
            _ => None,
        }
    }

    /// The start screen, or with `BF_OPEN` a career at once (`BF_TAB` opens a dialog).
    pub fn new(t: Translator, files: GameFiles) -> Self {
        let mut state = State::Opening(OpeningScreen::new());
        // development: BF_SCREEN=saved|new starts on the saved games or new game screen
        match std::env::var("BF_SCREEN").as_deref() {
            Ok("saved") => state = State::SavedGame(StartScreen::new(&files)),
            Ok("new") => state = State::NewGame(NewGameScreen::new(&files)),
            _ => {}
        }
        if let Some(path) = std::env::var_os("BF_OPEN").map(std::path::PathBuf::from) {
            let career = if path.extension().is_some_and(|x| x == "bfn") { Career::open(&path, &files) } else { Career::import(&path, &files) };
            if let Ok(career) = career {
                let mut screen = CareerScreen::new(career);
                match std::env::var("BF_TAB").as_deref() {
                    Ok("tables") => screen.tables_open = true,
                    Ok("calendar") => screen.calendar_open = true,
                    Ok("coaches") => screen.coaches_open = true,
                    Ok("lineup") => screen.lineup_open = true,
                    Ok("club-history") => screen.club_history = Some(screen.club),
                    Ok("loans") => screen.loans = Some(screen.club),
                    Ok("player") => screen.open_player = screen.career.world.club(screen.club).squad.get(5).copied(),
                    Ok("retire") => {
                        if let Some(&p) = screen.career.world.club(screen.club).squad.first() {
                            screen.retire = Some(screens::retire::RetireState::new(&screen.career.world, p));
                        }
                    }
                    Ok("sell") => {
                        if let Some(&p) = screen.career.world.club(screen.club).squad.first() {
                            screen.sell = Some(screens::player_actions::SellState::new(&screen.career.world, p));
                        }
                    }
                    Ok("team-of-round") => screen.team_of_round = Some(screens::team_of_round::TeamOfRoundState::new(&screen.career.world, screen.club, false)),
                    Ok("team-of-year") => screen.team_of_round = Some(screens::team_of_round::TeamOfRoundState::new(&screen.career.world, screen.club, true)),
                    Ok("head-to-head") => screen.open_head_to_head(),
                    Ok("club-ranking") => screen.club_ranking = Some(Default::default()),
                    Ok("champions") => screen.champions = Some(screens::champions::ChampionsState::new(&screen.career.world)),
                    Ok("scorers") => screen.leaders = Some(screens::leaders::LeadersState::new(&screen.career.world, screens::leaders::Board::Scorers, screen.club)),
                    Ok("assists") => screen.leaders = Some(screens::leaders::LeadersState::new(&screen.career.world, screens::leaders::Board::Assists, screen.club)),
                    Ok("messages") => screen.messages_open = true,
                    Ok("teams") => screen.teams_open = true,
                    Ok("preferences") => screen.preferences_open = true,
                    Ok("training") => screen.training_open = true,
                    Ok("stadium") => screen.open_stadium(),
                    Ok(tab) if tab.starts_with("market-") => screen.market = Some(screens::market::MarketState::new(tab["market-".len()..].parse().unwrap_or(0))),
                    Ok(tab) if tab.starts_with("finances-") => screen.finances = Some(screens::finances::FinancesState::new(tab["finances-".len()..].parse().unwrap_or(0))),
                    Ok(tab) if tab.starts_with("management-") => {
                        let mut state = screens::management::ManagementState::new();
                        state.select(tab["management-".len()..].parse().unwrap_or(0));
                        screen.management = Some(state);
                    }
                    Ok(tab) if tab.starts_with("satellites-") => {
                        let mut state = screens::satellites::SatellitesState::new(&mut screen);
                        state.select(tab["satellites-".len()..].parse().unwrap_or(0));
                        screen.satellites = Some(state);
                    }
                    Ok("classic-stadium") => {
                        if let Some(club) = screen.career.human_club() {
                            screen.stadium = Some(screens::stadium::StadiumState::new(&screen.career.world, club));
                        }
                    }
                    Ok(tab) if tab.starts_with("assets-") => {
                        if let Some(club) = screen.career.human_club() {
                            let mut state = screens::assets::AssetsState::new(&mut screen, club);
                            state.select(tab["assets-".len()..].parse().unwrap_or(0));
                            screen.assets = Some(state);
                        }
                    }
                    Ok("round") => {
                        // a 4-4-2 confirmed, then the day's round
                        if let Some(club) = screen.career.human_club()
                            && let Some(m) = view::next_match(&screen.career.world, club)
                        {
                            let mut lineup = brasfoot_simulation::human::lineup::LineupScreen::open(&mut screen.career.world, club, m);
                            lineup.pick_formation(&screen.career.world, 4);
                            if lineup.confirm(&mut screen.career.world).is_ok()
                                && let Ok(round) = screen.career.start_live_round()
                            {
                                let mut state = screens::round::RoundState::new(round);
                                state.window_asked = std::env::var_os("BF_SUBS").is_some();
                                screen.round_state = Some(state);
                                screen.round_open = true;
                            }
                        }
                    }
                    Ok("trophies") => screen.trophies_open = true,
                    Ok("awards") => screen.open_awards_history(),
                    // a.cz for the best player of another club; offer-<thousands>[-send] types (and sends) it
                    Ok(tab) if tab == "offer" || tab.starts_with("offer-") => {
                        let world = &screen.career.world;
                        let other = world.game.clubs.iter().copied().find(|&c| Some(c) != screen.career.human_club());
                        if let Some(p) = other.and_then(|c| world.club(c).squad.iter().copied().max_by_key(|&p| world.player(p).strength)) {
                            let mut state = screens::offer::OfferState::new(world, p);
                            if let Some(rest) = tab.strip_prefix("offer-") {
                                let (amount, send) = rest.strip_suffix("-send").map_or((rest, false), |a| (a, true));
                                state.type_amount(amount, send);
                            }
                            screen.offer_state = Some(state);
                            screen.offer_open = true;
                        }
                    }
                    Ok("contract") => {
                        if let Some(&p) = screen.career.world.club(screen.club).squad.first() {
                            screen.contract_state = Some(screens::contract::ContractState::new(&screen.career.world, p));
                            screen.contract_open = true;
                        }
                    }
                    // awards-<tab>[-<Hall of Fame tab>]
                    Ok(tab) if tab.starts_with("awards-") => {
                        screen.open_awards_history();
                        let mut parts = tab["awards-".len()..].split('-').filter_map(|n| n.parse::<usize>().ok());
                        if let Some(state) = screen.awards.as_mut() {
                            state.select(parts.next().unwrap_or(0), parts.next().unwrap_or(0));
                        }
                    }
                    Ok("coach-trophies") => screen.coach_trophies = screen.career.world.club(screen.club).coach,
                    Ok("search") => {
                        screen.search_state = Some(screens::search::SearchState::searched(&screen.career.world));
                        screen.search_open = true;
                    }
                    _ => {}
                }
                state = State::Career(Box::new(screen));
            }
        }
        App { t, files, state, frames: 0, fps: (0.0, 0, 0.0) }
    }
}

/// The size the screens are laid out for (the toolbar's 1280, the biggest dialogs' height).
const DESIGN: egui::Vec2 = egui::vec2(1240.0, 720.0);

/// A window smaller than `DESIGN` shows everything smaller, in proportion, instead of cutting
/// it; the text is drawn again at the new size, so it stays sharp. `BF_ZOOM` fixes the zoom.
fn fit_to_window(ctx: &egui::Context) {
    if std::env::var_os("BF_ZOOM").is_some() {
        return;
    }
    // the window's size at zoom 1, whatever the zoom now
    let native = ctx.content_rect().size() * ctx.zoom_factor();
    if native.x <= 0.0 || native.y <= 0.0 {
        return;
    }
    let zoom = (native.x / DESIGN.x).min(native.y / DESIGN.y).clamp(0.6, 1.0);
    if (zoom - ctx.zoom_factor()).abs() > 0.01 {
        ctx.set_zoom_factor(zoom);
    }
}

pub enum State {
    Opening(OpeningScreen),
    NewGame(NewGameScreen),
    SavedGame(StartScreen),
    Career(Box<CareerScreen>),
}

/// What a screen asks the app to do.
pub enum Navigation {
    Stay,
    Opening,
    NewGame,
    SavedGame,
    OpenCareer(Career),
    CloseCareer,
    Exit,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.show(ui);
        // development: BF_FPS=1 draws without pause and prints frames per second and CPU time
        if std::env::var_os("BF_FPS").is_some() {
            let now = ui.input(|i| i.time);
            let (start, frames, cpu) = &mut self.fps;
            *frames += 1;
            *cpu += frame.info().cpu_usage.unwrap_or(0.0);
            if now - *start >= 1.0 {
                eprintln!("fps {:.0}  cpu {:.1} ms/frame", *frames as f64 / (now - *start), *cpu * 1000.0 / *frames as f32);
                self.fps = (now, 0, 0.0);
            }
            ui.ctx().request_repaint();
        }
    }
}

impl App {
    /// One frame of the app.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        fit_to_window(ui.ctx());
        let navigation = match &mut self.state {
            State::Opening(screen) => screen.show(ui, &self.t, &self.files),
            State::NewGame(screen) => screen.show(ui, &self.t, &self.files),
            State::SavedGame(screen) => screen.show(ui, &self.t, &self.files),
            State::Career(screen) => screen.show(ui, &self.t, &self.files),
        };
        self.frames += 1;
        // a click changes state after its frame is drawn: draw once more so it shows at once
        if ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released() || !i.keys_down.is_empty()) {
            ui.ctx().request_repaint();
        }
        // development: BF_SCREENSHOT=<png> saves this window after a few frames and quits
        if let Some(path) = std::env::var_os("BF_SCREENSHOT")
            && self.frames == std::env::var("BF_SCREENSHOT_FRAME").ok().and_then(|f| f.parse().ok()).unwrap_or(30)
        {
            let ctx = ui.ctx().clone();
            ui.ctx().request_screenshot(move |image| {
                let [w, h] = image.size;
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buffer) = image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
                    let _ = buffer.save(&path);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
        if self.frames < 40 {
            ui.ctx().request_repaint();
        }
        match navigation {
            Navigation::Stay => {}
            Navigation::Opening => self.state = State::Opening(OpeningScreen::new()),
            Navigation::NewGame => self.state = State::NewGame(NewGameScreen::new(&self.files)),
            Navigation::SavedGame => self.state = State::SavedGame(StartScreen::new(&self.files)),
            Navigation::OpenCareer(career) => self.state = State::Career(Box::new(CareerScreen::new(career))),
            Navigation::CloseCareer => self.state = State::Opening(OpeningScreen::new()),
            Navigation::Exit => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }
}
