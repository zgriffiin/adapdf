use super::props::{Alignment, Color};
use super::theme::Rgb;
use super::*;
use crate::fixtures::{DocxBuilder, Para, Run, clean, zip_stored};

const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing""#;

/// A package made from raw parts, to test XML the builder does not write.
fn load(parts: &[(&str, &str)]) -> Document {
    let entries: Vec<(&str, &[u8])> = parts
        .iter()
        .map(|(name, text)| (*name, text.as_bytes()))
        .collect();
    Document::from_bytes(&zip_stored(&entries)).unwrap()
}

fn load_body(body: &str) -> Document {
    let xml = format!("<w:document {NS}><w:body>{body}</w:body></w:document>");
    load(&[("word/document.xml", &xml)])
}

#[test]
fn paragraphs_are_in_order_with_styles_and_headings() {
    let doc =
        Document::from_bytes(&clean().heading(2, "Second").para("Body text").build()).unwrap();
    let texts: Vec<&str> = doc.paragraphs.iter().map(|p| p.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Test guide",
            "This is a short paragraph of body text.",
            "Second",
            "Body text"
        ]
    );
    assert_eq!(doc.paragraphs[0].heading_level, Some(1));
    assert_eq!(doc.paragraphs[0].style_name.as_deref(), Some("heading 1"));
    assert_eq!(doc.paragraphs[1].heading_level, None);
    assert_eq!(doc.paragraphs[1].style_id.as_deref(), Some("Normal"));
    assert_eq!(doc.paragraphs[2].heading_level, Some(2));
    assert_eq!(doc.paragraphs[3].index, 3);
}

#[test]
fn effective_properties_follow_style_inheritance() {
    let doc = Document::from_bytes(
        &DocxBuilder::new()
            .language("en-US")
            .body_size(24)
            .heading(1, "Big")
            .para("Plain")
            .paragraph(Para::new("Direct").size(40).lang("es-ES"))
            .build(),
    )
    .unwrap();
    let heading = &doc.paragraphs[0].runs[0].props;
    assert!(heading.bold, "Heading 1 is bold");
    assert_eq!(heading.size_half_points, 32);
    assert_eq!(
        heading.lang.val.as_deref(),
        Some("en-US"),
        "from docDefaults"
    );
    assert_eq!(doc.paragraphs[0].props.outline_level, Some(1));
    let plain = &doc.paragraphs[1].runs[0].props;
    assert!(!plain.bold);
    assert_eq!(plain.size_half_points, 24);
    assert_eq!(plain.color, Color::Auto);
    let direct = &doc.paragraphs[2].runs[0].props;
    assert_eq!(direct.size_half_points, 40, "direct formatting wins");
    assert_eq!(direct.lang.val.as_deref(), Some("es-ES"));
    assert_eq!(doc.body_run.size_half_points, 24);
}

#[test]
fn character_styles_and_theme_colours_are_resolved() {
    let styles = format!(
        r#"<w:styles {NS}><w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val="22"/></w:rPr></w:rPrDefault></w:docDefaults>
        <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
        <w:style w:type="character" w:styleId="Accent"><w:name w:val="Accent"/><w:rPr><w:b/><w:color w:themeColor="accent1"/></w:rPr></w:style></w:styles>"#
    );
    let theme = r#"<a:theme xmlns:a="x"><a:clrScheme><a:accent1><a:srgbClr val="156082"/></a:accent1></a:clrScheme></a:theme>"#;
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#;
    let document = format!(
        r#"<w:document {NS}><w:body><w:p><w:r><w:rPr><w:rStyle w:val="Accent"/></w:rPr><w:t>Styled</w:t></w:r></w:p></w:body></w:document>"#
    );
    let doc = load(&[
        ("word/document.xml", &document),
        ("word/styles.xml", &styles),
        ("word/theme/theme1.xml", theme),
        ("word/_rels/document.xml.rels", rels),
    ]);
    let props = &doc.paragraphs[0].runs[0].props;
    assert!(props.bold);
    assert_eq!(props.color, Color::Rgb(Rgb::from_hex("156082").unwrap()));
}

