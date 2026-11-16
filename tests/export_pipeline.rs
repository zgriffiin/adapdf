//! The `export` command without Word and without Java: `pipeline::export_docx`
//! with fake stages (rows C-06 and C-07, S3-12, S3-13, S3-16, S5-07, S5-08,
//! S6-09, and the exit-code halves of S3-04, S3-06 and S4-06).
//!
//! The command `adapdf export` is a thin call to this function with the real
//! Word and the real veraPDF (`pipeline::RealStages`). The fakes give any
//! result a stage can give, which the real stages cannot do on demand (a
//! veraPDF that reports 1 failure, a Word that is not installed, a PDF with no
//! tags). The real stages are tested in `tests/export_verapdf.rs` and
//! `tests/export_pipeline_word.rs` (features `windows_word` and `verapdf`).
#![cfg(windows)]

mod pdf_support;

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use adapdf::export::{ExportError, ExportParameters, ExportResult, FieldUpdate};
use adapdf::findings::{RuleId, Severity};
use adapdf::fix::Timestamp;
use adapdf::fixtures::{FixtureFonts, catalogue, clean};
use adapdf::pipeline::{Exit, Run, Stages, export_docx, output_paths};
use adapdf::report::Verdict;
use adapdf::verapdf::{Profile, Validation, VeraPdfError, WcagOutcome, parse};
use adapdf::word::WordError;
use pdf_support::{Pdf, TempDir};
use serde_json::Value;

const UA1_PASS: &str = include_str!("data/verapdf/ua1-pass.json");
const UA1_FAIL: &str = include_str!("data/verapdf/ua1-figure-without-alt.json");

fn validation(json: &str) -> Validation {
    Validation {
        ua1: parse(json, Profile::Ua1).unwrap(),
        wcag: WcagOutcome::NotAvailable,
    }
}

type ExportFn = Box<dyn Fn(&Path, &Path) -> Result<ExportResult, ExportError>>;
type ValidateFn = Box<dyn Fn(&Path) -> Result<Validation, VeraPdfError>>;

/// Stages that do what the test says, and count how often they were called.
struct Fake {
    tools: Box<dyn Fn() -> Result<(), VeraPdfError>>,
    export: ExportFn,
    validate: ValidateFn,
    calls: Cell<[u32; 3]>,
}

impl Fake {
    /// Word writes a tagged PDF with all its metadata, veraPDF passes.
    fn passing() -> Fake {
        Fake::exporting(|| Pdf::complete().build())
    }

    fn exporting(pdf: impl Fn() -> Vec<u8> + 'static) -> Fake {
        Fake {
            tools: Box::new(|| Ok(())),
            export: Box::new(move |_, path| {
                fs::write(path, pdf()).unwrap();
                Ok(result(path))
            }),
            validate: Box::new(|_| Ok(validation(UA1_PASS))),
            calls: Cell::new([0; 3]),
        }
    }

    fn word_fails(error: impl Fn() -> ExportError + 'static) -> Fake {
        let mut fake = Fake::passing();
        fake.export = Box::new(move |_, _| Err(error()));
        fake
    }

    fn calls(&self) -> [u32; 3] {
        self.calls.get()
    }

    fn count(&self, stage: usize) {
        let mut calls = self.calls.get();
        calls[stage] += 1;
        self.calls.set(calls);
    }
}

impl Stages for Fake {
    fn check_tools(&self) -> Result<(), VeraPdfError> {
        self.count(0);
        (self.tools)()
    }

    fn export(&self, docx: &Path, pdf: &Path) -> Result<ExportResult, ExportError> {
        self.count(1);
        (self.export)(docx, pdf)
    }

    fn validate(&self, pdf: &Path) -> Result<Validation, VeraPdfError> {
        self.count(2);
        (self.validate)(pdf)
    }
}

