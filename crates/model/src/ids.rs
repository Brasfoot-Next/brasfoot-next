//! Typed indexes into the arenas of [`crate::game::Game`]. The original holds Java references;
//! shared objects (players, clubs, coaches, stadiums, matches, competitions and their stages)
//! become arena entries and every reference becomes one of these ids.

use serde::{Deserialize, Serialize};

macro_rules! id {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(pub u32);

        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    )*};
}

id!(
    /// A `best.F`.
    PlayerId,
    /// A `best.ah`.
    ClubId,
    /// A `best.al`.
    CoachId,
    /// A `best.v`.
    StadiumId,
    /// A `best.I`.
    MatchId,
    /// A `best.at` (the `f.*` competitions).
    CompetitionId,
    /// A `best.L` (`f.s`, `f.z`): a stage of a competition.
    StageId,
    /// A `best.Z`: a country with its leagues, cup and national team.
    CountryId,
    /// A `best.w`: a Brazilian state championship.
    StateLeagueId,
);
