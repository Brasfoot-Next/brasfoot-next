//! The lists of first names and surnames the original draws player names from, read from the
//! game's `arquivos/names/<code>.txt` and `arquivos/surnames/<code>.txt` (`best.M.a`). Names are
//! game data, like the teams.

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct NameLists {
    /// The folder that holds `arquivos/` (the game's jar, unpacked).
    root: Option<PathBuf>,
    /// Or the files themselves, by path inside the jar (`arquivos/names/FRA.txt`).
    files: HashMap<String, String>,
    /// `M.gy` and `M.gz`: each list is read once per country and kept for the session.
    first: HashMap<i32, Vec<String>>,
    last: HashMap<i32, Vec<String>>,
}

impl NameLists {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        NameLists { root: Some(root.into()), ..Default::default() }
    }

    /// The lists read out of the game's jar.
    pub fn from_files(files: HashMap<String, String>) -> Self {
        NameLists { files, ..Default::default() }
    }

    /// `M.a(String, int, int)`: the list of a country, by its code; the cache is keyed by the
    /// country id, with 221 and 223 read as 85 and 222 as 72, as in the original.
    pub fn list(&mut self, country: i32, code: &str, surnames: bool) -> &[String] {
        let key = match country {
            221 | 223 => 85,
            222 => 72,
            other => other,
        };
        let cache = if surnames { &mut self.last } else { &mut self.first };
        if !cache.get(&key).is_some_and(|l| !l.is_empty()) {
            let folder = if surnames { "surnames" } else { "names" };
            let inside = format!("arquivos/{folder}/{code}.txt");
            let text = match &self.root {
                // read as UTF-8, a bad byte becoming U+FFFD, as the original's InputStreamReader
                // does (CRN.txt is ISO-8859-1)
                Some(root) => std::fs::read(root.join(&inside)).ok().map(|b| String::from_utf8_lossy(&b).into_owned()),
                None => self.files.get(&inside).cloned(),
            };
            let lines = text
                .map(|text| {
                    text.lines()
                        .filter(|l| !l.is_empty() && !l.contains('.') && !l.chars().any(|c| c.is_ascii_digit()))
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            cache.insert(key, lines);
        }
        &cache[&key]
    }
}
