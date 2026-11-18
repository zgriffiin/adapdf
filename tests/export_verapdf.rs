//! Word, veraPDF and the report together: the real `adapdf export` and the
//! exit criterion of stage 5 (test plan rows C-05, S4-11, S5-10, S5-11).
//!
//! Needs desktop Word and the bundled veraPDF, so it is built only with both
//! cargo features:
//!
//! ```text
//! powershell -ExecutionPolicy Bypass -File scripts/fetch-vendor.ps1   # once
//! cargo test --features windows_word,verapdf --test export_verapdf
//! ```
//!
//! Every export starts the tool's own hidden Word (5 to 7 seconds), and never
//! touches a Word that a person has open: the `WINWORD.EXE` processes are listed
//! with `tasklist` before and after, and the lists must be equal. The folders
//! and files of the tests are removed at the end.
#![cfg(all(windows, feature = "windows_word", feature = "verapdf"))]

mod export_support;
mod word_support;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use adapdf::export::ExportOptions;
use adapdf::findings::{RuleId, Severity};
use adapdf::fix::Timestamp;
use adapdf::fixtures::{FixtureFonts, catalogue, clean};
use adapdf::pipeline::{Exit, RealStages, Run, export_docx, output_paths};
use adapdf::report::VeraPdfSection;
use adapdf::verapdf::Tool;
use export_support::{contains, count, names};
use serde_json::Value;
use word_support::{TempDir, exclusive, wait_for_word_pids, word_pids};

/// The 35 pass fixtures of S5-10: no picture, except IMG-006 (an equation).
const S5_10: [RuleId; 35] = {
    use RuleId::*;
    [
        Doc001, Doc002, Doc003, Doc004, Doc005, Doc006, Hdg001, Hdg002, Hdg003, Hdg004, Hdg006,
        Lst001, Lst002, Txb001, Txb002, Txb003, Txb004, Tbl001, Tbl002, Tbl003, Tbl004, Tbl005,
        Tbl006, Lnk001, Lnk002, Lnk003, Lnk004, Lnk005, Clr001, Clr002, Txt001, Txt002, Txt003,
        Txt004, Img006,
    ]
};

fn tool() -> Tool {
    let tool = Tool::locate();
    if let Err(error) = tool.check() {
        panic!(
            "{error}\nThese tests need the bundled tools. Run: \
             powershell -ExecutionPolicy Bypass -File scripts/fetch-vendor.ps1"
        );
    }
    tool
}

fn now() -> Timestamp {
    Timestamp::now()
}

/// The export of a .docx with the real Word and the real veraPDF.
fn run_real(docx: &Path) -> Run {
    let stages = RealStages {
        fonts: &FixtureFonts,
        tool: tool(),
        options: ExportOptions::default(),
    };
    export_docx(docx, &stages, &FixtureFonts, now())
}

/// The failed rules of the PDF/UA-1 profile in a run, as `clause-test`.
fn failed_rules(run: &Run) -> Vec<String> {
    match &run.report.as_ref().expect("a report").verapdf {
        VeraPdfSection::Ran { validation, .. } => validation
            .ua1
            .findings
            .iter()
            .map(|f| format!("{}-{}", f.clause, f.test_number))
            .collect(),
        VeraPdfSection::NotRun { reason } => vec![format!("not run: {reason}")],
    }
}

// C-05 and S6-09 with the real program
#[test]
fn cli_export_exit_0_on_a_pass() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("cli-export-pass");
    let fixture = catalogue()
        .into_iter()
        .find(|f| f.rule == RuleId::Doc001 && !f.fails)
        .unwrap();
    let docx = dir.write("Guide.docx", &fixture.bytes);
    let docx_bytes = fs::read(&docx).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .args(["export"])
        .arg(&docx)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(
        stdout.starts_with("Guide.docx: Pass (0 errors, 0 warnings)."),
        "{stdout}"
    );
    // Everything that the command made is in the folder of the guide, and nothing
    // else is next to the original.
    assert_eq!(names(dir.path()), ["Guide - Adapdf", "Guide.docx"]);
    assert_eq!(
        names(&dir.path().join("Guide - Adapdf")),
        [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.pdf"
        ]
    );
    assert_eq!(
        fs::read(&docx).unwrap(),
        docx_bytes,
        "the .docx is unchanged"
    );

    let (pdf, html, json) = output_paths(&docx);
    let pdf = fs::read(pdf).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(
        contains(&pdf, b"pdfuaid:part>1<"),
        "stage 4 added the PDF/UA part"
    );
    let json: Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(json["summary"]["verdict"], "pass");
    assert_eq!(json["verapdf"]["status"], "ran");
    assert_eq!(json["verapdf"]["verapdf_version"], "1.28.2");
    assert_eq!(json["verapdf"]["validation"]["ua1"]["failed_rules"], 0);
    assert!(
        json["export"]["word_build"]
            .as_str()
            .unwrap()
            .starts_with("16.0")
    );
    let html = fs::read_to_string(html).unwrap();
    assert!(html.contains("Result: <strong>Pass</strong>"));
    assert_eq!(wait_for_word_pids(&before), before, "Word was quit");
}

