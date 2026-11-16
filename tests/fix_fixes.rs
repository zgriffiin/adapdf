//! Each fix on its own: what it changes, what it must not change, and what it
//! refuses. Test plan rows S2-16 to S2-30.

mod fix_support;

use adapdf::findings::{FixId, RuleId};
use adapdf::fix::{self, Availability, FixInput, FixOptions, FixRequest};
use adapdf::fixtures::{
    DocxBuilder, FixtureFonts, Image, Para, Table, clean, raw_docx, split_runs,
};
use adapdf::model::{Document, Drawing};
use fix_support::*;

const DOCUMENT: &str = "word/document.xml";
const STYLES: &str = "word/styles.xml";
const CORE: &str = "docProps/core.xml";
const GOOD_ALT: &str = "Front panel of the printer with the power button circled in red";

fn model(bytes: &[u8]) -> Document {
    Document::from_bytes(bytes).expect("a valid .docx")
}

fn drawings(bytes: &[u8]) -> Vec<Drawing> {
    model(bytes)
        .paragraphs
        .iter()
        .flat_map(|p| p.drawings.clone())
        .collect()
}

fn texts(bytes: &[u8]) -> Vec<String> {
    model(bytes)
        .paragraphs
        .iter()
        .map(|p| p.text.clone())
        .collect()
}

fn with_image(image: &Image) -> Vec<u8> {
    build(&clean().paragraph(Para::empty().raw_run(&image.xml())))
}

