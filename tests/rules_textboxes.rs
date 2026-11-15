//! TXB-001 to TXB-004.

mod common;

use adapdf::findings::{FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{
    DocxBuilder, Image, Para, Table, clean, equation, split_runs, text_box, vml_text_box, words,
};
use adapdf::rules::textboxes::{MAX_SPACES_IN_A_ROW, MAX_TABS_IN_A_ROW};
use common::{fixture_findings, of, rule_findings, the_failing_finding};

fn count(builder: &DocxBuilder, rule: RuleId) -> usize {
    rule_findings(builder, rule).len()
}

fn with_box(run: &str) -> DocxBuilder {
    clean().paragraph(Para::new("Before.").raw_run(run))
}

// ---- TXB-001 -------------------------------------------------------------

#[test]
fn txb_001_fails_on_a_text_box_with_text() {
    let f = the_failing_finding(RuleId::Txb001);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    assert_eq!(f.location.paragraph_index, Some(2));
    assert_eq!(f.location.heading.as_deref(), Some("Test guide"));
    assert_eq!(
        f.location.snippet.as_deref(),
        Some("Note: save your work before you print.")
    );
}

#[test]
fn txb_001_passes_without_a_text_box() {
    assert!(of(&fixture_findings(RuleId::Txb001, false), RuleId::Txb001).is_empty());
}

#[test]
fn txb_001_finds_vml_text_boxes() {
    let found = rule_findings(&with_box(&vml_text_box("Note this.")), RuleId::Txb001);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].location.snippet.as_deref(), Some("Note this."));
}

#[test]
fn txb_001_the_vml_copy_in_the_fallback_is_not_a_second_text_box() {
    assert_eq!(count(&with_box(&text_box("Note this.")), RuleId::Txb001), 1);
}

#[test]
fn txb_001_a_text_box_with_no_text_is_not_reported() {
    assert_eq!(count(&with_box(&text_box("")), RuleId::Txb001), 0);
    assert_eq!(count(&with_box(&text_box("   ")), RuleId::Txb001), 0);
    assert_eq!(count(&with_box(&Image::shape().xml()), RuleId::Txb001), 0);
}

#[test]
fn txb_001_one_finding_for_each_text_box() {
    let builder = clean().paragraph(
        Para::new("Before.")
            .raw_run(&text_box("One"))
            .raw_run(&vml_text_box("Two")),
    );
    assert_eq!(count(&builder, RuleId::Txb001), 2);
}

#[test]
fn txb_001_text_in_a_text_box_is_not_body_text_for_other_rules() {
    // 500 words and a typed list marker, all inside the box.
    let boxed = format!("1. {}", words(500));
    let builder = DocxBuilder::new().paragraph(Para::new("Short.").raw_run(&text_box(&boxed)));
    assert!(rule_findings(&builder, RuleId::Hdg001).is_empty());
    assert!(rule_findings(&builder, RuleId::Lst001).is_empty());
    assert_eq!(count(&builder, RuleId::Txb001), 1);
}

/// A picture with alt text and these drawings anchored in the same paragraph.
fn screenshot_with(shapes: &[Image]) -> DocxBuilder {
    let picture = Image::picture().alt("The Save dialog with the File name box selected");
    let paragraph = shapes
        .iter()
        .fold(Para::empty().raw_run(&picture.xml()), |para, shape| {
            para.raw_run(&shape.xml())
        });
    clean().paragraph(paragraph)
}

fn boxed_arrow(text: &str) -> Image {
    Image::shape().floating().text(text)
}

const SCREENSHOT_MESSAGE: &str =
    "Text typed in a shape on a screenshot is read out of order or not at all";

