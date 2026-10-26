// Port of the end of a season, f.al(): each league's team of the season and best player
// (f.B.fi), and the world-class status the best players earn (F.ay, F.fd).

use std::cmp::Ordering;

use brasfoot_model::ids::{CompetitionId, PlayerId};
use brasfoot_model::saved::SeasonSelection;

use crate::Sim;

/// `Double.compare(a, b)`: NaN above everything and equal to itself, -0.0 below 0.0.
pub fn java_double_compare(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => a.total_cmp(&b),
    }
}

impl Sim<'_> {
    /// `f.al()`
    pub fn end_season(&mut self) {
        for country in self.world.game.league_countries.clone() {
            for (i, league) in self.world.country(country).divisions.clone().into_iter().enumerate() {
                self.season_awards(league, i);
            }
        }
        for p in self.world.game.players.clone() {
            // F.ay(0): world-class players lose the status after 34; the award count restarts
            let player = self.world.player_mut(p);
            if player.world_class && player.age > 34 {
                player.world_class = false;
            }
            self.runtime.call_ups.insert(p, 0);
        }
        for country in self.world.game.league_countries.clone() {
            let first = self.world.country(country).divisions[0];
            for p in self.world.competition(first).players_zx.clone() {
                self.count_best_player_award(p);
            }
        }
    }

    /// `f.B.fi(int)`: the league's team of the season and its best player, among those rated in
    /// at least half the rounds.
    fn season_awards(&mut self, league: CompetitionId, division_index: usize) {
        let mut ratings = std::mem::take(&mut self.world.competition_mut(league).ratings);
        // f.B.zL() with at.tZ: average, then games, both descending
        ratings.sort_by(|a, b| {
            if a.average != b.average {
                java_double_compare(b.average, a.average)
            } else if a.rated_games != b.rated_games {
                java_double_compare(b.rated_games, a.rated_games)
            } else {
                Ordering::Equal
            }
        });
        let table = self.world.competition(league).league.expect("a league has its table");
        let minimum = crate::league::mid_season_round(self.world.stage(table)) as f64;
        const SLOTS: [i32; 11] = [0, 1, 2, 2, 5, 6, 6, 3, 3, 4, 4];
        let mut team = SeasonSelection { season: self.world.season(), ..Default::default() };
        for kind in SLOTS {
            if let Some(p) = ratings
                .iter()
                .filter(|r| r.value_ac == kind && r.rated_games >= minimum)
                .filter_map(|r| r.player)
                .find(|p| !team.players.contains(p))
            {
                team.players.push(p);
                team.clubs.push(self.world.player(p).club);
            }
        }
        let best = ratings.iter().find(|r| r.rated_games >= minimum).and_then(|r| r.player);
        let c = self.world.competition_mut(league);
        c.ratings = ratings;
        c.selections_zy.push(team);
        if let Some(p) = best {
            c.players_zx.push(p);
            if division_index == 0 {
                // F.a(Boolean): the best player of a first division becomes a star
                self.world.player_mut(p).star = true;
            }
        }
    }

    /// `F.fd()`: one more best-player award this count; enough of them make a world-class
    /// player in some countries.
    fn count_best_player_award(&mut self, p: PlayerId) {
        let awards = {
            let count = self.runtime.call_ups.entry(p).or_insert(0);
            *count += 1;
            *count
        };
        let player = self.world.player(p);
        if player.world_class || awards < 2 || player.age >= 35 {
            return;
        }
        let Some(club) = player.club else { return };
        let country = self.world.club(club).country;
        let earns = if brasfoot_model::countries::confederation(country) == 0 {
            matches!(country, 1 | 65 | 97) || (matches!(country, 104 | 72 | 154) && awards >= 3)
        } else {
            country == 29 && awards >= 4
        };
        if earns {
            self.world.player_mut(p).world_class = true;
        }
    }
}
