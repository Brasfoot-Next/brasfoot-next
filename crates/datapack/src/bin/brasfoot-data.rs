//! `brasfoot-data`: converts the original game's data into a pack, validates packs and cuts
//! them into downloadable chunks. A tool for pack authors and their CI.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use brasfoot_datapack::database::{self, Layer};
use brasfoot_datapack::format::{FORMAT_VERSION, Manifest, SourceKind};
use brasfoot_datapack::issues::{Issue, Severity, has_errors};
use brasfoot_datapack::tree::DirTree;
use brasfoot_datapack::{legacy, pack, read};

const USAGE: &str = "usage:
  brasfoot-data convert <game folder> <out folder> --id <owner/name> --name <name> [--version <v>]
  brasfoot-data validate <pack folder> [--on <base folder>]...
  brasfoot-data pack <pack folder> <out folder> [--on <base folder>]... [--commit <sha>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("convert") => convert(&args[1..]),
        Some("validate") => split(&args[1..]).and_then(|(positional, options)| match positional[..] {
            [folder] => validate(folder, &options).map(|_| ()),
            _ => Err(USAGE.to_owned()),
        }),
        Some("pack") => pack(&args[1..]),
        _ => Err(USAGE.to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// `--name value` pairs.
type Options<'a> = Vec<(&'a str, &'a str)>;

/// Positional arguments and the values of `--name value` options.
fn split(args: &[String]) -> Result<(Vec<&str>, Options<'_>), String> {
    let mut positional = Vec::new();
    let mut options = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if let Some(name) = a.strip_prefix("--") {
            let value = it.next().ok_or(format!("--{name} needs a value\n{USAGE}"))?;
            options.push((name, value.as_str()));
        } else {
            positional.push(a.as_str());
        }
    }
    Ok((positional, options))
}

fn option<'a>(options: &[(&str, &'a str)], name: &str) -> Option<&'a str> {
    options.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

fn convert(args: &[String]) -> Result<(), String> {
    let (positional, options) = split(args)?;
    let [game, out] = positional[..] else { return Err(USAGE.to_owned()) };
    let (Some(id), Some(name)) = (option(&options, "id"), option(&options, "name")) else { return Err(USAGE.to_owned()) };
    let out = Path::new(out);
    if out.exists() && std::fs::read_dir(out).map_err(|e| e.to_string())?.next().is_some() {
        return Err(format!("{} is not empty", out.display()));
    }
    let manifest = Manifest {
        format: FORMAT_VERSION,
        id: id.to_owned(),
        kind: SourceKind::Base,
        name: name.to_owned(),
        description: None,
        version: option(&options, "version").map(str::to_owned),
        extends: None,
        remove: Default::default(),
    };
    let report = legacy::convert(Path::new(game), out, &manifest).map_err(|e| e.to_string())?;
    println!("clubs: {}", report.clubs);
    println!("players: {}", report.players);
    println!("leagues: {}", report.leagues);
    println!("state leagues: {}", report.state_leagues);
    println!("crests: {}, kits: {}", report.crests, report.kits);
    println!("national team pictures: {}", report.national_team_pictures);
    println!("trophies: {}", report.trophies);
    println!("rivalries: {} (missing from the data: {})", report.rivalries, report.missing_derbies.len());
    println!("clubs whose potentials were list places, drawn again: {} ({} players)", report.potentials_drawn.0, report.potentials_drawn.1);
    println!("clubs with a _bra file name from another country: {}", report.misplaced_suffix.len());
    println!("clubs renamed for a shared name: {}", report.renamed.len());
    for (from, to) in &report.renamed {
        println!("  {from} -> {to}");
    }
    println!("clubs without crest: {}", report.without_crest.len());
    for (folder, n) in &report.orphan_pictures {
        println!("pictures in {folder} of no club: {n}");
    }
    println!("trophies the game never asks for: {}", report.unknown_trophies.len());
    for t in &report.unknown_trophies {
        println!("  {t}");
    }
    println!("configurations the game never reads: {}", report.ignored_configs.len());
    for c in &report.ignored_configs {
        println!("  {c}");
    }
    Ok(())
}

struct Source {
    manifest: Manifest,
    tree: DirTree,
}

fn open(folder: &str) -> Result<Source, String> {
    let tree = DirTree::new(PathBuf::from(folder));
    let manifest = read::read_manifest(&tree).map_err(|i| format!("{folder}: {i}"))?;
    Ok(Source { manifest, tree })
}

/// Validates the pack in `folder` over the `--on` bases, prints the issues and fails on errors.
fn validate(folder: &str, options: &[(&str, &str)]) -> Result<Source, String> {
    let bases = options.iter().filter(|(n, _)| *n == "on").map(|(_, v)| open(v)).collect::<Result<Vec<_>, _>>()?;
    let source = open(folder)?;
    let mut issues: Vec<Issue> = read::check_manifest(&source.manifest);
    if source.manifest.kind == SourceKind::Patch && bases.is_empty() {
        // alone, a patch can only be checked file by file
        read::read_content(&source.tree, &mut issues);
    } else {
        let layers: Vec<Layer> =
            bases.iter().chain(std::iter::once(&source)).map(|s| Layer { manifest: &s.manifest, tree: &s.tree }).collect();
        let (db, found) = database::compose(&layers);
        issues.extend(found);
        let players: usize = db.content.clubs.values().map(|c| c.squad.as_ref().map_or(0, Vec::len) + c.youth.as_ref().map_or(0, Vec::len)).sum();
        println!(
            "{}: {} clubs, {} players, {} leagues, {} state leagues, {} other pictures",
            source.manifest.id,
            db.content.clubs.len(),
            players,
            db.content.leagues.len(),
            db.content.state_leagues.len(),
            db.content.pictures.len()
        );
    }
    issues.sort_by(|a, b| b.kind.severity().cmp(&a.kind.severity()).then(a.path.cmp(&b.path)).then(a.line.cmp(&b.line)));
    for issue in &issues {
        println!("{issue}");
    }
    let errors = issues.iter().filter(|i| i.kind.severity() == Severity::Error).count();
    println!("{errors} errors, {} warnings", issues.len() - errors);
    if has_errors(&issues) { Err(format!("{folder} has errors")) } else { Ok(source) }
}

fn pack(args: &[String]) -> Result<(), String> {
    let (positional, options) = split(args)?;
    let [folder, out] = positional[..] else { return Err(USAGE.to_owned()) };
    let source = validate(folder, &options)?;
    let index = pack::build(&source.tree, &source.manifest, option(&options, "commit"), Path::new(out)).map_err(|e| e.to_string())?;
    for chunk in &index.chunks {
        println!("{} {} files {} bytes", chunk.file, chunk.files, chunk.size);
    }
    println!("{} chunks, {} bytes", index.chunks.len(), index.total_size());
    Ok(())
}
