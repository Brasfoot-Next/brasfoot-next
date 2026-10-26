//! The data sources of a new game (`docs/patches.md`): GitHub repositories, folders and zips,
//! stacked base first, downloaded into a local store and composed into one database. The game
//! reads the database at a new game; the screens read the clubs' crests and the trophies from it.

pub mod download;
pub mod store;

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use brasfoot_datapack::database::{self, Database, Layer};
use brasfoot_datapack::format::Manifest;
use brasfoot_datapack::issues::Issue;
use brasfoot_datapack::pack::Chunk;
use brasfoot_datapack::tree::{FileTree, MemTree};
use eframe::egui;
use serde::{Deserialize, Serialize};

use self::download::{DownloadError, Http, Phase, Progress};
use self::store::{PlatformStore, Store};

/// The official database, in every new list of sources.
pub const OFFICIAL: (&str, &str) = ("brasfoot-next", "database");
/// Where the list of sources is kept in the store.
const LIST_KEY: &str = "sources.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Location {
    GitHub { owner: String, repo: String },
    /// A pack folder on disk, read as it is every time (for authors testing a pack).
    Folder { path: String },
    /// A zip copied into the store when added.
    Zip { path: String },
}

impl Location {
    /// `owner/repo`, `github.com/owner/repo` or any GitHub link into the repository.
    pub fn github(text: &str) -> Option<Location> {
        let text = text.trim().trim_end_matches('/');
        let rest = text.strip_prefix("https://").or_else(|| text.strip_prefix("http://")).unwrap_or(text);
        let rest = rest.strip_prefix("www.").unwrap_or(rest);
        let rest = rest.strip_prefix("github.com/").unwrap_or(rest);
        let mut parts = rest.split('/');
        let owner = parts.next()?.trim();
        let repo = parts.next()?.trim();
        let repo = repo.strip_suffix(".git").unwrap_or(repo);
        let valid = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
        (valid(owner) && valid(repo) && !rest.contains(' ') && !rest.contains(':')).then(|| Location::GitHub { owner: owner.to_lowercase(), repo: repo.to_lowercase() })
    }

    /// The source's folder in the store.
    pub fn key(&self) -> String {
        match self {
            Location::GitHub { owner, repo } => format!("github/{owner}/{repo}"),
            Location::Folder { path } | Location::Zip { path } => {
                let kind = if matches!(self, Location::Folder { .. }) { "folder" } else { "zip" };
                format!("{kind}/{}", &brasfoot_datapack::pack::sha256_hex(path.as_bytes())[..16])
            }
        }
    }

    /// `owner/repo` or the path.
    pub fn label(&self) -> String {
        match self {
            Location::GitHub { owner, repo } => format!("{owner}/{repo}"),
            Location::Folder { path } | Location::Zip { path } => path.clone(),
        }
    }

    pub fn is_official(&self) -> bool {
        matches!(self, Location::GitHub { owner, repo } if owner == OFFICIAL.0 && repo == OFFICIAL.1)
    }
}

/// What a source has in the store.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Installed {
    pub manifest: Manifest,
    /// The source commit (or the pack's hashes) it was downloaded at.
    pub version: Option<String>,
    pub chunks: Vec<Chunk>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub location: Location,
    pub enabled: bool,
    #[serde(default)]
    pub installed: Option<Installed>,
}

/// The sources, base first: each one goes over the ones before it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceList {
    pub sources: Vec<Source>,
}

impl Default for SourceList {
    fn default() -> Self {
        let official = Location::GitHub { owner: OFFICIAL.0.to_owned(), repo: OFFICIAL.1.to_owned() };
        SourceList { sources: vec![Source { location: official, enabled: true, installed: None }] }
    }
}

/// Whether a GitHub source has a newer version than the one downloaded.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Checking,
    UpToDate,
    UpdateAvailable,
    Failed(DownloadError),
}

/// A download running or finished, for the screen.
#[derive(Clone)]
pub struct Job {
    pub progress: Progress,
    pub result: Arc<Mutex<Option<Result<(), DownloadError>>>>,
}

pub struct LoadedLayer {
    pub location: Location,
    pub manifest: Manifest,
    pub tree: Box<dyn FileTree + Send + Sync>,
}

/// The enabled sources made into one database.
pub struct Loaded {
    pub layers: Vec<LoadedLayer>,
    pub database: Database,
    pub issues: Vec<Issue>,
    /// Enabled sources left out: not downloaded, or their files missing from the store.
    pub skipped: Vec<Location>,
}

#[derive(Default)]
struct Shared {
    /// `None` until read from the store.
    list: Option<SourceList>,
    loaded: Option<Arc<Loaded>>,
    /// Bumped by every change of the list; a composition of an older one is dropped.
    generation: u64,
    composing: bool,
    jobs: HashMap<String, Job>,
    status: HashMap<String, Status>,
    /// Pictures of the database handed to egui, by URI.
    pictures: HashMap<String, bool>,
}