// S5-10, the exit criterion
#[test]
fn verapdf_zero_failures_on_pass_fixtures() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s5-10");
    let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for rule in S5_10 {
        let fixture = catalogue()
            .into_iter()
            .find(|f| f.rule == rule && !f.fails)
            .expect("the pass fixture exists");
        let docx = dir.write(&fixture.file_name(), &fixture.bytes);
        let run = run_real(&docx);
        let rules = failed_rules(&run);
        eprintln!(
            "{}: exit {} failed rules {rules:?}",
            fixture.file_name(),
            run.exit.code()
        );
        assert_ne!(
            run.exit,
            Exit::ToolFailure,
            "{}: {}",
            fixture.file_name(),
            run.stderr
        );
        if !rules.is_empty() || run.exit != Exit::Pass {
            failures.insert(fixture.file_name(), rules);
        }
    }
    assert!(
        failures.is_empty(),
        "veraPDF failures after stage 4: {failures:#?}"
    );
    assert_eq!(wait_for_word_pids(&before), before);
}

/// The six pass fixtures that have a picture (the builder writes a small PNG
/// part now). They were left out of S5-10 until the builder could do this.
const PICTURES: [RuleId; 6] = {
    use RuleId::*;
    [Img001, Img002, Img003, Img004, Img005, Img007]
};

// S5-10, the picture fixtures that the first plan left out
#[test]
fn verapdf_zero_failures_on_the_picture_pass_fixtures() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s5-10-pictures");
    let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for rule in PICTURES {
        let fixture = catalogue()
            .into_iter()
            .find(|f| f.rule == rule && !f.fails)
            .expect("the pass fixture exists");
        let docx = dir.write(&fixture.file_name(), &fixture.bytes);
        let run = run_real(&docx);
        let rules = failed_rules(&run);
        eprintln!(
            "{}: exit {} failed rules {rules:?}",
            fixture.file_name(),
            run.exit.code()
        );
        assert_ne!(
            run.exit,
            Exit::ToolFailure,
            "{}: {}",
            fixture.file_name(),
            run.stderr
        );
        if !rules.is_empty() || run.exit != Exit::Pass {
            failures.insert(fixture.file_name(), rules);
        }
    }
    assert!(
        failures.is_empty(),
        "veraPDF failures after stage 4: {failures:#?}"
    );
    assert_eq!(wait_for_word_pids(&before), before);
}

// S5-11: information only. The failing fixtures of the warning rules, exported
// by Word and checked: which veraPDF rules fail? No pass condition, except that
// the tool does not fail. TBL-003 is an error since 2026-11-18 (its fixture fails
// 7.2-42), so stage 1 blocks it and it is not in this list.
#[test]
fn verapdf_on_warning_fixtures_is_recorded() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s5-11");
    let mut table = Vec::new();
    for fixture in catalogue()
        .into_iter()
        .filter(|f| f.fails && f.rule.severity() == Severity::Warning)
    {
        let docx = dir.write(&fixture.file_name(), &fixture.bytes);
        let run = run_real(&docx);
        assert_ne!(
            run.exit,
            Exit::ToolFailure,
            "{}: {}",
            fixture.file_name(),
            run.stderr
        );
        let line = match &run.report {
            Some(report) if report.summary.pdf.is_none() => {
                "blocked by an error of another rule".to_string()
            }
            Some(_) => format!(
                "exit {}, failed rules {:?}",
                run.exit.code(),
                failed_rules(&run)
            ),
            None => unreachable!("a tool failure was handled above"),
        };
        eprintln!("S5-11 {}: {line}", fixture.file_name());
        table.push((fixture.file_name(), line));
    }
    assert!(!table.is_empty());
    assert_eq!(wait_for_word_pids(&before), before);
}

