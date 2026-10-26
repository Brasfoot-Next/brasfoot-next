//! Reading a pack's files into [`Content`], with the issues of each file.

use std::collections::{BTreeMap, BTreeSet};

use crate::codes;
use crate::format::*;
use crate::issues::{Issue, IssueKind};
use crate::tree::FileTree;

/// The kinds of trophy folders (see [`crate::trophies`]).
pub const TROPHY_KINDS: [&str; 6] = ["league", "cup", "super-cup", "state", "regional", "continental"];
/// The pictures of a national team's folder.
pub const NATIONAL_TEAM_PICTURES: [&str; 3] = ["crest.png", "kit.png", "kit2.png"];

/// A club folder. In a patch any file may be missing: the layers below provide it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Club {
    /// `<country>/<slug>`, the folder below `clubs/`.
    pub id: String,
    pub data: Option<ClubFile>,
    pub squad: Option<Vec<PlayerRow>>,
    pub youth: Option<Vec<PlayerRow>>,
    /// The pictures in the folder, by file name.
    pub pictures: BTreeSet<String>,
}

/// Everything a pack, or a stack of packs, holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Content {
    pub clubs: BTreeMap<String, Club>,
    /// By country code.
    pub leagues: BTreeMap<String, LeagueFile>,
    /// By state code.
    pub state_leagues: BTreeMap<String, StateLeagueFile>,
    /// The pictures outside `clubs/` (national teams, trophies), by path.
    pub pictures: BTreeSet<String>,
}

pub fn read_manifest(tree: &dyn FileTree) -> Result<Manifest, Issue> {
    let issue = |kind| Issue { path: MANIFEST.to_owned(), line: None, kind };
    let bytes = tree.read(MANIFEST).map_err(|_| issue(IssueKind::MissingManifest))?;
    let manifest: Manifest = parse_toml(&bytes).map_err(issue)?;
    if manifest.format != FORMAT_VERSION {
        return Err(issue(IssueKind::UnsupportedFormat { found: manifest.format }));
    }
    Ok(manifest)
}

/// Checks of a manifest that do not stop reading it.
pub fn check_manifest(manifest: &Manifest) -> Vec<Issue> {
    let issue = |kind| Issue { path: MANIFEST.to_owned(), line: None, kind };
    let mut out = Vec::new();
    if manifest.kind == SourceKind::Base && !manifest.remove.is_empty() {
        out.push(issue(IssueKind::RemovalsInBase));
    }
    out
}

pub fn read_content(tree: &dyn FileTree, issues: &mut Vec<Issue>) -> Content {
    let mut content = Content::default();
    for path in tree.files() {
        let parts: Vec<&str> = path.split('/').collect();
        let mut issue = |kind| issues.push(Issue { path: path.clone(), line: None, kind });
        match parts.as_slice() {
            [MANIFEST] => {}
            [name] if is_documentation(name) => {}
            ["clubs", country, slug, file] => {
                if codes::country_id(&country.to_uppercase()).is_none() || *country != country.to_lowercase() {
                    issue(IssueKind::UnknownCountry { code: country.to_string() });
                    continue;
                }
                if !is_slug(slug) {
                    issue(IssueKind::BadFolderName { name: slug.to_string() });
                    continue;
                }
                let id = format!("{country}/{slug}");
                let known = *file == CLUB || *file == SQUAD || *file == YOUTH || *file == CREST || KITS.contains(file);
                if !known {
                    issue(IssueKind::UnknownFile);
                    continue;
                }
                let club = content.clubs.entry(id.clone()).or_insert_with(|| Club { id, ..Default::default() });
                match *file {
                    CLUB => club.data = read_club(tree, &path, country, issues),
                    SQUAD => club.squad = read_players(tree, &path, false, issues),
                    YOUTH => club.youth = read_players(tree, &path, true, issues),
                    picture => {
                        club.pictures.insert(picture.to_owned());
                    }
                }
            }
            ["leagues", file] => {
                if let Some(league) = read_league(tree, &path, file, issues) {
                    content.leagues.insert(league.country.clone(), league);
                }
            }
            ["state-leagues", file] => {
                if let Some(league) = read_state_league(tree, &path, file, issues) {
                    content.state_leagues.insert(league.state.clone(), league);
                }
            }
            ["national-teams", country, file] if NATIONAL_TEAM_PICTURES.contains(file) => {
                if codes::country_id(&country.to_uppercase()).is_none() || *country != country.to_lowercase() {
                    issue(IssueKind::UnknownCountry { code: country.to_string() });
                } else {
                    content.pictures.insert(path.clone());
                }
            }
            ["trophies", kind, file] if TROPHY_KINDS.contains(kind) && file.ends_with(".png") => {
                content.pictures.insert(path.clone());
            }
            _ => issue(IssueKind::UnknownFile),
        }
    }
    content
}

