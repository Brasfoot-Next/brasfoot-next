// Port of the second tiers of the continental cups: the Europa League (f.x), the Conference
// League (f.J) and the Copa Sudamericana (f.H), from their qualifying rounds to the final
// knockout.

use brasfoot_model::ids::{ClubId, CompetitionId, StageId};
use brasfoot_model::rng::JavaRandom;

use crate::Sim;
use crate::continental::{Tie, separate_countries};
use crate::sorting::by_level_and_reputation;
use crate::stages::StageOptions;

/// `n` ties of the two pots (`aq.e`), each one the way `new Random().nextBoolean()` turns it:
/// true starts from the first pot.
fn oriented_ties(sim: &Sim, r: &mut JavaRandom, n: usize, a: &mut Vec<ClubId>, b: &mut Vec<ClubId>) -> Vec<Tie> {
    let mut ties = vec![Tie::default(); n];
    for t in ties.iter_mut() {
        if r.next_boolean() {
            t.two(sim, a, b);
        } else {
            t.two(sim, b, a);
        }
    }
    ties
}

fn flatten(ties: &[Tie]) -> Vec<ClubId> {
    ties.iter().flat_map(|t| t.0.clone()).collect()
}

/// The group thirds (`gR().get(2)`) of a cup's group stage.
fn group_places(sim: &Sim, cup: Option<CompetitionId>, place: usize) -> Vec<ClubId> {
    let Some(main) = cup.and_then(|c| sim.world.competition(c).main_stage) else { return Vec::new() };
    sim.world.stage(main).groups.iter().map(|g| g.clubs[place]).collect()
}

/// Which second tier a function works for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tier {
    Europa,
    Conference,
}

impl Tier {
    /// The codes of its playoff round and its group stage.
    fn codes(self) -> (i32, i32) {
        match self {
            Tier::Europa => (6006, 6005),
            Tier::Conference => (12006, 12005),
        }
    }
}

