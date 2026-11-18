//! Stage 6 of the pipeline (spec 02): the report.
//!
//! [`build`] puts the results of the stages in one [`Report`]. The report is
//! written two ways: as HTML for people ([`html::render`], which is itself an
//! accessible document) and as JSON for later tooling (`serde_json` on the
//! [`Report`]). Both have the same content: the HTML is made from the same
//! value that the JSON is made from.
//!
//! The report is the evidence that a document passed, so the verdict is worked
//! out here, once, from the results:
//!
//! - **Fail** when there is any error: a stage 1 error, a stage 4 problem that
//!   is an error (an untagged PDF, a PDF/UA part that is not 1), or a failed
//!   PDF/UA-1 rule of veraPDF.
//! - **Fail** also when a stage did not run and nothing else failed (this does
//!   not happen with the `export` command; it is the safe reading).
//! - **Pass** otherwise. Warnings (stage 1 warnings and the WCAG profile of
//!   veraPDF) do not change the verdict.

pub mod html;
#[cfg(test)]
mod tests;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::findings::{Finding, Severity};
use crate::fix::FixLogEntry;
use crate::pdfmeta;
use crate::verapdf::{Validation, WcagOutcome};

/// The version of the JSON layout. It changes when a field is removed or its
/// meaning changes. Version 2 (Adapdf 0.2): the counts of the summary are split
/// (pre-check and veraPDF), `fixes` lists every fix recorded in the document
/// and names its original (no backup), a veraPDF failure has a `place` in
/// words, and the sections are in the order of the page.
pub const SCHEMA_VERSION: u32 = 2;

/// The checklist of spec 02, stage 6, word for word.
pub const CHECKLIST: [&str; 7] = [
    "Each piece of alt text describes what the image shows, for the guide's purpose.",
    "Images marked decorative really are decorative.",
    "The reading order makes sense when read aloud. Check with the PAC screen-reader preview.",
    "Colour is not the only way information is shown.",
    "Link text makes sense out of context.",
    "Headings describe their section.",
    "Tables are data tables, not layout.",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Fail,
}

impl Verdict {
    /// The word that the report shows: not only a colour.
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Pass => "Pass",
            Verdict::Fail => "Fail",
        }
    }
}

/// Section 1: pass or fail, the counts, the tool version and the date.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub verdict: Verdict,
    /// Every error of the report: the pre-check, the PDF metadata and tags, and
    /// veraPDF. The page shows the three parts below, not this number.
    pub errors: usize,
    pub warnings: usize,
    /// Errors found by the pre-check of the Word document.
    pub precheck_errors: usize,
    /// Warnings found by the pre-check of the Word document.
    pub precheck_warnings: usize,
    /// Rules of the PDF/UA-1 check (veraPDF) that failed. 0 when the check did
    /// not run.
    pub pdf_failed_rules: usize,
    /// Why the verdict is Fail, in plain words. `None` for a pass.
    pub reason: Option<String>,
    pub tool: String,
    pub tool_version: String,
    /// The local time of the run, `yyyy-mm-dd hh:mm:ss`.
    pub generated_local: String,
    /// The .docx file name.
    pub document: String,
    /// The PDF file name, when a PDF was written.
    pub pdf: Option<String>,
}

/// One export parameter, as it was passed to Word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub value: String,
}

/// How Word made the PDF (stage 3). Part of the summary section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportInfo {
    /// For example `16.0.20527.20000`.
    pub word_build: String,
    pub parameters: Vec<Parameter>,
    /// How many fields Word updated in the main text.
    pub fields_updated: u32,
    pub tables_of_contents_updated: u32,
    /// The number (from 1) of the first field that Word could not update.
    pub first_failed_field: Option<u32>,
    pub seconds: f64,
}

/// One fix that Adapdf applied to the document: the entry of the fix log that
/// the accessible copy holds (section 3).
pub type FixEntry = FixLogEntry;

