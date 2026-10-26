//! Downloading a GitHub source into a store. A repository with a `pack` branch (made by the
//! data pack workflow) is fetched as its `index.json` and the chunks whose hash the store does
//! not have yet; one without it is fetched file by file from its default branch, waiting when
//! GitHub limits the requests. Each source keeps its chunks under `<source key>/<sha256>.zip`.

use std::future::Future;
use std::io::Write;
use std::sync::{Arc, Mutex};

use brasfoot_datapack::format::{FORMAT_VERSION, MANIFEST};
use brasfoot_datapack::pack::{self, Chunk, PackIndex};
use brasfoot_datapack::read;
use brasfoot_datapack::tree::MemTree;

use super::Installed;
use super::store::Store;

pub struct Response {
    pub status: u16,
    pub bytes: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

impl Response {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

pub trait Fetch {
    fn get(&self, url: &str) -> impl Future<Output = Result<Response, String>>;
}

/// The real network.
#[derive(Clone, Copy)]
pub struct Http;

impl Fetch for Http {
    async fn get(&self, url: &str) -> Result<Response, String> {
        #[allow(unused_mut)]
        let mut request = ehttp::Request::get(url);
        // the GitHub API refuses requests without one; browsers send their own
        #[cfg(not(target_arch = "wasm32"))]
        request.headers.insert("User-Agent", concat!("brasfoot-next/", env!("CARGO_PKG_VERSION")));
        let r = ehttp::fetch_async(request).await?;
        let headers = r.headers.headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        Ok(Response { status: r.status, bytes: r.bytes, headers })
    }
}

/// What a download is doing, for the screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Starting,
    /// Bytes of the chunks still missing.
    Chunks { done: u64, total: u64 },
    /// Files of a repository without a pack.
    Files { done: usize, total: usize },
    /// GitHub limited the requests: the download goes on after this many seconds.
    Waiting { seconds: u64 },
    Saving,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DownloadError {
    Network(String),
    /// No such repository, or a private one.
    NotFound,
    /// The repository has no `manifest.toml`.
    NotAPack,
    UnsupportedFormat(u32),
    /// A chunk did not match its hash (the pack changed during the download, say).
    Corrupted(String),
    Storage(String),
    RateLimited,
}

pub type Progress = Arc<Mutex<Phase>>;

fn set(progress: &Progress, phase: Phase) {
    if let Ok(mut p) = progress.lock() {
        *p = phase;
    }
}

const RAW: &str = "https://raw.githubusercontent.com";
const API: &str = "https://api.github.com";
/// The longest wait for GitHub's limit before giving up.
const LONGEST_WAIT: u64 = 15 * 60;

/// Seconds GitHub asks to wait, when it is limiting the requests.
fn limited(r: &Response) -> Option<u64> {
    let limited = r.status == 429 || (r.status == 403 && r.header("x-ratelimit-remaining") == Some("0"));
    if !limited {
        return None;
    }
    if let Some(s) = r.header("retry-after").and_then(|v| v.trim().parse().ok()) {
        return Some(s);
    }
    let reset: Option<u64> = r.header("x-ratelimit-reset").and_then(|v| v.trim().parse().ok());
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    Some(reset.map_or(60, |reset| reset.saturating_sub(now).max(1)))
}

/// A GET that waits out GitHub's limits.
async fn get(fetch: &impl Fetch, url: &str, progress: &Progress) -> Result<Response, DownloadError> {
    for _ in 0..4 {
        let r = fetch.get(url).await.map_err(DownloadError::Network)?;
        match limited(&r) {
            Some(seconds) if seconds <= LONGEST_WAIT => {
                let before = progress.lock().map(|p| p.clone()).unwrap_or(Phase::Starting);
                set(progress, Phase::Waiting { seconds });
                super::sleep(seconds).await;
                set(progress, before);
            }
            Some(_) => return Err(DownloadError::RateLimited),
            None => return Ok(r),
        }
    }
    Err(DownloadError::RateLimited)
}

fn ok(r: Response) -> Result<Vec<u8>, DownloadError> {
    match r.status {
        200 => Ok(r.bytes),
        404 => Err(DownloadError::NotFound),
        s => Err(DownloadError::Network(format!("HTTP {s}"))),
    }
}

/// `path` with every byte outside the unreserved set percent-encoded, `/` kept.
fn encode(path: &str) -> String {
    let mut out = String::new();
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The commit a branch points at, through the API. `None` when the API does not answer (its
/// limit is 60 requests an hour without an account): the branch name is used then.
async fn branch_commit(fetch: &impl Fetch, owner: &str, repo: &str, branch: &str) -> Result<Option<String>, DownloadError> {
    let r = fetch.get(&format!("{API}/repos/{owner}/{repo}/branches/{branch}")).await.map_err(DownloadError::Network)?;
    match r.status {
        200 => {
            let v: serde_json::Value = serde_json::from_slice(&r.bytes).map_err(|e| DownloadError::Network(e.to_string()))?;
            Ok(v["commit"]["sha"].as_str().map(str::to_owned))
        }
        404 => Err(DownloadError::NotFound),
        _ => Ok(None),
    }
}

/// The pack's index and the ref its chunks are read at, or `None` without a pack.
async fn pack_index(fetch: &impl Fetch, owner: &str, repo: &str, progress: &Progress) -> Result<Option<(PackIndex, String)>, DownloadError> {
    // pinned to the branch's commit, so that a new pack published meanwhile cannot mix in
    let at = match branch_commit(fetch, owner, repo, "pack").await {
        Ok(commit) => commit.unwrap_or_else(|| "pack".to_owned()),
        Err(DownloadError::NotFound) => return Ok(None),
        Err(e) => return Err(e),
    };
    let r = get(fetch, &format!("{RAW}/{owner}/{repo}/{at}/{}", pack::INDEX), progress).await?;
    if r.status == 404 {
        return Ok(None);
    }
    let index = pack::parse_index(&ok(r)?).map_err(|e| DownloadError::Corrupted(e.to_string()))?;
    if index.format != FORMAT_VERSION {
        return Err(DownloadError::UnsupportedFormat(index.format));
    }
    Ok(Some((index, at)))
}

/// What identifies a pack's version: its source commit, or the hashes of its chunks.
fn pack_version(index: &PackIndex) -> String {
    index.commit.clone().unwrap_or_else(|| pack::sha256_hex(index.chunks.iter().map(|c| c.sha256.as_str()).collect::<String>().as_bytes()))
}

/// The version the repository has now, to tell whether an update is there.
pub async fn latest_version(fetch: &impl Fetch, owner: &str, repo: &str) -> Result<String, DownloadError> {
    let progress: Progress = Arc::new(Mutex::new(Phase::Starting));
    if let Some((index, _)) = pack_index(fetch, owner, repo, &progress).await? {
        return Ok(pack_version(&index));
    }
    let branch = default_branch(fetch, owner, repo, &progress).await?;
    branch_commit(fetch, owner, repo, &branch).await?.ok_or(DownloadError::RateLimited)
}

async fn default_branch(fetch: &impl Fetch, owner: &str, repo: &str, progress: &Progress) -> Result<String, DownloadError> {
    let bytes = ok(get(fetch, &format!("{API}/repos/{owner}/{repo}"), progress).await?)?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| DownloadError::Network(e.to_string()))?;
    Ok(v["default_branch"].as_str().unwrap_or("main").to_owned())
}

/// Removes the chunks of `key` the new version no longer has.
async fn prune(store: &impl Store, key: &str, chunks: &[Chunk]) {
    for k in store.keys(&format!("{key}/")).await {
        if !chunks.iter().any(|c| k == chunk_key(key, &c.sha256)) {
            store.remove(&k).await;
        }
    }
}

pub fn chunk_key(source: &str, sha256: &str) -> String {
    format!("{source}/{sha256}.zip")
}

/// Downloads `owner/repo` into `store` under `key`. Chunks already there are not downloaded again.
pub async fn install_github(fetch: &impl Fetch, store: &impl Store, key: &str, owner: &str, repo: &str, progress: &Progress) -> Result<Installed, DownloadError> {
    set(progress, Phase::Starting);
    let Some((index, at)) = pack_index(fetch, owner, repo, progress).await? else {
        return install_files(fetch, store, key, owner, repo, progress).await;
    };
    let have = store.keys(&format!("{key}/")).await;
    let missing: Vec<&Chunk> = index.chunks.iter().filter(|c| !have.contains(&chunk_key(key, &c.sha256))).collect();
    let total = missing.iter().map(|c| c.size).sum();
    let mut done = 0;
    set(progress, Phase::Chunks { done, total });
    for chunk in missing {
        let bytes = ok(get(fetch, &format!("{RAW}/{owner}/{repo}/{at}/{}", encode(&chunk.file)), progress).await?)?;
        if pack::sha256_hex(&bytes) != chunk.sha256 {
            return Err(DownloadError::Corrupted(chunk.file.clone()));
        }
        store.write(&chunk_key(key, &chunk.sha256), bytes).await.map_err(DownloadError::Storage)?;
        done += chunk.size;
        set(progress, Phase::Chunks { done, total });
    }
    set(progress, Phase::Saving);
    prune(store, key, &index.chunks).await;
    Ok(Installed { version: Some(pack_version(&index)), manifest: index.manifest, chunks: index.chunks })
}

/// Whether a repository file belongs to a pack (the rest is never downloaded).
fn pack_file(path: &str) -> bool {
    let top = path.split('/').next().unwrap_or("");
    path == MANIFEST || (matches!(top, "clubs" | "leagues" | "state-leagues" | "national-teams" | "trophies") && !path.split('/').any(|p| p.starts_with('.')))
}

/// A repository without a pack: the files of its default branch, one by one, made into one chunk.
async fn install_files(fetch: &impl Fetch, store: &impl Store, key: &str, owner: &str, repo: &str, progress: &Progress) -> Result<Installed, DownloadError> {
    let branch = default_branch(fetch, owner, repo, progress).await?;
    let commit = branch_commit(fetch, owner, repo, &branch).await?.ok_or(DownloadError::RateLimited)?;
    let tree = ok(get(fetch, &format!("{API}/repos/{owner}/{repo}/git/trees/{commit}?recursive=1"), progress).await?)?;
    let tree: serde_json::Value = serde_json::from_slice(&tree).map_err(|e| DownloadError::Network(e.to_string()))?;
    let mut paths: Vec<String> = tree["tree"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["type"] == "blob")
        .filter_map(|e| e["path"].as_str().map(str::to_owned))
        .filter(|p| pack_file(p))
        .collect();
    paths.sort();
    if !paths.iter().any(|p| p == MANIFEST) {
        return Err(DownloadError::NotAPack);
    }
    let total = paths.len();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (i, path) in paths.iter().enumerate() {
        set(progress, Phase::Files { done: i, total });
        let bytes = ok(get(fetch, &format!("{RAW}/{owner}/{repo}/{commit}/{}", encode(path)), progress).await?)?;
        let method = if path.ends_with(".png") { zip::CompressionMethod::Stored } else { zip::CompressionMethod::Deflated };
        zip.start_file(path.as_str(), zip::write::SimpleFileOptions::default().compression_method(method)).map_err(|e| DownloadError::Storage(e.to_string()))?;
        zip.write_all(&bytes).map_err(|e| DownloadError::Storage(e.to_string()))?;
    }
    set(progress, Phase::Saving);
    let bytes = zip.finish().map_err(|e| DownloadError::Storage(e.to_string()))?.into_inner();
    install_bytes(store, key, "repository.zip", bytes, Some(commit)).await
}

/// A zip (a GitHub archive, a pack someone sent) as the only chunk of `key`.
pub async fn install_bytes(store: &impl Store, key: &str, name: &str, bytes: Vec<u8>, version: Option<String>) -> Result<Installed, DownloadError> {
    let tree = MemTree::from_zip(&bytes).map_err(|e| DownloadError::Corrupted(e.to_string()))?;
    let manifest = read::read_manifest(&tree).map_err(|issue| match issue.kind {
        brasfoot_datapack::issues::IssueKind::UnsupportedFormat { found } => DownloadError::UnsupportedFormat(found),
        _ => DownloadError::NotAPack,
    })?;
    let chunk = Chunk { file: name.to_owned(), sha256: pack::sha256_hex(&bytes), size: bytes.len() as u64, files: brasfoot_datapack::tree::FileTree::files(&tree).len() };
    store.write(&chunk_key(key, &chunk.sha256), bytes).await.map_err(DownloadError::Storage)?;
    let chunks = vec![chunk];
    prune(store, key, &chunks).await;
    Ok(Installed { manifest, version, chunks })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::patches::store::MemStore;

    /// Status, body and headers.
    type Answer = (u16, Vec<u8>, Vec<(String, String)>);

    /// Answers from a map; anything else is a 404. Counts the requests.
    #[derive(Default)]
    struct Fake {
        answers: HashMap<String, Answer>,
        asked: Mutex<Vec<String>>,
    }

    impl Fake {
        fn answer(&mut self, url: &str, status: u16, bytes: Vec<u8>) {
            self.answers.insert(url.to_owned(), (status, bytes, Vec::new()));
        }
    }

    impl Fetch for Fake {
        async fn get(&self, url: &str) -> Result<Response, String> {
            self.asked.lock().unwrap().push(url.to_owned());
            let (status, bytes, headers) = self.answers.get(url).cloned().unwrap_or((404, Vec::new(), Vec::new()));
            Ok(Response { status, bytes, headers })
        }
    }

    fn block_on<F: Future>(f: F) -> F::Output {
        pollster::block_on(f)
    }

    const MANIFEST_TEXT: &str = "format = 1\nid = \"exemplo/base\"\nkind = \"base\"\nname = \"Base de exemplo\"\n";

    /// A pack with a data chunk and a picture chunk, published at commit `c1` of the pack branch.
    fn packed(fake: &mut Fake, picture: &[u8]) -> PackIndex {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("manifest.toml"), MANIFEST_TEXT).unwrap();
        std::fs::create_dir_all(root.join("trophies/cup")).unwrap();
        std::fs::write(root.join("trophies/cup/bra.png"), picture).unwrap();
        let tree = brasfoot_datapack::tree::DirTree::new(root);
        let manifest = read::read_manifest(&tree).unwrap();
        let out = tempfile::tempdir().unwrap();
        let index = pack::build(&tree, &manifest, Some("src1"), out.path()).unwrap();
        fake.answer(&format!("{API}/repos/exemplo/base/branches/pack"), 200, br#"{"commit":{"sha":"c1"}}"#.to_vec());
        fake.answer(&format!("{RAW}/exemplo/base/c1/index.json"), 200, std::fs::read(out.path().join("index.json")).unwrap());
        for c in &index.chunks {
            fake.answer(&format!("{RAW}/exemplo/base/c1/{}", c.file), 200, std::fs::read(out.path().join(&c.file)).unwrap());
        }
        index
    }

    #[test]
    fn a_pack_is_downloaded_once_and_updates_fetch_only_changed_chunks() {
        let mut fake = Fake::default();
        let first = packed(&mut fake, b"\x89PNGone");
        let store = MemStore::default();
        let progress: Progress = Arc::new(Mutex::new(Phase::Starting));
        let installed = block_on(install_github(&fake, &store, "github/exemplo/base", "exemplo", "base", &progress)).unwrap();
        assert_eq!(installed.version.as_deref(), Some("src1"));
        assert_eq!(installed.manifest.name, "Base de exemplo");
        assert_eq!(installed.chunks, first.chunks);
        assert_eq!(block_on(store.keys("github/exemplo/base/")).len(), 2);
        assert_eq!(*progress.lock().unwrap(), Phase::Saving);

        // a new picture: only the trophies chunk is fetched again, the old one is removed
        let second = packed(&mut fake, b"\x89PNGtwo");
        fake.asked.lock().unwrap().clear();
        let updated = block_on(install_github(&fake, &store, "github/exemplo/base", "exemplo", "base", &progress)).unwrap();
        assert_eq!(updated.chunks, second.chunks);
        let asked = fake.asked.lock().unwrap().clone();
        assert!(asked.iter().any(|u| u.ends_with("/trophies.zip")));
        assert!(!asked.iter().any(|u| u.ends_with("/data.zip")));
        let keys = block_on(store.keys("github/exemplo/base/"));
        assert_eq!(keys.len(), 2);
        assert!(second.chunks.iter().all(|c| keys.contains(&chunk_key("github/exemplo/base", &c.sha256))));
    }

    #[test]
    fn a_chunk_that_does_not_match_its_hash_is_refused() {
        let mut fake = Fake::default();
        let index = packed(&mut fake, b"\x89PNGone");
        fake.answer(&format!("{RAW}/exemplo/base/c1/{}", index.chunks[0].file), 200, b"changed".to_vec());
        let store = MemStore::default();
        let progress: Progress = Arc::new(Mutex::new(Phase::Starting));
        let result = block_on(install_github(&fake, &store, "k", "exemplo", "base", &progress));
        assert_eq!(result, Err(DownloadError::Corrupted(index.chunks[0].file.clone())));
    }

    #[test]
    fn a_repository_without_a_pack_is_fetched_file_by_file() {
        let mut fake = Fake::default();
        fake.answer(&format!("{API}/repos/exemplo/patch"), 200, br#"{"default_branch":"main"}"#.to_vec());
        fake.answer(&format!("{API}/repos/exemplo/patch/branches/main"), 200, br#"{"commit":{"sha":"abc"}}"#.to_vec());
        let tree = r#"{"tree":[
            {"path":"manifest.toml","type":"blob"},
            {"path":"README.md","type":"blob"},
            {"path":"clubs","type":"tree"},
            {"path":"clubs/bra/vila-exemplo/squad.csv","type":"blob"},
            {"path":".github/workflows/pack.yml","type":"blob"}]}"#;
        fake.answer(&format!("{API}/repos/exemplo/patch/git/trees/abc?recursive=1"), 200, tree.as_bytes().to_vec());
        let manifest = "format = 1\nid = \"exemplo/patch\"\nkind = \"patch\"\nname = \"Patch de exemplo\"\n";
        fake.answer(&format!("{RAW}/exemplo/patch/abc/manifest.toml"), 200, manifest.as_bytes().to_vec());
        fake.answer(&format!("{RAW}/exemplo/patch/abc/clubs/bra/vila-exemplo/squad.csv"), 200, b"name\n".to_vec());
        let store = MemStore::default();
        let progress: Progress = Arc::new(Mutex::new(Phase::Starting));
        let installed = block_on(install_github(&fake, &store, "github/exemplo/patch", "exemplo", "patch", &progress)).unwrap();
        assert_eq!(installed.version.as_deref(), Some("abc"));
        assert_eq!(installed.chunks.len(), 1);
        assert_eq!(installed.chunks[0].files, 2);
        let asked = fake.asked.lock().unwrap().clone();
        assert!(!asked.iter().any(|u| u.ends_with("README.md") || u.contains("/.github/")));
    }

    #[test]
    fn a_missing_repository_and_a_repository_that_is_not_a_pack_are_told_apart() {
        let fake = Fake::default();
        let store = MemStore::default();
        let progress: Progress = Arc::new(Mutex::new(Phase::Starting));
        assert_eq!(block_on(install_github(&fake, &store, "k", "ninguem", "nada", &progress)), Err(DownloadError::NotFound));

        let mut fake = Fake::default();
        fake.answer(&format!("{API}/repos/exemplo/outro"), 200, br#"{"default_branch":"main"}"#.to_vec());
        fake.answer(&format!("{API}/repos/exemplo/outro/branches/main"), 200, br#"{"commit":{"sha":"abc"}}"#.to_vec());
        fake.answer(&format!("{API}/repos/exemplo/outro/git/trees/abc?recursive=1"), 200, br#"{"tree":[{"path":"src/main.rs","type":"blob"}]}"#.to_vec());
        assert_eq!(block_on(install_github(&fake, &store, "k", "exemplo", "outro", &progress)), Err(DownloadError::NotAPack));
    }

    #[test]
    fn rate_limits_are_read_from_the_headers() {
        let r = |status, headers: &[(&str, &str)]| Response { status, bytes: Vec::new(), headers: headers.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() };
        assert_eq!(limited(&r(429, &[("Retry-After", "7")])), Some(7));
        assert_eq!(limited(&r(429, &[])), Some(60));
        assert_eq!(limited(&r(403, &[("x-ratelimit-remaining", "0"), ("x-ratelimit-reset", "1")])), Some(1));
        assert_eq!(limited(&r(403, &[])), None);
        assert_eq!(limited(&r(200, &[])), None);
        assert_eq!(encode("clubs/bra/são paulo.png"), "clubs/bra/s%C3%A3o%20paulo.png");
    }
}
