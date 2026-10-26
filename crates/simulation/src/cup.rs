// Port of the national cup's new format (f.f with phase code 2029): each round's winners are
// drawn into the next one by f.f.ad, ao, as, at, au and av.

use brasfoot_model::ids::{ClubId, CompetitionId};

use crate::Sim;

impl Sim<'_> {
    /// The dispatch of `f.G.b(boolean, boolean)` for the cup's rounds (format 2129 to 2629).
    /// The draws sort or shuffle the previous round's own list of winners (`aak`), as saved.
    pub fn cup_next_phase(&mut self, cup: CompetitionId, format: i32, previous: usize) {
        match format {
            2129 => self.cup_second_round(cup, previous),
            2229 => self.cup_round_of_32(cup, previous),
            2329 => self.cup_draw_round(cup, previous, 3, 2429),
            2429 => self.cup_draw_round(cup, previous, 4, 2529),
            2529 => self.cup_draw_round(cup, previous, 5, 2629),
            2629 => self.cup_draw_round(cup, previous, 6, 2729),
            _ => {}
        }
    }

    fn winners_of(&mut self, cup: CompetitionId, round: usize) -> &mut Vec<ClubId> {
        let stage = self.world.competition(cup).knockout.expect("the cup's knockout");
        &mut self.world.stage_mut(stage).rounds[round].winners
    }

    /// `G.a(...)` followed by `fb(format)` and `z.BH()`.
    fn cup_round(&mut self, cup: CompetitionId, clubs: &[ClubId], round: i32, two_legged: bool, format: i32) {
        let stage = self.world.competition(cup).knockout.expect("the cup's knockout");
        self.new_knockout_round(stage, clubs, round, two_legged, 0, 2, false);
        let s = self.world.stage_mut(stage);
        s.rounds.last_mut().expect("the new round").format_code = format;
        s.round += 1;
        s.leg = 1;
    }

    /// `f.f.ad(ArrayList)`: the 40 winners, the 20 strongest drawn against the 20 others.
    fn cup_second_round(&mut self, cup: CompetitionId, previous: usize) {
        let mut winners = self.winners_of(cup, previous).clone();
        winners.sort_by(|&a, &b| crate::sorting::by_level_and_reputation(self.world, a, b));
        *self.winners_of(cup, previous) = winners.clone();
        let mut top: Vec<ClubId> = winners[..=19].to_vec();
        let mut bottom: Vec<ClubId> = winners[20..=39].to_vec();
        self.runtime.rng.shuffle(&mut top);
        self.runtime.rng.shuffle(&mut bottom);
        let pairs: Vec<ClubId> = top.iter().zip(&bottom).flat_map(|(&t, &b)| [b, t]).collect();
        self.cup_round(cup, &pairs, 1, false, 2229);
    }

    /// `f.f.ao(ArrayList)`: the 20 winners and the 12 clubs that skipped the first rounds.
    fn cup_round_of_32(&mut self, cup: CompetitionId, previous: usize) {
        let mut clubs = self.winners_of(cup, previous).clone();
        clubs.extend(self.world.competition(cup).cup_clubs.clone());
        if clubs.len() != 32 {
            return;
        }
        clubs.sort_by(|&a, &b| crate::sorting::by_level_and_reputation(self.world, a, b));
        let mut top: Vec<ClubId> = clubs[..=15].to_vec();
        let mut bottom: Vec<ClubId> = clubs[16..=31].to_vec();
        self.runtime.rng.shuffle(&mut top);
        self.runtime.rng.shuffle(&mut bottom);
        let pairs: Vec<ClubId> = top.iter().zip(&bottom).flat_map(|(&t, &b)| [t, b]).collect();
        self.cup_round(cup, &pairs, 2, true, 2329);
    }

    /// `f.f.as`, `at`, `au`, `av`: an open draw among the winners.
    fn cup_draw_round(&mut self, cup: CompetitionId, previous: usize, round: i32, format: i32) {
        let mut winners = self.winners_of(cup, previous).clone();
        self.runtime.rng.shuffle(&mut winners);
        *self.winners_of(cup, previous) = winners.clone();
        self.cup_round(cup, &winners, round, true, format);
    }
}