#[test]
fn run_text_covers_tabs_breaks_symbols_and_hyperlinks() {
    let doc = load_body(
        r#"<w:p><w:r><w:t>A</w:t><w:tab/><w:t>B</w:t><w:br/><w:noBreakHyphen/><w:sym w:font="Symbol" w:char="F0B7"/></w:r>
        <w:hyperlink><w:r><w:t>link</w:t></w:r></w:hyperlink>
        <w:sdt><w:sdtContent><w:r><w:t>control</w:t></w:r></w:sdtContent></w:sdt>
        <w:del><w:r><w:delText>gone</w:delText></w:r></w:del></w:p>"#,
    );
    assert_eq!(doc.paragraphs[0].text, "A\tB\n-\u{F0B7}linkcontrol");
}

#[test]
fn table_paragraphs_are_flagged() {
    let doc = Document::from_bytes(
        &clean()
            .paragraph(Para::new("Cell").in_table())
            .para("After")
            .build(),
    )
    .unwrap();
    let cell = doc.paragraphs.iter().find(|p| p.text == "Cell").unwrap();
    assert!(cell.in_table);
    assert!(
        !doc.paragraphs
            .iter()
            .find(|p| p.text == "After")
            .unwrap()
            .in_table
    );
}

#[test]
fn text_box_paragraphs_are_not_body_paragraphs() {
    let doc = load_body(
        r#"<w:p><w:r><w:t>Before</w:t></w:r>
          <w:r><mc:AlternateContent>
            <mc:Choice Requires="wps"><w:drawing><wp:anchor><wp:docPr id="1" name="Box"/>
              <w:txbxContent><w:p><w:r><w:t>Boxed text</w:t></w:r></w:p></w:txbxContent></wp:anchor></w:drawing></mc:Choice>
            <mc:Fallback><w:pict><w:txbxContent><w:p><w:r><w:t>Boxed text</w:t></w:r></w:p></w:txbxContent></w:pict></mc:Fallback>
          </mc:AlternateContent></w:r></w:p>
        <w:p><w:r><w:t>After</w:t></w:r></w:p>"#,
    );
    let texts: Vec<&str> = doc.paragraphs.iter().map(|p| p.text.as_str()).collect();
    assert_eq!(texts, ["Before", "After"]);
    let drawings = &doc.paragraphs[0].drawings;
    assert_eq!(drawings.len(), 1, "the fallback copy is not counted");
    assert_eq!(drawings[0].kind, DrawingKind::Anchored);
    assert!(drawings[0].text_box);
    assert_eq!(drawings[0].text, "Boxed text");
}

#[test]
fn drawings_are_classified() {
    let doc = load_body(
        r#"<w:p><w:r><w:drawing><wp:inline><wp:docPr id="1" name="Picture"/></wp:inline></w:drawing></w:r></w:p>
        <w:p><w:r><w:pict/></w:r></w:p>"#,
    );
    let inline = &doc.paragraphs[0].drawings;
    assert_eq!((inline.len(), inline[0].kind), (1, DrawingKind::Inline));
    assert!(!inline[0].text_box);
    let legacy = &doc.paragraphs[1].drawings;
    assert_eq!((legacy.len(), legacy[0].kind), (1, DrawingKind::Legacy));
    assert!(!legacy[0].text_box);
    assert!(!doc.paragraphs[0].is_blank(), "a drawing is content");
}

#[test]
fn blank_paragraphs() {
    let doc = Document::from_bytes(
        &clean()
            .paragraph(Para::empty())
            .para("   ")
            .para("text")
            .build(),
    )
    .unwrap();
    assert!(doc.paragraphs[2].is_blank());
    assert!(doc.paragraphs[3].is_blank());
    assert!(!doc.paragraphs[4].is_blank());
}

#[test]
fn unknown_style_falls_back_to_the_default() {
    let doc = Document::from_bytes(
        &DocxBuilder::new()
            .paragraph(Para::new("x").style("Nope"))
            .build(),
    )
    .unwrap();
    assert_eq!(doc.paragraphs[0].style_id.as_deref(), Some("Normal"));
}

