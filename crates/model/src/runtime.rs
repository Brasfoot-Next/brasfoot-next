//! State the original keeps outside the save: `transient` fields and static fields. It starts
//! empty when a career is loaded, as in the game.

use std::collections::HashMap;

use crate::clock::Clock;
use crate::ids::{CompetitionId, MatchId, PlayerId};
use crate::names::NameLists;
use crate::rng::{JavaRandom, RandomSource};

/// Port of `components.s` (`F.gB()`, transient `F.fo`): what a player did in the match being
/// played. The engine renews it for everyone in the line-ups at kick-off (`F.fe()`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerMatchStats {
    /// `Pd` (`tu()`): shots, goals included (`tB()`, `z()`).
    pub shots: i32,
    /// `Pe` (`tK()`, `tL()`): shots on target the keeper stopped.
    pub shots_saved: i32,
    /// `Pf` (`tv()`, `tC()`)
    pub count_pf: i32,
    /// `Pg` (`tw()`, `tD()`)
    pub count_pg: i32,
    /// `V` (`y()`, `z()`)
    pub goals: i32,
    /// `Ph` (`tx()`, `tE()`)
    pub own_goals: i32,
    /// `Pi` (`ty()`, `tF()`)
    pub count_pi: i32,
    /// `Pj` (`tz()`, `tG()`)
    pub yellow_cards: i32,
    /// `Pk` (`tA()`, `tH()`)
    pub red_cards: i32,
    /// `cc` (`cD()`, `gV()`)
    pub assists: i32,
    /// `Pl` (`tI()`, `tJ()`)
    pub count_pl: i32,
}

impl PlayerMatchStats {
    /// `s.z()`: a goal is also a shot.
    pub fn add_goal(&mut self) {
        self.goals += 1;
        self.shots += 1;
    }
}

/// Port of `best.Y`: a rating of the round, for the team of the round (`f.B.zH`).
#[derive(Clone, Debug, PartialEq)]
pub struct RoundRating {
    /// `U`
    pub player: PlayerId,
    /// `Z`
    pub rating: f64,
    /// `hq`: the slot played.
    pub slot: i32,
    /// `hr`: `new Random().nextInt(100)` at creation, breaks ties.
    pub tiebreak: i32,
}

/// A human coach sacked (`S.a(al, al, ah, int)`): his successor and the reason (`aq.sg`).
#[derive(Clone, Debug, PartialEq)]
pub struct Sacking {
    pub coach: crate::ids::CoachId,
    pub successor: Option<crate::ids::CoachId>,
    pub club: crate::ids::ClubId,
    pub reason: i32,
}

/// The game's static lists of candidate minutes for AI substitutions (`I.gs`..`I.gw`), created
/// on first use and reshuffled in place match after match.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MinutePools {
    pub pools: [Option<Vec<i32>>; 5],
}

#[derive(Debug, Default)]
pub struct Runtime {
    pub rng: RandomSource,
    /// What `Calendar.getInstance()` reads: the clock and the time zone.
    pub clock: Clock,
    /// The game's name lists, for the players it creates.
    pub names: NameLists,
    /// `f.bA`: the youth players created by the season's intake, joining the youth list later.
    pub youth_intake: Vec<PlayerId>,
    /// `aq.tI`, the game's shared `Random` (`aq.cu(int)`).
    pub shared_random: Option<JavaRandom>,
    /// `Taticas.SORTE`.
    pub tactics_random: Option<JavaRandom>,
    /// `F.fo` of every player.
    pub match_stats: HashMap<PlayerId, PlayerMatchStats>,
    /// `F.fg` (`gi()`, `gj()`, `aw(int)`): yellow cards in the match being played.
    pub match_cards: HashMap<PlayerId, i32>,
    /// `F.fp` (`gC()`, `ay(int)`)
    pub call_ups: HashMap<PlayerId, i32>,
    pub minute_pools: MinutePools,
    /// `I.go`: the engine of each match in play, kept between ticks (human matches).
    pub engines_in_play: Vec<MatchId>,
    /// `f.B.ZZ`: each league's ratings of the current round.
    pub round_ratings: HashMap<CompetitionId, Vec<RoundRating>>,
    /// Human coaches sacked, for the screen that tells them (`a.dm`).
    pub sackings: Vec<Sacking>,
    /// `I.gr`: the ground of a match of a national teams' tournament, made at kick-off with the
    /// host's capacity (`at.cy(int)`); transient, the save does not keep it.
    pub venues: HashMap<MatchId, crate::saved::Stadium>,
    /// `Z.hD`: each country's clubs in the order its places in the continental cups go
    /// (`Z.jd()`, made on first use, cleared by `Z.jy()`); transient.
    pub country_rankings: HashMap<crate::ids::CountryId, Vec<crate::ids::ClubId>>,
    /// `f.bw` (`f.cb()` on load): the clubs' stadiums, biggest first; transient, made on first
    /// use here.
    pub stadiums_by_size: Option<Vec<crate::ids::StadiumId>>,
    /// `Z.hE`: each country's six biggest stadiums, for its national team's matches.
    pub national_grounds: HashMap<crate::ids::CountryId, Vec<crate::ids::StadiumId>>,
    /// `f.s.Zl`: the stage whose table `h.a.abn` reads, the last one a round ended in (`f.s.pO`)
    /// or a qualifiers' draw picked (`s.r`); static and transient, so none after a load (and
    /// then that comparator leaves a list as it is).
    pub table_stage: Option<crate::ids::StageId>,
    /// Differential debugging: print the seed counter at each [`Runtime::checkpoint`]
    /// (`BF_TRACE_RNG` in the tests; the Java harnesses print the same labels).
    pub trace_seeds: bool,
}

impl Runtime {
    /// The statics are created in the order the Java harness re-creates them after it resets
    /// the seed counter: `aq.tI`, then `Taticas.SORTE`.
    pub fn new(mut rng: RandomSource) -> Self {
        let shared_random = Some(rng.new_random());
        let tactics_random = Some(rng.new_random());
        Runtime { rng, shared_random, tactics_random, ..Default::default() }
    }

    /// `aq.cu(int)`
    pub fn shared_int(&mut self, bound: i32) -> i32 {
        if self.shared_random.is_none() {
            self.shared_random = Some(self.rng.new_random());
        }
        self.shared_random.as_mut().unwrap().next_int_bounded(bound)
    }

    /// With [`Runtime::trace_seeds`], prints a label and the next seed of the counter.
    pub fn checkpoint(&self, label: &str) {
        if self.trace_seeds {
            eprintln!("rng {label} {} draws {}", self.rng.next_seed().unwrap_or(-1), crate::rng::DRAWS.load(std::sync::atomic::Ordering::Relaxed));
        }
    }

    /// `F.gB()`: created on first use.
    pub fn stats(&mut self, player: PlayerId) -> &mut PlayerMatchStats {
        self.match_stats.entry(player).or_default()
    }

    /// `F.fe()`: fresh numbers for a new match.
    pub fn reset_stats(&mut self, player: PlayerId) {
        self.match_stats.insert(player, PlayerMatchStats::default());
    }
}