/// Section 3: every fix in the fix log of the document, oldest first, not only
/// the fixes of one run. `original` is the name of the Word document that the
/// accessible copy was made from. It is `None` when the document is not an
/// accessible copy. Adapdf never changes the original.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixesSection {
    pub original: Option<String>,
    pub applied: Vec<FixEntry>,
}

/// Section 4: stage 4, or why it did not run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum MetadataSection {
    NotRun { reason: String },
    Done(pdfmeta::Report),
}

/// Section 5: stage 5, or why it did not run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum VeraPdfSection {
    NotRun {
        reason: String,
    },
    Ran {
        verapdf_version: Option<String>,
        validation: Validation,
    },
}

/// What a person records from PAC (section 5). It is empty when the report is
/// made, and the app fills it in with the sign-off.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacSection {
    /// The PAC report file, as a path relative to the folder of the report.
    pub report_file: Option<String>,
    pub pac_version: Option<String>,
    /// `pass` (0 errors) or `fail`.
    pub result: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

/// Section 6: the checklist and the sign-off, empty when the report is made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanChecklist {
    pub items: Vec<ChecklistItem>,
    pub name: Option<String>,
    pub date: Option<String>,
}

/// The whole report. The field order is the section order: the summary, the
/// pre-check, the fixes, veraPDF, PAC, the checklist, and then the technical
/// details (how Word made the PDF, and the PDF metadata changes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub summary: Summary,
    /// All findings of stage 1, errors and warnings.
    pub precheck: Vec<Finding>,
    pub fixes: FixesSection,
    pub verapdf: VeraPdfSection,
    pub pac: PacSection,
    pub checklist: HumanChecklist,
    pub export: Option<ExportInfo>,
    pub pdf_metadata: MetadataSection,
}

/// Everything that [`build`] needs.
pub struct Input {
    /// The .docx file name (not the path).
    pub document: String,
    /// The local time of the run, `yyyy-mm-dd hh:mm:ss`.
    pub generated_local: String,
    pub findings: Vec<Finding>,
    /// The fix log of the document.
    pub applied_fixes: Vec<FixEntry>,
    /// The name of the original Word document, when the document is an
    /// accessible copy.
    pub original: Option<String>,
    /// The PDF file name, when stage 3 wrote one.
    pub pdf: Option<String>,
    pub export: Option<ExportInfo>,
    /// Stage 4. `None` when it did not run (no PDF).
    pub metadata: Option<pdfmeta::Report>,
    /// Stage 5. `None` when it did not run (no PDF).
    pub verapdf: Option<Validation>,
}

/// Why a stage did not run, when it did not: the PDF was not made.
const NO_PDF: &str = "Not run. No PDF was made, because the document has errors that must be fixed first (see the pre-check findings).";

