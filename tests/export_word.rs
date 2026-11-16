//! Stage 3 tests that need Microsoft Word (level `windows_word` in the test
//! plan): the export of `adapdf::export`.
//!
//! Run them with `cargo test --features windows_word --test export_word`. Every
//! test starts its own hidden Word through the export, and never touches a Word
//! that a person has open: the `WINWORD.EXE` processes are listed with
//! `tasklist` before and after, and the two lists must be equal. A test that
//! needs "a person's Word" starts a separate Word of its own with a document
//! and an unsaved edit, and checks that it is the same afterwards. No test
//! saves a document. The tests share one lock (`word_support::exclusive`).
#![cfg(all(windows, feature = "windows_word"))]

mod export_support;
mod word_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use adapdf::export::{ExportError, ExportOptions, ExportResult, export_pdf};
use adapdf::findings::{RuleId, Severity};
use adapdf::fixtures::{FixtureFonts, Para, catalogue, clean};
use adapdf::word::{WordError, WordSession};
use export_support::{assert_tagged, contains, count, fingerprint, names, without_content_types};
use word_support::{TempDir, exclusive, wait_for_word_pids, word_pids};

/// A time limit that a test that works will not reach.
const LONG: Duration = Duration::from_secs(120);

fn options() -> ExportOptions {
    ExportOptions {
        timeout: LONG,
        after_open: None,
    }
}

fn export(docx: &Path, pdf: &Path) -> Result<ExportResult, ExportError> {
    export_pdf(docx, pdf, &FixtureFonts, &options())
}

fn fixture(rule: RuleId, fails: bool) -> Vec<u8> {
    catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .expect("the fixture exists")
        .bytes
}

/// The files that a run left that are not the ones it should have: `~$` files
/// and temporary PDFs.
fn leftovers(folder: &Path) -> Vec<String> {
    names(folder)
        .into_iter()
        .filter(|name| name.starts_with("~$") || name.contains(".adapdf-"))
        .collect()
}

/// The text of a document that is open in a Word, to compare later.
fn text_of(document: &adapdf::word::WordDocument<'_>) -> String {
    let count = document.paragraph_count().unwrap() as usize;
    (0..count)
        .map(|n| document.paragraph_text(n).unwrap())
        .collect::<Vec<_>>()
        .join("|")
}

/// Everything about a person's Word that the export must not change.
#[derive(Debug, PartialEq)]
struct Standing {
    documents: u32,
    text: String,
    saved: bool,
}

fn standing(user: &WordSession, document: &adapdf::word::WordDocument<'_>) -> Standing {
    Standing {
        documents: user.document_count().unwrap(),
        text: text_of(document),
        saved: document.is_saved().unwrap(),
    }
}

// S3-01
#[test]
fn export_starts_its_own_hidden_word() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s3-01");
    let docx = dir.write("Guide.docx", &clean().build());

    // While the export runs, a new WINWORD.EXE process exists.
    let seen = Arc::new(Mutex::new(None));
    let seen_in_hook = Arc::clone(&seen);
    let options = ExportOptions {
        timeout: LONG,
        after_open: Some(Arc::new(move |pid| {
            *seen_in_hook.lock().unwrap() = Some((pid, word_pids().contains(&pid)));
        })),
    };
    export_pdf(
        &docx,
        &dir.path().join("Guide.pdf"),
        &FixtureFonts,
        &options,
    )
    .unwrap();
    let (pid, alive) = seen.lock().unwrap().expect("the hook ran");
    assert!(alive, "the process {pid} was running during the export");
    assert!(!before.contains(&pid), "{pid} is a new process");
    assert_eq!(wait_for_word_pids(&before), before);

    // The session that the export uses starts a Word that is hidden and has no alerts.
    let session = WordSession::start().unwrap();
    assert!(!session.is_visible().unwrap());
    assert_eq!(session.display_alerts().unwrap(), 0);
    drop(session);
    assert_eq!(wait_for_word_pids(&before), before);
}

