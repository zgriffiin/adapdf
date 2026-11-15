//! HDG-001 to HDG-006.

mod common;

use adapdf::findings::{FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{DocxBuilder, Para, Run, clean, words};
use adapdf::rules::headings::{FAKE_HEADING_MAX_WORDS, PAGE_WORD_LIMIT};
use common::{assert_location, fixture_findings, of, rule_findings, the_failing_finding};

// ---- HDG-001 -------------------------------------------------------------

#[test]
fn hdg_001_fails_on_a_long_document_without_headings() {
    let f = the_failing_finding(RuleId::Hdg001);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(&f, "word/document.xml", None, None, None);
    assert!(f.message.contains("401 words"), "{}", f.message);
}

#[test]
fn hdg_001_passes_with_a_heading() {
    assert!(of(&fixture_findings(RuleId::Hdg001, false), RuleId::Hdg001).is_empty());
}

#[test]
fn hdg_001_threshold_is_more_than_400_words() {
    assert_eq!(PAGE_WORD_LIMIT, 400);
    let exactly = DocxBuilder::new().para(&words(400));
    let over = DocxBuilder::new().para(&words(401));
    assert!(rule_findings(&exactly, RuleId::Hdg001).is_empty());
    assert_eq!(rule_findings(&over, RuleId::Hdg001).len(), 1);
}

#[test]
fn hdg_001_counts_words_over_all_paragraphs() {
    let split = DocxBuilder::new().para(&words(200)).para(&words(201));
    assert_eq!(rule_findings(&split, RuleId::Hdg001).len(), 1);
    let table = DocxBuilder::new().paragraph(Para::new(&words(401)).in_table());
    assert_eq!(rule_findings(&table, RuleId::Hdg001).len(), 1);
}

#[test]
fn hdg_001_title_and_empty_headings_are_not_headings() {
    let title = DocxBuilder::new()
        .paragraph(Para::new("Guide").style("Title"))
        .para(&words(401));
    assert_eq!(rule_findings(&title, RuleId::Hdg001).len(), 1);
    let empty_heading = DocxBuilder::new().heading(1, "").para(&words(401));
    assert_eq!(rule_findings(&empty_heading, RuleId::Hdg001).len(), 1);
}

// ---- HDG-002 -------------------------------------------------------------

#[test]
fn hdg_002_fails_when_a_level_is_skipped() {
    let f = the_failing_finding(RuleId::Hdg002);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(&f, "word/document.xml", Some(1), Some("Three"), Some("One"));
    assert!(f.message.contains("Heading 3"), "{}", f.message);
    assert!(f.message.contains("Heading 2 is missing"), "{}", f.message);
}

#[test]
fn hdg_002_passes_for_levels_in_order() {
    assert!(of(&fixture_findings(RuleId::Hdg002, false), RuleId::Hdg002).is_empty());
}

#[test]
fn hdg_002_the_first_heading_must_be_heading_1() {
    let f = rule_findings(
        &DocxBuilder::new().heading(2, "Start").heading(3, "Next"),
        RuleId::Hdg002,
    );
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].location.paragraph_index, Some(0));
    assert!(f[0].message.contains("first heading"), "{}", f[0].message);
}

#[test]
fn hdg_002_going_back_up_is_not_a_skip() {
    let builder = DocxBuilder::new()
        .heading(1, "a")
        .heading(2, "b")
        .heading(3, "c")
        .heading(1, "d")
        .heading(2, "e")
        .heading(2, "f");
    assert!(rule_findings(&builder, RuleId::Hdg002).is_empty());
}

#[test]
fn hdg_002_reports_each_skip() {
    let builder = DocxBuilder::new()
        .heading(1, "a")
        .heading(3, "b")
        .heading(4, "c")
        .heading(1, "d")
        .heading(4, "e");
    let places: Vec<_> = rule_findings(&builder, RuleId::Hdg002)
        .iter()
        .map(|f| f.location.paragraph_index)
        .collect();
    assert_eq!(places, [Some(1), Some(4)]);
}

#[test]
fn hdg_002_ignores_empty_headings() {
    let builder = DocxBuilder::new()
        .heading(1, "a")
        .heading(3, "")
        .heading(2, "b");
    assert!(rule_findings(&builder, RuleId::Hdg002).is_empty());
}

