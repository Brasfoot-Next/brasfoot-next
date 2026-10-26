//! The Patches screen, which the original does not have: the data sources of a new game (the
//! official database and the patches over it), their downloads and updates, and what the
//! database they make up holds. See `docs/patches.md`.

use brasfoot_datapack::format::SourceKind;
use brasfoot_datapack::issues::{Issue, IssueKind, Severity};
use brasfoot_i18n::{Translator, args};
use eframe::egui::{self, RichText};

use crate::Navigation;
use crate::format;
use crate::game_files::GameFiles;
use crate::patches::download::{DownloadError, Phase};
use crate::patches::{Library, Location, Source, Status};
use crate::screens::menu;
use crate::theme;

const HELP: &str = "https://github.com/Brasfoot-Next/brasfoot-next/blob/main/docs/patches.md";
/// Issues listed at most.
const SHOWN_ISSUES: usize = 200;

#[derive(Default)]
pub struct PatchesScreen {
    github: String,
    #[cfg(not(target_arch = "wasm32"))]
    path: String,
    message: Option<String>,
    show_issues: bool,
    checked: bool,
}

/// An issue of the validator as text.
pub fn issue_text(t: &Translator, issue: &Issue) -> String {
    let key = format!("patches-issue-{}", issue.kind.code());
    let text = match &issue.kind {
        IssueKind::Unreadable { reason } | IssueKind::Syntax { reason } => t.text(&key, Some(&args([("reason", reason.clone())]))),
        IssueKind::UnsupportedFormat { found } => t.text(&key, Some(&args([("found", found.to_string())]))),
        IssueKind::BadFolderName { name } => t.text(&key, Some(&args([("name", name.clone())]))),
        IssueKind::CountryFolderMismatch { folder, country } => t.text(&key, Some(&args([("folder", folder.clone()), ("country", country.clone())]))),
        IssueKind::FileNameMismatch { expected } => t.text(&key, Some(&args([("expected", expected.clone())]))),
        IssueKind::UnknownCountry { code }
        | IssueKind::UnknownState { code }
        | IssueKind::UnknownPosition { code }
        | IssueKind::UnknownSide { code }
        | IssueKind::UnknownTrait { code }
        | IssueKind::UnknownStateFormat { code } => t.text(&key, Some(&args([("code", code.clone())]))),
        IssueKind::BadColor { value } => t.text(&key, Some(&args([("value", value.clone())]))),
        IssueKind::OutOfRange { field, value, min, max } => t.text(
            &key,
            Some(&args([("field", t.text(&format!("patches-field-{field}"), None)), ("value", value.to_string()), ("min", min.to_string()), ("max", max.to_string())])),
        ),
        IssueKind::PotentialOffScale { value } => t.text(&key, Some(&args([("value", value.to_string())]))),
        IssueKind::UnknownRival { id } | IssueKind::UnknownRemoval { id } => t.text(&key, Some(&args([("id", id.clone())]))),
        IssueKind::BadDivision { division } | IssueKind::DuplicateDivision { division } => t.text(&key, Some(&args([("division", division.to_string())]))),
        _ => t.text(&key, None),
    };
    match issue.line {
        Some(line) => format!("{}:{line}: {text}", issue.path),
        None => format!("{}: {text}", issue.path),
    }
}

fn error_text(t: &Translator, e: &DownloadError) -> String {
    match e {
        DownloadError::Network(reason) => t.text("patches-error-network", Some(&args([("reason", reason.clone())]))),
        DownloadError::NotFound => t.text("patches-error-not-found", None),
        DownloadError::NotAPack => t.text("patches-error-not-a-pack", None),
        DownloadError::UnsupportedFormat(f) => t.text("patches-error-unsupported-format", Some(&args([("format", f.to_string())]))),
        DownloadError::Corrupted(_) => t.text("patches-error-corrupted", None),
        DownloadError::Storage(reason) => t.text("patches-error-storage", Some(&args([("reason", reason.clone())]))),
        DownloadError::RateLimited => t.text("patches-error-rate-limited", None),
    }
}