fn result(pdf: &Path) -> ExportResult {
    ExportResult {
        pdf: pdf.to_path_buf(),
        findings: Vec::new(),
        fields: FieldUpdate {
            fields: 2,
            tables_of_contents: 1,
            first_failed: None,
        },
        parameters: ExportParameters::SPEC,
        word_build: "16.0.fake".to_string(),
        duration: Duration::from_millis(1500),
    }
}

fn now() -> Timestamp {
    Timestamp::from_unix_seconds(1_772_705_730)
}

/// Runs the export of a .docx with the given bytes, in its own folder.
fn run_with(name: &str, docx: &[u8], stages: &Fake) -> (TempDir, PathBuf, Run) {
    let dir = TempDir::new("pipeline");
    let path = dir.write(name, docx);
    let run = export_docx(&path, stages, &FixtureFonts, now());
    (dir, path, run)
}

fn run(stages: &Fake) -> (TempDir, PathBuf, Run) {
    run_with("Guide.docx", &clean().build(), stages)
}

fn fixture(rule: RuleId, fails: bool) -> Vec<u8> {
    catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .expect("the fixture exists")
        .bytes
}

/// A tool failure: exit 2, a message that names the file, nothing on stdout,
/// no report and no change to files that were not written by the run.
fn assert_tool_failure(run: &Run, docx: &Path, expected: &str) {
    assert_eq!(run.exit, Exit::ToolFailure, "{}", run.stdout);
    assert_eq!(run.exit.code(), 2);
    assert!(run.stdout.is_empty(), "stdout: {}", run.stdout);
    assert!(run.report.is_none());
    assert!(run.files.is_empty());
    assert!(
        run.stderr.contains(&docx.display().to_string()),
        "stderr names the file: {}",
        run.stderr
    );
    assert!(
        run.stderr.contains(expected),
        "stderr should contain {expected:?}: {}",
        run.stderr
    );
    let (_dir, html, json) = output_paths(docx);
    assert!(!html.exists() && !json.exists(), "no report is written");
}

// C-05 (without Word), S6-09
#[test]
fn pipeline_exit_0_on_a_pass_and_writes_three_files() {
    let (dir, docx, run) = run(&Fake::passing());
    assert_eq!(run.exit, Exit::Pass);
    assert_eq!(run.exit.code(), 0);
    assert!(run.stderr.is_empty());
    assert_eq!(
        dir.names(),
        [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.docx",
            "Guide.pdf"
        ]
    );
    let (pdf, html, json) = output_paths(&docx);
    assert_eq!(run.files, [pdf, html.clone(), json.clone()]);
    let html = fs::read_to_string(html).unwrap();
    assert!(html.contains("Result: <strong>Pass</strong>"));
    let json: Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(json["summary"]["verdict"], "pass");
    assert_eq!(json["summary"]["document"], "Guide.docx");
    assert_eq!(json["summary"]["pdf"], "Guide.pdf");
    assert_eq!(json["summary"]["generated_local"], now().display());
    assert_eq!(json["export"]["word_build"], "16.0.fake");
    assert_eq!(json["export"]["fields_updated"], 2);
    assert_eq!(json["verapdf"]["verapdf_version"], "1.28.2");
    assert!(
        run.stdout
            .starts_with("Guide.docx: Pass (0 errors, 0 warnings).")
    );
    assert_eq!(run.report.unwrap().summary.verdict, Verdict::Pass);
}

// S6-09: the names follow the .docx name, also when it has dots and spaces.
#[test]
fn report_files_are_written_next_to_the_pdf() {
    let (dir, _, run) = run_with("My Guide v2.1.docx", &clean().build(), &Fake::passing());
    assert_eq!(run.exit, Exit::Pass);
    assert_eq!(
        dir.names(),
        [
            "My Guide v2.1.accessibility.html",
            "My Guide v2.1.accessibility.json",
            "My Guide v2.1.docx",
            "My Guide v2.1.pdf"
        ]
    );
}