// S3-02
#[test]
fn export_leaves_the_users_word_alone() {
    let _lock = exclusive();
    let dir = TempDir::new("s3-02");
    let a = dir.write("A.docx", &clean().build());
    let b = dir.write("B.docx", &clean().para("The user's own guide.").build());
    let b_bytes = fs::read(&b).unwrap();
    let real_before = word_pids();

    // The "user" Word: a separate Word with B open and an unsaved edit.
    let user = WordSession::start().unwrap();
    let user_document = user.open_for_editing(&b).unwrap();
    user_document
        .append_text("An edit that is not saved.")
        .unwrap();
    let before = standing(&user, &user_document);
    assert!(!before.saved);
    assert_eq!(before.documents, 1);
    let with_user = word_pids();
    assert!(with_user.contains(&user.pid()));

    export(&a, &dir.path().join("A.pdf")).unwrap();

    assert_eq!(wait_for_word_pids(&with_user), with_user);
    assert!(
        word_pids().contains(&user.pid()),
        "the user's Word is alive"
    );
    assert_eq!(standing(&user, &user_document), before);
    assert_eq!(
        fs::read(&b).unwrap(),
        b_bytes,
        "the user's file is unchanged"
    );

    // The variant: the tool exports the file that the user has open.
    let lock_files = leftovers(dir.path());
    assert_eq!(lock_files.len(), 1, "Word's lock file of B: {lock_files:?}");
    export(&b, &dir.path().join("B.pdf")).unwrap();
    assert_eq!(wait_for_word_pids(&with_user), with_user);
    assert_eq!(standing(&user, &user_document), before);
    assert_eq!(fs::read(&b).unwrap(), b_bytes);
    assert_eq!(
        leftovers(dir.path()),
        lock_files,
        "the tool did not remove the lock file of the user's Word"
    );
    assert_tagged("B.pdf", &fs::read(dir.path().join("B.pdf")).unwrap());

    user_document.close().unwrap();
    drop(user);
    assert_eq!(wait_for_word_pids(&real_before), real_before);
}

// S3-03 and S3-15
#[test]
fn export_closes_word_after_success() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s3-03");
    let docx = dir.write("Guide.docx", &clean().build());
    let docx_before = fingerprint(&docx);

    let result = export(&docx, &dir.path().join("Guide.pdf")).unwrap();
    assert!(result.duration > Duration::ZERO);
    assert!(
        result.word_build.starts_with("16."),
        "{}",
        result.word_build
    );

    assert_eq!(wait_for_word_pids(&before), before);
    assert_eq!(leftovers(dir.path()), Vec::<String>::new());
    assert_eq!(names(dir.path()), ["Guide.docx", "Guide.pdf"]);
    assert_eq!(fingerprint(&docx), docx_before, "bytes and modified time");
}

// S3-15 (one fixture here; the pass fixtures are in `tests/export_fixtures.rs`)
#[test]
fn export_leaves_the_docx_unchanged() {
    let _lock = exclusive();
    let dir = TempDir::new("s3-15");
    for (name, bytes) in [
        ("Plain.docx", clean().build()),
        ("Toc.docx", fixture(RuleId::Txt004, true)),
    ] {
        let docx = dir.write(name, &bytes);
        let before = fingerprint(&docx);
        export(&docx, &docx.with_extension("pdf")).unwrap();
        assert_eq!(fingerprint(&docx), before, "{name}");
    }
}

// S3-04
#[test]
fn export_closes_word_after_an_error() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s3-04");

    // The output folder does not exist.
    let docx = dir.write("Guide.docx", &clean().build());
    let missing = dir.path().join("no-such-folder").join("Guide.pdf");
    let error = export(&docx, &missing).unwrap_err();
    assert!(matches!(error, ExportError::OutputFolder { .. }), "{error}");
    assert_eq!(wait_for_word_pids(&before), before);

    // Stage 1 passes, and Word cannot open the file: the package does not say
    // what kind of part `word/document.xml` is.
    let broken = dir.write("Broken.docx", &without_content_types(&clean().build()));
    let pdf = dir.path().join("Broken.pdf");
    let error = export(&broken, &pdf).unwrap_err();
    println!("Word said: {error}");
    assert!(
        matches!(error, ExportError::Word(WordError::Open { .. })),
        "{error:?}"
    );
    assert_eq!(wait_for_word_pids(&before), before);
    assert!(!pdf.exists());
    assert_eq!(leftovers(dir.path()), Vec::<String>::new());
}

