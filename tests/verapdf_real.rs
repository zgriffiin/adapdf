//! Tests that run the real veraPDF with the bundled Java (rows S4-10, S5-09 and
//! the real-tool part of S5-06 and S5-07).
//!
//! They are built only with the cargo feature `verapdf`:
//!
//! ```text
//! powershell -ExecutionPolicy Bypass -File scripts/fetch-vendor.ps1   # once
//! cargo test --features verapdf
//! ```
//!
//! With the feature on and `vendor/` missing, the tests fail and say so. A plain
//! `cargo test` does not build them.
#![cfg(feature = "verapdf")]

mod pdf_support;

use std::collections::BTreeSet;
use std::time::Duration;

use adapdf::findings::RuleId;
use adapdf::pdfmeta::{self, SourceMetadata};
use adapdf::verapdf::{self, ProcessRunner, Tool, VeraPdfError, WcagOutcome, validate_with};
use pdf_support::{Pdf, TITLE, TempDir, layouts};

fn tool() -> Tool {
    let tool = Tool::locate();
    if let Err(error) = tool.check() {
        panic!(
            "{error}\nThe tests of the feature `verapdf` need the bundled tools. \
             Run: powershell -ExecutionPolicy Bypass -File scripts/fetch-vendor.ps1"
        );
    }
    tool
}

fn rule_ids(validation: &verapdf::Validation) -> BTreeSet<String> {
    validation
        .ua1
        .findings
        .iter()
        .map(|f| format!("{}-{}", f.clause, f.test_number))
        .collect()
}

fn ids(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn source() -> SourceMetadata {
    SourceMetadata {
        title: Some(TITLE.to_string()),
        language: Some("en-US".to_string()),
    }
}

// S5-09
#[test]
fn verapdf_runs_on_a_generated_pdf() {
    let tool = tool();
    let dir = TempDir::new("real-generated");

    // A tagged PDF with all the metadata: 0 failures.
    let path = dir.write("complete.pdf", &Pdf::complete().build());
    let validation = verapdf::validate(&tool, &path).unwrap();
    assert!(validation.passed(), "{:#?}", validation.ua1.findings);
    assert_eq!(validation.ua1.failed_checks, 0);
    assert_eq!(validation.ua1.profile_name, "PDF/UA-1 validation profile");
    assert!(validation.ua1.passed_rules > 50);

    // The same PDF made with object streams and a cross-reference stream.
    let path = dir.write(
        "complete-modern.pdf",
        &Pdf {
            modern: true,
            ..Pdf::complete()
        }
        .build(),
    );
    assert!(verapdf::validate(&tool, &path).unwrap().passed());

    // An untagged PDF with a font that is not embedded: the failures that the
    // test knows.
    let bad = Pdf {
        marked: false,
        struct_tree: false,
        unembedded_text: true,
        ..Pdf::default()
    };
    let path = dir.write("untagged.pdf", &bad.build());
    let validation = verapdf::validate(&tool, &path).unwrap();
    assert!(!validation.passed());
    assert_eq!(validation.ua1.failed_rules, 7);
    assert_eq!(
        rule_ids(&validation),
        ids(&[
            "6.2-1",
            "7.1-3",
            "7.1-8",
            "7.1-10",
            "7.1-11",
            "7.2-34",
            "7.21.4.1-1"
        ])
    );
    let font = validation
        .ua1
        .findings
        .iter()
        .find(|f| f.clause == "7.21.4.1")
        .unwrap();
    assert_eq!(font.pre_check, Some(RuleId::Doc006));
    assert!(font.context.as_deref().unwrap().contains("Helvetica"));

    // A figure with no alternative text: the one failure, and its pre-check rule.
    let path = dir.write(
        "figure.pdf",
        &Pdf {
            figure: Some(false),
            ..Pdf::complete()
        }
        .build(),
    );
    let validation = verapdf::validate(&tool, &path).unwrap();
    assert_eq!(rule_ids(&validation), ids(&["7.3-1"]));
    assert_eq!(validation.ua1.findings[0].pre_check, Some(RuleId::Img001));

    // ... and with alternative text it passes.
    let path = dir.write(
        "figure-alt.pdf",
        &Pdf {
            figure: Some(true),
            ..Pdf::complete()
        }
        .build(),
    );
    assert!(verapdf::validate(&tool, &path).unwrap().passed());
}

#[test]
fn the_wcag_machine_profile_of_the_bundle_runs() {
    let tool = tool();
    assert!(
        tool.wcag_profile().is_some(),
        "veraPDF 1.28.2 has WCAG-2-2-Machine.xml in its profiles folder"
    );
    let dir = TempDir::new("real-wcag");
    let path = dir.write("all-missing.pdf", &Pdf::all_missing().build());
    let validation = verapdf::validate(&tool, &path).unwrap();
    // The PDF/UA-1 failures decide. The WCAG profile repeats the same rules, so
    // no warning is left after the repeats are taken out.
    assert!(!validation.passed());
    let WcagOutcome::Ran(wcag) = &validation.wcag else {
        panic!("the WCAG profile did not run: {:?}", validation.wcag);
    };
    assert_eq!(wcag.profile_name, "WCAG 2.2 machine validation profile");
    assert!(wcag.findings.is_empty(), "{:#?}", wcag.findings);
    assert!(wcag.failed_rules > 0, "it did find the missing metadata");
}

// S4-10
#[test]
fn pdfmeta_output_passes_verapdf_metadata_checks() {
    let tool = tool();
    let dir = TempDir::new("real-pdfmeta");
    for (name, original) in layouts() {
        let file = name.replace([' ', '(', ')'], "-");
        let path = dir.write(&format!("{file}.pdf"), &original);

        // Before: the metadata failures, and nothing else.
        let before = verapdf::validate(&tool, &path).unwrap();
        assert_eq!(
            rule_ids(&before),
            ids(&["5-1", "7.1-9", "7.1-10"]),
            "{name}: before"
        );

        // The fix, written into the file.
        let report = pdfmeta::fix_file(&path, &source()).unwrap();
        assert_eq!(report.changes.len(), 5, "{name}");

        // After: veraPDF reads the file and the metadata failures are gone.
        let after = verapdf::validate(&tool, &path).unwrap();
        assert!(after.passed(), "{name}: {:#?}", after.ua1.findings);
        assert_eq!(after.ua1.failed_checks, 0, "{name}");
        assert_eq!(
            after.ua1.passed_rules,
            before.ua1.passed_rules + 3,
            "{name}"
        );

        // Running the fix again changes nothing.
        let bytes = std::fs::read(&path).unwrap();
        let again = pdfmeta::fix_file(&path, &source()).unwrap();
        assert!(again.changes.is_empty(), "{name}");
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "{name}");
    }
}

