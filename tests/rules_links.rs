//! LNK-001 to LNK-003, and the severities and fixes of all the M2b rules.

mod common;

use adapdf::findings::{FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{DocxBuilder, Para, clean};
use adapdf::rules::links::{BUILT_IN_BOOKMARKS, MAX_URL_LINK_CHARS, VAGUE_LINK_TEXTS};
use common::{assert_location, fixture_findings, of, rule_findings, the_failing_finding};

const URL: &str = "https://example.org/parking";

fn with_link(text: &str) -> DocxBuilder {
    clean().paragraph(Para::new("See ").hyperlink(text, URL))
}

/// A web address of exactly `chars` characters.
fn url_of_length(chars: usize) -> String {
    let prefix = "https://example.org/";
    format!("{prefix}{}", "a".repeat(chars - prefix.len()))
}

#[test]
fn severities_and_fixes_match_spec_03() {
    use FixId::*;
    use RuleId::*;
    let expected = [
        (Lnk001, Severity::Warning, FixKind::Manual),
        (Lnk002, Severity::Warning, FixKind::Manual),
        (Lnk003, Severity::Error, FixKind::Manual),
        (Clr001, Severity::Error, FixKind::Manual),
        (Clr002, Severity::Warning, FixKind::Manual),
        (Txt001, Severity::Warning, FixKind::Manual),
        (Txt002, Severity::Warning, FixKind::Auto(AlignLeft)),
        (Txt003, Severity::Warning, FixKind::Manual),
        (Txt004, Severity::Warning, FixKind::Auto(UpdateFields)),
    ];
    for (rule, severity, fix) in expected {
        assert_eq!(rule.severity(), severity, "{rule}");
        assert_eq!(rule.fix(), fix, "{rule}");
    }
}

// ---- LNK-001 -------------------------------------------------------------

#[test]
fn lnk_001_fails_on_click_here() {
    let f = the_failing_finding(RuleId::Lnk001);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("To read the parking policy, click here"),
        Some("Test guide"),
    );
    assert!(f.message.contains("click here"), "{}", f.message);
}

#[test]
fn lnk_001_passes_with_descriptive_text() {
    assert!(of(&fixture_findings(RuleId::Lnk001, false), RuleId::Lnk001).is_empty());
}

#[test]
fn lnk_001_phrase_list_is_the_one_in_spec_03() {
    assert_eq!(
        VAGUE_LINK_TEXTS,
        ["click here", "here", "link", "more", "read more"]
    );
}

#[test]
fn lnk_001_matches_every_phrase_ignoring_case_and_punctuation() {
    for text in [
        "click here",
        "Click Here",
        "HERE",
        "here.",
        "  Link  ",
        "more",
        "Read more...",
        "(read more)",
        "click   here",
        "READ MORE!",
    ] {
        let found = rule_findings(&with_link(text), RuleId::Lnk001);
        assert_eq!(found.len(), 1, "{text:?}");
    }
}

#[test]
fn lnk_001_needs_the_whole_text_to_match() {
    for text in [
        "here is the parking policy",
        "Read more about parking",
        "the link to the policy",
        "hereby",
        "Parking policy",
        "",
    ] {
        assert!(
            rule_findings(&with_link(text), RuleId::Lnk001).is_empty(),
            "{text:?}"
        );
    }
}

