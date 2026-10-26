//! Reading, layering, validating and packing made-up packs.

use std::path::Path;

use brasfoot_datapack::database::{self, Layer};
use brasfoot_datapack::format::Manifest;
use brasfoot_datapack::issues::{IssueKind, Severity, has_errors};
use brasfoot_datapack::pack;
use brasfoot_datapack::read::{self, read_content};
use brasfoot_datapack::tree::{DirTree, FileTree, MemTree};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

const HEADER: &str = "name,position,side,age,country,potential,starter,star,world_class,trait1,trait2\n";

fn write(root: &Path, path: &str, text: &[u8]) {
    let full = root.join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, text).unwrap();
}

fn club_toml(name: &str, country: &str, state: Option<&str>, rivals: &[&str]) -> String {
    let state = state.map(|s| format!("state = \"{s}\"\n")).unwrap_or_default();
    let rivals = if rivals.is_empty() { String::new() } else { format!("rivals = [{}]\n", rivals.iter().map(|r| format!("\"{r}\"")).collect::<Vec<_>>().join(", ")) };
    format!(
        "name = \"{name}\"\ncountry = \"{country}\"\n{state}level = 12\nreputation = 3\ncolors = [\"#112233\", \"#ffffff\"]\n{rivals}\n[stadium]\nname = \"Estádio Faz de Conta\"\ncapacity = 25000\n\n[coach]\nname = \"Técnico Inventado\"\ncountry = \"{country}\"\n"
    )
}

/// A base with two Brazilian clubs and one English, a league and a state league.
fn base(root: &Path) {
    write(root, "manifest.toml", b"format = 1\nid = \"exemplo/base\"\nkind = \"base\"\nname = \"Base de exemplo\"\n");
    write(root, "README.md", b"# Base de exemplo\n");
    write(root, "clubs/bra/vila-exemplo/club.toml", club_toml("Vila Exemplo", "BRA", Some("SP"), &["bra/uniao-ficticia"]).as_bytes());
    write(
        root,
        "clubs/bra/vila-exemplo/squad.csv",
        format!("{HEADER}João Exemplo,GK,R,29,BRA,7,true,false,false,reflexes,penalty-saving\n,FW,L,22,ARG,9,false,true,false,finishing,pace\n").as_bytes(),
    );
    write(root, "clubs/bra/vila-exemplo/crest.png", PNG);
    write(root, "clubs/bra/uniao-ficticia/club.toml", club_toml("União Fictícia", "BRA", Some("RJ"), &[]).as_bytes());
    write(root, "clubs/bra/uniao-ficticia/squad.csv", format!("{HEADER}Pedro Inventado,MF,R,25,BRA,5,true,false,false,passing,playmaking\n").as_bytes());
    write(root, "clubs/bra/uniao-ficticia/crest.png", PNG);
    write(root, "clubs/eng/imaginary-town/club.toml", club_toml("Imaginary Town", "ENG", None, &[]).as_bytes());
    write(root, "clubs/eng/imaginary-town/squad.csv", format!("{HEADER}John Madeup,CB,R,31,ENG,4,true,false,false,marking,heading\n").as_bytes());
    write(root, "clubs/eng/imaginary-town/crest.png", PNG);
    write(root, "leagues/eng.toml", b"country = \"ENG\"\n\n[[divisions]]\ndivision = 1\nteams = 20\nrelegated = 3\n");
    write(root, "state-leagues/sp.toml", b"state = \"SP\"\n\n[[divisions]]\ndivision = 1\nformat = \"16-teams-4-groups-sp-2021\"\nrelegated = 2\ntiebreak = 0\ntwo_legged_finals = [false, true, true]\n");
    write(root, "trophies/cup/bra.png", PNG);
    write(root, "national-teams/bra/crest.png", PNG);
}

fn open(root: &Path) -> (Manifest, DirTree) {
    let tree = DirTree::new(root);
    (read::read_manifest(&tree).unwrap(), tree)
}

