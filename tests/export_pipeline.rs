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

use std::cell::{Cell, RefCell};
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
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

/// The folder of a guide, `Guide - Adapdf`, inside a folder of its own for one
/// test. `path`, `write` and `names` work on the guide folder, where the
/// accessible copy `accessible.docx` is.
struct GuideDir {
    _root: TempDir,
    folder: PathBuf,
}

impl GuideDir {
    fn new(name: &str) -> GuideDir {
        let root = TempDir::new(name);
        let folder = root.path().join("Guide - Adapdf");
        fs::create_dir(&folder).unwrap();
        GuideDir {
            _root: root,
            folder,
        }
    }

    fn path(&self) -> &Path {
        &self.folder
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.folder.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn names(&self) -> Vec<String> {
        names_in(&self.folder)
    }
}

/// Runs the export of a .docx with the given bytes, in its own folder.
fn run_with(name: &str, docx: &[u8], stages: &Fake) -> (GuideDir, PathBuf, Run) {
    let dir = GuideDir::new("pipeline");
    let path = dir.write(name, docx);
    let run = export_docx(&path, stages, &FixtureFonts, now());
    (dir, path, run)
}

fn run(stages: &Fake) -> (GuideDir, PathBuf, Run) {
    run_with("accessible.docx", &clean().build(), stages)
}

fn fixture(rule: RuleId, fails: bool) -> Vec<u8> {
    catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .expect("the fixture exists")
        .bytes
}

/// What an earlier run left next to the .docx. A new run must replace these
/// only when it has a verdict, so a tool failure must leave them as they are.
const OLD_PDF: &[u8] = b"%PDF-1.4 the PDF of an earlier run";
const OLD_HTML: &[u8] = b"<html>the report of an earlier run</html>";
const OLD_JSON: &[u8] = b"{\"the\": \"report of an earlier run\"}";

/// Runs the export in a folder that already has an earlier `Guide.pdf` and
/// earlier reports.
fn run_over_earlier_outputs(stages: &Fake) -> (GuideDir, PathBuf, Run) {
    let dir = GuideDir::new("pipeline-earlier");
    let docx = dir.write("accessible.docx", &clean().build());
    dir.write("Guide.pdf", OLD_PDF);
    dir.write("report.html", OLD_HTML);
    dir.write("report.json", OLD_JSON);
    let run = export_docx(&docx, stages, &FixtureFonts, now());
    (dir, docx, run)
}

/// The folder is exactly as `run_over_earlier_outputs` made it: the same four
/// files with the same bytes, and no temporary file.
fn assert_earlier_outputs_untouched(dir: &GuideDir) {
    assert_eq!(
        dir.names(),
        ["Guide.pdf", "accessible.docx", "report.html", "report.json",]
    );
    let read = |name: &str| fs::read(dir.path().join(name)).unwrap();
    assert_eq!(read("Guide.pdf"), OLD_PDF);
    assert_eq!(read("report.html"), OLD_HTML);
    assert_eq!(read("report.json"), OLD_JSON);
}

/// A tool failure: exit 2, a message that names the file, nothing on stdout,
/// no report and no change to files that were not written by the run.
fn assert_tool_failure(run: &Run, docx: &Path, expected: &str) {
    assert_failed_run(run, docx, expected);
    let (_dir, html, json) = output_paths(docx);
    assert!(!html.exists() && !json.exists(), "no report is written");
}

/// The exit code, the empty stdout, the message and no report in the result.
fn assert_failed_run(run: &Run, docx: &Path, expected: &str) {
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
        ["Guide.pdf", "accessible.docx", "report.html", "report.json",]
    );
    let (pdf, html, json) = output_paths(&docx);
    assert_eq!(run.files, [pdf, html.clone(), json.clone()]);
    let html = fs::read_to_string(html).unwrap();
    assert!(html.contains("Result: <strong>Pass</strong>"));
    let json: Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(json["summary"]["verdict"], "pass");
    assert_eq!(json["summary"]["document"], "accessible.docx");
    assert_eq!(json["summary"]["pdf"], "Guide.pdf");
    assert_eq!(json["summary"]["generated_local"], now().display());
    assert_eq!(json["export"]["word_build"], "16.0.fake");
    assert_eq!(json["export"]["fields_updated"], 2);
    assert_eq!(json["verapdf"]["verapdf_version"], "1.28.2");
    assert!(
        run.stdout
            .starts_with("accessible.docx: Pass (0 errors, 0 warnings).")
    );
    assert_eq!(run.report.unwrap().summary.verdict, Verdict::Pass);
}

