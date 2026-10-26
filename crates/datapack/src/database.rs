//! A stack of sources made into one database, checked as a whole, and handed to the game as the
//! team files and league configurations a new game starts from.

use std::collections::BTreeMap;

use brasfoot_model::templates::{LeagueConfig, PlayerTemplate, StateLeagueConfig, TeamTemplate};

use crate::codes;
use crate::format::*;
use crate::issues::{Issue, IssueKind};
use crate::read::{Content, read_content};
use crate::tree::{FileTree, LayeredTree};

/// One layer: a source's manifest and its files.
pub struct Layer<'a> {
    pub manifest: &'a Manifest,
    pub tree: &'a dyn FileTree,
}

pub struct Database {
    pub content: Content,
    /// Which layer each file comes from.
    pub owner: BTreeMap<String, usize>,
}

impl Database {
    /// The layer that provides a file (a picture, say), to read it from that source.
    pub fn layer_of(&self, path: &str) -> Option<usize> {
        self.owner.get(path).copied()
    }
}

fn removes(manifest: &Manifest, path: &str) -> bool {
    let r = &manifest.remove;
    r.clubs.iter().any(|id| path.starts_with(&format!("clubs/{id}/")))
        || r.leagues.iter().any(|c| path == format!("leagues/{}.toml", c.to_lowercase()))
        || r.state_leagues.iter().any(|s| path == format!("state-leagues/{}.toml", s.to_lowercase()))
}

/// The layers, bottom first, as one database. The first layer must be a base.
pub fn compose(layers: &[Layer]) -> (Database, Vec<Issue>) {
    let mut issues = Vec::new();
    let manifest_issue = |kind| Issue { path: MANIFEST.to_owned(), line: None, kind };
    match layers.first() {
        Some(first) if first.manifest.kind == SourceKind::Patch => issues.push(manifest_issue(IssueKind::PatchWithoutBase)),
        None => issues.push(manifest_issue(IssueKind::MissingManifest)),
        _ => {}
    }
    // removals of things no layer below has
    let files: Vec<Vec<String>> = layers.iter().map(|l| l.tree.files()).collect();
    for (i, layer) in layers.iter().enumerate() {
        let r = &layer.manifest.remove;
        let targets = r
            .clubs
            .iter()
            .map(|id| (id.clone(), format!("clubs/{id}/")))
            .chain(r.leagues.iter().map(|c| (c.clone(), format!("leagues/{}.toml", c.to_lowercase()))))
            .chain(r.state_leagues.iter().map(|s| (s.clone(), format!("state-leagues/{}.toml", s.to_lowercase()))));
        for (id, prefix) in targets {
            if !files[..i].iter().flatten().any(|p| p.starts_with(&prefix)) {
                issues.push(manifest_issue(IssueKind::UnknownRemoval { id }));
            }
        }
    }
    let stacked = LayeredTree::new(layers.iter().map(|l| l.tree).collect(), |i, path| removes(layers[i].manifest, path));
    let content = read_content(&stacked, &mut issues);
    let owner: BTreeMap<String, usize> = stacked.files().into_iter().filter_map(|p| stacked.layer_of(&p).map(|l| (p, l))).collect();
    let patches: Vec<bool> = layers.iter().map(|l| l.manifest.kind == SourceKind::Patch).collect();
    check_complete(&content, &owner, &patches, &mut issues);
    (Database { content, owner }, issues)
}

/// What a whole database needs beyond each file being right.
fn check_complete(content: &Content, owner: &BTreeMap<String, usize>, patches: &[bool], issues: &mut Vec<Issue>) {
    for club in content.clubs.values() {
        let folder = format!("clubs/{}/", club.id);
        let mut issue = |file: &str, kind| issues.push(Issue { path: format!("{folder}{file}"), line: None, kind });
        let Some(data) = &club.data else {
            let from_patch = owner.range(folder.clone()..).take_while(|(p, _)| p.starts_with(&folder)).any(|(_, &l)| patches[l]);
            issue(CLUB, if from_patch { IssueKind::UnknownClub } else { IssueKind::MissingClubFile });
            continue;
        };
        if club.squad.is_none() {
            issue(SQUAD, IssueKind::MissingSquad);
        }
        if !club.pictures.contains(CREST) {
            issue(CREST, IssueKind::MissingCrest);
        }
        for rival in &data.rivals {
            if content.clubs.get(rival).is_none_or(|c| c.data.is_none()) {
                issue(CLUB, IssueKind::UnknownRival { id: rival.clone() });
            }
        }
    }
}