// S4-02 to S4-05 inside the command, and the report of stage 4.
#[test]
fn pipeline_fixes_the_metadata_and_lists_the_changes() {
    let original = Pdf::all_missing().build();
    let expected = original.clone();
    let fake = Fake::exporting(move || original.clone());
    let (dir, docx, run) = run(&fake);
    let (pdf, _, _) = output_paths(&docx);
    let fixed = fs::read(&pdf).unwrap();
    assert!(fixed.starts_with(&expected) && fixed.len() > expected.len());
    assert_eq!(run.exit, Exit::Pass, "{}", run.stdout);
    let report = run.report.unwrap();
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    let fields: Vec<&str> = json["pdf_metadata"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["field"].as_str().unwrap())
        .collect();
    assert_eq!(
        fields,
        [
            "xmp-pdfuaid-part",
            "xmp-title",
            "info-title",
            "display-doc-title",
            "lang"
        ]
    );
    // The title and the language are those of the .docx.
    assert_eq!(
        json["pdf_metadata"]["changes"][1]["new"],
        "Adapdf test guide"
    );
    assert_eq!(json["pdf_metadata"]["changes"][4]["new"], "en-US");
    let html = fs::read_to_string(output_paths(&docx).1).unwrap();
    assert!(html.contains("Changes made to the PDF metadata"));
    assert_eq!(dir.names().len(), 4, "no temporary file stays");
}

// C-06, S3-13, S3-12, decided point 7: stage 1 errors give a Fail report and no PDF.
#[test]
fn pipeline_exit_1_for_stage_1_errors() {
    let fake = Fake::passing();
    let (dir, docx, run) = run_with("Guide.docx", &fixture(RuleId::Hdg002, true), &fake);
    assert_eq!(run.exit, Exit::ErrorsFound);
    assert_eq!(run.exit.code(), 1);
    assert_eq!(fake.calls(), [0, 0, 0], "no tool, Word or veraPDF is used");
    assert_eq!(
        dir.names(),
        [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.docx"
        ]
    );
    let html = fs::read_to_string(output_paths(&docx).1).unwrap();
    assert!(html.contains("Result: <strong>Fail</strong>"));
    assert!(html.contains("HDG-002"));
    assert!(run.stdout.starts_with("Guide.docx: Fail"), "{}", run.stdout);
    assert!(run.stdout.contains("[HDG-002]"));
    assert!(run.stderr.is_empty());
}

// S3-13: every error rule, and the report says why. The fakes are never called.
#[test]
fn pipeline_writes_no_pdf_when_stage_1_has_errors() {
    let errors: Vec<_> = catalogue()
        .into_iter()
        .filter(|f| f.fails && f.rule.severity() == Severity::Error)
        .collect();
    assert_eq!(errors.len(), 15);
    for fixture in errors {
        let fake = Fake::passing();
        let (dir, docx, run) = run_with("Guide.docx", &fixture.bytes, &fake);
        let name = fixture.file_name();
        assert_eq!(run.exit, Exit::ErrorsFound, "{name}");
        assert_eq!(fake.calls(), [0, 0, 0], "{name}");
        assert!(!dir.names().contains(&"Guide.pdf".to_string()), "{name}");
        let json: Value =
            serde_json::from_str(&fs::read_to_string(output_paths(&docx).2).unwrap()).unwrap();
        assert_eq!(json["summary"]["verdict"], "fail", "{name}");
        assert_eq!(json["summary"]["pdf"], Value::Null, "{name}");
        let rules: Vec<&str> = json["precheck"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["rule_id"].as_str().unwrap())
            .collect();
        assert!(rules.contains(&fixture.rule.as_str()), "{name}: {rules:?}");
        assert_eq!(json["pdf_metadata"]["status"], "not-run", "{name}");
        assert_eq!(json["verapdf"]["status"], "not-run", "{name}");
    }
}

// S3-12: DOC-005 (tracked changes) blocks, with its own message in the report.
#[test]
fn pipeline_refuses_doc_005_with_a_fail_report() {
    let fake = Fake::passing();
    let (_dir, docx, run) = run_with("Guide.docx", &fixture(RuleId::Doc005, true), &fake);
    assert_eq!(run.exit, Exit::ErrorsFound);
    assert_eq!(fake.calls(), [0, 0, 0]);
    let html = fs::read_to_string(output_paths(&docx).1).unwrap();
    assert!(html.contains("DOC-005"));
    assert!(html.to_lowercase().contains("tracked changes"));
}