fn megabytes(t: &Translator, bytes: u64) -> String {
    t.text("patches-megabytes", Some(&args([("value", format::decimal(t, bytes as f64 / 1_000_000.0, 1))])))
}

fn phase_text(t: &Translator, phase: &Phase) -> (String, Option<f32>) {
    match phase {
        Phase::Starting => (t.text("patches-phase-starting", None), None),
        Phase::Chunks { done, total } => {
            let text = t.text("patches-phase-chunks", Some(&args([("done", megabytes(t, *done)), ("total", megabytes(t, *total))])));
            (text, Some(if *total == 0 { 1.0 } else { *done as f32 / *total as f32 }))
        }
        Phase::Files { done, total } => {
            let text = t.text("patches-phase-files", Some(&args([("done", format::thousands(t, *done as i64)), ("total", format::thousands(t, *total as i64))])));
            (text, Some(if *total == 0 { 1.0 } else { *done as f32 / *total as f32 }))
        }
        Phase::Waiting { seconds } => (t.text("patches-phase-waiting", Some(&args([("seconds", seconds.to_string())]))), None),
        Phase::Saving => (t.text("patches-phase-saving", None), Some(1.0)),
    }
}

/// A commit as GitHub shows it.
fn short_version(version: &str) -> &str {
    if version.len() == 40 && version.bytes().all(|b| b.is_ascii_hexdigit()) { &version[..7] } else { version }
}

fn muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).font(theme::regular(12.0)).color(theme::MUTED));
}

fn tag(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::new().fill(color.gamma_multiply(0.18)).corner_radius(4).inner_margin(egui::Margin::symmetric(6, 1)).show(ui, |ui| {
        ui.label(RichText::new(text).font(theme::bold(10.0)).color(color));
    });
}

impl PatchesScreen {
    pub fn new() -> Self {
        PatchesScreen::default()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, t: &Translator, files: &GameFiles) -> Navigation {
        let library = &files.patches;
        if !self.checked && library.list().is_some() {
            library.check_updates();
            self.checked = true;
        }
        if library.busy() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
        let mut navigation = Navigation::Stay;
        menu::page(ui, 680.0, |ui| {
            if menu::header(ui, t, &t.text("patches-title", None), &t.text("patches-subtitle", None), true) {
                navigation = Navigation::Opening;
            }
            menu::section(ui, &t.text("patches-sources", None));
            muted(ui, t.text("patches-order-hint", None));
            ui.add_space(8.0);
            match library.list() {
                Some(list) => {
                    let count = list.sources.len();
                    for (i, source) in list.sources.iter().enumerate() {
                        self.source_card(ui, t, library, source, i, count);
                        ui.add_space(8.0);
                    }
                }
                None => {
                    ui.spinner();
                }
            }
            menu::section(ui, &t.text("patches-add", None));
            self.add_panel(ui, t, library);
            menu::section(ui, &t.text("patches-database", None));
            self.database_panel(ui, t, library);
            ui.add_space(12.0);
            ui.hyperlink_to(RichText::new(t.text("patches-help", None)).font(theme::regular(13.0)).color(theme::TEXT_BLUE), HELP);
            ui.add_space(24.0);
        });
        navigation
    }