#[test]
fn txb_001_text_in_shapes_on_a_screenshot_is_one_finding_for_the_screenshot() {
    let builder = screenshot_with(&[
        boxed_arrow("Click Save"),
        boxed_arrow("Then close"),
        Image::shape().floating().decorative(),
    ]);
    let found = rule_findings(&builder, RuleId::Txb001);
    assert_eq!(found.len(), 1);
    let f = &found[0];
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    assert!(f.message.contains(SCREENSHOT_MESSAGE), "{}", f.message);
    assert!(f.message.contains("Click Save"), "{}", f.message);
    assert_eq!(
        f.location.paragraph_index,
        Some(2),
        "the picture's paragraph"
    );
    assert_eq!(f.location.snippet.as_deref(), Some("Click Save Then close"));
    assert!(f.how_to_fix.contains("step text"), "{}", f.how_to_fix);
    assert!(f.how_to_fix.contains("decorative"), "{}", f.how_to_fix);
    // The shapes are not reported again by IMG-001 or IMG-004.
    assert_eq!(count(&builder, RuleId::Img001), 0);
    assert_eq!(count(&builder, RuleId::Img004), 0);
    assert_eq!(count(&builder, RuleId::Img007), 0);
}

#[test]
fn txb_001_arrows_and_text_shapes_on_one_screenshot_give_img_007_and_txb_001() {
    let builder = screenshot_with(&[
        Image::shape().floating(),
        Image::shape().floating(),
        boxed_arrow("Click Save"),
    ]);
    assert_eq!(count(&builder, RuleId::Txb001), 1);
    assert_eq!(count(&builder, RuleId::Img007), 1);
    assert_eq!(count(&builder, RuleId::Img001), 0);
}

#[test]
fn txb_001_a_decorative_shape_with_text_on_a_screenshot_is_not_reported() {
    let builder = screenshot_with(&[boxed_arrow("Click Save").decorative()]);
    assert!(common::check(&builder).is_empty());
}

#[test]
fn txb_001_vml_shapes_with_text_on_a_screenshot() {
    let builder = screenshot_with(&[Image::vml_shape().floating().text("Click Save")]);
    let found = rule_findings(&builder, RuleId::Txb001);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains(SCREENSHOT_MESSAGE));
}

#[test]
fn txb_001_text_in_shapes_far_from_a_screenshot_is_a_text_box() {
    let builder = clean()
        .paragraph(Para::empty().raw_run(&Image::picture().alt("The Save dialog").xml()))
        .para("Step text.")
        .paragraph(Para::empty().raw_run(&boxed_arrow("Click Save").xml()));
    let found = rule_findings(&builder, RuleId::Txb001);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.starts_with("A text box contains text"));
    assert_eq!(found[0].location.paragraph_index, Some(4));
}

#[test]
fn txb_001_findings_stay_in_document_order() {
    let builder = clean()
        .paragraph(Para::new("Before.").raw_run(&text_box("First")))
        .para("Filler.")
        .paragraph(
            Para::empty()
                .raw_run(&Image::picture().alt("The Save dialog").xml())
                .raw_run(&boxed_arrow("Second").xml()),
        )
        .para("Filler.")
        .paragraph(Para::new("After.").raw_run(&text_box("Third")));
    let places: Vec<_> = rule_findings(&builder, RuleId::Txb001)
        .iter()
        .map(|f| f.location.paragraph_index)
        .collect();
    assert_eq!(places, [Some(2), Some(4), Some(6)]);
}

#[test]
fn txb_001_a_flowchart_canvas_with_text_in_its_shapes_is_one_finding() {
    let canvas = Image::canvas().alt("A flowchart of the sign-in steps");
    assert!(common::check(&clean().paragraph(Para::empty().raw_run(&canvas.xml()))).is_empty());
    let with_text = clean().paragraph(Para::empty().raw_run(&canvas.text("Start").xml()));
    let found = rule_findings(&with_text, RuleId::Txb001);
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .message
            .starts_with("A group of shapes or a drawing canvas contains text"),
        "{}",
        found[0].message
    );
    assert!(
        found[0].how_to_fix.contains("picture"),
        "{}",
        found[0].how_to_fix
    );
    assert_eq!(found[0].location.snippet.as_deref(), Some("Start"));
    // The canvas has alt text and is in line with the text.
    assert_eq!(count(&with_text, RuleId::Img001), 0);
    assert_eq!(count(&with_text, RuleId::Img004), 0);
}

