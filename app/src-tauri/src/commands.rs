//! What the app asks of the library, as plain functions with typed input and
//! output (row A-07). The Tauri commands in `shell.rs` only call these.
//!
//! Each function calls the same library function as the matching command of
//! `adapdf` (see `src/main.rs` of the library), so the app and the command line
//! give the same findings, the same fix results and the same result of an export:
//!
//! | App | Command line | Library |
//! |---|---|---|
//! | [`check`] | `adapdf check` | `adapdf::check_file` |
//! | [`working_info`] | the choice of `--continue` and `--start-over` | `adapdf::fix::accessible_copy_path` |
//! | [`fix_offers`] | the questions of `adapdf fix` | `adapdf::fix::preflight` and `adapdf::fix::offers` |
//! | [`apply_fixes`] | the answers of `adapdf fix` | `adapdf::fix::fix_file` |
//! | [`create_copy`] | (none: the command line makes the copy with the first fix) | `adapdf::fix::make_copy` |
//! | [`export`] | `adapdf export` | `adapdf::pipeline::export_docx` |
//!
//! A failure of the tool (the command line exits with 2) is an `Err` with a plain
//! message. Problems found in a document are never an `Err`.

use std::path::{Path, PathBuf};

use adapdf::findings::{Finding, FixId, RuleId};
use adapdf::fix::{self, Environment, ExistingCopy, FixInput, FixOptions, FixRequest};
use adapdf::fonts::FontLookup;
use adapdf::render;
use adapdf::report::{Report, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};

/// A failure of the tool, in words for the person who uses the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandError {
    pub message: String,
}

impl CommandError {
    pub fn new(message: impl Into<String>) -> CommandError {
        CommandError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

pub type CommandResult<T> = Result<T, CommandError>;

/// The name of a file, for a message.
fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn failure(path: &Path, message: impl std::fmt::Display) -> CommandError {
    CommandError::new(format!("{}: {message}", name_of(path)))
}

// ---------------------------------------------------------------- check

/// The result of `check`: the findings and the counts. `exit_code` is what
/// `adapdf check` exits with: 0 for no errors (warnings are allowed), 1 for errors.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckResult {
    pub file: String,
    pub errors: usize,
    pub warnings: usize,
    pub exit_code: u8,
    pub findings: Vec<Finding>,
}

impl CheckResult {
    fn new(path: &Path, findings: Vec<Finding>) -> CheckResult {
        let (errors, warnings) = render::counts(&findings);
        CheckResult {
            file: path.display().to_string(),
            errors,
            warnings,
            exit_code: u8::from(render::has_errors(&findings)),
            findings,
        }
    }
}

pub fn check(path: &Path, fonts: &dyn FontLookup) -> CommandResult<CheckResult> {
    let findings = adapdf::check_file(path, fonts).map_err(|e| failure(path, e))?;
    Ok(CheckResult::new(path, findings))
}

// ----------------------------------------------------------- fix offers

/// Whether a fix can be chosen now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Availability {
    /// The person can choose it.
    Ready,
    /// Word does it when the PDF is exported. There is nothing to choose.
    AtExport,
    /// The tool will not do it now. The reason says why and what to do instead.
    Unavailable { reason: String },
}

/// A finding that has a fix. `ask` is true when the person must type text or
/// answer a question (an ask fix). The tool never fills that in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OfferView {
    pub finding: Finding,
    /// One-click texts from the document itself, for the title (DOC-001): the
    /// file name, the Title paragraph and the first Heading 1. Nothing is filled
    /// in until the person chooses one. Empty for every other fix (alt text has
    /// none: a caption next to the picture would make IMG-005 report it).
    pub suggestions: Vec<fix::TitleSuggestion>,
    /// The place of the finding among the findings of its rule. With the rule ID
    /// it names the finding in `apply_fixes`.
    pub position: usize,
    pub fix: FixId,
    pub ask: bool,
    /// What the fix does, in a sentence.
    pub description: String,
    pub availability: Availability,
}

