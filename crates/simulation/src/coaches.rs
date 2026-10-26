// Port of best.al's match records: al.e(I) (career numbers and points), al.a(I, boolean, int)
// (board confidence and fan support after a match) and al.C(ah) (the season with a club).

use brasfoot_engine::messages;
use brasfoot_model::ids::{ClubId, CoachId, MatchId};
use brasfoot_model::saved::CoachSeason;

use crate::Sim;

impl Sim<'_> {
    /// `al.fg()`: the coach's club, by its game id.
    pub fn coach_club(&self, coach: CoachId) -> Option<ClubId> {
        self.world.coach(coach).club_id
    }

    /// `al.C(ah)`: this season's record with a club, created on first use.
    fn coach_season(&mut self, coach: CoachId, club: Option<ClubId>) -> &mut CoachSeason {
        let season = self.world.season();
        let club_id = club.map(|c| self.world.club(c).id);
        let seasons = &mut self.world.coach_mut(coach).seasons;
        let i = match seasons.iter().position(|j| Some(j.club_id) == club_id && j.season == season) {
            Some(i) => i,
            None => {
                // new j(ah): an unknown club would fail in the original
                seasons.push(CoachSeason { season, club_id: club_id.unwrap_or(-1), ..Default::default() });
                seasons.len() - 1
            }
        };
        &mut seasons[i]
    }

    /// `al.D(ah)`: a title in the season with a club.
    pub fn coach_title(&mut self, coach: CoachId, club: Option<ClubId>) {
        self.coach_season(coach, club).titles += 1;
        self.world.coach_mut(coach).titles += 1;
    }

    /// `al.e(I)`: a played match in the coach's record.
    pub fn coach_match_record(&mut self, coach: CoachId, m: MatchId) {
        let g = self.world.game_match(m).clone();
        let own = self.coach_club(coach);
        let other = self.world.coach(coach).national_team;
        let (mut scored, mut conceded, mut away, mut club) = (0, 0, false, None);
        if g.home.is_some() && g.home == own {
            (scored, conceded, club) = (g.home_goals, g.away_goals, own);
        } else if g.away.is_some() && g.away == own {
            (scored, conceded, away, club) = (g.away_goals, g.home_goals, true, own);
        } else if g.home == other {
            (scored, conceded, club) = (g.home_goals, g.away_goals, other);
        } else if g.away == other {
            (scored, conceded, club, away) = (g.away_goals, g.home_goals, other, true);
        }
        let (mut kind, mut division) = (0, 0);
        if let Some(c) = g.competition {
            let comp = self.world.competition(c);
            kind = comp.kind;
            if kind == 1 || kind == 3 {
                division = comp.division;
            }
        }
        let mut points = 0;
        let record = self.coach_season(coach, club);
        record.games += 1;
        let won = scored > conceded;
        let lost = scored < conceded;
        if won {
            record.wins += 1;
        } else if lost {
            record.losses += 1;
        }
        self.world.coach_mut(coach).games += 1;
        if won {
            self.world.coach_mut(coach).wins += 1;
            points = match kind {
                1 => match division {
                    1 => 4,
                    2 => 3,
                    3 => 2,
                    _ => 1,
                },
                2 | 6 | 8 => 4,
                // `var8` of the original is never set: always the false branch
                3 => 1,
                4 => 5,
                5 => 7,
                7 => 8,
                10 | 11 | 9 => 2,
                12 => 3,
                13 => 5,
                14 => 6,
                _ => 0,
            };
            if away && kind != 7 && kind != 5 {
                points += 1;
            }
        } else if lost {
            self.world.coach_mut(coach).losses += 1;
        } else {
            // the away bonus is added first and then overwritten for most kinds, as in the original
            if away && kind != 7 && kind != 5 {
                points += 1;
            }
            points = match kind {
                1 => match division {
                    1 => 2,
                    2 => 1,
                    _ => points,
                },
                2 => 1,
                4 | 5 | 6 | 7 | 8 | 11 => 2,
                10 | 12 | 9 => 1,
                13 | 14 => 2,
                _ => points,
            };
        }
        if (won || !lost) && points > 0 {
            self.coach_season(coach, club).points += points;
        }
        if points > 0 {
            self.world.coach_mut(coach).points += points;
        }
    }

    /// `al.cj(int)`: board confidence, 0 to 100.
    fn board(&mut self, coach: CoachId, delta: i32) {
        let c = self.world.coach_mut(coach);
        c.board_confidence = (c.board_confidence + delta).clamp(0, 100);
    }

    /// `al.ck(int)`: fan support, 0 to 100.
    fn fans(&mut self, coach: CoachId, delta: i32) {
        let c = self.world.coach_mut(coach);
        c.fan_support = (c.fan_support + delta).clamp(0, 100);
    }

    /// Port of `al.a(I, boolean, int)`: how a result moves the board and the fans. `friendly`
    /// skips the table position; `forced` > 0 replaces the result.
    pub fn coach_mood(&mut self, coach: CoachId, m: MatchId, friendly: bool, forced: i32) {
        let g = self.world.game_match(m).clone();
        let kind = g.competition.map(|c| self.world.competition(c).kind).unwrap_or(0);
        let own = self.coach_club(coach);
        let (mut scored, mut conceded, mut away, mut rival, mut level_gap) = (0, 0, false, false, 0);
        let mut band = [0; 3];
        if g.home.is_some() && g.home == own {
            let (h, a) = (g.home.unwrap(), g.away.unwrap());
            scored = g.home_goals;
            conceded = g.away_goals;
            rival = self.world.club(h).rivals.contains(&a);
            level_gap = self.world.club(h).level - self.world.club(a).level;
            if !friendly {
                band = self.table_band(h, g.competition);
            }
        } else if g.away.is_some() && g.away == own {
            let (h, a) = (g.home.unwrap(), g.away.unwrap());
            scored = g.away_goals;
            conceded = g.home_goals;
            away = true;
            rival = self.world.club(a).rivals.contains(&h);
            level_gap = self.world.club(a).level - self.world.club(h).level;
            if !friendly {
                band = self.table_band(a, g.competition);
            }
        }
        let _ = rival;
        if friendly {
            band[2] = -1;
        }
        let mut result = 0;
        let mut thrashing = false;
        if scored != conceded {
            result = if scored > conceded { 1 } else { 2 };
            thrashing = (scored - conceded).abs() >= 3;
        }
        if result == 0 {
            if level_gap >= 0 && !away {
                self.board(coach, -1);
                self.fans(coach, -1);
            } else if level_gap < 0 {
                self.board(coach, 1);
            }
        } else if result != 1 {
            if level_gap >= 0 && !away {
                self.board(coach, -3);
                self.fans(coach, -3);
            } else if level_gap >= 0 && away {
                self.board(coach, -1);
            }
        }
        let mut no_table = false;
        let mut band_index = 2usize;
        if kind == 1 || kind == 3 {
            band_index = band[1].max(0) as usize;
        }
        if band[2] == -1 || friendly {
            no_table = true;
        }
        if no_table && !g.knockout {
            no_table = false;
            band_index = 1;
        } else if forced >= 1 {
            result = forced;
            no_table = false;
            band_index = 1;
        }
        let (mut win_home, mut draw_home, mut loss_home) = ([0; 6], [0; 6], [0; 6]);
        let (mut win_away, mut draw_away, mut loss_away) = ([0; 6], [0; 6], [0; 6]);
        if !no_table {
            let broke = self.world.coach(coach).human && own.is_some_and(|c| self.world.club(c).cash < 0);
            let tables: ([i32; 6], [i32; 6], [i32; 6], [i32; 6], [i32; 6], [i32; 6]) = if kind == 1 {
                if broke {
                    self.board(coach, -10);
                }
                ([0, 5, 4, 1, 1, 0], [0, 2, 1, -1, -1, -3], [0, -2, -2, -3, -5, -7], [0, 6, 5, 2, 2, 1], [0, 2, 1, 0, -1, -2], [0, -1, -1, -2, -4, -5])
            } else if kind != 3 && kind != 10 {
                if broke {
                    self.board(coach, -5);
                }
                if kind != 4 && kind != 6 {
                    ([0, 5, 4, 1, 1, 0], [0, 2, 1, -1, -1, -3], [0, -2, -2, -3, -5, -7], [0, 6, 5, 2, 2, 1], [0, 2, 1, 0, -1, -2], [0, -1, -1, -2, -4, -5])
                } else {
                    ([0, 5, 4, 3, 3, 3], [0, 2, 1, 0, -1, -3], [0, -2, -2, -3, -3, -5], [0, 7, 6, 4, 4, 5], [0, 2, 1, 0, 0, -2], [0, -1, -1, -2, -2, -3])
                }
            } else {
                if broke {
                    self.board(coach, -5);
                }
                ([0, 2, 2, 1, 1, 1], [0, 1, 1, 0, 0, -1], [0, -2, -2, -3, -3, -5], [0, 3, 2, 2, 2, 1], [0, 1, 1, 1, 0, -1], [0, -1, -1, -2, -2, -3])
            };
            (win_home, draw_home, loss_home, win_away, draw_away, loss_away) = tables;
        }
        let i = band_index;
        match result {
            0 => self.board(coach, if away { draw_away[i] } else { draw_home[i] }),
            1 => self.board(coach, if away { win_away[i] } else { win_home[i] }),
            2 => self.board(coach, if away { loss_away[i] } else { loss_home[i] }),
            _ => {}
        }
        let human = self.world.coach(coach).human;
        let derby = crate::matchday_rivalry(self.world, m);
        match result {
            0 => self.fans(coach, if away { 1 } else { -1 }),
            1 => {
                self.fans(coach, if away { 4 } else { 3 });
                if thrashing {
                    if human && derby && !friendly {
                        messages::send(self.world, coach, 27, 81, String::new(), String::new());
                    }
                    self.fans(coach, if away { 5 } else { 4 });
                }
            }
            2 => {
                self.fans(coach, if away { -4 } else { -5 });
                if thrashing {
                    if derby {
                        if human && !friendly {
                            messages::send(self.world, coach, 6, 46, String::new(), String::new());
                        }
                    } else if !away && human && !friendly {
                        let id = 47 + self.runtime.rng.fresh_int(2);
                        messages::send(self.world, coach, 5, id, String::new(), String::new());
                    }
                    self.fans(coach, if away { -5 } else { -7 });
                }
            }
            _ => {}
        }
        if self.world.coach(coach).fan_support < 20 {
            self.board(coach, -3);
        }
        let c = self.world.coach_mut(coach);
        c.board_confidence = c.board_confidence.clamp(0, 100);
        c.fan_support = c.fan_support.clamp(0, 100);
    }
}