// S6-09: the PDF is named after the original, also when the name has dots and
// spaces. The copy is `accessible.docx` in the folder `My Guide v2.1 - Adapdf`;
// the reports have fixed names.
#[test]
fn the_pdf_is_named_after_the_original_and_the_reports_have_fixed_names() {
    let root = TempDir::new("pipeline-odd-name");
    let folder = root.path().join("My Guide v2.1 - Adapdf");
    fs::create_dir(&folder).unwrap();
    let docx = folder.join("accessible.docx");
    fs::write(&docx, clean().build()).unwrap();
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::Pass);
    assert_eq!(
        names_in(&folder),
        [
            "My Guide v2.1.pdf",
            "accessible.docx",
            "report.html",
            "report.json",
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
    let (dir, docx, run) = run_with("accessible.docx", &fixture(RuleId::Hdg002, true), &fake);
    assert_eq!(run.exit, Exit::ErrorsFound);
    assert_eq!(run.exit.code(), 1);
    assert_eq!(fake.calls(), [0, 0, 0], "no tool, Word or veraPDF is used");
    assert_eq!(
        dir.names(),
        ["accessible.docx", "report.html", "report.json",]
    );
    let html = fs::read_to_string(output_paths(&docx).1).unwrap();
    assert!(html.contains("Result: <strong>Fail</strong>"));
    assert!(html.contains("HDG-002"));
    assert!(
        run.stdout.starts_with("accessible.docx: Fail"),
        "{}",
        run.stdout
    );
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
    assert_eq!(errors.len(), 18);
    for fixture in errors {
        let fake = Fake::passing();
        let (dir, docx, run) = run_with("accessible.docx", &fixture.bytes, &fake);
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
    let (_dir, docx, run) = run_with("accessible.docx", &fixture(RuleId::Doc005, true), &fake);
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
    let (_dir, _, run) = run_with("accessible.docx", &fixture(RuleId::Hdg002, true), &fake);
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
    assert!(dir.names().contains(&"report.html".to_string()));
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
    let (dir, docx_path, run) = run_with("accessible.docx", &docx, &fake);
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
        // An untagged PDF has a verdict (Fail), so it replaces the old PDF.
        let (dir, docx, run) = run_over_earlier_outputs(&fake);
        let (pdf_path, html, _) = output_paths(&docx);
        assert_eq!(run.exit, Exit::ErrorsFound, "{name}: {}", run.stdout);
        assert_eq!(run.exit.code(), 1);
        // Nothing was added to the file (spec 02: the tags are not fixed), and
        // the file stays where Word wrote it.
        assert_eq!(fs::read(pdf_path).unwrap(), kept, "{name}");
        assert_eq!(dir.names().len(), 4, "{name}: no temporary file stays");
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

// C-06 and S5-08: a veraPDF failure is a verdict (Fail), not a tool failure, so
// the new PDF replaces the old one and the Fail report replaces the old reports.
#[test]
fn a_verapdf_failure_replaces_the_old_pdf_and_the_old_reports() {
    let new_pdf = Pdf::complete().build();
    let expected = new_pdf.clone();
    let mut fake = Fake::exporting(move || new_pdf.clone());
    fake.validate = Box::new(|_| Ok(validation(UA1_FAIL)));
    let (dir, docx, run) = run_over_earlier_outputs(&fake);
    assert_eq!(run.exit, Exit::ErrorsFound, "{}", run.stdout);
    assert_eq!(
        dir.names(),
        ["Guide.pdf", "accessible.docx", "report.html", "report.json",],
        "no temporary file stays"
    );
    let (pdf, html, json) = output_paths(&docx);
    assert_eq!(fs::read(pdf).unwrap(), expected, "the new PDF is in place");
    assert!(
        fs::read_to_string(html)
            .unwrap()
            .contains("Result: <strong>Fail</strong>")
    );
    let json: Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(json["summary"]["verdict"], "fail");
    assert_eq!(json["summary"]["pdf"], "Guide.pdf");
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
    let (dir, docx, run) = run_over_earlier_outputs(&fake);
    assert_failed_run(&run, &docx, "Microsoft Word is not installed");
    assert_eq!(fake.calls(), [1, 1, 0]);
    assert_earlier_outputs_untouched(&dir);
}

// C-07 and S3-06: the timeout.
#[test]
fn pipeline_exit_2_on_a_timeout() {
    let fake = Fake::word_fails(|| ExportError::Timeout {
        limit: Duration::from_secs(300),
        pid: Some(4242),
        ended: true,
    });
    let (dir, docx, run) = run_over_earlier_outputs(&fake);
    assert_failed_run(&run, &docx, "did not finish the export within 5 minutes");
    assert_earlier_outputs_untouched(&dir);
}

// C-07 and S3-04: Word cannot open the file.
#[test]
fn pipeline_exit_2_when_word_rejects_the_file() {
    let fake = Fake::word_fails(|| {
        ExportError::Word(WordError::Open {
            path: "accessible.docx".into(),
            reason: "The file appears to be corrupted".to_string(),
        })
    });
    let (dir, docx, run) = run_over_earlier_outputs(&fake);
    assert_failed_run(&run, &docx, "The file appears to be corrupted");
    assert_earlier_outputs_untouched(&dir);
}

// C-07 and decided point 6: a locked PDF names the PDF.
#[test]
fn pipeline_exit_2_for_a_locked_pdf_names_the_pdf() {
    let fake = Fake::word_fails(|| ExportError::PdfLocked {
        file: "C:/g/Guide.pdf".into(),
        source: std::io::Error::other("in use"),
    });
    let (dir, docx, run) = run_over_earlier_outputs(&fake);
    assert_failed_run(&run, &docx, "C:/g/Guide.pdf");
    assert_earlier_outputs_untouched(&dir);
}

// C-07 and decided point 6: another program holds the old `Guide.pdf` before the
// run. The pipeline finds it before Word starts, and nothing changes.
#[test]
fn a_locked_pdf_is_found_before_word_starts() {
    let dir = GuideDir::new("pipeline-locked");
    let docx = dir.write("accessible.docx", &clean().build());
    let old = dir.write("Guide.pdf", OLD_PDF);
    dir.write("report.html", OLD_HTML);
    dir.write("report.json", OLD_JSON);
    let held = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&old)
        .unwrap();
    let fake = Fake::passing();
    let run = export_docx(&docx, &fake, &FixtureFonts, now());
    drop(held);
    assert_failed_run(&run, &docx, "Guide.pdf");
    assert!(run.stderr.contains("cannot be replaced"), "{}", run.stderr);
    assert_eq!(fake.calls(), [1, 0, 0], "Word is not started");
    assert_earlier_outputs_untouched(&dir);
}

// C-07 and decided point 6: the lock comes after the check of the PDF, while
// the new PDF is ready. The old PDF stays, the new one is thrown away, and the
// old reports are not replaced.
#[test]
fn a_pdf_that_is_locked_at_the_end_changes_nothing() {
    let dir = GuideDir::new("pipeline-locked-late");
    let docx = dir.write("accessible.docx", &clean().build());
    let old = dir.write("Guide.pdf", OLD_PDF);
    dir.write("report.html", OLD_HTML);
    dir.write("report.json", OLD_JSON);
    // Another program opens `Guide.pdf` while veraPDF works.
    let held = Rc::new(RefCell::new(None));
    let mut fake = Fake::passing();
    fake.validate = Box::new({
        let held = Rc::clone(&held);
        let old = old.clone();
        move |_| {
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&old)
                .unwrap();
            *held.borrow_mut() = Some(file);
            Ok(validation(UA1_PASS))
        }
    });
    let run = export_docx(&docx, &fake, &FixtureFonts, now());
    drop(held.borrow_mut().take());
    assert_failed_run(&run, &docx, "Guide.pdf");
    assert_eq!(fake.calls(), [1, 1, 1]);
    assert_earlier_outputs_untouched(&dir);
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
        let (dir, docx, run) = run_over_earlier_outputs(&fake);
        assert_failed_run(
            &run,
            &docx,
            if name == "java" {
                "java.exe"
            } else {
                "vendor/verapdf"
            },
        );
        assert_eq!(fake.calls(), [1, 0, 0], "Word is not started");
        assert_earlier_outputs_untouched(&dir);
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
        let (dir, docx, run) = run_over_earlier_outputs(&fake);
        assert_failed_run(&run, &docx, "The PDF could not be checked");
        assert_eq!(fake.calls(), [1, 1, 1]);
        // The new PDF was made, and it is thrown away: it has no verdict.
        assert_earlier_outputs_untouched(&dir);
    }
}