pub fn fix_offers(
    path: &Path,
    env: &Environment,
    options: &FixOptions,
) -> CommandResult<Vec<OfferView>> {
    // Like `adapdf fix`: stop at once when Word has the accessible copy open.
    fix::preflight(env.fs, path).map_err(|e| CommandError::new(e.to_string()))?;
    let bytes = read_bytes(path, env)?;
    let offers = fix::offers(&bytes, options, env.fonts).map_err(|e| failure(path, e))?;
    let suggestions = adapdf::model::Document::from_bytes(&bytes)
        .map(|doc| fix::title_suggestions(&doc, &name_of(path)))
        .unwrap_or_default();
    Ok(offers
        .into_iter()
        .map(|offer| OfferView {
            description: fix::describe(offer.fix, options),
            availability: match offer.availability {
                fix::Availability::Ready => Availability::Ready,
                fix::Availability::AtExport => Availability::AtExport,
                fix::Availability::Unavailable(reason) => Availability::Unavailable { reason },
            },
            suggestions: if offer.fix == FixId::SetTitle {
                suggestions.clone()
            } else {
                Vec::new()
            },
            finding: offer.finding,
            position: offer.position,
            fix: offer.fix,
            ask: offer.ask,
        })
        .collect())
}

fn read_bytes(path: &Path, env: &Environment) -> CommandResult<Vec<u8>> {
    env.fs
        .read(path)
        .map_err(|e| failure(path, format!("the file cannot be read: {e}")))
}

fn read_offers(
    path: &Path,
    env: &Environment,
    options: &FixOptions,
) -> CommandResult<Vec<fix::Offer>> {
    let bytes = read_bytes(path, env)?;
    fix::offers(&bytes, options, env.fonts).map_err(|e| failure(path, e))
}

// ------------------------------------------------------- the working files

/// The files of one guide: the original, the accessible copy, and the folder.
/// Every screen of the app shows them, and the page asks which one to use when
/// the copy already exists (`copy_exists` and not `is_copy`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkingInfo {
    /// The file that the person chose.
    pub chosen: String,
    /// The Word document that Adapdf never changes. For a copy that was moved out
    /// of its folder, the file of that name next to it (it may not exist).
    pub original: String,
    /// The accessible copy: the file the fixes go into and Word edits.
    pub copy: String,
    /// The folder of the guide, `<name> - Adapdf`, where the copy, the PDF and the
    /// reports are. It is not there until the first copy or export.
    pub folder: String,
    /// The folder of the chosen file, which is what "Open folder" opens.
    pub chosen_folder: String,
    /// The chosen file is the accessible copy.
    pub is_copy: bool,
    pub copy_exists: bool,
}

fn parent_of(path: &Path) -> String {
    path.parent()
        .map_or_else(String::new, |p| p.display().to_string())
}

/// What the page needs to know about a file that was chosen.
pub fn working_info(path: &Path, env: &Environment) -> WorkingInfo {
    let is_copy = fix::is_accessible_copy(path);
    let copy = fix::accessible_copy_path(path);
    WorkingInfo {
        chosen: path.display().to_string(),
        original: fix::original_path(path).display().to_string(),
        folder: parent_of(&copy),
        chosen_folder: parent_of(path),
        copy_exists: env.fs.exists(&copy),
        copy: copy.display().to_string(),
        is_copy,
    }
}

/// Makes the accessible copy of an original with no fix, so that the person can
/// open it in Word. `start_over` replaces a copy that exists. Returns the files
/// of the guide, with the copy as the file to work on.
pub fn create_copy(path: &Path, start_over: bool, env: &Environment) -> CommandResult<WorkingInfo> {
    let existing = if start_over {
        ExistingCopy::Replace
    } else {
        ExistingCopy::Refuse
    };
    let copy = fix::make_copy(path, env, existing).map_err(|e| failure(path, e))?;
    Ok(working_info(&copy, env))
}

/// Checks that a path may be opened in Word: the accessible copy of a guide, as
/// a .docx file with an absolute path that is not a network path
/// (`\\server\share`), that exists. The page cannot ask the app to open any other
/// file, and the original is never opened by the app: the person edits the copy.
pub fn check_open_in_word(path: &Path) -> CommandResult<PathBuf> {
    let name = name_of(path);
    if !fix::is_accessible_copy(path) {
        return Err(CommandError::new(format!(
            "{name} is not an accessible copy. The app opens only the copy that it made (a file whose name ends with {}.docx), so that your original is not changed.",
            fix::COPY_SUFFIX
        )));
    }
    if !is_docx_name(&name) {
        return Err(CommandError::new(format!(
            "{name} is not a Word document (.docx)."
        )));
    }
    check_local_file(path, &name, "Make the copy again.")
}