/// The clubs as the game's team files, in id order, and the league configurations. Clubs
/// without `club.toml` are left out; unknown codes become the values the game falls back to.
pub fn to_templates(content: &Content) -> (Vec<TeamTemplate>, Vec<LeagueConfig>, Vec<StateLeagueConfig>) {
    let country = |code: &str| codes::country_id(code).unwrap_or(-1);
    let teams = content
        .clubs
        .values()
        .filter_map(|club| {
            let data = club.data.as_ref()?;
            let state = match &data.state {
                Some(s) => codes::state_id(s).unwrap_or(-1),
                // a team file of another country holds 0
                None if data.country == "BRA" => -1,
                None => 0,
            };
            let players = |rows: &Option<Vec<PlayerRow>>| rows.iter().flatten().map(player_template).collect();
            Some(TeamTemplate {
                file_ref: club.id.clone(),
                valid: true,
                id: 0,
                country: country(&data.country),
                state,
                level: data.level,
                name: data.name.clone(),
                stadium: data.stadium.name.clone(),
                capacity: data.stadium.capacity,
                coach: data.coach.name.clone(),
                coach_country: country(&data.coach.country),
                players: players(&club.squad),
                youth: players(&club.youth),
                reputation: data.reputation,
                vid: 185,
                color1: data.colors[0].clone(),
                color2: data.colors[1].clone(),
                rivals: data.rivals.clone(),
                ..Default::default()
            })
        })
        .collect();
    let leagues = content
        .leagues
        .values()
        .flat_map(|league| {
            league.divisions.iter().map(|d| LeagueConfig {
                country: country(&league.country),
                division: d.division,
                formula: d.round_count,
                tiebreak: d.tiebreak,
                teams: d.teams,
                two_rounds: d.two_rounds,
                groups: d.groups,
                games_within_group: d.games_within_group,
                relegated_by_group: d.relegated_by_group,
                knockout_teams: d.knockout_teams,
                valid: true,
                relegated: d.relegated,
                two_legged_knockout: d.two_legged_knockout.clone(),
                qualify_by_overall_table: d.qualify_by_overall_table,
                best_third_places: d.best_third_places,
                name: d.name.clone(),
                division_name: d.division_name.clone(),
                relegation_playoff: d.relegation_playoff,
                two_legged_relegation_playoff: d.two_legged_relegation_playoff.clone(),
                two_legged_promotion_knockout: d.two_legged_promotion_knockout.clone(),
                promotion_places: d.promotion_places,
                relegated_directly: d.relegated_directly,
                // the version whose relegated_directly the game honours
                file_version: 22,
            })
        })
        .collect();
    let state_leagues = content
        .state_leagues
        .values()
        .flat_map(|league| {
            league.divisions.iter().map(|d| StateLeagueConfig {
                state: codes::state_id(&league.state).unwrap_or(-1),
                division: d.division,
                formula: codes::state_format_id(&d.format).unwrap_or(0),
                relegated: d.relegated,
                tiebreak: d.tiebreak,
                two_legged_finals: d.two_legged_finals.map(|two| if two { 2 } else { 1 }),
            })
        })
        .collect();
    (teams, leagues, state_leagues)
}

fn player_template(p: &PlayerRow) -> PlayerTemplate {
    PlayerTemplate {
        name: p.name.clone(),
        star: p.star,
        country: codes::country_id(&p.country).unwrap_or(-1),
        age: p.age,
        position: codes::position_id(&p.position).unwrap_or(0),
        status: i32::from(p.starter),
        trait1: codes::trait_id(&p.trait1).unwrap_or(-1),
        trait2: codes::trait_id(&p.trait2).unwrap_or(-1),
        side: codes::side_id(&p.side).unwrap_or(0),
        hash: p.potential,
        world_class: p.world_class,
        ..Default::default()
    }
}
