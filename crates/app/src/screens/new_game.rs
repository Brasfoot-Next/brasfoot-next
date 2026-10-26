//! The new game: "Configurações iniciais" (`a.hU`), then "Adicionar técnico humano" (`a.bV`).
//! The settings screen reads the teams of the data packs (`p.dd()` on the Patches screen's
//! database instead of the game folder's team files); "Iniciar Jogo" makes the world
//! (`p.dg()`); the coach screen hires the human coaches and starts the career (`bV.AB()`).
//! The parts of the original's new game not ported yet stay on screen, disabled.

use std::sync::{Arc, mpsc};

use brasfoot_i18n::{Translator, args};
use brasfoot_model::countries;
use brasfoot_model::ids::{ClubId, CountryId};
use brasfoot_model::options::GameOptions;
use brasfoot_simulation::new_game::{NewGame, STATES, coach_screen_divisions};
use eframe::egui::{self, Color32, RichText};

use crate::Navigation;
use crate::career::{Career, CareerError};
use crate::game_files::GameFiles;
use crate::patches::Loaded;
use crate::screens::menu;
use crate::theme;
use crate::screens::controls;

/// What the screen is doing.
enum Step {
    /// Waiting for the data sources to be read.
    Waiting,
    /// No data source downloaded.
    NoData,
    /// Reading the team files.
    Loading(mpsc::Receiver<(NewGame, GameOptions)>),
    Settings(Settings),
    /// Making the world.
    Creating(mpsc::Receiver<Result<Career, CareerError>>, Settings),
    Coach(Box<CoachStep>),
}

/// The choices of `a.hU`.
struct Settings {
    new_game: NewGame,
    options: GameOptions,
    /// `afA`: 0 "2022", 1 "2023".
    year: usize,
    /// The league row selected (`JG`).
    row: Option<usize>,
    error: Option<String>,
}

/// The state of `a.bV`.
struct CoachStep {
    career: Career,
    /// `ue`: the country shown, an index of the league countries.
    country: usize,
    /// `Bu`: the clubs of each division without a human coach.
    divisions: Vec<Vec<ClubId>>,
    /// `zu` and the division it was picked from.
    chosen: Option<(usize, ClubId)>,
    /// `By` and `Bt`
    random: Option<ClubId>,
    random_on: bool,
    /// `uf`
    name: String,
    /// `BD`: a country id.
    nationality: i32,
    /// `count`: coaches added with "Adicionar e incluir outro".
    added: usize,
    error: Option<String>,
}

pub struct NewGameScreen {
    step: Step,
}

/// A country's name from the catalog.
pub fn country_name(t: &Translator, id: i32) -> String {
    let code = countries::country(id).map(|c| c.code.to_lowercase()).unwrap_or_default();
    t.text(&format!("country-{code}"), None)
}

fn flag(ui: &mut egui::Ui, files: &GameFiles, id: i32, size: f32) {
    if let Some(uri) = files.jar_image(ui.ctx(), &format!("aflags/{id}.png")) {
        ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(size, size)));
    } else {
        ui.add_space(size);
    }
}

/// A crest by team file, or a blank place of its size.
fn crest_of(ui: &mut egui::Ui, files: &GameFiles, file_ref: &str, size: f32) {
    if let Some(uri) = files.crest(ui.ctx(), file_ref) {
        ui.add(egui::Image::new(uri).fit_to_exact_size(egui::vec2(size, size)));
    } else {
        ui.add_space(size);
    }
}

/// A labelled check box; disabled ones explain that the part is not ported yet.
fn labelled_check(ui: &mut egui::Ui, t: &Translator, text: &str, value: &mut bool, enabled: bool) {
    ui.horizontal(|ui| {
        let r = menu::check(ui, *value, enabled);
        ui.add_space(4.0);
        let label = ui.add(egui::Label::new(RichText::new(text).size(13.0).color(if enabled { theme::TEXT } else { theme::MUTED })).sense(egui::Sense::click()));
        if enabled && (r.clicked() || label.clicked()) {
            *value = !*value;
        }
        if !enabled {
            let tip = t.text("initial-settings-not-ported", None);
            r.on_hover_text(&tip);
            label.on_hover_text(tip);
        }
    });
}

impl NewGameScreen {
    /// Waits for the data sources (the Patches screen) to be read, then makes the team files of
    /// their database.
    pub fn new(_files: &GameFiles) -> Self {
        NewGameScreen { step: Step::Waiting }
    }

