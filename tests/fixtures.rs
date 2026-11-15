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

/// The rule IDs in the tables of spec 03, for example `HDG-002`.
fn spec_rule_ids() -> BTreeSet<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/specs/03-precheck-rules.md");
    let spec = fs::read_to_string(path).unwrap();
    let is_rule_id = |id: &str| {
        let (letters, digits) = id.split_once('-').unwrap_or_default();
        letters.len() == 3
            && letters.chars().all(|c| c.is_ascii_uppercase())
            && digits.len() == 3
            && digits.chars().all(|c| c.is_ascii_digit())
    };
    spec.lines()
        .filter_map(|line| line.strip_prefix("| ")?.split(" |").next())
        .filter(|id| is_rule_id(id))
        .map(str::to_string)
        .collect()
}

#[test]
fn the_rules_in_the_code_are_the_rules_in_spec_03() {
    let in_code: BTreeSet<String> = RuleId::ALL.iter().map(|r| r.to_string()).collect();
    assert_eq!(in_code, spec_rule_ids());
}

/// For every finding of every fixture: the texts a person reads are there, and
/// the location texts are short.
#[test]
fn every_finding_of_every_fixture_is_complete() {
    let spec_ids = spec_rule_ids();
    let mut seen = 0;
    for fixture in catalogue() {
        let name = fixture.file_name();
        for f in adapdf::check_bytes(&fixture.bytes, &FixtureFonts).unwrap() {
            seen += 1;
            assert!(
                !f.message.trim().is_empty(),
                "{name}: {} message",
                f.rule_id
            );
            assert!(
                !f.how_to_fix.trim().is_empty(),
                "{name}: {} how_to_fix",
                f.rule_id
            );
            for (what, text) in [
                ("snippet", &f.location.snippet),
                ("heading", &f.location.heading),
            ] {
                let chars = text.as_deref().map_or(0, |t| t.chars().count());
                assert!(
                    chars <= 60,
                    "{name}: {} {what} has {chars} characters",
                    f.rule_id
                );
            }
            assert!(
                spec_ids.contains(f.rule_id.as_str()),
                "{name}: {} is not in spec 03",
                f.rule_id
            );
        }
    }
    assert!(seen > 39, "the failing fixtures give findings: {seen}");
}

/// Word needs the `id` of every `wp:docPr` in a document to be different.
#[test]
fn drawing_ids_are_unique_in_every_fixture() {
    const WP: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
    let mut documents_with_drawings = 0;
    for fixture in catalogue() {
        let package = adapdf::docx::Package::from_bytes(&fixture.bytes).unwrap();
        let text = package.text("word/document.xml").unwrap().unwrap();
        let xml = adapdf::docx::parse_xml("word/document.xml", text).unwrap();
        let ids: Vec<&str> = xml
            .descendants()
            .filter(|n| n.tag_name().name() == "docPr" && n.tag_name().namespace() == Some(WP))
            .map(|n| n.attribute("id").expect("docPr has an id"))
            .collect();
        let unique: BTreeSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "{}: {ids:?}", fixture.file_name());
        documents_with_drawings += usize::from(!ids.is_empty());
    }
    assert!(documents_with_drawings > 5);
}
