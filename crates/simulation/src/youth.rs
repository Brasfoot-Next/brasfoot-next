// Port of the players' year, f.af(): the generated players (F.gc), the players' birthday and
// retirement (F.gd, F.ge, F.a(String, int, boolean)), the youth players' birthday and promotion
// (F.gf, K.a(boolean, F, ah)), the clubs' youth intake (f.c(true)) and the new values and
// salaries (F.fK, F.fJ).

use brasfoot_model::ids::{ClubId, PlayerId};
use brasfoot_model::rng::java_round;
use brasfoot_model::saved::{Coach, RetiredIdol};

use crate::Sim;

/// The lists `f.af()` fills while it walks the players (`f.by`, `bz`, `bA`, `bB`).
#[derive(Default)]
struct YearLists {
    /// `bz`: players who retired, removed from the player list after the walk.
    retired: Vec<PlayerId>,
    /// `bB`: youth players promoted, removed from the youth list after the walk.
    promoted: Vec<PlayerId>,
}

impl Sim<'_> {
    /// `f.af()`
    pub fn players_year(&mut self) {
        let mut lists = YearLists::default();
        self.runtime.youth_intake.clear();
        for (i, p) in self.world.game.generated_players.clone().into_iter().enumerate() {
            if i == 0 {
                self.runtime.checkpoint("best.F.gc");
            }
            self.generated_player_year(p);
        }
        // the list grows while it is walked: promoted youth players are walked too
        let mut i = 0;
        while i < self.world.game.players.len() {
            let p = self.world.game.players[i];
            if i == 0 {
                self.runtime.checkpoint("best.F.gd");
            }
            self.player_year(p, &mut lists);
            i += 1;
        }
        let retired = std::mem::take(&mut lists.retired);
        for p in retired {
            if let Some(i) = self.world.game.players.iter().position(|&x| x == p) {
                self.world.game.players.remove(i);
            }
        }
        let mut i = 0;
        while i < self.world.game.youth_players.len() {
            let p = self.world.game.youth_players[i];
            if i == 0 {
                self.runtime.checkpoint("best.F.gf");
            }
            self.youth_year(p, &mut lists);
            i += 1;
        }
        self.youth_intake();
        for p in std::mem::take(&mut lists.promoted) {
            if let Some(i) = self.world.game.youth_players.iter().position(|&x| x == p) {
                self.world.game.youth_players.remove(i);
            }
        }
        for p in std::mem::take(&mut self.runtime.youth_intake) {
            if !self.world.game.youth_players.contains(&p) {
                self.world.game.youth_players.push(p);
            }
        }
        for p in self.world.game.players.clone() {
            self.compute_market_value(p);
            self.compute_salary(p);
        }
    }

    /// `F.gc()`: a generated player gets older; past 35 he becomes someone else.
    fn generated_player_year(&mut self, p: PlayerId) {
        self.world.player_mut(p).age += 1;
        if self.world.player(p).age > 35 {
            let age = self.runtime.rng.fresh_int(10) + 18;
            let country = self.world.player(p).country;
            let name = self.draw_name(country);
            let player = self.world.player_mut(p);
            player.age = age;
            player.name = name.unwrap_or_default();
            player.season_stats.clear();
            player.competition_stats.clear();
            player.career_games = 0;
            player.career_goals = 0;
        }
        let player = self.world.player_mut(p);
        player.competition_stats.clear();
        player.ratings.clear();
    }

    /// `F.gd()` with `F.ge()`: a birthday and, for the old, maybe the end of the career.
    fn player_year(&mut self, p: PlayerId, lists: &mut YearLists) {
        let player = self.world.player_mut(p);
        player.age += 1;
        player.competition_stats.clear();
        player.ratings.clear();
        if player.name.is_empty() || player.name == "TESTE" {
            let country = player.country;
            let name = self.draw_name(country);
            self.world.player_mut(p).name = name.unwrap_or_default();
        }
        let player = self.world.player(p);
        let Some(club) = player.club else { return };
        if player.on_loan {
            return;
        }
        if self.world.club(club).human {
            self.world.player_mut(p).listed_for_sale = false;
        }
        let draw = self.runtime.shared_int(100) + 1;
        let player = self.world.player(p);
        if player.age <= 32 {
            return;
        }
        let mut age = player.age;
        if player.star {
            age -= 1;
        }
        if player.world_class {
            age -= 3;
        }
        if player.position == 0 {
            age -= 3;
        }
        let retires = match age {
            ..32 => false,
            32 => draw > 99,
            33..=34 => draw > 90,
            35 => draw > 55,
            36 => draw > 30,
            37..=38 => draw > 15,
            39 => draw > 5,
            40 => draw > 3,
            41..=42 => draw > 2,
            43..=48 => draw > 1,
            _ => true,
        };
        if retires {
            if self.world.club(club).human {
                // a human club's player does not retire by himself: his coach hears he wants to
                let coach = self.world.club(club).coach.expect("a human club has its coach");
                let name = self.world.player(p).name.clone();
                brasfoot_engine::messages::send(self.world, coach, 34, 89, String::new(), name);
            } else {
                self.retire(p, lists);
            }
        }
    }

    /// `F.a(String, int, boolean)`: the player retires; the club may promote or create a youth
    /// player in his place, and some become coaches.
    fn retire(&mut self, p: PlayerId, lists: &mut YearLists) {
        self.retire_with(p, None, -1, true, lists);
    }

    /// `F.a(name, position, false)` from the retire window (`a.ji.qW()`): the human's player
    /// retires now; the youth player who takes his place may get the name and the position the
    /// human chose (-1 any). False when the player has no club.
    pub fn retire_by_request(&mut self, p: PlayerId, name: Option<String>, position: i32) -> bool {
        if self.world.player(p).club.is_none() {
            return false;
        }
        self.retire_with(p, name, position, false, &mut YearLists::default());
        true
    }

    /// `F.a(String, int, boolean)`: `year_end` is its third argument, the retirement of the
    /// players' year (the player leaves the list after the walk, `SR.bz`) or of the screen
    /// (he leaves it now).
    fn retire_with(&mut self, p: PlayerId, name: Option<String>, asked_position: i32, year_end: bool, lists: &mut YearLists) {
        self.runtime.checkpoint("best.F.a(Ljava/lang/String;IZ)Z");
        let player = self.world.player(p).clone();
        let club = player.club.expect("a player who retires has a club");
        let c = self.world.club(club).clone();
        let mut becomes_coach = false;
        if c.league_club {
            let mut odds = match c.division {
                0 => 120,
                1 => 25,
                2 => 50,
                3 => 60,
                4 => 100,
                _ => 125,
            };
            if player.star {
                odds = 5;
            }
            if player.world_class {
                odds = 2;
            }
            becomes_coach = self.runtime.rng.fresh_int(odds) == 1;
        }
        const MINIMUM: [i32; 5] = [2, 3, 3, 6, 4];
        let same_position = c.squad.iter().filter(|&&x| self.world.player(x).position == player.position).count() as i32 - 1;
        let mut position = asked_position;
        let mut promote = false;
        if !c.human {
            position = player.position;
            if same_position < MINIMUM[player.position as usize] {
                promote = true;
            }
            if c.squad.len() < 16 {
                promote = true;
            }
        }
        let youth = c.youth_squad.len();
        if youth >= 20 && c.human {
            promote = true;
        }
        let mut margin = 2;
        if c.division > 2 {
            margin = 5;
        }
        if c.reputation < 3 {
            margin = 5;
        }
        if c.human {
            margin = 0;
        }
        let no_replacement = youth as i32 >= 20 - margin && !promote;
        self.world.club_mut(club).squad.retain(|&x| x != p);
        let season = self.world.season();
        let goals = player.career_goals;
        let idol = (season < 2 && goals > 0) || (season < 10 && goals > 10) || (season < 20 && goals > 30) || goals > 50;
        if idol {
            // new b(String, int, int)
            self.world.game.retired_idols.push(RetiredIdol { name: player.name.clone(), goals, games: player.career_games });
        }
        let pl = self.world.player_mut(p);
        pl.season_stats.clear();
        pl.competition_stats.clear();
        pl.career_games = 0;
        pl.career_goals = 0;
        if pl.on_loan {
            pl.on_loan = false;
        }
        pl.club = None;
        if year_end {
            lists.retired.push(p);
        } else if let Some(i) = self.world.game.players.iter().position(|&x| x == p) {
            self.world.game.players.remove(i);
        }
        if !no_replacement {
            let new = self.make_youth_player(club, position, None, name, year_end);
            if promote {
                self.promote(new, club, year_end, lists);
            }
        }
        if becomes_coach {
            // new al(String), f.a(al), al.b(null, F)
            self.world.game.last_coach_id += 1;
            let coach = Coach {
                name: player.name.clone(),
                id: self.world.game.last_coach_id,
                country: player.country,
                start_season: season,
                ..Default::default()
            };
            let id = brasfoot_model::ids::CoachId(self.world.coaches.len() as u32);
            self.world.coaches.push(coach);
            self.world.game.coaches.push(id);
        }
    }

    /// `K.a(false, F, ah)`: a youth player of the youth list joins the first team.
    pub fn promote_from_youth_list(&mut self, p: PlayerId, club: ClubId) {
        self.promote(p, club, false, &mut YearLists::default());
    }

    /// `K.a(boolean, F, ah)`: a youth player joins the first team.
    fn promote(&mut self, p: PlayerId, club: ClubId, intake: bool, lists: &mut YearLists) {
        self.runtime.checkpoint("best.K.a(ZLbest/F;Lbest/ah;)V");
        let c = self.world.club(club);
        let confederation = brasfoot_model::countries::confederation(c.country);
        let mut boost = if c.league_club {
            match c.division {
                ..1 => 5,
                1 => 22,
                2 => 17,
                3 => 14,
                _ => 7,
            }
        } else {
            match c.reputation {
                5 => 20,
                4 => 15,
                3 => 12,
                2 => 7,
                _ => 5,
            }
        };
        if confederation == 0 {
            boost += 5;
        }
        boost += self.runtime.rng.fresh_int(5);
        let development = self.world.player(p).development.min(100);
        let share = development as f64 / 100.0;
        let mut strength = self.world.player(p).potential + java_round(boost as f64 * share) as i32;
        if self.world.player(p).potential >= 9 {
            strength += self.runtime.rng.fresh_int(10);
        }
        self.world.player_mut(p).strength = strength;
        if self.world.game.individual_skills {
            let c = self.world.club(club);
            let extra = if c.league_club {
                match c.division {
                    1 => 7,
                    2 => 3,
                    _ => 1,
                }
            } else {
                match c.reputation {
                    4 => 4,
                    5 => 7,
                    _ => 1,
                }
            };
            self.draw_attributes(p, (strength - 5).max(5), extra);
        }
        let season = self.world.season();
        let player = self.world.player_mut(p);
        player.youth = false;
        player.signed_season = season;
        player.strength_progress = 0.0;
        self.extend_contract(p, 180, true);
        self.compute_market_value(p);
        self.compute_salary(p);
        let player = self.world.player(p);
        if player.star {
            let star = self.runtime.rng.fresh_int(3) == 1;
            self.world.player_mut(p).set_star(star);
        } else if player.world_class {
            let player = self.world.player_mut(p);
            player.set_star(true);
            player.world_class = false;
        } else if self.runtime.rng.fresh_int(200) == 1 {
            self.world.player_mut(p).set_star(true);
        }
        let player = self.world.player_mut(p);
        if player.star && player.potential < 8 {
            player.set_potential(8);
        }
        for _ in 0..2 {
            let youth = &mut self.world.club_mut(club).youth_squad;
            if let Some(i) = youth.iter().position(|&x| x == p) {
                youth.remove(i);
            }
        }
        if intake && let Some(i) = self.runtime.youth_intake.iter().position(|&x| x == p) {
            self.runtime.youth_intake.remove(i);
        }
        if !self.world.club(club).squad.contains(&p) {
            self.world.club_mut(club).squad.push(p);
        }
        if !self.world.game.players.contains(&p) {
            self.world.game.players.push(p);
        }
        if intake {
            lists.promoted.push(p);
        } else if let Some(i) = self.world.game.youth_players.iter().position(|&x| x == p) {
            self.world.game.youth_players.remove(i);
        }
    }

    /// `F.gf()`: a youth player's birthday; at 20 he is promoted or replaced.
    fn youth_year(&mut self, p: PlayerId, lists: &mut YearLists) {
        let player = self.world.player_mut(p);
        player.age += 1;
        if player.name.is_empty() || player.name == "TESTE" {
            let country = player.country;
            let name = self.draw_name(country);
            self.world.player_mut(p).name = name.unwrap_or_default();
        }
        let player = self.world.player(p);
        let Some(club) = player.club else { return };
        if player.age < 20 {
            return;
        }
        const POTENTIAL_NEEDED: [i32; 6] = [1, 4, 5, 6, 6, 6];
        const MAXIMUM: [i32; 5] = [3, 5, 5, 8, 6];
        let c = self.world.club(club);
        let needed = POTENTIAL_NEEDED[c.reputation as usize];
        // ah.J(true): every player of the squad by position
        let in_position = c.squad.iter().filter(|&&x| self.world.player(x).position == player.position).count() as i32;
        let promoted = player.potential >= needed && in_position < MAXIMUM[player.position as usize];
        if promoted {
            if c.squad.len() < 32 {
                self.promote(p, club, true, lists);
                if self.world.club(club).youth_squad.len() < 20 {
                    self.make_youth_player(club, -1, None, None, true);
                }
            }
        } else if !c.human {
            self.make_youth_player(club, -1, Some(p), None, true);
        }
    }

    /// `f.c(true)`: the clubs with fewer than 15 youth players get new ones.
    fn youth_intake(&mut self) {
        for club in self.world.game.clubs.clone() {
            let youth = self.world.club(club).youth_squad.len() as i32;
            if youth >= 15 {
                continue;
            }
            let mut count = if self.world.club(club).league_club {
                2 + self.runtime.rng.fresh_int(3)
            } else {
                let mut n = self.runtime.rng.fresh_int(3);
                if self.world.club(club).level >= 20 {
                    n += 1;
                }
                n
            };
            if count + youth >= 20 {
                count = 20 - youth - 2;
            }
            if count > 0 {
                for _ in 0..=count {
                    self.make_youth_player(club, -1, None, None, true);
                }
            }
        }
    }
}
