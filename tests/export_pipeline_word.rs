//! The exit codes of `adapdf export` with the real Word (the export halves of
//! test plan rows S3-04, S3-06, S3-14 and S3-19). veraPDF is not needed here:
//! the stage that checks the tools and the stage that runs veraPDF are stand-ins
//! (`WordOnly`), so these tests run with the feature `windows_word` alone. The
//! real veraPDF is in `tests/export_verapdf.rs`.
//!
//! Run them with `cargo test --features windows_word --test export_pipeline_word`.
//! The tool starts its own hidden Word; a Word that a person has open is never
//! touched, and the `WINWORD.EXE` processes are the same before and after.
#![cfg(all(windows, feature = "windows_word"))]

mod export_support;
mod word_support;

use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::time::Duration;

use adapdf::export::{ExportError, ExportOptions, ExportResult, export_pdf};
use adapdf::findings::RuleId;
use adapdf::fix::Timestamp;
use adapdf::fixtures::{FixtureFonts, catalogue, clean};
use adapdf::pipeline::{Exit, Stages, export_docx, output_paths};
use adapdf::verapdf::{Profile, Validation, VeraPdfError, WcagOutcome, parse};
use adapdf::word::WordSession;
use export_support::{names, without_content_types};
use word_support::{TempDir, exclusive, wait_for_word_pids, within, word_pids};

/// The real Word, with a veraPDF that always passes.
struct WordOnly(ExportOptions);

impl Stages for WordOnly {
    fn check_tools(&self) -> Result<(), VeraPdfError> {
        Ok(())
    }

    fn export(&self, docx: &Path, pdf: &Path) -> Result<ExportResult, ExportError> {
        export_pdf(docx, pdf, &FixtureFonts, &self.0)
    }

    fn validate(&self, _pdf: &Path) -> Result<Validation, VeraPdfError> {
        Ok(Validation {
            ua1: parse(include_str!("data/verapdf/ua1-pass.json"), Profile::Ua1).unwrap(),
            wcag: WcagOutcome::NotAvailable,
        })
    }
}

fn long() -> WordOnly {
    WordOnly(ExportOptions {
        timeout: Duration::from_secs(120),
        after_open: None,
    })
}

/// Writes the PDF and the reports of an earlier run next to `accessible.docx`.
fn write_earlier_outputs(dir: &TempDir) {
    dir.write("Guide.pdf", b"%PDF-1.4 an earlier export");
    dir.write("report.html", b"<html>an earlier report</html>");
    dir.write("report.json", b"{\"an\": \"earlier report\"}");
}

/// After a tool failure the folder is as `write_earlier_outputs` left it.
fn assert_earlier_outputs_untouched(dir: &TempDir) {
    assert_eq!(
        names(dir.path()),
        ["Guide.pdf", "accessible.docx", "report.html", "report.json"],
        "no new or temporary file"
    );
    let read = |name: &str| fs::read(dir.path().join(name)).unwrap();
    assert_eq!(read("Guide.pdf"), b"%PDF-1.4 an earlier export");
    assert_eq!(read("report.html"), b"<html>an earlier report</html>");
    assert_eq!(read("report.json"), b"{\"an\": \"earlier report\"}");
}

fn fixture(rule: RuleId, fails: bool) -> Vec<u8> {
    catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .unwrap()
        .bytes
}

// S3-04 (exit code): a .docx that stage 1 accepts and Word rejects is exit 2.
#[test]
fn pipeline_word_exit_2_when_word_cannot_open_the_file() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::guide("pipeline-word-open");
    let docx = dir.write("accessible.docx", &without_content_types(&clean().build()));
    write_earlier_outputs(&dir);
    let run = export_docx(&docx, &long(), &FixtureFonts, Timestamp::now());
    assert_eq!(run.exit, Exit::ToolFailure, "{}", run.stdout);
    assert!(run.stdout.is_empty());
    assert!(run.stderr.contains("accessible.docx"), "{}", run.stderr);
    assert!(run.stderr.contains("corrupted"), "{}", run.stderr);
    assert_earlier_outputs_untouched(&dir);
    assert_eq!(wait_for_word_pids(&before), before);
}

// S3-06 (exit code): a timeout is exit 2, and only the tool's own Word ends.
#[test]
fn pipeline_word_exit_2_on_a_timeout() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::guide("pipeline-word-timeout");
    let docx = dir.write("accessible.docx", &clean().build());
    write_earlier_outputs(&dir);
    let stages = WordOnly(ExportOptions {
        timeout: Duration::from_millis(1),
        after_open: None,
    });
    let run = export_docx(&docx, &stages, &FixtureFonts, Timestamp::now());
    assert_eq!(run.exit, Exit::ToolFailure);
    assert!(run.stdout.is_empty());
    assert!(
        run.stderr.contains("did not finish the export within"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr
            .contains("ended the copy of Word that it had started")
    );
    assert_earlier_outputs_untouched(&dir);
    assert_eq!(
        wait_for_word_pids(&before),
        before,
        "the person's Word is not ended"
    );
}