// ---- HDG-003 -------------------------------------------------------------

#[test]
fn hdg_003_fails_on_a_bold_line_without_a_heading_style() {
    let f = the_failing_finding(RuleId::Hdg003);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Auto(FixId::ApplyHeadingStyle));
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Setting up the printer"),
        Some("Test guide"),
    );
    assert_eq!(
        f.suggestion.as_deref(),
        Some("Heading 2"),
        "one level below Heading 1"
    );
}

#[test]
fn hdg_003_passes_when_the_line_has_a_heading_style() {
    assert!(of(&fixture_findings(RuleId::Hdg003, false), RuleId::Hdg003).is_empty());
}

#[test]
fn hdg_003_threshold_is_12_words() {
    assert_eq!(FAKE_HEADING_MAX_WORDS, 12);
    let twelve = DocxBuilder::new().paragraph(Para::new(&words(12)).bold());
    let thirteen = DocxBuilder::new().paragraph(Para::new(&words(13)).bold());
    assert_eq!(rule_findings(&twelve, RuleId::Hdg003).len(), 1);
    assert!(rule_findings(&thirteen, RuleId::Hdg003).is_empty());
}

#[test]
fn hdg_003_larger_than_the_body_text_counts_too() {
    let larger = DocxBuilder::new()
        .body_size(22)
        .paragraph(Para::new("Big line").size(28));
    let same = DocxBuilder::new()
        .body_size(22)
        .paragraph(Para::new("Same line").size(22));
    let smaller = DocxBuilder::new()
        .body_size(22)
        .paragraph(Para::new("Small line").size(18));
    assert_eq!(rule_findings(&larger, RuleId::Hdg003).len(), 1);
    assert!(rule_findings(&same, RuleId::Hdg003).is_empty());
    assert!(rule_findings(&smaller, RuleId::Hdg003).is_empty());
}

#[test]
fn hdg_003_all_of_the_text_must_be_bold() {
    let all = DocxBuilder::new().paragraph(
        Para::empty()
            .then(Run::new("Setting ").bold())
            .then(Run::new("up").bold()),
    );
    let part = DocxBuilder::new().paragraph(
        Para::empty()
            .then(Run::new("Setting ").bold())
            .then(Run::new("up")),
    );
    let spaces_do_not_count = DocxBuilder::new().paragraph(
        Para::empty()
            .then(Run::new("Setting").bold())
            .then(Run::new("   ")),
    );
    assert_eq!(rule_findings(&all, RuleId::Hdg003).len(), 1);
    assert!(rule_findings(&part, RuleId::Hdg003).is_empty());
    assert_eq!(rule_findings(&spaces_do_not_count, RuleId::Hdg003).len(), 1);
}

#[test]
fn hdg_003_skips_headings_tables_lists_and_special_styles() {
    let builder = DocxBuilder::new()
        .paragraph(Para::new("A real heading").style("Heading1").bold())
        .paragraph(Para::new("Header cell").bold().in_table())
        .paragraph(Para::new("List item").bold().style("ListNumber"))
        .paragraph(Para::new("Direct list item").bold().num(2, 0))
        .paragraph(Para::new("Guide title").bold().style("Title"))
        .paragraph(Para::empty())
        .paragraph(Para::new("   ").bold());
    assert!(rule_findings(&builder, RuleId::Hdg003).is_empty());
}

#[test]
fn hdg_003_skips_the_subtitle_caption_and_toc_styles() {
    // (style id, style name as Word writes it)
    let styles = [
        ("Subtitle", "Subtitle"),
        ("Caption", "caption"),
        ("TOC1", "toc 1"),
        ("TOC2", "toc 2"),
    ];
    for (id, name) in styles {
        let builder = DocxBuilder::new()
            .custom_style(id, name, None)
            .paragraph(Para::new("Short bold line").bold().style(id));
        assert!(
            rule_findings(&builder, RuleId::Hdg003).is_empty(),
            "{name} is exempt"
        );
    }
    // The same line in a style that is not exempt is reported.
    let builder = DocxBuilder::new()
        .custom_style("Callout", "Callout", None)
        .paragraph(Para::new("Short bold line").bold().style("Callout"));
    assert_eq!(rule_findings(&builder, RuleId::Hdg003).len(), 1);
}

