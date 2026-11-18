//! Unit tests of the export. They use a fake Word that records its calls, so
//! they need no Word. The tests that use the real Word are in
//! `tests/export_word.rs`.

use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use super::*;
use crate::findings::RuleId;
use crate::fixtures::{FixtureFonts, Para, catalogue, clean};
use crate::word::{ComError, ExportParameters, FieldUpdate};
use engine::{WordApp, WordDoc};

const PDF_BYTES: &[u8] = b"%PDF-1.7\nfake";

#[derive(Debug, Clone, PartialEq)]
enum Call {
    Start,
    Build,
    Open(PathBuf),
    UpdateFields,
    Export(PathBuf, ExportParameters),
    Close,
    Quit,
}

type Calls = Arc<Mutex<Vec<Call>>>;

/// What the fake Word does.
#[derive(Clone)]
struct Behavior {
    pid: u32,
    start_delay: Duration,
    export_delay: Duration,
    start_error: bool,
    export_error: bool,
    export_panics: bool,
    /// What the fake writes to the output file. `None`: nothing.
    output: Option<&'static [u8]>,
}

impl Default for Behavior {
    fn default() -> Behavior {
        Behavior {
            pid: 111,
            start_delay: Duration::ZERO,
            export_delay: Duration::ZERO,
            start_error: false,
            export_error: false,
            export_panics: false,
            output: Some(PDF_BYTES),
        }
    }
}

struct FakeApp {
    calls: Calls,
    behavior: Behavior,
}

struct FakeDoc {
    calls: Calls,
    behavior: Behavior,
}

fn record(calls: &Calls, call: Call) {
    calls.lock().unwrap().push(call);
}

fn fake_start(calls: &Calls, behavior: &Behavior) -> StartWord {
    let calls = Arc::clone(calls);
    let behavior = behavior.clone();
    Box::new(move || {
        sleep(behavior.start_delay);
        if behavior.start_error {
            return Err(WordError::NotInstalled);
        }
        record(&calls, Call::Start);
        Ok(Box::new(FakeApp { calls, behavior }) as Box<dyn WordApp>)
    })
}

impl WordApp for FakeApp {
    fn pid(&self) -> u32 {
        self.behavior.pid
    }

    fn build(&self) -> crate::word::Result<String> {
        record(&self.calls, Call::Build);
        Ok("16.0.99999.00000".to_string())
    }

    fn open_read_only(&self, path: &Path) -> crate::word::Result<Box<dyn WordDoc + '_>> {
        record(&self.calls, Call::Open(path.to_path_buf()));
        Ok(Box::new(FakeDoc {
            calls: Arc::clone(&self.calls),
            behavior: self.behavior.clone(),
        }))
    }
}

impl Drop for FakeApp {
    fn drop(&mut self) {
        record(&self.calls, Call::Quit);
    }
}

impl WordDoc for FakeDoc {
    fn is_read_only(&self) -> crate::word::Result<bool> {
        Ok(true)
    }

    fn update_fields(&self) -> crate::word::Result<FieldUpdate> {
        record(&self.calls, Call::UpdateFields);
        Ok(FieldUpdate {
            fields: 4,
            tables_of_contents: 1,
            first_failed: None,
        })
    }

    fn export(&self, output: &Path, parameters: &ExportParameters) -> crate::word::Result<()> {
        record(&self.calls, Call::Export(output.to_path_buf(), *parameters));
        sleep(self.behavior.export_delay);
        assert!(!self.behavior.export_panics, "the fake Word panics");
        if self.behavior.export_error {
            return Err(WordError::Word(ComError {
                call: "ExportAsFixedFormat".to_string(),
                code: -1,
                message: "the fake export failed".to_string(),
            }));
        }
        if let Some(bytes) = self.behavior.output {
            fs::write(output, bytes).unwrap();
        }
        Ok(())
    }

    fn close(self: Box<Self>) -> crate::word::Result<()> {
        record(&self.calls, Call::Close);
        Ok(())
    }
}

/// Records the ids it is asked to end.
#[derive(Default)]
struct FakeEnder(Mutex<Vec<u32>>);

impl EndProcess for FakeEnder {
    fn end(&self, pid: u32) -> crate::word::Result<bool> {
        self.0.lock().unwrap().push(pid);
        Ok(true)
    }
}

/// A folder under the temp folder, removed at the end.
struct TempDir(PathBuf);

static NEXT_DIR: AtomicU32 = AtomicU32::new(0);

