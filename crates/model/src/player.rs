//! Players: methods of `best.F` and of the statistics it owns (`best.H`, `best.k`, `best.J`).
//! The data is in [`crate::saved`].

pub use crate::saved::{CompetitionStats, MatchRating, Player, SeasonStats};

/// Field positions (`F.en`). Brasfoot's G, L, Z, M, A.
pub mod position {
    pub const GOALKEEPER: i32 = 0;
    pub const FULL_BACK: i32 = 1;
    pub const CENTRE_BACK: i32 = 2;
    pub const MIDFIELDER: i32 = 3;
    pub const FORWARD: i32 = 4;
}

/// Indexes of `Player::attributes` (individual-skills mode), in the game's own order
/// (`F.aF(int)`): the mod's labels Gol, Vel, Tec, Pas, Des, Arm, Fin.
pub mod attribute {
    pub const GOALKEEPING: usize = 0;
    pub const SPEED: usize = 1;
    pub const TECHNIQUE: usize = 2;
    pub const PASSING: usize = 3;
    pub const DEFENDING: usize = 4;
    pub const PLAYMAKING: usize = 5;
    pub const FINISHING: usize = 6;
    pub const COUNT: usize = 7;
}

impl CompetitionStats {
    /// `H.gT`: suspended for the next match.
    pub fn is_suspended(&self) -> bool {
        self.yellow_cards >= 3 || self.red_cards >= 1
    }

    /// `H.gU`: a match served.
    pub fn serve_suspension(&mut self) {
        if self.yellow_cards >= 3 {
            self.yellow_cards = 0;
        } else if self.red_cards >= 1 {
            self.red_cards -= 1;
        }
    }

    /// `H.i(double)`
    pub fn add_rating(&mut self, rating: f64) {
        self.rated_games += 1;
        self.rating_sum += rating;
    }

    /// `H.F()`
    pub fn average_rating(&self) -> f64 {
        if self.rated_games > 0 { self.rating_sum / self.rated_games as f64 } else { 0.0 }
    }
}

impl SeasonStats {
    /// `k.b(double)`: only ratings of 2 or more count.
    pub fn add_rating(&mut self, rating: f64) {
        if rating >= 2.0 {
            self.rating_sum += rating;
            self.rated_games += 1;
        }
    }

    /// `k.cE()` without the text: the average, if any match was rated.
    pub fn average_rating(&self) -> Option<f64> {
        (self.rated_games > 0).then(|| self.rating_sum / self.rated_games as f64)
    }
}

impl Player {
    /// `F.a(Boolean)`: world-class players are always stars.
    pub fn set_star(&mut self, star: bool) {
        self.star = star || self.world_class;
    }

    /// `F.setIdade`: out-of-range ages become 18 (youth) or 35.
    pub fn set_age(&mut self, age: i32) {
        self.age = if self.youth {
            if !(16..=20).contains(&age) { 18 } else { age }
        } else if !(16..=48).contains(&age) {
            35
        } else {
            age
        };
    }

    /// `F.setLado`
    pub fn set_side(&mut self, side: i32) {
        self.side = if side == 0 { 0 } else { 1 };
    }

    /// `F.aq(int)`: stars above 8 are 10.
    pub fn set_potential(&mut self, potential: i32) {
        self.potential = if self.star && potential > 8 { 10 } else { potential };
    }

    /// `F.fG`: the style that follows from position and traits.
    pub fn compute_style(&mut self) {
        let (t1, t2) = (self.trait1, self.trait2);
        match self.position {
            position::GOALKEEPER | position::CENTRE_BACK => self.style = 0,
            position::FULL_BACK => {
                self.style = if t1 == 13 || t1 == 6 {
                    1
                } else if t1 == 7 || t1 == 10 {
                    0
                // the original tests `var1 == 6` again here instead of trait 2
                } else if t2 == 13 || t1 == 6 {
                    1
                } else if t2 == 7 || t2 == 10 {
                    0
                } else if t1 != 8 && t1 != 9 && t1 != 11 && t1 != 4 {
                    0
                } else {
                    1
                };
            }
            position::MIDFIELDER => {
                self.style = if matches!(t1, 11 | 9 | 8 | 4) {
                    1
                } else if t1 == 7 || t1 == 10 {
                    0
                } else if matches!(t2, 11 | 9 | 8 | 4) {
                    1
                } else if t2 != 7 && t2 != 10 {
                    1
                } else {
                    0
                };
            }
            position::FORWARD => {
                self.style = if t1 == 7 || t1 == 10 {
                    0
                } else if t1 != 8 && t1 != 13 && t1 != 6 {
                    1
                } else {
                    2
                };
            }
            _ => {}
        }
    }

