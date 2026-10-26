//! Packs: a source cut into a few zip chunks plus `index.json`, for downloading. The data
//! (manifest, TOML, CSV) goes in `data.zip`; the pictures go by area, the clubs' by the first
//! letter of their country, so that a change touches few chunks. The chunks are deterministic
//! (sorted files, fixed dates): unchanged content keeps its hash and is not downloaded again.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

use crate::format::{FORMAT_VERSION, Manifest};
use crate::tree::{FileTree, MemTree};

pub const INDEX: &str = "index.json";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackIndex {
    pub format: u32,
    pub manifest: Manifest,
    pub chunks: Vec<Chunk>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    pub file: String,
    pub sha256: String,
    pub size: u64,
    pub files: usize,
}

impl PackIndex {
    pub fn total_size(&self) -> u64 {
        self.chunks.iter().map(|c| c.size).sum()
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The chunk a file of the pack goes in.
pub fn chunk_name(path: &str) -> String {
    if !path.ends_with(".png") {
        return "data.zip".to_owned();
    }
    let parts: Vec<&str> = path.split('/').collect();
    match parts.as_slice() {
        ["clubs", country, ..] => format!("clubs-{}.zip", country.chars().next().unwrap_or('_')),
        [area, ..] => format!("{area}.zip"),
        _ => "data.zip".to_owned(),
    }
}

/// Cuts `tree` into chunks in `out`, with `index.json`.
pub fn build(tree: &dyn FileTree, manifest: &Manifest, out: &Path) -> io::Result<PackIndex> {
    std::fs::create_dir_all(out)?;
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in tree.files() {
        groups.entry(chunk_name(&path)).or_default().push(path);
    }
    let mut chunks = Vec::new();
    for (name, paths) in groups {
        let mut zip = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        // pictures are already compressed
        let method = if name == "data.zip" { zip::CompressionMethod::Deflated } else { zip::CompressionMethod::Stored };
        let options = SimpleFileOptions::default().compression_method(method).last_modified_time(zip::DateTime::default()).unix_permissions(0o644);
        for path in &paths {
            zip.start_file(path.as_str(), options).map_err(io::Error::other)?;
            zip.write_all(&tree.read(path)?)?;
        }
        let bytes = zip.finish().map_err(io::Error::other)?.into_inner();
        std::fs::write(out.join(&name), &bytes)?;
        chunks.push(Chunk { sha256: sha256_hex(&bytes), size: bytes.len() as u64, files: paths.len(), file: name });
    }
    let index = PackIndex { format: FORMAT_VERSION, manifest: manifest.clone(), chunks };
    std::fs::write(out.join(INDEX), serde_json::to_vec_pretty(&index).map_err(io::Error::other)?)?;
    Ok(index)
}

pub fn parse_index(bytes: &[u8]) -> io::Result<PackIndex> {
    serde_json::from_slice(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// The files of downloaded chunks, after checking each against the index.
pub fn unpack(index: &PackIndex, read_chunk: impl Fn(&str) -> io::Result<Vec<u8>>) -> io::Result<MemTree> {
    let mut tree = MemTree::default();
    for chunk in &index.chunks {
        let bytes = read_chunk(&chunk.file)?;
        if sha256_hex(&bytes) != chunk.sha256 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("{} does not match its hash", chunk.file)));
        }
        let part = MemTree::from_zip(&bytes)?;
        for path in part.files() {
            let data = part.read(&path)?;
            tree.insert(path, data);
        }
    }
    Ok(tree)
}
