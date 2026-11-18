//! Tests of stage 6, rows S6-01 to S6-08 and S6-10 of the test plan. They need
//! no Word and no Java: the results of the other stages are made here.

use roxmltree::{Document as Xml, Node};
use serde_json::Value;

use super::*;
use crate::findings::{Finding, FixId, Location, RuleId};
use crate::model::colour::contrast_ratio;
use crate::model::theme::Rgb;
use crate::pdfmeta::{
    Change, Field, Issue, IssueKind, Report as MetadataReport, TableScopeChange, UnscopedHeader,
};
use crate::verapdf::{Profile, Validation, WcagOutcome, parse};

const DATE: &str = "2026-03-05 10:15:30";

fn finding(rule: RuleId, snippet: &str) -> Finding {
    let location = Location {
        place: format!("paragraph 5 \"{snippet}\", after the heading \"Setting up\""),
        heading: Some("Setting up".to_string()),
        snippet: Some(snippet.to_string()),
        paragraph_index: Some(4),
        part: "word/document.xml".to_string(),
    };
    Finding::new(
        rule,
        format!("Message of {rule}."),
        format!("Fix {rule} in Word."),
        location,
    )
}

fn document_finding(rule: RuleId) -> Finding {
    Finding::new(
        rule,
        format!("Message of {rule}."),
        format!("Fix {rule} in Word."),
        Location::document("docProps/core.xml"),
    )
}

/// 2 errors and 3 warnings.
fn findings() -> Vec<Finding> {
    vec![
        document_finding(RuleId::Doc001),
        finding(RuleId::Hdg002, "Overview"),
        document_finding(RuleId::Doc004),
        finding(RuleId::Txt002, "Justified text"),
        finding(RuleId::Lnk001, "click here"),
    ]
}

const UA1_FAIL: &str = include_str!("../../tests/data/verapdf/ua1-figure-without-alt.json");
const UA1_PASS: &str = include_str!("../../tests/data/verapdf/ua1-pass.json");
const WCAG: &str = include_str!("../../tests/data/verapdf/wcag-metadata-missing.json");

fn validation(ua1: &str, wcag: WcagOutcome) -> Validation {
    Validation {
        ua1: parse(ua1, Profile::Ua1).unwrap(),
        wcag,
    }
}

fn metadata_changes() -> MetadataReport {
    MetadataReport {
        changes: vec![
            Change {
                field: Field::XmpPdfUaPart,
                what: Field::XmpPdfUaPart.describe().to_string(),
                old: None,
                new: "1".to_string(),
            },
            Change {
                field: Field::Lang,
                what: Field::Lang.describe().to_string(),
                old: Some("en".to_string()),
                new: "en-US".to_string(),
            },
        ],
        table_scopes: vec![
            TableScopeChange {
                table: 1,
                column: 2,
                row: 3,
            },
            TableScopeChange {
                table: 3,
                column: 1,
                row: 0,
            },
        ],
        unscoped_headers: vec![UnscopedHeader {
            table: 2,
            row: 3,
            cell: 2,
        }],
        issues: Vec::new(),
    }
}

fn export_info() -> ExportInfo {
    ExportInfo {
        word_build: "16.0.20527".to_string(),
        parameters: vec![Parameter {
            name: "DocStructureTags".to_string(),
            value: "true".to_string(),
        }],
        fields_updated: 3,
        tables_of_contents_updated: 1,
        first_failed_field: None,
        seconds: 5.5,
    }
}

fn input() -> Input {
    Input {
        document: "Guide.docx".to_string(),
        generated_local: DATE.to_string(),
        findings: Vec::new(),
        applied_fixes: Vec::new(),
        original: None,
        pdf: Some("Guide.pdf".to_string()),
        export: Some(export_info()),
        metadata: Some(MetadataReport::default()),
        verapdf: Some(validation(UA1_PASS, WcagOutcome::NotAvailable)),
    }
}