    /// `F.aj(int)`: tiredness; never below 1.
    pub fn tire(&mut self, amount: i32) {
        self.fitness -= amount;
        if self.fitness < 0 {
            self.fitness = 1;
        }
    }

    /// `F.ak(int)`
    pub fn recover(&mut self, amount: i32) {
        self.fitness = (self.fitness + amount).min(100);
    }

    /// `F.fq`: the tiredness of a match, by age.
    pub fn tire_after_match(&mut self) {
        let amount = match self.age {
            ..=20 => 1,
            ..=25 => 2,
            ..=31 => 3,
            ..=36 => 4,
            _ => 5,
        };
        self.tire(amount);
    }

    /// `F.fr`: recovery between match days. `human_club` is `fg().jZ()`.
    pub fn recover_between_matches(&mut self, human_club: bool) {
        let age = self.age;
        let amount = if self.started_recently {
            if human_club {
                match age {
                    ..=20 => 13,
                    ..=25 => 24,
                    ..=31 => 37,
                    ..=36 => 40,
                    _ => 30,
                }
            } else {
                match age {
                    ..=20 => 20,
                    ..=25 => 30,
                    ..=31 => 50,
                    ..=36 => 52,
                    _ => 42,
                }
            }
        } else {
            match age {
                ..20 => 30,
                ..26 => 30,
                ..33 => 35,
                ..45 => 35,
                _ => 30,
            }
        };
        self.recover(amount);
    }

    /// `F.fP`: injured on `today` (epoch millis of the current day).
    pub fn is_injured(&self, today: i64) -> bool {
        self.injured_until > 0 && self.injured_until > today
    }

    /// `F.fR`: whole days of contract left, 0 when expired.
    pub fn contract_days_left(&self, today: i64) -> i32 {
        days_between(today, self.contract_until)
    }

    /// `F.fU`: strength scaled by fitness. Integer division in the original: only a fully fit
    /// player keeps his strength, everyone else counts 0.
    pub fn effective_strength(&self) -> i32 {
        self.fitness / 100 * self.strength
    }

    /// `F.aF(int)`
    pub fn attribute(&self, index: usize) -> i32 {
        self.attributes[index.min(attribute::FINISHING)]
    }

    /// `F.aG(int)`: one point more, up to 100.
    pub fn raise_attribute(&mut self, index: usize) {
        if index < attribute::COUNT {
            self.attributes[index] = (self.attributes[index] + 1).min(100);
        }
    }

    /// `F.aH(int)`
    pub fn lower_attribute(&mut self, index: usize) {
        if index < attribute::COUNT {
            self.attributes[index] -= 1;
        }
    }

    /// `F.c(double, int)`: goalkeeping and defending progress 20% faster.
    pub fn add_attribute_progress(&mut self, amount: f64, index: usize) {
        let amount = if index == attribute::DEFENDING || index == attribute::GOALKEEPING { amount * 1.2 } else { amount };
        self.attribute_progress[index] += amount;
    }

