//! Output of `adapdf check`: plain text for people and JSON for tools.

use std::fmt::Write;

use serde::Serialize;

use crate::findings::{Finding, Severity};

/// The number of errors and warnings.
pub fn counts(findings: &[Finding]) -> (usize, usize) {
    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    (errors, findings.len() - errors)
}

/// True when a finding blocks export.
pub fn has_errors(findings: &[Finding]) -> bool {
    counts(findings).0 > 0
}

#[derive(Serialize)]
struct JsonReport<'a> {
    file: &'a str,
    errors: usize,
    warnings: usize,
    findings: &'a [Finding],
}

pub fn to_json(file: &str, findings: &[Finding]) -> String {
    let (errors, warnings) = counts(findings);
    serde_json::to_string_pretty(&JsonReport {
        file,
        errors,
        warnings,
        findings,
    })
    .expect("findings can always be turned into JSON")
}

fn plural(count: usize, word: &str) -> String {
    format!("{count} {word}{}", if count == 1 { "" } else { "s" })
}

pub fn to_text(file: &str, findings: &[Finding]) -> String {
    let (errors, warnings) = counts(findings);
    let mut out = format!("Checked: {file}\n");
    if findings.is_empty() {
        out.push_str("No problems found.\n");
        return out;
    }
    let _ = writeln!(
        out,
        "Found {} and {}.",
        plural(errors, "error"),
        plural(warnings, "warning")
    );
    for (severity, title) in [(Severity::Error, "ERRORS"), (Severity::Warning, "WARNINGS")] {
        let group: Vec<&Finding> = findings.iter().filter(|f| f.severity == severity).collect();
        if group.is_empty() {
            continue;
        }
        let note = match severity {
            Severity::Error => "these block export",
            Severity::Warning => "these do not block export",
        };
        let _ = writeln!(out, "\n{title} ({}, {note})", group.len());
        for finding in group {
            write_finding(&mut out, finding);
        }
    }
    if errors > 0 {
        out.push_str("\nFix the errors in Word, then run the check again.\n");
    }
    out
}

/// One finding as text: the message, where it is, how to fix it, and the
/// suggestion (marked "not applied").
pub fn write_finding(out: &mut String, f: &Finding) {
    let _ = writeln!(out, "\n[{}] {}", f.rule_id, f.message);
    let _ = writeln!(out, "    Where: {}", f.location.place);
    let _ = writeln!(out, "    How to fix: {}", f.how_to_fix);
    if let Some(suggestion) = &f.suggestion {
        let _ = writeln!(out, "    Suggestion (not applied): {suggestion}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::{Location, RuleId};

    fn finding(rule: RuleId) -> Finding {
        Finding::new(
            rule,
            "Message.",
            "Do this.",
            Location::document("word/settings.xml"),
        )
    }

    #[test]
    fn counts_by_severity() {
        let findings = [
            finding(RuleId::Doc001),
            finding(RuleId::Doc004),
            finding(RuleId::Doc005),
        ];
        assert_eq!(counts(&findings), (2, 1));
        assert!(has_errors(&findings));
        assert!(!has_errors(&findings[1..2]));
    }

    #[test]
    fn text_groups_errors_before_warnings() {
        let text = to_text(
            "a.docx",
            &[finding(RuleId::Doc004), finding(RuleId::Doc001)],
        );
        let errors = text.find("ERRORS (1").expect("errors group");
        let warnings = text.find("WARNINGS (1").expect("warnings group");
        assert!(errors < warnings, "{text}");
        assert!(text.contains("Found 1 error and 1 warning."), "{text}");
        assert!(
            text.contains("Where: Word's template setting (Developer > Document Template)"),
            "{text}"
        );
        assert!(!text.contains("word/settings.xml"), "{text}");
    }

    #[test]
    fn text_for_a_clean_document() {
        assert_eq!(
            to_text("a.docx", &[]),
            "Checked: a.docx\nNo problems found.\n"
        );
    }

    #[test]
    fn text_shows_suggestions_as_not_applied() {
        let f = finding(RuleId::Doc001).with_suggestion("My title");
        assert!(to_text("a.docx", &[f]).contains("Suggestion (not applied): My title"));
    }
}