#[test]
fn txb_001_groups_with_text_in_their_shapes_are_reported_once() {
    for group in [Image::group(), Image::vml_group()] {
        let builder =
            clean().paragraph(Para::empty().raw_run(&group.alt("A flowchart").text("Start").xml()));
        assert_eq!(count(&builder, RuleId::Txb001), 1);
    }
}

// ---- TXB-002 -------------------------------------------------------------

#[test]
fn txb_002_fails_on_a_column_section() {
    let f = the_failing_finding(RuleId::Txb002);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    // The section holds the whole document, so it starts at the first paragraph.
    assert_eq!(f.location.paragraph_index, Some(0));
    assert!(f.message.contains("1 section "), "{}", f.message);
}

#[test]
fn txb_002_passes_with_one_column() {
    assert!(of(&fixture_findings(RuleId::Txb002, false), RuleId::Txb002).is_empty());
}

#[test]
fn txb_002_a_section_with_one_column_is_not_reported() {
    let builder = clean().paragraph(Para::new("Text.").ends_section(1));
    assert_eq!(count(&builder, RuleId::Txb002), 0);
}

#[test]
fn txb_002_is_one_finding_for_the_document_at_the_first_column_section() {
    let builder = clean()
        .paragraph(Para::new("One.").ends_section(1))
        .paragraph(Para::new("Two.").ends_section(2))
        .paragraph(Para::new("Three.").ends_section(3));
    let found = rule_findings(&builder, RuleId::Txb002);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("2 sections"),
        "{}",
        found[0].message
    );
    assert_eq!(found[0].location.snippet.as_deref(), Some("Two."));
}

// ---- TXB-003 -------------------------------------------------------------

#[test]
fn txb_003_fails_on_spacing_made_with_spaces() {
    let f = the_failing_finding(RuleId::Txb003);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_eq!(f.location.paragraph_index, Some(2));
    assert_eq!(f.location.snippet.as_deref(), Some("Name:     Ana Silva"));
}

#[test]
fn txb_003_passes_with_single_spaces() {
    assert!(of(&fixture_findings(RuleId::Txb003, false), RuleId::Txb003).is_empty());
}

#[test]
fn txb_003_threshold_is_3_spaces() {
    assert_eq!(MAX_SPACES_IN_A_ROW, 2);
    assert_eq!(count(&clean().para("a  b"), RuleId::Txb003), 0);
    assert_eq!(count(&clean().para("a   b"), RuleId::Txb003), 1);
}

#[test]
fn txb_003_threshold_is_2_tabs() {
    assert_eq!(MAX_TABS_IN_A_ROW, 1);
    let tabs = |n: usize| {
        let run = "<w:r><w:t>a</w:t>".to_string() + &"<w:tab/>".repeat(n) + "<w:t>b</w:t></w:r>";
        clean().paragraph(Para::empty().raw_run(&run))
    };
    assert_eq!(count(&tabs(1), RuleId::Txb003), 0);
    assert_eq!(count(&tabs(2), RuleId::Txb003), 1);
}

#[test]
fn txb_003_a_row_of_spaces_can_cross_runs() {
    let builder = clean().paragraph(split_runs(&["Name: ", "  ", "Ana"]));
    assert_eq!(count(&builder, RuleId::Txb003), 1);
    let two = clean().paragraph(split_runs(&["Name: ", " ", "Ana"]));
    assert_eq!(count(&two, RuleId::Txb003), 0);
}

#[test]
fn txb_003_spaces_and_tabs_are_counted_separately() {
    assert_eq!(count(&clean().para("a \t \t b  c"), RuleId::Txb003), 0);
}

#[test]
fn txb_003_a_paragraph_with_only_spaces_is_not_reported() {
    assert_eq!(count(&clean().para("      "), RuleId::Txb003), 0);
}

#[test]
fn txb_003_one_finding_for_each_paragraph() {
    let builder = clean().para("a     b          c");
    assert_eq!(count(&builder, RuleId::Txb003), 1);
}

// ---- TXB-004 -------------------------------------------------------------