    /// `F.h(double)`: progress on the main attribute of the position and, at 40% and 30%, on
    /// two secondary ones. Returns the main attribute.
    pub fn add_position_progress(&mut self, amount: f64) -> usize {
        use attribute::*;
        let (main, second, third): (usize, Option<usize>, Option<usize>) = match self.position {
            position::GOALKEEPER => (GOALKEEPING, None, None),
            position::FULL_BACK | position::MIDFIELDER if self.style == 0 => (DEFENDING, Some(PLAYMAKING), Some(FINISHING)),
            position::FULL_BACK => (PASSING, Some(FINISHING), Some(DEFENDING)),
            position::MIDFIELDER => (PLAYMAKING, Some(FINISHING), Some(DEFENDING)),
            position::CENTRE_BACK => (DEFENDING, Some(PASSING), Some(PLAYMAKING)),
            position::FORWARD => (FINISHING, Some(PLAYMAKING), Some(DEFENDING)),
            _ => (GOALKEEPING, None, None),
        };
        self.add_attribute_progress(amount, main);
        if let Some(second) = second {
            self.add_attribute_progress(amount * 0.4, second);
        }
        if let Some(third) = third {
            self.add_attribute_progress(amount * 0.3, third);
        }
        main
    }

    /// `F.gw`: career totals: games, goals, yellow cards, red cards, injuries, assists.
    pub fn career_totals(&self) -> [i32; 6] {
        let mut totals = [0; 6];
        for s in self.season_stats.iter().rev() {
            totals[0] += s.games;
            totals[1] += s.goals;
            totals[2] += s.yellow_cards;
            totals[3] += s.red_cards;
            totals[4] += s.injuries;
            totals[5] += s.assists;
        }
        totals
    }

    /// `F.gx` without the text: the career average rating, if any match was rated.
    pub fn career_average_rating(&self) -> Option<f64> {
        let (count, sum) = self.season_stats.iter().rev().fold((0, 0.0), |(c, s), k| (c + k.rated_games, s + k.rating_sum));
        (count > 0).then(|| sum / count as f64)
    }

    /// `F.gn` without the text: average of this season's ratings of 2 or more.
    pub fn season_average_rating(&self) -> Option<f64> {
        let (sum, count) = self.ratings.iter().rev().filter(|r| r.rating >= 2.0).fold((0.0, 0.0), |(s, c), r| (s + r.rating, c + 1.0));
        (count > 0.0).then(|| sum / count)
    }

    /// `F.gs`, `F.gt`, `F.gu`: assists, goals and games in `season`.
    pub fn season_totals(&self, season: i32) -> (i32, i32, i32) {
        self.season_stats.iter().filter(|k| k.season == season).fold((0, 0, 0), |(a, g, p), k| (a + k.assists, g + k.goals, p + k.games))
    }
}

/// `F.a(long, long)`: whole days from `from` to `to`, 0 if `to` is not later.
pub fn days_between(from: i64, to: i64) -> i32 {
    if to > from { ((to - from) / 86_400_000) as i32 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fictional() -> Player {
        Player {
            name: "Fulano de Tal".into(),
            age: 24,
            strength: 60,
            fitness: 100,
            attribute_progress: vec![0.0; attribute::COUNT],
            ..Default::default()
        }
    }

    #[test]
    fn effective_strength_keeps_the_integer_division() {
        let mut p = fictional();
        assert_eq!(p.effective_strength(), 60);
        p.fitness = 99;
        assert_eq!(p.effective_strength(), 0);
    }

    #[test]
    fn fitness_bounds() {
        let mut p = fictional();
        p.tire(150);
        assert_eq!(p.fitness, 1);
        p.started_recently = true;
        p.recover_between_matches(false);
        assert_eq!(p.fitness, 31);
        p.recover(500);
        assert_eq!(p.fitness, 100);
    }

    #[test]
    fn suspensions() {
        let mut s = CompetitionStats { yellow_cards: 3, red_cards: 1, ..Default::default() };
        assert!(s.is_suspended());
        s.serve_suspension();
        assert_eq!((s.yellow_cards, s.red_cards), (0, 1));
        s.serve_suspension();
        assert!(!s.is_suspended());
    }

    #[test]
    fn style_of_a_forward() {
        let mut p = Player { position: position::FORWARD, trait1: 8, ..fictional() };
        p.compute_style();
        assert_eq!(p.style, 2);
        p.trait1 = 7;
        p.compute_style();
        assert_eq!(p.style, 0);
    }
}