impl TempDir {
    fn new() -> TempDir {
        let n = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("adapdf-export-unit-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn pdf(&self) -> PathBuf {
        self.0.join("Guide.pdf")
    }

    /// The names of the files in the folder.
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn options() -> ExportOptions {
    ExportOptions::default()
}

fn clean_docx(dir: &TempDir) -> PathBuf {
    dir.write("Guide.docx", &clean().build())
}

/// Runs the export with a fake Word and fake process ending.
fn export(
    docx: &Path,
    pdf: &Path,
    behavior: &Behavior,
    options: &ExportOptions,
) -> (Result<ExportResult, ExportError>, Vec<Call>, Vec<u32>) {
    let calls = Calls::default();
    let ender = FakeEnder::default();
    let result = run(
        docx,
        pdf,
        &FixtureFonts,
        options,
        fake_start(&calls, behavior),
        &ender,
    );
    let calls = calls.lock().unwrap().clone();
    let ended = ender.0.lock().unwrap().clone();
    (result, calls, ended)
}

/// A start that fails the test when it is used: Word must not be started.
fn word_must_not_start() -> StartWord {
    Box::new(|| panic!("Word was started, but the export must stop before"))
}

fn run_without_word(docx: &Path, pdf: &Path) -> Result<ExportResult, ExportError> {
    run(
        docx,
        pdf,
        &FixtureFonts,
        &options(),
        word_must_not_start(),
        &FakeEnder::default(),
    )
}

// S3-07
#[test]
fn export_timeout_is_five_minutes_by_default() {
    assert_eq!(ExportOptions::default().timeout, Duration::from_secs(300));
    assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(300));
}

// S3-09
#[test]
fn export_passes_the_spec_parameters() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let (result, calls, _) = export(&docx, &dir.pdf(), &Behavior::default(), &options());
    let result = result.unwrap();

    let [
        ..,
        Call::Export(output, parameters),
        Call::Close,
        Call::Quit,
    ] = calls.as_slice()
    else {
        panic!("unexpected calls: {calls:?}");
    };
    // The values of the table in spec 02, written out here once more.
    assert_eq!(
        *parameters,
        ExportParameters {
            export_format: 17,
            open_after_export: false,
            optimize_for: 0,
            item: 0,
            include_doc_props: true,
            create_bookmarks: 1,
            doc_structure_tags: true,
            bitmap_missing_fonts: true,
            use_iso19005_1: false,
        }
    );
    assert_eq!(result.parameters, *parameters);
    // Word writes next to the PDF, to a temporary name, never to the PDF itself.
    assert_eq!(output.parent(), dir.pdf().parent());
    assert_ne!(output, &dir.pdf());
    // The document is opened read-only (the only open the engine offers).
    assert!(calls.contains(&Call::Open(docx)));
}

/// The parameter values in the code are the values in the table of spec 02.
#[test]
fn the_parameters_are_the_spec_table() {
    let spec = include_str!("../../docs/specs/02-pipeline.md");
    let value = |name: &str| -> String {
        let row = spec
            .lines()
            .find(|line| line.starts_with(&format!("| `{name}` |")))
            .unwrap_or_else(|| panic!("no row for {name} in spec 02"));
        row.split('|').nth(2).unwrap().trim().to_string()
    };
    let p = ExportParameters::SPEC;
    assert_eq!(value("ExportFormat"), p.export_format.to_string());
    assert_eq!(value("OpenAfterExport"), p.open_after_export.to_string());
    assert_eq!(value("OptimizeFor"), p.optimize_for.to_string());
    assert_eq!(value("Item"), p.item.to_string());
    assert_eq!(value("IncludeDocProps"), p.include_doc_props.to_string());
    assert_eq!(value("CreateBookmarks"), p.create_bookmarks.to_string());
    assert_eq!(value("DocStructureTags"), p.doc_structure_tags.to_string());
    assert_eq!(
        value("BitmapMissingFonts"),
        p.bitmap_missing_fonts.to_string()
    );
    assert_eq!(value("UseISO19005_1"), p.use_iso19005_1.to_string());
}

// S3-11 (the order; the effect is tested with the real Word)
#[test]
fn export_updates_fields_before_exporting() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let (result, calls, ended) = export(&docx, &dir.pdf(), &Behavior::default(), &options());
    let result = result.unwrap();

    let names: Vec<&str> = calls
        .iter()
        .map(|call| match call {
            Call::Start => "start",
            Call::Build => "build",
            Call::Open(_) => "open",
            Call::UpdateFields => "update_fields",
            Call::Export(..) => "export",
            Call::Close => "close",
            Call::Quit => "quit",
        })
        .collect();
    assert_eq!(
        names,
        [
            "start",
            "build",
            "open",
            "update_fields",
            "export",
            "close",
            "quit"
        ]
    );
    assert!(ended.is_empty(), "nothing is ended on success");
    assert_eq!(result.fields.fields, 4);
    assert_eq!(result.fields.tables_of_contents, 1);
    assert_eq!(result.word_build, "16.0.99999.00000");
    assert_eq!(result.pdf, dir.pdf());
}