// A stage 1 error is reported even when veraPDF is missing: the tools are
// needed only when there is a PDF to check.
#[test]
fn a_blocked_export_does_not_need_the_tools() {
    let mut fake = Fake::passing();
    fake.tools = Box::new(|| Err(VeraPdfError::JavaMissing("vendor/jre/bin/java.exe".into())));
    let (_dir, _, run) = run_with("Guide.docx", &fixture(RuleId::Hdg002, true), &fake);
    assert_eq!(run.exit, Exit::ErrorsFound);
}

// The export itself found an error that the first check did not (it runs stage
// 1 again): the same Fail report.
#[test]
fn a_blocked_answer_from_the_export_gives_the_fail_report() {
    let blocked = fixture(RuleId::Hdg002, true);
    let findings = adapdf::check_bytes(&blocked, &FixtureFonts).unwrap();
    let fake = Fake::word_fails(move || ExportError::Blocked {
        findings: findings.clone(),
    });
    let (dir, _, run) = run(&fake);
    assert_eq!(run.exit, Exit::ErrorsFound);
    assert!(
        dir.names()
            .contains(&"Guide.accessibility.html".to_string())
    );
    assert!(!dir.names().contains(&"Guide.pdf".to_string()));
}

// S3-14: warnings do not block. They are in the report and the exit code is 0.
#[test]
fn pipeline_runs_when_only_warnings_exist() {
    let warnings = adapdf::check_bytes(&fixture(RuleId::Txt002, true), &FixtureFonts).unwrap();
    assert!(warnings.iter().all(|f| f.severity == Severity::Warning));
    let docx = fixture(RuleId::Txt002, true);
    let fake = Fake {
        export: Box::new(move |_, path| {
            fs::write(path, Pdf::complete().build()).unwrap();
            let mut result = result(path);
            result.findings = warnings.clone();
            Ok(result)
        }),
        ..Fake::passing()
    };
    let (dir, docx_path, run) = run_with("Guide.docx", &docx, &fake);
    assert_eq!(run.exit, Exit::Pass, "{}", run.stdout);
    assert!(dir.names().contains(&"Guide.pdf".to_string()));
    let report = run.report.unwrap();
    assert_eq!((report.summary.errors, report.summary.warnings), (0, 1));
    let html = fs::read_to_string(output_paths(&docx_path).1).unwrap();
    assert!(html.contains("TXT-002"));
}

// C-06, S4-06, decided point 8: a PDF with no tags is exit 1.
#[test]
fn pipeline_exit_1_for_an_untagged_pdf() {
    for (name, pdf) in [
        (
            "no-markinfo",
            Pdf {
                marked: false,
                ..Pdf::complete()
            },
        ),
        (
            "no-structtreeroot",
            Pdf {
                struct_tree: false,
                ..Pdf::complete()
            },
        ),
    ] {
        let bytes = pdf.build();
        let kept = bytes.clone();
        let fake = Fake::exporting(move || bytes.clone());
        let (_dir, docx, run) = run(&fake);
        let (pdf_path, html, _) = output_paths(&docx);
        assert_eq!(run.exit, Exit::ErrorsFound, "{name}: {}", run.stdout);
        assert_eq!(run.exit.code(), 1);
        // Nothing was added to the file (spec 02: the tags are not fixed), and
        // the file stays where Word wrote it.
        assert_eq!(fs::read(pdf_path).unwrap(), kept, "{name}");
        let report = run.report.unwrap();
        assert_eq!(report.summary.verdict, Verdict::Fail, "{name}");
        assert!(report.summary.errors >= 1);
        let html = fs::read_to_string(html).unwrap();
        assert!(
            html.contains("Problems that the tool does not fix"),
            "{name}"
        );
        assert!(run.stdout.contains("[PDF]"), "{name}: {}", run.stdout);
    }
}

