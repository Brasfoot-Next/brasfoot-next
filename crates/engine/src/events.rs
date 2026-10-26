//! A player's numbers as the match records them (`F.a(int, ah, double, at)`, `F.h(at)`).

use brasfoot_model::ids::{ClubId, CompetitionId, PlayerId};
use brasfoot_model::saved::{CompetitionStats, SeasonStats};
use brasfoot_model::world::World;

/// The kinds of `F.a(int, ah, double, at)`.
pub mod record {
    pub const GAME: i32 = 0;
    pub const GOAL: i32 = 1;
    pub const YELLOW: i32 = 2;
    pub const SECOND_YELLOW: i32 = 3;
    pub const RED: i32 = 4;
    pub const INJURY: i32 = 5;
    pub const ASSIST: i32 = 8;
    pub const RATING: i32 = 100;
}

/// `F.h(at)`: the player's numbers in a competition, created on first use.
pub fn competition_stats(world: &mut World, player: PlayerId, competition: Option<CompetitionId>) -> &mut CompetitionStats {
    let stats = &mut world.player_mut(player).competition_stats;
    let index = match stats.iter().position(|h| h.competition_id == competition) {
        Some(i) => i,
        None => {
            stats.push(CompetitionStats { competition_id: competition, ..Default::default() });
            stats.len() - 1
        }
    };
    &mut stats[index]
}

/// `F.o(ah)`: the player's numbers for a club this season, if any.
fn season_index(world: &World, player: PlayerId, club_game_id: i32) -> Option<usize> {
    let season = world.season();
    world.player(player).season_stats.iter().position(|k| k.club_id == club_game_id && k.season == season)
}

/// `F.a(int, ah, double, at)`: one thing to count for a club this season.
pub fn record_season(world: &mut World, player: PlayerId, kind: i32, club: ClubId, value: f64, competition: Option<CompetitionId>) {
    let club_game_id = world.club(club).id;
    let index = match season_index(world, player, club_game_id) {
        Some(i) => i,
        None => {
            let season = world.season();
            let stats = &mut world.player_mut(player).season_stats;
            stats.push(SeasonStats { season, club_id: club_game_id, ..Default::default() });
            stats.len() - 1
        }
    };
    let youth = world.player(player).youth;
    let k = &mut world.player_mut(player).season_stats[index];
    match kind {
        record::YELLOW => k.yellow_cards += 1,
        record::RED => k.red_cards += 1,
        record::SECOND_YELLOW => {
            k.yellow_cards += 1;
            k.red_cards += 1;
        }
        record::GOAL => k.goals += 1,
        record::INJURY => k.injuries += 1,
        record::GAME => k.games += 1,
        record::ASSIST => {
            // k.a(F, at)
            if competition.is_some() && !youth {
                k.assists += 1;
                competition_stats(world, player, competition).assists += 1;
            }
        }
        record::RATING => k.add_rating(value),
        _ => {}
    }
}

/// `F.a(int, ah, at)` with kind 8: an assist.
pub fn record_assist(world: &mut World, player: PlayerId, club: Option<ClubId>, competition: Option<CompetitionId>) {
    if let Some(club) = club {
        record_season(world, player, record::ASSIST, club, 0.0, competition);
    }
}