/// READMEs, licenses and other text at the root are not data.
fn is_documentation(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".md") || lower.ends_with(".txt") || lower.starts_with("license") || lower.starts_with("copying")
}

pub fn is_slug(s: &str) -> bool {
    !s.is_empty() && !s.starts_with('-') && !s.ends_with('-') && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn parse_toml<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, IssueKind> {
    let text = std::str::from_utf8(bytes).map_err(|e| IssueKind::Unreadable { reason: e.to_string() })?;
    toml::from_str(text).map_err(|e| IssueKind::Syntax { reason: e.message().to_owned() })
}

fn read_toml<T: serde::de::DeserializeOwned>(tree: &dyn FileTree, path: &str, issues: &mut Vec<Issue>) -> Option<T> {
    let parsed = tree.read(path).map_err(|e| IssueKind::Unreadable { reason: e.to_string() }).and_then(|bytes| parse_toml(&bytes));
    parsed.map_err(|kind| issues.push(Issue { path: path.to_owned(), line: None, kind })).ok()
}

fn is_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn range(field: &'static str, value: i32, min: i32, max: i32) -> Option<IssueKind> {
    (!(min..=max).contains(&value)).then_some(IssueKind::OutOfRange { field, value: value.into(), min: min.into(), max: max.into() })
}

fn country_issue(code: &str) -> Option<IssueKind> {
    codes::country_id(code).is_none().then(|| IssueKind::UnknownCountry { code: code.to_owned() })
}

fn read_club(tree: &dyn FileTree, path: &str, folder: &str, issues: &mut Vec<Issue>) -> Option<ClubFile> {
    let club: ClubFile = read_toml(tree, path, issues)?;
    let mut found = Vec::new();
    if club.name.trim().is_empty() {
        found.push(IssueKind::EmptyName);
    }
    if let Some(k) = country_issue(&club.country) {
        found.push(k);
    } else if club.country.to_lowercase() != folder {
        found.push(IssueKind::CountryFolderMismatch { folder: folder.to_uppercase(), country: club.country.clone() });
    }
    match &club.state {
        Some(state) if codes::state_id(state).is_none() => found.push(IssueKind::UnknownState { code: state.clone() }),
        Some(_) if club.country != "BRA" => found.push(IssueKind::StateOutsideBrazil),
        None if club.country == "BRA" => found.push(IssueKind::MissingState),
        _ => {}
    }
    found.extend(range("level", club.level, 1, 25));
    found.extend(range("capacity", club.stadium.capacity, 1000, 120000));
    found.extend(club.colors.iter().filter(|c| !is_color(c)).map(|c| IssueKind::BadColor { value: c.clone() }));
    found.extend(country_issue(&club.coach.country));
    found.extend(club.rivals.iter().filter(|r| !r.split_once('/').is_some_and(|(c, s)| is_slug(c) && is_slug(s))).map(|r| IssueKind::UnknownRival { id: r.clone() }));
    issues.extend(found.into_iter().map(|kind| Issue { path: path.to_owned(), line: None, kind }));
    Some(club)
}

fn read_players(tree: &dyn FileTree, path: &str, youth: bool, issues: &mut Vec<Issue>) -> Option<Vec<PlayerRow>> {
    let bytes = match tree.read(path) {
        Ok(b) => b,
        Err(e) => {
            issues.push(Issue { path: path.to_owned(), line: None, kind: IssueKind::Unreadable { reason: e.to_string() } });
            return None;
        }
    };
    let mut reader = csv::ReaderBuilder::new().trim(csv::Trim::All).from_reader(bytes.as_slice());
    let mut players = Vec::new();
    let mut failed = false;
    for row in reader.deserialize::<PlayerRow>() {
        match row {
            Ok(p) => {
                let line = Some(players.len() as u64 + 2);
                let mut push = |kind| issues.push(Issue { path: path.to_owned(), line, kind });
                if codes::position_id(&p.position).is_none() {
                    push(IssueKind::UnknownPosition { code: p.position.clone() });
                }
                if codes::side_id(&p.side).is_none() {
                    push(IssueKind::UnknownSide { code: p.side.clone() });
                }
                if let Some(k) = country_issue(&p.country) {
                    push(k);
                }
                for t in [&p.trait1, &p.trait2] {
                    if !t.is_empty() && codes::trait_id(t).is_none() {
                        push(IssueKind::UnknownTrait { code: t.clone() });
                    }
                }
                let (min, max) = if youth { (16, 20) } else { (16, 48) };
                if let Some(k) = range("age", p.age, min, max) {
                    push(k);
                }
                if !(0..=10).contains(&p.potential) {
                    push(IssueKind::PotentialOffScale { value: p.potential });
                }
                players.push(p);
            }
            Err(e) => {
                failed = true;
                let line = e.position().map(|p| p.line());
                issues.push(Issue { path: path.to_owned(), line, kind: IssueKind::Syntax { reason: csv_reason(&e) } });
            }
        }
    }
    (!failed).then_some(players)
}

fn csv_reason(e: &csv::Error) -> String {
    match e.kind() {
        csv::ErrorKind::Deserialize { err, .. } => match err.field() {
            Some(field) => format!("column {}: {}", field + 1, err.kind()),
            None => err.kind().to_string(),
        },
        _ => e.to_string(),
    }
}

fn check_divisions(divisions: impl Iterator<Item = i32>, path: &str, issues: &mut Vec<Issue>) {
    let mut seen = BTreeSet::new();
    for d in divisions {
        let kind = if !(1..=4).contains(&d) {
            IssueKind::BadDivision { division: d }
        } else if !seen.insert(d) {
            IssueKind::DuplicateDivision { division: d }
        } else {
            continue;
        };
        issues.push(Issue { path: path.to_owned(), line: None, kind });
    }
}

fn read_league(tree: &dyn FileTree, path: &str, file: &str, issues: &mut Vec<Issue>) -> Option<LeagueFile> {
    let league: LeagueFile = read_toml(tree, path, issues)?;
    let mut issue = |kind| issues.push(Issue { path: path.to_owned(), line: None, kind });
    if let Some(k) = country_issue(&league.country) {
        issue(k);
        return None;
    }
    let expected = format!("{}.toml", league.country.to_lowercase());
    if file != expected {
        issue(IssueKind::FileNameMismatch { expected });
        return None;
    }
    check_divisions(league.divisions.iter().map(|d| d.division), path, issues);
    Some(league)
}

fn read_state_league(tree: &dyn FileTree, path: &str, file: &str, issues: &mut Vec<Issue>) -> Option<StateLeagueFile> {
    let league: StateLeagueFile = read_toml(tree, path, issues)?;
    let mut issue = |kind| issues.push(Issue { path: path.to_owned(), line: None, kind });
    if codes::state_id(&league.state).is_none() {
        issue(IssueKind::UnknownState { code: league.state.clone() });
        return None;
    }
    let expected = format!("{}.toml", league.state.to_lowercase());
    if file != expected {
        issue(IssueKind::FileNameMismatch { expected });
        return None;
    }
    for d in &league.divisions {
        if codes::state_format_id(&d.format).is_none() {
            issue(IssueKind::UnknownStateFormat { code: d.format.clone() });
        }
    }
    check_divisions(league.divisions.iter().map(|d| d.division), path, issues);
    Some(league)
}
