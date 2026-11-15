//! IMG-001 to IMG-006.

mod common;

use adapdf::findings::{FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{
    DocxBuilder, Image, Para, alt_of_length, clean, equation, ole_object, text_box,
};
use adapdf::rules::images::MAX_ALT_TEXT_CHARS;
use common::{fixture_findings, of, rule_findings, the_failing_finding};

/// A clean document with one paragraph that holds only this drawing.
fn with(image: &Image) -> DocxBuilder {
    clean().paragraph(Para::empty().raw_run(&image.xml()))
}

fn count(builder: &DocxBuilder, rule: RuleId) -> usize {
    rule_findings(builder, rule).len()
}

// ---- IMG-001 -------------------------------------------------------------

#[test]
fn img_001_fails_on_a_picture_without_alt_text() {
    let f = the_failing_finding(RuleId::Img001);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Ask(FixId::SetAltText));
    assert_eq!(f.location.part, "word/document.xml");
    assert_eq!(f.location.paragraph_index, Some(2));
    assert_eq!(f.location.heading.as_deref(), Some("Test guide"));
    assert!(f.message.contains("picture"), "{}", f.message);
    assert_eq!(f.suggestion, None, "the tool never invents alt text");
}

#[test]
fn img_001_passes_with_alt_text() {
    assert!(of(&fixture_findings(RuleId::Img001, false), RuleId::Img001).is_empty());
}

#[test]
fn img_001_a_decorative_image_with_no_alt_text_passes() {
    assert_eq!(count(&with(&Image::picture()), RuleId::Img001), 1);
    assert_eq!(
        count(&with(&Image::picture().decorative()), RuleId::Img001),
        0
    );
    assert_eq!(
        count(
            &with(&Image::picture().alt("A cat").decorative()),
            RuleId::Img001
        ),
        0
    );
}

#[test]
fn img_001_covers_charts_shapes_and_smart_art() {
    for (image, name) in [
        (Image::chart(), "chart"),
        (Image::shape(), "shape"),
        (Image::smart_art(), "SmartArt"),
    ] {
        let found = rule_findings(&with(&image), RuleId::Img001);
        assert_eq!(found.len(), 1, "{name}");
        assert!(found[0].message.contains(name), "{}", found[0].message);
        assert_eq!(
            count(&with(&image.clone().alt("Sales by year")), RuleId::Img001),
            0
        );
        assert_eq!(count(&with(&image.decorative()), RuleId::Img001), 0);
    }
}

#[test]
fn img_001_vml_pictures_use_the_alt_attribute_and_the_title() {
    assert_eq!(count(&with(&Image::vml_picture()), RuleId::Img001), 1);
    assert_eq!(
        count(&with(&Image::vml_picture().alt("A cat")), RuleId::Img001),
        0
    );
    assert_eq!(
        count(&with(&Image::vml_picture().title("A cat")), RuleId::Img001),
        0
    );
    assert_eq!(count(&with(&Image::vml_shape()), RuleId::Img001), 1);
    assert_eq!(
        count(&with(&Image::vml_shape().alt("A box")), RuleId::Img001),
        0
    );
}

#[test]
fn img_001_the_title_counts_when_there_is_no_description() {
    assert_eq!(
        count(&with(&Image::picture().title("A cat")), RuleId::Img001),
        0
    );
}

#[test]
fn img_001_alt_text_of_only_spaces_is_no_alt_text() {
    assert_eq!(
        count(&with(&Image::picture().alt("   ")), RuleId::Img001),
        1
    );
}

#[test]
fn img_001_one_finding_for_each_drawing() {
    let builder = clean().paragraph(
        Para::empty()
            .raw_run(&Image::picture().xml())
            .raw_run(&Image::picture().alt("A cat").xml())
            .raw_run(&Image::chart().xml()),
    );
    assert_eq!(count(&builder, RuleId::Img001), 2);
}

#[test]
fn img_001_does_not_ask_for_alt_text_on_text_boxes_or_embedded_objects() {
    let boxed = clean().paragraph(Para::new("Before.").raw_run(&text_box("Note this.")));
    assert_eq!(count(&boxed, RuleId::Img001), 0);
    let ole = clean().paragraph(Para::empty().raw_run(&ole_object()));
    assert_eq!(count(&ole, RuleId::Img001), 0);
}

// ---- IMG-002 -------------------------------------------------------------