#[test]
fn a_base_becomes_the_game_team_files_and_configurations() {
    let dir = tempfile::tempdir().unwrap();
    base(dir.path());
    let (manifest, tree) = open(dir.path());
    let (db, issues) = database::compose(&[Layer { manifest: &manifest, tree: &tree }]);
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(db.content.pictures.len(), 2);

    let (teams, leagues, states) = database::to_templates(&db.content);
    let ids: Vec<&str> = teams.iter().map(|t| t.file_ref.as_str()).collect();
    assert_eq!(ids, ["bra/uniao-ficticia", "bra/vila-exemplo", "eng/imaginary-town"]);
    let vila = &teams[1];
    assert_eq!((vila.country, vila.state, vila.level, vila.capacity, vila.vid), (29, 25, 12, 25000, 185));
    assert_eq!(vila.rivals, ["bra/uniao-ficticia"]);
    let keeper = &vila.players[0];
    assert_eq!((keeper.position, keeper.side, keeper.status, keeper.hash, keeper.trait1, keeper.trait2), (0, 0, 1, 7, 2, 1));
    let drawn = &vila.players[1];
    assert_eq!((drawn.name.as_str(), drawn.country, drawn.side, drawn.star, drawn.trait2), ("", 11, 1, true, 13));
    // a club of another country holds state 0, as the team files do
    assert_eq!(teams[2].state, 0);

    assert_eq!(leagues.len(), 1);
    let l = &leagues[0];
    // omitted fields take est.ConfigLigaType's initialisers; the version is the one honoured
    assert_eq!((l.country, l.division, l.teams, l.relegated, l.relegated_directly, l.file_version), (97, 1, 20, 3, 2, 22));
    assert!(l.two_rounds && l.two_legged_knockout == vec![true; 7]);
    assert_eq!((states[0].state, states[0].formula, states[0].two_legged_finals), (25, 7, [1, 2, 2]));
}

#[test]
fn patches_change_files_add_clubs_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    base(dir.path());
    let patch = tempfile::tempdir().unwrap();
    let p = patch.path();
    write(
        p,
        "manifest.toml",
        b"format = 1\nid = \"exemplo/janeiro\"\nkind = \"patch\"\nname = \"Janeiro\"\nextends = \"exemplo/base\"\n\n[remove]\nclubs = [\"eng/imaginary-town\"]\nleagues = [\"ENG\"]\n",
    );
    // a new squad only, the club's other files stay
    write(p, "clubs/bra/uniao-ficticia/squad.csv", format!("{HEADER}Novo Reforço,FW,R,27,BRA,8,true,false,false,pace,dribbling\n").as_bytes());
    // a crest only
    write(p, "clubs/bra/vila-exemplo/crest.png", b"\x89PNG\r\n\x1a\nnew");
    write(p, "clubs/arg/club-de-mentira/club.toml", club_toml("Club de Mentira", "ARG", None, &[]).as_bytes());
    write(p, "clubs/arg/club-de-mentira/squad.csv", format!("{HEADER}Juan Ficticio,MF,L,23,ARG,6,true,false,false,passing,stamina\n").as_bytes());
    write(p, "clubs/arg/club-de-mentira/crest.png", PNG);

    let (bm, bt) = open(dir.path());
    let (pm, pt) = open(p);
    let (db, issues) = database::compose(&[Layer { manifest: &bm, tree: &bt }, Layer { manifest: &pm, tree: &pt }]);
    assert!(issues.is_empty(), "{issues:?}");
    let ids: Vec<&String> = db.content.clubs.keys().collect();
    assert_eq!(ids, ["arg/club-de-mentira", "bra/uniao-ficticia", "bra/vila-exemplo"]);
    assert!(db.content.leagues.is_empty());
    let uniao = &db.content.clubs["bra/uniao-ficticia"];
    assert_eq!(uniao.data.as_ref().unwrap().name, "União Fictícia");
    assert_eq!(uniao.squad.as_ref().unwrap()[0].name, "Novo Reforço");
    assert_eq!(db.layer_of("clubs/bra/vila-exemplo/crest.png"), Some(1));
    assert_eq!(db.layer_of("clubs/bra/vila-exemplo/club.toml"), Some(0));
}

#[test]
fn a_patch_over_nothing_or_over_missing_clubs_is_reported() {
    let patch = tempfile::tempdir().unwrap();
    let p = patch.path();
    write(p, "manifest.toml", b"format = 1\nid = \"exemplo/escudos\"\nkind = \"patch\"\nname = \"Escudos\"\n\n[remove]\nclubs = [\"bra/nao-existe\"]\n");
    write(p, "clubs/bra/sem-clube/crest.png", PNG);
    let (pm, pt) = open(p);
    let (_, issues) = database::compose(&[Layer { manifest: &pm, tree: &pt }]);
    let kinds: Vec<&IssueKind> = issues.iter().map(|i| &i.kind).collect();
    assert!(kinds.contains(&&IssueKind::PatchWithoutBase));
    assert!(kinds.contains(&&IssueKind::UnknownRemoval { id: "bra/nao-existe".into() }));
    assert!(kinds.contains(&&IssueKind::UnknownClub));
}

