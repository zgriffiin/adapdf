//! Helpers for the export tests that need Word (`tests/export_word.rs` and
//! `tests/export_fixtures.rs`).
#![allow(dead_code)] // each test file uses a different part of this module

use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// How often `needle` is in `bytes`.
pub fn count(bytes: &[u8], needle: &[u8]) -> usize {
    bytes.windows(needle.len()).filter(|w| *w == needle).count()
}

pub fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    count(bytes, needle) > 0
}

/// Word wrote a tagged PDF: `/MarkInfo` with `/Marked true` and a
/// `/StructTreeRoot` (the check of spec 02, stage 4).
pub fn assert_tagged(name: &str, pdf: &[u8]) {
    assert!(pdf.starts_with(b"%PDF-"), "{name}: not a PDF");
    assert!(contains(pdf, b"/MarkInfo"), "{name}: no /MarkInfo");
    assert!(contains(pdf, b"/Marked true"), "{name}: not /Marked true");
    assert!(
        contains(pdf, b"/StructTreeRoot"),
        "{name}: no /StructTreeRoot"
    );
}

/// The bytes and the modified time of a file, to compare before and after.
pub fn fingerprint(path: &Path) -> (Vec<u8>, SystemTime) {
    (
        fs::read(path).unwrap(),
        fs::metadata(path).unwrap().modified().unwrap(),
    )
}

/// The names in a folder, sorted.
pub fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