// C-06, S5-08: any veraPDF failure is exit 1; the rule is in the report.
#[test]
fn pipeline_exit_1_on_a_verapdf_failure() {
    let mut fake = Fake::passing();
    fake.validate = Box::new(|_| Ok(validation(UA1_FAIL)));
    let (_dir, docx, run) = run(&fake);
    assert_eq!(run.exit, Exit::ErrorsFound);
    assert!(run.stdout.contains("[veraPDF 7.3-1]"), "{}", run.stdout);
    let json: Value =
        serde_json::from_str(&fs::read_to_string(output_paths(&docx).2).unwrap()).unwrap();
    assert_eq!(json["summary"]["verdict"], "fail");
    assert_eq!(json["summary"]["errors"], 1);
    assert_eq!(
        json["verapdf"]["validation"]["ua1"]["findings"][0]["pre_check"],
        "IMG-001"
    );
    // The PDF is kept, with the failing report next to it.
    assert!(output_paths(&docx).0.exists());
}

// S5-05 and S5-08: WCAG warnings are warnings and do not change a pass.
#[test]
fn pipeline_wcag_results_are_warnings_and_do_not_fail_the_run() {
    let mut fake = Fake::passing();
    fake.validate = Box::new(|_| {
        Ok(Validation {
            ua1: parse(UA1_PASS, Profile::Ua1).unwrap(),
            wcag: WcagOutcome::Ran(
                parse(
                    include_str!("data/verapdf/wcag-metadata-missing.json"),
                    Profile::Wcag,
                )
                .unwrap(),
            ),
        })
    });
    let (_dir, _, run) = run(&fake);
    assert_eq!(run.exit, Exit::Pass, "{}", run.stdout);
    let report = run.report.unwrap();
    assert_eq!(report.summary.errors, 0);
    assert!(report.summary.warnings >= 1);
}

// C-07 and S3-16: Word is not installed.
#[test]
fn pipeline_exit_2_when_word_is_missing() {
    let fake = Fake::word_fails(|| ExportError::Word(WordError::NotInstalled));
    let (_dir, docx, run) = run(&fake);
    assert_tool_failure(&run, &docx, "Microsoft Word is not installed");
    assert_eq!(fake.calls(), [1, 1, 0]);
}

// C-07 and S3-06: the timeout.
#[test]
fn pipeline_exit_2_on_a_timeout() {
    let fake = Fake::word_fails(|| ExportError::Timeout {
        limit: Duration::from_secs(300),
        pid: Some(4242),
        ended: true,
    });
    let (_dir, docx, run) = run(&fake);
    assert_tool_failure(&run, &docx, "did not finish the export within 5 minutes");
}

// C-07 and S3-04: Word cannot open the file.
#[test]
fn pipeline_exit_2_when_word_rejects_the_file() {
    let fake = Fake::word_fails(|| {
        ExportError::Word(WordError::Open {
            path: "Guide.docx".into(),
            reason: "The file appears to be corrupted".to_string(),
        })
    });
    let (_dir, docx, run) = run(&fake);
    assert_tool_failure(&run, &docx, "The file appears to be corrupted");
}

// C-07 and decided point 6: a locked PDF names the PDF.
#[test]
fn pipeline_exit_2_for_a_locked_pdf_names_the_pdf() {
    let fake = Fake::word_fails(|| ExportError::PdfLocked {
        file: "C:/g/Guide.pdf".into(),
        source: std::io::Error::other("in use"),
    });
    let (_dir, docx, run) = run(&fake);
    assert_tool_failure(&run, &docx, "C:/g/Guide.pdf");
}