// C-07 and S4-12: a PDF that stage 4 cannot read is exit 2.
#[test]
fn a_pdf_that_cannot_be_read_is_exit_2() {
    let fake = Fake::exporting(|| b"%PDF-1.4\nnot really a pdf\n%%EOF\n".to_vec());
    let (dir, docx, run) = run_over_earlier_outputs(&fake);
    assert_failed_run(&run, &docx, "The PDF metadata could not be checked");
    assert_eq!(fake.calls(), [1, 1, 0], "veraPDF is not run");
    assert_earlier_outputs_untouched(&dir);
}

// C-07: the .docx cannot be read.
#[test]
fn an_unreadable_docx_is_exit_2() {
    let (_dir, docx, run) = run_with(
        "accessible.docx",
        b"plain text, not a zip",
        &Fake::passing(),
    );
    assert_tool_failure(&run, &docx, "not a .docx");
    let dir = TempDir::new("missing");
    let missing = dir.path().join("Missing.docx");
    let run = export_docx(&missing, &Fake::passing(), &FixtureFonts, now());
    assert_tool_failure(&run, &missing, "cannot be read");
}

// C-07, C-11: the reports are written, but the name of the HTML report is a
// folder, so it cannot be replaced. This comes after the PDF was moved (rare):
// the new PDF is in place, the message says so and names the report, and no
// temporary file stays. The JSON of the earlier run is not replaced either,
// because the HTML report comes first.
#[test]
fn a_report_that_cannot_be_replaced_leaves_the_new_pdf_and_says_so() {
    let dir = GuideDir::new("report-not-replaced");
    let docx = dir.write("accessible.docx", &clean().build());
    dir.write("Guide.pdf", OLD_PDF);
    dir.write("report.json", OLD_JSON);
    fs::create_dir(dir.path().join("report.html")).unwrap();
    let expected = Pdf::complete().build();
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_failed_run(&run, &docx, "report.html");
    assert!(run.stderr.contains("cannot be replaced"), "{}", run.stderr);
    assert!(
        run.stderr.contains("new PDF") && run.stderr.contains("already in place"),
        "{}",
        run.stderr
    );
    assert_eq!(
        dir.names(),
        ["Guide.pdf", "accessible.docx", "report.html", "report.json",],
        "no temporary file"
    );
    assert!(dir.path().join("report.html").is_dir());
    assert_eq!(fs::read(dir.path().join("Guide.pdf")).unwrap(), expected);
    assert_eq!(fs::read(dir.path().join("report.json")).unwrap(), OLD_JSON);
}

