//! IMG-001 to IMG-007.

mod common;

use adapdf::findings::{FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{
    DocxBuilder, Image, Para, alt_of_length, clean, equation, ole_object, text_box,
};
use adapdf::rules::images::MAX_ALT_TEXT_CHARS;
use common::{check, fixture_findings, of, rule_findings, the_failing_finding};

/// A clean document with one paragraph that holds only this drawing.
fn with(image: &Image) -> DocxBuilder {
    clean().paragraph(Para::empty().raw_run(&image.xml()))
}

/// A picture with alt text.
fn screenshot() -> Image {
    Image::picture().alt("The Save dialog with the File name box selected")
}

/// An arrow or box drawn over a picture: a floating shape with no alt text.
fn arrow() -> Image {
    Image::shape().floating()
}

/// One paragraph with these drawings, in order.
fn paragraph_of(images: &[&Image]) -> Para {
    images
        .iter()
        .fold(Para::empty(), |para, image| para.raw_run(&image.xml()))
}

/// The rules that fired, as text such as `IMG-007`.
fn fired(builder: &DocxBuilder) -> Vec<String> {
    check(builder)
        .iter()
        .map(|f| f.rule_id.to_string())
        .collect()
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

#[test]
fn img_001_a_group_or_canvas_is_one_object_with_its_own_alt_text() {
    for (image, name) in [
        (Image::group(), "group of shapes"),
        (Image::canvas(), "drawing canvas"),
        (Image::vml_group(), "group of shapes"),
    ] {
        let found = rule_findings(&with(&image), RuleId::Img001);
        assert_eq!(found.len(), 1, "{name}: the shapes inside are not objects");
        assert!(found[0].message.contains(name), "{}", found[0].message);
        let with_alt = image.clone().alt("The sign-in steps");
        assert_eq!(count(&with(&with_alt), RuleId::Img001), 0, "{name}");
    }
    assert_eq!(
        count(&with(&Image::group().decorative()), RuleId::Img001),
        0
    );
    assert_eq!(
        count(&with(&Image::canvas().title("A flowchart")), RuleId::Img001),
        0,
        "the title counts when there is no description"
    );
}

#[test]
fn img_001_a_group_with_text_in_its_shapes_still_needs_alt_text() {
    for image in [Image::group(), Image::canvas(), Image::vml_group()] {
        let image = image.text("Start");
        assert_eq!(count(&with(&image), RuleId::Img001), 1);
        assert_eq!(
            count(&with(&image.clone().alt("A flowchart")), RuleId::Img001),
            0
        );
    }
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
fn img_004_reports_every_floating_object_that_needs_a_reading_order() {
    for (image, name) in [
        (Image::picture(), "picture"),
        (Image::chart(), "chart"),
        (Image::smart_art(), "SmartArt graphic"),
        (Image::shape(), "shape"),
        (Image::group(), "group of shapes"),
        (Image::canvas(), "drawing canvas"),
        (Image::vml_shape(), "shape"),
        (Image::vml_group(), "group of shapes"),
    ] {
        let image = image.alt("Something");
        let found = rule_findings(&with(&image.clone().floating()), RuleId::Img004);
        assert_eq!(found.len(), 1, "{name}");
        assert!(found[0].message.contains(name), "{}", found[0].message);
        assert!(
            found[0].how_to_fix.contains(name),
            "{}",
            found[0].how_to_fix
        );
        assert_eq!(found[0].fix, FixKind::Auto(FixId::MakeImageInline));
        assert_eq!(count(&with(&image), RuleId::Img004), 0, "{name} inline");
    }
}

#[test]
fn img_004_does_not_report_a_decorative_object() {
    for image in [
        Image::picture(),
        Image::chart(),
        Image::smart_art(),
        Image::shape(),
        Image::group(),
        Image::canvas(),
    ] {
        assert_eq!(
            count(&with(&image.clone().floating()), RuleId::Img004),
            1,
            "not decorative"
        );
        assert_eq!(
            count(&with(&image.floating().decorative()), RuleId::Img004),
            0,
            "decorative"
        );
    }
}

#[test]
fn img_004_leaves_a_text_box_with_text_to_txb_001() {
    let boxed = clean().paragraph(Para::new("Before.").raw_run(&text_box("Note this.")));
    assert_eq!(count(&boxed, RuleId::Img004), 0);
    assert_eq!(count(&boxed, RuleId::Txb001), 1);
    let shape = with(&Image::shape().floating().text("Note this."));
    assert_eq!(count(&shape, RuleId::Img004), 0);
    // A shape with no text is not a text box.
    assert_eq!(count(&with(&Image::shape().floating()), RuleId::Img004), 1);
}

#[test]
fn img_004_a_floating_group_with_text_is_still_reported() {
    let group = with(&Image::group().floating().alt("A flowchart").text("Start"));
    assert_eq!(count(&group, RuleId::Img004), 1);
    assert_eq!(count(&group, RuleId::Txb001), 1);
}

#[test]
fn img_004_embedded_objects_are_never_floating() {
    let ole = clean().paragraph(Para::empty().raw_run(&ole_object()));
    assert_eq!(count(&ole, RuleId::Img004), 0);
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

// ---- IMG-007 -------------------------------------------------------------

/// A picture with alt text and these shapes anchored in the same paragraph.
fn annotated(shapes: &[Image]) -> DocxBuilder {
    let picture = screenshot();
    let mut images: Vec<&Image> = vec![&picture];
    images.extend(shapes);
    clean().paragraph(paragraph_of(&images))
}

#[test]
fn img_007_fails_on_a_screenshot_with_shapes_drawn_on_it() {
    let f = the_failing_finding(RuleId::Img007);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Ask(FixId::MarkAnnotationsDecorative));
    assert_eq!(f.location.part, "word/document.xml");
    assert_eq!(
        f.location.paragraph_index,
        Some(2),
        "the picture's paragraph"
    );
    assert_eq!(f.location.heading.as_deref(), Some("Test guide"));
    assert!(
        f.message
            .starts_with("This screenshot has 3 arrows or boxes drawn on it in Word."),
        "{}",
        f.message
    );
    assert!(f.message.contains("no alt text"), "{}", f.message);
    for part in [
        "Mark as decorative",
        "Group",
        "In Line with Text",
        "only after you confirm",
    ] {
        assert!(f.how_to_fix.contains(part), "{part}: {}", f.how_to_fix);
    }
    assert_eq!(f.suggestion, None, "the tool never invents alt text");
}

#[test]
fn img_007_passes_when_the_shapes_are_decorative() {
    assert!(fixture_findings(RuleId::Img007, false).is_empty());
}

#[test]
fn img_007_three_shapes_are_one_error_and_replace_img_001_and_img_004() {
    let builder = annotated(&[arrow(), arrow(), arrow()]);
    assert_eq!(fired(&builder), ["IMG-007"]);
    let found = rule_findings(&builder, RuleId::Img007);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("3 arrows or boxes"));
}

#[test]
fn img_007_all_shapes_decorative_and_alt_text_on_the_picture_gives_no_findings() {
    let builder = annotated(&[
        arrow().decorative(),
        arrow().decorative(),
        arrow().decorative(),
    ]);
    assert!(check(&builder).is_empty(), "{:#?}", check(&builder));
}

#[test]
fn img_007_a_picture_without_alt_text_still_gets_img_001() {
    let picture = Image::picture();
    let builder = clean().paragraph(paragraph_of(&[&picture, &arrow(), &arrow()]));
    assert_eq!(count(&builder, RuleId::Img001), 1, "for the picture");
    assert_eq!(count(&builder, RuleId::Img007), 1, "for the shapes");
    // Decorative shapes: only the picture is wrong.
    let decorative = clean().paragraph(paragraph_of(&[
        &picture,
        &arrow().decorative(),
        &arrow().decorative(),
    ]));
    assert_eq!(fired(&decorative), ["IMG-001"]);
}

#[test]
fn img_007_counts_only_shapes_with_no_alt_text_and_no_decorative_mark() {
    let builder = annotated(&[
        arrow(),
        arrow().decorative(),
        arrow().alt("A red box around the File name field"),
    ]);
    let found = rule_findings(&builder, RuleId::Img007);
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .message
            .starts_with("This screenshot has 1 arrow or box drawn"),
        "{}",
        found[0].message
    );
    assert_eq!(count(&builder, RuleId::Img001), 0);
    // The shape with alt text floats and is not decorative: IMG-004 still
    // reports it. The shape that IMG-007 reports is not reported twice.
    assert_eq!(count(&builder, RuleId::Img004), 1);
}

#[test]
fn img_007_shapes_with_alt_text_pass() {
    let builder = annotated(&[arrow().alt("A red box"), arrow().title("An arrow")]);
    assert_eq!(count(&builder, RuleId::Img007), 0);
    assert_eq!(count(&builder, RuleId::Img001), 0);
}

#[test]
fn img_007_the_shapes_can_be_in_the_paragraph_before_or_after() {
    let shapes = || paragraph_of(&[&arrow(), &arrow()]);
    let picture = || Para::empty().raw_run(&screenshot().xml());
    let before = clean().paragraph(shapes()).paragraph(picture());
    let after = clean().paragraph(picture()).paragraph(shapes());
    for (name, builder, picture_index) in [("before", before, 3), ("after", after, 2)] {
        let found = rule_findings(&builder, RuleId::Img007);
        assert_eq!(found.len(), 1, "{name}");
        assert_eq!(found[0].location.paragraph_index, Some(picture_index));
        assert_eq!(count(&builder, RuleId::Img001), 0, "{name}");
        assert_eq!(count(&builder, RuleId::Img004), 0, "{name}");
    }
}

#[test]
fn img_007_shapes_two_paragraphs_away_are_not_annotations() {
    let builder = clean()
        .paragraph(Para::empty().raw_run(&screenshot().xml()))
        .para("Step text.")
        .paragraph(paragraph_of(&[&arrow(), &arrow(), &arrow()]));
    assert_eq!(count(&builder, RuleId::Img007), 0);
    assert_eq!(count(&builder, RuleId::Img001), 3, "one for each shape");
    assert_eq!(count(&builder, RuleId::Img004), 3, "one for each shape");
}

#[test]
fn img_007_shapes_with_no_picture_near_them_are_reported_one_by_one() {
    let builder = clean().paragraph(paragraph_of(&[&arrow(), &arrow()]));
    assert_eq!(count(&builder, RuleId::Img007), 0);
    assert_eq!(count(&builder, RuleId::Img001), 2);
}

#[test]
fn img_007_a_shape_that_is_not_floating_is_not_an_annotation() {
    let picture = screenshot();
    let inline_shape = Image::shape();
    let builder = clean().paragraph(paragraph_of(&[&picture, &inline_shape]));
    assert_eq!(count(&builder, RuleId::Img007), 0);
    assert_eq!(count(&builder, RuleId::Img001), 1);
}

#[test]
fn img_007_charts_smart_art_groups_and_pictures_are_not_annotation_shapes() {
    for other in [
        Image::chart().floating(),
        Image::smart_art().floating(),
        Image::group().floating(),
        Image::canvas().floating(),
        Image::picture().floating(),
    ] {
        let builder = annotated(std::slice::from_ref(&other));
        assert_eq!(count(&builder, RuleId::Img007), 0);
        assert_eq!(count(&builder, RuleId::Img001), 1, "needs its own alt text");
    }
}

#[test]
fn img_007_a_floating_picture_can_be_the_screenshot() {
    let picture = screenshot().floating();
    let builder = clean().paragraph(paragraph_of(&[&picture, &arrow(), &arrow()]));
    assert_eq!(count(&builder, RuleId::Img007), 1);
    assert_eq!(count(&builder, RuleId::Img004), 1, "the picture floats");
    assert_eq!(count(&builder, RuleId::Img001), 0);
}

#[test]
fn img_007_vml_shapes_count() {
    let builder = annotated(&[Image::vml_shape().floating(), Image::vml_shape().floating()]);
    let found = rule_findings(&builder, RuleId::Img007);
    assert_eq!(found.len(), 1);
    assert!(found[0].message.contains("2 arrows or boxes"));
    assert_eq!(count(&builder, RuleId::Img001), 0);
    assert_eq!(count(&builder, RuleId::Img004), 0);
    // A VML picture is a picture too.
    let vml_picture = Image::vml_picture().alt("The Save dialog");
    let builder = clean().paragraph(paragraph_of(&[&vml_picture, &arrow()]));
    assert_eq!(count(&builder, RuleId::Img007), 1);
}

#[test]
fn img_007_a_picture_and_shapes_already_grouped_are_one_figure() {
    // The group checks as one drawing: its own alt text decides.
    let grouped = clean().paragraph(paragraph_of(&[&Image::group().alt("The Save dialog")]));
    assert!(check(&grouped).is_empty(), "{:#?}", check(&grouped));
    let no_alt = clean().paragraph(paragraph_of(&[&Image::group()]));
    assert_eq!(fired(&no_alt), ["IMG-001"]);
    let vml = clean().paragraph(paragraph_of(&[&Image::vml_group().alt("The Save dialog")]));
    assert!(check(&vml).is_empty(), "{:#?}", check(&vml));
    let vml_no_alt = clean().paragraph(paragraph_of(&[&Image::vml_group()]));
    assert_eq!(fired(&vml_no_alt), ["IMG-001"]);
}

#[test]
fn img_007_a_floating_group_is_reported_by_img_004() {
    let group = Image::group().alt("The Save dialog").floating();
    assert_eq!(
        fired(&clean().paragraph(paragraph_of(&[&group]))),
        ["IMG-004"]
    );
    let vml = Image::vml_group().alt("The Save dialog").floating();
    assert_eq!(
        fired(&clean().paragraph(paragraph_of(&[&vml]))),
        ["IMG-004"]
    );
}

#[test]
fn img_007_a_shape_belongs_to_one_picture() {
    // Picture, shapes, picture: the shapes go to the picture before them.
    let builder = clean()
        .paragraph(Para::empty().raw_run(&screenshot().xml()))
        .paragraph(paragraph_of(&[&arrow(), &arrow()]))
        .paragraph(Para::empty().raw_run(&screenshot().xml()));
    let found = rule_findings(&builder, RuleId::Img007);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].location.paragraph_index, Some(2));
}

#[test]
fn img_007_two_screenshots_give_two_findings() {
    let builder = clean()
        .paragraph(paragraph_of(&[&screenshot(), &arrow()]))
        .para("Step text.")
        .para("More step text.")
        .paragraph(paragraph_of(&[&screenshot(), &arrow(), &arrow()]));
    let found = rule_findings(&builder, RuleId::Img007);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].location.paragraph_index, Some(2));
    assert_eq!(found[1].location.paragraph_index, Some(5));
    assert!(found[0].message.contains("1 arrow or box"));
    assert!(found[1].message.contains("2 arrows or boxes"));
}

#[test]
fn img_007_a_shape_with_text_is_left_to_txb_001() {
    let builder = annotated(&[Image::shape().floating().text("Click here")]);
    assert_eq!(count(&builder, RuleId::Img007), 0);
    assert_eq!(count(&builder, RuleId::Txb001), 1);
}
