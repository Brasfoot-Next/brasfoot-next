// Port of bfmod.Tecnicos: the coach of the year, from his titles and how far above the
// expected place his club finished. The expected place is the club's rank in its league by
// squad strength; the game keeps no past standings, so only the season in progress has one.

use std::collections::HashMap;

use brasfoot_model::ids::{ClubId, CoachId};
use brasfoot_model::mods::{CoachSummary, LeagueCampaign, SeasonTitle};
use brasfoot_model::world::World;

use brasfoot_model::labels::CompetitionLabel;

use super::season::Season;
use crate::economy::is_national_team;

/// `Tecnicos.Campanha`
#[derive(Clone, Debug)]
pub struct Campaign {
    pub club: ClubId,
    pub league: CompetitionLabel,
    /// 1 for the first division.
    pub division: i32,
    pub position: i32,
    pub expected: i32,
    pub clubs: i32,
}

/// `Tecnicos.Candidato`
#[derive(Clone, Debug)]
pub struct CoachCandidate {
    pub coach: CoachId,
    pub club: ClubId,
    pub campaign: Option<Campaign>,
    pub titles: Vec<SeasonTitle>,
    pub points: f64,
}

impl CoachCandidate {
    /// `resumo()`, as data: the campaign and the titles.
    pub fn summary(&self) -> CoachSummary {
        CoachSummary {
            campaign: self.campaign.as_ref().map(|k| LeagueCampaign { league: k.league.clone(), position: k.position, expected: k.expected }),
            titles: self.titles.clone(),
        }
    }
}

/// `Tecnicos.ranking(Temporada)`: the human coaches, then the AI ones, best first.
pub fn ranking(world: &World, season: &Season) -> Vec<CoachCandidate> {
    let mut out = Vec::new();
    if !season.current {
        return out;
    }
    let campaigns = campaigns(world);
    for &coach in world.game.human_coaches.iter().chain(&world.game.coaches) {
        let Some(club) = world.coach(coach).club_id else { continue };
        if is_national_team(world, club) {
            continue;
        }
        let campaign = campaigns.get(&club).cloned();
        let weight = season.weights.get(&club).copied().unwrap_or(1.0);
        let mut points = season.title_points(club);
        if let Some(k) = &campaign {
            let division_factor = if k.division == 1 { 1.0 } else { 0.5 };
            points += (k.expected - k.position).clamp(0, 10) as f64 * 1.2 * weight * division_factor;
            points += (k.clubs - k.position + 1) as f64 / k.clubs as f64 * 5.0 * weight * division_factor;
        }
        out.push(CoachCandidate { coach, club, campaign, titles: season.titles(club), points });
    }
    out.sort_by(|a, b| b.points.total_cmp(&a.points));
    out
}

/// `Tecnicos.campanhas()`: the final and the expected place of every club of a national league.
pub fn campaigns(world: &World) -> HashMap<ClubId, Campaign> {
    let mut out = HashMap::new();
    for &country in &world.game.league_countries {
        for (d, &league) in world.country(country).divisions.iter().enumerate() {
            let Some(stage) = world.competition(league).league else { continue };
            let table = &world.stage(stage).clubs;
            if table.is_empty() {
                continue;
            }
            let strength: HashMap<ClubId, f64> = table.iter().map(|&c| (c, squad_strength(world, c))).collect();
            let mut by_strength = table.clone();
            by_strength.sort_by(|a, b| strength[b].total_cmp(&strength[a]));
            let name = world.competition_label(league);
            for (i, &club) in table.iter().enumerate() {
                let expected = by_strength.iter().position(|&c| c == club).expect("the club is in its table") as i32 + 1;
                let campaign =
                    Campaign { club, league: name.clone(), division: d as i32 + 1, position: i as i32 + 1, expected, clubs: table.len() as i32 };
                out.insert(club, campaign);
            }
        }
    }
    out
}

/// `forcaDoElenco(ah)`: the average strength of the 16 best players.
fn squad_strength(world: &World, club: ClubId) -> f64 {
    let mut strengths: Vec<i32> = world.club(club).squad.iter().map(|&p| world.player(p).strength).collect();
    strengths.sort_by(|a, b| b.cmp(a));
    let n = strengths.len().min(16);
    let sum = strengths[..n].iter().fold(0.0, |s, &x| s + x as f64);
    if n > 0 { sum / n as f64 } else { 0.0 }
}