/// Checks that the folder of a .docx file or of an accessibility report may be
/// opened: the file exists, with an absolute path that is not a network path. The
/// folder is the one that holds it.
pub fn check_open_folder(path: &Path) -> CommandResult<PathBuf> {
    let name = name_of(path);
    if !is_docx_name(&name) && !is_report_name(&name) {
        return Err(CommandError::new(format!(
            "{name} is not a Word document or a report. Only the folder of a .docx file or of an accessibility report is opened."
        )));
    }
    let file = check_local_file(path, &name, "Choose the document again.")?;
    file.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| CommandError::new(format!("{name} has no folder.")))
}

fn is_docx_name(name: &str) -> bool {
    name.to_lowercase().ends_with(".docx") && name.len() > ".docx".len()
}

fn is_report_name(name: &str) -> bool {
    [JSON_SUFFIX, HTML_SUFFIX]
        .iter()
        .any(|suffix| name.ends_with(suffix) && name.len() > suffix.len())
}

fn check_local_file(path: &Path, name: &str, advice: &str) -> CommandResult<PathBuf> {
    if !path.is_absolute() || path.to_string_lossy().starts_with(r"\\") {
        return Err(CommandError::new(format!(
            "{name} is not on a local drive, so the app does not open it."
        )));
    }
    if !path.is_file() {
        return Err(CommandError::new(format!("{name} is not there. {advice}")));
    }
    Ok(path.to_path_buf())
}

// -------------------------------------------------------- apply fixes

/// What a person gave for one fix.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Input {
    /// Yes. For an auto fix, and for the question about arrows and boxes.
    Confirm,
    /// The text that the person typed: a title or alt text.
    Text { text: String },
    /// "Mark as decorative", for alt text.
    Decorative,
}

/// One fix that the person chose. The finding is named by its rule and its
/// position, as in [`OfferView`]: the page cannot make up a finding.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Choice {
    pub rule_id: RuleId,
    pub position: usize,
    pub input: Input,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppliedView {
    pub rule_id: RuleId,
    /// What the tool did, in a sentence.
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotAppliedView {
    pub rule_id: RuleId,
    /// The message of the finding, so that the person knows which one it is.
    pub message: String,
    pub reason: String,
}

/// The result of `apply_fixes`. `exit_code` is what `adapdf fix` exits with when
/// it has done the same: 0 when no errors remain, 1 when errors remain.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApplyResult {
    /// The accessible copy that was written (its full path). `None` when nothing
    /// was written.
    pub copy: Option<String>,
    /// The file to work on from now: the copy when one was written, and the file
    /// that was given otherwise.
    pub working_file: String,
    /// The original, which is never changed (its full path).
    pub original: String,
    pub applied: Vec<AppliedView>,
    /// Findings that Word fixes when the PDF is exported.
    pub at_export: Vec<NotAppliedView>,
    pub refused: Vec<NotAppliedView>,
    /// The check of the file as it is now.
    pub findings: Vec<Finding>,
    pub errors: usize,
    pub warnings: usize,
    pub exit_code: u8,
}

