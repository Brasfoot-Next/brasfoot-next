//! Where a pack's files come from: a folder, a zip in memory (a GitHub archive, a pack chunk) or
//! the layers of several sources seen as one.

use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// A read-only set of files with `/`-separated paths relative to the pack's root.
pub trait FileTree {
    /// Every file, sorted.
    fn files(&self) -> Vec<String>;
    fn read(&self, path: &str) -> io::Result<Vec<u8>>;
}

/// A folder on disk.
pub struct DirTree {
    root: PathBuf,
}

impl DirTree {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        DirTree { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl FileTree for DirTree {
    fn files(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut pending = vec![self.root.clone()];
        while let Some(dir) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                // hidden files and folders (.git, .github, .DS_Store) are never part of a pack
                if name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    pending.push(path);
                } else if let Ok(relative) = path.strip_prefix(&self.root) {
                    out.push(relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
                }
            }
        }
        out.sort();
        out
    }

    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        std::fs::read(self.root.join(path))
    }
}

/// Files held in memory.
#[derive(Default)]
pub struct MemTree {
    files: BTreeMap<String, Vec<u8>>,
}

impl MemTree {
    pub fn insert(&mut self, path: impl Into<String>, bytes: Vec<u8>) {
        self.files.insert(path.into(), bytes);
    }

    /// The files of a zip. A GitHub archive puts everything inside one folder
    /// (`<repo>-<ref>/`); when the manifest is there and not at the root, that folder is dropped.
    pub fn from_zip(bytes: &[u8]) -> io::Result<MemTree> {
        let mut archive = zip::ZipArchive::new(io::Cursor::new(bytes)).map_err(io::Error::other)?;
        let mut tree = MemTree::default();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(io::Error::other)?;
            if file.is_dir() {
                continue;
            }
            let Some(name) = file.enclosed_name() else { continue };
            let path = name.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
            let mut data = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut data)?;
            tree.files.insert(path, data);
        }
        if !tree.files.contains_key(crate::format::MANIFEST)
            && let Some(prefix) = tree.files.keys().find(|p| p.ends_with(&format!("/{}", crate::format::MANIFEST))).map(|p| p[..=p.find('/').unwrap_or(0)].to_owned())
            && tree.files.keys().all(|p| p.starts_with(&prefix))
        {
            tree.files = std::mem::take(&mut tree.files).into_iter().map(|(p, d)| (p[prefix.len()..].to_owned(), d)).collect();
        }
        tree.files.retain(|p, _| !p.split('/').any(|part| part.starts_with('.')));
        Ok(tree)
    }
}

impl FileTree for MemTree {
    fn files(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        self.files.get(path).cloned().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.to_owned()))
    }
}

/// Several sources seen as one: each path comes from the highest layer that has it, after the
/// removals of the layers above the one that brought it.
pub struct LayeredTree<'a> {
    layers: Vec<&'a dyn FileTree>,
    /// path -> layer
    owner: BTreeMap<String, usize>,
}

impl<'a> LayeredTree<'a> {
    /// `removed(i, path)` says whether layer `i` takes `path` away from the layers below it.
    pub fn new(layers: Vec<&'a dyn FileTree>, removed: impl Fn(usize, &str) -> bool) -> Self {
        let mut owner = BTreeMap::new();
        for (i, layer) in layers.iter().enumerate() {
            owner.retain(|path: &String, _| !removed(i, path));
            for path in layer.files() {
                // the manifest belongs to each source, not to the stack
                if path != crate::format::MANIFEST {
                    owner.insert(path, i);
                }
            }
        }
        LayeredTree { layers, owner }
    }

    /// The layer a path comes from.
    pub fn layer_of(&self, path: &str) -> Option<usize> {
        self.owner.get(path).copied()
    }
}

impl FileTree for LayeredTree<'_> {
    fn files(&self) -> Vec<String> {
        self.owner.keys().cloned().collect()
    }

    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        match self.owner.get(path) {
            Some(&i) => self.layers[i].read(path),
            None => Err(io::Error::new(io::ErrorKind::NotFound, path.to_owned())),
        }
    }
}