    /// `p.dd()` on the database's clubs and league configurations, with the game folder's options.
    fn load(loaded: Arc<Loaded>, dir: std::path::PathBuf) -> mpsc::Receiver<(NewGame, GameOptions)> {
        let (tx, rx) = mpsc::channel();
        let load = move || {
            let (teams, configs, _) = brasfoot_datapack::database::to_templates(&loaded.database.content);
            let options = brasfoot_persistence::game_data::read_options(&dir.join("options.bcf"));
            let mut new_game = NewGame::new(teams, configs);
            // a.hU: Brazil is the league chosen at first
            for l in &mut new_game.leagues {
                l.selected = l.country == 29;
            }
            if !new_game.leagues.iter().any(|l| l.selected) && !new_game.leagues.is_empty() {
                new_game.leagues[0].selected = true;
            }
            (new_game, options)
        };

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let _ = tx.send(load());
        });

        #[cfg(target_arch = "wasm32")]
        {
            let _ = tx.send(load());
        }

        rx
    }

    pub fn show(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> Navigation {
        let mut navigation = Navigation::Stay;
        // what the background work finished
        let step = std::mem::replace(&mut self.step, Step::Loading(mpsc::channel().1));
        self.step = match step {
            Step::Waiting if files.patches.ready() => match files.patches.loaded() {
                Some(loaded) => Step::Loading(Self::load(loaded, files.dir.clone())),
                None => Step::NoData,
            },
            Step::Loading(rx) => match rx.try_recv() {
                Ok((new_game, options)) => {
                    let row = new_game.leagues.iter().position(|l| l.selected);
                    Step::Settings(Settings { new_game, options, year: 0, row, error: None })
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ui.ctx().request_repaint();
                    Step::Loading(rx)
                }
                Err(mpsc::TryRecvError::Disconnected) => Step::NoData,
            },
            Step::Creating(rx, settings) => match rx.try_recv() {
                Ok(Ok(career)) => Step::Coach(Box::new(CoachStep::new(career))),
                Ok(Err(CareerError::NotPortedYet(what) | CareerError::Read(what))) => {
                    Step::Settings(Settings { error: Some(t.text("status-not-ported", Some(&args([("what", what)])))), ..settings })
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ui.ctx().request_repaint();
                    Step::Creating(rx, settings)
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    Step::Settings(Settings { error: Some("Erro ao criar carreira".to_string()), ..settings })
                }
            },
            other => other,
        };
        let width = if matches!(self.step, Step::Coach(_)) { 560.0 } else { 1000.0 };
        menu::page(ui, width, |ui| {
            let mut next = None;
            match &mut self.step {
                Step::Waiting | Step::Loading(_) => {
                    waiting(ui, &t.text("initial-settings-loading", None));
                }
                Step::NoData => match no_data(ui, t) {
                    SettingsAction::Back => navigation = Navigation::Opening,
                    SettingsAction::Start => navigation = Navigation::Patches,
                    SettingsAction::Stay => {}
                },
                Step::Creating(..) => {
                    waiting(ui, &t.text("add-human-coach-creating", None));
                }
                Step::Settings(settings) => match settings.show(ui, t, files) {
                    SettingsAction::Stay => {}
                    SettingsAction::Back => navigation = Navigation::Opening,
                    SettingsAction::Start => next = Some(settings.start(files)),
                },
                Step::Coach(coach) => {
                    if let Some(career) = coach.show(ui, t, files) {
                        navigation = Navigation::OpenCareer(career);
                    }
                }
            }
            if let Some(rx) = next
                && let Step::Settings(settings) = std::mem::replace(&mut self.step, Step::Loading(mpsc::channel().1))
            {
                self.step = Step::Creating(rx, settings);
            }
        });
        if matches!(self.step, Step::Waiting | Step::Loading(_) | Step::Creating(..)) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
        navigation
    }
}

/// No data source downloaded: back, or Start for the Patches screen.
fn no_data(ui: &mut egui::Ui, t: &Translator) -> SettingsAction {
    let mut action = SettingsAction::Stay;
    if menu::header(ui, t, &t.text("new-game-no-data", None), &t.text("new-game-no-data-detail", None), true) {
        action = SettingsAction::Back;
    }
    if controls::button(ui, &t.text("new-game-open-patches", None), controls::Kind::Primary, true).clicked() {
        action = SettingsAction::Start;
    }
    action
}