fn gaps(empties: usize) -> DocxBuilder {
    let mut builder = clean().para("Before.");
    for _ in 0..empties {
        builder = builder.paragraph(Para::empty());
    }
    builder.para("After.")
}

#[test]
fn txb_004_fails_on_two_empty_paragraphs_in_a_row() {
    let f = the_failing_finding(RuleId::Txb004);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Auto(FixId::DeleteEmptyParagraphs));
    assert_eq!(f.location.paragraph_index, Some(3), "the first empty one");
    assert_eq!(f.location.heading.as_deref(), Some("Test guide"));
    assert!(f.message.contains("2 empty"), "{}", f.message);
}

#[test]
fn txb_004_passes_with_one_empty_paragraph() {
    assert!(of(&fixture_findings(RuleId::Txb004, false), RuleId::Txb004).is_empty());
}

#[test]
fn txb_004_threshold_is_2_empty_paragraphs() {
    assert_eq!(count(&gaps(0), RuleId::Txb004), 0);
    assert_eq!(count(&gaps(1), RuleId::Txb004), 0);
    assert_eq!(count(&gaps(2), RuleId::Txb004), 1);
}

#[test]
fn txb_004_a_long_row_is_one_finding_and_each_row_is_reported() {
    let long = rule_findings(&gaps(5), RuleId::Txb004);
    assert_eq!(long.len(), 1);
    assert!(long[0].message.contains("5 empty"), "{}", long[0].message);
    let two_rows = clean()
        .paragraph(Para::empty())
        .paragraph(Para::empty())
        .para("Text.")
        .paragraph(Para::empty())
        .paragraph(Para::empty())
        .paragraph(Para::empty());
    let places: Vec<_> = rule_findings(&two_rows, RuleId::Txb004)
        .iter()
        .map(|f| f.location.paragraph_index)
        .collect();
    assert_eq!(places, [Some(2), Some(5)]);
}

#[test]
fn txb_004_spaces_and_line_breaks_are_empty() {
    let builder = clean()
        .para("   ")
        .paragraph(Para::empty().raw_run("<w:r><w:br/></w:r>"));
    assert_eq!(count(&builder, RuleId::Txb004), 1);
}

#[test]
fn txb_004_a_picture_break_field_or_equation_is_not_empty() {
    let items = [
        r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="P"/></wp:inline></w:drawing></w:r>"#
            .to_string(),
        r#"<w:r><w:br w:type="page"/></w:r>"#.to_string(),
        r#"<w:r><w:br w:type="column"/></w:r>"#.to_string(),
        r#"<w:fldSimple w:instr="PAGE"/>"#.to_string(),
        r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r>"#.to_string(),
        equation("a=b"),
    ];
    for item in &items {
        let item_para = || Para::empty().raw_run(item);
        let two_items = clean().paragraph(item_para()).paragraph(item_para());
        assert_eq!(count(&two_items, RuleId::Txb004), 0, "{item}");
        let between = clean()
            .paragraph(Para::empty())
            .paragraph(item_para())
            .paragraph(Para::empty());
        assert_eq!(count(&between, RuleId::Txb004), 0, "{item}");
    }
}

#[test]
fn txb_004_a_section_break_is_not_empty() {
    let builder = clean()
        .paragraph(Para::empty())
        .paragraph(Para::empty().ends_section(1))
        .paragraph(Para::empty());
    assert_eq!(count(&builder, RuleId::Txb004), 0);
    let before_break = clean()
        .paragraph(Para::empty())
        .paragraph(Para::empty())
        .paragraph(Para::empty().ends_section(1));
    assert_eq!(count(&before_break, RuleId::Txb004), 1);
}

#[test]
fn txb_004_empty_table_cells_and_paragraphs_around_a_table_are_not_a_row() {
    let table = Table::new(&[&["", ""], &["", ""]]);
    assert_eq!(count(&clean().table(table.clone()), RuleId::Txb004), 0);
    let around = clean()
        .paragraph(Para::empty())
        .table(table)
        .paragraph(Para::empty());
    assert_eq!(count(&around, RuleId::Txb004), 0);
}