impl Sim<'_> {
    /// `f.x.g(ArrayList, ArrayList)` (the end of 6001): the losers to the Conference League; seven
    /// qualifiers, six of the Champions League's losers and the winners, shuffled, in ten ties
    /// (6002).
    pub(crate) fn europa_league_second_round(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 6001);
        }
        self.send_to_conference_league(losers);
        let c = self.world.competition(cup);
        let mut clubs: Vec<ClubId> = c.clubs_ys[0..=6].to_vec();
        clubs.extend_from_slice(&c.clubs_ze[0..=5]);
        clubs.extend_from_slice(winners);
        self.runtime.rng.shuffle(&mut clubs);
        let mut a = clubs[0..=9].to_vec();
        let mut b = clubs[10..=19].to_vec();
        let mut r = self.runtime.rng.new_random();
        let ties = oriented_ties(self, &mut r, 10, &mut a, &mut b);
        let stage = self.cup_knockout(cup, &flatten(&ties), 1, vec![true; 7], 6002);
        self.world.competition_mut(cup).qualifying_round_2 = Some(stage);
    }

    /// `m(ArrayList, ArrayList)` of f.x (the end of 6002 and 6004) and f.J (12004): the losers
    /// placed (f.x files them under 6004 either way) and, for the Europa League, to the
    /// Conference League; the clubs that came down from the cup above, the winners and the
    /// cup's own places in eight groups of four by level.
    fn second_tier_groups(&mut self, cup: CompetitionId, tier: Tier, winners: &[ClubId], losers: &[ClubId]) {
        let (placed_as, from) = match tier {
            Tier::Europa => (6004, 6..=15),
            Tier::Conference => (12004, 25..=34),
        };
        for &l in losers {
            self.place(l, cup, -1, placed_as);
        }
        if tier == Tier::Europa {
            self.send_to_conference_league(losers);
        }
        let c = self.world.competition(cup);
        let mut clubs: Vec<ClubId> = c.clubs_ze[from].to_vec();
        clubs.extend_from_slice(winners);
        clubs.extend_from_slice(&c.clubs);
        clubs.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        let mut pots: Vec<Vec<ClubId>> = clubs.chunks(8).take(4).map(|p| p.to_vec()).collect();
        for pot in pots.iter_mut() {
            self.runtime.rng.shuffle(pot);
        }
        let [p1, p2, p3, p4] = &mut pots[..] else { unreachable!("four pots") };
        let mut groups = vec![Tie::default(); 8];
        for g in groups.iter_mut() {
            g.four(self, [p1, p2, p3, p4]);
        }
        separate_countries(self, &mut groups);
        let drawn = flatten(&groups);
        if drawn.len() == 32 {
            self.second_tier_group_stage(cup, &drawn, 2, vec![true, true, true, true, false, false, false], tier.codes().1);
        }
    }

    /// `new s(options, clubs, 0, ..., cup)` with `fb(format)` and the cup's name: the group stage
    /// of 32 clubs in eight groups.
    fn second_tier_group_stage(&mut self, cup: CompetitionId, drawn: &[ClubId], knockout_teams: i32, legs: Vec<bool>, format: i32) {
        let options = StageOptions {
            teams: 32,
            groups: 8,
            knockout_teams,
            two_rounds: true,
            skip_two_dates: false,
            two_legged_knockout: legs,
            ..Default::default()
        };
        let kind = self.world.competition(cup).kind;
        let stage = self.new_league_stage(&options, drawn, 0, kind, cup, true);
        self.world.stage_mut(stage).format_code = format;
        self.world.stage_mut(stage).name = self.world.competition(cup).name.clone();
        self.world.competition_mut(cup).main_stage = Some(stage);
    }

    /// `Be()` of f.x (6005) and f.J (12005): the group seconds against the thirds of the cup
    /// above, from other countries, in the playoff round of the group stage (6006, 12006).
    pub(crate) fn second_tier_playoffs(&mut self, cup: CompetitionId) {
        let tier = self.tier_of(cup);
        let above = match tier {
            Tier::Europa => self.world.game.champions_league,
            Tier::Conference => self.world.game.europa_league,
        };
        let mut seconds = group_places(self, Some(cup), 1);
        let mut thirds = group_places(self, above, 2);
        self.runtime.rng.shuffle(&mut seconds);
        self.runtime.rng.shuffle(&mut thirds);
        let mut ties = vec![Tie::default(); 8];
        for t in ties.iter_mut() {
            t.two(self, &mut seconds, &mut thirds);
        }
        separate_countries(self, &mut ties);
        let drawn = flatten(&ties);
        let main = self.world.competition(cup).main_stage.expect("the cup's groups");
        let kind = self.world.competition(cup).kind;
        let stage = self.new_knockout_stage(cup, Some(main), drawn.len() as i32, kind, 1, vec![true, true, true, false, false, false, false], tier.codes().0);
        self.world.stage_mut(main).knockout = Some(stage);
        self.new_knockout_round(stage, &drawn, 0, true, 0, kind, false);
    }

    /// `l(ArrayList, ArrayList)` of f.x (6006) and f.J (12006): the group winners against the
    /// playoff winners, from other countries, in the final knockout (`afT`).
    pub(crate) fn second_tier_final_knockout(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        let tier = self.tier_of(cup);
        for &l in losers {
            self.place(l, cup, -1, tier.codes().0);
        }
        let mut firsts = group_places(self, Some(cup), 0);
        let mut winners = winners.to_vec();
        self.runtime.rng.shuffle(&mut firsts);
        self.runtime.rng.shuffle(&mut winners);
        let mut ties = vec![Tie::default(); 8];
        for t in ties.iter_mut() {
            t.two(self, &mut firsts, &mut winners);
        }
        separate_countries(self, &mut ties);
        let stage = self.cup_knockout(cup, &flatten(&ties), 0, vec![true, true, true, false, false, false, false], -1);
        self.world.competition_mut(cup).playoff_round = Some(stage);
    }

    fn tier_of(&self, cup: CompetitionId) -> Tier {
        if Some(cup) == self.world.game.conference_league { Tier::Conference } else { Tier::Europa }
    }

    /// `f.J.g(ArrayList, ArrayList)` (the end of 12001): sixteen of the clubs that came down in
    /// eight ties among them; 45 qualifiers against fifteen more and the winners (12002).
    pub(crate) fn conference_league_second_round(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 12001);
        }
        let c = self.world.competition(cup);
        let mut a = c.clubs_ze[0..=7].to_vec();
        let mut b = c.clubs_ze[8..=15].to_vec();
        let mut d = c.clubs_ys[14..=58].to_vec();
        let mut e = c.clubs_ys[59..=73].to_vec();
        e.extend_from_slice(winners);
        for pot in [&mut a, &mut b, &mut d, &mut e] {
            self.runtime.rng.shuffle(pot);
        }
        let mut r = self.runtime.rng.new_random();
        let mut ties = oriented_ties(self, &mut r, 8, &mut a, &mut b);
        ties.extend(oriented_ties(self, &mut r, 45, &mut d, &mut e));
        let stage = self.cup_knockout(cup, &flatten(&ties), 1, vec![true, false, false, false, false, false, false], 12002);
        self.world.competition_mut(cup).qualifying_round_2 = Some(stage);
    }

    /// `f.J.h(ArrayList, ArrayList)` (the end of 12002): five ties of the best winners against
    /// three more and two clubs that came down; 27 more (12003).
    pub(crate) fn conference_league_third_round(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 12002);
        }
        let c = self.world.competition(cup);
        let mut a = winners[0..=4].to_vec();
        let mut b = winners[5..=7].to_vec();
        b.extend_from_slice(&c.clubs_ze[16..=17]);
        let mut d = c.clubs_ys[5..=13].to_vec();
        d.extend_from_slice(&winners[8..=25]);
        let mut e = winners[26..=52].to_vec();
        for pot in [&mut a, &mut b, &mut d, &mut e] {
            self.runtime.rng.shuffle(pot);
        }
        let mut r = self.runtime.rng.new_random();
        let mut ties = oriented_ties(self, &mut r, 5, &mut a, &mut b);
        ties.extend(oriented_ties(self, &mut r, 27, &mut d, &mut e));
        let stage = self.cup_knockout(cup, &flatten(&ties), 1, vec![true, false, false, false, false, false, false], 12003);
        self.world.competition_mut(cup).qualifying_round_3 = Some(stage);
    }

    /// `f.J.j(ArrayList, ArrayList)` (the end of 12003): the play-offs, with the first five
    /// qualifiers and seven more clubs that came down (12004).
    pub(crate) fn conference_league_playoffs(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 12003);
        }
        let c = self.world.competition(cup);
        let mut a = winners[0..=4].to_vec();
        let mut b = c.clubs_ze[18..=22].to_vec();
        let mut d = c.clubs_ys[0..=4].to_vec();
        d.extend_from_slice(&winners[5..=16]);
        let mut e = winners[17..=31].to_vec();
        e.extend_from_slice(&c.clubs_ze[23..=24]);
        for pot in [&mut a, &mut b, &mut d, &mut e] {
            self.runtime.rng.shuffle(pot);
        }
        let mut r = self.runtime.rng.new_random();
        let mut ties = oriented_ties(self, &mut r, 5, &mut a, &mut b);
        ties.extend(oriented_ties(self, &mut r, 17, &mut d, &mut e));
        let stage = self.cup_knockout(cup, &flatten(&ties), 1, vec![true, false, false, false, false, false, false], 12004);
        self.world.competition_mut(cup).qualifying_round_4 = Some(stage);
    }

    /// `f.x.m` and `f.J.m`.
    pub(crate) fn europa_league_groups(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        self.second_tier_groups(cup, Tier::Europa, winners, losers);
    }

    pub(crate) fn conference_league_groups(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        self.second_tier_groups(cup, Tier::Conference, winners, losers);
    }

    /// `f.H.o(ArrayList, ArrayList)` (the end of 6100): the Libertadores' qualifiers that came
    /// down, the winners and the cup's own places in eight groups of four by level, the group
    /// winners on (6105).
    pub(crate) fn sudamericana_groups(&mut self, cup: CompetitionId, winners: &[ClubId], losers: &[ClubId]) {
        for &l in losers {
            self.place(l, cup, -1, 6100);
        }
        let c = self.world.competition(cup);
        let mut clubs: Vec<ClubId> = c.clubs_aao.clone();
        clubs.extend_from_slice(winners);
        clubs.extend_from_slice(&c.clubs);
        clubs.sort_by(|&a, &b| by_level_and_reputation(self.world, a, b));
        // f.H.b(ArrayList, int, int): each pot shuffled as it is taken
        let mut pots: Vec<Vec<ClubId>> = Vec::new();
        for p in 0..4 {
            let mut pot = clubs[p * 8..p * 8 + 8].to_vec();
            self.runtime.rng.shuffle(&mut pot);
            pots.push(pot);
        }
        let [p1, p2, p3, p4] = &mut pots[..] else { unreachable!("four pots") };
        let mut groups = vec![Tie::default(); 8];
        for g in groups.iter_mut() {
            g.four(self, [p1, p2, p3, p4]);
        }
        separate_countries(self, &mut groups);
        let drawn = flatten(&groups);
        if drawn.len() == 32 {
            self.second_tier_group_stage(cup, &drawn, 1, vec![true, true, true, false, false, false, false], 6105);
        }
    }

    /// `f.H.yt()` (the end of 6105): the group winners against the Libertadores' group thirds, as
    /// they come, in the knockout of the group stage.
    pub(crate) fn sudamericana_knockout(&mut self, cup: CompetitionId) {
        let mut firsts = group_places(self, Some(cup), 0);
        let mut thirds = group_places(self, self.world.game.libertadores, 2);
        self.runtime.rng.shuffle(&mut firsts);
        self.runtime.rng.shuffle(&mut thirds);
        let mut ties = vec![Tie::default(); 8];
        for t in ties.iter_mut() {
            t.two_plain(&mut firsts, &mut thirds);
        }
        let drawn = flatten(&ties);
        let main = self.world.competition(cup).main_stage.expect("the cup's groups");
        let kind = self.world.competition(cup).kind;
        let stage = self.new_knockout_stage(cup, Some(main), drawn.len() as i32, kind, 0, vec![true, true, true, false, false, false, false], -1);
        self.world.stage_mut(main).knockout = Some(stage);
        self.new_knockout_round(stage, &drawn, 0, true, 0, kind, false);
    }

    /// `f.s.f(z)`, the `mr()` of f.x and f.J: the final places from the final knockout's last
    /// round back, the winner of each tie first; then the rest of the group stage's table but
    /// the clubs of its playoff round (`Zc`), whose losers keep their place there.
    pub(crate) fn places_after_final_knockout(&mut self, stage: StageId, knockout: StageId) {
        let competition = self.world.stage(stage).competition.expect("a stage belongs to a competition");
        let mut placed: Vec<ClubId> = Vec::new();
        let mut position = 1;
        let rounds = self.world.stage(knockout).rounds.clone();
        for (depth, r) in rounds.iter().rev().enumerate() {
            for &m in &r.first_leg {
                let g = self.world.game_match(m);
                let (home, away) = (g.home.expect("home club"), g.away.expect("away club"));
                let pair = if r.winners.contains(&home) {
                    [home, away]
                } else if r.winners.contains(&away) {
                    [away, home]
                } else {
                    crate::unported!("f.s.f(z) with a tie neither club won");
                };
                for club in pair {
                    if !placed.contains(&club) {
                        placed.push(club);
                        self.place(club, competition, position, depth as i32);
                        position += 1;
                    }
                }
            }
        }
        let playoff = self.world.stage(stage).knockout.expect("the group stage's playoff round");
        let mut first_round = Vec::new();
        for &m in &self.world.stage(playoff).rounds[0].first_leg {
            let g = self.world.game_match(m);
            first_round.extend(g.home);
            first_round.extend(g.away);
        }
        let kind = self.world.competition(competition).kind;
        let rest_depth = if matches!(kind, 4 | 6 | 12) { 1001 } else { -1 };
        for club in self.world.stage(stage).clubs.clone() {
            if !placed.contains(&club) && !first_round.contains(&club) {
                self.place(club, competition, position, rest_depth);
                position += 1;
            }
        }
    }
}