fn waiting(ui: &mut egui::Ui, text: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() / 2.0 - 30.0).max(120.0));
        ui.add(egui::Spinner::new().size(28.0).color(theme::ACCENT));
        ui.add_space(8.0);
        ui.label(RichText::new(text).size(14.0).color(theme::MUTED));
    });
}

enum SettingsAction {
    Stay,
    Back,
    Start,
}

impl Settings {
    /// `bW.oq()` without its message: the leagues chosen and the world made in the background.
    fn start(&mut self, files: &GameFiles) -> mpsc::Receiver<Result<Career, CareerError>> {
        let mut options = self.options.clone();
        // the parts of the new game not ported yet stay off
        options.plays_state = false;
        options.plays_regionals = false;
        options.plays_international_clubs = false;
        options.plays_all_national_teams = false;
        let year = if self.year == 1 { 2023 } else { 2022 };
        let new_game = self.new_game.clone();
        let files = GameFiles::locate_at(&files.dir);
        let (tx, rx) = mpsc::channel();
        let create = move || Career::create(&new_game, &options, year, &files);

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let _ = tx.send(create());
        });

        #[cfg(target_arch = "wasm32")]
        {
            let _ = tx.send(create());
        }

        rx
    }

    fn show(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> SettingsAction {
        let mut action = SettingsAction::Stay;
        if menu::header(ui, t, &t.text("initial-settings-title", None), &t.text("initial-settings-subtitle", None), true) {
            action = SettingsAction::Back;
        }
        // two columns on a wide window, one under the other on a narrow one
        let wide = ui.available_width() >= 760.0;
        if wide {
            ui.columns(2, |cols| {
                self.left_column(&mut cols[0], t, files);
                self.right_column(&mut cols[1], t);
            });
        } else {
            self.left_column(ui, t, files);
            ui.add_space(12.0);
            self.right_column(ui, t);
        }
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if let Some(e) = &self.error {
                ui.label(RichText::new(e).color(theme::TEXT_RED).size(13.0));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if controls::button(ui, &t.text("initial-settings-button-start", None), controls::Kind::Primary, true).clicked() {
                    if self.new_game.leagues.iter().any(|l| l.selected) {
                        action = SettingsAction::Start;
                    } else {
                        self.error = Some(t.text("initial-settings-error-no-league", None));
                    }
                }
            });
        });
        ui.add_space(32.0);
        action
    }

    fn left_column(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) {
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.leagues(ui, t, files);
        });
    }

    /// The state leagues, the international competitions and the career's rules.
    fn right_column(&mut self, ui: &mut egui::Ui, t: &Translator) {
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            self.states(ui, t);
        });
        ui.add_space(12.0);
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            menu::panel_title(ui, &t.text("initial-settings-section-international", None), |ui| {
                controls::button(ui, &t.text("initial-settings-button-preferences", None), controls::Kind::Secondary, false).on_disabled_hover_text(t.text("initial-settings-not-ported", None));
            });
            let mut off = false;
            labelled_check(ui, t, &t.text("initial-settings-checkbox-club-cups", None), &mut off, false);
            ui.add_space(6.0);
            labelled_check(ui, t, &t.text("initial-settings-checkbox-national-teams", None), &mut off, false);
            ui.add_space(6.0);
            let mut groups = self.options.use_real_cup_groups;
            labelled_check(ui, t, &t.text("initial-settings-checkbox-real-groups", None), &mut groups, false);
        });
        ui.add_space(12.0);
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            menu::panel_title(ui, &t.text("initial-settings-section-rules", None), |_| {});
            menu::field_label(ui, &t.text("initial-settings-section-salary", None));
            let salary = [t.text("initial-settings-radio-monthly", None), t.text("initial-settings-radio-weekly", None)];
            if let Some(i) = controls::segmented(ui, &salary, if self.options.monthly_salary { 0 } else { 1 }) {
                self.options.monthly_salary = i == 0;
            }
            ui.add_space(10.0);
            menu::field_label(ui, &t.text("initial-settings-section-strength", None));
            let strength = [t.text("initial-settings-radio-individual", None), t.text("initial-settings-radio-classic", None)];
            if let Some(i) = controls::segmented(ui, &strength, if self.options.individual_skills { 0 } else { 1 }) {
                self.options.individual_skills = i == 0;
            }
            ui.add_space(10.0);
            menu::field_label(ui, &t.text("initial-settings-section-start-season", None));
            let years = [t.text("initial-settings-combo-2022", None), t.text("initial-settings-combo-2023", None)];
            if let Some(i) = controls::segmented(ui, &years, self.year) {
                self.year = i;
            }
        });
    }

    /// The national leagues (`JG`): play, country, teams.
    fn leagues(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) {
        let mut all = false;
        menu::panel_title(ui, &t.text("initial-settings-section-leagues", None), |ui| {
            controls::button(ui, &t.text("initial-settings-button-configure", None), controls::Kind::Secondary, false).on_disabled_hover_text(t.text("initial-settings-not-ported", None));
            // f.bi(): every league
            all = controls::button(ui, &t.text("initial-settings-select-all", None), controls::Kind::Secondary, true).clicked();
        });
        if all {
            for i in 0..self.new_game.leagues.len() {
                let country = self.new_game.leagues[i].country;
                self.new_game.leagues[i].selected = !self.new_game.unported_format(country);
            }
            self.error = None;
        }
        let chosen = self.new_game.leagues.iter().filter(|l| l.selected).count();
        ui.label(RichText::new(t.text("initial-settings-leagues-chosen", Some(&args([("count", chosen)])))).size(12.0).color(theme::MUTED));
        ui.add_space(6.0);
        egui::ScrollArea::vertical().id_salt("leagues").max_height(470.0).auto_shrink([false, false]).show(ui, |ui| {
            for i in 0..self.new_game.leagues.len() {
                let l = &self.new_game.leagues[i];
                let (country, teams, selected) = (l.country, l.teams, l.selected);
                let available = !self.new_game.unported_format(country);
                let row_rect = ui.available_rect_before_wrap();
                let row_rect = egui::Rect::from_min_size(row_rect.min, egui::vec2(row_rect.width(), 32.0));
                let row = ui.interact(row_rect, ui.auto_id_with(("league", i)), egui::Sense::click());
                let fill = if self.row == Some(i) {
                    theme::SELECTION
                } else if row.hovered() {
                    theme::HOVER
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(row_rect, 6.0, fill);
                let mut toggled = false;
                ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.add_space(8.0);
                        toggled = menu::check(ui, selected, available).clicked();
                        ui.add_space(8.0);
                        flag(ui, files, country, 18.0);
                        ui.add_space(2.0);
                        ui.label(RichText::new(country_name(t, country)).size(13.0).color(if available { theme::TEXT } else { theme::MUTED }));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(12.0);
                            let teams = t.text("initial-settings-teams-count", Some(&args([("count", teams)])));
                            ui.label(RichText::new(teams).size(12.0).color(theme::MUTED));
                        });
                    });
                });
                ui.allocate_rect(row_rect, egui::Sense::hover());
                if !available {
                    row.on_hover_text(t.text("initial-settings-league-not-ported", None));
                    continue;
                }
                if toggled || row.clicked() {
                    self.new_game.leagues[i].selected = !selected;
                    self.row = Some(i);
                    self.error = None;
                }
            }
        });
    }

    /// The state leagues (`Mz`) and the regionals.
    fn states(&mut self, ui: &mut egui::Ui, t: &Translator) {
        menu::panel_title(ui, &t.text("initial-settings-section-states", None), |ui| {
            controls::button(ui, &t.text("initial-settings-button-configure", None), controls::Kind::Secondary, false).on_disabled_hover_text(t.text("initial-settings-not-ported", None));
        });
        egui::ScrollArea::vertical().id_salt("states").max_height(120.0).auto_shrink([false, false]).show(ui, |ui| {
            for state in 0..STATES {
                let row_rect = ui.available_rect_before_wrap();
                let row_rect = egui::Rect::from_min_size(row_rect.min, egui::vec2(row_rect.width(), 24.0));
                if state % 2 == 1 {
                    ui.painter().rect_filled(row_rect, 4.0, theme::ROW_ALT);
                }
                ui.scope_builder(egui::UiBuilder::new().max_rect(row_rect), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.add_space(8.0);
                        ui.label(RichText::new(t.text(&format!("state-{state}"), None)).size(12.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(12.0);
                            let teams = t.text("initial-settings-teams-count", Some(&args([("count", self.new_game.state_counts[state])])));
                            ui.label(RichText::new(teams).size(12.0).color(theme::MUTED));
                        });
                    });
                });
                ui.allocate_rect(row_rect, egui::Sense::hover());
            }
        });
        ui.add_space(8.0);
        let mut off = false;
        labelled_check(ui, t, &t.text("initial-settings-checkbox-play-states", None), &mut off, false);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            labelled_check(ui, t, &t.text("initial-settings-checkbox-play-regionals", None), &mut off, false);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                controls::button(ui, &t.text("initial-settings-button-configure", None), controls::Kind::Secondary, false).on_disabled_hover_text(t.text("initial-settings-not-ported", None));
            });
        });
    }
}