#[test]
fn img_002_fails_on_alt_text_that_looks_like_a_file_name() {
    let f = the_failing_finding(RuleId::Img002);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Ask(FixId::SetAltText));
    assert_eq!(f.location.paragraph_index, Some(2));
    assert!(f.message.contains("IMG_1234.jpg"), "{}", f.message);
    assert_eq!(f.suggestion, None);
}

#[test]
fn img_002_passes_with_a_real_description() {
    assert!(of(&fixture_findings(RuleId::Img002, false), RuleId::Img002).is_empty());
}

#[test]
fn img_002_placeholders_are_reported_and_descriptions_are_not() {
    for alt in [
        "image",
        "Picture.",
        " SCREENSHOT ",
        "graphic",
        "photo",
        "Figure",
        "IMG_1234",
        "DSC00123",
        "Screenshot 2024-03-05 101530",
        "Picture 1",
        "setup.png",
        "photo.JPEG",
    ] {
        assert_eq!(
            count(&with(&Image::picture().alt(alt)), RuleId::Img002),
            1,
            "{alt}"
        );
    }
    for alt in [
        "Screenshot of the login page",
        "Image 3 shows the printer",
        "A picture of a cat",
        "Front panel of the printer",
    ] {
        assert_eq!(
            count(&with(&Image::picture().alt(alt)), RuleId::Img002),
            0,
            "{alt}"
        );
    }
}

#[test]
fn img_002_checks_vml_alt_text_and_the_title_fallback() {
    assert_eq!(
        count(&with(&Image::vml_picture().alt("image")), RuleId::Img002),
        1
    );
    assert_eq!(
        count(&with(&Image::picture().title("Picture 1")), RuleId::Img002),
        1
    );
}

#[test]
fn img_002_a_decorative_image_has_no_alt_text_to_check() {
    let image = Image::picture().alt("image").decorative();
    assert_eq!(count(&with(&image), RuleId::Img002), 0);
}

// ---- IMG-003 -------------------------------------------------------------

#[test]
fn img_003_fails_on_alt_text_longer_than_250_characters() {
    let f = the_failing_finding(RuleId::Img003);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_eq!(f.location.paragraph_index, Some(2));
    assert!(f.message.contains("251"), "{}", f.message);
}

#[test]
fn img_003_passes_at_250_characters() {
    assert!(of(&fixture_findings(RuleId::Img003, false), RuleId::Img003).is_empty());
}

#[test]
fn img_003_threshold_is_more_than_250_characters() {
    assert_eq!(MAX_ALT_TEXT_CHARS, 250);
    let alt = |n| with(&Image::picture().alt(&alt_of_length(n)));
    assert_eq!(count(&alt(250), RuleId::Img003), 0);
    assert_eq!(count(&alt(251), RuleId::Img003), 1);
}

#[test]
fn img_003_counts_characters_not_bytes_and_ignores_outer_spaces() {
    let alt = |text: String| with(&Image::picture().alt(&text));
    assert_eq!(count(&alt("é".repeat(250)), RuleId::Img003), 0);
    assert_eq!(count(&alt("é".repeat(251)), RuleId::Img003), 1);
    assert_eq!(
        count(&alt(format!("  {}  ", "x".repeat(250))), RuleId::Img003),
        0
    );
}

#[test]
fn img_003_checks_vml_alt_text() {
    let long = Image::vml_picture().alt(&alt_of_length(251));
    assert_eq!(count(&with(&long), RuleId::Img003), 1);
}

// ---- IMG-004 -------------------------------------------------------------

#[test]
fn img_004_fails_on_a_floating_picture() {
    let f = the_failing_finding(RuleId::Img004);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Auto(FixId::MakeImageInline));
    assert_eq!(f.location.paragraph_index, Some(2));
}

#[test]
fn img_004_passes_for_an_inline_picture() {
    assert!(of(&fixture_findings(RuleId::Img004, false), RuleId::Img004).is_empty());
}

#[test]
fn img_004_vml_pictures_float_when_the_style_says_absolute() {
    assert_eq!(
        count(&with(&Image::vml_picture().floating()), RuleId::Img004),
        1
    );
    assert_eq!(count(&with(&Image::vml_picture()), RuleId::Img004), 0);
}