/// `start_over` replaces an accessible copy that exists (the person chose "Start
/// over from the original"). Without it, a copy that exists is not replaced.
pub fn apply_fixes(
    path: &Path,
    choices: &[Choice],
    start_over: bool,
    env: &Environment,
    options: &FixOptions,
) -> CommandResult<ApplyResult> {
    let offers = read_offers(path, env, options)?;
    let mut requests: Vec<FixRequest> = Vec::new();
    let mut refused: Vec<NotAppliedView> = Vec::new();
    for choice in choices {
        let offer = offers
            .iter()
            .find(|o| o.finding.rule_id == choice.rule_id && o.position == choice.position);
        match offer {
            Some(offer) => requests.push(offer.request(match &choice.input {
                Input::Confirm => FixInput::Confirm,
                Input::Text { text } => FixInput::Text(text.clone()),
                Input::Decorative => FixInput::Decorative,
            })),
            None => refused.push(NotAppliedView {
                rule_id: choice.rule_id,
                message: String::new(),
                reason: "The document has changed since the check. Check it again.".to_string(),
            }),
        }
    }
    let existing = if start_over {
        ExistingCopy::Replace
    } else {
        ExistingCopy::Refuse
    };
    let outcome = fix::fix_file(path, &requests, options, env, existing)
        .map_err(|e| CommandError::new(e.to_string()))?;
    refused.extend(outcome.refused.iter().map(|r| NotAppliedView {
        rule_id: r.finding.rule_id,
        message: r.finding.message.clone(),
        reason: r.reason.clone(),
    }));
    let (errors, warnings) = render::counts(&outcome.findings);
    Ok(ApplyResult {
        copy: outcome.copy.as_ref().map(|c| c.display().to_string()),
        working_file: outcome
            .copy
            .as_deref()
            .unwrap_or(path)
            .display()
            .to_string(),
        original: fix::original_path(path).display().to_string(),
        applied: outcome
            .applied
            .iter()
            .map(|a| AppliedView {
                rule_id: a.finding.rule_id,
                summary: a.summary.clone(),
            })
            .collect(),
        at_export: outcome
            .at_export
            .iter()
            .map(|f| NotAppliedView {
                rule_id: f.rule_id,
                message: f.message.clone(),
                reason: "Word does this when the PDF is exported.".to_string(),
            })
            .collect(),
        refused,
        exit_code: u8::from(render::has_errors(&outcome.findings)),
        findings: outcome.findings,
        errors,
        warnings,
    })
}

// -------------------------------------------------------------- reports

/// A report on disk and where its files are.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportView {
    pub report: Report,
    /// The full path of the JSON file (the evidence that the sign-off changes).
    pub report_json: String,
    pub report_html: String,
    /// The full path of the PDF, when the report is about one.
    pub pdf: Option<String>,
}

impl ReportView {
    pub fn new(report: Report, report_json: &Path) -> CommandResult<ReportView> {
        let (json, html) = report_paths(report_json)?;
        let pdf = report
            .summary
            .pdf
            .as_ref()
            .map(|name| json.with_file_name(name).display().to_string());
        Ok(ReportView {
            report,
            report_json: json.display().to_string(),
            report_html: html.display().to_string(),
            pdf,
        })
    }
}

const JSON_SUFFIX: &str = ".accessibility.json";
const HTML_SUFFIX: &str = ".accessibility.html";

/// The JSON and the HTML of a report, from the path of the JSON. Only a file
/// that `adapdf export` could have written has a path that is accepted.
pub fn report_paths(report_json: &Path) -> CommandResult<(PathBuf, PathBuf)> {
    let name = name_of(report_json);
    match name.strip_suffix(JSON_SUFFIX) {
        Some(stem) if !stem.is_empty() => Ok((
            report_json.to_path_buf(),
            report_json.with_file_name(format!("{stem}{HTML_SUFFIX}")),
        )),
        _ => Err(CommandError::new(format!(
            "{name} is not an accessibility report. Choose a file whose name ends with {JSON_SUFFIX}."
        ))),
    }
}

/// Reads a report that `adapdf export` wrote (and the app may have signed).
pub fn read_report(report_json: &Path) -> CommandResult<ReportView> {
    report_paths(report_json)?;
    let text = std::fs::read_to_string(report_json)
        .map_err(|e| failure(report_json, format!("the report cannot be read: {e}")))?;
    // The layout version first: a report of another version may lack fields that
    // this version needs, and the person must get this message, not "missing field".
    #[derive(Deserialize)]
    struct Layout {
        schema_version: u32,
    }
    let not_ours = |e: serde_json::Error| {
        failure(
            report_json,
            format!("this is not a report that this tool wrote ({e})"),
        )
    };
    let layout: Layout = serde_json::from_str(&text).map_err(not_ours)?;
    if layout.schema_version < SCHEMA_VERSION {
        return Err(failure(
            report_json,
            format!(
                "This report was made by an older version of Adapdf (layout {}). Export the document again to make a new report.",
                layout.schema_version
            ),
        ));
    }
    if layout.schema_version > SCHEMA_VERSION {
        return Err(failure(
            report_json,
            format!(
                "This report was made by a newer version of Adapdf (layout {}). Update Adapdf to open it.",
                layout.schema_version
            ),
        ));
    }
    let report: Report = serde_json::from_str(&text).map_err(not_ours)?;
    ReportView::new(report, report_json)
}

