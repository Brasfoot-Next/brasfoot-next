// Port of the players' evolution: f.aT() with F.fV() (fW() up to 31, fX() from 32), F.fY()
// (development of the youth), F.e(double), F.f(double), F.b(double, int) and ah.lj().

use brasfoot_model::ids::{ClubId, PlayerId};

use crate::Sim;

/// The strength ceiling a league club's division and reputation allow (`F.fW`, `F.b`).
const LEAGUE_CEILING: [[i32; 6]; 6] = [
    [30, 30, 30, 30, 30, 30],
    [80, 85, 90, 95, 100, 100],
    [50, 60, 60, 65, 80, 80],
    [40, 40, 40, 45, 70, 70],
    [25, 30, 30, 30, 50, 60],
    [30, 30, 30, 30, 30, 30],
];

/// The ceiling of the club's country, by confederation and country id.
fn country_ceiling(confederation: i32, country: i32) -> i32 {
    match confederation {
        0 => match country {
            3 | 72 | 104 | 65 | 97 => 95,
            154 | 85 => 90,
            21 | 162 => 80,
            _ => 70,
        },
        1 => match country {
            29 | 11 => 90,
            195 | 46 | 42 => 80,
            151 | 150 => 70,
            _ => 60,
        },
        2 => match country {
            10 | 129 | 57 => 75,
            141 | 169 | 190 => 70,
            _ => 60,
        },
        3 => match country {
            107 | 49 => 75,
            98 | 9 | 59 | 43 => 70,
            _ => 60,
        },
        4 => match country {
            131 => 80,
            68 => 70,
            51 => 65,
            _ => 55,
        },
        5 => {
            if country == 143 {
                60
            } else {
                45
            }
        }
        _ => 100,
    }
}