/// Removes a paragraph style from `word/styles.xml`.
fn without_style(bytes: &[u8], id: &str) -> Vec<u8> {
    replace_part(bytes, STYLES, |t| {
        let start = t
            .find(&format!(r#"<w:style w:type="paragraph" w:styleId="{id}">"#))
            .expect("the style is in the document");
        let end = start + t[start..].find("</w:style>").unwrap() + "</w:style>".len();
        format!("{}{}", &t[..start], &t[end..])
    })
}

/// Checks that the fix leaves no new finding and returns the findings after.
fn assert_fixed(
    original: &[u8],
    applied: &fix::Applied,
    rule: RuleId,
) -> Vec<adapdf::findings::Finding> {
    assert!(applied.refused.is_empty(), "{:#?}", applied.refused);
    assert_only_the_splices_changed(original, applied);
    let before = findings(original);
    let after = findings(fixed(applied));
    assert!(
        of(&after, rule).len() < of(&before, rule).len(),
        "{rule} is still there"
    );
    let mut remaining = rules(&before);
    for r in rules(&after) {
        match remaining.iter().position(|x| *x == r) {
            Some(i) => {
                remaining.remove(i);
            }
            None => panic!("{r} is a new finding after the fix of {rule}"),
        }
    }
    after
}

// ---------- S2-16 SetTitle ----------

#[test]
fn ask_set_title_writes_dc_title() {
    let original = fixture(RuleId::Doc001, true);
    let found = findings(&original);
    let applied = apply(
        &original,
        &requests_for(&found, RuleId::Doc001, "Printer guide"),
    );
    let after = assert_fixed(&original, &applied, RuleId::Doc001);
    assert!(of(&after, RuleId::Doc001).is_empty());
    assert_eq!(
        applied
            .splices
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [CORE],
        "the title is written in docProps/core.xml only"
    );
    assert!(part(fixed(&applied), CORE).contains("<dc:title>Printer guide</dc:title>"));
    assert_eq!(
        model(fixed(&applied)).core.title.as_deref(),
        Some("Printer guide")
    );

    // No dc:title element, an empty one written as <dc:title/>, and one that
    // holds only spaces.
    for (name, change) in [
        ("none", "".to_string()),
        ("self-closing", "<dc:title/>".to_string()),
        ("spaces", "<dc:title>   </dc:title>".to_string()),
    ] {
        let copy = replace_part(&original, CORE, |t| {
            t.replace("<dc:title></dc:title>", &change)
        });
        let found = findings(&copy);
        assert_eq!(of(&found, RuleId::Doc001).len(), 1, "{name}");
        let applied = apply(
            &copy,
            &requests_for(&found, RuleId::Doc001, "Printer guide"),
        );
        assert_fixed(&copy, &applied, RuleId::Doc001);
        assert_eq!(
            model(fixed(&applied)).core.title.as_deref(),
            Some("Printer guide"),
            "{name}"
        );
    }

    // Spaces only is refused.
    let applied = apply(&original, &[FixRequest::text(&found[0], "  ")]);
    assert!(applied.bytes.is_none());
    assert_eq!(applied.refused.len(), 1);
}

#[test]
fn set_title_refuses_a_document_with_no_properties_part() {
    let original = fixture(RuleId::Doc001, true);
    let parts_without_core: Vec<(String, Vec<u8>)> = parts(&original)
        .into_iter()
        .filter(|(n, _)| n != CORE)
        .collect();
    let entries: Vec<(&str, &[u8])> = parts_without_core
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    let copy = adapdf::fixtures::zip_stored(&entries);
    let found = of(&findings(&copy), RuleId::Doc001);
    assert_eq!(found.len(), 1);
    let applied = apply(&copy, &[FixRequest::text(&found[0], "Title")]);
    assert!(applied.bytes.is_none());
    assert!(
        applied.refused[0].reason.contains("File > Info"),
        "{:?}",
        applied.refused
    );
}

// ---------- S2-17 SetDefaultLanguage ----------

fn language_of(bytes: &[u8]) -> Option<String> {
    model(bytes).default_language
}

#[test]
fn fix_set_default_language() {
    let original = fixture(RuleId::Doc002, true);
    let applied = fix_rule(&original, RuleId::Doc002);
    let after = assert_fixed(&original, &applied, RuleId::Doc002);
    assert!(of(&after, RuleId::Doc002).is_empty());
    assert_eq!(language_of(fixed(&applied)).as_deref(), Some("en-US"));
    assert_eq!(
        applied
            .splices
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [STYLES]
    );
    // After the other properties, as the schema orders them.
    assert!(part(fixed(&applied), STYLES).contains(r#"<w:sz w:val="22"/><w:lang w:val="en-US"/>"#));

    // An empty w:lang, in two forms.
    for empty in [
        r#"<w:lang w:val=""/>"#,
        "<w:lang/>",
        r#"<w:lang w:eastAsia="ja-JP"/>"#,
    ] {
        let copy = replace_part(&original, STYLES, |t| {
            t.replace(
                r#"<w:sz w:val="22"/>"#,
                &format!(r#"<w:sz w:val="22"/>{empty}"#),
            )
        });
        let applied = fix_rule(&copy, RuleId::Doc002);
        assert_fixed(&copy, &applied, RuleId::Doc002);
        assert_eq!(
            language_of(fixed(&applied)).as_deref(),
            Some("en-US"),
            "{empty}"
        );
        assert_eq!(
            part(fixed(&applied), STYLES).matches("<w:lang").count(),
            1,
            "{empty}: no second w:lang"
        );
    }

    // Other children in the defaults, and a property that must come after lang.
    let copy = build(&DocxBuilder::new().no_language().font("Arial").title("T"));
    let copy = replace_part(&copy, STYLES, |t| {
        t.replace(
            r#"<w:sz w:val="22"/>"#,
            r#"<w:sz w:val="22"/><w:eastAsianLayout w:id="1"/>"#,
        )
    });
    let applied = fix_rule(&copy, RuleId::Doc002);
    assert_fixed(&copy, &applied, RuleId::Doc002);
    assert!(part(fixed(&applied), STYLES).contains(r#"<w:lang w:val="en-US"/><w:eastAsianLayout"#));
}

#[test]
fn set_default_language_writes_the_language_of_the_setting() {
    let original = fixture(RuleId::Doc002, true);
    let found = findings(&original);
    let request = requests_for(&found, RuleId::Doc002, "");
    let gb = FixOptions {
        default_language: "en-GB".to_string(),
    };
    let applied = fix::apply(&original, &request, &gb).unwrap();
    assert_eq!(language_of(fixed(&applied)).as_deref(), Some("en-GB"));
    assert!(part(fixed(&applied), STYLES).contains(r#"w:val="en-GB""#));

    let bad = FixOptions {
        default_language: "english please".to_string(),
    };
    let applied = fix::apply(&original, &request, &bad).unwrap();
    assert!(applied.bytes.is_none());
    assert!(applied.refused[0].reason.contains("language tag"));
}

#[test]
fn set_default_language_in_a_styles_part_with_fewer_parts() {
    let original = fixture(RuleId::Doc002, true);
    let defaults = |t: &str, inner: &str| {
        let start = t.find("<w:docDefaults>").unwrap();
        let end = t.find("</w:docDefaults>").unwrap() + "</w:docDefaults>".len();
        format!("{}{inner}{}", &t[..start], &t[end..])
    };
    for (name, inner) in [
        ("no defaults", ""),
        ("empty defaults", "<w:docDefaults/>"),
        (
            "no run defaults",
            "<w:docDefaults><w:pPrDefault/></w:docDefaults>",
        ),
        (
            "empty run defaults",
            "<w:docDefaults><w:rPrDefault/></w:docDefaults>",
        ),
        (
            "empty run properties",
            "<w:docDefaults><w:rPrDefault><w:rPr/></w:rPrDefault></w:docDefaults>",
        ),
    ] {
        let copy = replace_part(&original, STYLES, |t| defaults(t, inner));
        let applied = fix_rule(&copy, RuleId::Doc002);
        assert_fixed(&copy, &applied, RuleId::Doc002);
        assert_eq!(
            language_of(fixed(&applied)).as_deref(),
            Some("en-US"),
            "{name}"
        );
    }
}

// ---------- S2-18 ApplyHeadingStyle ----------

fn style_of(bytes: &[u8], index: usize) -> Option<String> {
    model(bytes).paragraphs[index].style_id.clone()
}

#[test]
fn fix_apply_heading_style_at_the_suggested_level() {
    let original = fixture(RuleId::Hdg003, true);
    let found = of(&findings(&original), RuleId::Hdg003);
    assert_eq!(found[0].suggestion.as_deref(), Some("Heading 2"));
    let applied = fix_rule(&original, RuleId::Hdg003);
    let after = assert_fixed(&original, &applied, RuleId::Hdg003);
    assert!(of(&after, RuleId::Hdg002).is_empty(), "no HDG-002 appears");
    assert_eq!(style_of(fixed(&applied), 2).as_deref(), Some("Heading2"));
    assert_eq!(
        texts(fixed(&applied)),
        texts(&original),
        "the text is the same"
    );
    assert_eq!(
        applied
            .splices
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [DOCUMENT],
        "the style is defined, so styles.xml is not changed"
    );
    assert!(
        part(fixed(&applied), DOCUMENT)
            .contains(r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r>"#)
    );
}

#[test]
fn fix_apply_heading_style_when_the_style_is_not_defined() {
    let original = without_style(&fixture(RuleId::Hdg003, true), "Heading2");
    // The style is undefined: Word's definition is added, then applied.
    let found = findings(&original);
    assert_eq!(
        of(&found, RuleId::Hdg003)[0].suggestion.as_deref(),
        Some("Heading 2")
    );
    let applied = fix_rule(&original, RuleId::Hdg003);
    let after = assert_fixed(&original, &applied, RuleId::Hdg003);
    assert!(of(&after, RuleId::Hdg002).is_empty());
    let styles = part(fixed(&applied), STYLES);
    assert!(styles.contains(r#"w:styleId="Heading2""#), "{styles}");
    assert!(styles.contains(r#"<w:name w:val="heading 2"/>"#));
    assert!(styles.contains(r#"<w:outlineLvl w:val="1"/>"#));
    assert!(styles.contains(r#"<w:basedOn w:val="Normal"/>"#));
    let after_model = model(fixed(&applied));
    assert_eq!(after_model.paragraphs[2].heading_level, Some(2));
    assert_eq!(
        applied
            .splices
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [DOCUMENT, STYLES]
    );

    // Two paragraphs that need the same style add it once.
    let two = without_style(
        &build(
            &clean()
                .paragraph(Para::new("Setting up the printer").bold())
                .para("Plug it in.")
                .paragraph(Para::new("Loading the paper").bold())
                .para("Open the tray."),
        ),
        "Heading2",
    );
    let applied = fix_rule(&two, RuleId::Hdg003);
    assert_fixed(&two, &applied, RuleId::Hdg003);
    assert_eq!(
        part(fixed(&applied), STYLES)
            .matches(r#"w:styleId="Heading2""#)
            .count(),
        1
    );
}

#[test]
fn fix_apply_heading_style_with_no_heading_above_and_a_style_already_set() {
    // No heading above: Heading 1, and HDG-002 does not appear.
    let none_above = build(
        &DocxBuilder::new()
            .title("T")
            .language("en-US")
            .paragraph(Para::new("Setting up the printer").bold())
            .para("Plug it in."),
    );
    let found = of(&findings(&none_above), RuleId::Hdg003);
    assert_eq!(found[0].suggestion.as_deref(), Some("Heading 1"));
    let applied = fix_rule(&none_above, RuleId::Hdg003);
    let after = assert_fixed(&none_above, &applied, RuleId::Hdg003);
    assert!(of(&after, RuleId::Hdg002).is_empty());
    assert_eq!(style_of(fixed(&applied), 0).as_deref(), Some("Heading1"));

    // A paragraph that names a style already: the name is replaced.
    let styled = build(
        &clean()
            .paragraph(Para::new("Setting up the printer").bold().style("Normal"))
            .para("Plug it in."),
    );
    let applied = fix_rule(&styled, RuleId::Hdg003);
    assert_fixed(&styled, &applied, RuleId::Hdg003);
    assert_eq!(style_of(fixed(&applied), 2).as_deref(), Some("Heading2"));
    assert!(!part(fixed(&applied), DOCUMENT).contains(r#"w:val="Normal""#));
}

// ---------- S2-19 DeleteEmptyHeading ----------

#[test]
fn fix_delete_empty_heading() {
    let original = fixture(RuleId::Hdg004, true);
    let applied = fix_rule(&original, RuleId::Hdg004);
    let after = assert_fixed(&original, &applied, RuleId::Hdg004);
    assert!(of(&after, RuleId::Hdg004).is_empty());
    assert_eq!(
        texts(fixed(&applied)),
        [
            "Test guide",
            "This is a short paragraph of body text.",
            "Plug it in."
        ]
    );
    // One deletion; the neighbours are byte-equal.
    let splices = &applied.splices[DOCUMENT];
    assert_eq!(splices.len(), 1);
    assert_eq!(splices[0].text, "");
    let old = part(&original, DOCUMENT);
    assert_eq!(
        &old[splices[0].range.clone()],
        r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t xml:space="preserve"></w:t></w:r></w:p>"#
    );
}

#[test]
fn fix_delete_empty_heading_refuses_what_it_must_keep() {
    let cases = [
        (
            "a section break",
            Para::empty().style("Heading2").ends_section(1),
            "section break",
        ),
        (
            "a bookmark",
            Para::empty()
                .style("Heading2")
                .raw_run(r#"<w:bookmarkStart w:id="9" w:name="Target"/><w:bookmarkEnd w:id="9"/>"#),
            "bookmark",
        ),
        (
            "a page break",
            Para::empty()
                .style("Heading2")
                .raw_run(r#"<w:r><w:br w:type="page"/></w:r>"#),
            "page or column break",
        ),
    ];
    for (name, heading, reason) in cases {
        let original = build(&clean().paragraph(heading).para("Plug it in."));
        let found = findings(&original);
        assert_eq!(of(&found, RuleId::Hdg004).len(), 1, "{name}");
        let applied = apply(&original, &requests_for(&found, RuleId::Hdg004, ""));
        assert!(applied.bytes.is_none(), "{name}: nothing is written");
        assert_eq!(applied.refused.len(), 1, "{name}");
        assert!(
            applied.refused[0].reason.contains(reason),
            "{name}: {}",
            applied.refused[0].reason
        );
        assert!(applied.refused[0].reason.contains("not deleted"));
    }
    // The only paragraph of a table cell: a cell must end with a paragraph.
    let original = build(
        &clean()
            .paragraph(Para::empty().style("Heading2").in_table())
            .para("After."),
    );
    let found = findings(&original);
    let applied = apply(&original, &requests_for(&found, RuleId::Hdg004, ""));
    assert!(applied.bytes.is_none());
    assert!(
        applied.refused[0].reason.contains("table cell"),
        "{:?}",
        applied.refused
    );
}

// ---------- S2-20 ApplyListStyle ----------

#[test]
fn fix_apply_list_style() {
    let original = fixture(RuleId::Lst001, true);
    let applied = fix_rule(&original, RuleId::Lst001);
    let after = assert_fixed(&original, &applied, RuleId::Lst001);
    assert!(
        of(&after, RuleId::Lst002).is_empty(),
        "LST-002 does not appear"
    );
    let m = model(fixed(&applied));
    assert_eq!(m.paragraphs[2].text, "Open the file.");
    assert_eq!(m.paragraphs[3].text, "Save the file.");
    assert_eq!(m.paragraphs[2].style_id.as_deref(), Some("ListNumber"));
    assert_eq!(m.paragraphs[3].style_id.as_deref(), Some("ListNumber"));
    assert!(
        m.paragraphs[2].props.num.is_some(),
        "it is a real list item"
    );
    // The runs that only held the marker are gone.
    let xml = part(fixed(&applied), DOCUMENT);
    assert!(!xml.contains("1. "), "{xml}");
}

/// A paragraph made of these runs, with the text that must be left.
fn list_cases() -> Vec<(&'static str, Para, &'static str, &'static str)> {
    vec![
        ("typed bullet", Para::new("• Open the file."), "Open the file.", "ListBullet"),
        ("hyphen", Para::new("- Open the file."), "Open the file.", "ListBullet"),
        ("star", Para::new("* Open the file."), "Open the file.", "ListBullet"),
        (
            "marker in two runs",
            split_runs(&["1", ". Open the file."]),
            "Open the file.",
            "ListNumber",
        ),
        (
            "marker, space and text in three runs",
            split_runs(&["1)", " ", "Open the file."]),
            "Open the file.",
            "ListNumber",
        ),
        ("tab after the marker", Para::new("1.\tOpen the file."), "Open the file.", "ListNumber"),
        (
            "a tab element after the marker",
            Para::empty().raw_run(
                r#"<w:r><w:t>2.</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>Open the file.</w:t></w:r>"#,
            ),
            "Open the file.",
            "ListNumber",
        ),
        (
            "a symbol bullet",
            Para::empty().raw_run(
                r#"<w:r><w:sym w:font="Symbol" w:char="F0B7"/></w:r><w:r><w:t xml:space="preserve"> Open the file.</w:t></w:r>"#,
            ),
            "Open the file.",
            "ListBullet",
        ),
        (
            "a bullet written as a character reference",
            Para::empty().raw_run(r#"<w:r><w:t xml:space="preserve">&#8226; Open &amp; close.</w:t></w:r>"#),
            "Open & close.",
            "ListBullet",
        ),
        ("leading spaces", Para::new("  3.   Open the file."), "Open the file.", "ListNumber"),
        (
            "a run with formatting",
            Para::empty().raw_run(
                r#"<w:r><w:rPr><w:b/></w:rPr><w:t>1.</w:t></w:r><w:r><w:rPr><w:i/></w:rPr><w:t xml:space="preserve"> Open the file.</w:t></w:r>"#,
            ),
            "Open the file.",
            "ListNumber",
        ),
    ]
}

#[test]
fn fix_apply_list_style_for_every_form_of_typed_marker() {
    for (name, paragraph, left, style) in list_cases() {
        let original = build(&clean().paragraph(paragraph));
        let applied = fix_rule(&original, RuleId::Lst001);
        let after = assert_fixed(&original, &applied, RuleId::Lst001);
        assert!(of(&after, RuleId::Lst002).is_empty(), "{name}");
        let m = model(fixed(&applied));
        assert_eq!(m.paragraphs[2].text, left, "{name}");
        assert_eq!(m.paragraphs[2].style_id.as_deref(), Some(style), "{name}");
    }
}

#[test]
fn fix_apply_list_style_keeps_formatting_of_the_text_that_stays() {
    let original = build(&clean().paragraph(Para::empty().raw_run(
        r#"<w:r><w:rPr><w:b/></w:rPr><w:t>1.</w:t></w:r><w:r><w:rPr><w:i/></w:rPr><w:t xml:space="preserve"> Open the file.</w:t></w:r>"#,
    )));
    let applied = fix_rule(&original, RuleId::Lst001);
    let xml = part(fixed(&applied), DOCUMENT);
    assert!(
        xml.contains(
            r#"<w:r><w:rPr><w:i/></w:rPr><w:t xml:space="preserve">Open the file.</w:t></w:r>"#
        ),
        "{xml}"
    );
    assert!(
        !xml.contains("<w:b/>"),
        "the run that held only the marker is gone: {xml}"
    );
}

#[test]
fn fix_apply_list_style_refuses_when_the_document_has_no_list_style() {
    let original = without_style(&fixture(RuleId::Lst001, true), "ListNumber");
    let found = findings(&original);
    let applied = apply(&original, &requests_for(&found, RuleId::Lst001, ""));
    assert!(applied.bytes.is_none());
    assert_eq!(applied.refused.len(), 2);
    assert!(
        applied.refused[0].reason.contains("List Number"),
        "{:?}",
        applied.refused
    );
}

#[test]
fn fix_apply_list_style_closes_a_mixed_list() {
    let original = fixture(RuleId::Lst002, true);
    let before = findings(&original);
    assert_eq!(of(&before, RuleId::Lst002).len(), 1);
    let applied = fix_rule(&original, RuleId::Lst001);
    let after = assert_fixed(&original, &applied, RuleId::Lst001);
    assert!(
        of(&after, RuleId::Lst002).is_empty(),
        "the list is all automatic now"
    );
}

// ---------- S2-21 SetAltText ----------

#[test]
fn ask_set_alt_text_writes_descr_or_alt() {
    // A picture with no alt text: the text equals what the builder writes.
    let original = fixture(RuleId::Img001, true);
    let applied = fix_rule(&original, RuleId::Img001);
    let after = assert_fixed(&original, &applied, RuleId::Img001);
    assert!(of(&after, RuleId::Img001).is_empty());
    let expected = with_image(&Image::picture().alt("Typed text"));
    assert_eq!(
        part(fixed(&applied), DOCUMENT),
        part(&expected, DOCUMENT),
        "the same as a picture made with that alt text"
    );

    // A placeholder is replaced.
    let original = fixture(RuleId::Img002, true);
    let applied = fix_rule(&original, RuleId::Img002);
    let after = assert_fixed(&original, &applied, RuleId::Img002);
    assert!(of(&after, RuleId::Img002).is_empty());
    assert_eq!(
        drawings(fixed(&applied))[0].description.as_deref(),
        Some("Typed text")
    );

    // The typed text is itself a placeholder: IMG-002 stays.
    let found = findings(&original);
    let applied = apply(
        &original,
        &[FixRequest::text(
            &of(&found, RuleId::Img002)[0],
            "Picture 2",
        )],
    );
    assert_eq!(of(&findings(fixed(&applied)), RuleId::Img002).len(), 1);
}

#[test]
fn ask_set_alt_text_for_charts_shapes_groups_and_vml() {
    for (name, image) in [
        ("chart", Image::chart()),
        ("smart art", Image::smart_art()),
        ("shape", Image::shape()),
        ("group", Image::group()),
        ("canvas", Image::canvas()),
        ("vml picture", Image::vml_picture()),
        ("vml shape", Image::vml_shape()),
        ("vml group", Image::vml_group()),
    ] {
        let original = with_image(&image);
        let found = findings(&original);
        assert_eq!(of(&found, RuleId::Img001).len(), 1, "{name}");
        let applied = fix_rule(&original, RuleId::Img001);
        let after = assert_fixed(&original, &applied, RuleId::Img001);
        assert!(of(&after, RuleId::Img001).is_empty(), "{name}");
        assert_eq!(
            drawings(fixed(&applied))[0].description.as_deref(),
            Some("Typed text"),
            "{name}"
        );
        // Shapes, groups and canvases have a VML copy: both copies agree.
        let xml = part(fixed(&applied), DOCUMENT);
        if matches!(name, "shape" | "group" | "canvas") {
            assert!(xml.contains(r#"descr="Typed text""#), "{name}");
            assert!(
                xml.contains(r#"<v:"#) && xml.matches(r#"alt="Typed text""#).count() == 1,
                "{name}: {xml}"
            );
        }
        assert_eq!(
            xml.matches("Typed text").count(),
            if matches!(name, "shape" | "group" | "canvas") {
                2
            } else {
                1
            },
            "{name}"
        );
    }
}

#[test]
fn ask_set_alt_text_can_mark_as_decorative() {
    let original = fixture(RuleId::Img001, true);
    let found = of(&findings(&original), RuleId::Img001);
    let applied = apply(&original, &[FixRequest::decorative(&found[0])]);
    let after = assert_fixed(&original, &applied, RuleId::Img001);
    assert!(of(&after, RuleId::Img001).is_empty());
    assert!(drawings(fixed(&applied))[0].decorative);
    // The same as a picture that was made decorative.
    let expected = with_image(&Image::picture().decorative());
    assert_eq!(part(fixed(&applied), DOCUMENT), part(&expected, DOCUMENT));
    assert!(
        !part(fixed(&applied), DOCUMENT).contains("descr="),
        "no alt text is written"
    );

    // A docPr written as <wp:docPr .../> is opened and the namespace declared.
    let copy = replace_part(&original, DOCUMENT, |t| {
        t.replace(r#"name="Object 1"></wp:docPr>"#, r#"name="Object 1"/>"#)
    });
    let found = of(&findings(&copy), RuleId::Img001);
    let applied = apply(&copy, &[FixRequest::decorative(&found[0])]);
    assert_fixed(&copy, &applied, RuleId::Img001);
    assert_eq!(part(fixed(&applied), DOCUMENT), part(&expected, DOCUMENT));

    // An a:extLst that has other extensions, and one that says val="0".
    let other = replace_part(&original, DOCUMENT, |t| {
        t.replace(
            r#"name="Object 1"></wp:docPr>"#,
            r#"name="Object 1"><a:extLst><a:ext uri="{OTHER}"/></a:extLst></wp:docPr>"#,
        )
    });
    let found = of(&findings(&other), RuleId::Img001);
    let applied = apply(&other, &[FixRequest::decorative(&found[0])]);
    assert_fixed(&other, &applied, RuleId::Img001);
    assert!(drawings(fixed(&applied))[0].decorative);
    assert!(part(fixed(&applied), DOCUMENT).contains(r#"<a:ext uri="{OTHER}"/>"#));
    let off = replace_part(&original, DOCUMENT, |t| {
        t.replace(
            r#"name="Object 1"></wp:docPr>"#,
            r#"name="Object 1"><a:extLst><a:ext uri="{C183D7F6-B498-43B3-948B-1728B52AA6E4}"><adec:decorative xmlns:adec="http://schemas.microsoft.com/office/drawing/2017/decorative" val="0"/></a:ext></a:extLst></wp:docPr>"#,
        )
    });
    let found = of(&findings(&off), RuleId::Img001);
    let applied = apply(&off, &[FixRequest::decorative(&found[0])]);
    assert_fixed(&off, &applied, RuleId::Img001);
    assert!(drawings(fixed(&applied))[0].decorative);
    assert_eq!(
        part(fixed(&applied), DOCUMENT)
            .matches("decorative")
            .count(),
        2,
        "val is set, nothing is added"
    );

    // A placeholder that is marked decorative is cleared.
    let placeholder = fixture(RuleId::Img002, true);
    let found = of(&findings(&placeholder), RuleId::Img002);
    let applied = apply(&placeholder, &[FixRequest::decorative(&found[0])]);
    let after = assert_fixed(&placeholder, &applied, RuleId::Img002);
    assert!(of(&after, RuleId::Img002).is_empty());
    assert!(part(fixed(&applied), DOCUMENT).contains(r#"descr="""#));

    // VML has no decorative mark.
    let vml = with_image(&Image::vml_picture());
    let found = of(&findings(&vml), RuleId::Img001);
    let applied = apply(&vml, &[FixRequest::decorative(&found[0])]);
    assert!(applied.bytes.is_none());
    assert!(
        applied.refused[0].reason.contains("VML"),
        "{:?}",
        applied.refused
    );
}

#[test]
fn ask_set_alt_text_for_two_drawings_in_one_paragraph() {
    let original = build(
        &clean().paragraph(
            Para::empty()
                .raw_run(&Image::picture().xml())
                .raw_run(&Image::picture().xml()),
        ),
    );
    let found = of(&findings(&original), RuleId::Img001);
    assert_eq!(found.len(), 2);
    let applied = apply(
        &original,
        &[
            FixRequest::text(&found[0], "First"),
            FixRequest::text(&found[1], "Second"),
        ],
    );
    assert_fixed(&original, &applied, RuleId::Img001);
    let alts: Vec<_> = drawings(fixed(&applied))
        .into_iter()
        .map(|d| d.description)
        .collect();
    assert_eq!(
        alts,
        [Some("First".to_string()), Some("Second".to_string())]
    );
    // Only the second one.
    let applied = apply(&original, &[FixRequest::text(&found[1], "Second").at(1)]);
    let alts: Vec<_> = drawings(fixed(&applied))
        .into_iter()
        .map(|d| d.description)
        .collect();
    assert_eq!(alts, [None, Some("Second".to_string())]);
}

// ---------- S2-22 MakeImageInline ----------

#[test]
fn fix_make_inline_for_pictures_charts_and_smart_art() {
    let original = fixture(RuleId::Img004, true);
    let applied = fix_rule(&original, RuleId::Img004);
    let after = assert_fixed(&original, &applied, RuleId::Img004);
    assert!(of(&after, RuleId::Img004).is_empty());
    let xml = part(fixed(&applied), DOCUMENT);
    assert!(xml.contains("<wp:inline"), "{xml}");
    assert!(
        !xml.contains("anchor")
            && !xml.contains("simplePos")
            && !xml.contains("positionH")
            && !xml.contains("wrapSquare")
    );
    // The extent, the docPr and the graphic are kept as they were.
    let expected = with_image(&Image::picture().alt(GOOD_ALT));
    assert_eq!(
        xml,
        part(&expected, DOCUMENT),
        "the same as an inline picture"
    );
    assert!(!drawings(fixed(&applied))[0].floating);

    // A chart and SmartArt (in memory).
    let original = build(
        &clean().paragraph(
            Para::empty()
                .raw_run(&Image::chart().alt("A chart").floating().xml())
                .raw_run(&Image::smart_art().alt("A diagram").floating().xml()),
        ),
    );
    let found = findings(&original);
    assert_eq!(of(&found, RuleId::Img004).len(), 2);
    let applied = fix_rule(&original, RuleId::Img004);
    let after = assert_fixed(&original, &applied, RuleId::Img004);
    assert!(of(&after, RuleId::Img004).is_empty());
    let d = drawings(fixed(&applied));
    assert!(d.iter().all(|d| !d.floating));
    assert_eq!(d[0].description.as_deref(), Some("A chart"));
    assert_eq!(d[1].description.as_deref(), Some("A diagram"));
}

#[test]
fn fix_does_not_offer_inline_for_groups_canvases_and_shapes() {
    for (name, image) in [
        ("group", Image::group()),
        ("canvas", Image::canvas()),
        ("shape", Image::shape()),
        ("vml picture", Image::vml_picture()),
    ] {
        let original = with_image(&image.alt("A drawing").floating());
        let found = findings(&original);
        assert_eq!(of(&found, RuleId::Img004).len(), 1, "{name}");
        let offers = fix::offers(&original, &FixOptions::default(), &FixtureFonts).unwrap();
        let offer = offers
            .iter()
            .find(|o| o.fix == FixId::MakeImageInline)
            .unwrap();
        match &offer.availability {
            Availability::Unavailable(reason) => {
                assert!(reason.contains("In Line with Text"), "{name}: {reason}");
            }
            other => panic!("{name}: {other:?}"),
        }
        let applied = apply(&original, &requests_for(&found, RuleId::Img004, ""));
        assert!(applied.bytes.is_none(), "{name}: nothing is written");
        assert_eq!(applied.refused.len(), 1, "{name}");
    }
    // A picture is offered.
    let offers = fix::offers(
        &fixture(RuleId::Img004, true),
        &FixOptions::default(),
        &FixtureFonts,
    )
    .unwrap();
    assert_eq!(offers[0].availability, Availability::Ready);
}

// ---------- S2-24, S2-25 MarkAnnotationsDecorative ----------

#[test]
fn ask_mark_annotations_decorative() {
    let original = fixture(RuleId::Img007, true);
    let found = of(&findings(&original), RuleId::Img007);
    assert_eq!(found.len(), 1);
    let applied = apply(&original, &[FixRequest::confirm(&found[0])]);
    let after = assert_fixed(&original, &applied, RuleId::Img007);
    assert!(of(&after, RuleId::Img007).is_empty());
    let d = drawings(fixed(&applied));
    assert_eq!(d.len(), 4);
    assert_eq!(
        d[0].description.as_deref(),
        Some(GOOD_ALT),
        "the picture's alt text is unchanged"
    );
    assert!(!d[0].decorative);
    assert!(
        d[1..].iter().all(|d| d.decorative),
        "each shape is decorative"
    );
    let xml = part(fixed(&applied), DOCUMENT);
    assert_eq!(xml.matches("<adec:decorative").count(), 3);
    assert_eq!(
        xml.matches("xmlns:adec=").count(),
        3,
        "the namespace is declared"
    );

    // With the shapes' docPr written as <wp:docPr .../>, the element is opened.
    let copy = replace_part(&original, DOCUMENT, |t| {
        t.replace(r#"name="Object 1"></wp:docPr>"#, r#"name="Object 1"/>"#)
    });
    let found = of(&findings(&copy), RuleId::Img007);
    let applied = apply(&copy, &[FixRequest::confirm(&found[0])]);
    assert_fixed(&copy, &applied, RuleId::Img007);
    assert!(drawings(fixed(&applied))[1..].iter().all(|d| d.decorative));

    // Without the confirmation, nothing is written.
    let applied = apply(&original, &[FixRequest::decorative(&found[0])]);
    assert!(applied.bytes.is_none());
    let applied = apply(&original, &[]);
    assert!(applied.bytes.is_none());
}

#[test]
fn ask_mark_annotations_waits_for_the_alt_text_of_the_picture() {
    let original = build(
        &clean().paragraph(
            Para::empty()
                .raw_run(&Image::picture().xml())
                .raw_run(&Image::shape().floating().xml()),
        ),
    );
    let found = findings(&original);
    assert_eq!(
        of(&found, RuleId::Img001).len(),
        1,
        "the picture needs alt text first"
    );
    let marks = of(&found, RuleId::Img007);
    assert_eq!(marks.len(), 1);
    let applied = apply(&original, &[FixRequest::confirm(&marks[0])]);
    assert!(applied.bytes.is_none());
    assert!(
        applied.refused[0].reason.contains("IMG-001"),
        "{:?}",
        applied.refused
    );

    // After the alt text is typed, the fix can run (in a second run).
    let first = apply(
        &original,
        &requests_for(
            &found,
            RuleId::Img001,
            "A printer with a red box on the power button",
        ),
    );
    let step2 = fixed(&first).to_vec();
    let found = findings(&step2);
    let marks = of(&found, RuleId::Img007);
    let applied = apply(&step2, &[FixRequest::confirm(&marks[0])]);
    let after = assert_fixed(&step2, &applied, RuleId::Img007);
    assert!(of(&after, RuleId::Img007).is_empty());
}

#[test]
fn ask_mark_annotations_for_vml_shapes() {
    let two_vml = |alt: Option<&str>| {
        let shape = |s: Image| match alt {
            Some(a) => s.alt(a),
            None => s,
        };
        build(
            &clean().paragraph(
                Para::empty()
                    .raw_run(&Image::picture().alt(GOOD_ALT).xml())
                    .raw_run(&shape(Image::vml_shape().floating()).xml())
                    .raw_run(&shape(Image::vml_shape().floating()).xml()),
            ),
        )
    };
    let original = two_vml(None);
    let found = findings(&original);
    let marks = of(&found, RuleId::Img007);
    assert_eq!(marks.len(), 1);
    // The tool does not claim to mark them.
    let applied = apply(&original, &[FixRequest::confirm(&marks[0])]);
    assert!(applied.bytes.is_none());
    let reason = &applied.refused[0].reason;
    assert!(
        reason.contains("VML") && reason.contains("alt text") && reason.contains("convert"),
        "{reason}"
    );
    let offers = fix::offers(&original, &FixOptions::default(), &FixtureFonts).unwrap();
    let offer = offers
        .iter()
        .find(|o| o.fix == FixId::MarkAnnotationsDecorative)
        .unwrap();
    assert!(matches!(offer.availability, Availability::Unavailable(_)));
    // IMG-007 is gone after the shapes have alt text (typed in Word).
    assert!(of(&findings(&two_vml(Some("A red arrow"))), RuleId::Img007).is_empty());
}

// ---------- S2-26 DeleteEmptyParagraphs ----------

#[test]
fn fix_delete_empty_paragraphs() {
    // A row of 2: one is deleted.
    let original = fixture(RuleId::Txb004, true);
    let applied = fix_rule(&original, RuleId::Txb004);
    let after = assert_fixed(&original, &applied, RuleId::Txb004);
    assert!(of(&after, RuleId::Txb004).is_empty());
    let m = model(fixed(&applied));
    assert_eq!(m.paragraphs.len(), model(&original).paragraphs.len() - 1);
    assert_eq!(
        texts(fixed(&applied)),
        [
            "Test guide",
            "This is a short paragraph of body text.",
            "First paragraph.",
            "",
            "Second paragraph."
        ]
    );

    // A row of 4: three are deleted, one stays.
    let four = build(
        &clean()
            .para("First.")
            .paragraph(Para::empty())
            .paragraph(Para::empty())
            .paragraph(Para::empty())
            .paragraph(Para::empty())
            .para("Second."),
    );
    let applied = fix_rule(&four, RuleId::Txb004);
    assert_fixed(&four, &applied, RuleId::Txb004);
    assert_eq!(
        model(fixed(&applied)).paragraphs.len(),
        model(&four).paragraphs.len() - 3
    );

    // Two rows in one document.
    let two = build(
        &clean()
            .para("First.")
            .paragraph(Para::empty())
            .paragraph(Para::empty())
            .para("Second.")
            .paragraph(Para::empty())
            .paragraph(Para::empty())
            .paragraph(Para::empty())
            .para("Third."),
    );
    assert_eq!(of(&findings(&two), RuleId::Txb004).len(), 2);
    let applied = fix_rule(&two, RuleId::Txb004);
    assert_fixed(&two, &applied, RuleId::Txb004);
    assert_eq!(
        model(fixed(&applied)).paragraphs.len(),
        model(&two).paragraphs.len() - 3
    );
    assert_eq!(
        texts(fixed(&applied))
            .iter()
            .filter(|t| t.is_empty())
            .count(),
        2
    );
}

#[test]
fn fix_delete_empty_paragraphs_keeps_what_is_not_empty() {
    // A paragraph with a page break ends the row and is not deleted.
    let with_break = build(
        &clean()
            .paragraph(Para::empty())
            .paragraph(Para::empty())
            .paragraph(Para::empty().raw_run(r#"<w:r><w:br w:type="page"/></w:r>"#))
            .para("After."),
    );
    let applied = fix_rule(&with_break, RuleId::Txb004);
    assert_fixed(&with_break, &applied, RuleId::Txb004);
    assert!(part(fixed(&applied), DOCUMENT).contains(r#"<w:br w:type="page"/>"#));
    assert_eq!(
        model(fixed(&applied)).paragraphs.len(),
        model(&with_break).paragraphs.len() - 1
    );

    // A paragraph with a drawing is not empty, so it ends the row.
    let with_picture = build(
        &clean()
            .paragraph(Para::empty())
            .paragraph(Para::empty().raw_run(&Image::picture().alt("A").xml()))
            .paragraph(Para::empty())
            .para("After."),
    );
    assert!(of(&findings(&with_picture), RuleId::Txb004).is_empty());

    // An empty paragraph that holds a bookmark is not deleted.
    let with_bookmark =
        build(
            &clean()
                .paragraph(Para::empty())
                .paragraph(Para::empty().raw_run(
                    r#"<w:bookmarkStart w:id="4" w:name="Here"/><w:bookmarkEnd w:id="4"/>"#,
                ))
                .para("After."),
        );
    let found = findings(&with_bookmark);
    assert_eq!(of(&found, RuleId::Txb004).len(), 1);
    let applied = apply(&with_bookmark, &requests_for(&found, RuleId::Txb004, ""));
    assert!(applied.bytes.is_none());
    assert!(
        applied.refused[0].reason.contains("bookmark"),
        "{:?}",
        applied.refused
    );
}

// ---------- S2-27, S2-28, S2-29 tables ----------

fn good_table() -> Table {
    Table::new(&[&["Setting", "Value"], &["Paper", "A4"], &["Colour", "Mono"]])
}

#[test]
fn fix_mark_header_row() {
    let original = fixture(RuleId::Tbl001, true);
    let applied = fix_rule(&original, RuleId::Tbl001);
    let after = assert_fixed(&original, &applied, RuleId::Tbl001);
    assert!(of(&after, RuleId::Tbl001).is_empty());
    let xml = part(fixed(&applied), DOCUMENT);
    assert!(
        xml.contains("<w:trPr><w:cantSplit/><w:tblHeader/></w:trPr>"),
        "cantSplit stays: {xml}"
    );
    let m = model(fixed(&applied));
    assert!(m.tables[0].rows[0].header && m.tables[0].rows[0].cant_split);
    assert!(!m.tables[0].rows[1].header);

    // A w:trPr that has another property, and one with no w:trPr at all.
    for (name, properties) in [
        ("another property", r#"<w:trHeight w:val="300"/>"#),
        ("no trPr", ""),
        ("tblHeader off", r#"<w:tblHeader w:val="0"/>"#),
        (
            "tblHeader false",
            r#"<w:cantSplit/><w:tblHeader w:val="false"/>"#,
        ),
    ] {
        let original = build(
            &clean().table(
                good_table()
                    .no_header_row()
                    .raw_row_properties(0, properties),
            ),
        );
        let applied = fix_rule(&original, RuleId::Tbl001);
        let after = assert_fixed(&original, &applied, RuleId::Tbl001);
        assert!(of(&after, RuleId::Tbl001).is_empty(), "{name}");
        let m = model(fixed(&applied));
        assert!(m.tables[0].rows[0].header, "{name}");
        assert_eq!(
            m.tables[0].rows[0].cant_split,
            properties.contains("cantSplit"),
            "{name}"
        );
        assert_eq!(
            part(fixed(&applied), DOCUMENT)
                .matches("<w:tblHeader")
                .count(),
            1,
            "{name}"
        );
    }
}

#[test]
fn fix_mark_header_row_changes_the_right_table_only() {
    let inner = Table::new(&[&["Name", "Phone"], &["Ana", "555"]]).no_header_row();
    let original = build(&clean().table(good_table().nested(1, 1, &inner)));
    let found = of(&findings(&original), RuleId::Tbl001);
    assert_eq!(found.len(), 1, "only the inner table has no header row");
    let applied = apply(&original, &[FixRequest::confirm(&found[0])]);
    assert_fixed(&original, &applied, RuleId::Tbl001);
    let m = model(fixed(&applied));
    assert!(m.tables[0].rows[0].header);
    assert!(m.tables[1].nested && m.tables[1].rows[0].header);

    // Both tables lack the mark: the user picks the inner one.
    let both = build(&clean().table(good_table().no_header_row().nested(1, 1, &inner)));
    let found = of(&findings(&both), RuleId::Tbl001);
    assert_eq!(found.len(), 2);
    let applied = apply(&both, &[FixRequest::confirm(&found[1])]);
    assert_fixed(&both, &applied, RuleId::Tbl001);
    let m = model(fixed(&applied));
    assert!(
        !m.tables[0].rows[0].header,
        "the outer table is not changed"
    );
    assert!(m.tables[1].rows[0].header);
}

#[test]
fn fix_delete_empty_row() {
    let original = fixture(RuleId::Tbl005, true);
    let applied = fix_rule(&original, RuleId::Tbl005);
    let after = assert_fixed(&original, &applied, RuleId::Tbl005);
    assert!(of(&after, RuleId::Tbl005).is_empty());
    let m = model(fixed(&applied));
    assert_eq!(m.tables[0].rows.len(), 3);
    assert_eq!(
        applied.splices[DOCUMENT].len(),
        1,
        "one row is deleted and nothing else changes"
    );
    assert_eq!(applied.splices[DOCUMENT][0].text, "");

    let shapes = [
        (
            "two empty rows",
            Table::new(&[
                &["Setting", "Value"],
                &["", ""],
                &["Paper", "A4"],
                &["", ""],
                &["Ink", "Black"],
            ]),
            3,
        ),
        (
            "empty row last",
            Table::new(&[&["Setting", "Value"], &["Paper", "A4"], &["", ""]]),
            2,
        ),
        (
            "two empty rows in a row",
            Table::new(&[
                &["Setting", "Value"],
                &["", ""],
                &["", ""],
                &["Ink", "Black"],
            ]),
            2,
        ),
    ];
    for (name, table, rows) in shapes {
        let original = build(&clean().table(table));
        let applied = fix_rule(&original, RuleId::Tbl005);
        let after = assert_fixed(&original, &applied, RuleId::Tbl005);
        assert!(of(&after, RuleId::Tbl005).is_empty(), "{name}");
        assert_eq!(model(fixed(&applied)).tables[0].rows.len(), rows, "{name}");
    }

    // If one row is left, the re-run shows TBL-004 (a table of one row).
    let one_left = build(&clean().table(Table::new(&[&["Setting", "Value"], &["", ""]])));
    let applied = fix_rule(&one_left, RuleId::Tbl005);
    let after = findings(fixed(&applied));
    assert!(of(&after, RuleId::Tbl005).is_empty());
    assert_eq!(
        of(&after, RuleId::Tbl004).len(),
        1,
        "the one-row table is now reported"
    );
}

#[test]
fn fix_delete_empty_row_refuses_what_it_must_keep() {
    // Every row is empty: the table would have no row.
    let all_empty = build(&clean().table(Table::new(&[&["", ""], &["", ""]])));
    let found = findings(&all_empty);
    let applied = apply(&all_empty, &requests_for(&found, RuleId::Tbl005, ""));
    assert!(applied.bytes.is_none());
    assert!(
        applied.refused[0]
            .reason
            .contains("Delete the table in Word"),
        "{:?}",
        applied.refused
    );

    // An empty row that starts a vertical merge.
    let merged = build(
        &clean().table(
            Table::new(&[
                &["Setting", "Value"],
                &["", ""],
                &["", "x"],
                &["Ink", "Black"],
            ])
            .merge_down(1, 0, true),
        ),
    );
    let found = of(&findings(&merged), RuleId::Tbl005);
    assert_eq!(found.len(), 1);
    let applied = apply(&merged, &[FixRequest::confirm(&found[0])]);
    assert!(applied.bytes.is_none());
    assert!(
        applied.refused[0].reason.contains("merged"),
        "{:?}",
        applied.refused
    );
}

#[test]
fn fix_disallow_row_break() {
    let original = fixture(RuleId::Tbl006, true);
    let applied = fix_rule(&original, RuleId::Tbl006);
    let after = assert_fixed(&original, &applied, RuleId::Tbl006);
    assert!(of(&after, RuleId::Tbl006).is_empty());
    let m = model(fixed(&applied));
    assert!(
        m.tables[0].rows.iter().all(|r| r.cant_split),
        "every row has cantSplit"
    );
    assert!(m.tables[0].rows[0].header, "the header flag stays");
    assert!(
        part(fixed(&applied), DOCUMENT).contains("<w:trPr><w:cantSplit/><w:tblHeader/></w:trPr>")
    );

    for (name, table) in [
        (
            "trPr without cantSplit",
            good_table()
                .rows_can_split()
                .raw_row_properties(1, r#"<w:trHeight w:val="300"/>"#),
        ),
        (
            "cantSplit off",
            good_table()
                .rows_can_split()
                .raw_row_properties(2, r#"<w:cantSplit w:val="0"/>"#),
        ),
        ("only one row can split", good_table().row_can_split(1)),
    ] {
        let original = build(&clean().table(table));
        let applied = fix_rule(&original, RuleId::Tbl006);
        let after = assert_fixed(&original, &applied, RuleId::Tbl006);
        assert!(of(&after, RuleId::Tbl006).is_empty(), "{name}");
        let m = model(fixed(&applied));
        assert!(m.tables[0].rows.iter().all(|r| r.cant_split), "{name}");
        assert!(m.tables[0].rows[0].header, "{name}");
    }
}

#[test]
fn header_row_and_row_break_fixes_on_the_same_row_make_one_trpr() {
    let original = build(&clean().table(good_table().no_header_row().rows_can_split()));
    let found = findings(&original);
    let mut requests = requests_for(&found, RuleId::Tbl001, "");
    requests.extend(requests_for(&found, RuleId::Tbl006, ""));
    let applied = apply(&original, &requests);
    let after = assert_fixed(&original, &applied, RuleId::Tbl001);
    assert!(of(&after, RuleId::Tbl001).is_empty() && of(&after, RuleId::Tbl006).is_empty());
    assert_eq!(
        part(fixed(&applied), DOCUMENT).matches("<w:trPr>").count(),
        3,
        "one for each row"
    );
}

// ---------- S2-30 AlignLeft ----------

#[test]
fn fix_align_left() {
    let original = fixture(RuleId::Txt002, true);
    let applied = fix_rule(&original, RuleId::Txt002);
    let after = assert_fixed(&original, &applied, RuleId::Txt002);
    assert!(of(&after, RuleId::Txt002).is_empty());
    let m = model(fixed(&applied));
    assert_eq!(
        m.paragraphs[2].props.alignment,
        adapdf::model::props::Alignment::Left
    );
    let splices = &applied.splices[DOCUMENT];
    assert_eq!(splices.len(), 1);
    assert_eq!(splices[0].text, "left", "only the value changes");

    // The style is justified: the paragraph gets its own setting, the style
    // is not changed, and the other paragraphs are not changed.
    let styles_justified = |bytes: &[u8]| {
        replace_part(bytes, STYLES, |t| {
            t.replace(
                r#"<w:name w:val="Normal"/>"#,
                r#"<w:name w:val="Normal"/><w:pPr><w:jc w:val="both"/></w:pPr>"#,
            )
        })
    };
    let original = styles_justified(&build(
        &clean()
            .para("Text of the first paragraph.")
            .para("Text of the second paragraph."),
    ));
    let found = findings(&original);
    assert_eq!(
        of(&found, RuleId::Txt002).len(),
        3,
        "the heading is a heading, so three body paragraphs"
    );
    let applied = apply(&original, &requests_for(&found, RuleId::Txt002, "")[1..2]);
    assert_fixed(&original, &applied, RuleId::Txt002);
    assert_eq!(
        applied
            .splices
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [DOCUMENT],
        "the style is not changed"
    );
    let m = model(fixed(&applied));
    use adapdf::model::props::Alignment;
    assert_eq!(m.paragraphs[2].props.alignment, Alignment::Left);
    assert_eq!(
        m.paragraphs[1].props.alignment,
        Alignment::Justify,
        "other paragraphs are not changed"
    );
    assert_eq!(m.paragraphs[3].props.alignment, Alignment::Justify);
    assert!(part(fixed(&applied), DOCUMENT)
        .contains(r#"<w:p><w:pPr><w:jc w:val="left"/></w:pPr><w:r><w:t xml:space="preserve">Text of the first paragraph."#));

    // A paragraph style that sets the alignment, with a style on the paragraph.
    let body = r#"<w:p><w:pPr><w:pStyle w:val="Just"/></w:pPr><w:r><w:t>Justified by its style.</w:t></w:r></w:p>"#;
    let styles = r#"<w:style w:type="paragraph" w:styleId="Just"><w:name w:val="Just"/><w:basedOn w:val="Normal"/><w:pPr><w:jc w:val="both"/></w:pPr></w:style>"#;
    let original = raw_docx(body, styles, false);
    let applied = fix_rule(&original, RuleId::Txt002);
    assert_fixed(&original, &applied, RuleId::Txt002);
    assert!(
        part(fixed(&applied), DOCUMENT)
            .contains(r#"<w:pPr><w:pStyle w:val="Just"/><w:jc w:val="left"/></w:pPr>"#)
    );
}

#[test]
fn heading_style_and_align_left_on_the_same_paragraph_make_one_ppr() {
    let original = build(
        &clean()
            .paragraph(Para::new("Setting up the printer").bold().justify())
            .para("Plug it in."),
    );
    let found = findings(&original);
    let mut requests = requests_for(&found, RuleId::Hdg003, "");
    requests.extend(requests_for(&found, RuleId::Txt002, ""));
    assert_eq!(requests.len(), 2);
    let applied = apply(&original, &requests);
    assert_fixed(&original, &applied, RuleId::Hdg003);
    let after = findings(fixed(&applied));
    assert!(of(&after, RuleId::Txt002).is_empty());
    assert!(
        part(fixed(&applied), DOCUMENT)
            .contains(r#"<w:pPr><w:pStyle w:val="Heading2"/><w:jc w:val="left"/></w:pPr>"#)
    );
}

// ---------- offers ----------

#[test]
fn offers_list_each_fixable_finding_with_its_kind() {
    let original = fixture(RuleId::Doc001, true);
    let offers = fix::offers(&original, &FixOptions::default(), &FixtureFonts).unwrap();
    let offer = offers.iter().find(|o| o.fix == FixId::SetTitle).unwrap();
    assert!(offer.ask);
    assert_eq!(offer.availability, Availability::Ready);
    assert_eq!(offer.finding.suggestion.as_deref(), Some("Test guide"));
    assert!(offers.iter().all(|o| o.fix != FixId::UpdateFields));
    // A text for the request is never taken from the suggestion.
    assert!(matches!(
        FixRequest::confirm(&offer.finding).input,
        FixInput::Confirm
    ));
}

#[test]
fn offers_tell_identical_findings_apart() {
    let original = build(
        &clean().paragraph(
            Para::empty()
                .raw_run(&Image::picture().xml())
                .raw_run(&Image::picture().xml()),
        ),
    );
    let offers = fix::offers(&original, &FixOptions::default(), &FixtureFonts).unwrap();
    let alt: Vec<_> = offers
        .iter()
        .filter(|o| o.fix == FixId::SetAltText)
        .collect();
    assert_eq!(alt.len(), 2);
    assert_eq!(alt[0].finding, alt[1].finding, "the findings are the same");
    assert_eq!((alt[0].position, alt[1].position), (0, 1));
    let applied = apply(
        &original,
        &[alt[1].request(FixInput::Text("Second".into()))],
    );
    let alts: Vec<_> = drawings(fixed(&applied))
        .into_iter()
        .map(|d| d.description)
        .collect();
    assert_eq!(alts, [None, Some("Second".to_string())]);
    // A request for a position that is not that finding is refused.
    let wrong = FixRequest::text(&alt[0].finding, "X").at(5);
    assert!(apply(&original, &[wrong]).bytes.is_none());
}

#[test]
fn two_fixes_on_the_same_drawing_do_not_overlap() {
    // A floating picture with no alt text: IMG-001 and IMG-004 both fire.
    let original = with_image(&Image::picture().floating());
    let found = findings(&original);
    let mut requests = requests_for(&found, RuleId::Img001, "A printer");
    requests.extend(requests_for(&found, RuleId::Img004, ""));
    assert_eq!(requests.len(), 2);
    let applied = apply(&original, &requests);
    assert_only_the_splices_changed(&original, &applied);
    let after = findings(fixed(&applied));
    assert!(of(&after, RuleId::Img001).is_empty() && of(&after, RuleId::Img004).is_empty());
    let d = drawings(fixed(&applied));
    assert!(!d[0].floating);
    assert_eq!(d[0].description.as_deref(), Some("A printer"));
}

#[test]
fn new_properties_go_in_schema_order_in_a_busy_paragraph_and_row() {
    // A paragraph the way Word writes it, with revision ids and several
    // properties. Its style is justified.
    let body = concat!(
        r#"<w:p w:rsidR="00AB12CD" w:rsidRDefault="00AB12CD"><w:pPr><w:keepNext/><w:spacing w:after="0"/>"#,
        r#"<w:ind w:left="720"/><w:rPr><w:b/></w:rPr></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>Setting up the printer</w:t></w:r></w:p>"#,
        r#"<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w="4680"/></w:tblGrid>"#,
        r#"<w:tr w:rsidR="00AB12CD"><w:tblPrEx><w:tblW w:w="0" w:type="auto"/></w:tblPrEx>"#,
        r#"<w:tc><w:p><w:r><w:t>Setting</w:t></w:r></w:p></w:tc></w:tr>"#,
        r#"<w:tr><w:trPr><w:trHeight w:val="300"/><w:ins w:id="1" w:author="A" w:date="2020-01-01T00:00:00Z"/></w:trPr>"#,
        r#"<w:tc><w:p><w:r><w:t>Paper</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p/>"#,
    );
    let styles = r#"<w:style w:type="paragraph" w:styleId="Just"><w:name w:val="Just"/><w:basedOn w:val="Normal"/><w:pPr><w:jc w:val="both"/></w:pPr></w:style>"#;
    let original = replace_part(&raw_docx(body, styles, false), DOCUMENT, |t| {
        t.replace(
            "<w:pPr><w:keepNext/>",
            r#"<w:pPr><w:pStyle w:val="Just"/><w:keepNext/>"#,
        )
    });
    let found = findings(&original);
    let mut requests = requests_for(&found, RuleId::Txt002, "");
    requests.extend(requests_for(&found, RuleId::Hdg003, ""));
    requests.extend(requests_for(&found, RuleId::Tbl001, ""));
    requests.extend(requests_for(&found, RuleId::Tbl006, ""));
    assert_eq!(requests.len(), 4, "{found:#?}");
    let applied = apply(&original, &requests);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_only_the_splices_changed(&original, &applied);
    let xml = part(fixed(&applied), DOCUMENT);
    // jc after ind, before rPr; pStyle was already there and is replaced.
    assert!(
        xml.contains(r#"<w:ind w:left="720"/><w:jc w:val="left"/><w:rPr>"#),
        "{xml}"
    );
    assert!(
        xml.contains(r#"<w:pStyle w:val="Heading1"/><w:keepNext/>"#),
        "{xml}"
    );
    // A new w:trPr goes after w:tblPrEx. The second row has trHeight and w:ins:
    // the new properties go before w:ins.
    assert!(
        xml.contains(r#"</w:tblPrEx><w:trPr><w:cantSplit/><w:tblHeader/></w:trPr>"#),
        "{xml}"
    );
    assert!(
        xml.contains(r#"<w:trPr><w:cantSplit/><w:trHeight w:val="300"/><w:ins "#),
        "{xml}"
    );
    let after = findings(fixed(&applied));
    for rule in [
        RuleId::Txt002,
        RuleId::Hdg003,
        RuleId::Tbl001,
        RuleId::Tbl006,
    ] {
        assert!(of(&after, rule).is_empty(), "{rule}");
    }
}
