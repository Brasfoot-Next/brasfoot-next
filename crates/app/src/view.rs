//! What the screens read from the world: plain data for drawing, no text.

use brasfoot_model::ids::{ClubId, CompetitionId, MatchId, PlayerId, StageId};
use brasfoot_model::saved::StageKind;
use brasfoot_model::world::World;

/// A row of a table: the club and `ah.d(f.s)` {points, games, wins, draws, losses, for,
/// against, difference}.
pub struct StandingRow {
    pub club: ClubId,
    pub numbers: [i32; 8],
}

/// The clubs of a league stage in table order (`f.s.yK()`), or per group.
pub fn standings(world: &World, stage: StageId) -> Vec<Vec<StandingRow>> {
    let s = world.stage(stage);
    let row = |c: ClubId| StandingRow { club: c, numbers: brasfoot_simulation::clubs::stage_record_numbers(world, c, stage) };
    if s.groups.is_empty() {
        vec![s.clubs.iter().map(|&c| row(c)).collect()]
    } else {
        s.groups.iter().map(|g| g.clubs.iter().map(|&c| row(c)).collect()).collect()
    }
}

/// The stages of a competition worth showing, main first.
pub fn stages_of(world: &World, c: CompetitionId) -> Vec<StageId> {
    let comp = world.competition(c);
    let mut out = Vec::new();
    for s in [comp.league, comp.second_league, comp.main_stage, comp.knockout, comp.final_stage, comp.playoff, comp.promotion_knockout, comp.relegation_playoff]
        .into_iter()
        .flatten()
    {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// Competitions with something to show (`best.at.a(cat, false, 0)`), leagues first.
pub fn competitions(world: &World) -> Vec<CompetitionId> {
    let mut out: Vec<CompetitionId> = Vec::new();
    for &country in &world.game.league_countries {
        let k = world.country(country);
        out.extend(k.divisions.iter().copied());
        out.extend(k.cup);
    }
    out.extend(world.game.national_super_cups.iter().copied());
    for (i, c) in world.competitions.iter().enumerate() {
        let id = CompetitionId(i as u32);
        if !out.contains(&id) && !stages_of(world, id).is_empty() && world.game.days.iter().any(|d| d.competitions.contains(&id)) {
            out.push(id);
        }
        let _ = c;
    }
    out
}

/// The club's matches this season, in calendar order.
pub fn club_matches(world: &World, club: ClubId) -> Vec<MatchId> {
    world
        .game
        .days
        .iter()
        .flat_map(|d| d.matches.iter().copied())
        .filter(|&m| {
            let g = world.game_match(m);
            g.home == Some(club) || g.away == Some(club)
        })
        .collect()
}

/// `best.f.bl()`: the club's next match not played yet.
pub fn next_match(world: &World, club: ClubId) -> Option<MatchId> {
    club_matches(world, club).into_iter().find(|&m| !world.game_match(m).played)
}

/// The club's main league and place in it.
pub fn league_place(world: &World, club: ClubId) -> Option<(CompetitionId, i32)> {
    for &country in &world.game.league_countries {
        for &league in &world.country(country).divisions {
            if let Some(stage) = world.competition(league).league
                && let Some(i) = world.stage(stage).clubs.iter().position(|&c| c == club)
            {
                return Some((league, i as i32 + 1));
            }
        }
    }
    None
}

/// The top scorers of a competition: player, goals, games (`at.ms()`).
pub fn top_scorers(world: &World, c: CompetitionId, count: usize) -> Vec<(PlayerId, i32, i32)> {
    let mut rows: Vec<(PlayerId, i32, i32)> = world
        .competition(c)
        .scorers
        .iter()
        .filter_map(|s| s.player.map(|p| {
            let games = world.player(p).competition_stats.iter().find(|h| h.competition_id == Some(c)).map(|h| h.games).unwrap_or(s.games);
            (p, s.goals, games)
        }))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));
    rows.truncate(count);
    rows
}

/// The rounds of a knockout stage: each tie's first and second legs.
pub fn knockout_rounds(world: &World, stage: StageId) -> Vec<Vec<(MatchId, Option<MatchId>)>> {
    let s = world.stage(stage);
    if s.class != StageKind::Knockout {
        return Vec::new();
    }
    s.rounds
        .iter()
        .map(|r| r.first_leg.iter().enumerate().map(|(i, &m)| (m, r.second_leg.get(i).copied())).collect())
        .collect()
}