#[test]
fn lnk_001_reads_hyperlink_fields() {
    let complex =
        clean().paragraph(Para::empty().field(r#"HYPERLINK "https://example.org""#, "here"));
    assert_eq!(rule_findings(&complex, RuleId::Lnk001).len(), 1);
    let simple = clean()
        .paragraph(Para::empty().simple_field(r#"HYPERLINK "https://example.org""#, "Click here"));
    assert_eq!(rule_findings(&simple, RuleId::Lnk001).len(), 1);
    let not_a_link = clean().paragraph(Para::empty().field("PAGE", "here"));
    assert!(rule_findings(&not_a_link, RuleId::Lnk001).is_empty());
}

#[test]
fn lnk_001_reports_each_link() {
    let builder = clean().paragraph(
        Para::empty()
            .hyperlink("here", URL)
            .internal_link("link", "_top"),
    );
    assert_eq!(rule_findings(&builder, RuleId::Lnk001).len(), 2);
}

// ---- LNK-002 -------------------------------------------------------------

#[test]
fn lnk_002_fails_on_a_long_address() {
    let f = the_failing_finding(RuleId::Lnk002);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    // The snippet is the first 60 characters of the paragraph.
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Visit https://example.org/departments/parking/policy-2026-fi"),
        Some("Test guide"),
    );
    assert!(f.message.contains("characters"), "{}", f.message);
}

#[test]
fn lnk_002_passes_with_a_short_address() {
    assert!(of(&fixture_findings(RuleId::Lnk002, false), RuleId::Lnk002).is_empty());
}

#[test]
fn lnk_002_threshold_is_more_than_40_characters() {
    assert_eq!(MAX_URL_LINK_CHARS, 40);
    let forty = url_of_length(40);
    let forty_one = url_of_length(41);
    assert_eq!(forty.chars().count(), 40);
    assert!(rule_findings(&with_link(&forty), RuleId::Lnk002).is_empty());
    assert_eq!(
        rule_findings(&with_link(&forty_one), RuleId::Lnk002).len(),
        1
    );
}

#[test]
fn lnk_002_recognises_http_https_and_www() {
    for start in ["http://", "https://", "HTTPS://", "www."] {
        let text = format!("{start}{}", "a".repeat(45));
        assert_eq!(
            rule_findings(&with_link(&text), RuleId::Lnk002).len(),
            1,
            "{start}"
        );
    }
}

#[test]
fn lnk_002_ignores_text_that_is_not_a_bare_address() {
    let long_words = format!("https://example.org {}", "word ".repeat(10));
    for text in [
        "example.org/departments/parking/policy-2026-final.pdf-long",
        long_words.as_str(),
        "ftp://example.org/departments/parking/policy-2026-final.pdf",
    ] {
        assert!(
            rule_findings(&with_link(text), RuleId::Lnk002).is_empty(),
            "{text:?}"
        );
    }
}

#[test]
fn lnk_002_counts_characters_not_bytes() {
    let text = format!("https://example.org/{}", "é".repeat(20));
    assert_eq!(text.chars().count(), 40);
    assert!(rule_findings(&with_link(&text), RuleId::Lnk002).is_empty());
}

// ---- LNK-003 -------------------------------------------------------------

#[test]
fn lnk_003_fails_when_the_bookmark_is_missing() {
    let f = the_failing_finding(RuleId::Lnk003);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(3),
        Some("See the setup section"),
        Some("Setup"),
    );
    assert!(f.message.contains("\"Nowhere\""), "{}", f.message);
}

#[test]
fn lnk_003_passes_when_the_bookmark_exists() {
    assert!(of(&fixture_findings(RuleId::Lnk003, false), RuleId::Lnk003).is_empty());
}

#[test]
fn lnk_003_accepts_the_built_in_top_bookmark() {
    assert_eq!(BUILT_IN_BOOKMARKS, ["_top"]);
    let builder = clean().paragraph(Para::empty().internal_link("Go to the top", "_top"));
    assert!(rule_findings(&builder, RuleId::Lnk003).is_empty());
}

#[test]
fn lnk_003_ignores_case_in_bookmark_names() {
    let builder = clean()
        .paragraph(Para::new("Setup").style("Heading2").bookmark("Setup"))
        .paragraph(Para::empty().internal_link("See setup", "SETUP"));
    assert!(rule_findings(&builder, RuleId::Lnk003).is_empty());
}

#[test]
fn lnk_003_finds_the_bookmark_anywhere_in_the_document() {
    // The bookmark comes after the link, and is in a table cell.
    let builder = clean()
        .paragraph(Para::empty().internal_link("See the table", "Later"))
        .paragraph(Para::new("Cell").in_table().bookmark("Later"));
    assert!(rule_findings(&builder, RuleId::Lnk003).is_empty());
}

#[test]
fn lnk_003_reads_the_anchor_of_a_hyperlink_field() {
    let missing = clean().paragraph(Para::empty().field(r#"HYPERLINK \l "Nowhere""#, "See below"));
    assert_eq!(rule_findings(&missing, RuleId::Lnk003).len(), 1);
    let found = clean()
        .paragraph(Para::new("Setup").style("Heading2").bookmark("Setup"))
        .paragraph(Para::empty().simple_field(r#"HYPERLINK \l "Setup""#, "See setup"));
    assert!(rule_findings(&found, RuleId::Lnk003).is_empty());
    let top = clean().paragraph(Para::empty().field(r#"HYPERLINK \l "_top""#, "Top"));
    assert!(rule_findings(&top, RuleId::Lnk003).is_empty());
}

#[test]
fn lnk_003_does_not_check_external_links() {
    let builder = clean()
        .paragraph(Para::empty().hyperlink("The policy", URL))
        .paragraph(Para::empty().field(r#"HYPERLINK "other.docx" \l "Elsewhere""#, "Other file"));
    assert!(rule_findings(&builder, RuleId::Lnk003).is_empty());
}

#[test]
fn lnk_003_the_links_in_a_table_of_contents_need_their_bookmarks() {
    let entries = [("Test guide", 1), ("Setup", 2)];
    let heading = |text: &str, level: u8, n: u32| {
        Para::new(text)
            .style(&format!("Heading{level}"))
            .bookmark(&format!("_Toc{n}"))
    };
    let ok = DocxBuilder::new()
        .toc(&entries, false)
        .paragraph(heading("Test guide", 1, 1))
        .paragraph(heading("Setup", 2, 2));
    assert!(rule_findings(&ok, RuleId::Lnk003).is_empty());
    // The second heading lost its bookmark (for example it was retyped).
    let broken = DocxBuilder::new()
        .toc(&entries, false)
        .paragraph(heading("Test guide", 1, 1))
        .paragraph(Para::new("Setup").style("Heading2"));
    let f = rule_findings(&broken, RuleId::Lnk003);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("_Toc2"), "{}", f[0].message);
}

#[test]
fn links_in_a_clean_document_make_no_findings() {
    let all = common::check(&clean());
    assert!(all.is_empty(), "{all:#?}");
}