// S3-12
#[test]
fn export_is_refused_when_doc_005_fires() {
    let dir = TempDir::new();
    let tracked = dir.write(
        "Tracked.docx",
        &catalogue()
            .into_iter()
            .find(|f| f.rule == RuleId::Doc005 && f.fails)
            .unwrap()
            .bytes,
    );
    let comment = dir.write(
        "Comment.docx",
        &clean()
            .paragraph(Para::new("Check this.").comment("Is this right?"))
            .build(),
    );
    for (docx, what) in [(tracked, "tracked changes"), (comment, "comments")] {
        let before = fs::read(&docx).unwrap();
        let pdf = dir.0.join(format!("{what}.pdf").replace(' ', "-"));
        let Err(error) = run_without_word(&docx, &pdf) else {
            panic!("the export of a document with {what} must be refused");
        };
        let ExportError::Blocked { findings } = &error else {
            panic!("expected Blocked, got {error:?}");
        };
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == RuleId::Doc005 && f.severity == Severity::Error),
            "{findings:?}"
        );
        assert!(error.to_string().contains(what), "{error}");
        assert!(error.to_string().contains("DOC-005"), "{error}");
        assert!(!pdf.exists());
        assert_eq!(fs::read(&docx).unwrap(), before);
    }
    assert!(
        dir.names().iter().all(|name| !name.ends_with(".pdf")),
        "{:?}",
        dir.names()
    );
}

// S3-13 without Word: every failing fixture of an error rule is refused.
#[test]
fn export_writes_no_pdf_when_stage_1_has_errors() {
    let dir = TempDir::new();
    let mut refused = Vec::new();
    for fixture in catalogue()
        .into_iter()
        .filter(|f| f.fails && f.rule.severity() == Severity::Error)
    {
        let docx = dir.write(&fixture.file_name(), &fixture.bytes);
        let pdf = docx.with_extension("pdf");
        let Err(ExportError::Blocked { findings }) = run_without_word(&docx, &pdf) else {
            panic!("{} must be refused", fixture.file_name());
        };
        assert!(
            findings.iter().any(|f| f.rule_id == fixture.rule),
            "{}",
            fixture.file_name()
        );
        assert!(!pdf.exists());
        refused.push(fixture.rule);
    }
    assert_eq!(refused.len(), 18, "{refused:?}");
    assert!(dir.names().iter().all(|n| !n.ends_with(".pdf")));
}

// S3-14 without Word: a warning does not block.
#[test]
fn export_runs_when_only_warnings_exist() {
    let dir = TempDir::new();
    for rule in [RuleId::Hdg006, RuleId::Txt002] {
        let fixture = catalogue()
            .into_iter()
            .find(|f| f.rule == rule && f.fails)
            .unwrap();
        assert_eq!(rule.severity(), Severity::Warning);
        let docx = dir.write(&fixture.file_name(), &fixture.bytes);
        let pdf = docx.with_extension("pdf");
        let (result, _, _) = export(&docx, &pdf, &Behavior::default(), &options());
        let result = result.unwrap();
        assert!(result.findings.iter().any(|f| f.rule_id == rule));
        assert!(
            result
                .findings
                .iter()
                .all(|f| f.severity == Severity::Warning)
        );
        assert!(pdf.is_file());
    }
}

// S3-16
#[test]
fn export_reports_missing_word() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let behavior = Behavior {
        start_error: true,
        ..Behavior::default()
    };
    let (result, _, ended) = export(&docx, &dir.pdf(), &behavior, &options());
    let Err(error @ ExportError::Word(WordError::NotInstalled)) = result else {
        panic!("expected NotInstalled, got {result:?}");
    };
    assert!(error.to_string().contains("not installed"), "{error}");
    assert!(ended.is_empty());
    assert_eq!(dir.names(), ["Guide.docx"]);
}