/// The sources and their database, shared by the screens. Cheap to clone.
#[derive(Clone)]
pub struct Library {
    shared: Arc<Mutex<Shared>>,
    store: PlatformStore,
}

/// Runs `make()`'s future in the background: a thread on the desktop, the browser's event loop
/// on the web.
pub fn spawn<F, Fut>(make: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || pollster::block_on(make()));
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(make());
}

pub async fn sleep(seconds: u64) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::sleep(std::time::Duration::from_secs(seconds));
    #[cfg(target_arch = "wasm32")]
    {
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            if let Some(w) = web_sys::window() {
                let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, (seconds * 1000).min(i32::MAX as u64) as i32);
            }
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}

/// The files of a downloaded source, from its chunks in the store.
async fn installed_tree(store: &impl Store, key: &str, installed: &Installed) -> Option<MemTree> {
    let mut tree = MemTree::default();
    for chunk in &installed.chunks {
        let bytes = store.read(&download::chunk_key(key, &chunk.sha256)).await?;
        let part = MemTree::from_zip(&bytes).ok()?;
        for path in part.files() {
            let data = part.read(&path).ok()?;
            tree.insert(path, data);
        }
    }
    Some(tree)
}

/// The enabled sources of `list` as one database.
pub async fn compose(store: &impl Store, list: &SourceList) -> Option<Loaded> {
    let mut layers = Vec::new();
    let mut skipped = Vec::new();
    for source in list.sources.iter().filter(|s| s.enabled) {
        let layer = match (&source.location, &source.installed) {
            #[cfg(not(target_arch = "wasm32"))]
            (Location::Folder { path }, _) => {
                let tree = brasfoot_datapack::tree::DirTree::new(path);
                brasfoot_datapack::read::read_manifest(&tree).ok().map(|manifest| LoadedLayer { location: source.location.clone(), manifest, tree: Box::new(tree) })
            }
            (location, Some(installed)) => installed_tree(store, &location.key(), installed)
                .await
                .map(|tree| LoadedLayer { location: location.clone(), manifest: installed.manifest.clone(), tree: Box::new(tree) }),
            _ => None,
        };
        match layer {
            Some(layer) => layers.push(layer),
            None => skipped.push(source.location.clone()),
        }
    }
    if layers.is_empty() {
        return None;
    }
    let (database, issues) = {
        let stack: Vec<Layer> = layers.iter().map(|l| Layer { manifest: &l.manifest, tree: l.tree.as_ref() }).collect();
        database::compose(&stack)
    };
    Some(Loaded { layers, database, issues, skipped })
}

