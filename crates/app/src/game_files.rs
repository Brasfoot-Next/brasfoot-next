//! The installed game, read-only: crests, icons and the lists inside its jar, plus the data
//! packs (`patches`) the clubs of a new career come from. Nothing here writes to the game folder.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use brasfoot_model::names::NameLists;

use crate::patches::Library;

pub struct GameFiles {
    pub dir: PathBuf,
    jar: Option<PathBuf>,
    zip: Mutex<Option<zip::ZipArchive<std::fs::File>>>,
    /// Pictures of the jar already handed to egui, by path inside the jar.
    loaded: Mutex<HashMap<String, bool>>,
    crests: Mutex<HashMap<String, Option<PathBuf>>>,
    /// The data sources; detached until the app attaches the real ones (`with_patches`).
    pub patches: Library,
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
            patches: Library::detached(),
        }
    }

    pub fn with_patches(mut self, patches: Library) -> Self {
        self.patches = patches;
        self
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
        GameFiles { dir, jar, zip: Mutex::new(zip), loaded: Mutex::new(HashMap::new()), crests: Mutex::new(HashMap::new()), patches: Library::detached() }
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

    /// A club's crest as an image URI: a club of a data pack (`bra/vila-exemplo`) from the packs,
    /// a team file's from `escudos/<team file>.png`.
    pub fn crest(&self, ctx: &eframe::egui::Context, file_ref: &str) -> Option<String> {
        if file_ref.contains('/') {
            return self.patches.picture(ctx, &format!("clubs/{file_ref}/crest.png"));
        }
        // asked every frame for every club on screen: the disk is checked once per crest
        let mut known = self.crests.lock().ok()?;
        known
            .entry(file_ref.to_owned())
            .or_insert_with(|| {
                let path = self.dir.join("escudos").join(format!("{file_ref}.png"));
                path.exists().then_some(path)
            })
            .clone()
            .map(|p| format!("file://{}", p.display()))
    }

    /// Whether a trophy's picture exists (`aq.w`): in the data packs (see
    /// `brasfoot_datapack::trophies`) or in the game folder.
    pub fn has_trophy(&self, name: &str) -> bool {
        brasfoot_datapack::trophies::pack_path(name).is_some_and(|p| self.patches.has(&p)) || self.trophy_on_disk(name).is_some()
    }

    /// A trophy's picture (`aq.x`) as an image URI, from the data packs or the game folder.
    pub fn trophy(&self, ctx: &eframe::egui::Context, name: &str) -> Option<String> {
        if let Some(uri) = brasfoot_datapack::trophies::pack_path(name).and_then(|p| self.patches.picture(ctx, &p)) {
            return Some(uri);
        }
        self.trophy_on_disk(name).map(|p| format!("file://{}", p.display()))
    }

    /// `trofeus/<name>.png`, checked on disk once per name.
    fn trophy_on_disk(&self, name: &str) -> Option<PathBuf> {
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
