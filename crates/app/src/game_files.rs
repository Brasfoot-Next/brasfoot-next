//! The installed game, read-only: crests, icons and the lists inside its jar. Nothing here
//! writes to the game folder.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use brasfoot_model::names::NameLists;

pub struct GameFiles {
    pub dir: PathBuf,
    jar: Option<PathBuf>,
    zip: Mutex<Option<zip::ZipArchive<std::fs::File>>>,
    /// Pictures of the jar already handed to egui, by path inside the jar.
    loaded: Mutex<HashMap<String, bool>>,
    crests: Mutex<HashMap<String, Option<PathBuf>>>,
}

impl GameFiles {
    /// In-memory or empty files, for testing or WASM.
    pub fn empty() -> Self {
        GameFiles {
            dir: PathBuf::new(),
            jar: None,
            zip: Mutex::new(None),
            loaded: Mutex::new(HashMap::new()),
            crests: Mutex::new(HashMap::new()),
        }
    }

    /// `BRASFOOT_GAME_DIR`, or the default install folder.
    pub fn locate() -> Self {
        let dir = std::env::var_os("BRASFOOT_GAME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".local/share/brasfoot"));
        Self::locate_at(&dir)
    }

    pub fn locate_at(dir: &Path) -> Self {
        let dir = dir.to_path_buf();
        let jar = std::fs::read_dir(&dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "jar"))
            .max_by_key(|p| p.file_name().is_some_and(|n| n.to_string_lossy().contains("mod")));
        let zip = jar.as_ref().and_then(|j| std::fs::File::open(j).ok()).and_then(|f| zip::ZipArchive::new(f).ok());
        GameFiles { dir, jar, zip: Mutex::new(zip), loaded: Mutex::new(HashMap::new()), crests: Mutex::new(HashMap::new()) }
    }

    /// The original saves (`sav/*.s22`), newest first.
    pub fn saves(&self) -> Vec<PathBuf> {
        let mut saves: Vec<PathBuf> = std::fs::read_dir(self.dir.join("sav"))
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "s22"))
            .collect();
        saves.sort_by_key(|p| std::cmp::Reverse(std::fs::metadata(p).and_then(|m| m.modified()).ok()));
        saves
    }

    /// A club's crest (`escudos/<team file>.png`).
    pub fn crest(&self, file_ref: &str) -> Option<PathBuf> {
        // asked every frame for every club on screen: the disk is checked once per crest
        let mut known = self.crests.lock().ok()?;
        known
            .entry(file_ref.to_owned())
            .or_insert_with(|| {
                let path = self.dir.join("escudos").join(format!("{file_ref}.png"));
                path.exists().then_some(path)
            })
            .clone()
    }

    /// A trophy's picture (`trofeus/<name>.png`, `aq.x`), checked on disk once per name.
    pub fn trophy(&self, name: &str) -> Option<PathBuf> {
        if name.is_empty() {
            return None;
        }
        let mut known = self.crests.lock().ok()?;
        known
            .entry(format!("trofeus/{name}"))
            .or_insert_with(|| {
                let path = self.dir.join("trofeus").join(format!("{name}.png"));
                path.exists().then_some(path)
            })
            .clone()
    }

    /// A file inside the game's jar.
    pub fn jar_bytes(&self, path: &str) -> Option<Vec<u8>> {
        let mut zip = self.zip.lock().ok()?;
        let mut entry = zip.as_mut()?.by_name(path).ok()?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }

    /// An image of the jar (`aicons/icon02.png`) as an egui URI, registered on first use.
    pub fn jar_image(&self, ctx: &eframe::egui::Context, path: &str) -> Option<String> {
        let uri = format!("bytes://jar/{path}");
        let mut loaded = self.loaded.lock().ok()?;
        let ok = *loaded.entry(path.to_owned()).or_insert_with(|| match self.jar_bytes(path) {
            Some(bytes) => {
                ctx.include_bytes(uri.clone(), bytes);
                true
            }
            None => false,
        });
        ok.then_some(uri)
    }

    /// The name lists of every country, from the jar.
    pub fn name_lists(&self) -> NameLists {
        let mut files = HashMap::new();
        if let Some(jar) = &self.jar
            && let Ok(file) = std::fs::File::open(jar)
            && let Ok(mut zip) = zip::ZipArchive::new(file)
        {
            for i in 0..zip.len() {
                let Ok(mut entry) = zip.by_index(i) else { continue };
                let name = entry.name().to_owned();
                if (name.starts_with("arquivos/names/") || name.starts_with("arquivos/surnames/")) && name.ends_with(".txt") {
                    // UTF-8 with bad bytes replaced, as the game reads them (CRN.txt is ISO-8859-1)
                    let mut bytes = Vec::new();
                    if entry.read_to_end(&mut bytes).is_ok() {
                        files.insert(name, String::from_utf8_lossy(&bytes).into_owned());
                    }
                }
            }
        }
        NameLists::from_files(files)
    }
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

/// Where this app keeps its own saves (`.bfn`).
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/share"))
        .join("brasfoot-next")
}

pub fn file_stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Default embedded fictional teams for WASM or when no teams are found on disk.
pub fn default_teams() -> Vec<brasfoot_model::templates::TeamTemplate> {
    const ZIP: &[u8] = include_bytes!("../assets/default_teams.zip");
    let mut teams = Vec::new();
    let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(ZIP)) else {
        return teams;
    };
    for i in 0..archive.len() {
        let Ok(mut file) = archive.by_index(i) else { continue };
        let name = file.name().to_string();
        if !name.ends_with(".ban") {
            continue;
        }
        let file_ref = name[..name.rfind('.').unwrap_or(name.len())].to_string();
        let mut bytes = Vec::new();
        if file.read_to_end(&mut bytes).is_ok() {
            if let Ok(mut team) = brasfoot_persistence::game_data::read_team(&bytes) {
                if team.valid {
                    team.file_ref = file_ref;
                    teams.push(team);
                }
            }
        }
    }
    teams
}
