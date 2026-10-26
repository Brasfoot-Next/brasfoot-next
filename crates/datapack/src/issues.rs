//! What the reader and the validator find wrong in a pack. Structured: the app turns an issue
//! into text with the catalog (`datapack-issue-<code>`), the command line tool with `Display`.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The game reads the pack anyway, fixing the value the way the original does.
    Warning,
    /// The pack, or the file, cannot be used.
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    /// The file, relative to the pack's root.
    pub path: String,
    /// The line of a CSV file (the header is line 1).
    pub line: Option<u64>,
    pub kind: IssueKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IssueKind {
    Unreadable { reason: String },
    Syntax { reason: String },
    MissingManifest,
    UnsupportedFormat { found: u32 },
    PatchWithoutBase,
    RemovalsInBase,
    /// A file that is not part of the format.
    UnknownFile,
    /// A club folder name that is not lowercase letters, digits and hyphens.
    BadFolderName { name: String },
    /// A club folder without `club.toml` (a patch may leave it out to change only other files
    /// of a club the layers below have).
    MissingClubFile,
    MissingSquad,
    /// A patch changes files of a club no layer below has.
    UnknownClub,
    CountryFolderMismatch { folder: String, country: String },
    FileNameMismatch { expected: String },
    UnknownCountry { code: String },
    UnknownState { code: String },
    StateOutsideBrazil,
    MissingState,
    EmptyName,
    BadColor { value: String },
    OutOfRange { field: &'static str, value: i64, min: i64, max: i64 },
    /// A potential off the game's 0 to 10 scale, which the game takes as it is.
    PotentialOffScale { value: i32 },
    UnknownPosition { code: String },
    UnknownSide { code: String },
    UnknownTrait { code: String },
    UnknownRival { id: String },
    MissingCrest,
    BadDivision { division: i32 },
    DuplicateDivision { division: i32 },
    UnknownStateFormat { code: String },
    UnknownRemoval { id: String },
}

impl IssueKind {
    pub fn severity(&self) -> Severity {
        use IssueKind::*;
        match self {
            UnknownFile | MissingState | OutOfRange { .. } | PotentialOffScale { .. } | UnknownRival { .. } | MissingCrest | UnknownRemoval { .. } => Severity::Warning,
            _ => Severity::Error,
        }
    }

    /// The catalog key's suffix.
    pub fn code(&self) -> &'static str {
        use IssueKind::*;
        match self {
            Unreadable { .. } => "unreadable",
            Syntax { .. } => "syntax",
            MissingManifest => "missing-manifest",
            UnsupportedFormat { .. } => "unsupported-format",
            PatchWithoutBase => "patch-without-base",
            RemovalsInBase => "removals-in-base",
            UnknownFile => "unknown-file",
            BadFolderName { .. } => "bad-folder-name",
            MissingClubFile => "missing-club-file",
            MissingSquad => "missing-squad",
            UnknownClub => "unknown-club",
            CountryFolderMismatch { .. } => "country-folder-mismatch",
            FileNameMismatch { .. } => "file-name-mismatch",
            UnknownCountry { .. } => "unknown-country",
            UnknownState { .. } => "unknown-state",
            StateOutsideBrazil => "state-outside-brazil",
            MissingState => "missing-state",
            EmptyName => "empty-name",
            BadColor { .. } => "bad-color",
            OutOfRange { .. } => "out-of-range",
            PotentialOffScale { .. } => "potential-off-scale",
            UnknownPosition { .. } => "unknown-position",
            UnknownSide { .. } => "unknown-side",
            UnknownTrait { .. } => "unknown-trait",
            UnknownRival { .. } => "unknown-rival",
            MissingCrest => "missing-crest",
            BadDivision { .. } => "bad-division",
            DuplicateDivision { .. } => "duplicate-division",
            UnknownStateFormat { .. } => "unknown-state-format",
            UnknownRemoval { .. } => "unknown-removal",
        }
    }
}

impl fmt::Display for IssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IssueKind::*;
        match self {
            Unreadable { reason } => write!(f, "cannot read the file: {reason}"),
            Syntax { reason } => write!(f, "{reason}"),
            MissingManifest => write!(f, "no manifest.toml at the root"),
            UnsupportedFormat { found } => write!(f, "format {found}, this version reads {}", crate::format::FORMAT_VERSION),
            PatchWithoutBase => write!(f, "a patch needs a base below it"),
            RemovalsInBase => write!(f, "only a patch removes things"),
            UnknownFile => write!(f, "not part of the format, ignored"),
            BadFolderName { name } => write!(f, "folder name {name:?} must be lowercase letters, digits and hyphens"),
            MissingClubFile => write!(f, "club folder without club.toml"),
            MissingSquad => write!(f, "club without squad.csv"),
            UnknownClub => write!(f, "changes a club no layer below has"),
            CountryFolderMismatch { folder, country } => write!(f, "club of {country} in the folder of {folder}"),
            FileNameMismatch { expected } => write!(f, "the file should be named {expected}"),
            UnknownCountry { code } => write!(f, "unknown country {code:?} (FIFA trigram expected)"),
            UnknownState { code } => write!(f, "unknown state {code:?}"),
            StateOutsideBrazil => write!(f, "only Brazilian clubs have a state"),
            MissingState => write!(f, "Brazilian club without a state, it plays no state league"),
            EmptyName => write!(f, "empty name"),
            BadColor { value } => write!(f, "color {value:?} is not #rrggbb"),
            OutOfRange { field, value, min, max } => write!(f, "{field} {value} outside {min}..{max}, the game replaces it"),
            PotentialOffScale { value } => write!(f, "potential {value} off the 0..10 scale, the game uses it as it is"),
            UnknownPosition { code } => write!(f, "unknown position {code:?}"),
            UnknownSide { code } => write!(f, "unknown side {code:?}"),
            UnknownTrait { code } => write!(f, "unknown trait {code:?}"),
            UnknownRival { id } => write!(f, "rival {id} is not in the database"),
            MissingCrest => write!(f, "club without crest.png"),
            BadDivision { division } => write!(f, "division {division} outside 1..4"),
            DuplicateDivision { division } => write!(f, "division {division} configured twice"),
            UnknownStateFormat { code } => write!(f, "unknown state league format {code:?}"),
            UnknownRemoval { id } => write!(f, "removes {id}, which no layer below has"),
        }
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.kind.severity() {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        match self.line {
            Some(line) => write!(f, "{level}: {}:{line}: {}", self.path, self.kind),
            None => write!(f, "{level}: {}: {}", self.path, self.kind),
        }
    }
}

pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.kind.severity() == Severity::Error)
}
