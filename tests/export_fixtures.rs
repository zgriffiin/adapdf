//! The inputs of the veraPDF exit criterion (test plan S5-10) and S3-15 for
//! them: Word exports every stage 1 pass fixture that has content and needs no
//! picture data, and each PDF is tagged. veraPDF is not run here (it is stage
//! 5); this proves that the criterion has PDFs to check.
//!
//! Run it with `cargo test --features windows_word --test export_fixtures`. It
//! starts one hidden Word for each fixture (about 6 seconds each), and never
//! touches a Word that a person has open.
#![cfg(all(windows, feature = "windows_word"))]

mod export_support;
mod word_support;

use std::fs;

use adapdf::export::{ExportOptions, export_pdf};
use adapdf::findings::RuleId::{self, *};
use adapdf::fixtures::{FixtureFonts, catalogue};
use export_support::{assert_tagged, fingerprint, names};
use word_support::{TempDir, exclusive, wait_for_word_pids, word_pids};

/// The 33 pass fixtures of S5-10: no picture, except IMG-006 (an equation).
const S5_10: [RuleId; 33] = [
    Doc001, Doc002, Doc003, Doc004, Doc005, Doc006, Hdg001, Hdg002, Hdg003, Hdg004, Hdg006, Lst001,
    Lst002, Txb001, Txb002, Txb003, Txb004, Tbl001, Tbl002, Tbl003, Tbl004, Tbl005, Tbl006, Lnk001,
    Lnk002, Lnk003, Clr001, Clr002, Txt001, Txt002, Txt003, Txt004, Img006,
];

#[test]
fn export_makes_tagged_pdfs_of_the_pass_fixtures() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s5-10-inputs");
    let mut done = Vec::new();
    for rule in S5_10 {
        let fixture = catalogue()
            .into_iter()
            .find(|f| f.rule == rule && !f.fails)
            .expect("the pass fixture exists");
        let docx = dir.write(&fixture.file_name(), &fixture.bytes);
        let pdf = docx.with_extension("pdf");
        let docx_before = fingerprint(&docx);

        let result = export_pdf(&docx, &pdf, &FixtureFonts, &ExportOptions::default())
            .unwrap_or_else(|e| panic!("{}: {e}", fixture.file_name()));
        assert!(result.findings.is_empty(), "{}", fixture.file_name());
        assert_tagged(&fixture.file_name(), &fs::read(&pdf).unwrap());
        // S3-15: the source is the same, bytes and modified time.
        assert_eq!(fingerprint(&docx), docx_before, "{}", fixture.file_name());
        done.push(rule);
    }
    assert_eq!(done.len(), 33);
    // 33 sources and 33 PDFs, and nothing else.
    assert_eq!(names(dir.path()).len(), 66);
    assert_eq!(wait_for_word_pids(&before), before);
}
