//! The HTML form of the report (spec 02, stage 6).
//!
//! The report is itself an accessible document:
//!
//! - `<html lang="en">`, a `<title>`, one `h1`, and the sections as `h2` with
//!   `h3` below them (no level is skipped);
//! - real tables, each with a `<caption>` and `<th scope>`;
//! - the page has no form control. A static page cannot save what is typed or
//!   ticked, so the PAC result, the checklist and the sign-off are shown as text
//!   from the report, and are recorded in the Adapdf app. The page has no script
//!   at all;
//! - the style sheet is inline. Nothing is loaded from the network, and there is
//!   no external font, style sheet, script or image;
//! - every text colour has a contrast of at least 4.5:1 against its background
//!   (the test `report_html_is_accessible` checks the style sheet with
//!   `model::colour::contrast_ratio`), and a result is always a word (Pass,
//!   Fail, Error, Warning), never only a colour;
//! - the text of the document (titles, snippets, messages) is escaped.
//!
//! The markup is well-formed XML as well as HTML, so that a test can read it
//! with `roxmltree`.

use std::fmt::Write;

use super::{ExportInfo, FixesSection, MetadataSection, Report, VeraPdfSection, Verdict};
use crate::findings::{Finding, Severity};
use crate::verapdf::{ProfileResult, VeraFinding, WcagOutcome};

/// The seven sections of spec 02, stage 6, in order. The changes to the PDF after
/// the export are inside "Technical details".
pub const SECTIONS: [&str; 7] = [
    "Summary",
    "Pre-check findings",
    "Fixes applied",
    "veraPDF result",
    "PAC",
    "Human checklist and sign-off",
    "Technical details",
];