/// A failing report with everything in it: 2 pre-check errors and 3 warnings,
/// two fixes in the log of an accessible copy, metadata changes, a failed
/// veraPDF rule and a WCAG warning.
fn full_report() -> Report {
    let mut input = input();
    input.document = "Guide (accessible).docx".to_string();
    input.original = Some("Guide.docx".to_string());
    input.findings = findings();
    input.applied_fixes = vec![
        FixEntry {
            time: "2026-03-04 09:00:00".to_string(),
            fix: FixId::SetDefaultLanguage,
            rule: RuleId::Doc002,
            place: "The whole document (its default language)".to_string(),
            summary: "Set the default language to en-US.".to_string(),
            text: None,
        },
        FixEntry {
            time: "2026-03-05 10:15:30".to_string(),
            fix: FixId::SetTitle,
            rule: RuleId::Doc001,
            place: "The document's properties (File > Info)".to_string(),
            summary: "Set the title.".to_string(),
            text: Some("Parks Training Guide".to_string()),
        },
    ];
    input.metadata = Some(metadata_changes());
    let mut validation = validation(
        UA1_FAIL,
        WcagOutcome::Ran(parse(WCAG, Profile::Wcag).unwrap()),
    );
    validation.ua1.findings[0].place = Some("Figure 3".to_string());
    input.verapdf = Some(validation);
    build(input)
}

fn pass_report() -> Report {
    build(input())
}

fn xml(html: &str) -> Xml<'_> {
    // `<!DOCTYPE html>` is a DTD to an XML reader.
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    Xml::parse_with_options(html, options).unwrap_or_else(|e| {
        panic!(
            "the report is well-formed: {e}
{html}"
        )
    })
}

fn text(node: Node) -> String {
    node.descendants()
        .filter(Node::is_text)
        .filter_map(|n| n.text())
        .collect::<String>()
}

fn elements<'a, 'input>(doc: &'a Xml<'input>, name: &str) -> Vec<Node<'a, 'input>> {
    doc.descendants().filter(|n| n.has_tag_name(name)).collect()
}

fn by_id<'a, 'input>(doc: &'a Xml<'input>, id: &str) -> Node<'a, 'input> {
    doc.descendants()
        .find(|n| n.attribute("id") == Some(id))
        .unwrap_or_else(|| panic!("no element with the id {id}"))
}

/// The text of a section: the elements between its `h2` and the next one.
fn section_text(doc: &Xml, number: usize) -> String {
    text(by_id(doc, &format!("section-{number}")).parent().unwrap())
}

#[test]
fn report_has_the_seven_sections_in_order() {
    for report in [full_report(), pass_report()] {
        let html = html::render(&report);
        let doc = xml(&html);
        let headings: Vec<String> = elements(&doc, "h2").into_iter().map(text).collect();
        assert_eq!(headings, html::SECTIONS);
        assert_eq!(
            html::SECTIONS,
            [
                "Summary",
                "Pre-check findings",
                "Fixes applied",
                "veraPDF result",
                "PAC",
                "Human checklist and sign-off",
                "Technical details"
            ]
        );
    }
    // A section with no content says so.
    let html = html::render(&pass_report());
    let doc = xml(&html);
    assert!(section_text(&doc, 2).contains("None."));
    assert!(section_text(&doc, 3).contains("No fixes were made by Adapdf."));
    assert!(section_text(&doc, 4).contains("No PDF/UA-1 rule failed."));
    // The technical details come last, and the changes to the PDF are in them.
    let technical = section_text(&doc, 7);
    assert!(technical.contains("How Word made the PDF"), "{technical}");
    assert!(
        technical.contains("Changes to the PDF after export"),
        "{technical}"
    );
    assert!(
        technical.contains("no table header cell needed a scope"),
        "{technical}"
    );
}