// Decided point 7 with the temporary files: a blocked export writes its Fail
// report the same way, leaves no temporary file, and leaves an older PDF alone.
#[test]
fn a_blocked_export_writes_its_reports_without_leaving_temporary_files() {
    let dir = GuideDir::new("blocked-temp");
    let docx = dir.write("accessible.docx", &fixture(RuleId::Hdg002, true));
    dir.write("Guide.pdf", OLD_PDF);
    dir.write("report.html", OLD_HTML);
    dir.write("report.json", OLD_JSON);
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::ErrorsFound);
    assert_eq!(
        dir.names(),
        ["Guide.pdf", "accessible.docx", "report.html", "report.json",]
    );
    let read = |name: &str| fs::read(dir.path().join(name)).unwrap();
    assert_eq!(read("Guide.pdf"), OLD_PDF, "an older PDF stays");
    assert_ne!(read("report.html"), OLD_HTML);
    assert!(
        String::from_utf8(read("report.json"))
            .unwrap()
            .contains("\"verdict\": \"fail\"")
    );
}

// ------------------------------------------------------- the folder of the guide

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn entry(n: u32, text: Option<&str>) -> adapdf::fix::FixLogEntry {
    adapdf::fix::FixLogEntry {
        time: format!("2026-03-0{n} 10:15:30"),
        rule: RuleId::Doc001,
        fix: adapdf::findings::FixId::SetTitle,
        place: "The document's properties (File > Info)".to_string(),
        summary: format!("Set the title {n}."),
        text: text.map(str::to_string),
    }
}