#[test]
fn lists_come_from_the_style_with_the_level_from_the_numbering() {
    let doc = Document::from_bytes(
        &DocxBuilder::new()
            .paragraph(Para::new("one").style("ListNumber"))
            .paragraph(Para::new("dot").style("ListBullet"))
            .paragraph(Para::new("direct").num(1, 0))
            .paragraph(Para::new("plain"))
            .build(),
    )
    .unwrap();
    let num = |i: usize| doc.paragraphs[i].props.num;
    assert_eq!(num(0).map(|n| (n.num_id, n.level)), Some((1, 0)));
    assert!(doc.numbering.is_numbered(num(0).unwrap()));
    assert!(
        !doc.numbering.is_numbered(num(1).unwrap()),
        "bullets are not numbered"
    );
    assert_eq!(num(2).map(|n| n.num_id), Some(1));
    assert_eq!(num(3), None);
}

#[test]
fn alignment_is_effective() {
    let doc = Document::from_bytes(
        &DocxBuilder::new()
            .paragraph(Para::new("a").justify())
            .para("b")
            .build(),
    )
    .unwrap();
    assert_eq!(doc.paragraphs[0].props.alignment, Alignment::Justify);
    assert_eq!(doc.paragraphs[1].props.alignment, Alignment::Left);
}

#[test]
fn core_title_and_missing_title() {
    let with = Document::from_bytes(&DocxBuilder::new().title("My guide").build()).unwrap();
    assert_eq!(with.core.title.as_deref(), Some("My guide"));
    let empty = Document::from_bytes(&DocxBuilder::new().title("").build()).unwrap();
    assert_eq!(empty.core.title.as_deref(), Some(""));
    let none = Document::from_bytes(&DocxBuilder::new().build()).unwrap();
    assert_eq!(none.core.title, None);
}

#[test]
fn attached_template_is_percent_decoded() {
    let doc = Document::from_bytes(
        &DocxBuilder::new()
            .template("Adapdf Training Guide.dotx")
            .build(),
    )
    .unwrap();
    let target = doc.settings.attached_template.unwrap();
    assert!(target.starts_with("file:///C:"), "{target}");
    assert!(target.ends_with(r"\Adapdf Training Guide.dotx"), "{target}");
    let none = Document::from_bytes(&DocxBuilder::new().build()).unwrap();
    assert_eq!(none.settings.attached_template, None);
}

#[test]
fn percent_decoding() {
    assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
    assert_eq!(percent_decode("100%"), "100%");
    assert_eq!(percent_decode("%zz"), "%zz");
    assert_eq!(percent_decode("%C3%A9"), "é");
}

#[test]
fn default_language_comes_from_the_styles_then_the_theme_font_language() {
    let none = Document::from_bytes(&DocxBuilder::new().build()).unwrap();
    assert_eq!(none.default_language, None);
    let styles = Document::from_bytes(
        &DocxBuilder::new()
            .language("fr-CA")
            .theme_font_lang("en-US")
            .build(),
    )
    .unwrap();
    assert_eq!(styles.default_language.as_deref(), Some("fr-CA"));
    let theme = Document::from_bytes(&DocxBuilder::new().theme_font_lang("en-US").build()).unwrap();
    assert_eq!(theme.default_language.as_deref(), Some("en-US"));
}

#[test]
fn counts_tracked_changes_and_comments() {
    let none = Document::from_bytes(&clean().build()).unwrap();
    assert_eq!(none.markup, Markup::default());
    let marked = Document::from_bytes(
        &clean()
            .paragraph(Para::empty().then(Run::new("new").inserted()))
            .paragraph(Para::new("Commented").comment("First").comment("Second"))
            .build(),
    )
    .unwrap();
    assert_eq!(marked.markup.tracked_changes, 1);
    assert_eq!(marked.markup.comments, 2);
}