impl CoachStep {
    fn new(mut career: Career) -> Self {
        let mut step = CoachStep {
            divisions: Vec::new(),
            chosen: None,
            random: None,
            random_on: false,
            name: String::new(),
            nationality: 29,
            added: 0,
            error: None,
            country: 0,
            career: {
                career.world.game.interactive = true;
                career
            },
        };
        step.show_country(0);
        step
    }

    fn league_countries(&self) -> Vec<CountryId> {
        self.career.world.game.league_countries.clone()
    }

    /// `bV.os()` and `ot()`: the country's divisions; the first club of the first division is
    /// chosen.
    fn show_country(&mut self, index: usize) {
        self.country = index;
        let Some(&country) = self.league_countries().get(index) else { return };
        self.divisions = coach_screen_divisions(&mut self.career.world, country);
        self.chosen = self.divisions.iter().enumerate().find_map(|(d, clubs)| clubs.first().map(|&c| (d, c)));
        self.random_on = false;
        self.random = None;
    }

    /// `bV.ol()`: a division and one of its clubs drawn.
    fn draw_random(&mut self) {
        let divisions = self.divisions.iter().rposition(|d| !d.is_empty()).map_or(0, |d| d + 1);
        if divisions == 0 {
            return;
        }
        let d = self.career.runtime.rng.fresh_int(divisions as i32) as usize;
        let clubs = &self.divisions[d];
        if clubs.is_empty() {
            return;
        }
        let i = self.career.runtime.rng.fresh_int(clubs.len() as i32) as usize;
        self.random = Some(clubs[i]);
    }

