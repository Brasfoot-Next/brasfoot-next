//! The game's options (`est.Options`, kept in `options.bcf` and asked on the new game screen).
//! A new career copies them into its own fields (`best.f()`).

use serde::{Deserialize, Serialize};

use crate::saved::Game;

/// `est.Options`, with its Java field initialisers as defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameOptions {
    pub plays_state: bool,
    pub plays_regionals: bool,
    pub default_state_groups: bool,
    pub plays_international_clubs: bool,
    pub plays_all_national_teams: bool,
    pub monthly_salary: bool,
    pub individual_skills: bool,
    pub international_in_first_year: bool,
    pub default_international_groups: bool,
    pub show_international: Vec<bool>,
    pub autosave: i32,
    pub speed: i32,
    pub speed_non_human: i32,
    pub show_ai_penalties: i32,
    pub show_coach_changes: i32,
    pub show_substitutions_window: i32,
    pub contract_end_warning: i32,
    pub auto_renew_contracts: bool,
    pub use_scoreboard_color: bool,
    pub scoreboard_color: i32,
    pub use_sounds: bool,
    pub bold_home: bool,
    pub use_list_colors: bool,
    pub show_states_grouped: bool,
    pub ignore_leagues: bool,
    pub ignore_state: bool,
    pub use_real_cup_groups: bool,
    pub regionals_all: Vec<bool>,
    pub regionals_invitation: bool,
    pub regionals_without_humans: i32,
    pub new_cup_format: bool,
    pub theme_color: i32,
    pub show_auctions: bool,
}

impl Default for GameOptions {
    fn default() -> Self {
        GameOptions {
            plays_state: true,
            plays_regionals: true,
            default_state_groups: true,
            plays_international_clubs: true,
            plays_all_national_teams: true,
            monthly_salary: true,
            individual_skills: true,
            international_in_first_year: true,
            default_international_groups: false,
            show_international: vec![
                true, true, false, false, false, false, true, true, true, true, true, true, true, true, true, false, true, true, true, true, true,
            ],
            autosave: 0,
            speed: 4,
            speed_non_human: 4,
            show_ai_penalties: 0,
            show_coach_changes: 1,
            show_substitutions_window: 1,
            contract_end_warning: 0,
            auto_renew_contracts: true,
            use_scoreboard_color: true,
            scoreboard_color: 0,
            use_sounds: true,
            bold_home: true,
            use_list_colors: false,
            show_states_grouped: false,
            ignore_leagues: true,
            ignore_state: true,
            use_real_cup_groups: false,
            regionals_all: vec![true; 4],
            regionals_invitation: false,
            regionals_without_humans: 0,
            new_cup_format: true,
            theme_color: 0,
            show_auctions: false,
        }
    }
}

impl GameOptions {
    /// `best.f()`: a new career takes the options.
    pub fn apply(&self, game: &mut Game) {
        game.show_international = self.show_international.clone();
        game.international_in_first_year = self.international_in_first_year;
        game.default_international_groups = self.default_international_groups;
        game.plays_international_clubs = self.plays_international_clubs;
        game.plays_all_national_teams = self.plays_all_national_teams;
        game.plays_state = self.plays_state;
        game.plays_regionals = self.plays_regionals;
        game.monthly_salary = self.monthly_salary;
        game.individual_skills = self.individual_skills;
        game.ignore_state = self.ignore_state;
        game.default_state_groups = self.default_state_groups;
        game.autosave = self.autosave;
        game.speed = self.speed;
        game.speed_non_human = self.speed_non_human;
        game.show_ai_penalties = self.show_ai_penalties;
        game.show_coach_changes = self.show_coach_changes;
        game.show_substitutions_window = self.show_substitutions_window;
        game.contract_end_warning = self.contract_end_warning;
        game.ignore_leagues = self.ignore_leagues;
        game.auto_renew_contracts = self.auto_renew_contracts;
        game.use_scoreboard_color = self.use_scoreboard_color;
        game.scoreboard_color = self.scoreboard_color;
        game.use_sounds = self.use_sounds;
        game.use_list_colors = self.use_list_colors;
        game.bold_home = self.bold_home;
        game.show_states_grouped = self.show_states_grouped;
        game.use_real_cup_groups = self.use_real_cup_groups;
        game.regionals_all = self.regionals_all.clone();
        game.show_auctions = self.show_auctions;
    }
}
