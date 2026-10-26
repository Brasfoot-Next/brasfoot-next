//! Where downloaded packs and the list of sources are kept: files on the desktop, IndexedDB in
//! the browser, memory in tests. Keys are `/`-separated (`github/owner/repo/<sha256>.zip`).

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

pub trait Store: Clone + 'static {
    fn read(&self, key: &str) -> impl Future<Output = Option<Vec<u8>>>;
    fn write(&self, key: &str, bytes: Vec<u8>) -> impl Future<Output = Result<(), String>>;
    fn remove(&self, key: &str) -> impl Future<Output = ()>;
    /// The keys that start with `prefix`.
    fn keys(&self, prefix: &str) -> impl Future<Output = Vec<String>>;
}

/// For tests.
#[derive(Clone, Default)]
pub struct MemStore(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

impl Store for MemStore {
    async fn read(&self, key: &str) -> Option<Vec<u8>> {
        self.0.lock().ok()?.get(key).cloned()
    }

    async fn write(&self, key: &str, bytes: Vec<u8>) -> Result<(), String> {
        self.0.lock().map_err(|e| e.to_string())?.insert(key.to_owned(), bytes);
        Ok(())
    }

    async fn remove(&self, key: &str) {
        if let Ok(mut m) = self.0.lock() {
            m.remove(key);
        }
    }

    async fn keys(&self, prefix: &str) -> Vec<String> {
        self.0.lock().map(|m| m.keys().filter(|k| k.starts_with(prefix)).cloned().collect()).unwrap_or_default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use disk::DiskStore as PlatformStore;
#[cfg(target_arch = "wasm32")]
pub use web::IdbStore as PlatformStore;

#[cfg(not(target_arch = "wasm32"))]
mod disk {
    use std::path::{Path, PathBuf};

    use super::Store;

    /// A folder, `<data dir>/patches`.
    #[derive(Clone)]
    pub struct DiskStore {
        root: PathBuf,
    }

    impl DiskStore {
        pub fn open() -> Self {
            DiskStore { root: crate::game_files::data_dir().join("patches") }
        }

        pub fn at(root: impl Into<PathBuf>) -> Self {
            DiskStore { root: root.into() }
        }

        fn path(&self, key: &str) -> Option<PathBuf> {
            let safe = !key.is_empty() && key.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
            safe.then(|| self.root.join(key))
        }
    }

    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
            }
        }
    }

    impl Store for DiskStore {
        async fn read(&self, key: &str) -> Option<Vec<u8>> {
            std::fs::read(self.path(key)?).ok()
        }

        async fn write(&self, key: &str, bytes: Vec<u8>) -> Result<(), String> {
            let path = self.path(key).ok_or_else(|| format!("bad key {key}"))?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            // written aside and renamed, so a crash never leaves half a file under the key
            let tmp = path.with_extension("part");
            std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
        }

        async fn remove(&self, key: &str) {
            if let Some(path) = self.path(key) {
                let _ = std::fs::remove_file(path);
            }
        }

        async fn keys(&self, prefix: &str) -> Vec<String> {
            let mut out = Vec::new();
            walk(&self.root, &self.root, &mut out);
            out.retain(|k| k.starts_with(prefix) && !k.ends_with(".part"));
            out.sort();
            out
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{IdbDatabase, IdbRequest, IdbTransactionMode};

    use super::Store;

    const DATABASE: &str = "brasfoot-next";
    const FILES: &str = "patches";

    /// The `patches` object store of the `brasfoot-next` IndexedDB database.
    #[derive(Clone)]
    pub struct IdbStore;

    impl IdbStore {
        pub fn open() -> Self {
            IdbStore
        }
    }

    /// The result of a request, once it succeeds.
    async fn wait(request: &IdbRequest) -> Result<JsValue, JsValue> {
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let done = request.clone();
            let ok = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = resolve.call1(&JsValue::NULL, &done.result().unwrap_or(JsValue::UNDEFINED));
            });
            let failed = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("IndexedDB request failed"));
            });
            request.set_onsuccess(Some(ok.unchecked_ref()));
            request.set_onerror(Some(failed.unchecked_ref()));
        });
        JsFuture::from(promise).await
    }

    async fn database() -> Result<IdbDatabase, JsValue> {
        let factory = web_sys::window().ok_or("no window")?.indexed_db()?.ok_or("no IndexedDB")?;
        let open = factory.open_with_u32(DATABASE, 1)?;
        let upgrade = Closure::once_into_js(move |event: web_sys::Event| {
            let request: web_sys::IdbOpenDbRequest = event.target().expect("the open request").unchecked_into();
            if let Ok(result) = request.result() {
                let db: IdbDatabase = result.unchecked_into();
                let _ = db.create_object_store(FILES);
            }
        });
        open.set_onupgradeneeded(Some(upgrade.unchecked_ref()));
        Ok(wait(&open).await?.unchecked_into())
    }

    async fn store(mode: IdbTransactionMode) -> Result<web_sys::IdbObjectStore, JsValue> {
        database().await?.transaction_with_str_and_mode(FILES, mode)?.object_store(FILES)
    }

    impl Store for IdbStore {
        async fn read(&self, key: &str) -> Option<Vec<u8>> {
            let request = store(IdbTransactionMode::Readonly).await.ok()?.get(&JsValue::from_str(key)).ok()?;
            let value = wait(&request).await.ok()?;
            value.dyn_into::<js_sys::Uint8Array>().ok().map(|a| a.to_vec())
        }

        async fn write(&self, key: &str, bytes: Vec<u8>) -> Result<(), String> {
            let fail = |e: JsValue| format!("{e:?}");
            let array = js_sys::Uint8Array::from(bytes.as_slice());
            let request = store(IdbTransactionMode::Readwrite).await.map_err(fail)?.put_with_key(&array, &JsValue::from_str(key)).map_err(fail)?;
            wait(&request).await.map(|_| ()).map_err(fail)
        }

        async fn remove(&self, key: &str) {
            if let Ok(s) = store(IdbTransactionMode::Readwrite).await
                && let Ok(request) = s.delete(&JsValue::from_str(key))
            {
                let _ = wait(&request).await;
            }
        }

        async fn keys(&self, prefix: &str) -> Vec<String> {
            let Ok(s) = store(IdbTransactionMode::Readonly).await else { return Vec::new() };
            let Ok(request) = s.get_all_keys() else { return Vec::new() };
            let Ok(keys) = wait(&request).await else { return Vec::new() };
            let keys: js_sys::Array = keys.unchecked_into();
            let mut out: Vec<String> = keys.iter().filter_map(|k| k.as_string()).filter(|k| k.starts_with(prefix)).collect();
            out.sort();
            out
        }
    }
}