// S4-11: what Word already writes, and what stage 4 adds to it.
#[test]
fn pdfmeta_on_a_real_word_export() {
    use adapdf::export::export_pdf;
    use adapdf::pdfmeta::{self, Field, SourceMetadata};
    use adapdf::verapdf::validate;

    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s4-11");
    let documents = [
        (
            "DOC-001-pass.docx",
            catalogue()
                .into_iter()
                .find(|f| f.rule == RuleId::Doc001 && !f.fails)
                .unwrap()
                .bytes,
        ),
        (
            "accents.docx",
            clean()
                .title("Guía de uso: déjà vu")
                .language("es-US")
                .build(),
        ),
        // A table: Word writes `TH` for the first row and the first column and no
        // `/Scope`, and stage 4 adds the scopes (S5-14).
        (
            "TBL-001-pass.docx",
            catalogue()
                .into_iter()
                .find(|f| f.rule == RuleId::Tbl001 && !f.fails)
                .unwrap()
                .bytes,
        ),
    ];
    for (name, bytes) in documents {
        let docx = dir.write(name, &bytes);
        let pdf = docx.with_extension("pdf");
        export_pdf(&docx, &pdf, &FixtureFonts, &ExportOptions::default()).unwrap();
        let original = fs::read(&pdf).unwrap();
        let model = adapdf::model::Document::open(&docx).unwrap();
        let outcome =
            pdfmeta::check_bytes(&original, &SourceMetadata::from_document(&model)).unwrap();
        let fields: Vec<Field> = outcome.report.changes.iter().map(|c| c.field).collect();
        eprintln!(
            "S4-11 {name}: Word wrote everything but {fields:?}; issues {:?}",
            outcome.report.issues
        );
        // Word's own title, language and DisplayDocTitle are left alone: nothing
        // that Word wrote is replaced. Only the PDF/UA part is added.
        assert!(
            outcome.report.changes.iter().all(|c| c.old.is_none()),
            "{name}"
        );
        assert_eq!(fields, [Field::XmpPdfUaPart], "{name}");
        assert!(outcome.report.issues.is_empty(), "{name}");
        // A table gets its scopes: 2 column headings and the 2 row headings below.
        let scopes: Vec<(usize, usize, usize)> = outcome
            .report
            .table_scopes
            .iter()
            .map(|t| (t.table, t.column, t.row))
            .collect();
        if name.starts_with("TBL") {
            assert_eq!(scopes, [(1, 2, 2)], "{name}");
        } else {
            assert!(scopes.is_empty(), "{name}");
        }
        assert!(outcome.report.unscoped_headers.is_empty(), "{name}");
        // The prefix property: Word's bytes are not rewritten.
        let fixed = outcome.apply(&original);
        assert!(
            fixed.starts_with(&original) && fixed.len() > original.len(),
            "{name}"
        );
        // And veraPDF accepts the result (its own reader reads the update).
        fs::write(&pdf, &fixed).unwrap();
        let validation = validate(&tool(), &pdf).unwrap();
        assert!(
            validation.passed(),
            "{name}: {:?}",
            validation
                .ua1
                .findings
                .iter()
                .map(|f| (&f.clause, f.test_number))
                .collect::<Vec<_>>()
        );
        let again = pdfmeta::check_bytes(&fixed, &SourceMetadata::from_document(&model)).unwrap();
        assert_eq!(again.report.changes.len(), 0, "{name}: stage 4 is stable");
        assert!(again.appended.is_empty(), "{name}: stage 4 is stable");
    }
    assert_eq!(wait_for_word_pids(&before), before);
}

