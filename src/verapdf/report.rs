//! Reads veraPDF's JSON report and turns each failed rule into a
//! [`VeraFinding`].
//!
//! The JSON has `report.jobs[]`. Each job has `validationResult`, with
//! `details.ruleSummaries[]` (one entry for each failed rule) and counts. Only
//! the fields that the tool needs are read.

use serde::{Deserialize, Serialize};

use super::VeraPdfError;
use super::rules;
use crate::findings::{RuleId, Severity};

/// The profile that a result is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// The PDF/UA-1 profile. A failure means the PDF does not pass.
    Ua1,
    /// The WCAG 2.2 machine-check profile. Its results are warnings.
    Wcag,
}

/// One failed rule of a veraPDF profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VeraFinding {
    pub profile: Profile,
    /// `error` for the PDF/UA-1 profile, `warning` for the WCAG profile.
    pub severity: Severity,
    /// For example `ISO 14289-1:2014`.
    pub specification: String,
    /// The clause of the specification, for example `7.3`.
    pub clause: String,
    pub test_number: u32,
    /// The rule text of veraPDF, as it is.
    pub description: String,
    /// How many places failed this rule.
    pub failed_checks: u32,
    /// Where the first failure is, in veraPDF's own path form.
    pub context: Option<String>,
    /// The same place in words that a Word user knows (`Table 1, body row 1,
    /// cell 2`). `None` when the path does not name an element that the tool
    /// knows. See [`super::place`].
    #[serde(default)]
    pub place: Option<String>,
    /// veraPDF's message for the first failure.
    pub error_message: Option<String>,
    /// What is wrong, in plain language. For a rule that is not in the table,
    /// this is veraPDF's description with the rule ID in front of it.
    pub message: String,
    /// The likely cause in Word, and the fix.
    pub how_to_fix: String,
    /// The pre-check rule of stage 1 that finds the same problem in the .docx.
    pub pre_check: Option<RuleId>,
    /// `true` when the rule is in the table of [`rules::TABLE`].
    pub mapped: bool,
}

/// The result of one veraPDF run with one profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileResult {
    pub profile: Profile,
    /// veraPDF's name of the profile, for example `PDF/UA-1 validation profile`.
    pub profile_name: String,
    pub failed_rules: u32,
    pub failed_checks: u32,
    pub passed_rules: u32,
    pub passed_checks: u32,
    pub findings: Vec<VeraFinding>,
    /// The version of veraPDF that wrote the result, for example `1.28.2`
    /// (the `core` release in its report).
    pub verapdf_version: Option<String>,
}

impl ProfileResult {
    /// A profile passes only when no rule and no check failed.
    pub fn passed(&self) -> bool {
        self.failed_rules == 0 && self.failed_checks == 0 && self.findings.is_empty()
    }
}

#[derive(Deserialize)]
struct Output {
    report: ReportJson,
}

#[derive(Deserialize)]
struct ReportJson {
    #[serde(default, rename = "buildInformation")]
    build_information: Option<BuildInformation>,
    #[serde(default)]
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
struct BuildInformation {
    #[serde(default, rename = "releaseDetails")]
    release_details: Vec<Release>,
}

#[derive(Deserialize)]
struct Release {
    #[serde(default)]
    id: String,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Deserialize)]
struct Job {
    /// An array in current versions; one object in some earlier ones.
    #[serde(default, rename = "validationResult")]
    validation_result: Option<OneOrMany>,
    #[serde(default, rename = "taskException")]
    task_exception: Option<TaskException>,
}

/// veraPDF could not process the file (a damaged or encrypted PDF, for example).
#[derive(Deserialize)]
struct TaskException {
    #[serde(default)]
    exception: Option<String>,
    #[serde(default, rename = "exceptionMessage")]
    exception_message: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    Many(Vec<Validation>),
    One(Box<Validation>),
}

#[derive(Deserialize)]
struct Validation {
    details: Details,
    #[serde(default, rename = "jobEndStatus")]
    job_end_status: Option<String>,
    #[serde(default, rename = "profileName")]
    profile_name: String,
    #[serde(default)]
    compliant: Option<bool>,
}

#[derive(Deserialize)]
struct Details {
    #[serde(default, rename = "passedRules")]
    passed_rules: u32,
    #[serde(default, rename = "failedRules")]
    failed_rules: u32,
    #[serde(default, rename = "passedChecks")]
    passed_checks: u32,
    #[serde(default, rename = "failedChecks")]
    failed_checks: u32,
    #[serde(default, rename = "ruleSummaries")]
    rule_summaries: Vec<RuleSummary>,
}

#[derive(Deserialize)]
struct RuleSummary {
    #[serde(default)]
    specification: String,
    clause: String,
    #[serde(rename = "testNumber")]
    test_number: u32,
    #[serde(default, rename = "failedChecks")]
    failed_checks: u32,
    #[serde(default)]
    description: String,
    #[serde(default)]
    checks: Vec<Check>,
}

