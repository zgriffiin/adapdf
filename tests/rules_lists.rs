//! LST-001 and LST-002.

mod common;

use adapdf::findings::{FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{DocxBuilder, Para};
use common::{fixture_findings, of, rule_findings};

fn step(text: &str) -> Para {
    Para::new(text).style("ListNumber")
}

fn typed(text: &str) -> DocxBuilder {
    DocxBuilder::new().para(text)
}

// ---- LST-001 -------------------------------------------------------------

#[test]
fn lst_001_fails_on_typed_numbers() {
    let findings = fixture_findings(RuleId::Lst001, true);
    let f = of(&findings, RuleId::Lst001);
    assert_eq!(f.len(), 2, "one finding for each typed paragraph");
    assert_eq!(f[0].severity, Severity::Error);
    assert_eq!(f[0].fix, FixKind::Auto(FixId::ApplyListStyle));
    assert_eq!(f[0].location.paragraph_index, Some(2));
    assert_eq!(f[1].location.paragraph_index, Some(3));
    assert_eq!(f[0].location.snippet.as_deref(), Some("1. Open the file."));
    assert_eq!(f[0].location.heading.as_deref(), Some("Test guide"));
    assert!(
        f[0].message.contains("typed number (\"1.\")"),
        "{}",
        f[0].message
    );
}

#[test]
fn lst_001_passes_for_a_real_list() {
    assert!(of(&fixture_findings(RuleId::Lst001, false), RuleId::Lst001).is_empty());
}

#[test]
fn lst_001_finds_each_kind_of_typed_marker() {
    for text in [
        "1. Step", "1) Step", "12. Step", "a. Step", "a) Step", "B. Step", "• Item", "- Item",
        "* Item", "·\tItem", "1.\tStep",
    ] {
        assert_eq!(
            rule_findings(&typed(text), RuleId::Lst001).len(),
            1,
            "{text:?}"
        );
    }
}

#[test]
fn lst_001_leaves_ordinary_text_alone() {
    for text in [
        "1.5 million people",
        "2024. A long year",
        "Item 1. Open",
        "-5 degrees",
        "e.g. this",
        "Step 1",
        "1.",
        "-",
    ] {
        assert!(
            rule_findings(&typed(text), RuleId::Lst001).is_empty(),
            "{text:?}"
        );
    }
}

#[test]
fn lst_001_says_bullet_for_a_typed_bullet() {
    let f = rule_findings(&typed("• Item"), RuleId::Lst001);
    assert!(f[0].message.contains("typed bullet"), "{}", f[0].message);
}

#[test]
fn lst_001_ignores_paragraphs_that_are_real_list_items() {
    let builder = DocxBuilder::new()
        .paragraph(step("1. Typed number in a real list item"))
        .paragraph(Para::new("• Typed bullet in a real list item").num(2, 0));
    assert!(rule_findings(&builder, RuleId::Lst001).is_empty());
}

#[test]
fn lst_001_ignores_typed_numbers_in_headings() {
    let builder = DocxBuilder::new().heading(1, "1. Overview");
    assert!(rule_findings(&builder, RuleId::Lst001).is_empty());
}

#[test]
fn lst_001_sees_symbol_font_bullets() {
    let builder = DocxBuilder::new().paragraph(Para::empty().raw_run(
        r#"<w:r><w:sym w:font="Symbol" w:char="F0B7"/><w:t xml:space="preserve"> Item</w:t></w:r>"#,
    ));
    assert_eq!(rule_findings(&builder, RuleId::Lst001).len(), 1);
}

#[test]
fn lst_001_finds_typed_markers_in_table_cells() {
    let builder = DocxBuilder::new().paragraph(Para::new("1. Cell step").in_table());
    assert_eq!(rule_findings(&builder, RuleId::Lst001).len(), 1);
}

// ---- LST-002 -------------------------------------------------------------

#[test]
fn lst_002_fails_when_a_list_mixes_typed_and_automatic_numbers() {
    let findings = fixture_findings(RuleId::Lst002, true);
    let f = of(&findings, RuleId::Lst002);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].severity, Severity::Warning);
    assert_eq!(f[0].fix, FixKind::Manual);
    assert_eq!(
        f[0].location.paragraph_index,
        Some(4),
        "at the typed number"
    );
    assert_eq!(f[0].location.snippet.as_deref(), Some("3. Close the file."));
}

#[test]
fn lst_002_passes_for_one_kind_of_numbering() {
    assert!(of(&fixture_findings(RuleId::Lst002, false), RuleId::Lst002).is_empty());
}

#[test]
fn lst_002_typed_numbers_alone_are_not_a_mix() {
    let builder = DocxBuilder::new().para("1. One").para("2. Two");
    assert!(rule_findings(&builder, RuleId::Lst002).is_empty());
}

#[test]
fn lst_002_typed_first_then_automatic_is_a_mix() {
    let builder = DocxBuilder::new().para("1. One").paragraph(step("Two"));
    let f = rule_findings(&builder, RuleId::Lst002);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].location.paragraph_index, Some(0));
}

#[test]
fn lst_002_bullets_are_not_numbers() {
    let builder = DocxBuilder::new()
        .paragraph(Para::new("Dot").style("ListBullet"))
        .para("1. Typed number")
        .paragraph(Para::new("Dot").style("ListBullet"))
        .para("• Typed bullet");
    assert!(rule_findings(&builder, RuleId::Lst002).is_empty());
}

#[test]
fn lst_002_another_paragraph_ends_the_list() {
    let builder = DocxBuilder::new()
        .paragraph(step("One"))
        .paragraph(step("Two"))
        .para("A paragraph between the lists.")
        .para("3. Typed number in a new list");
    assert!(rule_findings(&builder, RuleId::Lst002).is_empty());
}

#[test]
fn lst_002_reports_each_mixed_list_once_including_a_list_at_the_end() {
    let builder = DocxBuilder::new()
        .paragraph(step("One"))
        .para("2. Two")
        .para("3. Three")
        .para("Break")
        .para("1. One")
        .paragraph(step("Two"));
    let places: Vec<_> = rule_findings(&builder, RuleId::Lst002)
        .iter()
        .map(|f| f.location.paragraph_index)
        .collect();
    assert_eq!(places, [Some(1), Some(4)]);
}

#[test]
fn lst_002_headings_with_typed_numbers_are_not_list_items() {
    let builder = DocxBuilder::new()
        .paragraph(step("One"))
        .heading(2, "2. Not a list item")
        .paragraph(step("Three"));
    assert!(rule_findings(&builder, RuleId::Lst002).is_empty());
}
