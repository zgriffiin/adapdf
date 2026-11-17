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
//! | [`fix_offers`] | the questions of `adapdf fix` | `adapdf::fix::preflight` and `adapdf::fix::offers` |
//! | [`apply_fixes`] | the answers of `adapdf fix` | `adapdf::fix::fix_file` |
//! | [`export`] | `adapdf export` | `adapdf::pipeline::export_docx` |
//!
//! A failure of the tool (the command line exits with 2) is an `Err` with a plain
//! message. Problems found in a document are never an `Err`.

use std::path::{Path, PathBuf};

use adapdf::findings::{Finding, FixId, RuleId};
use adapdf::fix::{self, Environment, FixInput, FixOptions, FixRequest};
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
    // Like `adapdf fix`: stop at once when Word has the file open.
    fix::preflight(env.fs, path).map_err(|e| CommandError::new(e.to_string()))?;
    let offers = read_offers(path, env, options)?;
    Ok(offers
        .into_iter()
        .map(|offer| OfferView {
            description: fix::describe(offer.fix, options),
            availability: match offer.availability {
                fix::Availability::Ready => Availability::Ready,
                fix::Availability::AtExport => Availability::AtExport,
                fix::Availability::Unavailable(reason) => Availability::Unavailable { reason },
            },
            finding: offer.finding,
            position: offer.position,
            fix: offer.fix,
            ask: offer.ask,
        })
        .collect())
}

fn read_offers(
    path: &Path,
    env: &Environment,
    options: &FixOptions,
) -> CommandResult<Vec<fix::Offer>> {
    let bytes = env
        .fs
        .read(path)
        .map_err(|e| failure(path, format!("the file cannot be read: {e}")))?;
    fix::offers(&bytes, options, env.fonts).map_err(|e| failure(path, e))
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
    /// The name of the backup file. `None` when nothing was written.
    pub backup: Option<String>,
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

pub fn apply_fixes(
    path: &Path,
    choices: &[Choice],
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
    let outcome = fix::fix_file(path, &requests, options, env)
        .map_err(|e| CommandError::new(e.to_string()))?;
    refused.extend(outcome.refused.iter().map(|r| NotAppliedView {
        rule_id: r.finding.rule_id,
        message: r.finding.message.clone(),
        reason: r.reason.clone(),
    }));
    let (errors, warnings) = render::counts(&outcome.findings);
    Ok(ApplyResult {
        backup: outcome.backup.as_deref().map(name_of),
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
    let report: Report = serde_json::from_str(&text).map_err(|e| {
        failure(
            report_json,
            format!("this is not a report that this tool wrote ({e})"),
        )
    })?;
    if report.schema_version != SCHEMA_VERSION {
        return Err(failure(
            report_json,
            format!(
                "this report has layout version {}, and this tool reads version {SCHEMA_VERSION}",
                report.schema_version
            ),
        ));
    }
    ReportView::new(report, report_json)
}

/// Checks that a path may be opened as a report in the browser: the HTML file of
/// a report, on a local drive, that exists. The page cannot ask the app to open
/// any other file or address.
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