impl Library {
    /// Opens the platform's store and starts reading the sources.
    pub fn open() -> Self {
        let library = Library { shared: Arc::default(), store: PlatformStore::open() };
        let this = library.clone();
        spawn(move || async move {
            let list = match this.store.read(LIST_KEY).await {
                Some(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
                None => SourceList::default(),
            };
            this.replace_list(list, false);
        });
        library
    }

    /// No sources and no store reads: for tests and tools.
    pub fn detached() -> Self {
        let shared = Shared { list: Some(SourceList { sources: Vec::new() }), ..Default::default() };
        Library { shared: Arc::new(Mutex::new(shared)), store: PlatformStore::open() }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The sources, once read.
    pub fn list(&self) -> Option<SourceList> {
        self.lock().list.clone()
    }

    /// Whether the list is read and the database composed (or there is nothing to compose).
    pub fn ready(&self) -> bool {
        let s = self.lock();
        s.list.is_some() && !s.composing
    }

    pub fn loaded(&self) -> Option<Arc<Loaded>> {
        self.lock().loaded.clone()
    }

    pub fn job(&self, location: &Location) -> Option<Job> {
        self.lock().jobs.get(&location.key()).cloned()
    }

    pub fn status(&self, location: &Location) -> Option<Status> {
        self.lock().status.get(&location.key()).cloned()
    }

    /// Whether something runs in the background (the screen keeps repainting meanwhile).
    pub fn busy(&self) -> bool {
        let s = self.lock();
        s.list.is_none()
            || s.composing
            || s.status.values().any(|st| *st == Status::Checking)
            || s.jobs.values().any(|j| j.result.lock().map(|r| r.is_none()).unwrap_or(false))
    }

    /// Sets the list, saves it when asked, and composes the database again.
    fn replace_list(&self, list: SourceList, save: bool) {
        let generation = {
            let mut s = self.lock();
            s.list = Some(list.clone());
            s.generation += 1;
            s.composing = true;
            s.generation
        };
        let this = self.clone();
        spawn(move || async move {
            if save && let Ok(bytes) = serde_json::to_vec_pretty(&list) {
                let _ = this.store.write(LIST_KEY, bytes).await;
            }
            let loaded = compose(&this.store, &list).await.map(Arc::new);
            let mut s = this.lock();
            if s.generation == generation {
                s.loaded = loaded;
                s.composing = false;
                s.pictures.clear();
            }
        });
    }

    fn edit(&self, change: impl FnOnce(&mut Vec<Source>)) {
        let Some(mut list) = self.list() else { return };
        change(&mut list.sources);
        self.replace_list(list, true);
    }

    /// Adds a source on top and, for GitHub, starts downloading it. False when it is there already.
    pub fn add(&self, location: Location) -> bool {
        if self.list().is_some_and(|l| l.sources.iter().any(|s| s.location == location)) {
            return false;
        }
        let download = matches!(location, Location::GitHub { .. } | Location::Zip { .. });
        self.edit(|sources| sources.push(Source { location: location.clone(), enabled: true, installed: None }));
        if download {
            self.install(&location);
        }
        true
    }

    /// Removes a source and its files.
    pub fn remove(&self, location: &Location) {
        self.edit(|sources| sources.retain(|s| s.location != *location));
        let key = location.key();
        self.lock().jobs.remove(&key);
        let store = self.store.clone();
        spawn(move || async move {
            for k in store.keys(&format!("{key}/")).await {
                store.remove(&k).await;
            }
        });
    }

    pub fn set_enabled(&self, location: &Location, enabled: bool) {
        self.edit(|sources| {
            if let Some(s) = sources.iter_mut().find(|s| s.location == *location) {
                s.enabled = enabled;
            }
        });
    }

    /// Moves a source one place down the stack (`-1`, under the one before) or up (`1`).
    pub fn shift(&self, location: &Location, by: i32) {
        self.edit(|sources| {
            if let Some(i) = sources.iter().position(|s| s.location == *location) {
                let j = i as i32 + by;
                if j >= 0 && (j as usize) < sources.len() {
                    sources.swap(i, j as usize);
                }
            }
        });
    }

    /// Downloads (or updates) a GitHub source, or copies a zip, in the background.
    pub fn install(&self, location: &Location) {
        let key = location.key();
        let job = Job { progress: Arc::new(Mutex::new(Phase::Starting)), result: Arc::default() };
        {
            let mut s = self.lock();
            if s.jobs.get(&key).is_some_and(|j| j.result.lock().map(|r| r.is_none()).unwrap_or(false)) {
                return;
            }
            s.jobs.insert(key.clone(), job.clone());
            s.status.remove(&key);
        }
        let this = self.clone();
        let location = location.clone();
        spawn(move || async move {
            let result = match &location {
                Location::GitHub { owner, repo } => download::install_github(&Http, &this.store, &key, owner, repo, &job.progress).await,
                #[cfg(not(target_arch = "wasm32"))]
                Location::Zip { path } => match std::fs::read(path) {
                    Ok(bytes) => download::install_bytes(&this.store, &key, "source.zip", bytes, None).await,
                    Err(e) => Err(DownloadError::Storage(e.to_string())),
                },
                // folders are read in place; the browser has no files to copy
                _ => Err(DownloadError::NotAPack),
            };
            let outcome = result.map(|installed| {
                this.edit(|sources| {
                    if let Some(s) = sources.iter_mut().find(|s| s.location == location) {
                        s.installed = Some(installed);
                    }
                });
            });
            if outcome.is_ok() {
                this.lock().status.insert(key.clone(), Status::UpToDate);
            }
            if let Ok(mut r) = job.result.lock() {
                *r = Some(outcome);
            }
        });
    }

    /// Asks GitHub, once per source, whether the downloaded sources have a newer version.
    pub fn check_updates(&self) {
        let Some(list) = self.list() else { return };
        for source in list.sources {
            let (Location::GitHub { owner, repo }, Some(installed)) = (&source.location, &source.installed) else { continue };
            let key = source.location.key();
            {
                let mut s = self.lock();
                if s.status.contains_key(&key) || s.jobs.contains_key(&key) {
                    continue;
                }
                s.status.insert(key.clone(), Status::Checking);
            }
            let (this, owner, repo, current) = (self.clone(), owner.clone(), repo.clone(), installed.version.clone());
            spawn(move || async move {
                let status = match download::latest_version(&Http, &owner, &repo).await {
                    Ok(latest) if Some(&latest) == current.as_ref() => Status::UpToDate,
                    Ok(_) => Status::UpdateAvailable,
                    Err(e) => Status::Failed(e),
                };
                this.lock().status.insert(key, status);
            });
        }
    }

    /// Whether the database has a file.
    pub fn has(&self, path: &str) -> bool {
        self.loaded().is_some_and(|l| l.database.layer_of(path).is_some())
    }

    /// A picture of the database (`clubs/bra/x/crest.png`) as an egui URI, registered on first use.
    pub fn picture(&self, ctx: &egui::Context, path: &str) -> Option<String> {
        let loaded = self.loaded()?;
        let layer = loaded.database.layer_of(path)?;
        // the address changes with the database, so egui never shows an old picture
        let uri = format!("bytes://patches/{:p}/{path}", Arc::as_ptr(&loaded));
        let mut s = self.lock();
        let ok = *s.pictures.entry(uri.clone()).or_insert_with(|| match loaded.layers[layer].tree.read(path) {
            Ok(bytes) => {
                ctx.include_bytes(uri.clone(), bytes);
                true
            }
            Err(_) => false,
        });
        ok.then_some(uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patches::store::MemStore;

    #[test]
    fn github_links_of_every_shape_name_the_repository() {
        let repo = |o: &str, r: &str| Some(Location::GitHub { owner: o.into(), repo: r.into() });
        assert_eq!(Location::github("brasfoot-next/database"), repo("brasfoot-next", "database"));
        assert_eq!(Location::github("https://github.com/Fulano/Liga-Exemplo"), repo("fulano", "liga-exemplo"));
        assert_eq!(Location::github("github.com/fulano/liga.git"), repo("fulano", "liga"));
        assert_eq!(Location::github("https://github.com/fulano/liga/tree/main/clubs"), repo("fulano", "liga"));
        assert_eq!(Location::github(" https://www.github.com/fulano/liga/ "), repo("fulano", "liga"));
        assert_eq!(Location::github("fulano"), None);
        assert_eq!(Location::github("not a link/at all"), None);
        assert!(Location::github("Brasfoot-Next/Database").unwrap().is_official());
    }

    #[test]
    fn a_new_list_has_the_official_database_and_survives_a_round_trip() {
        let list = SourceList::default();
        assert!(list.sources[0].location.is_official() && list.sources[0].enabled && list.sources[0].installed.is_none());
        let text = serde_json::to_string(&list).unwrap();
        assert!(text.contains(r#""kind":"github""#));
        assert_eq!(serde_json::from_str::<SourceList>(&text).unwrap(), list);
    }

    #[test]
    fn enabled_downloaded_sources_compose_in_order_and_the_rest_are_skipped() {
        let store = MemStore::default();
        let zip = |files: &[(&str, &str)]| {
            let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            for (path, text) in files {
                z.start_file(*path, zip::write::SimpleFileOptions::default()).unwrap();
                std::io::Write::write_all(&mut z, text.as_bytes()).unwrap();
            }
            z.finish().unwrap().into_inner()
        };
        let base = zip(&[
            ("manifest.toml", "format = 1\nid = \"exemplo/base\"\nkind = \"base\"\nname = \"Base\"\n"),
            ("leagues/eng.toml", "country = \"ENG\"\n\n[[divisions]]\ndivision = 1\nteams = 20\n"),
        ]);
        let patch = zip(&[
            ("manifest.toml", "format = 1\nid = \"exemplo/patch\"\nkind = \"patch\"\nname = \"Patch\"\n"),
            ("leagues/eng.toml", "country = \"ENG\"\n\n[[divisions]]\ndivision = 1\nteams = 18\n"),
        ]);
        let base_at = Location::GitHub { owner: "exemplo".into(), repo: "base".into() };
        let patch_at = Location::GitHub { owner: "exemplo".into(), repo: "patch".into() };
        let missing_at = Location::GitHub { owner: "exemplo".into(), repo: "nunca-baixado".into() };
        let b = pollster::block_on(download::install_bytes(&store, &base_at.key(), "b.zip", base, None)).unwrap();
        let p = pollster::block_on(download::install_bytes(&store, &patch_at.key(), "p.zip", patch, None)).unwrap();
        let mut list = SourceList {
            sources: vec![
                Source { location: base_at.clone(), enabled: true, installed: Some(b) },
                Source { location: patch_at.clone(), enabled: true, installed: Some(p) },
                Source { location: missing_at.clone(), enabled: true, installed: None },
            ],
        };
        let loaded = pollster::block_on(compose(&store, &list)).unwrap();
        assert_eq!(loaded.layers.len(), 2);
        assert_eq!(loaded.skipped, [missing_at]);
        assert_eq!(loaded.database.content.leagues["ENG"].divisions[0].teams, 18);

        list.sources[1].enabled = false;
        let loaded = pollster::block_on(compose(&store, &list)).unwrap();
        assert_eq!(loaded.database.content.leagues["ENG"].divisions[0].teams, 20);

        list.sources[0].enabled = false;
        assert!(pollster::block_on(compose(&store, &list)).is_none());
    }
}