#[test]
fn wrong_values_are_reported_with_their_file_and_line() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    base(root);
    write(root, "clubs/eng/imaginary-town/club.toml", club_toml("Imaginary Town", "BRA", Some("XX"), &["bra/nobody"]).as_bytes());
    write(
        root,
        "clubs/bra/uniao-ficticia/squad.csv",
        format!("{HEADER}Ok Player,MF,R,25,BRA,5,true,false,false,passing,\nBad Player,ST,X,60,ZZZ,38,true,false,false,shooting,pace\n").as_bytes(),
    );
    write(root, "clubs/bra/Vila_Errada/club.toml", b"");
    write(root, "leagues/ger.toml", b"country = \"ENG\"\n\n[[divisions]]\ndivision = 1\n");
    write(root, "notes.json", b"{}");
    let (manifest, tree) = open(root);
    let (_, issues) = database::compose(&[Layer { manifest: &manifest, tree: &tree }]);
    let has = |path: &str, line: Option<u64>, kind: IssueKind| issues.iter().any(|i| i.path == path && i.line == line && i.kind == kind);
    let club = "clubs/eng/imaginary-town/club.toml";
    assert!(has(club, None, IssueKind::CountryFolderMismatch { folder: "ENG".into(), country: "BRA".into() }));
    assert!(has(club, None, IssueKind::UnknownState { code: "XX".into() }));
    assert!(has(club, None, IssueKind::UnknownRival { id: "bra/nobody".into() }));
    let squad = "clubs/bra/uniao-ficticia/squad.csv";
    assert!(has(squad, Some(3), IssueKind::UnknownPosition { code: "ST".into() }));
    assert!(has(squad, Some(3), IssueKind::UnknownSide { code: "X".into() }));
    assert!(has(squad, Some(3), IssueKind::UnknownCountry { code: "ZZZ".into() }));
    assert!(has(squad, Some(3), IssueKind::UnknownTrait { code: "shooting".into() }));
    assert!(has(squad, Some(3), IssueKind::OutOfRange { field: "age", value: 60, min: 16, max: 48 }));
    assert!(has(squad, Some(3), IssueKind::PotentialOffScale { value: 38 }));
    assert!(has("clubs/bra/Vila_Errada/club.toml", None, IssueKind::BadFolderName { name: "Vila_Errada".into() }));
    assert!(has("leagues/ger.toml", None, IssueKind::FileNameMismatch { expected: "eng.toml".into() }));
    assert!(has("notes.json", None, IssueKind::UnknownFile));
    // a trailing empty trait is none, not an error
    assert!(!issues.iter().any(|i| i.path == squad && i.line == Some(2)));
    assert!(has_errors(&issues));
    assert_eq!(IssueKind::UnknownFile.severity(), Severity::Warning);
}

#[test]
fn a_broken_csv_line_is_a_syntax_error_and_the_squad_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    base(root);
    write(root, "clubs/bra/uniao-ficticia/squad.csv", format!("{HEADER}Only Name\n").as_bytes());
    let mut issues = Vec::new();
    let content = read_content(&DirTree::new(root), &mut issues);
    assert!(content.clubs["bra/uniao-ficticia"].squad.is_none());
    assert!(issues.iter().any(|i| matches!(i.kind, IssueKind::Syntax { .. }) && i.line == Some(2)));
}

#[test]
fn packs_are_deterministic_and_unpack_to_the_same_files() {
    let dir = tempfile::tempdir().unwrap();
    base(dir.path());
    let (manifest, tree) = open(dir.path());
    let out1 = tempfile::tempdir().unwrap();
    let out2 = tempfile::tempdir().unwrap();
    let index = pack::build(&tree, &manifest, Some("abc123"), out1.path()).unwrap();
    let again = pack::build(&tree, &manifest, Some("abc123"), out2.path()).unwrap();
    assert_eq!(index, again);
    let names: Vec<&str> = index.chunks.iter().map(|c| c.file.as_str()).collect();
    assert_eq!(names, ["clubs-b.zip", "clubs-e.zip", "data.zip", "national-teams.zip", "trophies.zip"]);

    let read_index = pack::parse_index(&std::fs::read(out1.path().join(pack::INDEX)).unwrap()).unwrap();
    assert_eq!(read_index.manifest, manifest);
    assert_eq!(read_index.commit.as_deref(), Some("abc123"));
    let unpacked = pack::unpack(&read_index, |name| std::fs::read(out1.path().join(name))).unwrap();
    assert_eq!(unpacked.files(), tree.files());
    for f in tree.files() {
        assert_eq!(unpacked.read(&f).unwrap(), tree.read(&f).unwrap(), "{f}");
    }

    // a corrupted chunk is refused
    std::fs::write(out1.path().join("data.zip"), b"broken").unwrap();
    assert!(pack::unpack(&read_index, |name| std::fs::read(out1.path().join(name))).is_err());
}

#[test]
fn a_github_archive_loses_its_top_folder() {
    let dir = tempfile::tempdir().unwrap();
    base(dir.path());
    let tree = DirTree::new(dir.path());
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for f in tree.files() {
        zip.start_file(format!("database-main/{f}"), zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut zip, &tree.read(&f).unwrap()).unwrap();
    }
    zip.start_file("database-main/.github/workflows/pack.yml", zip::write::SimpleFileOptions::default()).unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let mem = MemTree::from_zip(&bytes).unwrap();
    assert_eq!(mem.files(), tree.files());
    assert!(read::read_manifest(&mem).is_ok());
}