// S3-05
#[test]
fn export_closes_the_instance_when_the_code_panics() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let behavior = Behavior {
        export_panics: true,
        ..Behavior::default()
    };
    let (result, calls, ended) = export(&docx, &dir.pdf(), &behavior, &options());
    assert!(
        matches!(result, Err(ExportError::WorkerStopped)),
        "{result:?}"
    );
    assert_eq!(calls.last(), Some(&Call::Quit), "{calls:?}");
    assert!(ended.is_empty());
    assert_eq!(dir.names(), ["Guide.docx"], "no temporary PDF is left");
}

// S3-04 (without Word): an error in Word still quits it and leaves nothing.
#[test]
fn export_closes_word_after_an_error() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let old = dir.write("Guide.pdf", b"%PDF-1.4 the old one");
    let behavior = Behavior {
        export_error: true,
        ..Behavior::default()
    };
    let (result, calls, _) = export(&docx, &dir.pdf(), &behavior, &options());
    let Err(error @ ExportError::Word(_)) = result else {
        panic!("{result:?}");
    };
    assert!(error.to_string().contains("the fake export failed"));
    assert_eq!(calls.last(), Some(&Call::Quit));
    // The old PDF is as it was, and no temporary file remains.
    assert_eq!(fs::read(&old).unwrap(), b"%PDF-1.4 the old one");
    assert_eq!(dir.names(), ["Guide.docx", "Guide.pdf"]);
}

// S3-06
#[test]
fn export_times_out_and_ends_only_its_own_word() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let behavior = Behavior {
        pid: 111,
        export_delay: Duration::from_secs(1),
        ..Behavior::default()
    };
    // A second Word, the user's, with another id. The export does not know it.
    let users_calls = Calls::default();
    let users_word = fake_start(
        &users_calls,
        &Behavior {
            pid: 222,
            ..Behavior::default()
        },
    )()
    .unwrap();
    let limit = Duration::from_millis(100);
    let (result, calls, ended) = export(
        &docx,
        &dir.pdf(),
        &behavior,
        &ExportOptions {
            timeout: limit,
            ..options()
        },
    );
    let Err(error @ ExportError::Timeout { .. }) = result else {
        panic!("expected a timeout, got {result:?}");
    };
    assert!(matches!(
        error,
        ExportError::Timeout {
            pid: Some(111),
            ended: true,
            ..
        }
    ));
    assert_eq!(ended, [111], "only the tool's own Word is ended");
    assert_eq!(users_word.pid(), 222);
    assert_eq!(*users_calls.lock().unwrap(), [Call::Start]);
    assert!(calls.contains(&Call::Start));
    assert!(
        error.to_string().contains("ended the copy of Word"),
        "{error}"
    );
    assert_eq!(dir.names(), ["Guide.docx"], "no PDF, no temporary file");
}

/// The deadline passes while Word is still starting: the tool waits for the
/// id of its Word, ends it, and does not leave Word running.
#[test]
fn export_times_out_before_word_has_started() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let behavior = Behavior {
        start_delay: Duration::from_millis(300),
        export_delay: Duration::from_millis(500),
        ..Behavior::default()
    };
    let (result, _, ended) = export(
        &docx,
        &dir.pdf(),
        &behavior,
        &ExportOptions {
            timeout: Duration::from_millis(50),
            ..options()
        },
    );
    assert!(
        matches!(
            result,
            Err(ExportError::Timeout {
                pid: Some(111),
                ended: true,
                ..
            })
        ),
        "{result:?}"
    );
    assert_eq!(ended, [111]);
    assert_eq!(dir.names(), ["Guide.docx"]);
}

/// A Word that finishes just after the deadline is a result, not a timeout:
/// nothing is ended, because the id may already belong to another program.
#[test]
fn a_result_is_not_lost_to_the_deadline() {
    let (sender, receiver) = mpsc::channel();
    sender.send(Message::Started(111)).unwrap();
    sender
        .send(Message::Finished(Ok(WordWork {
            build: "b".to_string(),
            fields: FieldUpdate {
                fields: 0,
                tables_of_contents: 0,
                first_failed: None,
            },
        })))
        .unwrap();
    let ender = FakeEnder::default();
    let work = wait_for_word(&receiver, Duration::ZERO, &ender);
    assert!(work.is_ok());
    assert!(ender.0.lock().unwrap().is_empty());
}

// S3-19, the first half
#[test]
fn export_replaces_an_existing_pdf() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    dir.write("Guide.pdf", b"%PDF-1.4 the old one");
    let (result, _, _) = export(&docx, &dir.pdf(), &Behavior::default(), &options());
    result.unwrap();
    assert_eq!(fs::read(dir.pdf()).unwrap(), PDF_BYTES);
    assert_eq!(dir.names(), ["Guide.docx", "Guide.pdf"]);
}