/// Works out the verdict and puts the results in a [`Report`].
pub fn build(input: Input) -> Report {
    let precheck_errors = count(&input.findings, Severity::Error);
    let precheck_warnings = count(&input.findings, Severity::Warning);
    let metadata_errors = input.metadata.as_ref().map_or(0, |m| {
        m.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    });
    let metadata_warnings = input
        .metadata
        .as_ref()
        .map_or(0, |m| m.issues.len() - metadata_errors);
    let verapdf_failed_rules = input.verapdf.as_ref().map_or(0, |v| {
        if v.passed() {
            0
        } else {
            v.ua1.findings.len().max(1)
        }
    });
    let (verapdf_errors, verapdf_warnings) = input.verapdf.as_ref().map_or((0, 0), |v| {
        let warnings = match &v.wcag {
            WcagOutcome::Ran(result) => result.findings.len(),
            _ => 0,
        };
        let errors = if v.passed() {
            0
        } else {
            v.ua1.findings.len().max(1)
        };
        (errors, warnings)
    });

    let errors = precheck_errors + metadata_errors + verapdf_errors;
    let warnings = precheck_warnings + metadata_warnings + verapdf_warnings;
    let incomplete = input.metadata.is_none() || input.verapdf.is_none();

    let reason = if errors > 0 {
        let mut parts = Vec::new();
        if precheck_errors > 0 {
            parts.push(format!(
                "{} found by the pre-check of the Word document",
                plural(precheck_errors, "error")
            ));
        }
        if metadata_errors > 0 {
            parts.push(format!(
                "{} in the PDF metadata or tags",
                plural(metadata_errors, "error")
            ));
        }
        if verapdf_errors > 0 {
            parts.push(format!(
                "{} of the PDF/UA-1 check (veraPDF)",
                plural(verapdf_errors, "failed rule")
            ));
        }
        Some(format!("The report has {}.", parts.join(", ")))
    } else if incomplete {
        Some("A stage of the check did not run, so the result cannot be a pass.".to_string())
    } else {
        None
    };
    let verdict = if reason.is_some() {
        Verdict::Fail
    } else {
        Verdict::Pass
    };

    let metadata = match input.metadata {
        Some(report) => MetadataSection::Done(report),
        None => MetadataSection::NotRun {
            reason: NO_PDF.to_string(),
        },
    };
    let verapdf = match input.verapdf {
        Some(validation) => VeraPdfSection::Ran {
            verapdf_version: validation.verapdf_version().map(str::to_string),
            validation,
        },
        None => VeraPdfSection::NotRun {
            reason: NO_PDF.to_string(),
        },
    };

    Report {
        schema_version: SCHEMA_VERSION,
        summary: Summary {
            verdict,
            errors,
            warnings,
            precheck_errors,
            precheck_warnings,
            pdf_failed_rules: verapdf_failed_rules,
            reason,
            tool: "adapdf".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            generated_local: input.generated_local,
            document: input.document,
            pdf: input.pdf,
        },
        export: input.export,
        precheck: input.findings,
        fixes: FixesSection {
            original: input.original,
            applied: input.applied_fixes,
        },
        pdf_metadata: metadata,
        verapdf,
        pac: PacSection {
            report_file: None,
            pac_version: None,
            result: None,
        },
        checklist: HumanChecklist {
            items: CHECKLIST
                .iter()
                .map(|text| ChecklistItem {
                    text: (*text).to_string(),
                    done: false,
                })
                .collect(),
            name: None,
            date: None,
        },
    }
}

fn count(findings: &[Finding], severity: Severity) -> usize {
    findings.iter().filter(|f| f.severity == severity).count()
}

fn plural(count: usize, word: &str) -> String {
    format!("{count} {word}{}", if count == 1 { "" } else { "s" })
}

impl Report {
    /// The JSON of the report.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a report can always be turned into JSON")
    }

    /// A short text for the console: the verdict, the errors, and the files.
    pub fn console_summary(&self, files: &[&Path]) -> String {
        let s = &self.summary;
        let mut out = format!(
            "{}: {} ({}, {}).\n",
            s.document,
            s.verdict.word(),
            plural(s.errors, "error"),
            plural(s.warnings, "warning")
        );
        if let Some(reason) = &s.reason {
            out.push_str(reason);
            out.push('\n');
        }
        for finding in self
            .precheck
            .iter()
            .filter(|f| f.severity == Severity::Error)
        {
            out.push_str(&format!("- [{}] {}\n", finding.rule_id, finding.message));
        }
        if let MetadataSection::Done(metadata) = &self.pdf_metadata {
            for issue in metadata
                .issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
            {
                out.push_str(&format!("- [PDF] {}\n", issue.message));
            }
        }
        if let VeraPdfSection::Ran { validation, .. } = &self.verapdf {
            for finding in &validation.ua1.findings {
                out.push_str(&format!(
                    "- [veraPDF {}-{}] {}\n",
                    finding.clause, finding.test_number, finding.message
                ));
            }
        }
        for file in files {
            out.push_str(&format!("Wrote {}\n", file.display()));
        }
        out
    }
}