// S3-14 (the report lists the warnings): warnings do not block the export.
#[test]
fn pipeline_word_runs_when_only_warnings_exist() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("pipeline-word-warnings");
    for (rule, file) in [(RuleId::Hdg006, "Hdg.docx"), (RuleId::Txt002, "Txt.docx")] {
        let docx = dir.write(file, &fixture(rule, true));
        let run = export_docx(&docx, &long(), &FixtureFonts, Timestamp::now());
        assert_eq!(run.exit, Exit::Pass, "{rule}: {}", run.stderr);
        let (pdf, html, _) = output_paths(&docx);
        assert!(fs::read(pdf).unwrap().starts_with(b"%PDF-"), "{rule}");
        let report = run.report.unwrap();
        assert_eq!(
            (report.summary.errors, report.summary.warnings),
            (0, 1),
            "{rule}"
        );
        assert!(fs::read_to_string(html).unwrap().contains(rule.as_str()));
    }
    assert_eq!(wait_for_word_pids(&before), before);
}

// S3-19 (exit code, decided point 6): a PDF that another program holds is
// exit 2, names the PDF, and keeps it.
#[test]
fn pipeline_word_exit_2_for_a_locked_pdf_and_exit_0_for_one_to_replace() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::guide("pipeline-word-locked");
    let docx = dir.write("accessible.docx", &clean().build());
    let pdf = dir.write("Guide.pdf", b"%PDF-1.4 an earlier export");

    let holder = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&pdf)
        .unwrap();
    let run = export_docx(&docx, &long(), &FixtureFonts, Timestamp::now());
    assert_eq!(run.exit, Exit::ToolFailure, "{}", run.stdout);
    assert!(run.stdout.is_empty());
    assert!(run.stderr.contains("Guide.pdf"), "{}", run.stderr);
    drop(holder);
    assert_eq!(fs::read(&pdf).unwrap(), b"%PDF-1.4 an earlier export");
    assert_eq!(names(dir.path()), ["Guide.pdf", "accessible.docx"]);

    // Not locked any more: the old PDF is replaced.
    let run = export_docx(&docx, &long(), &FixtureFonts, Timestamp::now());
    assert_eq!(run.exit, Exit::Pass, "{}", run.stderr);
    assert!(fs::read(&pdf).unwrap().len() > 1000);
    assert_eq!(wait_for_word_pids(&before), before);
}

// S3-23: a guide in a deep folder. The full path of the accessible copy is over
// 260 characters, so Word cannot open it. The export gives Word a copy in a short
// temporary folder, and the PDF and the reports land in the long folder. The short
// folder is removed.
#[test]
fn pipeline_word_exports_a_guide_with_a_path_over_260_characters() {
    let _lock = exclusive();
    let before = word_pids();
    let temp_before = work_folders();
    let dir = TempDir::new("pipeline-word-long");
    let mut folder = dir.path().to_path_buf();
    while folder.as_os_str().len() < 300 {
        folder.push("a".repeat(60));
    }
    let guide = folder.join("Guide - Adapdf");
    fs::create_dir_all(&guide).unwrap();
    let docx = guide.join("accessible.docx");
    fs::write(&docx, clean().build()).unwrap();
    assert!(docx.as_os_str().len() > 300);

    // The premise: Word itself cannot open this path.
    let opened = within(Duration::from_secs(120), {
        let docx = docx.clone();
        move |session: &WordSession| session.open_read_only(&docx).map(|d| d.close()).is_ok()
    });
    assert!(!opened, "Word must not open a path over its limit");

    let run = export_docx(&docx, &long(), &FixtureFonts, Timestamp::now());
    assert_eq!(run.exit, Exit::Pass, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        names(&guide),
        ["Guide.pdf", "accessible.docx", "report.html", "report.json"]
    );
    assert!(
        fs::read(guide.join("Guide.pdf"))
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert!(run.files.iter().all(|f| f.starts_with(&guide)));
    assert_eq!(work_folders(), temp_before, "the short folder is removed");
    assert_eq!(wait_for_word_pids(&before), before);
}

/// The short folders that the export makes in the temp folder (`adapdf-<process>-<n>`).
fn work_folders() -> Vec<String> {
    let prefix = format!("adapdf-{}-", std::process::id());
    let mut found: Vec<String> = fs::read_dir(std::env::temp_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(&prefix))
        .collect();
    found.sort();
    found
}