#[test]
fn tracked_changes_in_headers_are_counted() {
    let header = format!(
        r#"<w:hdr {NS}><w:p><w:ins w:id="1"><w:r><w:t>x</w:t></w:r></w:ins></w:p></w:hdr>"#
    );
    let rels = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;
    let document = format!("<w:document {NS}><w:body><w:p/></w:body></w:document>");
    let doc = load(&[
        ("word/document.xml", &document),
        ("word/header1.xml", &header),
        ("word/_rels/document.xml.rels", rels),
    ]);
    assert_eq!(doc.header_footer_parts, ["word/header1.xml"]);
    assert_eq!(doc.markup.tracked_changes, 1);
}

#[test]
fn sections_and_columns() {
    let doc = Document::from_bytes(
        &DocxBuilder::new()
            .paragraph(Para::new("one").ends_section(2))
            .para("two")
            .build(),
    )
    .unwrap();
    let columns: Vec<u32> = doc.sections.iter().map(|s| s.columns).collect();
    assert_eq!(columns, [2, 1]);
    assert_eq!(doc.paragraphs[0].section, 0);
    assert_eq!(doc.paragraphs[1].section, 1);
}

#[test]
fn paragraph_range_points_at_the_xml_element() {
    let package = Package::from_bytes(&clean().build()).unwrap();
    let doc = Document::load(&package).unwrap();
    let text = package.text("word/document.xml").unwrap().unwrap();
    for p in &doc.paragraphs {
        let xml = &text[p.range.clone()];
        assert!(xml.starts_with("<w:p>") && xml.ends_with("</w:p>"), "{xml}");
        assert!(xml.contains(&p.text));
    }
}

#[test]
fn headings_skip_blank_headings_and_heading_above_finds_the_nearest() {
    let doc = Document::from_bytes(
        &clean()
            .heading(2, "Real")
            .para("text")
            .heading(2, "")
            .para("more")
            .build(),
    )
    .unwrap();
    let headings: Vec<&str> = doc.headings().map(|h| h.text.as_str()).collect();
    assert_eq!(headings, ["Test guide", "Real"]);
    let more = doc
        .paragraphs
        .iter()
        .position(|p| p.text == "more")
        .unwrap();
    assert_eq!(doc.heading_above(more).unwrap().text, "Real");
    assert!(doc.heading_above(0).is_none());
    assert_eq!(doc.word_count(), 2 + 8 + 1 + 1 + 1);
}

#[test]
fn bad_xml_is_an_error_naming_the_part() {
    let bytes = zip_stored(&[("word/document.xml", b"<w:document>".as_slice())]);
    let err = Document::from_bytes(&bytes).unwrap_err();
    assert!(err.to_string().contains("word/document.xml"), "{err}");
}

#[test]
fn a_document_part_that_is_not_a_word_document_is_an_error() {
    let bytes = zip_stored(&[("word/document.xml", b"<html/>".as_slice())]);
    let err = Document::from_bytes(&bytes).unwrap_err();
    assert!(matches!(err, Error::NotDocx(_)), "{err:?}");
}

#[test]
fn a_utf8_byte_order_mark_is_accepted() {
    let xml = format!(
        "\u{FEFF}<w:document {NS}><w:body><w:p><w:r><w:t>Hi</w:t></w:r></w:p></w:body></w:document>"
    );
    let doc = load(&[("word/document.xml", &xml)]);
    assert_eq!(doc.paragraphs[0].text, "Hi");
}

// ---- headers and footers by section ---------------------------------------

