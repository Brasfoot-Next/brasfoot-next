//! The native save format, `<name>.bfn`. See docs/save.md.
//!
//! A `.bfn` is a zstd stream of two frames:
//! 1. a skippable frame (ignored by zstd tools) holding `BFNX` and the [`SaveMeta`] as JSON, so the
//!    load screen reads a few hundred bytes instead of the whole save;
//! 2. a regular zstd frame, with content checksum, holding the payload as JSON.
//!
//! So `zstdcat career.bfn | jq` shows the whole save. Payloads are versioned: an older save is
//! upgraded by the [`Migration`]s on its JSON before it is deserialized.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Bumped whenever the payload changes in a way `#[serde(default)]` cannot absorb; each bump adds
/// a [`Migration`].
pub const FORMAT_VERSION: u32 = 3;
pub const EXTENSION: &str = "bfn";
/// How many previous versions of a save are kept next to it (`<name>.bfn.bak1` is the newest).
pub const BACKUPS: u32 = 3;

const SKIPPABLE_FRAME_MAGIC: u32 = 0x184D_2A50;
const HEADER_TAG: &[u8; 4] = b"BFNX";
const COMPRESSION_LEVEL: i32 = 3;

/// What the load screen shows, readable without loading the save.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SaveMeta {
    pub format_version: u32,
    /// Version of the app that wrote the save.
    pub app_version: String,
    pub club: String,
    pub coach: String,
    /// Game date, ISO `yyyy-mm-dd`.
    pub date: String,
    pub season: i32,
    /// Unix time of the save.
    pub saved_at: u64,
    /// Original save this career was imported from (`career.s22`), if any.
    #[serde(default)]
    pub imported_from: Option<String>,
}

/// Upgrades a payload written with `from` to `from + 1`.
pub struct Migration {
    pub from: u32,
    pub apply: fn(&mut ciborium::Value),
}

/// Payload upgrades, in order.
pub const MIGRATIONS: &[Migration] = &[
    Migration { from: 1, apply: crate::migrations::add_save_numbers },
    Migration { from: 2, apply: crate::migrations::name_national_tournaments },
];

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    NotASave,
    /// Written by a newer version of the app.
    TooNew { version: u32 },
    Json(serde_json::Error),
    Cbor(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "save: {e}"),
            SaveError::NotASave => write!(f, "save: not a Brasfoot Next save"),
            SaveError::TooNew { version } => write!(f, "save: format {version} is newer than {FORMAT_VERSION}"),
            SaveError::Json(e) => write!(f, "save: {e}"),
            SaveError::Cbor(e) => write!(f, "save: {e}"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        SaveError::Io(e)
    }
}

impl From<serde_json::Error> for SaveError {
    fn from(e: serde_json::Error) -> Self {
        SaveError::Json(e)
    }
}

/// Encodes a save to an in-memory byte buffer (skippable frame with meta, then zstd CBOR payload).
pub fn encode_save<T: Serialize>(meta: &SaveMeta, payload: &T) -> Result<Vec<u8>, SaveError> {
    let meta = SaveMeta { format_version: FORMAT_VERSION, ..meta.clone() };
    let mut header = HEADER_TAG.to_vec();
    serde_json::to_writer(&mut header, &meta)?;
    let mut out = Vec::new();
    out.write_all(&SKIPPABLE_FRAME_MAGIC.to_le_bytes())?;
    out.write_all(&(header.len() as u32).to_le_bytes())?;
    out.write_all(&header)?;
    let mut encoder = zstd::Encoder::new(out, COMPRESSION_LEVEL)?;
    encoder.include_checksum(true)?;
    ciborium::into_writer(payload, &mut encoder).map_err(|e| SaveError::Cbor(e.to_string()))?;
    let out = encoder.finish()?;
    Ok(out)
}