#[derive(Deserialize)]
struct Check {
    #[serde(default)]
    context: Option<String>,
    #[serde(default, rename = "errorMessage")]
    error_message: Option<String>,
}

/// Reads the text that veraPDF wrote to its standard output.
///
/// An output that is empty, is not JSON, or has no validation result is an
/// error, never a pass.
pub fn parse(output: &str, profile: Profile) -> Result<ProfileResult, VeraPdfError> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err(VeraPdfError::BadOutput("veraPDF wrote nothing".to_string()));
    }
    // veraPDF can write log lines before the JSON on some builds.
    let json = &trimmed[trimmed.find('{').unwrap_or(0)..];
    let parsed: Output = serde_json::from_str(json)
        .map_err(|e| VeraPdfError::BadOutput(format!("the output is not veraPDF JSON ({e})")))?;
    let verapdf_version = parsed.report.build_information.and_then(|info| {
        info.release_details
            .into_iter()
            .find(|release| release.id == "core")
            .and_then(|release| release.version)
    });
    let job = parsed
        .report
        .jobs
        .into_iter()
        .next()
        .ok_or_else(|| VeraPdfError::BadOutput("the report has no job".to_string()))?;
    let validation = match job.validation_result {
        Some(OneOrMany::Many(mut list)) if !list.is_empty() => list.remove(0),
        Some(OneOrMany::One(one)) => *one,
        _ => {
            let reason = match job.task_exception {
                Some(e) => format!(
                    "veraPDF could not read the PDF ({})",
                    e.exception_message
                        .or(e.exception)
                        .unwrap_or_else(|| "no reason given".to_string())
                ),
                None => "the report has no validation result".to_string(),
            };
            return Err(VeraPdfError::BadOutput(reason));
        }
    };
    if let Some(status) = &validation.job_end_status
        && status != "normal"
    {
        return Err(VeraPdfError::BadOutput(format!(
            "veraPDF did not finish normally ({status})"
        )));
    }

    let severity = match profile {
        Profile::Ua1 => Severity::Error,
        Profile::Wcag => Severity::Warning,
    };
    let details = validation.details;
    let mut findings: Vec<VeraFinding> = details
        .rule_summaries
        .into_iter()
        .map(|rule| finding(profile, severity, rule))
        .collect();
    // veraPDF does not write the rules in a fixed order. The tool does: by
    // clause (as numbers) and then by test number.
    findings.sort_by_cached_key(|f| (clause_numbers(&f.clause), f.test_number));
    // A report that says "not compliant" but lists no failed rule or check is
    // not a pass either. `passed()` looks at the counts.
    let mut failed_rules = details.failed_rules;
    if validation.compliant == Some(false) && failed_rules == 0 && findings.is_empty() {
        failed_rules = 1;
    }
    Ok(ProfileResult {
        profile,
        profile_name: validation.profile_name,
        failed_rules,
        failed_checks: details.failed_checks,
        passed_rules: details.passed_rules,
        passed_checks: details.passed_checks,
        findings,
        verapdf_version,
    })
}

/// `7.21.4.1` as `[7, 21, 4, 1]`, so that clause 7.2 comes before 7.21.
fn clause_numbers(clause: &str) -> Vec<u32> {
    clause
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

fn finding(profile: Profile, severity: Severity, rule: RuleSummary) -> VeraFinding {
    let first = rule.checks.first();
    let context = first.and_then(|c| c.context.clone());
    let error_message = first.and_then(|c| c.error_message.clone());
    // The table is for rules of ISO 14289-1. The WCAG profile repeats some of
    // them; its other rules have other clause numbers.
    let entry = if profile == Profile::Ua1 || rule.specification.starts_with("ISO 14289-1") {
        rules::lookup(&rule.clause, rule.test_number)
    } else {
        None
    };
    let (message, how_to_fix, pre_check) = match entry {
        Some(entry) => (
            entry.message.to_string(),
            entry.how_to_fix.to_string(),
            entry.pre_check,
        ),
        None => (
            format!(
                "veraPDF reports a problem (clause {}, test {}): {}",
                rule.clause, rule.test_number, rule.description
            ),
            format!(
                "Adapdf has no advice for this check yet. Read clause {} of the standard ({}), and look at the place in the PDF with PAC.",
                rule.clause,
                if rule.specification.is_empty() {
                    "the specification named in the report"
                } else {
                    rule.specification.as_str()
                }
            ),
            None,
        ),
    };
    VeraFinding {
        profile,
        severity,
        specification: rule.specification,
        clause: rule.clause,
        test_number: rule.test_number,
        description: rule.description,
        failed_checks: rule.failed_checks,
        context,
        place: None,
        error_message,
        message,
        how_to_fix,
        pre_check,
        mapped: entry.is_some(),
    }
}