const REL_BASE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// A document with these sections. Each section is the `sectPr` content of its
/// last paragraph (the last one is the body's own). The header and footer parts
/// are `word/header1.xml`, `word/header2.xml`, `word/footer1.xml`, and each has the
/// relationship id `rId1`, `rId2`, `rId3` in that order.
fn doc_with_sections(sections: &[&str]) -> Document {
    let mut body = String::new();
    let (last, earlier) = sections.split_last().unwrap();
    for (i, section) in earlier.iter().enumerate() {
        body.push_str(&format!(
            "<w:p><w:pPr><w:sectPr>{section}</w:sectPr></w:pPr><w:r><w:t>Section {}</w:t></w:r></w:p>",
            i + 1
        ));
    }
    body.push_str(&format!(
        "<w:p><w:r><w:t>Last</w:t></w:r></w:p><w:sectPr>{last}</w:sectPr>"
    ));
    let document = format!("<w:document {NS}><w:body>{body}</w:body></w:document>");
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{REL_BASE}/header" Target="header1.xml"/><Relationship Id="rId2" Type="{REL_BASE}/header" Target="header2.xml"/><Relationship Id="rId3" Type="{REL_BASE}/footer" Target="footer1.xml"/></Relationships>"#
    );
    let part = |tag: &str, text: &str| {
        format!("<w:{tag} {NS}><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:{tag}>")
    };
    load(&[
        ("word/document.xml", &document),
        ("word/header1.xml", &part("hdr", "One")),
        ("word/header2.xml", &part("hdr", "Two")),
        ("word/footer1.xml", &part("ftr", "Foot")),
        ("word/_rels/document.xml.rels", &rels),
    ])
}

fn place(footer: bool, kind: HeaderFooterKind, sections: &[usize]) -> HeaderFooterPlace {
    HeaderFooterPlace {
        footer,
        kind,
        sections: sections.to_vec(),
    }
}

const HEADER1_DEFAULT: &str = r#"<w:headerReference w:type="default" r:id="rId1"/>"#;
const HEADER2_DEFAULT: &str = r#"<w:headerReference w:type="default" r:id="rId2"/>"#;