/// Writes a save atomically (temporary file, fsync, rename) and rotates the previous versions
/// into `.bak1`..`.bakN`. `meta.format_version` is set to [`FORMAT_VERSION`].
pub fn write_save<T: Serialize>(path: &Path, meta: &SaveMeta, payload: &T) -> Result<(), SaveError> {
    let bytes = encode_save(meta, payload)?;
    let temporary = sibling(path, "tmp");
    {
        let mut file = BufWriter::with_capacity(1 << 20, File::create(&temporary)?);
        file.write_all(&bytes)?;
        file.into_inner().map_err(|e| e.into_error())?.sync_all()?;
    }
    if path.exists() {
        rotate_backups(path)?;
    }
    fs::rename(&temporary, path)?;
    Ok(())
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

fn rotate_backups(path: &Path) -> io::Result<()> {
    for n in (1..BACKUPS).rev() {
        let older = sibling(path, &format!("bak{n}"));
        if older.exists() {
            fs::rename(&older, sibling(path, &format!("bak{}", n + 1)))?;
        }
    }
    fs::copy(path, sibling(path, "bak1"))?;
    Ok(())
}

fn read_header(reader: &mut impl Read) -> Result<SaveMeta, SaveError> {
    let mut word = [0u8; 4];
    reader.read_exact(&mut word).map_err(|_| SaveError::NotASave)?;
    if u32::from_le_bytes(word) != SKIPPABLE_FRAME_MAGIC {
        return Err(SaveError::NotASave);
    }
    reader.read_exact(&mut word)?;
    let mut header = vec![0u8; u32::from_le_bytes(word) as usize];
    reader.read_exact(&mut header)?;
    let json = header.strip_prefix(HEADER_TAG).ok_or(SaveError::NotASave)?;
    let meta: SaveMeta = serde_json::from_slice(json)?;
    if meta.format_version > FORMAT_VERSION {
        return Err(SaveError::TooNew { version: meta.format_version });
    }
    Ok(meta)
}

/// Reads only the metadata from a reader.
pub fn decode_meta(reader: &mut impl Read) -> Result<SaveMeta, SaveError> {
    read_header(reader)
}

/// Reads only the metadata, for the load screen.
pub fn read_meta(path: &Path) -> Result<SaveMeta, SaveError> {
    decode_meta(&mut BufReader::new(File::open(path)?))
}

/// Decodes a save from a reader, upgrading older payloads.
pub fn decode_save<T: DeserializeOwned>(reader: impl Read) -> Result<(SaveMeta, T), SaveError> {
    let mut reader = BufReader::new(reader);
    let meta = read_header(&mut reader)?;
    let decoder = zstd::Decoder::with_buffer(reader)?;
    let cbor = |e: &dyn std::fmt::Display| SaveError::Cbor(e.to_string());
    let payload = if meta.format_version == FORMAT_VERSION {
        ciborium::from_reader(BufReader::new(decoder)).map_err(|e| cbor(&e))?
    } else {
        let mut value: ciborium::Value = ciborium::from_reader(BufReader::new(decoder)).map_err(|e| cbor(&e))?;
        for version in meta.format_version..FORMAT_VERSION {
            for migration in MIGRATIONS.iter().filter(|m| m.from == version) {
                (migration.apply)(&mut value);
            }
        }
        value.deserialized().map_err(|e| cbor(&e))?
    };
    Ok((meta, payload))
}

/// Reads a save, upgrading older payloads. The returned meta keeps the version it was written
/// with.
pub fn read_save<T: DeserializeOwned>(path: &Path) -> Result<(SaveMeta, T), SaveError> {
    decode_save(BufReader::with_capacity(1 << 20, File::open(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Payload {
        clubs: Vec<String>,
        cash: i64,
    }

    fn meta() -> SaveMeta {
        SaveMeta { club: "Esporte Clube Fictício".into(), coach: "Técnico de Teste".into(), date: "2031-03-15".into(), season: 2031, ..Default::default() }
    }

    #[test]
    fn round_trip_and_meta_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fictional.bfn");
        let payload = Payload { clubs: vec!["Esporte Clube Fictício".into(), "Atlético Imaginário".into()], cash: -12 };
        write_save(&path, &meta(), &payload).unwrap();
        let read_meta = read_meta(&path).unwrap();
        assert_eq!(read_meta.club, "Esporte Clube Fictício");
        assert_eq!(read_meta.format_version, FORMAT_VERSION);
        let (_, read): (SaveMeta, Payload) = read_save(&path).unwrap();
        assert_eq!(read, payload);
    }

    #[test]
    fn plain_zstd_tools_see_the_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bfn");
        write_save(&path, &meta(), &Payload { clubs: vec![], cash: 7 }).unwrap();
        // a stock zstd decoder skips the header frame and yields the CBOR payload
        let bytes = zstd::decode_all(File::open(&path).unwrap()).unwrap();
        let back: Payload = ciborium::from_reader(bytes.as_slice()).unwrap();
        assert_eq!(back, Payload { clubs: vec![], cash: 7 });
    }

    #[test]
    fn keeps_java_doubles_exactly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nan.bfn");
        let values = vec![f64::NAN, -0.0, f64::INFINITY, 0.1 + 0.2];
        write_save(&path, &meta(), &values).unwrap();
        let (_, back): (SaveMeta, Vec<f64>) = read_save(&path).unwrap();
        let bits = |v: &[f64]| v.iter().map(|d| d.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&back), bits(&values));
    }

    #[test]
    fn keeps_backups_of_previous_saves() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.bfn");
        for cash in 0..5 {
            write_save(&path, &meta(), &Payload { clubs: vec![], cash }).unwrap();
        }
        let cash_in = |p: PathBuf| read_save::<Payload>(&p).unwrap().1.cash;
        assert_eq!(cash_in(path.clone()), 4);
        assert_eq!(cash_in(sibling(&path, "bak1")), 3);
        assert_eq!(cash_in(sibling(&path, "bak3")), 1);
        assert!(!sibling(&path, "bak4").exists());
    }

    #[test]
    fn rejects_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.bfn");
        fs::write(&path, b"not a save").unwrap();
        assert!(matches!(read_meta(&path), Err(SaveError::NotASave)));
    }
}