#[test]
fn the_language_failure_goes_away_when_the_pdf_has_a_language() {
    // 7.2-34 needs text on the page, and the test font is not embedded, so
    // only this one rule is looked at.
    let tool = tool();
    let dir = TempDir::new("real-lang");
    let original = Pdf {
        unembedded_text: true,
        ..Pdf::complete()
    };
    let without = Pdf {
        lang: None,
        ..original.clone()
    };
    let path = dir.write("no-lang.pdf", &without.build());
    let before = verapdf::validate(&tool, &path).unwrap();
    assert!(rule_ids(&before).contains("7.2-34"));

    let report = pdfmeta::fix_file(&path, &source()).unwrap();
    assert_eq!(report.changes.len(), 1);
    let after = verapdf::validate(&tool, &path).unwrap();
    assert!(
        !rule_ids(&after).contains("7.2-34"),
        "{:?}",
        rule_ids(&after)
    );
}

// S5-07, with the real tool
#[test]
fn a_file_that_is_not_a_pdf_is_an_error_and_not_a_pass() {
    let tool = tool();
    let dir = TempDir::new("real-not-pdf");
    let path = dir.write("text.pdf", b"This is not a PDF.");
    let error = verapdf::validate(&tool, &path).unwrap_err();
    assert!(matches!(error, VeraPdfError::BadOutput(_)), "{error}");
    assert!(
        error.to_string().contains("could not read the PDF"),
        "{error}"
    );
}

#[test]
fn a_run_that_takes_too_long_is_stopped() {
    let tool = tool();
    let dir = TempDir::new("real-timeout");
    let path = dir.write("complete.pdf", &Pdf::complete().build());
    let started = std::time::Instant::now();
    let error =
        validate_with(&tool, &path, &ProcessRunner, Duration::from_millis(100)).unwrap_err();
    assert!(matches!(error, VeraPdfError::TimedOut(_)), "{error}");
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn the_tool_is_found_next_to_the_source() {
    let tool = Tool::locate();
    tool.check().unwrap();
    assert!(tool.java.ends_with("jre/bin/java.exe") || tool.java.ends_with("jre\\bin\\java.exe"));
    assert!(tool.verapdf.join("bin").is_dir());
}

#[test]
fn the_pdf_path_may_have_spaces_and_other_letters() {
    let tool = tool();
    let dir = TempDir::new("real-paths");
    let path = dir.write("Guía de usuario – año 2026.pdf", &Pdf::complete().build());
    assert!(verapdf::validate(&tool, &path).unwrap().passed());
}
