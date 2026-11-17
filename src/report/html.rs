//! The HTML form of the report (spec 02, stage 6).
//!
//! The report is itself an accessible document:
//!
//! - `<html lang="en">`, a `<title>`, one `h1`, and the sections as `h2` with
//!   `h3` below them (no level is skipped);
//! - real tables, each with a `<caption>` and `<th scope>`;
//! - the checklist, the PAC fields and the sign-off are labelled form controls
//!   that work in a browser with no script. The page has no script at all;
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

/// The seven sections of spec 02, stage 6, in order.
pub const SECTIONS: [&str; 7] = [
    "Summary",
    "Pre-check findings",
    "Fixes applied",
    "PDF metadata changes",
    "veraPDF result",
    "PAC",
    "Human checklist",
];

/// The style sheet. Each rule that sets a text colour sets its own background,
/// or has the page's white one. The test reads these values.
const CSS: &str = "\
body { font-family: Arial, Helvetica, sans-serif; line-height: 1.5; color: #161616; background: #ffffff; max-width: 70rem; margin: 0 auto; padding: 1rem 1.5rem; }
h1 { font-size: 1.75rem; margin: 0.5rem 0 1rem; }
h2 { font-size: 1.4rem; margin: 2rem 0 0.5rem; border-bottom: 2px solid #525252; padding-bottom: 0.25rem; }
h3 { font-size: 1.15rem; margin: 1.25rem 0 0.5rem; }
a { color: #0043ce; }
a:focus, input:focus, select:focus { outline: 3px solid #0f62fe; outline-offset: 2px; }
table { border-collapse: collapse; width: 100%; margin: 0.5rem 0 1rem; }
caption { text-align: left; font-weight: bold; padding: 0.25rem 0; color: #161616; }
th, td { border: 1px solid #8d8d8d; padding: 0.4rem 0.6rem; text-align: left; vertical-align: top; }
thead th { background: #e0e0e0; color: #161616; }
tbody th { background: #f4f4f4; color: #161616; }
.verdict { display: inline-block; border: 3px solid #161616; padding: 0.25rem 0.75rem; font-size: 1.25rem; }
.pass { color: #0e6027; background: #defbe6; border-color: #0e6027; }
.fail { color: #a2191f; background: #fff1f1; border-color: #a2191f; }
.note { color: #525252; }
fieldset { border: 1px solid #8d8d8d; margin: 0.75rem 0; padding: 0.5rem 1rem; }
legend { font-weight: bold; color: #161616; }
.field { margin: 0.5rem 0; }
.field label { display: inline-block; min-width: 14rem; }
input[type=text], input[type=date], select { font-size: 1rem; padding: 0.25rem; border: 1px solid #525252; color: #161616; background: #ffffff; min-width: 20rem; }
.check { margin: 0.4rem 0; }
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
    section(&mut out, 3, |out| fixes(out, &report.fixes));
    section(&mut out, 4, |out| metadata(out, &report.pdf_metadata));
    section(&mut out, 5, |out| verapdf(out, &report.verapdf));
    section(&mut out, 6, pac);
    section(&mut out, 7, |out| checklist(out, report));

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
    let mut rows = vec![
        ("Result", s.verdict.word().to_string()),
        ("Errors", s.errors.to_string()),
        ("Warnings", s.warnings.to_string()),
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
    if let Some(export) = &report.export {
        rows.push(("Word build", export.word_build.clone()));
    }
    facts(out, "Summary of the result", &rows);
    if let Some(export) = &report.export {
        export_details(out, export);
    }
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

fn location_text(finding: &Finding) -> String {
    let loc = &finding.location;
    let mut place = match (&loc.paragraph_index, &loc.snippet) {
        (Some(index), Some(snippet)) => format!("Paragraph {} \"{snippet}\"", index + 1),
        _ => format!("The document ({})", loc.part),
    };
    if let Some(heading) = &loc.heading {
        let _ = write!(place, ", after the heading \"{heading}\"");
    }
    place
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
                cell(&location_text(f)),
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

fn fixes(out: &mut String, fixes: &FixesSection) {
    if fixes.applied.is_empty() {
        paragraph(out, "None. No fixes were applied in this run.");
    } else {
        let rows: Vec<Vec<Cell>> = fixes
            .applied
            .iter()
            .map(|f| {
                vec![
                    header_cell(f.rule.as_str()),
                    cell(&format!("{:?}", f.fix)),
                    cell(&f.summary),
                ]
            })
            .collect();
        table(
            out,
            "Fixes applied to the Word document",
            &["Rule", "Fix", "What was done"],
            &rows,
        );
    }
    match &fixes.backup {
        Some(name) => paragraph(
            out,
            &format!(
                "Backup file: {name}. The newest backup of this document that is next to it. The export itself does not change the Word document."
            ),
        ),
        None => paragraph(out, "Backup file: none was found next to the document."),
    }
}

fn metadata(out: &mut String, section: &MetadataSection) {
    match section {
        MetadataSection::NotRun { reason } => paragraph(out, reason),
        MetadataSection::Done(report) => {
            if report.changes.is_empty() {
                paragraph(
                    out,
                    "None. The PDF already had all the metadata that the tool checks.",
                );
            } else {
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
            if !report.issues.is_empty() {
                out.push_str("<h3>Problems that the tool does not fix</h3>\n");
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
            let mut what = f.message.clone();
            if let Some(context) = &f.context {
                let _ = write!(what, " First place: {context}");
            }
            vec![
                header_cell(&format!("{}-{}", f.clause, f.test_number)),
                cell(severity_word(f.severity)),
                cell(&what),
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

fn pac(out: &mut String) {
    paragraph(
        out,
        "PAC is the checker of record. Open the PDF in PAC, run the check, and save the PAC report next to the PDF. Then record the report and the result here.",
    );
    out.push_str("<fieldset>\n<legend>PAC report</legend>\n");
    out.push_str(
        "<div class=\"field\"><label for=\"pac-file\">PAC report file (a file name or a link)</label> <input type=\"text\" id=\"pac-file\" name=\"pac-file\" /></div>\n",
    );
    out.push_str(
        "<div class=\"field\"><label for=\"pac-version\">PAC version</label> <input type=\"text\" id=\"pac-version\" name=\"pac-version\" /></div>\n",
    );
    out.push_str(
        "<div class=\"field\"><label for=\"pac-result\">PAC result</label> <select id=\"pac-result\" name=\"pac-result\">\n\
<option value=\"\">Not checked yet</option>\n\
<option value=\"pass\">Pass: 0 errors</option>\n\
<option value=\"fail\">Fail: PAC found errors</option>\n\
</select></div>\n",
    );
    out.push_str("</fieldset>\n");
}

fn checklist(out: &mut String, report: &Report) {
    let list = &report.checklist;
    paragraph(
        out,
        "A person checks each item in the PDF and ticks it. The automatic checks cannot decide these.",
    );
    // A report that was signed in the app has the sign-off in it. The same
    // facts are in the form below, so a reader of the page sees them twice: as
    // text, and as the state of the controls.
    if let Some(name) = &list.name {
        let done = list.items.iter().filter(|item| item.done).count();
        let date = list
            .date
            .as_deref()
            .map_or_else(|| "no date was given".to_string(), |d| format!("on {d}"));
        paragraph(
            out,
            &format!(
                "Signed off by {name}, {date}. {done} of {} checks are ticked.",
                list.items.len()
            ),
        );
    }
    out.push_str("<fieldset>\n<legend>Checks that a person makes</legend>\n");
    for (n, item) in list.items.iter().enumerate() {
        let checked = if item.done {
            " checked=\"checked\""
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "<div class=\"check\"><input type=\"checkbox\" id=\"check-{n}\" name=\"check-{n}\"{checked} /> <label for=\"check-{n}\">{}</label></div>",
            escape(&item.text),
            n = n + 1
        );
    }
    out.push_str("</fieldset>\n<fieldset>\n<legend>Sign-off</legend>\n");
    let value = |v: &Option<String>| {
        v.as_deref()
            .map_or_else(String::new, |v| format!(" value=\"{}\"", escape(v)))
    };
    let _ = writeln!(
        out,
        "<div class=\"field\"><label for=\"signoff-name\">Name</label> <input type=\"text\" id=\"signoff-name\" name=\"signoff-name\"{} /></div>",
        value(&list.name)
    );
    let _ = writeln!(
        out,
        "<div class=\"field\"><label for=\"signoff-date\">Date</label> <input type=\"date\" id=\"signoff-date\" name=\"signoff-date\"{} /></div>",
        value(&list.date)
    );
    out.push_str("</fieldset>\n");
}