#[test]
fn hdg_003_suggests_a_level_below_the_heading_above() {
    let suggestion = |builder: DocxBuilder| {
        rule_findings(&builder, RuleId::Hdg003)[0]
            .suggestion
            .clone()
            .unwrap()
    };
    let fake = || Para::new("Fake").bold();
    assert_eq!(
        suggestion(DocxBuilder::new().paragraph(fake())),
        "Heading 1"
    );
    assert_eq!(
        suggestion(
            DocxBuilder::new()
                .heading(1, "a")
                .heading(2, "b")
                .paragraph(fake())
        ),
        "Heading 3"
    );
    assert_eq!(
        suggestion(
            DocxBuilder::new()
                .heading(1, "a")
                .heading(2, "b")
                .heading(3, "c")
                .heading(4, "d")
                .paragraph(fake())
        ),
        "Heading 4",
        "never deeper than Heading 4"
    );
}

// ---- HDG-004 -------------------------------------------------------------

#[test]
fn hdg_004_fails_on_an_empty_heading() {
    let f = the_failing_finding(RuleId::Hdg004);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Auto(FixId::DeleteEmptyHeading));
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some(""),
        Some("Test guide"),
    );
}

#[test]
fn hdg_004_passes_when_the_heading_has_text() {
    assert!(of(&fixture_findings(RuleId::Hdg004, false), RuleId::Hdg004).is_empty());
}

#[test]
fn hdg_004_spaces_are_empty_but_a_picture_is_content() {
    let spaces = DocxBuilder::new().heading(2, "   ");
    let picture = DocxBuilder::new().paragraph(
        Para::empty()
            .style("Heading2")
            .raw_run(r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="Picture"/></wp:inline></w:drawing></w:r>"#),
    );
    assert_eq!(rule_findings(&spaces, RuleId::Hdg004).len(), 1);
    assert!(rule_findings(&picture, RuleId::Hdg004).is_empty());
}

#[test]
fn hdg_004_only_looks_at_headings() {
    let builder = DocxBuilder::new()
        .paragraph(Para::empty())
        .paragraph(Para::new(""));
    assert!(rule_findings(&builder, RuleId::Hdg004).is_empty());
}

// ---- HDG-006 -------------------------------------------------------------

#[test]
fn hdg_006_fails_on_a_custom_style_with_an_outline_level() {
    let f = the_failing_finding(RuleId::Hdg006);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    // The style is defined in styles.xml; the location is the first paragraph
    // that uses it.
    assert_location(
        &f,
        "word/styles.xml",
        Some(2),
        Some("Look here."),
        Some("Test guide"),
    );
    assert!(f.message.contains("\"Callout\""), "{}", f.message);
    assert!(f.message.contains("outline level 2"), "{}", f.message);
}

#[test]
fn hdg_006_passes_when_the_custom_style_has_no_outline_level() {
    assert!(of(&fixture_findings(RuleId::Hdg006, false), RuleId::Hdg006).is_empty());
}

#[test]
fn hdg_006_reports_a_style_once_and_only_when_it_is_used() {
    let used_three_times = DocxBuilder::new()
        .custom_style("Callout", "Callout", Some(0))
        .paragraph(Para::new("a").style("Callout"))
        .paragraph(Para::new("b").style("Callout"))
        .paragraph(Para::new("c").style("Callout"));
    let f = rule_findings(&used_three_times, RuleId::Hdg006);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].location.paragraph_index, Some(0));

    let unused = DocxBuilder::new()
        .custom_style("Callout", "Callout", Some(0))
        .para("text");
    assert!(rule_findings(&unused, RuleId::Hdg006).is_empty());
}

#[test]
fn hdg_006_built_in_headings_and_body_text_outline_level_are_fine() {
    let builder = DocxBuilder::new()
        .heading(1, "Built in")
        .custom_style("Plain", "Plain", Some(9)) // 9 means "body text"
        .paragraph(Para::new("x").style("Plain"));
    assert!(rule_findings(&builder, RuleId::Hdg006).is_empty());
}

#[test]
fn heading_rules_are_quiet_on_a_clean_document() {
    let findings = common::check(&clean());
    assert!(findings.is_empty(), "{findings:#?}");
}