// S6-12: the export of an original writes only into `Guide - Adapdf`, which it
// makes. Nothing else is written next to the original, and the original is as it
// was.
#[test]
fn the_export_of_an_original_writes_only_into_its_adapdf_folder() {
    let bytes = clean().build();
    let dir = TempDir::new("original-layout");
    let docx = dir.write("Guide.docx", &bytes);
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::Pass, "{}", run.stderr);
    assert_eq!(dir.names(), ["Guide - Adapdf", "Guide.docx"]);
    let folder = dir.path().join("Guide - Adapdf");
    assert_eq!(
        names_in(&folder),
        ["Guide.pdf", "report.html", "report.json"]
    );
    let (pdf, html, json) = output_paths(&docx);
    assert_eq!(pdf, folder.join("Guide.pdf"));
    assert_eq!(html, folder.join("report.html"));
    assert_eq!(json, folder.join("report.json"));
    assert_eq!(fs::read(&docx).unwrap(), bytes);
    let report = run.report.unwrap();
    assert_eq!(report.fixes.original, None);
    assert!(report.fixes.applied.is_empty());
    let page = fs::read_to_string(html).unwrap();
    assert!(page.contains("No fixes were made by Adapdf. The Word document is Guide.docx."));
    assert!(!page.to_lowercase().contains("backup"));
    // The files that the run says it wrote are those in the folder.
    assert_eq!(run.files.len(), 3);
    assert!(run.files.iter().all(|f| f.starts_with(&folder)));
}

// S6-12: the accessible copy in its folder gives `Guide.pdf` and the reports in
// that folder, named after the original, and the report lists every fix of the
// log, also those of an earlier day.
#[test]
fn the_export_of_the_copy_names_its_files_after_the_original_and_lists_the_fixes() {
    let dir = TempDir::new("copy-layout");
    let folder = dir.path().join("Guide - Adapdf");
    fs::create_dir_all(&folder).unwrap();
    let logged = adapdf::fix::fixlog::append(
        &clean().build(),
        &[entry(1, Some("A title")), entry(2, None)],
    )
    .unwrap();
    let copy = folder.join("accessible.docx");
    fs::write(&copy, &logged).unwrap();
    let original = dir.write("Guide.docx", &clean().build());
    let run = export_docx(&copy, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::Pass, "{}", run.stderr);
    assert_eq!(
        names_in(&folder),
        ["Guide.pdf", "accessible.docx", "report.html", "report.json"]
    );
    assert_eq!(dir.names(), ["Guide - Adapdf", "Guide.docx"]);
    assert_eq!(fs::read(&original).unwrap(), clean().build());
    assert_eq!(
        fs::read(&copy).unwrap(),
        logged,
        "the export does not change the copy"
    );
    let report = run.report.unwrap();
    assert_eq!(report.summary.document, "accessible.docx");
    assert_eq!(report.summary.pdf.as_deref(), Some("Guide.pdf"));
    assert_eq!(report.fixes.original.as_deref(), Some("Guide.docx"));
    assert_eq!(report.fixes.applied.len(), 2);
    let json: Value =
        serde_json::from_str(&fs::read_to_string(folder.join("report.json")).unwrap()).unwrap();
    assert_eq!(json["fixes"]["original"], "Guide.docx");
    assert_eq!(json["fixes"]["applied"][0]["text"], "A title");
    assert_eq!(json["fixes"]["applied"][1]["summary"], "Set the title 2.");
    let page = fs::read_to_string(folder.join("report.html")).unwrap();
    for part in [
        "The original Guide.docx was not changed. Adapdf made its fixes in accessible.docx in the folder Guide - Adapdf.",
        "2026-03-01 10:15:30",
        "Set the title 2.",
        "A title",
    ] {
        assert!(page.contains(part), "{part}");
    }
    assert!(!page.contains("No fixes were made by Adapdf"));
}