/// The style sheet. Each rule that sets a text colour sets its own background,
/// or has the page's white one. The test reads these values.
const CSS: &str = "\
body { font-family: Arial, Helvetica, sans-serif; line-height: 1.5; color: #161616; background: #ffffff; max-width: 70rem; margin: 0 auto; padding: 1rem 1.5rem; }
h1 { font-size: 1.75rem; margin: 0.5rem 0 1rem; }
h2 { font-size: 1.4rem; margin: 2rem 0 0.5rem; border-bottom: 2px solid #525252; padding-bottom: 0.25rem; }
h3 { font-size: 1.15rem; margin: 1.25rem 0 0.5rem; }
a { color: #0043ce; }
a:focus, summary:focus { outline: 3px solid #0f62fe; outline-offset: 2px; }
table { border-collapse: collapse; width: 100%; margin: 0.5rem 0 1rem; }
caption { text-align: left; font-weight: bold; padding: 0.25rem 0; color: #161616; }
th, td { border: 1px solid #8d8d8d; padding: 0.4rem 0.6rem; text-align: left; vertical-align: top; }
thead th { background: #e0e0e0; color: #161616; }
tbody th { background: #f4f4f4; color: #161616; }
.verdict { display: inline-block; border: 3px solid #161616; padding: 0.25rem 0.75rem; font-size: 1.25rem; }
.pass { color: #0e6027; background: #defbe6; border-color: #0e6027; }
.fail { color: #a2191f; background: #fff1f1; border-color: #a2191f; }
.note { color: #525252; }
summary { cursor: pointer; color: #0043ce; }
details { margin: 0.25rem 0; }
.context { font-family: Consolas, monospace; font-size: 0.85rem; word-break: break-all; color: #161616; }
";

/// The style sheet that the report has inline, for tests that read its colours.
pub fn css() -> &'static str {
    CSS
}

/// Makes text safe to put in HTML text or in a quoted attribute.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            // Control characters other than tab and line breaks are not valid
            // in XML or HTML text. A snippet can hold one.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => out.push('\u{FFFD}'),
            c => out.push(c),
        }
    }
    out
}

/// A table cell: the HTML inside it (already escaped), and whether it is the
/// header of its row.
struct Cell {
    html: String,
    row_header: bool,
}

/// A cell with HTML that the caller has already made safe.
fn html_cell(html: String) -> Cell {
    Cell {
        html,
        row_header: false,
    }
}

fn cell(text: &str) -> Cell {
    Cell {
        html: escape(text),
        row_header: false,
    }
}

fn header_cell(text: &str) -> Cell {
    Cell {
        html: escape(text),
        row_header: true,
    }
}

/// A data table with a caption. The first cell of a row is the row header when
/// it was made with [`header_cell`].
fn table(out: &mut String, caption: &str, headers: &[&str], rows: &[Vec<Cell>]) {
    let _ = writeln!(
        out,
        "<table>\n<caption>{}</caption>\n<thead>\n<tr>",
        escape(caption)
    );
    for header in headers {
        let _ = writeln!(out, "<th scope=\"col\">{}</th>", escape(header));
    }
    out.push_str("</tr>\n</thead>\n<tbody>\n");
    for row in rows {
        out.push_str("<tr>\n");
        for cell in row {
            if cell.row_header {
                let _ = writeln!(out, "<th scope=\"row\">{}</th>", cell.html);
            } else {
                let _ = writeln!(out, "<td>{}</td>", cell.html);
            }
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");
}

/// A two-column table of a name and a value, the name being the row header.
fn facts(out: &mut String, caption: &str, rows: &[(&str, String)]) {
    let rows: Vec<Vec<Cell>> = rows
        .iter()
        .map(|(name, value)| vec![header_cell(name), cell(value)])
        .collect();
    table(out, caption, &["Item", "Value"], &rows);
}

fn paragraph(out: &mut String, text: &str) {
    let _ = writeln!(out, "<p>{}</p>", escape(text));
}

fn plural(count: usize, word: &str) -> String {
    format!("{count} {word}{}", if count == 1 { "" } else { "s" })
}

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "Error",
        Severity::Warning => "Warning",
    }
}

/// The report as an HTML document.
pub fn render(report: &Report) -> String {
    let s = &report.summary;
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\" />\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n");
    let _ = writeln!(
        out,
        "<title>Accessibility report: {} ({})</title>",
        escape(&s.document),
        s.verdict.word()
    );
    let _ = writeln!(out, "<style>\n{CSS}</style>\n</head>\n<body>\n<main>");
    let _ = writeln!(
        out,
        "<h1>Accessibility report: {}</h1>",
        escape(&s.document)
    );

    out.push_str("<nav aria-label=\"Sections of this report\">\n<ol>\n");
    for (n, title) in SECTIONS.iter().enumerate() {
        let _ = writeln!(out, "<li><a href=\"#section-{}\">{}</a></li>", n + 1, title);
    }
    out.push_str("</ol>\n</nav>\n");

    section(&mut out, 1, |out| summary(out, report));
    section(&mut out, 2, |out| precheck(out, &report.precheck));
    section(&mut out, 3, |out| fixes(out, &report.fixes, &s.document));
    section(&mut out, 4, |out| verapdf(out, &report.verapdf));
    section(&mut out, 5, |out| pac(out, report));
    section(&mut out, 6, |out| checklist(out, report));
    section(&mut out, 7, |out| technical(out, report));

    out.push_str("</main>\n</body>\n</html>\n");
    out
}

fn section(out: &mut String, number: usize, body: impl FnOnce(&mut String)) {
    let _ = writeln!(
        out,
        "<section aria-labelledby=\"section-{number}\">\n<h2 id=\"section-{number}\">{}</h2>",
        SECTIONS[number - 1]
    );
    body(out);
    out.push_str("</section>\n");
}

fn summary(out: &mut String, report: &Report) {
    let s = &report.summary;
    let class = match s.verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
    };
    let _ = writeln!(
        out,
        "<p class=\"verdict {class}\">Result: <strong>{}</strong></p>",
        s.verdict.word()
    );
    if let Some(reason) = &s.reason {
        paragraph(out, reason);
    }
    let pdf_check = match &report.verapdf {
        VeraPdfSection::Ran { .. } => format!("{} failed", plural(s.pdf_failed_rules, "rule")),
        VeraPdfSection::NotRun { .. } => "Not run (no PDF was made)".to_string(),
    };
    let rows = vec![
        ("Result", s.verdict.word().to_string()),
        (
            "Word document (pre-check)",
            format!(
                "{}, {}",
                plural(s.precheck_errors, "error"),
                plural(s.precheck_warnings, "warning")
            ),
        ),
        ("PDF check (veraPDF PDF/UA-1)", pdf_check),
        ("Document", s.document.clone()),
        (
            "PDF",
            s.pdf
                .clone()
                .unwrap_or_else(|| "None was written".to_string()),
        ),
        ("Tool", format!("{} {}", s.tool, s.tool_version)),
        ("Date and time (local time)", s.generated_local.clone()),
    ];
    facts(out, "Summary of the result", &rows);
}

fn export_details(out: &mut String, export: &ExportInfo) {
    out.push_str("<h3>How Word made the PDF</h3>\n");
    let first_failed = export.first_failed_field.map_or_else(
        || "None".to_string(),
        |n| format!("Field {n} could not be updated"),
    );
    facts(
        out,
        "Word and the fields it updated before the export",
        &[
            ("Word build", export.word_build.clone()),
            ("Fields updated", export.fields_updated.to_string()),
            (
                "Tables of contents updated",
                export.tables_of_contents_updated.to_string(),
            ),
            ("Field that failed to update", first_failed),
            ("Export time in seconds", format!("{:.1}", export.seconds)),
        ],
    );
    let rows: Vec<Vec<Cell>> = export
        .parameters
        .iter()
        .map(|p| vec![header_cell(&p.name), cell(&p.value)])
        .collect();
    table(
        out,
        "Parameters of ExportAsFixedFormat",
        &["Parameter", "Value"],
        &rows,
    );
}

/// Section 7: how Word made the PDF, and the changes to the PDF after the export.
fn technical(out: &mut String, report: &Report) {
    match &report.export {
        Some(export) => export_details(out, export),
        None => paragraph(
            out,
            "Word did not make a PDF, so there is nothing to show about the export.",
        ),
    }
    out.push_str("<h3>Changes to the PDF after export</h3>\n");
    metadata(out, &report.pdf_metadata);
}

fn finding_rows(findings: &[&Finding]) -> Vec<Vec<Cell>> {
    findings
        .iter()
        .map(|f| {
            let mut how = f.how_to_fix.clone();
            if let Some(suggestion) = &f.suggestion {
                let _ = write!(how, " Suggestion (not applied): {suggestion}");
            }
            vec![
                header_cell(f.rule_id.as_str()),
                cell(severity_word(f.severity)),
                cell(&f.message),
                cell(&f.location.place),
                cell(&how),
            ]
        })
        .collect()
}

fn precheck(out: &mut String, findings: &[Finding]) {
    if findings.is_empty() {
        paragraph(
            out,
            "None. The pre-check found no problems in the Word document.",
        );
        return;
    }
    for (severity, title) in [(Severity::Error, "Errors"), (Severity::Warning, "Warnings")] {
        let group: Vec<&Finding> = findings.iter().filter(|f| f.severity == severity).collect();
        if group.is_empty() {
            continue;
        }
        let _ = writeln!(out, "<h3>{title} ({})</h3>", group.len());
        table(
            out,
            &format!("{title} found in the Word document"),
            &["Rule", "Severity", "What is wrong", "Where", "How to fix"],
            &finding_rows(&group),
        );
    }
}

fn fixes(out: &mut String, fixes: &FixesSection, document: &str) {
    if let Some(original) = &fixes.original {
        paragraph(
            out,
            &format!(
                "The original {original} was not changed. Adapdf made its fixes in {document}."
            ),
        );
    }
    if fixes.applied.is_empty() {
        paragraph(
            out,
            &format!("No fixes were made by Adapdf. The Word document is {document}."),
        );
        return;
    }
    let rows: Vec<Vec<Cell>> = fixes
        .applied
        .iter()
        .map(|f| {
            vec![
                header_cell(&f.time),
                cell(f.rule.as_str()),
                cell(&f.place),
                cell(&f.summary),
                cell(f.text.as_deref().unwrap_or("")),
            ]
        })
        .collect();
    table(
        out,
        "Fixes that Adapdf made in the Word document, oldest first",
        &[
            "Date and time",
            "Rule",
            "Where",
            "What was done",
            "Text typed or chosen",
        ],
        &rows,
    );
}

fn metadata(out: &mut String, section: &MetadataSection) {
    match section {
        MetadataSection::NotRun { reason } => paragraph(out, reason),
        MetadataSection::Done(report) => {
            paragraph(
                out,
                "After the export Adapdf changes only the metadata of the whole document, and adds a scope to table header cells that Word wrote without one. It never changes the text, the tag types or the reading order.",
            );
            if report.changes.is_empty() && report.table_scopes.is_empty() {
                paragraph(
                    out,
                    "None. The PDF already had all the metadata that the tool checks, and no table header cell needed a scope.",
                );
            }
            if !report.changes.is_empty() {
                let rows: Vec<Vec<Cell>> = report
                    .changes
                    .iter()
                    .map(|c| {
                        vec![
                            header_cell(&c.what),
                            cell(c.old.as_deref().unwrap_or("Not set")),
                            cell(&c.new),
                        ]
                    })
                    .collect();
                table(
                    out,
                    "Changes made to the PDF metadata",
                    &["What", "Before", "After"],
                    &rows,
                );
            }
            if !report.table_scopes.is_empty() {
                let rows: Vec<Vec<Cell>> = report
                    .table_scopes
                    .iter()
                    .map(|t| {
                        vec![
                            header_cell(&format!("Table {}", t.table)),
                            cell(&t.summary()),
                        ]
                    })
                    .collect();
                table(
                    out,
                    "Scopes added to table header cells",
                    &["Table", "What was added"],
                    &rows,
                );
            }
            if !report.unscoped_headers.is_empty() {
                paragraph(
                    out,
                    "Adapdf does not guess a scope for a heading in the middle of a table. Simplify these tables in Word: no merged or split cells, and headings only in the first row and the first column (pre-check rule TBL-003). Then export again.",
                );
                let rows: Vec<Vec<Cell>> = report
                    .unscoped_headers
                    .iter()
                    .map(|u| vec![cell(&u.describe())])
                    .collect();
                table(
                    out,
                    "Table header cells that were not given a scope",
                    &["What happened"],
                    &rows,
                );
            }
            if !report.issues.is_empty() {
                out.push_str("<h4>Problems that the tool does not fix</h4>\n");
                let rows: Vec<Vec<Cell>> = report
                    .issues
                    .iter()
                    .map(|i| {
                        vec![
                            header_cell(severity_word(i.severity)),
                            cell(&i.message),
                            cell(&i.how_to_fix),
                        ]
                    })
                    .collect();
                table(
                    out,
                    "PDF problems found in the metadata and tags",
                    &["Severity", "What is wrong", "How to fix"],
                    &rows,
                );
            }
        }
    }
}

fn vera_rows(findings: &[VeraFinding]) -> Vec<Vec<Cell>> {
    findings
        .iter()
        .map(|f| {
            let mut what = escape(&f.message);
            if let Some(place) = &f.place {
                let _ = write!(what, " First place: {}.", escape(place));
            }
            // veraPDF's own path stays, for people who use PAC or veraPDF.
            if let Some(context) = &f.context {
                let _ = write!(
                    what,
                    "<details><summary>Technical detail</summary><p class=\"context\">{}</p></details>",
                    escape(context)
                );
            }
            vec![
                header_cell(&format!("{}-{}", f.clause, f.test_number)),
                cell(severity_word(f.severity)),
                html_cell(what),
                cell(&f.how_to_fix),
                cell(&f.failed_checks.to_string()),
                cell(f.pre_check.map_or("None", |r| r.as_str())),
            ]
        })
        .collect()
}

const VERA_HEADERS: [&str; 6] = [
    "Clause and test",
    "Severity",
    "What is wrong",
    "How to fix",
    "Places",
    "Pre-check rule",
];

fn profile_facts(out: &mut String, caption: &str, result: &ProfileResult, version: &str) {
    let verdict = if result.passed() { "Pass" } else { "Fail" };
    facts(
        out,
        caption,
        &[
            ("Profile", result.profile_name.clone()),
            ("veraPDF version", version.to_string()),
            ("Result", verdict.to_string()),
            ("Failed rules", result.failed_rules.to_string()),
            ("Failed checks", result.failed_checks.to_string()),
            ("Passed rules", result.passed_rules.to_string()),
            ("Passed checks", result.passed_checks.to_string()),
        ],
    );
}

fn verapdf(out: &mut String, section: &VeraPdfSection) {
    let (version, validation) = match section {
        VeraPdfSection::NotRun { reason } => {
            paragraph(out, reason);
            return;
        }
        VeraPdfSection::Ran {
            verapdf_version,
            validation,
        } => (
            verapdf_version.as_deref().unwrap_or("not given"),
            validation,
        ),
    };
    out.push_str("<h3>PDF/UA-1</h3>\n");
    profile_facts(
        out,
        "Result of the PDF/UA-1 check",
        &validation.ua1,
        version,
    );
    if validation.ua1.findings.is_empty() {
        if validation.passed() {
            paragraph(out, "No PDF/UA-1 rule failed.");
        }
    } else {
        table(
            out,
            "PDF/UA-1 rules that failed",
            &VERA_HEADERS,
            &vera_rows(&validation.ua1.findings),
        );
    }

    out.push_str("<h3>WCAG 2.2 machine profile (warnings)</h3>\n");
    match &validation.wcag {
        WcagOutcome::NotAvailable => paragraph(
            out,
            "Not available. This veraPDF has no WCAG machine-check profile, so it was not run.",
        ),
        WcagOutcome::Failed(reason) => paragraph(
            out,
            &format!(
                "The WCAG profile did not give a result: {reason}. This does not change the result above."
            ),
        ),
        WcagOutcome::Ran(result) => {
            paragraph(
                out,
                &format!(
                    "{} ({}). These are warnings. They do not change the result above. Rules that repeat a PDF/UA-1 failure are shown only above.",
                    result.profile_name,
                    plural(result.findings.len(), "warning")
                ),
            );
            if !result.findings.is_empty() {
                table(
                    out,
                    "WCAG machine profile warnings",
                    &VERA_HEADERS,
                    &vera_rows(&result.findings),
                );
            }
        }
    }
}

fn recorded(report: &Report) -> String {
    let list = &report.checklist;
    match (&list.name, &list.date) {
        (Some(name), Some(date)) => format!(", recorded by {name} on {date}"),
        (Some(name), None) => format!(", recorded by {name}"),
        _ => String::new(),
    }
}

fn pac(out: &mut String, report: &Report) {
    paragraph(
        out,
        "PAC is the checker of record. Open the PDF in PAC, run the check, and save the PAC report next to the PDF. Then record the result in the Adapdf app.",
    );
    let pac = &report.pac;
    let line = match pac.result.as_deref() {
        Some(result) => {
            let word = if result == "pass" { "Pass" } else { "Fail" };
            let mut line = format!("PAC: {word}");
            if let Some(file) = &pac.report_file {
                let _ = write!(line, ", report {file}");
            }
            if let Some(version) = &pac.pac_version {
                let _ = write!(line, ", PAC {version}");
            }
            line.push_str(&recorded(report));
            line
        }
        None => "PAC: not checked yet".to_string(),
    };
    paragraph(out, &line);
}

fn checklist(out: &mut String, report: &Report) {
    let list = &report.checklist;
    paragraph(
        out,
        "A person checks each item in the PDF. The automatic checks cannot decide these. Record the PAC result and sign off in the Adapdf app (open this report there). Changes made in this page are not saved.",
    );
    let rows: Vec<Vec<Cell>> = list
        .items
        .iter()
        .map(|item| {
            vec![
                header_cell(&item.text),
                cell(if item.done {
                    "Checked"
                } else {
                    "Not checked yet"
                }),
            ]
        })
        .collect();
    table(
        out,
        "Checks that a person makes",
        &["Check", "Status"],
        &rows,
    );
    let signed = match (&list.name, &list.date) {
        (Some(name), Some(date)) => format!("Signed by {name} on {date}"),
        (Some(name), None) => format!("Signed by {name} (no date was given)"),
        (None, _) => "Not signed yet".to_string(),
    };
    let done = list.items.iter().filter(|item| item.done).count();
    paragraph(
        out,
        &format!(
            "Sign-off: {signed}. {done} of {} checks are checked.",
            list.items.len()
        ),
    );
}
