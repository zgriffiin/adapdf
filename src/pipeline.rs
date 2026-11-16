//! `adapdf export`: stages 1, 3, 4, 5 and 6 of the pipeline (spec 02), in order.
//!
//! 1. Stage 1 runs on the .docx. With any error, no PDF is made. The report is
//!    still written, says Fail and lists the errors (decided point 7). The exit
//!    code is 1.
//! 2. The tools are checked (veraPDF and its Java), so that a missing tool is
//!    found before Word starts.
//! 3. Stage 3: Word exports the PDF ([`crate::export::export_pdf`]).
//! 4. Stage 4: [`crate::pdfmeta::fix_file`] adds the document metadata that is
//!    missing, with the title and language of the .docx.
//! 5. Stage 5: veraPDF checks the final PDF.
//! 6. Stage 6: the report is written as HTML and JSON next to the .docx.
//!
//! # Exit codes
//!
//! | Exit | When |
//! |---|---|
//! | 0 | No error anywhere: the verdict is Pass. |
//! | 1 | A stage 1 error (no PDF, a Fail report), a PDF without tags (the PDF and a Fail report are written; decided point 8), a stage 4 problem that is an error, or any PDF/UA-1 failure of veraPDF. |
//! | 2 | The tool could not do its work: the .docx cannot be read, veraPDF or Java is missing, Word is missing or fails, the export times out, `Guide.pdf` is locked, the PDF cannot be changed, or the report cannot be written. A message is on stderr and nothing is on stdout. No report is written (a report is the evidence of a check that ran). |
//!
//! The stages are behind the trait [`Stages`], so that a test can replace Word
//! and veraPDF with fakes.

use std::fs;
use std::path::{Path, PathBuf};

use crate::export::{ExportError, ExportOptions, ExportResult, export_pdf};
use crate::findings::Severity;
use crate::fix::Timestamp;
use crate::fonts::FontLookup;
use crate::model::Document;
use crate::pdfmeta::{self, SourceMetadata};
use crate::report::{self, ExportInfo, Input, Parameter, Report, Verdict, html};
use crate::rules;
use crate::verapdf::{self, Tool, Validation, VeraPdfError};

/// The three stages that need something outside the tool.
pub trait Stages {
    /// Checks that veraPDF and its Java are there, before Word starts.
    fn check_tools(&self) -> Result<(), VeraPdfError>;
    /// Stage 3. Writes the PDF at `pdf`.
    fn export(&self, docx: &Path, pdf: &Path) -> Result<ExportResult, ExportError>;
    /// Stage 5.
    fn validate(&self, pdf: &Path) -> Result<Validation, VeraPdfError>;
}

/// The real Word and the real veraPDF.
pub struct RealStages<'a> {
    pub fonts: &'a dyn FontLookup,
    pub tool: Tool,
    pub options: ExportOptions,
}

impl Stages for RealStages<'_> {
    fn check_tools(&self) -> Result<(), VeraPdfError> {
        self.tool.check()
    }

    fn export(&self, docx: &Path, pdf: &Path) -> Result<ExportResult, ExportError> {
        export_pdf(docx, pdf, self.fonts, &self.options)
    }

    fn validate(&self, pdf: &Path) -> Result<Validation, VeraPdfError> {
        verapdf::validate(&self.tool, pdf)
    }
}

/// How a run ended. See the module documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Pass,
    ErrorsFound,
    ToolFailure,
}

impl Exit {
    /// The exit code of the command: 0, 1 or 2.
    pub fn code(self) -> u8 {
        match self {
            Exit::Pass => 0,
            Exit::ErrorsFound => 1,
            Exit::ToolFailure => 2,
        }
    }
}

/// The result of a run: what to print and what was written.
#[derive(Debug, Clone)]
pub struct Run {
    pub exit: Exit,
    /// The report. `None` for a tool failure.
    pub report: Option<Report>,
    /// The files that this run wrote: the PDF, the HTML and the JSON.
    pub files: Vec<PathBuf>,
    /// For standard output. Empty for a tool failure.
    pub stdout: String,
    /// For standard error. Only a tool failure has a message.
    pub stderr: String,
}

impl Run {
    fn failure(docx: &Path, message: &str) -> Run {
        Run {
            exit: Exit::ToolFailure,
            report: None,
            files: Vec::new(),
            stdout: String::new(),
            stderr: format!("adapdf: {}: {message}\n", docx.display()),
        }
    }
}

/// The path of the PDF, the HTML report and the JSON report for a .docx.
pub fn output_paths(docx: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        docx.with_extension("pdf"),
        docx.with_extension("accessibility.html"),
        docx.with_extension("accessibility.json"),
    )
}

