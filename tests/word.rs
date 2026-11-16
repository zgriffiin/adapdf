//! Tests that need Microsoft Word (level `windows_word` in the test plan).
//!
//! Run them with `cargo test --features windows_word`. Every test starts its own
//! hidden Word and never touches a Word that the user has open. The tests share
//! one lock, so the process counts in one test are not disturbed by the Word
//! that another test started.
#![cfg(all(windows, feature = "windows_word"))]

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use adapdf::fixtures::{DocxBuilder, zip_stored};
use adapdf::word::{WordError, WordSession};

mod word_support;
use word_support::{TempDir, exclusive, kill, wait_for_word_pids, within, word_pids};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// W-01: Word opens every fixture.
#[test]
fn word_opens_every_fixture() {
    let _lock = exclusive();
    let before = word_pids();

    let mut files: Vec<PathBuf> = fs::read_dir(fixtures_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "docx"))
        .collect();
    files.sort();
    assert!(files.len() > 70, "the fixtures are in tests/fixtures");

    let session = WordSession::start().expect("start Word");
    assert!(
        !before.contains(&session.pid()),
        "the session started a new Word process"
    );
    let build = session.build().expect("read the Word build");
    println!("Word build {build}, process {}", session.pid());

    let mut open = Vec::new();
    let mut failures = Vec::new();
    for file in &files {
        match session.open_read_only(file) {
            Ok(document) => open.push((file, document)),
            Err(error) => failures.push(format!("{}: {error}", file.display())),
        }
    }
    assert!(
        failures.is_empty(),
        "Word rejected:\n{}",
        failures.join("\n")
    );
    for (file, document) in &open {
        let paragraphs = document.paragraph_count().expect("count the paragraphs");
        assert!(paragraphs > 0, "{} has no paragraphs", file.display());
    }
    for (_, document) in open {
        document.close().expect("close the document");
    }
    drop(session);

    let after = wait_for_word_pids(&before);
    assert_eq!(
        after, before,
        "the Word processes after the test are not the ones from before"
    );
}

/// A file that Word cannot open is an error, and it comes without a dialog that
/// waits for a person (a hang would fail the test after 90 seconds).
#[test]
fn a_damaged_docx_gives_an_error_and_not_a_hang() {
    let _lock = exclusive();
    let before = word_pids();
    let good = DocxBuilder::new().para("Hello").build();
    let dir = TempDir::new("damaged");
    let cases: Vec<(&str, PathBuf)> = vec![
        (
            "half of a docx",
            dir.write("half.docx", &good[..good.len() / 2]),
        ),
        (
            "plain text",
            dir.write("text.docx", b"This is not a Word document."),
        ),
        // A file with 0 bytes is not here: Word opens it as an empty document.
        // Stage 1 refuses it before Word is used (it is not a zip file).
        (
            "bad XML in word/document.xml",
            dir.write(
                "bad-xml.docx",
                &zip_stored(&[
                    ("[Content_Types].xml", b"<Types/>".as_slice()),
                    ("word/document.xml", b"<w:document><w:body>".as_slice()),
                ]),
            ),
        ),
    ];

    let results = within(Duration::from_secs(90), move |session| {
        cases
            .iter()
            .map(|(what, path)| {
                let outcome = session.open_read_only(path).map(|_| ());
                (what.to_string(), outcome)
            })
            .collect::<Vec<_>>()
    });
    for (what, outcome) in results {
        match outcome {
            Err(WordError::Open { reason, .. }) => {
                assert!(!reason.is_empty(), "{what}: the reason is empty");
                println!("{what}: {reason}");
            }
            other => panic!("{what}: expected an open error, got {other:?}"),
        }
    }
    assert_eq!(wait_for_word_pids(&before), before);
}

#[test]
fn a_missing_file_is_an_error() {
    let _lock = exclusive();
    let before = word_pids();
    let result = within(Duration::from_secs(60), |session| {
        session
            .open_read_only(Path::new("C:/adapdf-no-such-folder/missing.docx"))
            .map(|_| ())
    });
    assert!(
        matches!(result, Err(WordError::FileNotFound(_))),
        "{result:?}"
    );
    assert_eq!(wait_for_word_pids(&before), before);
}

/// The session quits its Word when the code that uses it panics.
#[test]
fn the_session_quits_word_after_a_panic() {
    let _lock = exclusive();
    let before = word_pids();
    let started = AtomicU32::new(0);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let session = WordSession::start().expect("start Word");
        started.store(session.pid(), Ordering::SeqCst);
        assert!(word_pids().contains(&session.pid()));
        panic!("the code that uses Word fails");
    }));
    assert!(outcome.is_err(), "the closure panicked");
    let pid = started.load(Ordering::SeqCst);
    assert_ne!(pid, 0, "Word was started");
    let after = wait_for_word_pids(&before);
    if after.contains(&pid) {
        kill(pid);
    }
    assert_eq!(after, before, "the Word that the session started is gone");
}

/// The session quits its Word after an error, too.
#[test]
fn the_session_quits_word_after_an_error() {
    let _lock = exclusive();
    let before = word_pids();
    let result = (|| -> Result<(), WordError> {
        let session = WordSession::start()?;
        session.open_read_only(Path::new("C:/adapdf-no-such-folder/missing.docx"))?;
        Ok(())
    })();
    assert!(result.is_err());
    assert_eq!(wait_for_word_pids(&before), before);
}