    fn source_card(&mut self, ui: &mut egui::Ui, t: &Translator, library: &Library, source: &Source, index: usize, count: usize) {
        let location = &source.location;
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let mut enabled = source.enabled;
                if ui.checkbox(&mut enabled, "").on_hover_text(t.text("patches-enabled", None)).changed() {
                    library.set_enabled(location, enabled);
                }
                let name = source.installed.as_ref().map(|i| i.manifest.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| location.label());
                ui.label(RichText::new(name).font(theme::bold(15.0)));
                if location.is_official() {
                    tag(ui, &t.text("patches-official", None), theme::ACCENT);
                }
                if let Some(installed) = &source.installed {
                    let kind = if installed.manifest.kind == SourceKind::Base { "patches-kind-base" } else { "patches-kind-patch" };
                    tag(ui, &t.text(kind, None), theme::MUTED);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if menu::small_action(ui, &t.text("patches-remove", None), true).clicked() {
                        library.remove(location);
                    }
                    if menu::small_action(ui, &t.text("patches-move-down", None), index + 1 < count).clicked() {
                        library.shift(location, 1);
                    }
                    if menu::small_action(ui, &t.text("patches-move-up", None), index > 0).clicked() {
                        library.shift(location, -1);
                    }
                });
            });
            let mut detail = location.label();
            if let Some(version) = source.installed.as_ref().and_then(|i| i.version.as_deref()) {
                detail = format!("{detail} · {}", t.text("patches-version", Some(&args([("version", short_version(version).to_owned())]))));
            }
            muted(ui, detail);
            if let Some(description) = source.installed.as_ref().and_then(|i| i.manifest.description.clone()) {
                ui.label(RichText::new(description).font(theme::regular(13.0)));
            }
            ui.add_space(6.0);
            self.source_state(ui, t, library, source);
        });
    }

    /// The download running, its error, or whether the source is up to date.
    fn source_state(&mut self, ui: &mut egui::Ui, t: &Translator, library: &Library, source: &Source) {
        let location = &source.location;
        if matches!(location, Location::Folder { .. }) {
            muted(ui, t.text("patches-folder-live", None));
            return;
        }
        let job = library.job(location);
        let result = job.as_ref().and_then(|j| j.result.lock().ok().and_then(|r| r.clone()));
        if let Some(job) = &job
            && result.is_none()
        {
            let phase = job.progress.lock().map(|p| p.clone()).unwrap_or(Phase::Starting);
            let (text, fraction) = phase_text(t, &phase);
            match fraction {
                Some(f) => {
                    ui.add(egui::ProgressBar::new(f).text(text));
                }
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        muted(ui, text);
                    });
                }
            }
            return;
        }
        ui.horizontal(|ui| {
            if let Some(Err(e)) = &result {
                ui.label(RichText::new(error_text(t, e)).font(theme::regular(12.0)).color(theme::BAD));
                if menu::small_action(ui, &t.text("patches-retry", None), true).clicked() {
                    library.install(location);
                }
                return;
            }
            if source.installed.is_none() {
                muted(ui, t.text("patches-not-downloaded", None));
                if menu::small_action(ui, &t.text("patches-download", None), true).clicked() {
                    library.install(location);
                }
                return;
            }
            match library.status(location) {
                Some(Status::Checking) => {
                    ui.spinner();
                    muted(ui, t.text("patches-checking", None));
                }
                Some(Status::UpdateAvailable) => {
                    ui.label(RichText::new(t.text("patches-update-available", None)).font(theme::bold(12.0)).color(theme::ACCENT));
                    if menu::small_action(ui, &t.text("patches-update", None), true).clicked() {
                        library.install(location);
                    }
                }
                Some(Status::Failed(e)) => {
                    muted(ui, error_text(t, &e));
                    if menu::small_action(ui, &t.text("patches-update", None), true).clicked() {
                        library.install(location);
                    }
                }
                Some(Status::UpToDate) => muted(ui, t.text("patches-up-to-date", None)),
                None => {
                    if menu::small_action(ui, &t.text("patches-update", None), true).clicked() {
                        library.install(location);
                    }
                }
            }
        });
    }

    fn add_panel(&mut self, ui: &mut egui::Ui, t: &Translator, library: &Library) {
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            menu::field_label(ui, &t.text("patches-add-github", None));
            ui.horizontal(|ui| {
                let edit = egui::TextEdit::singleline(&mut self.github).hint_text(t.text("patches-add-github-hint", None)).desired_width(ui.available_width() - 110.0);
                let response = ui.add(edit);
                let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if menu::primary_button(ui, &t.text("patches-add-button", None), !self.github.trim().is_empty()).clicked() || submitted {
                    let outcome = match Location::github(&self.github) {
                        Some(location) => (!library.add(location)).then(|| t.text("patches-add-duplicate", None)),
                        None => Some(t.text("patches-add-invalid", None)),
                    };
                    if outcome.is_none() {
                        self.github.clear();
                    }
                    self.message = outcome;
                }
            });
            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.add_space(10.0);
                menu::field_label(ui, &t.text("patches-add-path", None));
                ui.add(egui::TextEdit::singleline(&mut self.path).hint_text(t.text("patches-add-path-hint", None)).desired_width(ui.available_width()));
                ui.horizontal(|ui| {
                    let path = self.path.trim().to_owned();
                    let folder = std::path::Path::new(&path).is_dir();
                    let zip = std::path::Path::new(&path).is_file();
                    let mut add = |location: Location| {
                        self.message = if library.add(location) {
                            self.path.clear();
                            None
                        } else {
                            Some(t.text("patches-add-duplicate", None))
                        };
                    };
                    if menu::small_action(ui, &t.text("patches-add-folder-button", None), folder).clicked() {
                        add(Location::Folder { path: path.clone() });
                    }
                    if menu::small_action(ui, &t.text("patches-add-zip-button", None), zip).clicked() {
                        add(Location::Zip { path: path.clone() });
                    }
                    if !path.is_empty() && !folder && !zip {
                        muted(ui, t.text("patches-add-path-missing", None));
                    }
                });
            }
            if let Some(message) = &self.message {
                ui.add_space(6.0);
                ui.label(RichText::new(message).font(theme::regular(12.0)).color(theme::BAD));
            }
        });
    }

    fn database_panel(&mut self, ui: &mut egui::Ui, t: &Translator, library: &Library) {
        menu::panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !library.ready() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    muted(ui, t.text("patches-database-loading", None));
                });
                return;
            }
            let Some(loaded) = library.loaded() else {
                muted(ui, t.text("patches-database-empty", None));
                return;
            };
            let content = &loaded.database.content;
            let players: usize = content.clubs.values().map(|c| c.squad.as_ref().map_or(0, Vec::len) + c.youth.as_ref().map_or(0, Vec::len)).sum();
            let summary = t.text(
                "patches-database-summary",
                Some(&args([
                    ("clubs", format::thousands(t, content.clubs.len() as i64)),
                    ("players", format::thousands(t, players as i64)),
                    ("leagues", content.leagues.len().to_string()),
                    ("states", content.state_leagues.len().to_string()),
                ])),
            );
            ui.label(RichText::new(summary).font(theme::bold(14.0)));
            if !loaded.skipped.is_empty() {
                muted(ui, t.text("patches-skipped", Some(&args([("count", loaded.skipped.len())]))));
            }
            let errors = loaded.issues.iter().filter(|i| i.kind.severity() == Severity::Error).count();
            let warnings = loaded.issues.len() - errors;
            if loaded.issues.is_empty() {
                return;
            }
            ui.horizontal(|ui| {
                let color = if errors > 0 { theme::BAD } else { theme::MUTED };
                ui.label(RichText::new(t.text("patches-issues", Some(&args([("errors", errors), ("warnings", warnings)])))).font(theme::regular(12.0)).color(color));
                let toggle = if self.show_issues { "patches-hide-issues" } else { "patches-show-issues" };
                if menu::small_action(ui, &t.text(toggle, None), true).clicked() {
                    self.show_issues = !self.show_issues;
                }
            });
            if self.show_issues {
                let mut issues: Vec<&Issue> = loaded.issues.iter().collect();
                issues.sort_by(|a, b| b.kind.severity().cmp(&a.kind.severity()).then(a.path.cmp(&b.path)));
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for issue in issues.iter().take(SHOWN_ISSUES) {
                        let color = if issue.kind.severity() == Severity::Error { theme::BAD } else { theme::MUTED };
                        ui.label(RichText::new(issue_text(t, issue)).font(theme::regular(11.0)).color(color));
                    }
                    if issues.len() > SHOWN_ISSUES {
                        muted(ui, t.text("patches-issues-more", Some(&args([("count", issues.len() - SHOWN_ISSUES)]))));
                    }
                });
            }
        });
    }
}