#[test]
fn a_section_names_its_headers_and_footers_with_their_kind() {
    let doc = doc_with_sections(&[
        r#"<w:headerReference w:type="default" r:id="rId1"/><w:headerReference w:type="first" r:id="rId2"/><w:footerReference r:id="rId3"/>"#,
    ]);
    assert_eq!(doc.sections.len(), 1);
    let refs: Vec<(&str, bool, HeaderFooterKind)> = doc.sections[0]
        .header_footers
        .iter()
        .map(|r| (r.part.as_str(), r.footer, r.kind))
        .collect();
    assert_eq!(
        refs,
        [
            ("word/header1.xml", false, HeaderFooterKind::Default),
            ("word/header2.xml", false, HeaderFooterKind::First),
            // No `w:type` is the default.
            ("word/footer1.xml", true, HeaderFooterKind::Default),
        ]
    );
    assert_eq!(
        doc.header_footer_places("word/header2.xml"),
        [place(false, HeaderFooterKind::First, &[0])]
    );
    // An even-pages header.
    let even = doc_with_sections(&[r#"<w:headerReference w:type="even" r:id="rId1"/>"#]);
    assert_eq!(
        even.header_footer_places("word/header1.xml"),
        [place(false, HeaderFooterKind::Even, &[0])]
    );
}

#[test]
fn each_section_has_its_own_header() {
    let doc = doc_with_sections(&[HEADER1_DEFAULT, HEADER2_DEFAULT]);
    assert_eq!(
        doc.header_footer_places("word/header1.xml"),
        [place(false, HeaderFooterKind::Default, &[0])]
    );
    assert_eq!(
        doc.header_footer_places("word/header2.xml"),
        [place(false, HeaderFooterKind::Default, &[1])]
    );
    // A part that no section names has no place.
    assert!(doc.header_footer_places("word/footer1.xml").is_empty());
}

#[test]
fn a_section_with_no_header_of_its_own_shows_the_one_before_it() {
    // Section 1 names header 1. Section 2 names nothing ("Link to Previous"), and
    // section 3 names header 2, so header 1 is shown in sections 1 and 2.
    let doc = doc_with_sections(&[HEADER1_DEFAULT, "", HEADER2_DEFAULT]);
    assert_eq!(
        doc.header_footer_places("word/header1.xml"),
        [place(false, HeaderFooterKind::Default, &[0, 1])]
    );
    assert_eq!(
        doc.header_footer_places("word/header2.xml"),
        [place(false, HeaderFooterKind::Default, &[2])]
    );
}

#[test]
fn a_part_that_two_sections_name_is_in_both() {
    let doc = doc_with_sections(&[HEADER1_DEFAULT, HEADER2_DEFAULT, HEADER1_DEFAULT]);
    assert_eq!(
        doc.header_footer_places("word/header1.xml"),
        [place(false, HeaderFooterKind::Default, &[0, 2])]
    );
}

#[test]
fn the_kinds_are_followed_apart() {
    // Section 1: a default header and a first-page header. Section 2 names only a
    // new default header, so the first-page header of section 1 goes on.
    let doc = doc_with_sections(&[
        r#"<w:headerReference w:type="default" r:id="rId1"/><w:headerReference w:type="first" r:id="rId2"/>"#,
        HEADER2_DEFAULT,
    ]);
    assert_eq!(
        doc.header_footer_places("word/header2.xml"),
        [
            place(false, HeaderFooterKind::Default, &[1]),
            place(false, HeaderFooterKind::First, &[0, 1]),
        ]
    );
}

#[test]
fn a_reference_to_a_part_that_does_not_exist_is_left_out() {
    let doc = doc_with_sections(&[r#"<w:headerReference w:type="default" r:id="rId99"/>"#]);
    assert!(doc.sections[0].header_footers.is_empty());
}

#[test]
fn the_place_of_a_header_or_footer_is_in_words() {
    use crate::findings::header_footer_place as words;
    // One section: no "of section 1".
    let one = doc_with_sections(&[
        r#"<w:headerReference w:type="default" r:id="rId1"/><w:headerReference w:type="first" r:id="rId2"/><w:footerReference w:type="even" r:id="rId3"/>"#,
    ]);
    assert_eq!(
        words(&one, "word/header1.xml"),
        "In the page header (all pages)"
    );
    assert_eq!(
        words(&one, "word/header2.xml"),
        "In the page header (first page)"
    );
    assert_eq!(
        words(&one, "word/footer1.xml"),
        "In the page footer (even pages)"
    );

    // Several sections: the section is named.
    let several = doc_with_sections(&[HEADER1_DEFAULT, HEADER2_DEFAULT]);
    assert_eq!(
        words(&several, "word/header2.xml"),
        "In the page header of section 2 (all pages)"
    );
    // A part that several sections show: they are listed.
    let shared = doc_with_sections(&[HEADER1_DEFAULT, "", HEADER2_DEFAULT, HEADER1_DEFAULT]);
    assert_eq!(
        words(&shared, "word/header1.xml"),
        "In the page header of sections 1, 2 and 4 (all pages)"
    );
    let two = doc_with_sections(&[HEADER1_DEFAULT, HEADER2_DEFAULT, HEADER1_DEFAULT]);
    assert_eq!(
        words(&two, "word/header1.xml"),
        "In the page header of sections 1 and 3 (all pages)"
    );
    // One part in two places: both are said.
    let both = doc_with_sections(&[
        r#"<w:headerReference w:type="default" r:id="rId1"/><w:headerReference w:type="first" r:id="rId1"/>"#,
    ]);
    assert_eq!(
        words(&both, "word/header1.xml"),
        "In the page header (all pages) and the page header (first page)"
    );
    // A part that no section uses.
    assert_eq!(
        words(&several, "word/footer1.xml"),
        "In a page footer that no section of the document uses"
    );
}

#[test]
fn the_places_of_the_other_locations_are_in_words() {
    use crate::findings::Location;
    assert_eq!(
        Location::document("docProps/core.xml").place,
        "The document's properties (File > Info)"
    );
    assert_eq!(
        Location::document("word/settings.xml").place,
        "Word's template setting (Developer > Document Template)"
    );
    assert_eq!(
        Location::document("word/document.xml").place,
        "The whole document"
    );
    // A paragraph keeps its form: the number, the text, and the heading above.
    let doc = Document::from_bytes(
        &clean()
            .heading(2, "Setup")
            .para("Press the red button.")
            .para("")
            .build(),
    )
    .unwrap();
    let place = |i: usize| Location::paragraph(&doc, &doc.paragraphs[i]).place;
    assert_eq!(place(0), "paragraph 1 \"Test guide\"");
    assert_eq!(
        place(3),
        "paragraph 4 \"Press the red button.\", after the heading \"Setup\""
    );
    // A paragraph with no text still says which one it is.
    assert_eq!(
        place(4),
        "paragraph 5 (no text), after the heading \"Setup\""
    );
    // A table says which one and what is under which heading.
    let table = Location::table(&doc, 2, "Name", Some(3));
    assert_eq!(
        table.place,
        "The table under the heading \"Setup\" (table 2, first cell \"Name\")"
    );
    assert_eq!(
        Location::table(&doc, 1, "", Some(0)).place,
        "The table (table 1)"
    );
    // A style says its name and where to change it.
    let styled = Location::style(&doc, &doc.paragraphs[3]);
    assert!(
        styled.place.starts_with("The style \"")
            && styled.place.contains(
                "\" (Home > Styles), first used in paragraph 4 \"Press the red button.\""
            ),
        "{}",
        styled.place
    );
    assert_eq!(styled.part, "word/styles.xml");
    // Other text in the place of the snippet (a text box).
    let boxed = Location::paragraph(&doc, &doc.paragraphs[3]).with_snippet("Note this.");
    assert_eq!(
        boxed.place,
        "paragraph 4 \"Note this.\", after the heading \"Setup\""
    );
    assert_eq!(boxed.snippet.as_deref(), Some("Note this."));
}

#[test]
fn txt_003_says_which_section_and_which_page_the_header_is_in() {
    use crate::rules::text::txt_003;
    // Section 2 has the header "Two" for all pages.
    let doc = doc_with_sections(&[HEADER1_DEFAULT_EMPTY, HEADER2_DEFAULT]);
    let found = txt_003(&doc);
    // The header of section 1 and the footer are not used by any section, and are
    // found too: header1 says so.
    let unused = found
        .iter()
        .find(|f| f.location.part == "word/header1.xml")
        .unwrap();
    assert_eq!(
        unused.location.place,
        "In a page header that no section of the document uses"
    );
    let f = found
        .iter()
        .find(|f| f.location.part == "word/header2.xml")
        .expect("a finding for the header of section 2");
    assert_eq!(
        f.location.place,
        "In the page header of section 2 (all pages)"
    );
    assert_eq!(f.location.part, "word/header2.xml");
    assert_eq!(f.location.snippet.as_deref(), Some("Two"));
    assert!(
        f.how_to_fix.starts_with(
            "Double-click the top of a page in section 2 to open the header. Delete \"Two\", or move it into the body of the document."
        ),
        "{}",
        f.how_to_fix
    );
    assert!(
        f.how_to_fix
            .ends_with("Keep only the title, page numbers and dates in the header.")
    );

    // A first-page header in the one section of the document.
    let doc = doc_with_sections(&[r#"<w:headerReference w:type="first" r:id="rId2"/>"#]);
    let found = txt_003(&doc);
    let f = found
        .iter()
        .find(|f| f.location.part == "word/header2.xml")
        .unwrap();
    assert_eq!(f.location.place, "In the page header (first page)");
    assert!(
        f.how_to_fix
            .starts_with("Double-click the top of the first page to open the header."),
        "{}",
        f.how_to_fix
    );

    // A footer.
    let doc = doc_with_sections(&[
        r#"<w:footerReference w:type="even" r:id="rId3"/>"#,
        r#"<w:footerReference w:type="even" r:id="rId3"/>"#,
    ]);
    let found = txt_003(&doc);
    let f = found
        .iter()
        .find(|f| f.location.part == "word/footer1.xml")
        .unwrap();
    assert_eq!(
        f.location.place,
        "In the page footer of sections 1 and 2 (even pages)"
    );
    assert!(
        f.how_to_fix.starts_with(
            "Double-click the bottom of an even-numbered page in section 1 to open the footer."
        ),
        "{}",
        f.how_to_fix
    );
    assert!(
        f.message.starts_with("The footer has text"),
        "{}",
        f.message
    );
}

const HEADER1_DEFAULT_EMPTY: &str = "";
