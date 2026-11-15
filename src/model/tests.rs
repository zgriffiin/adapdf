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
    assert_eq!(
        doc.paragraphs[0].drawings,
        [Drawing {
            kind: DrawingKind::Anchored,
            text_box: true
        }]
    );
}

#[test]
fn drawings_are_classified() {
    let doc = load_body(
        r#"<w:p><w:r><w:drawing><wp:inline><wp:docPr id="1" name="Picture"/></wp:inline></w:drawing></w:r></w:p>
        <w:p><w:r><w:pict/></w:r></w:p>"#,
    );
    assert_eq!(
        doc.paragraphs[0].drawings,
        [Drawing {
            kind: DrawingKind::Inline,
            text_box: false
        }]
    );
    assert_eq!(
        doc.paragraphs[1].drawings,
        [Drawing {
            kind: DrawingKind::Legacy,
            text_box: false
        }]
    );
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