/// The league stage's matches by round, from the calendar.
pub fn league_rounds(world: &World, stage: StageId) -> Vec<Vec<MatchId>> {
    let s = world.stage(stage);
    s.numbers
        .iter()
        .map(|&day| {
            world.game.days.get(day as usize).map(|d| d.matches.iter().copied().filter(|&m| world.game_match(m).stage == Some(stage)).collect()).unwrap_or_default()
        })
        .collect()
}

/// `at.mz()`: the five best average ratings above 5 (player, average, rated games). The original
/// takes five once it has four and fails with exactly four; here four or fewer show nothing.
pub fn best_ratings(world: &World, c: CompetitionId) -> Vec<(PlayerId, f64, i32)> {
    let mut rows: Vec<(PlayerId, f64, i32)> = world
        .game
        .players
        .iter()
        .filter_map(|&p| {
            let h = world.player(p).competition_stats.iter().find(|h| h.competition_id == Some(c))?;
            let average = if h.rated_games > 0 { h.rating_sum / h.rated_games as f64 } else { 0.0 };
            (average > 5.0).then_some((p, average, h.rated_games))
        })
        .collect();
    rows.sort_by(|a, b| if a.1 != b.1 { b.1.total_cmp(&a.1) } else { a.2.cmp(&b.2) });
    if rows.len() < 5 {
        return Vec::new();
    }
    rows.truncate(5);
    rows
}

/// `at.my()`: goals per game, the biggest win, the last champion and the most titles.
pub struct CompetitionHistory {
    pub goals: i32,
    pub games: i32,
    pub biggest_win: Option<MatchId>,
    pub last_champion: Option<ClubId>,
    pub most_titles: Vec<(ClubId, usize)>,
}

pub fn competition_history(world: &World, c: CompetitionId) -> CompetitionHistory {
    let (mut goals, mut games, mut biggest, mut margin) = (0, 0, None, 3);
    for day in world.game.days.iter().filter(|d| d.done) {
        for &m in &day.matches {
            let g = world.game_match(m);
            if g.competition != Some(c) {
                continue;
            }
            games += 1;
            goals += g.home_goals + g.away_goals;
            if g.home_goals - g.away_goals >= margin {
                biggest = Some(m);
                margin = g.home_goals - g.away_goals;
            }
            if g.away_goals - g.home_goals >= margin {
                biggest = Some(m);
                margin = g.away_goals - g.home_goals;
            }
        }
    }
    let titles = &world.competition(c).titles;
    let season = world.season();
    let last = titles.iter().find(|t| t.season == season).or_else(|| titles.iter().find(|t| t.season == season - 1));
    let last_champion = last.and_then(|t| world.club_by_game_id(t.champion));
    let mut most: Vec<(ClubId, usize)> = Vec::new();
    for t in titles {
        if let Some(club) = world.club_by_game_id(t.champion) {
            match most.iter_mut().find(|(c, _)| *c == club) {
                Some((_, n)) => *n += 1,
                None => most.push((club, 1)),
            }
        }
    }
    most.sort_by(|a, b| b.1.cmp(&a.1));
    most.truncate(3);
    CompetitionHistory { goals, games, biggest_win: biggest, last_champion, most_titles: most }
}

/// `f.s.Q(ah)`: the club's place in a table stage, in its group when there are groups; none
/// once the stage has gone on to its knockout.
pub fn stage_place(world: &World, stage: StageId, club: ClubId) -> Option<usize> {
    let s = world.stage(stage);
    if s.class != StageKind::League || s.knockout.is_some() {
        return None;
    }
    if s.group_count == 0 {
        s.clubs.iter().position(|&c| c == club).map(|i| i + 1)
    } else {
        s.groups.iter().find_map(|g| g.clubs.iter().position(|&c| c == club)).map(|i| i + 1)
    }
}

/// The place `a.eg` shows next to a club: only from the second round of a table on (`zb() > 1`).
pub fn shown_place(world: &World, stage: Option<StageId>, club: ClubId) -> Option<usize> {
    let stage = stage?;
    let s = world.stage(stage);
    if s.class != StageKind::League || s.round <= 1 {
        return None;
    }
    stage_place(world, stage, club)
}