/// Runs the export of `docx`. `now` is the local time that the report shows.
pub fn export_docx(
    docx: &Path,
    stages: &dyn Stages,
    fonts: &dyn FontLookup,
    now: Timestamp,
) -> Run {
    let document = match Document::open(docx) {
        Ok(document) => document,
        Err(error) => return Run::failure(docx, &error.to_string()),
    };
    let findings = rules::check(&document, fonts);
    if findings.iter().any(|f| f.severity == Severity::Error) {
        return blocked(docx, findings, now);
    }

    if let Err(error) = stages.check_tools() {
        return Run::failure(docx, &error.to_string());
    }
    let (pdf, _, _) = output_paths(docx);
    let exported = match stages.export(docx, &pdf) {
        Ok(exported) => exported,
        // Stage 1 inside the export found an error that the check above did not.
        Err(ExportError::Blocked { findings }) => return blocked(docx, findings, now),
        Err(error) => return Run::failure(docx, &error.to_string()),
    };

    let source = SourceMetadata::from_document(&document);
    let metadata = match pdfmeta::fix_file(&pdf, &source) {
        Ok(metadata) => metadata,
        Err(error) => {
            return Run::failure(
                docx,
                &format!("The PDF metadata could not be checked or fixed: {error}."),
            );
        }
    };
    let validation = match stages.validate(&pdf) {
        Ok(validation) => validation,
        Err(error) => {
            return Run::failure(docx, &format!("The PDF could not be checked: {error}."));
        }
    };

    let report = report::build(Input {
        document: file_name(docx),
        generated_local: now.display(),
        findings: exported.findings.clone(),
        applied_fixes: Vec::new(),
        backup: newest_backup(docx),
        pdf: Some(file_name(&pdf)),
        export: Some(export_info(&exported)),
        metadata: Some(metadata),
        verapdf: Some(validation),
    });
    finish(docx, report, Some(pdf))
}

/// Stage 1 found errors: no PDF, a Fail report.
fn blocked(docx: &Path, findings: Vec<crate::findings::Finding>, now: Timestamp) -> Run {
    let report = report::build(Input {
        document: file_name(docx),
        generated_local: now.display(),
        findings,
        applied_fixes: Vec::new(),
        backup: newest_backup(docx),
        pdf: None,
        export: None,
        metadata: None,
        verapdf: None,
    });
    finish(docx, report, None)
}

/// Writes the two report files and works out the exit code.
fn finish(docx: &Path, report: Report, pdf: Option<PathBuf>) -> Run {
    let (_, html_path, json_path) = output_paths(docx);
    let written = fs::write(&html_path, html::render(&report))
        .map_err(|e| (&html_path, e))
        .and_then(|()| fs::write(&json_path, report.to_json()).map_err(|e| (&json_path, e)));
    if let Err((path, error)) = written {
        return Run::failure(
            docx,
            &format!(
                "The report \"{}\" cannot be written ({error}).",
                path.display()
            ),
        );
    }
    let mut files: Vec<PathBuf> = pdf.into_iter().collect();
    files.push(html_path);
    files.push(json_path);
    let paths: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let exit = match report.summary.verdict {
        Verdict::Pass => Exit::Pass,
        Verdict::Fail => Exit::ErrorsFound,
    };
    Run {
        exit,
        stdout: report.console_summary(&paths),
        report: Some(report),
        files,
        stderr: String::new(),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The newest `<name>.backup-*.docx` in the folder of the .docx, if there is one.
fn newest_backup(docx: &Path) -> Option<String> {
    let folder = match docx.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder,
        _ => Path::new("."),
    };
    let stem = docx.file_stem()?.to_string_lossy().into_owned();
    let names: Vec<String> = fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    report::latest_backup(&stem, names.iter().map(String::as_str))
}

fn export_info(result: &ExportResult) -> ExportInfo {
    let p = &result.parameters;
    let on_off = |value: bool| if value { "true" } else { "false" }.to_string();
    let parameter = |name: &str, value: String| Parameter {
        name: name.to_string(),
        value,
    };
    ExportInfo {
        word_build: result.word_build.clone(),
        parameters: vec![
            parameter(
                "ExportFormat",
                format!("{} (wdExportFormatPDF)", p.export_format),
            ),
            parameter("OpenAfterExport", on_off(p.open_after_export)),
            parameter(
                "OptimizeFor",
                format!("{} (wdExportOptimizeForPrint)", p.optimize_for),
            ),
            parameter("Item", format!("{} (wdExportDocumentContent)", p.item)),
            parameter("IncludeDocProps", on_off(p.include_doc_props)),
            parameter(
                "CreateBookmarks",
                format!("{} (wdExportCreateHeadingBookmarks)", p.create_bookmarks),
            ),
            parameter("DocStructureTags", on_off(p.doc_structure_tags)),
            parameter("BitmapMissingFonts", on_off(p.bitmap_missing_fonts)),
            parameter("UseISO19005_1", on_off(p.use_iso19005_1)),
        ],
        fields_updated: result.fields.fields,
        tables_of_contents_updated: result.fields.tables_of_contents,
        first_failed_field: result.fields.first_failed,
        seconds: result.duration.as_secs_f64(),
    }
}
