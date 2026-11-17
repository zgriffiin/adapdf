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

use crate::findings::{Finding, FixId, RuleId, Severity};
use crate::pdfmeta;
use crate::verapdf::{Validation, WcagOutcome};

/// The version of the JSON layout. It changes when a field is removed or its
/// meaning changes.
pub const SCHEMA_VERSION: u32 = 1;

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
    pub errors: usize,
    pub warnings: usize,
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

/// One fix that was applied before the export (section 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixEntry {
    pub fix: FixId,
    pub rule: RuleId,
    /// What was done, in plain words.
    pub summary: String,
}

/// Section 3. The `export` command applies no fixes, so `applied` is empty
/// there. `backup` is then the newest backup of the document that is next to it
/// (made by an earlier `adapdf fix`), if there is one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixesSection {
    pub applied: Vec<FixEntry>,
    pub backup: Option<String>,
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

/// Section 6: what a person records from PAC. It is empty when the report is
/// made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacSection {
    /// The PAC report file, or a link to it.
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

/// Section 7: the checklist and the sign-off, empty when the report is made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanChecklist {
    pub items: Vec<ChecklistItem>,
    pub name: Option<String>,
    pub date: Option<String>,
}

/// The whole report. The field order is the section order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub summary: Summary,
    pub export: Option<ExportInfo>,
    /// All findings of stage 1, errors and warnings.
    pub precheck: Vec<Finding>,
    pub fixes: FixesSection,
    pub pdf_metadata: MetadataSection,
    pub verapdf: VeraPdfSection,
    pub pac: PacSection,
    pub checklist: HumanChecklist,
}

/// Everything that [`build`] needs.
pub struct Input {
    /// The .docx file name (not the path).
    pub document: String,
    /// The local time of the run, `yyyy-mm-dd hh:mm:ss`.
    pub generated_local: String,
    pub findings: Vec<Finding>,
    pub applied_fixes: Vec<FixEntry>,
    pub backup: Option<String>,
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
            applied: input.applied_fixes,
            backup: input.backup,
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

/// The newest backup that `adapdf fix` wrote for `docx_stem` (the name of the
/// .docx without its extension), from the names in the folder. A backup is
/// `<stem>.backup-<yyyymmdd-hhmmss>.docx`, or `...-<n>.docx` when the name was
/// already used; the later time wins, and for the same time the larger `n`.
pub fn latest_backup<'a>(
    docx_stem: &str,
    names: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let prefix = format!("{docx_stem}.backup-");
    names
        .into_iter()
        .filter_map(|name| {
            let rest = name.strip_prefix(&prefix)?.strip_suffix(".docx")?;
            let stamp = rest.get(..15)?;
            let valid = stamp.bytes().enumerate().all(|(i, b)| {
                if i == 8 {
                    b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            });
            let attempt = match rest.get(15..)? {
                "" => 0,
                tail => tail.strip_prefix('-')?.parse::<u32>().ok()?,
            };
            valid.then(|| ((stamp.to_string(), attempt), name))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, name)| name.to_string())
}