/// Checks that a path may be opened as a report in the browser: the HTML file of
/// a report, with an absolute path that is not a network path (`\\server\share`),
/// that exists. The page cannot ask the app to open any other file or address.
///
/// A network drive that is mapped to a letter (`Z:`) is not told from a local
/// drive: that needs `GetDriveTypeW`, and so a new dependency (`windows`) for the
/// app. The file is one that the app wrote next to the person's document, or one
/// that the person chose, so this is a limit and not a way in (see the known gaps
/// of M5a in the test plan).
pub fn check_report_html(path: &Path) -> CommandResult<PathBuf> {
    let name = name_of(path);
    if !name.ends_with(HTML_SUFFIX) || name.len() == HTML_SUFFIX.len() {
        return Err(CommandError::new(format!(
            "{name} is not an accessibility report. Only a file whose name ends with {HTML_SUFFIX} can be opened."
        )));
    }
    if !path.is_absolute() || path.to_string_lossy().starts_with(r"\\") {
        return Err(CommandError::new(format!(
            "{name} is not on a local drive, so the app does not open it."
        )));
    }
    if !path.is_file() {
        return Err(CommandError::new(format!(
            "{name} is not there. Run the export again."
        )));
    }
    Ok(path.to_path_buf())
}

// --------------------------------------------------------------- export

/// The result of `export` when the tool did its work. `exit_code` is what
/// `adapdf export` exits with: 0 when the report says Pass, 1 when it says Fail.
/// The report is in `view`. A failure of the tool (exit 2) is an `Err`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExportResult {
    pub exit_code: u8,
    pub view: ReportView,
    /// The text that `adapdf export` prints.
    pub console: String,
    /// The files that this run wrote.
    pub files: Vec<String>,
}

#[cfg(windows)]
pub fn export(
    path: &Path,
    stages: &dyn adapdf::pipeline::Stages,
    fonts: &dyn FontLookup,
    now: fix::Timestamp,
) -> CommandResult<ExportResult> {
    use adapdf::pipeline::{Exit, export_docx, output_paths};

    let run = export_docx(path, stages, fonts, now);
    if run.exit == Exit::ToolFailure {
        // The message starts with "adapdf: <path>: ", which is for a console.
        let message = run.stderr.trim();
        let prefix = format!("adapdf: {}: ", path.display());
        return Err(CommandError::new(
            message.strip_prefix(&prefix).unwrap_or(message),
        ));
    }
    let report = run.report.ok_or_else(|| {
        failure(
            path,
            "the export gave no report. This is a fault in the tool",
        )
    })?;
    let (_, _, json) = output_paths(path);
    Ok(ExportResult {
        exit_code: run.exit.code(),
        view: ReportView::new(report, &json)?,
        console: run.stdout,
        files: run.files.iter().map(|f| f.display().to_string()).collect(),
    })
}

/// The export with the real Word and the real veraPDF, as `adapdf export` runs it.
#[cfg(windows)]
pub fn export_with_word(path: &Path, fonts: &dyn FontLookup) -> CommandResult<ExportResult> {
    let stages = adapdf::pipeline::RealStages {
        fonts,
        tool: adapdf::verapdf::Tool::locate(),
        options: adapdf::export::ExportOptions::default(),
    };
    export(path, &stages, fonts, fix::Timestamp::now())
}

#[cfg(not(windows))]
pub fn export_with_word(path: &Path, _fonts: &dyn FontLookup) -> CommandResult<ExportResult> {
    Err(failure(
        path,
        "the export needs Microsoft Word, which runs on Windows only.",
    ))
}