// S3-19, the second half
#[test]
fn export_reports_a_locked_pdf_and_keeps_it() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let old = dir.write("Guide.pdf", b"%PDF-1.4 the old one");
    let _held = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&old)
        .unwrap();
    let result = run_without_word(&docx, &old);
    let Err(error @ ExportError::PdfLocked { .. }) = result else {
        panic!("expected PdfLocked, got {result:?}");
    };
    let text = error.to_string();
    assert!(text.contains("Guide.pdf"), "{text}");
    assert!(text.contains("cannot be replaced"), "{text}");
    assert_eq!(dir.names(), ["Guide.docx", "Guide.pdf"]);
}

/// Word is done but the PDF cannot be moved over the old one: the old one stays.
#[test]
fn a_pdf_that_cannot_be_replaced_at_the_end_keeps_the_old_one() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let old = dir.write("Guide.pdf", b"%PDF-1.4 the old one");
    // The fake Word takes the lock while it "exports", after the early check.
    let held = Arc::new(Mutex::new(None));
    let hold = Arc::clone(&held);
    let target = old.clone();
    let options = ExportOptions {
        after_open: Some(Arc::new(move |_| {
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(&target)
                .unwrap();
            *hold.lock().unwrap() = Some(file);
        })),
        ..options()
    };
    let (result, _, _) = export(&docx, &old, &Behavior::default(), &options);
    let Err(ExportError::PdfLocked { file, .. }) = result else {
        panic!("expected PdfLocked, got {result:?}");
    };
    assert_eq!(file, old);
    drop(held.lock().unwrap().take());
    assert_eq!(fs::read(&old).unwrap(), b"%PDF-1.4 the old one");
    assert_eq!(dir.names(), ["Guide.docx", "Guide.pdf"]);
}

#[test]
fn a_missing_output_folder_is_an_error_that_names_it() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let pdf = dir.0.join("no-such-folder").join("Guide.pdf");
    let result = run_without_word(&docx, &pdf);
    let Err(error @ ExportError::OutputFolder { .. }) = result else {
        panic!("expected OutputFolder, got {result:?}");
    };
    assert!(error.to_string().contains("no-such-folder"), "{error}");
}

#[test]
fn output_that_is_not_a_pdf_is_an_error_and_keeps_the_old_pdf() {
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    dir.write("Guide.pdf", b"%PDF-1.4 the old one");
    for output in [None, Some(b"not a pdf".as_slice())] {
        let behavior = Behavior {
            output,
            ..Behavior::default()
        };
        let (result, _, _) = export(&docx, &dir.pdf(), &behavior, &options());
        assert!(matches!(result, Err(ExportError::NoPdf)), "{result:?}");
        assert_eq!(fs::read(dir.pdf()).unwrap(), b"%PDF-1.4 the old one");
        assert_eq!(dir.names(), ["Guide.docx", "Guide.pdf"]);
    }
}

#[test]
fn a_file_that_is_not_a_docx_is_a_check_error() {
    let dir = TempDir::new();
    let docx = dir.write("Guide.docx", b"This is not a Word document.");
    let result = run_without_word(&docx, &dir.pdf());
    assert!(matches!(result, Err(ExportError::Check(_))), "{result:?}");
    assert_eq!(dir.names(), ["Guide.docx"]);
}

#[test]
fn the_docx_is_not_changed_by_the_fake_export() {
    // The fake never writes it; this guards the export code (the stage 1 read
    // and the temporary file) against writing the source.
    let dir = TempDir::new();
    let docx = clean_docx(&dir);
    let before = fs::read(&docx).unwrap();
    let (result, _, _) = export(&docx, &dir.pdf(), &Behavior::default(), &options());
    result.unwrap();
    assert_eq!(fs::read(&docx).unwrap(), before);
}

#[test]
fn error_messages_are_plain() {
    let timeout = ExportError::Timeout {
        limit: DEFAULT_TIMEOUT,
        pid: Some(7),
        ended: true,
    };
    assert_eq!(
        timeout.to_string(),
        "Word did not finish the export within 5 minutes. The tool ended the copy of Word that it had started. No PDF was written."
    );
    let stuck = ExportError::Timeout {
        limit: Duration::from_secs(90),
        pid: Some(7),
        ended: false,
    };
    assert!(stuck.to_string().contains("process 7"));
    let unknown = ExportError::Timeout {
        limit: Duration::from_millis(1500),
        pid: None,
        ended: false,
    };
    assert!(unknown.to_string().contains("1.5 seconds"));
    assert!(unknown.to_string().contains("ended none"));
}
