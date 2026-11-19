//! Word's limit on the length of the full path of a document (test plan rows
//! S3-22 and S3-23). Word cannot open a file whose full path is longer than
//! `WORD_PATH_LIMIT` characters, so the tool checks the path of the accessible
//! copy before it writes or opens it, and the export never gives Word the real
//! path (`tests/export_pipeline_word.rs` has the export of a deep folder).
//!
//! Run it with `cargo test --features windows_word --test word_path_limit`. The
//! test starts its own hidden Word. The `WINWORD.EXE` processes are listed with
//! `tasklist` before and after, and the two lists must be equal.
#![cfg(all(windows, feature = "windows_word"))]

mod word_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use adapdf::fix::{WORD_PATH_LIMIT, word_path_length};
use adapdf::fixtures::clean;
use adapdf::word::WordSession;
use word_support::{TempDir, exclusive, wait_for_word_pids, within, word_pids};

/// A document called `accessible.docx` whose full path has `length` characters:
/// the folder above it has a name that makes up the difference.
fn docx_with_path_of(dir: &Path, length: usize) -> PathBuf {
    let file = "accessible.docx";
    let folder_name = "p".repeat(length - word_path_length(dir) - 1 - 1 - file.len());
    let folder = dir.join(folder_name);
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join(file);
    fs::write(&path, clean().build()).unwrap();
    assert_eq!(word_path_length(&path), length);
    path
}

// S3-22: the constant is the longest path that Word opens, read-only (as the
// export does) and for editing (as a person does after "Open in Word").
#[test]
fn word_opens_the_longest_path_and_no_more() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("path-limit");
    let lengths = [WORD_PATH_LIMIT - 1, WORD_PATH_LIMIT, WORD_PATH_LIMIT + 1];
    let paths: Vec<(usize, PathBuf)> = lengths
        .iter()
        .map(|length| (*length, docx_with_path_of(dir.path(), *length)))
        .collect();

    let (build, outcomes) = within(Duration::from_secs(180), move |session: &WordSession| {
        let outcomes: Vec<(usize, bool, bool)> = paths
            .iter()
            .map(|(length, path)| {
                let read_only = session.open_read_only(path).map(|d| d.close()).is_ok();
                let editing = session.open_for_editing(path).map(|d| d.close()).is_ok();
                (*length, read_only, editing)
            })
            .collect();
        (session.build().unwrap(), outcomes)
    });
    eprintln!("Word {build}: (length, opens read-only, opens for editing) {outcomes:?}");
    assert_eq!(
        outcomes,
        [
            (WORD_PATH_LIMIT - 1, true, true),
            (WORD_PATH_LIMIT, true, true),
            (WORD_PATH_LIMIT + 1, false, false),
        ]
    );
    assert_eq!(wait_for_word_pids(&before), before);
}