#[test]
fn img_004_checks_pictures_only() {
    assert_eq!(count(&with(&Image::chart().floating()), RuleId::Img004), 0);
    assert_eq!(count(&with(&Image::shape().floating()), RuleId::Img004), 0);
    assert_eq!(
        count(&with(&Image::smart_art().floating()), RuleId::Img004),
        0
    );
    let boxed = clean().paragraph(Para::new("Before.").raw_run(&text_box("Note this.")));
    assert_eq!(count(&boxed, RuleId::Img004), 0);
}

// ---- IMG-005 -------------------------------------------------------------

fn captioned(alt: &str, before: Option<&str>, after: Option<&str>) -> DocxBuilder {
    let mut builder = clean().custom_style("Caption", "Caption", None);
    if let Some(text) = before {
        builder = builder.paragraph(Para::new(text).style("Caption"));
    }
    builder = builder.paragraph(Para::empty().raw_run(&Image::picture().alt(alt).xml()));
    if let Some(text) = after {
        builder = builder.paragraph(Para::new(text).style("Caption"));
    }
    builder
}

#[test]
fn img_005_fails_when_the_alt_text_is_the_caption() {
    let f = the_failing_finding(RuleId::Img005);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_eq!(f.location.paragraph_index, Some(2));
    assert!(f.message.contains("Figure 1: the printer"), "{}", f.message);
}

#[test]
fn img_005_passes_when_they_differ() {
    assert!(of(&fixture_findings(RuleId::Img005, false), RuleId::Img005).is_empty());
}

#[test]
fn img_005_the_caption_can_be_before_or_after() {
    assert_eq!(
        count(
            &captioned("The printer", None, Some("The printer")),
            RuleId::Img005
        ),
        1
    );
    assert_eq!(
        count(
            &captioned("The printer", Some("The printer"), None),
            RuleId::Img005
        ),
        1
    );
}

#[test]
fn img_005_ignores_case_spaces_and_end_punctuation() {
    let builder = captioned("the  printer", None, Some("  The Printer. "));
    assert_eq!(count(&builder, RuleId::Img005), 1);
}

#[test]
fn img_005_only_a_caption_style_paragraph_next_to_the_image_counts() {
    let plain = clean()
        .paragraph(Para::empty().raw_run(&Image::picture().alt("The printer").xml()))
        .para("The printer");
    assert_eq!(count(&plain, RuleId::Img005), 0);
    let far = captioned("The printer", None, Some("Another caption"))
        .paragraph(Para::new("The printer").style("Caption"));
    assert_eq!(count(&far, RuleId::Img005), 0);
}

#[test]
fn img_005_a_decorative_image_has_no_alt_text_to_compare() {
    let builder = clean()
        .custom_style("Caption", "Caption", None)
        .paragraph(Para::empty().raw_run(&Image::picture().alt("The printer").decorative().xml()))
        .paragraph(Para::new("The printer").style("Caption"));
    assert_eq!(count(&builder, RuleId::Img005), 0);
}

// ---- IMG-006 -------------------------------------------------------------

#[test]
fn img_006_fails_on_an_equation() {
    let f = the_failing_finding(RuleId::Img006);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_eq!(f.location.paragraph_index, Some(2));
    assert!(f.message.contains("1 equation"), "{}", f.message);
}

#[test]
fn img_006_passes_without_objects_and_equations() {
    assert!(of(&fixture_findings(RuleId::Img006, false), RuleId::Img006).is_empty());
}

#[test]
fn img_006_reports_embedded_objects() {
    let builder = clean().paragraph(Para::empty().raw_run(&ole_object()));
    let found = rule_findings(&builder, RuleId::Img006);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("1 embedded object"),
        "{}",
        found[0].message
    );
    assert_eq!(found[0].location.paragraph_index, Some(2));
}

#[test]
fn img_006_is_one_finding_for_the_document_with_counts() {
    let builder = clean()
        .para("Intro.")
        .paragraph(
            Para::empty()
                .raw_run(&equation("a=b"))
                .raw_run(&equation("c=d")),
        )
        .paragraph(Para::empty().raw_run(&ole_object()))
        .paragraph(Para::empty().raw_run(&ole_object()));
    let found = rule_findings(&builder, RuleId::Img006);
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .message
            .contains("2 embedded objects and 2 equations"),
        "{}",
        found[0].message
    );
    assert_eq!(
        found[0].location.paragraph_index,
        Some(3),
        "at the first one"
    );
}

#[test]
fn img_006_a_picture_is_not_an_embedded_object() {
    assert_eq!(
        count(&with(&Image::picture().alt("A cat")), RuleId::Img006),
        0
    );
}