// S5-12: the exit criterion for links. A guide with links that have no ScreenTip
// (the three ways that Word writes a link) and a table of contents made of links
// is refused by stage 1 (LNK-004, LNK-005). After `adapdf fix` has done both
// fixes, `adapdf export` gives a PDF that veraPDF passes.
#[test]
fn verapdf_zero_failures_after_the_link_fixes() {
    use std::io::Write;
    use std::process::Stdio;

    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s5-12");
    let url = "https://example.org/parking";
    let original = adapdf::fixtures::DocxBuilder::new()
        .title("Adapdf test guide")
        .language("en-US")
        .template(adapdf::rules::document::TEMPLATE_FILE_NAME)
        .toc(&[("Test guide", 1)], false)
        .paragraph(
            adapdf::fixtures::Para::new("Test guide")
                .style("Heading1")
                .bookmark("_Toc1"),
        )
        .paragraph(adapdf::fixtures::Para::new("Read the ").hyperlink("parking policy", url))
        .paragraph(
            adapdf::fixtures::Para::empty()
                .field(&format!(r#"HYPERLINK "{url}""#), "the same policy"),
        )
        .paragraph(
            adapdf::fixtures::Para::empty()
                .simple_field(&format!(r#"HYPERLINK "{url}""#), "and again"),
        )
        .build();
    let docx = dir.write("Guide.docx", &original);

    // Before: stage 1 refuses the export, with the two new rules.
    let run = run_real(&docx);
    assert_eq!(run.exit, Exit::ErrorsFound, "{}", run.stderr);
    assert!(run.report.as_ref().unwrap().summary.pdf.is_none());
    assert!(!dir.path().join("Guide - Adapdf").join("Guide.pdf").exists());
    let rules: Vec<RuleId> = adapdf::check_bytes(&original, &FixtureFonts)
        .unwrap()
        .iter()
        .map(|f| f.rule_id)
        .collect();
    assert_eq!(
        rules,
        [
            RuleId::Lnk004,
            RuleId::Lnk004,
            RuleId::Lnk004,
            RuleId::Lnk005
        ]
    );

    // `adapdf fix`, with a yes for each of the 4 fixes.
    let mut child = Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .arg("fix")
        .arg(&docx)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"y\ny\ny\ny\n")
        .unwrap();
    let fixed = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&fixed.stdout);
    assert_eq!(fixed.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("Fixes applied (4):"), "{stdout}");

    // After: Word makes the PDF from the accessible copy (the original is not
    // changed since v0.2) and veraPDF passes it.
    assert_eq!(
        fs::read(&docx).unwrap(),
        original,
        "the original is not changed"
    );
    let copy = dir
        .path()
        .join("Guide - Adapdf")
        .join("Guide (accessible).docx");
    let run = run_real(&copy);
    let failed = failed_rules(&run);
    eprintln!("S5-12: exit {} failed rules {failed:?}", run.exit.code());
    assert_eq!(run.exit, Exit::Pass, "{failed:?} {}", run.stderr);
    assert!(failed.is_empty(), "{failed:?}");
    let pdf = fs::read(output_paths(&copy).0).unwrap();
    assert_eq!(
        count(&pdf, b"/Subtype/Link"),
        3,
        "the 3 links of the text; the table of contents has none"
    );
    assert_eq!(
        count(&pdf, b"/Contents("),
        3,
        "each link has its description"
    );
    assert!(contains(&pdf, b"/Contents(parking policy)"));
    assert!(contains(&pdf, b"/Contents(the same policy)"));
    assert!(contains(&pdf, b"/Contents(and again)"));
    assert_eq!(wait_for_word_pids(&before), before);
}

// S5-13 (the place of a failure in words, with the real Word and the real veraPDF):
// a guide with two tables is exported and checked. Whatever veraPDF reports at an
// element of the structure tree has its place in words, and the raw path is kept. A
// guide that passes has no failure to describe, and then the report still has the
// two tables in its PDF (the structure tree is read for the numbers).
#[test]
fn verapdf_places_are_in_words_for_a_guide_with_two_tables() {
    use adapdf::fixtures::Table;
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s5-places");
    let look = r#"<w:tblLook w:val="04A0" w:firstRow="1" w:lastRow="0" w:firstColumn="1" w:lastColumn="0" w:noHBand="0" w:noVBand="1"/>"#;
    let borders = r#"<w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders></w:tblPr>"#;
    let docx = dir.write(
        "Guide.docx",
        &clean()
            .table_style("TableGrid", "Table Grid", None, borders)
            .table(
                Table::new(&[&["Name", "Role"], &["Ann", "Lead"]])
                    .style("TableGrid")
                    .look(look),
            )
            .para("Between the tables.")
            .table(
                Table::new(&[&["Item", "Count"], &["Cups", "4"], &["Plates", "6"]])
                    .style("TableGrid")
                    .look(look),
            )
            .build(),
    );
    let run = run_real(&docx);
    assert!(
        matches!(run.exit, Exit::Pass | Exit::ErrorsFound),
        "{}",
        run.stderr
    );
    let report = run.report.expect("a report");
    let adapdf::report::VeraPdfSection::Ran { validation, .. } = &report.verapdf else {
        panic!("veraPDF ran");
    };
    for failure in validation.findings() {
        if let Some(context) = &failure.context
            && context.contains("(")
            && context.contains(" obj SE")
        {
            let place = failure.place.as_deref().unwrap_or_else(|| {
                panic!("a failure in the structure tree has a place in words: {context}")
            });
            println!(
                "{}-{}: {place}   [{context}]",
                failure.clause, failure.test_number
            );
            assert!(!place.contains("obj"), "{place}");
        }
    }
    let pdf = fs::read(dir.path().join("Guide - Adapdf").join("Guide.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(wait_for_word_pids(&before), before);
}