// C-07 and S5-06: veraPDF or Java missing, found before Word starts.
#[test]
fn verapdf_missing_gives_exit_2() {
    for (name, error) in [
        (
            "java",
            VeraPdfError::JavaMissing("vendor/jre/bin/java.exe".into()),
        ),
        (
            "verapdf",
            VeraPdfError::VeraPdfMissing("vendor/verapdf".into()),
        ),
    ] {
        let mut fake = Fake::passing();
        fake.tools = Box::new(move || {
            Err(match &error {
                VeraPdfError::JavaMissing(p) => VeraPdfError::JavaMissing(p.clone()),
                VeraPdfError::VeraPdfMissing(p) => VeraPdfError::VeraPdfMissing(p.clone()),
                _ => unreachable!(),
            })
        });
        let (dir, docx, run) = run(&fake);
        assert_tool_failure(
            &run,
            &docx,
            if name == "java" {
                "java.exe"
            } else {
                "vendor/verapdf"
            },
        );
        assert_eq!(fake.calls(), [1, 0, 0], "Word is not started");
        assert!(!dir.names().contains(&"Guide.pdf".to_string()));
    }
}

// C-07 and S5-07: a veraPDF that crashes or gives no usable output is not a pass.
#[test]
fn verapdf_crash_or_bad_output_is_exit_2() {
    for error in [
        || VeraPdfError::Crashed {
            exit_code: Some(1),
            stderr: "boom".to_string(),
        },
        || VeraPdfError::TimedOut(180),
        || VeraPdfError::BadOutput("veraPDF wrote nothing".to_string()),
    ] {
        let mut fake = Fake::passing();
        fake.validate = Box::new(move |_| Err(error()));
        let (_dir, docx, run) = run(&fake);
        assert_tool_failure(&run, &docx, "The PDF could not be checked");
    }
}

// C-07 and S4-12: a PDF that stage 4 cannot read is exit 2.
#[test]
fn a_pdf_that_cannot_be_read_is_exit_2() {
    let fake = Fake::exporting(|| b"%PDF-1.4\nnot really a pdf\n%%EOF\n".to_vec());
    let (_dir, docx, run) = run(&fake);
    assert_tool_failure(&run, &docx, "The PDF metadata could not be checked");
    assert_eq!(fake.calls(), [1, 1, 0], "veraPDF is not run");
}

// C-07: the .docx cannot be read.
#[test]
fn an_unreadable_docx_is_exit_2() {
    let (_dir, docx, run) = run_with("Guide.docx", b"plain text, not a zip", &Fake::passing());
    assert_tool_failure(&run, &docx, "not a .docx");
    let dir = TempDir::new("missing");
    let missing = dir.path().join("Missing.docx");
    let run = export_docx(&missing, &Fake::passing(), &FixtureFonts, now());
    assert_tool_failure(&run, &missing, "cannot be read");
}

// C-07: the report cannot be written (a folder has its name).
#[test]
fn a_report_that_cannot_be_written_is_exit_2() {
    let dir = TempDir::new("report-blocked");
    let docx = dir.write("Guide.docx", &clean().build());
    fs::create_dir(dir.path().join("Guide.accessibility.html")).unwrap();
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::ToolFailure);
    assert!(run.stdout.is_empty());
    assert!(
        run.stderr.contains("Guide.accessibility.html"),
        "{}",
        run.stderr
    );
}

// Section 3 of the report: the export applies no fix, and it names the newest
// backup that an earlier `adapdf fix` left next to the file.
#[test]
fn report_names_the_newest_backup_next_to_the_document() {
    let dir = TempDir::new("backup");
    let docx = dir.write("Guide.docx", &clean().build());
    dir.write("Guide.backup-20260301-090000.docx", b"old");
    dir.write("Guide.backup-20260304-101530.docx", b"newer");
    dir.write("Other.backup-20270101-000000.docx", b"other guide");
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::Pass);
    let report = run.report.unwrap();
    assert!(report.fixes.applied.is_empty());
    assert_eq!(
        report.fixes.backup.as_deref(),
        Some("Guide.backup-20260304-101530.docx")
    );
    let html = fs::read_to_string(output_paths(&docx).1).unwrap();
    assert!(html.contains("Guide.backup-20260304-101530.docx"));
    assert!(html.contains("No fixes were applied in this run."));
}
