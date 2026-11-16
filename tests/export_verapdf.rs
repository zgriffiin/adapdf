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
use export_support::{contains, names};
use serde_json::Value;
use word_support::{TempDir, exclusive, wait_for_word_pids, word_pids};

/// The 33 pass fixtures of S5-10: no picture, except IMG-006 (an equation).
const S5_10: [RuleId; 33] = {
    use RuleId::*;
    [
        Doc001, Doc002, Doc003, Doc004, Doc005, Doc006, Hdg001, Hdg002, Hdg003, Hdg004, Hdg006,
        Lst001, Lst002, Txb001, Txb002, Txb003, Txb004, Tbl001, Tbl002, Tbl003, Tbl004, Tbl005,
        Tbl006, Lnk001, Lnk002, Lnk003, Clr001, Clr002, Txt001, Txt002, Txt003, Txt004, Img006,
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
    assert_eq!(
        names(dir.path()),
        [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.docx",
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
// the tool does not fail.
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
        assert_eq!(
            pdfmeta::check_bytes(&fixed, &SourceMetadata::from_document(&model))
                .unwrap()
                .report
                .changes
                .len(),
            0,
            "{name}: stage 4 is stable"
        );
    }
    assert_eq!(wait_for_word_pids(&before), before);
}