    /// `bV.oo()`
    fn validate(&self, t: &Translator) -> Option<String> {
        let name = self.name.trim();
        let length = name.chars().count();
        if name.is_empty() {
            Some(t.text("add-human-coach-error-name-empty", None))
        } else if length < 2 {
            Some(t.text("add-human-coach-error-name-short", None))
        } else if length > 35 {
            Some(t.text("add-human-coach-error-name-long", None))
        } else if self.chosen.is_none() && self.random.is_none() {
            Some(t.text("add-human-coach-error-no-team", None))
        } else {
            None
        }
    }

    /// `bV.or()`: the coach takes the drawn club or the chosen one.
    fn hire(&mut self) {
        let club = if self.random_on { self.random } else { self.chosen.map(|(_, c)| c) };
        if let Some(club) = club {
            let name = self.name.trim().to_owned();
            self.career.add_human_coach(club, &name, self.nationality);
        }
    }

    fn show(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> Option<Career> {
        let mut start = false;
        let title = if self.added == 0 {
            t.text("add-human-coach-title", None)
        } else {
            t.text("add-human-coach-title-n", Some(&args([("number", (self.added + 1).to_string())])))
        };
        menu::header(ui, t, &title, &t.text("add-human-coach-subtitle", None), false);
        let combo_width = ui.available_width() - 32.0;
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            // the country
            let countries = self.league_countries();
            let mut picked_country = None;
            menu::field_label(ui, &t.text("add-human-coach-label-country", None));
            let id = countries.get(self.country).map(|&c| self.career.world.country(c).country).unwrap_or(-1);
            egui::ComboBox::from_id_salt("coach-country").width(combo_width).selected_text(country_name(t, id)).show_ui(ui, |ui| {
                for (i, &c) in countries.iter().enumerate() {
                    let cid = self.career.world.country(c).country;
                    ui.horizontal(|ui| {
                        flag(ui, files, cid, 16.0);
                        if ui.selectable_label(i == self.country, country_name(t, cid)).clicked() {
                            picked_country = Some(i);
                        }
                    });
                }
            });
            if let Some(i) = picked_country {
                self.show_country(i);
            }
            ui.add_space(12.0);
            // the divisions
            let mut picked = None;
            for d in 0..self.divisions.len() {
                if self.divisions[d].is_empty() {
                    continue;
                }
                let label = t.text("add-human-coach-label-division", Some(&args([("division", (d + 1).to_string())])));
                menu::field_label(ui, &label);
                let clubs = self.divisions[d].clone();
                let world = &self.career.world;
                let shown = self.chosen.filter(|&(cd, _)| cd == d).map(|(_, c)| c).or(clubs.first().copied());
                let text = shown.map(|c| world.club(c).name.clone()).unwrap_or_default();
                ui.add_enabled_ui(!self.random_on, |ui| {
                    egui::ComboBox::from_id_salt(("coach-division", d)).width(combo_width).height(520.0).selected_text(text).show_ui(ui, |ui| {
                        for &c in &clubs {
                            ui.horizontal(|ui| {
                                crest_of(ui, files, &world.club(c).file_ref, 18.0);
                                if ui.selectable_label(shown == Some(c), &world.club(c).name).clicked() {
                                    picked = Some((d, c));
                                }
                            });
                        }
                    });
                });
                ui.add_space(8.0);
            }
            if let Some(p) = picked {
                // bV.cW(int), bV.ou()
                self.chosen = Some(p);
                self.random_on = false;
                self.error = None;
            }
            let mut on = self.random_on;
            labelled_check(ui, t, &t.text("add-human-coach-checkbox-random-team", None), &mut on, true);
            if on != self.random_on {
                self.random_on = on;
                if on {
                    self.draw_random();
                }
            }
            ui.add_space(14.0);
            menu::field_label(ui, &t.text("add-human-coach-label-name", None));
            ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(combo_width).char_limit(40).font(theme::regular(14.0)).margin(egui::vec2(10.0, 8.0)));
            ui.add_space(12.0);
            menu::field_label(ui, &t.text("add-human-coach-label-nationality", None));
            let mut nationality = self.nationality;
            egui::ComboBox::from_id_salt("coach-nationality").width(combo_width).height(360.0).selected_text(country_name(t, nationality)).show_ui(ui, |ui| {
                let mut all: Vec<(String, i32)> = countries::COUNTRIES.iter().map(|c| (country_name(t, c.id), c.id)).collect();
                all.sort_by(|a, b| collate(&a.0).cmp(&collate(&b.0)));
                for (name, id) in all {
                    ui.horizontal(|ui| {
                        flag(ui, files, id, 16.0);
                        if ui.selectable_label(id == nationality, name).clicked() {
                            nationality = id;
                        }
                    });
                }
            });
            self.nationality = nationality;
        });
        ui.add_space(12.0);
        // the club the coach takes
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            menu::field_label(ui, &t.text("add-human-coach-label-selected-team", None));
            ui.horizontal(|ui| {
                if self.random_on {
                    ui.label(RichText::new(t.text("add-human-coach-selected-random", None)).font(theme::bold(18.0)));
                } else if let Some((_, c)) = self.chosen {
                    let club = self.career.world.club(c);
                    crest_of(ui, files, &club.file_ref, 36.0);
                    ui.add_space(6.0);
                    ui.label(RichText::new(&club.name).font(theme::bold(18.0)));
                }
            });
        });
        ui.add_space(14.0);
        if let Some(e) = &self.error {
            menu::error(ui, e);
        }
        ui.horizontal(|ui| {
            if controls::button(ui, &t.text("add-human-coach-button-add-another", None), controls::Kind::Secondary, true).clicked() {
                // bV.om()
                match self.validate(t) {
                    None => {
                        self.hire();
                        self.added += 1;
                        self.name.clear();
                        let index = self.country;
                        self.show_country(index);
                        self.error = None;
                    }
                    Some(e) => self.error = Some(e),
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if controls::button(ui, &t.text("add-human-coach-button-start", None), controls::Kind::Primary, true).clicked() {
                    // bV.oq(), then bV.AB() (the friendly tournament of a.jH is not ported)
                    match self.validate(t) {
                        None => {
                            self.hire();
                            start = true;
                        }
                        Some(e) => self.error = Some(e),
                    }
                }
            });
        });
        ui.add_space(32.0);
        if start {
            let mut career = std::mem::replace(&mut self.career, Career::empty());
            match career.begin() {
                Ok(()) => return Some(career),
                Err(e) => {
                    self.error = Some(e);
                    self.career = career;
                }
            }
        }
        None
    }
}

/// A name for sorting by the reader's alphabet: accents fold onto their letter.
pub fn collate(name: &str) -> String {
    name.chars()
        .flat_map(|c| {
            let base = match c {
                'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
                'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
                'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
                'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
                'ç' | 'Ç' => 'c',
                other => other,
            };
            base.to_lowercase()
        })
        .collect()
}