#[test]
fn report_summary_fields() {
    let report = full_report();
    let summary = &report.summary;
    assert_eq!(summary.verdict, Verdict::Fail);
    assert_eq!(
        summary.errors,
        2 + 1,
        "2 pre-check errors and 1 failed rule"
    );
    assert_eq!(summary.warnings, 3 + report_wcag_warnings(&report));
    // The three parts that the page shows apart.
    assert_eq!(summary.precheck_errors, 2);
    assert_eq!(summary.precheck_warnings, 3);
    assert_eq!(summary.pdf_failed_rules, 1);
    assert_eq!(summary.tool_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(summary.generated_local, DATE);

    let html = html::render(&report);
    let doc = xml(&html);
    let section = section_text(&doc, 1);
    assert!(section.contains("Result: Fail"), "{section}");
    assert!(section.contains(env!("CARGO_PKG_VERSION")), "{section}");
    assert!(section.contains(DATE), "{section}");
    // The counts are in a table, next to their names.
    assert_eq!(fact(&doc, "Result"), "Fail");
    // The page does not add the Word document and the PDF into one number.
    assert_eq!(
        fact(&doc, "Word document (pre-check)"),
        "2 errors, 3 warnings"
    );
    assert_eq!(fact(&doc, "PDF check (veraPDF PDF/UA-1)"), "1 rule failed");
    assert!(
        !section.contains("Errors") || !section.contains(&format!("Errors{}", summary.errors)),
        "{section}"
    );

    let pass = pass_report();
    assert_eq!(pass.summary.verdict, Verdict::Pass);
    assert_eq!((pass.summary.errors, pass.summary.warnings), (0, 0));
    assert_eq!(
        fact(&xml(&html::render(&pass)), "PDF check (veraPDF PDF/UA-1)"),
        "0 rules failed"
    );
    assert!(section_text(&xml(&html::render(&pass)), 1).contains("Result: Pass"));
}

/// The value next to a name in the first table of the report (the summary).
fn fact(doc: &Xml, name: &str) -> String {
    let table = elements(doc, "table")[0];
    let cells: Vec<String> = table
        .descendants()
        .filter(|n| n.has_tag_name("th") || n.has_tag_name("td"))
        .map(text)
        .collect();
    let at = cells
        .iter()
        .position(|c| c == name)
        .unwrap_or_else(|| panic!("no {name} in the summary table"));
    cells[at + 1].clone()
}

fn report_wcag_warnings(report: &Report) -> usize {
    match &report.verapdf {
        VeraPdfSection::Ran { validation, .. } => match &validation.wcag {
            WcagOutcome::Ran(result) => result.findings.len(),
            _ => 0,
        },
        VeraPdfSection::NotRun { .. } => 0,
    }
}

#[test]
fn report_lists_every_fix_of_the_log_and_names_its_original() {
    let report = full_report();
    let html = html::render(&report);
    let doc = xml(&html);
    let section = section_text(&doc, 3);
    // The statement comes first.
    assert!(
        section.contains(
            "The original Guide.docx was not changed. Adapdf made its fixes in Guide (accessible).docx."
        ),
        "{section}"
    );
    // Every fix of the log, also the one of an earlier day, with its text.
    for part in [
        "2026-03-04 09:00:00",
        "Set the default language to en-US.",
        "2026-03-05 10:15:30",
        "Set the title.",
        "Parks Training Guide",
        "The document's properties (File > Info)",
        "DOC-001",
        "DOC-002",
    ] {
        assert!(section.contains(part), "{part}: {section}");
    }
    assert!(!section.to_lowercase().contains("backup"), "{section}");
    assert!(!html.to_lowercase().contains("backup"));
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(json["fixes"]["original"], "Guide.docx");
    assert!(json["fixes"].get("backup").is_none());
    assert_eq!(json["fixes"]["applied"].as_array().unwrap().len(), 2);
    assert_eq!(json["fixes"]["applied"][0]["fix"], "set-default-language");
    assert_eq!(json["fixes"]["applied"][0]["rule"], "DOC-002");
    assert_eq!(json["fixes"]["applied"][0]["time"], "2026-03-04 09:00:00");
    assert_eq!(json["fixes"]["applied"][1]["text"], "Parks Training Guide");

    // An empty log: the section says so, and names the document.
    let none = section_text(&xml(&html::render(&pass_report())), 3);
    assert!(
        none.contains("No fixes were made by Adapdf. The Word document is Guide.docx."),
        "{none}"
    );
    assert!(!none.contains("The original"), "{none}");
    // A copy with an empty log still names its original.
    let mut input = input();
    input.document = "Guide (accessible).docx".to_string();
    input.original = Some("Guide.docx".to_string());
    let copy = section_text(&xml(&html::render(&build(input))), 3);
    assert!(
        copy.contains("The original Guide.docx was not changed."),
        "{copy}"
    );
    assert!(
        copy.contains(
            "No fixes were made by Adapdf. The Word document is Guide (accessible).docx."
        ),
        "{copy}"
    );
}

#[test]
fn report_shows_the_pac_result_as_text() {
    let html = html::render(&pass_report());
    let doc = xml(&html);
    let section = section_text(&doc, 5);
    assert!(section.contains("PAC: not checked yet"), "{section}");
    // No control: a static page cannot save what is typed.
    for tag in ["input", "select", "textarea", "label", "form", "button"] {
        assert!(elements(&doc, tag).is_empty(), "<{tag}> is not allowed");
    }
    let mut report = pass_report();
    report.pac = PacSection {
        report_file: Some("Guide.pac-report.pdf".to_string()),
        pac_version: Some("2024.1".to_string()),
        result: Some("pass".to_string()),
    };
    report.checklist.name = Some("A. Reviewer".to_string());
    report.checklist.date = Some("2026-03-06".to_string());
    let section = section_text(&xml(&html::render(&report)), 5);
    assert!(
        section.contains(
            "PAC: Pass, report Guide.pac-report.pdf, PAC 2024.1, recorded by A. Reviewer on 2026-03-06"
        ),
        "{section}"
    );
    report.pac.result = Some("fail".to_string());
    let section = section_text(&xml(&html::render(&report)), 5);
    assert!(section.contains("PAC: Fail, report"), "{section}");
}

#[test]
fn report_checklist_items_and_signoff() {
    let html = html::render(&pass_report());
    let doc = xml(&html);
    // The items are the seven of spec 02, word for word.
    let spec = include_str!("../../docs/specs/02-pipeline.md")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for item in CHECKLIST {
        assert!(spec.contains(&format!("- {item}")), "{item}");
    }
    let section = by_id(&doc, "section-6").parent().unwrap();
    let rows: Vec<Vec<String>> = section
        .descendants()
        .filter(|n| n.has_tag_name("tr"))
        .map(|tr| {
            tr.children()
                .filter(Node::is_element)
                .map(text)
                .collect::<Vec<_>>()
        })
        .filter(|cells| cells.len() == 2 && cells[0] != "Check")
        .collect();
    assert_eq!(rows.len(), 7);
    for (row, item) in rows.iter().zip(CHECKLIST) {
        assert_eq!(row[0], item);
        assert_eq!(row[1], "Not checked yet", "not ticked by the tool");
    }
    let text = text(section);
    assert!(text.contains("Sign-off: Not signed yet."), "{text}");
    assert!(
        text.contains("Record the PAC result and sign off in the Adapdf app (open this report there). Changes made in this page are not saved."),
        "{text}"
    );
    // The JSON has them too, all open.
    let json: Value = serde_json::from_str(&pass_report().to_json()).unwrap();
    let items = json["checklist"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 7);
    assert!(items.iter().all(|i| i["done"] == false));
    assert_eq!(json["checklist"]["name"], Value::Null);
    assert_eq!(json["checklist"]["date"], Value::Null);
}

fn hex(value: &str) -> Rgb {
    Rgb::from_hex(value.trim_start_matches('#')).unwrap_or_else(|| panic!("colour {value}"))
}

/// The colour of a property in a block of the style sheet.
fn property(block: &str, name: &str) -> Option<String> {
    block.split(';').find_map(|declaration| {
        let (key, value) = declaration.split_once(':')?;
        (key.trim() == name).then(|| value.trim().to_string())
    })
}

/// The report after the app saved a sign-off in it (row A-08).
fn signed_report() -> Report {
    let mut report = full_report();
    for item in report.checklist.items.iter_mut().take(5) {
        item.done = true;
    }
    report.checklist.name = Some("A. Reviewer".to_string());
    report.checklist.date = Some("2026-03-06".to_string());
    report
}

#[test]
fn report_html_is_accessible() {
    assert_report_html_is_accessible(&full_report());
}

#[test]
fn a_signed_report_html_is_accessible() {
    assert_report_html_is_accessible(&signed_report());
}

fn assert_report_html_is_accessible(report: &Report) {
    let html = html::render(report);
    let doc = xml(&html);

    // Language, title, one h1.
    assert_eq!(doc.root_element().attribute("lang"), Some("en"));
    assert!(!text(elements(&doc, "title")[0]).trim().is_empty());
    assert_eq!(elements(&doc, "h1").len(), 1);

    // Headings: no level is skipped.
    let mut level = 0;
    for node in doc.descendants().filter(|n| {
        n.tag_name().name().len() == 2 && n.tag_name().name().starts_with('h') && n.is_element()
    }) {
        let next: usize = node.tag_name().name()[1..].parse().unwrap();
        assert!(next <= level + 1, "h{next} follows h{level}");
        level = next;
    }

    // Tables: a caption, column headers with scope, and no layout table.
    let tables = elements(&doc, "table");
    assert!(tables.len() >= 8, "a full report has many tables");
    for table in tables {
        let caption = table
            .children()
            .find(|n| n.has_tag_name("caption"))
            .unwrap();
        assert!(!text(caption).trim().is_empty());
        let headers: Vec<Node> = table
            .descendants()
            .filter(|n| n.has_tag_name("th"))
            .collect();
        assert!(!headers.is_empty());
        for header in headers {
            assert!(
                matches!(header.attribute("scope"), Some("col" | "row")),
                "th without scope: {}",
                text(header)
            );
        }
        assert!(table.attribute("role").is_none(), "no layout table");
        // Every data row has a header cell or a data cell, and no nested table.
        assert_eq!(
            table
                .descendants()
                .filter(|n| n.has_tag_name("table"))
                .count(),
            1
        );
    }

    // The page has no form control: a static page cannot save what is typed.
    for tag in ["input", "select", "textarea", "button", "form"] {
        assert!(elements(&doc, tag).is_empty(), "<{tag}> is not allowed");
    }
    // The technical detail of a veraPDF failure is a closed `details`, with no script.
    assert!(!elements(&doc, "details").is_empty());
    for details in elements(&doc, "details") {
        assert_eq!(
            text(
                details
                    .children()
                    .find(|n| n.has_tag_name("summary"))
                    .unwrap()
            ),
            "Technical detail"
        );
    }

    // No script, and no vague link text (the phrases of LNK-001).
    assert!(elements(&doc, "script").is_empty());
    assert!(
        doc.descendants()
            .all(|n| n.attributes().all(|a| !a.name().starts_with("on")))
    );
    for link in elements(&doc, "a") {
        let words = text(link).trim().to_lowercase();
        assert!(
            !crate::rules::links::VAGUE_LINK_TEXTS.contains(&words.as_str()),
            "vague link text: {words}"
        );
    }

    // The result is a word, in the title and in the summary.
    assert!(text(elements(&doc, "title")[0]).contains("Fail"));
    for finding in report.precheck.iter() {
        assert!(html.contains(match finding.severity {
            Severity::Error => ">Error<",
            Severity::Warning => ">Warning<",
        }));
    }
}

#[test]
fn report_html_colours_have_enough_contrast() {
    let css = html::css();
    let white = hex("#ffffff");
    let mut pairs = 0;
    for block in css.split('}') {
        let Some((selector, declarations)) = block.split_once('{') else {
            continue;
        };
        let Some(colour) = property(declarations, "color") else {
            continue;
        };
        // A rule without its own background is on the page (or on the
        // background of the rule that it adds to, which also has its own).
        let background = property(declarations, "background").map_or(white, |b| hex(&b));
        let ratio = contrast_ratio(hex(&colour), background);
        assert!(
            ratio >= 4.5,
            "{}: {colour} on {background:?} is {ratio:.2}:1",
            selector.trim()
        );
        pairs += 1;
    }
    assert!(pairs >= 8, "the test found only {pairs} colour pairs");
    // The border colours that show a state are a second cue next to the word.
    // They are not text, but they also reach the 3:1 of WCAG 1.4.11.
    for block in css.split('}') {
        if let Some(border) = property(block.split_once('{').map_or("", |b| b.1), "border-color") {
            assert!(contrast_ratio(hex(&border), white) >= 3.0, "{border}");
        }
    }
}

#[test]
fn report_escapes_text_from_the_document() {
    let nasty = "<script>alert(1)</script> & \"x\"";
    let mut input = input();
    input.findings = vec![finding(RuleId::Doc004, nasty)];
    input.document = "A <b> & \"Guide\".docx".to_string();
    let report = build(input);
    let html = html::render(&report);
    assert!(!html.contains("<script"), "{html}");
    assert!(!html.contains("<b>"));
    let doc = xml(&html);
    assert!(elements(&doc, "script").is_empty());
    // The page shows the text as it was.
    assert!(
        elements(&doc, "td")
            .into_iter()
            .any(|td| text(td).contains(nasty)),
        "the snippet is shown as text"
    );
    assert!(text(elements(&doc, "h1")[0]).contains("A <b> & \"Guide\".docx"));
    // The JSON keeps it unchanged.
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(json["precheck"][0]["location"]["snippet"], nasty);
    assert_eq!(json["summary"]["document"], "A <b> & \"Guide\".docx");
}

#[test]
fn report_escape_handles_control_characters() {
    assert_eq!(super::html::escape("a\u{1}b\tc"), "a\u{FFFD}b\tc");
    assert_eq!(super::html::escape("'\"<>&"), "&#39;&quot;&lt;&gt;&amp;");
}

#[test]
fn report_json_matches_html() {
    let report = full_report();
    let html = html::render(&report);
    let doc = xml(&html);
    let page = text(doc.root_element());
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();

    assert_eq!(json["schema_version"], SCHEMA_VERSION);
    assert_eq!(json["summary"]["verdict"], "fail");
    let errors = json["summary"]["errors"].as_u64().unwrap();
    let warnings = json["summary"]["warnings"].as_u64().unwrap();
    assert_eq!(errors, 3);
    assert!(warnings >= 3);
    assert_eq!(json["summary"]["precheck_errors"], 2);
    assert_eq!(json["summary"]["precheck_warnings"], 3);
    assert_eq!(json["summary"]["pdf_failed_rules"], 1);
    assert_eq!(
        fact(&doc, "Word document (pre-check)"),
        "2 errors, 3 warnings"
    );
    assert_eq!(json["summary"]["generated_local"], DATE);
    assert!(page.contains(DATE));
    assert_eq!(json["summary"]["tool_version"], env!("CARGO_PKG_VERSION"));

    // Every pre-check finding: rule ID, message, location.
    let precheck = json["precheck"].as_array().unwrap();
    assert_eq!(precheck.len(), 5);
    for f in precheck {
        let id = f["rule_id"].as_str().unwrap();
        assert!(page.contains(id), "{id}");
        assert!(page.contains(f["message"].as_str().unwrap()));
        assert!(page.contains(f["how_to_fix"].as_str().unwrap()));
        // The page shows the place in words. The part name is data for tooling.
        assert!(page.contains(f["location"]["place"].as_str().unwrap()));
        if let Some(snippet) = f["location"]["snippet"].as_str() {
            assert!(page.contains(snippet), "{snippet}");
        }
        assert!(f["location"]["part"].is_string());
    }

    // Fixes and the original.
    for fix in json["fixes"]["applied"].as_array().unwrap() {
        assert!(page.contains(fix["summary"].as_str().unwrap()));
        assert!(page.contains(fix["time"].as_str().unwrap()));
        assert!(page.contains(fix["place"].as_str().unwrap()));
    }
    assert!(page.contains(json["fixes"]["original"].as_str().unwrap()));

    // Metadata changes.
    let changes = json["pdf_metadata"]["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 2);
    for change in changes {
        assert!(page.contains(change["what"].as_str().unwrap()));
        assert!(page.contains(change["new"].as_str().unwrap()));
    }

    // Table scopes: the cells that were given one and those that were not.
    let scopes = json["pdf_metadata"]["table_scopes"].as_array().unwrap();
    assert_eq!(scopes.len(), 2);
    assert!(page.contains("Table 1"));
    assert!(page.contains("2 header cells given Column, 3 given Row"));
    assert!(page.contains("Table 3"));
    assert!(page.contains("1 header cell given Column"));
    let unscoped = json["pdf_metadata"]["unscoped_headers"].as_array().unwrap();
    assert_eq!(unscoped.len(), 1);
    assert!(page.contains("header cell not given a scope (table 2, row 3, cell 2)"));

    // veraPDF: failed rules and warnings.
    assert_eq!(json["verapdf"]["status"], "ran");
    assert_eq!(json["verapdf"]["verapdf_version"], "1.28.2");
    assert!(page.contains("1.28.2"));
    let validation = &json["verapdf"]["validation"];
    let failures = validation["ua1"]["findings"].as_array().unwrap();
    assert_eq!(failures.len(), 1);
    for f in failures {
        assert!(page.contains(&format!(
            "{}-{}",
            f["clause"].as_str().unwrap(),
            f["test_number"]
        )));
        assert!(page.contains(f["message"].as_str().unwrap()));
        // The place in words is on the page, and the raw path is kept apart.
        assert_eq!(f["place"], "Figure 3");
        assert!(page.contains("First place: Figure 3."), "{page}");
        assert!(page.contains(f["context"].as_str().unwrap()));
    }
    assert_eq!(validation["wcag"]["status"], "ran");
    for f in validation["wcag"]["result"]["findings"].as_array().unwrap() {
        assert!(page.contains(f["message"].as_str().unwrap()));
    }

    // Export details.
    assert_eq!(json["export"]["word_build"], "16.0.20527");
    assert!(page.contains("16.0.20527"));
    assert_eq!(json["export"]["fields_updated"], 3);
}

#[test]
fn report_loads_nothing_from_the_network() {
    assert_report_loads_nothing_from_the_network(&full_report());
}

#[test]
fn a_signed_report_loads_nothing_from_the_network() {
    assert_report_loads_nothing_from_the_network(&signed_report());
}

fn assert_report_loads_nothing_from_the_network(report: &Report) {
    let html = html::render(report);
    assert!(
        !html.contains("http:") && !html.contains("https:"),
        "no address at all"
    );
    assert!(!html.contains("@import") && !html.contains("url("));
    let doc = xml(&html);
    for tag in [
        "script", "link", "img", "iframe", "object", "embed", "video", "audio", "source",
    ] {
        assert!(elements(&doc, tag).is_empty(), "<{tag}> is not allowed");
    }
    // The only `src` or `href` is a link to a place in this page.
    for node in doc.descendants().filter(Node::is_element) {
        assert!(node.attribute("src").is_none());
        if let Some(href) = node.attribute("href") {
            assert!(href.starts_with("#"), "{href}");
        }
    }
    // The style sheet is in the file.
    assert_eq!(elements(&doc, "style").len(), 1);
}

#[test]
fn a_report_is_pass_only_when_every_stage_ran_and_nothing_failed() {
    // Warnings and WCAG warnings do not change a pass.
    let mut with_warnings = input();
    with_warnings.findings = vec![document_finding(RuleId::Doc004)];
    with_warnings.verapdf = Some(validation(
        UA1_PASS,
        WcagOutcome::Ran(parse(WCAG, Profile::Wcag).unwrap()),
    ));
    let report = build(with_warnings);
    assert_eq!(report.summary.verdict, Verdict::Pass);
    assert!(report.summary.warnings >= 1);

    // A stage 4 error is a fail.
    let mut untagged = input();
    untagged.metadata = Some(MetadataReport {
        issues: vec![Issue {
            kind: IssueKind::NoStructTreeRoot,
            severity: Severity::Error,
            message: "The PDF has no tags.".to_string(),
            how_to_fix: "Export again.".to_string(),
        }],
        ..MetadataReport::default()
    });
    let report = build(untagged);
    assert_eq!(report.summary.verdict, Verdict::Fail);
    assert_eq!(report.summary.errors, 1);
    assert!(section_text(&xml(&html::render(&report)), 7).contains("The PDF has no tags."));

    // A veraPDF failure is a fail.
    let mut failed = input();
    failed.verapdf = Some(validation(UA1_FAIL, WcagOutcome::NotAvailable));
    assert_eq!(build(failed).summary.verdict, Verdict::Fail);

    // A stage that did not run is not a pass.
    let mut no_vera = input();
    no_vera.verapdf = None;
    let report = build(no_vera);
    assert_eq!(report.summary.verdict, Verdict::Fail);
    assert!(report.summary.reason.is_some());
}

#[test]
fn a_report_for_a_blocked_export_says_fail_and_lists_the_errors() {
    let mut blocked = input();
    blocked.findings = findings();
    blocked.pdf = None;
    blocked.export = None;
    blocked.metadata = None;
    blocked.verapdf = None;
    let report = build(blocked);
    assert_eq!(report.summary.verdict, Verdict::Fail);
    assert_eq!(report.summary.errors, 2);
    assert_eq!(report.summary.pdf, None);
    let html = html::render(&report);
    let doc = xml(&html);
    assert!(section_text(&doc, 2).contains("Message of DOC-001."));
    assert!(section_text(&doc, 4).contains("No PDF was made"));
    assert!(section_text(&doc, 7).contains("No PDF was made"));
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(json["pdf_metadata"]["status"], "not-run");
    assert_eq!(json["verapdf"]["status"], "not-run");
    assert_eq!(json["export"], Value::Null);
}

#[test]
fn the_json_has_the_sections_in_order_and_a_schema_version() {
    let json: Value = serde_json::from_str(&full_report().to_json()).unwrap();
    let keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    // serde_json sorts the keys unless `preserve_order` is on; the names are the
    // contract, so compare as sets.
    let mut expected = vec![
        "schema_version",
        "summary",
        "precheck",
        "fixes",
        "verapdf",
        "pac",
        "checklist",
        "export",
        "pdf_metadata",
    ];
    let mut keys = keys;
    keys.sort_unstable();
    expected.sort_unstable();
    assert_eq!(keys, expected);
    assert_eq!(json["schema_version"], 3);
    assert_eq!(json["pac"]["result"], Value::Null);
}

#[test]
fn console_summary_names_the_errors_and_the_files() {
    let report = full_report();
    let text = report.console_summary(&[std::path::Path::new("C:/g/Guide.accessibility.html")]);
    assert!(
        text.starts_with("Guide (accessible).docx: Fail (3 errors"),
        "{text}"
    );
    assert!(text.contains("[DOC-001] Message of DOC-001."));
    assert!(text.contains("[veraPDF 7.3-1]"), "{text}");
    assert!(text.contains("Wrote C:/g/Guide.accessibility.html"));
}

#[test]
fn the_json_of_a_report_reads_back_to_the_same_report() {
    // The app signs a report by reading its JSON, changing the checklist and
    // writing both files again (row A-08), so the JSON must carry everything.
    for report in [full_report(), pass_report()] {
        let json = report.to_json();
        let back: Report = serde_json::from_str(&json).unwrap();
        assert_eq!(back, report);
        assert_eq!(back.to_json(), json);
    }
}

#[test]
fn the_html_shows_a_sign_off_that_the_report_holds() {
    let mut report = pass_report();
    for item in report.checklist.items.iter_mut().take(5) {
        item.done = true;
    }
    report.checklist.name = Some("A. <Reviewer> & \"Co\"".to_string());
    report.checklist.date = Some("2026-03-06".to_string());
    let html = html::render(&report);
    let doc = xml(&html);
    let section = section_text(&doc, 6);
    assert!(
        section.contains(
            "Sign-off: Signed by A. <Reviewer> & \"Co\" on 2026-03-06. 5 of 7 checks are checked."
        ),
        "{section}"
    );
    let statuses: Vec<String> = elements(&doc, "td").into_iter().map(text).collect();
    assert_eq!(statuses.iter().filter(|t| *t == "Checked").count(), 5);
    assert_eq!(
        statuses.iter().filter(|t| *t == "Not checked yet").count(),
        2
    );
    // A name without a date.
    report.checklist.date = None;
    let section = section_text(&xml(&html::render(&report)), 6);
    assert!(
        section.contains("Signed by A. <Reviewer> & \"Co\" (no date was given)"),
        "{section}"
    );
    // An unsigned report has no such sentence.
    let unsigned = html::render(&pass_report());
    assert!(!unsigned.contains("Signed by"));
}
