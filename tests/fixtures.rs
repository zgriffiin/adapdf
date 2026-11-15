//! The fixture files in `tests/fixtures/` against the generator.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use adapdf::findings::RuleId;
use adapdf::fixtures::{FixtureFonts, catalogue};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn every_rule_has_a_passing_and_a_failing_fixture() {
    let fixtures = catalogue();
    for rule in RuleId::ALL {
        for fails in [false, true] {
            let count = fixtures
                .iter()
                .filter(|f| f.rule == rule && f.fails == fails)
                .count();
            assert_eq!(count, 1, "{rule} fails={fails}");
        }
    }
    assert_eq!(fixtures.len(), RuleId::ALL.len() * 2);
}

#[test]
fn the_files_on_disk_match_the_generator_byte_for_byte() {
    let mut on_disk = BTreeSet::new();
    for entry in fs::read_dir(fixtures_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "docx") {
            on_disk.insert(path.file_name().unwrap().to_str().unwrap().to_string());
        }
    }
    let expected: BTreeSet<String> = catalogue().iter().map(|f| f.file_name()).collect();
    assert_eq!(
        on_disk, expected,
        "run `cargo run --example gen-fixtures` to update tests/fixtures"
    );
    for fixture in catalogue() {
        let bytes = fs::read(fixtures_dir().join(fixture.file_name())).unwrap();
        assert!(
            bytes == fixture.bytes,
            "{} differs from the generator; run `cargo run --example gen-fixtures`",
            fixture.file_name()
        );
    }
}

#[test]
fn the_generator_is_deterministic() {
    let first: Vec<_> = catalogue().into_iter().map(|f| f.bytes).collect();
    let second: Vec<_> = catalogue().into_iter().map(|f| f.bytes).collect();
    assert!(first == second);
}

#[test]
fn failing_fixtures_trigger_their_rule_and_passing_fixtures_have_no_findings() {
    for fixture in catalogue() {
        let findings = adapdf::check_bytes(&fixture.bytes, &FixtureFonts).unwrap();
        if fixture.fails {
            assert!(
                findings.iter().any(|f| f.rule_id == fixture.rule),
                "{} should trigger {}: {findings:#?}",
                fixture.file_name(),
                fixture.rule
            );
        } else {
            assert!(
                findings.is_empty(),
                "{} should have no findings: {findings:#?}",
                fixture.file_name()
            );
        }
    }
}

#[test]
fn fixtures_are_valid_zip_files_with_the_main_parts() {
    for fixture in catalogue() {
        let package = adapdf::docx::Package::from_bytes(&fixture.bytes).unwrap();
        for part in [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/document.xml",
            "word/styles.xml",
        ] {
            assert!(
                package.part(part).is_some(),
                "{} lacks {part}",
                fixture.file_name()
            );
        }
    }
}