/// The real version of S3-05: the code on the Word thread panics.
#[test]
fn export_closes_word_when_the_word_thread_panics() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s3-05");
    let docx = dir.write("Guide.docx", &clean().build());
    let options = ExportOptions {
        timeout: LONG,
        after_open: Some(Arc::new(|_| panic!("the code that uses Word fails"))),
    };
    let result = export_pdf(
        &docx,
        &dir.path().join("Guide.pdf"),
        &FixtureFonts,
        &options,
    );
    assert!(
        matches!(result, Err(ExportError::WorkerStopped)),
        "{result:?}"
    );
    assert_eq!(wait_for_word_pids(&before), before);
    assert_eq!(names(dir.path()), ["Guide.docx"]);
}

// S3-08
#[test]
fn export_times_out_with_real_word() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s3-08");
    let docx = dir.write("Guide.docx", &clean().build());
    let docx_before = fingerprint(&docx);
    // 1 ms: the deadline is gone before Word has started.
    let options = ExportOptions {
        timeout: Duration::from_millis(1),
        after_open: None,
    };
    let started = Instant::now();
    let result = export_pdf(
        &docx,
        &dir.path().join("Guide.pdf"),
        &FixtureFonts,
        &options,
    );
    let Err(error @ ExportError::Timeout { .. }) = result else {
        panic!("expected a timeout, got {result:?}");
    };
    println!("{error} (after {:?})", started.elapsed());
    assert!(
        matches!(
            error,
            ExportError::Timeout {
                pid: Some(_),
                ended: true,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(
        wait_for_word_pids(&before),
        before,
        "no Word of the tool remains"
    );
    assert_eq!(names(dir.path()), ["Guide.docx"], "no partial PDF");
    assert_eq!(fingerprint(&docx), docx_before);
}

/// The real version of S3-06: Word hangs with the document open, a second Word
/// (a person's) is running, and only the tool's own Word is ended.
#[test]
fn export_times_out_and_closes_only_its_own_word() {
    let _lock = exclusive();
    let dir = TempDir::new("s3-06");
    let a = dir.write("A.docx", &clean().build());
    let b = dir.write("B.docx", &clean().para("The user's own guide.").build());
    let real_before = word_pids();

    let user = WordSession::start().unwrap();
    let user_document = user.open_for_editing(&b).unwrap();
    user_document
        .append_text("An edit that is not saved.")
        .unwrap();
    let before = standing(&user, &user_document);
    let with_user = word_pids();

    // The hook does not return until the tool's Word is gone: a Word that hangs.
    let hook_pid = Arc::new(AtomicU32::new(0));
    let hook_pid_seen = Arc::clone(&hook_pid);
    let options = ExportOptions {
        timeout: Duration::from_secs(15),
        after_open: Some(Arc::new(move |pid| {
            hook_pid_seen.store(pid, Ordering::SeqCst);
            let give_up = Instant::now() + Duration::from_secs(100);
            while word_pids().contains(&pid) && Instant::now() < give_up {
                sleep(Duration::from_millis(200));
            }
        })),
    };
    let result = export_pdf(&a, &dir.path().join("A.pdf"), &FixtureFonts, &options);
    let Err(ExportError::Timeout { pid, ended, .. }) = result else {
        panic!("expected a timeout, got {result:?}");
    };
    let tool_pid = pid.expect("the id of the tool's Word is known");
    assert!(ended);
    let hung = hook_pid.load(Ordering::SeqCst);
    assert!(hung == 0 || hung == tool_pid, "the hook saw {hung}");
    assert_ne!(tool_pid, user.pid());

    // The tool's Word is gone, the user's Word and document are as they were.
    assert!(!word_pids().contains(&tool_pid));
    assert_eq!(wait_for_word_pids(&with_user), with_user);
    assert_eq!(standing(&user, &user_document), before);
    assert_eq!(
        names(dir.path())
            .iter()
            .filter(|n| n.ends_with(".pdf"))
            .count(),
        0
    );
    assert_eq!(leftovers(dir.path()).len(), 1, "only the user's lock file");

    user_document.close().unwrap();
    drop(user);
    assert_eq!(wait_for_word_pids(&real_before), real_before);
}

// S3-10
#[test]
fn export_makes_a_tagged_pdf_with_bookmarks_and_properties() {
    let _lock = exclusive();
    let dir = TempDir::new("s3-10");
    let docx = dir.write(
        "Guide.docx",
        &clean()
            .heading(2, "Setting up the printer")
            .para("Plug it in.")
            .build(),
    );
    let pdf_path = dir.path().join("Guide.pdf");
    let result = export(&docx, &pdf_path).unwrap();
    let pdf = fs::read(&pdf_path).unwrap();

    assert_tagged("Guide.pdf", &pdf);
    // Bookmarks for the two headings.
    assert!(contains(&pdf, b"/Outlines"));
    assert!(contains(&pdf, b"/Title(Test guide)"));
    assert!(contains(&pdf, b"/Title(Setting up the printer)"));
    // Document properties: the Info title and the XMP title.
    assert!(contains(&pdf, b"/Title(Adapdf test guide)"));
    assert!(contains(&pdf, b"Adapdf test guide</rdf:li>"));
    // PDF/A is off.
    assert!(!contains(&pdf, b"pdfaid"));
    assert!(!result.parameters.use_iso19005_1);
}

// S3-11
#[test]
fn export_updates_fields_before_exporting() {
    let _lock = exclusive();
    let dir = TempDir::new("s3-11");
    // A table of contents with one entry, in a document that has four
    // headings. Word makes one link for each entry of the table of contents, so
    // the number of links in the PDF is the number of entries that Word wrote.
    let stale = clean()
        .toc(&[("Test guide", 1)], false)
        .paragraph(Para::new("Test guide").style("Heading1").bookmark("_Toc1"))
        .paragraph(Para::new("Setup steps").style("Heading2").bookmark("_Toc2"))
        .para("Plug it in.")
        .paragraph(Para::new("Cleaning up").style("Heading2").bookmark("_Toc3"))
        .para("Wipe it.")
        .build();
    let docx = dir.write("Stale.docx", &stale);
    let findings = adapdf::check_bytes(&stale, &FixtureFonts).unwrap();
    assert!(findings.iter().any(|f| f.rule_id == RuleId::Txt004));
    let before = fingerprint(&docx);

    let pdf_path = dir.path().join("Stale.pdf");
    let result = export(&docx, &pdf_path).unwrap();
    let pdf = fs::read(&pdf_path).unwrap();
    assert_eq!(result.fields.tables_of_contents, 1);
    assert!(result.fields.fields >= 1);
    assert_eq!(result.fields.first_failed, None);
    assert_eq!(
        count(&pdf, b"/Subtype/Link"),
        4,
        "the table of contents has an entry for each of the 4 headings, not the 1 it had"
    );
    assert_eq!(fingerprint(&docx), before, "the .docx on disk is unchanged");

    // The fixture of TXT-004: a heading renamed after the table was made. The
    // text of the entry is not read from the PDF (see "Known gaps in M4a").
    let renamed = dir.write("Renamed.docx", &fixture(RuleId::Txt004, true));
    let result = export(&renamed, &dir.path().join("Renamed.pdf")).unwrap();
    assert_eq!(result.fields.tables_of_contents, 1);
    let pdf = fs::read(dir.path().join("Renamed.pdf")).unwrap();
    assert_eq!(count(&pdf, b"/Subtype/Link"), 2);
}

// S3-12 with the real Word
#[test]
fn export_is_refused_when_doc_005_fires_and_starts_no_word() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s3-12");
    let tracked = dir.write("Tracked.docx", &fixture(RuleId::Doc005, true));
    let comment = dir.write(
        "Comment.docx",
        &clean()
            .paragraph(Para::new("Check this.").comment("Is this right?"))
            .build(),
    );
    for docx in [tracked, comment] {
        let bytes = fs::read(&docx).unwrap();
        let pdf = docx.with_extension("pdf");
        let error = export(&docx, &pdf).unwrap_err();
        assert!(matches!(error, ExportError::Blocked { .. }), "{error}");
        assert!(!pdf.exists());
        assert_eq!(word_pids(), before, "Word was not started");
        assert_eq!(fs::read(&docx).unwrap(), bytes);
    }
}

// S3-14
#[test]
fn export_runs_when_only_warnings_exist() {
    let _lock = exclusive();
    let dir = TempDir::new("s3-14");
    for rule in [RuleId::Hdg006, RuleId::Txt002] {
        assert_eq!(rule.severity(), Severity::Warning);
        let docx = dir.write(&format!("{rule}.docx"), &fixture(rule, true));
        let result = export(&docx, &docx.with_extension("pdf")).unwrap();
        assert!(result.findings.iter().any(|f| f.rule_id == rule));
        assert_tagged(
            &format!("{rule}.pdf"),
            &fs::read(docx.with_extension("pdf")).unwrap(),
        );
    }
}

// S3-18
#[test]
fn export_handles_awkward_paths() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s3-18");

    // Spaces and letters that are not ASCII.
    let folder = dir.path().join("Ünïcode folder, with spaces — and a dash");
    fs::create_dir_all(&folder).unwrap();
    let docx = folder.join("Guide – résumé (final).docx");
    fs::write(&docx, clean().build()).unwrap();
    let pdf = folder.join("Guide – résumé (final).pdf");
    export(&docx, &pdf).unwrap();
    assert_tagged("unicode", &fs::read(&pdf).unwrap());

    // A long path: under the 260 characters that Word accepts, with room for
    // the temporary name.
    let mut long: PathBuf = dir.path().to_path_buf();
    while long.as_os_str().len() < 180 {
        long.push("a-long-folder-name-0123456789");
    }
    fs::create_dir_all(&long).unwrap();
    let docx = long.join("Guide.docx");
    fs::write(&docx, clean().build()).unwrap();
    let pdf = long.join("Guide.pdf");
    println!("a path of {} characters", pdf.as_os_str().len());
    export(&docx, &pdf).unwrap();
    assert_tagged("long", &fs::read(&pdf).unwrap());

    assert_eq!(wait_for_word_pids(&before), before);
}

// S3-19
#[test]
fn export_replaces_or_reports_a_locked_pdf() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s3-19");
    let docx = dir.write("Guide.docx", &clean().build());

    // A PDF from an earlier run is replaced.
    let old = dir.write("Guide.pdf", b"%PDF-1.4 an earlier run");
    export(&docx, &old).unwrap();
    let new = fs::read(&old).unwrap();
    assert_tagged("replaced", &new);
    assert!(!contains(&new, b"an earlier run"));

    // A PDF that another program holds is reported, and it is kept.
    let held_file = {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&old)
            .unwrap()
    };
    let error = export(&docx, &old).unwrap_err();
    assert!(matches!(error, ExportError::PdfLocked { .. }), "{error:?}");
    assert!(error.to_string().contains("Guide.pdf"), "{error}");
    drop(held_file);
    assert_eq!(fs::read(&old).unwrap(), new);
    assert_eq!(names(dir.path()), ["Guide.docx", "Guide.pdf"]);
    assert_eq!(wait_for_word_pids(&before), before);
}