// C-11 for the new folder: a run that fails leaves no new folder, and a run that
// was blocked by errors still writes its report into the folder.
#[test]
fn a_failed_run_leaves_no_new_folder_and_a_blocked_run_writes_its_report() {
    let dir = TempDir::new("folder-failure");
    let docx = dir.write("Guide.docx", &clean().build());
    let fake = Fake::word_fails(|| ExportError::Word(WordError::NotInstalled));
    let run = export_docx(&docx, &fake, &FixtureFonts, now());
    assert_eq!(run.exit, Exit::ToolFailure);
    assert_eq!(
        dir.names(),
        ["Guide.docx"],
        "the folder was not left behind"
    );

    let dir = TempDir::new("folder-blocked");
    let docx = dir.write("Guide.docx", &fixture(RuleId::Hdg002, true));
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::ErrorsFound);
    assert_eq!(dir.names(), ["Guide - Adapdf", "Guide.docx"]);
    assert_eq!(
        names_in(&dir.path().join("Guide - Adapdf")),
        ["report.html", "report.json"],
        "a report and no PDF"
    );
}

// The folder of a guide can be so deep that the full path of the PDF is over 260
// characters. The Rust file calls of the export (the copy, the temporary files,
// the moves, the reports) work with such a path. The fake Word does not need a
// short path, but the PDF and the reports must land in the long folder.
#[test]
fn the_export_works_in_a_folder_with_a_path_over_260_characters() {
    let root = TempDir::new("long-path");
    let mut folder = root.path().to_path_buf();
    while folder.as_os_str().len() < 300 {
        folder.push("a".repeat(60));
    }
    let guide = folder.join("Guide - Adapdf");
    fs::create_dir_all(&guide).unwrap();
    let docx = guide.join("accessible.docx");
    fs::write(&docx, clean().build()).unwrap();
    assert!(docx.as_os_str().len() > 300);
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::Pass, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        names_in(&guide),
        ["Guide.pdf", "accessible.docx", "report.html", "report.json"]
    );
    assert!(run.files.iter().all(|f| f.starts_with(&guide)));
}

// A fix log that is not a log of this tool stops the run before anything is written.
#[test]
fn a_fix_log_that_cannot_be_read_is_exit_2() {
    let dir = TempDir::new("bad-log");
    let base = clean().build();
    let package = adapdf::docx::Package::from_bytes(&base).unwrap();
    let broken_custom = r#"<?xml version="1.0"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes"><property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid="2" name="Adapdf fix log"><vt:lpwstr>not json</vt:lpwstr></property></Properties>"#;
    let mut entries: Vec<(&str, &[u8])> = package
        .parts()
        .iter()
        .map(|p| (p.name.as_str(), p.bytes.as_slice()))
        .collect();
    entries.push(("docProps/custom.xml", broken_custom.as_bytes()));
    let docx = dir.write("Guide.docx", &adapdf::fixtures::zip_stored(&entries));
    let run = export_docx(&docx, &Fake::passing(), &FixtureFonts, now());
    assert_eq!(run.exit, Exit::ToolFailure);
    assert!(run.stderr.contains("fix log"), "{}", run.stderr);
    assert!(run.stdout.is_empty());
    assert_eq!(dir.names(), ["Guide.docx"]);
}

// A failure of veraPDF has its place in words in the report, and the raw path of
// veraPDF is kept apart.
#[test]
fn a_verapdf_failure_has_its_place_in_words_and_keeps_the_raw_path() {
    let mut fake = Fake::passing();
    fake.validate = Box::new(|_| Ok(validation(UA1_FAIL)));
    let (_dir, _, run) = run(&fake);
    assert_eq!(run.exit, Exit::ErrorsFound);
    let report = run.report.unwrap();
    let adapdf::report::VeraPdfSection::Ran { validation, .. } = &report.verapdf else {
        panic!("veraPDF ran");
    };
    let failure = &validation.ua1.findings[0];
    assert!(failure.context.as_deref().unwrap().starts_with("root/"));
    let place = failure.place.as_deref().expect("a place in words");
    assert!(place.to_lowercase().contains("figure"), "{place}");
}