impl Sim<'_> {
    /// `f.aT()`: the evolution task of the calendar.
    pub fn evolve_players(&mut self) {
        let individual = self.world.game.individual_skills;
        if individual && self.world.game.training_cycle == 3 {
            for club in self.world.game.clubs.clone() {
                // ah.lj(): the AI picks new training focuses
                for i in 0..5 {
                    let focus = 1 + self.runtime.rng.fresh_int(3);
                    self.world.club_mut(club).training_focus[i] = focus;
                }
            }
        }
        for p in self.world.game.players.clone() {
            self.evolve(p);
        }
        for p in self.world.game.youth_players.clone() {
            self.develop_youth(p);
        }
        if individual {
            if self.world.game.training_cycle == 3 {
                self.world.game.training_cycle = 0;
            } else {
                self.world.game.training_cycle += 1;
            }
        }
    }

    /// `F.fV()`
    fn evolve(&mut self, p: PlayerId) {
        let Some(club) = self.world.player(p).club else { return };
        if self.world.player(p).age < 32 {
            self.grow(p, club);
        } else {
            self.decline(p, club);
        }
        self.world.player_mut(p).started_recently = false;
    }

    /// The strength ceiling of a club, from `F.fW` (`reputation_three` 65) and `F.b` (70);
    /// raised for well developed players with potential. Fixes a reputation out of range to 1,
    /// as the original does.
    fn ceiling(&mut self, p: PlayerId, club: ClubId, reputation_three: i32) -> i32 {
        let c = self.world.club(club);
        let mut ceiling = if c.league_club {
            if c.reputation < 0 || c.reputation > 5 {
                self.world.club_mut(club).reputation = 1;
            }
            let c = self.world.club(club);
            LEAGUE_CEILING[c.division as usize][c.reputation as usize]
        } else {
            match c.reputation {
                5 | 4 => 100,
                3 => reputation_three,
                2 => 40,
                1 => 30,
                _ => 20,
            }
        };
        let c = self.world.club(club);
        let country_cap = country_ceiling(brasfoot_model::countries::confederation(c.country), c.country);
        ceiling = ceiling.min(country_cap);
        let (development, potential) = (self.world.player(p).development, self.world.player(p).potential);
        if development >= 60 {
            let extra = self.runtime.rng.fresh_int(5);
            match potential {
                7 => ceiling += 5 + extra,
                8 => ceiling += 15 + extra,
                9 => ceiling += 25 + extra,
                10 => ceiling += 30 + extra,
                _ => {}
            }
            ceiling = ceiling.min(100);
        }
        ceiling
    }

    /// `F.fW()`: players up to 31 gain strength.
    fn grow(&mut self, p: PlayerId, club: ClubId) {
        let base = 50.0;
        let c = self.world.club(club);
        let mut level = c.level;
        let mut bonus = 0.0;
        if !c.league_club {
            bonus = 0.03;
            level = match c.reputation {
                4.. => 20,
                3 => 18,
                _ => 12,
            };
        }
        let player = self.world.player(p);
        let age = player.age;
        let mut gain = if level >= 19 {
            match age {
                ..20 => 8.0,
                ..23 => 6.0,
                ..29 => 5.0,
                _ => 4.0,
            }
        } else if level >= 15 {
            match age {
                ..18 => 6.0,
                ..21 => 5.0,
                ..29 => 4.0,
                _ => 3.0,
            }
        } else if level >= 11 {
            match age {
                ..18 => 5.0,
                ..21 => 4.0,
                ..29 => 3.0,
                _ => 2.0,
            }
        } else {
            match age {
                ..18 => 4.0,
                ..21 => 3.0,
                ..29 => 2.0,
                _ => 1.0,
            }
        } / base;
        if player.started_recently {
            gain += 0.04;
        }
        match player.strength {
            30..=40 => gain -= 0.02,
            41..=50 => gain -= 0.03,
            51..=70 => gain -= 0.04,
            71..=100 => gain -= 0.05,
            _ => {}
        }
        if player.development > 0 {
            if player.development < 50 {
                gain -= 0.05;
            } else if player.development < 70 {
                gain -= 0.02;
            }
            if player.potential >= 9 {
                gain += 0.07;
            } else if player.potential >= 7 {
                gain += 0.05;
            }
        }
        if player.world_class {
            gain += 0.02;
        } else if player.star {
            gain += 0.01;
        }
        let country = c.country;
        match brasfoot_model::countries::confederation(country) {
            2 | 3 => {
                if gain > 0.06 {
                    gain -= 0.02;
                }
            }
            5 => {
                if gain > 0.06 {
                    gain -= 0.04;
                }
            }
            4 => {
                if country != 131 && gain > 0.06 {
                    gain -= 0.03;
                }
            }
            // the original's test (id != 29 || id != 11 || ...) always holds
            1 => {
                if gain > 0.06 {
                    gain -= 0.02;
                }
            }
            0 => {
                if matches!(country, 3 | 72 | 104 | 65 | 97) {
                    gain += 0.01;
                } else if !matches!(country, 154 | 85 | 21) {
                    if gain > 0.06 {
                        gain -= 0.02;
                    }
                } else if gain > 0.06 {
                    gain -= 0.01;
                }
            }
            _ => {}
        }
        gain += bonus;
        if gain < 0.0 {
            gain = 0.01;
        }
        if self.world.game.individual_skills {
            self.train_attributes(p, club, gain);
        }
        self.world.player_mut(p).strength_progress += gain;
        let ceiling = self.ceiling(p, club, 65);
        let player = self.world.player_mut(p);
        if player.strength_progress > 1.0 && player.strength < 100 {
            if player.strength < ceiling {
                player.strength += 1;
                player.strength_progress -= 1.0;
            } else {
                player.strength_progress = 1.0;
            }
        }
        player.strength = player.strength.min(100);
    }

    /// `F.f(double)`: the attributes a young player trains.
    fn train_attributes(&mut self, p: PlayerId, club: ClubId, amount: f64) {
        let position = self.world.player(p).position;
        let c = self.world.club(club);
        let focus = c.training_focus[position as usize] as usize;
        let mut by_position = self.world.game.training_cycle != 3;
        if c.human && !c.auto_training && c.training_mode == 1 {
            by_position = false;
        }
        if by_position {
            self.world.player_mut(p).add_position_progress(amount);
        } else {
            self.world.player_mut(p).add_attribute_progress(amount * 2.0, focus);
        }
        for i in 0..7 {
            let (progress, value) = (self.world.player(p).attribute_progress[i], self.world.player(p).attributes[i]);
            if self.attribute_may_grow(p, club, progress, value) && self.world.player(p).attributes[i] < 100 {
                let player = self.world.player_mut(p);
                player.raise_attribute(i);
                player.attribute_progress[i] -= 1.0;
            }
        }
    }

    /// `F.b(double, int)`: whether an attribute with this progress may grow.
    fn attribute_may_grow(&mut self, p: PlayerId, club: ClubId, progress: f64, value: i32) -> bool {
        let ceiling = self.ceiling(p, club, 70);
        progress > 1.0 && value < 100 && value < ceiling
    }

    /// `F.fX()`: players from 32 lose strength.
    fn decline(&mut self, p: PlayerId, club: ClubId) {
        let c = self.world.club(club);
        let mut division = c.division;
        if !c.league_club {
            division = match c.reputation {
                4.. => 1,
                3 => 2,
                _ => 3,
            };
        }
        let player = self.world.player(p);
        let mut years = (player.age - 31) as f64;
        if c.level >= 20 {
            years -= 2.0;
        }
        let loss = match player.strength {
            1..=50 => 0.7 * years,
            51..=70 => 1.0 * years,
            71..=100 => 1.2 * years,
            _ => 0.0,
        };
        if loss > 0.0 {
            let amount = loss / 50.0;
            let floor = match division {
                1 => 35,
                2 => 25,
                3 => 10,
                _ => 1,
            };
            self.world.player_mut(p).strength_progress += amount;
            if self.world.game.individual_skills {
                self.lose_attributes(p, amount);
            }
            let player = self.world.player_mut(p);
            if player.strength_progress > 1.0 && player.strength > floor {
                player.strength -= 1;
                player.strength_progress -= 1.0;
            }
        }
        let player = self.world.player_mut(p);
        player.strength = player.strength.max(1);
    }

    /// `F.e(double)`: an older player's attributes fade.
    fn lose_attributes(&mut self, p: PlayerId, amount: f64) {
        let by_position = self.world.game.training_cycle != 3;
        let player = self.world.player_mut(p);
        let mut index = 0;
        if by_position {
            index = player.add_position_progress(amount);
        } else {
            player.add_attribute_progress(amount * 10.0, index);
        }
        if player.attribute_progress[index] > 1.0 && player.attributes[index] > 1 {
            player.lower_attribute(index);
            player.attribute_progress[index] -= 1.0;
        }
    }

    /// `F.fY()`: the youth develop up to 20.
    fn develop_youth(&mut self, p: PlayerId) {
        let player = self.world.player_mut(p);
        if player.club.is_none() || player.age > 20 {
            return;
        }
        let base = 40.0;
        let mut gain = match player.age {
            ..=17 => 20.0 / base,
            18 => 15.0 / base,
            19 => 14.0 / base,
            _ => 5.0 / base,
        };
        gain += match player.potential {
            ..=3 => 0.03,
            ..=6 => 0.04,
            ..=8 => 0.07,
            9 => 0.1,
            10 => 0.11,
            _ => 0.0,
        };
        player.development_progress += gain;
        if player.development_progress > 1.0 && player.development < 100 {
            player.development += 1;
            player.development_progress -= 1.0;
        }
        player.development = player.development.min(100);
    }
}
