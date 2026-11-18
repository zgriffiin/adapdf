//! TXT-001 to TXT-004.

mod common;

use adapdf::findings::{Finding, FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{
    DocxBuilder, FixtureFonts, Para, Run, clean, complex_field_xml, page_x_of_y_xml, raw_docx,
    simple_field_xml, text_paragraph_xml,
};
use adapdf::rules::text::{HEADER_FOOTER_FIELDS, MIN_BODY_SIZE_HALF_POINTS};
use common::{assert_location, fixture_findings, of, rule_findings, the_failing_finding};

fn raw(body: &str, styles: &str, rule: RuleId) -> Vec<Finding> {
    adapdf::check_bytes(&raw_docx(body, styles, false), &FixtureFonts)
        .expect("a valid .docx")
        .into_iter()
        .filter(|f| f.rule_id == rule)
        .collect()
}

// ---- TXT-001 --------------------------------------------------------------

#[test]
fn txt_001_fails_for_small_body_text() {
    let f = the_failing_finding(RuleId::Txt001);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Small print at 9.5 pt."),
        Some("Test guide"),
    );
    assert!(f.message.contains("9.5 pt"), "{}", f.message);
}

#[test]
fn txt_001_passes_at_10_pt() {
    assert!(of(&fixture_findings(RuleId::Txt001, false), RuleId::Txt001).is_empty());
}

#[test]
fn txt_001_threshold_is_10_pt() {
    assert_eq!(MIN_BODY_SIZE_HALF_POINTS, 20);
    let at = |half_points: u32| clean().paragraph(Para::new("Small print.").size(half_points));
    assert_eq!(rule_findings(&at(19), RuleId::Txt001).len(), 1, "9.5 pt");
    assert!(rule_findings(&at(20), RuleId::Txt001).is_empty(), "10 pt");
    assert!(rule_findings(&at(21), RuleId::Txt001).is_empty(), "10.5 pt");
    assert_eq!(rule_findings(&at(2), RuleId::Txt001).len(), 1, "1 pt");
}

#[test]
fn txt_001_uses_the_size_that_the_style_gives() {
    let builder = DocxBuilder::new().body_size(19).para("Body text.");
    assert_eq!(rule_findings(&builder, RuleId::Txt001).len(), 1);
    let ok = DocxBuilder::new().body_size(20).para("Body text.");
    assert!(rule_findings(&ok, RuleId::Txt001).is_empty());
}

#[test]
fn txt_001_reports_a_paragraph_once() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("One ").size(18))
            .then(Run::new("two ").size(16))
            .then(Run::new("three.").size(18)),
    );
    let f = rule_findings(&builder, RuleId::Txt001);
    assert_eq!(f.len(), 1);
    assert!(
        f[0].message.contains("8 pt"),
        "the smallest run: {}",
        f[0].message
    );
}

#[test]
fn txt_001_one_small_run_in_a_normal_paragraph_is_reported() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("Normal text "))
            .then(Run::new("fine print").size(16)),
    );
    assert_eq!(rule_findings(&builder, RuleId::Txt001).len(), 1);
}

#[test]
fn txt_001_ignores_small_runs_with_no_text() {
    let builder = clean().paragraph(Para::empty().then(Run::new("  ").size(8)));
    assert!(rule_findings(&builder, RuleId::Txt001).is_empty());
}

#[test]
fn txt_001_checks_table_cells() {
    let builder = clean().paragraph(Para::new("In a cell.").size(16).in_table());
    assert_eq!(rule_findings(&builder, RuleId::Txt001).len(), 1);
}

#[test]
fn txt_001_skips_headings_titles_captions_and_tables_of_contents() {
    let builder = clean()
        .custom_style("Caption", "caption", None)
        .custom_style("TOC1", "toc 1", None)
        .custom_style("Subtitle", "Subtitle", None)
        .paragraph(Para::new("Small heading").style("Heading2").size(16))
        .paragraph(Para::new("Small title").style("Title").size(16))
        .paragraph(Para::new("Small caption").style("Caption").size(16))
        .paragraph(Para::new("Small entry").style("TOC1").size(16))
        .paragraph(Para::new("Small subtitle").style("Subtitle").size(16));
    assert!(rule_findings(&builder, RuleId::Txt001).is_empty());
}

#[test]
fn txt_001_headers_and_footers_are_not_body_text() {
    let builder = clean().header_xml(
        r#"<w:p><w:r><w:rPr><w:sz w:val="16"/></w:rPr><w:t>Adapdf test guide</w:t></w:r></w:p>"#,
    );
    assert!(rule_findings(&builder, RuleId::Txt001).is_empty());
}

// ---- TXT-002 --------------------------------------------------------------

#[test]
fn txt_002_fails_for_a_justified_paragraph() {
    let f = the_failing_finding(RuleId::Txt002);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Auto(FixId::AlignLeft));
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Text that is justified."),
        Some("Test guide"),
    );
}

#[test]
fn txt_002_passes_for_left_aligned_text() {
    assert!(of(&fixture_findings(RuleId::Txt002, false), RuleId::Txt002).is_empty());
}

const JUSTIFIED_STYLES: &str = r#"
  <w:style w:type="paragraph" w:styleId="Justified"><w:name w:val="Justified"/><w:basedOn w:val="Normal"/><w:pPr><w:jc w:val="both"/></w:pPr></w:style>
  <w:style w:type="paragraph" w:styleId="Justified2"><w:name w:val="Justified 2"/><w:basedOn w:val="Justified"/></w:style>
  <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:pPr><w:jc w:val="both"/><w:outlineLvl w:val="0"/></w:pPr></w:style>"#;

fn jc(style: &str, jc: &str) -> String {
    let style = if style.is_empty() {
        String::new()
    } else {
        format!(r#"<w:pStyle w:val="{style}"/>"#)
    };
    let jc = if jc.is_empty() {
        String::new()
    } else {
        format!(r#"<w:jc w:val="{jc}"/>"#)
    };
    format!("<w:p><w:pPr>{style}{jc}</w:pPr><w:r><w:t>Some text.</w:t></w:r></w:p>")
}

#[test]
fn txt_002_every_justified_value_counts() {
    for value in [
        "both",
        "distribute",
        "thaiDistribute",
        "lowKashida",
        "mediumKashida",
        "highKashida",
    ] {
        let found = raw(&jc("", value), "", RuleId::Txt002);
        assert_eq!(found.len(), 1, "{value}");
    }
    for value in ["left", "start", "center", "right", "end"] {
        assert!(
            raw(&jc("", value), "", RuleId::Txt002).is_empty(),
            "{value}"
        );
    }
}

#[test]
fn txt_002_is_effective_after_styles() {
    assert_eq!(
        raw(&jc("Justified", ""), JUSTIFIED_STYLES, RuleId::Txt002).len(),
        1
    );
    assert_eq!(
        raw(&jc("Justified2", ""), JUSTIFIED_STYLES, RuleId::Txt002).len(),
        1
    );
    // Direct left alignment wins over the style.
    assert!(raw(&jc("Justified", "left"), JUSTIFIED_STYLES, RuleId::Txt002).is_empty());
}

#[test]
fn txt_002_only_body_paragraphs_count() {
    assert!(raw(&jc("Heading1", ""), JUSTIFIED_STYLES, RuleId::Txt002).is_empty());
    let empty = clean().paragraph(Para::empty().justify());
    assert!(rule_findings(&empty, RuleId::Txt002).is_empty());
    let in_table = clean().paragraph(Para::new("In a cell.").justify().in_table());
    assert_eq!(rule_findings(&in_table, RuleId::Txt002).len(), 1);
}

// ---- TXT-003 --------------------------------------------------------------

fn header_findings(header: &str) -> Vec<Finding> {
    rule_findings(&clean().header_xml(header), RuleId::Txt003)
}

fn footer_findings(footer: &str) -> Vec<Finding> {
    rule_findings(&clean().footer_xml(footer), RuleId::Txt003)
}

#[test]
fn txt_003_fails_for_other_text_in_a_header() {
    let f = the_failing_finding(RuleId::Txt003);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    // A header or footer has no body paragraph. The snippet is the extra words.
    assert_location(
        &f,
        "word/header1.xml",
        None,
        Some("Parks and Recreation Department"),
        None,
    );
    assert!(f.message.contains("header"), "{}", f.message);
    // The place is in Word's words, and the part name stays in the data.
    assert_eq!(f.location.place, "In the page header (all pages)");
    assert!(
        f.how_to_fix
            .starts_with("Double-click the top of a page to open the header.")
    );
    assert!(
        f.how_to_fix
            .contains("Delete \"Parks and Recreation Department\"")
    );
}

#[test]
fn txt_003_passes_for_the_title_and_page_x_of_y() {
    assert!(of(&fixture_findings(RuleId::Txt003, false), RuleId::Txt003).is_empty());
}

#[test]
fn txt_003_uses_the_part_name_for_a_footer() {
    let f = footer_findings(&text_paragraph_xml("Confidential"));
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].location.part, "word/footer1.xml");
    assert!(f[0].message.contains("footer"), "{}", f[0].message);
    assert_eq!(f[0].location.place, "In the page footer (all pages)");
    assert!(
        f[0].how_to_fix
            .starts_with("Double-click the bottom of a page to open the footer.")
    );
}

#[test]
fn txt_003_page_number_and_date_fields_are_allowed_with_any_result() {
    assert_eq!(
        HEADER_FOOTER_FIELDS,
        [
            "PAGE",
            "NUMPAGES",
            "SECTIONPAGES",
            "DATE",
            "TIME",
            "CREATEDATE",
            "SAVEDATE",
            "PRINTDATE",
            "TITLE"
        ]
    );
    for field in HEADER_FOOTER_FIELDS {
        let complex = format!(
            "<w:p>{}</w:p>",
            complex_field_xml(field, "Some result text")
        );
        assert!(header_findings(&complex).is_empty(), "{field} complex");
        let simple = format!("<w:p>{}</w:p>", simple_field_xml(field, "Some result text"));
        assert!(header_findings(&simple).is_empty(), "{field} simple");
    }
}

#[test]
fn txt_003_field_switches_do_not_matter() {
    let header = format!(
        "<w:p>{}</w:p>",
        complex_field_xml(r#"DATE \@ "MMMM d, yyyy" \* MERGEFORMAT"#, "May 5, 2026")
    );
    assert!(header_findings(&header).is_empty());
}

#[test]
fn txt_003_the_result_of_another_field_is_text() {
    let header = format!("<w:p>{}</w:p>", complex_field_xml("AUTHOR", "Dana Smith"));
    let f = header_findings(&header);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].location.snippet.as_deref(), Some("Dana Smith"));
    let simple = format!("<w:p>{}</w:p>", simple_field_xml("FILENAME", "guide.docx"));
    assert_eq!(header_findings(&simple).len(), 1);
}

#[test]
fn txt_003_quotes_the_text_as_it_was_written() {
    // The capitals and the spaces of the document, not a lower-case list of words.
    let f = header_findings(&text_paragraph_xml("Parks  &  Rec   Dept. - Draft 3"));
    assert_eq!(f.len(), 1);
    assert_eq!(
        f[0].location.snippet.as_deref(),
        Some("Parks  &  Rec   Dept. - Draft")
    );
    assert!(
        f[0].message.contains("(\"Parks  &  Rec   Dept. - Draft\")"),
        "{}",
        f[0].message
    );
    assert!(
        f[0].how_to_fix
            .contains("Delete \"Parks  &  Rec   Dept. - Draft\""),
        "{}",
        f[0].how_to_fix
    );
    // Allowed words in the middle split the text in two: the parts are shown, each
    // as it was written.
    let f = header_findings(&text_paragraph_xml("DRAFT Page 3 Confidential"));
    assert_eq!(f[0].location.snippet.as_deref(), Some("DRAFT Confidential"));
    // Letters with accents keep their case too.
    let f = header_findings(&text_paragraph_xml("Département Ünité"));
    assert_eq!(f[0].location.snippet.as_deref(), Some("Département Ünité"));
}

#[test]
fn txt_003_the_words_page_and_of_and_numbers_are_allowed() {
    for text in ["Page", "Page 3 of 10", "page 3 OF 10", "3", "- 3 -"] {
        assert!(
            footer_findings(&text_paragraph_xml(text)).is_empty(),
            "{text:?}"
        );
    }
    assert_eq!(footer_findings(&page_x_of_y_xml()).len(), 0);
}

#[test]
fn txt_003_other_words_next_to_a_page_number_are_reported() {
    let footer = format!(
        "<w:p><w:r><w:t xml:space=\"preserve\">Confidential - Page </w:t></w:r>{}</w:p>",
        complex_field_xml("PAGE", "1")
    );
    let f = footer_findings(&footer);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].location.snippet.as_deref(), Some("Confidential"));
}

#[test]
fn txt_003_the_title_may_be_typed_in_any_case() {
    // The document title property is "Adapdf test guide".
    for text in [
        "Adapdf test guide",
        "ADAPDF TEST GUIDE",
        "  Adapdf   test guide - 2",
    ] {
        assert!(
            header_findings(&text_paragraph_xml(text)).is_empty(),
            "{text:?}"
        );
    }
    let longer = header_findings(&text_paragraph_xml("Adapdf test guide - Draft"));
    assert_eq!(longer.len(), 1);
    assert_eq!(longer[0].location.snippet.as_deref(), Some("Draft"));
}

#[test]
fn txt_003_the_title_may_be_split_over_runs() {
    let header = "<w:p><w:r><w:t>Adap</w:t></w:r><w:r><w:t>df test </w:t></w:r><w:r><w:t>guide</w:t></w:r></w:p>";
    assert!(header_findings(header).is_empty());
}

#[test]
fn txt_003_the_first_title_or_heading_1_also_counts_as_the_title() {
    // The property is "Adapdf test guide"; the Heading 1 is "Test guide".
    assert!(header_findings(&text_paragraph_xml("Test guide")).is_empty());
    let header = DocxBuilder::new()
        .paragraph(Para::new("Parking handbook").style("Title"))
        .header_xml(&text_paragraph_xml("Parking handbook"));
    assert!(rule_findings(&header, RuleId::Txt003).is_empty());
}

#[test]
fn txt_003_a_typed_date_is_text() {
    let f = footer_findings(&text_paragraph_xml("Revised March 5, 2026"));
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].location.snippet.as_deref(), Some("Revised March"));
}

#[test]
fn txt_003_an_empty_header_is_fine() {
    assert!(header_findings("<w:p/>").is_empty());
    assert!(header_findings(&text_paragraph_xml("   ")).is_empty());
}

#[test]
fn txt_003_reports_each_part_once() {
    let builder = clean()
        .header_xml(&format!(
            "{}{}",
            text_paragraph_xml("Parks"),
            text_paragraph_xml("Department")
        ))
        .footer_xml(&text_paragraph_xml("Confidential"));
    let f = rule_findings(&builder, RuleId::Txt003);
    let parts: Vec<&str> = f.iter().map(|f| f.location.part.as_str()).collect();
    assert_eq!(parts, ["word/header1.xml", "word/footer1.xml"]);
    assert_eq!(f[0].location.snippet.as_deref(), Some("Parks Department"));
}

// ---- TXT-004 --------------------------------------------------------------

/// A document with a table of contents (instruction, entries) and these
/// headings (text, level). Each heading has its `_Toc` bookmark.
fn toc_doc(instruction: &str, entries: &[(&str, u8)], headings: &[(&str, u8)]) -> DocxBuilder {
    let mut builder = DocxBuilder::new().toc_with(instruction, entries, false);
    for (i, (text, level)) in headings.iter().enumerate() {
        builder = builder.paragraph(
            Para::new(text)
                .style(&format!("Heading{level}"))
                .bookmark(&format!("_Toc{}", i + 1)),
        );
    }
    builder
}

fn txt004(builder: &DocxBuilder) -> Vec<Finding> {
    rule_findings(builder, RuleId::Txt004)
}

const DEFAULT: &str = adapdf::fixtures::TOC_INSTRUCTION;

#[test]
fn txt_004_fails_when_a_heading_was_renamed() {
    let f = the_failing_finding(RuleId::Txt004);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Auto(FixId::UpdateFields));
    // The first paragraph of the table of contents; no heading above it.
    assert_location(&f, "word/document.xml", Some(0), Some("Test guide 1"), None);
    assert!(f.message.contains("entry 2"), "{}", f.message);
    assert!(f.message.contains("\"Installation\""), "{}", f.message);
}

#[test]
fn txt_004_passes_when_the_entries_are_the_headings() {
    assert!(of(&fixture_findings(RuleId::Txt004, false), RuleId::Txt004).is_empty());
}

#[test]
fn txt_004_no_table_of_contents_no_finding() {
    assert!(txt004(&clean()).is_empty());
}

#[test]
fn txt_004_a_heading_without_an_entry() {
    let b = toc_doc(DEFAULT, &[("One", 1)], &[("One", 1), ("Two", 2)]);
    let f = txt004(&b);
    assert_eq!(f.len(), 1);
    assert!(
        f[0].message.contains("no entry for the heading \"Two\""),
        "{}",
        f[0].message
    );
}

#[test]
fn txt_004_an_entry_that_is_not_a_heading() {
    let b = toc_doc(DEFAULT, &[("One", 1), ("Gone", 2)], &[("One", 1)]);
    let f = txt004(&b);
    assert_eq!(f.len(), 1);
    assert!(
        f[0].message.contains("\"Gone\" is not a heading"),
        "{}",
        f[0].message
    );
}

#[test]
fn txt_004_a_different_order_is_out_of_date() {
    let b = toc_doc(
        DEFAULT,
        &[("Two", 1), ("One", 1)],
        &[("One", 1), ("Two", 1)],
    );
    assert_eq!(txt004(&b).len(), 1);
}

#[test]
fn txt_004_a_table_of_contents_with_no_entries_is_out_of_date() {
    let b = toc_doc(DEFAULT, &[], &[("One", 1)]);
    assert_eq!(txt004(&b).len(), 1);
    // No headings and no entries: nothing to update.
    assert!(txt004(&toc_doc(DEFAULT, &[], &[])).is_empty());
}

#[test]
fn txt_004_a_dirty_field_is_out_of_date_even_when_it_matches() {
    let b = DocxBuilder::new()
        .toc(&[("One", 1)], true)
        .paragraph(Para::new("One").style("Heading1").bookmark("_Toc1"));
    let f = txt004(&b);
    assert_eq!(f.len(), 1);
    assert!(
        f[0].message.contains("needing an update"),
        "{}",
        f[0].message
    );
}

#[test]
fn txt_004_ignores_page_numbers_tabs_and_spacing() {
    let builder = DocxBuilder::new()
        .toc(&[("One", 1)], false)
        .paragraph(Para::new("  One ").style("Heading1").bookmark("_Toc1"));
    assert!(txt004(&builder).is_empty());
    let spaced = toc_doc(DEFAULT, &[("Two  words", 1)], &[("Two words", 1)]);
    assert!(txt004(&spaced).is_empty());
}

#[test]
fn txt_004_ignores_the_numbers_in_numbered_entries() {
    let entry = |n: u32, text: &str| {
        format!(
            r#"<w:hyperlink w:anchor="_Toc{n}"><w:r><w:t>{n}</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>{text}</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>{n}</w:t></w:r></w:hyperlink>"#
        )
    };
    let begin = adapdf::fixtures::complex_field_xml(DEFAULT, "");
    let begin = begin.split("<w:r><w:t").next().unwrap().to_string(); // up to "separate"
    let builder = DocxBuilder::new()
        .custom_style("TOC1", "toc 1", None)
        .paragraph(
            Para::empty()
                .style("TOC1")
                .raw_run(&begin)
                .raw_run(&entry(1, "Setup")),
        )
        .paragraph(Para::empty().raw_run(r#"<w:r><w:fldChar w:fldCharType="end"/></w:r>"#))
        .paragraph(Para::new("Setup").style("Heading1").bookmark("_Toc1"));
    assert!(txt004(&builder).is_empty(), "{:#?}", txt004(&builder));
}

#[test]
fn txt_004_only_the_levels_of_the_o_switch_count() {
    let two_levels = r#"TOC \o "1-2" \h \z \u"#;
    let b = toc_doc(
        two_levels,
        &[("One", 1), ("Two", 2)],
        &[("One", 1), ("Two", 2), ("Three", 3)],
    );
    assert!(txt004(&b).is_empty(), "Heading 3 is not listed");
    let all = toc_doc(
        DEFAULT,
        &[("One", 1), ("Two", 2)],
        &[("One", 1), ("Two", 2), ("Three", 3)],
    );
    assert_eq!(txt004(&all).len(), 1, "Heading 3 is listed by 1-3");
    let deeper = toc_doc(
        r#"TOC \o "2-3" \h"#,
        &[("Two", 2)],
        &[("One", 1), ("Two", 2)],
    );
    assert!(txt004(&deeper).is_empty(), "Heading 1 is not listed by 2-3");
}

#[test]
fn txt_004_the_u_switch_adds_paragraphs_with_an_outline_level() {
    let builder = clean_toc_with_outline_style(r#"TOC \o "1-3" \h \u"#, &[("Callout title", 1)]);
    assert!(txt004(&builder).is_empty());
    let without_u = clean_toc_with_outline_style(r#"TOC \o "1-3" \h"#, &[]);
    assert!(txt004(&without_u).is_empty(), "no \\u: only Heading styles");
}

/// A document with a custom style that has outline level 1, one paragraph in
/// it, and a table of contents with these entries.
fn clean_toc_with_outline_style(instruction: &str, entries: &[(&str, u8)]) -> DocxBuilder {
    DocxBuilder::new()
        .custom_style("Callout", "Callout title", Some(0))
        .toc_with(instruction, entries, false)
        .paragraph(Para::new("Callout title").style("Callout"))
}

#[test]
fn txt_004_a_toc_in_a_content_control_is_checked() {
    let ok = DocxBuilder::new()
        .toc_content_control(&[("One", 1)], false)
        .paragraph(Para::new("One").style("Heading1").bookmark("_Toc1"));
    assert!(txt004(&ok).is_empty());
    let stale = DocxBuilder::new()
        .toc_content_control(&[("One", 1)], false)
        .paragraph(Para::new("Uno").style("Heading1").bookmark("_Toc1"));
    let f = txt004(&stale);
    assert_eq!(
        f.len(),
        1,
        "one finding, not one for the control and one for the field"
    );
    let dirty = DocxBuilder::new()
        .toc_content_control(&[("One", 1)], true)
        .paragraph(Para::new("One").style("Heading1").bookmark("_Toc1"));
    assert_eq!(txt004(&dirty).len(), 1);
}

#[test]
fn txt_004_a_content_control_without_a_field_is_checked_by_its_entries() {
    let toc_style = r#"<w:style w:type="paragraph" w:styleId="TOC1"><w:name w:val="toc 1"/><w:basedOn w:val="Normal"/></w:style>
      <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>"#;
    let body = |entry: &str| {
        format!(
            r#"<w:sdt><w:sdtPr><w:docPartObj><w:docPartGallery w:val="Table of Contents"/></w:docPartObj></w:sdtPr><w:sdtContent><w:p><w:pPr><w:pStyle w:val="TOC1"/></w:pPr><w:r><w:t>{entry}</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>1</w:t></w:r></w:p></w:sdtContent></w:sdt><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>One</w:t></w:r></w:p>"#
        )
    };
    assert!(raw(&body("One"), toc_style, RuleId::Txt004).is_empty());
    assert_eq!(raw(&body("Uno"), toc_style, RuleId::Txt004).len(), 1);
}

#[test]
fn txt_004_with_a_t_switch_extra_entries_are_allowed_but_headings_must_be_there() {
    let instruction = r#"TOC \o "1-3" \t "Callout title,2""#;
    let extra = toc_doc(instruction, &[("One", 1), ("Side note", 2)], &[("One", 1)]);
    assert!(txt004(&extra).is_empty());
    let missing = toc_doc(instruction, &[("Side note", 2)], &[("One", 1)]);
    assert_eq!(txt004(&missing).len(), 1);
}

#[test]
fn txt_004_a_toc_without_page_numbers_is_read_with_the_n_switch() {
    let entry = r#"<w:hyperlink w:anchor="_Toc1"><w:r><w:t>1.1</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>Setup</w:t></w:r></w:hyperlink>"#;
    let field = adapdf::fixtures::complex_field_xml(r#"TOC \o "1-3" \n"#, "");
    let begin = field.split("<w:r><w:t").next().unwrap().to_string();
    let builder = DocxBuilder::new()
        .custom_style("TOC1", "toc 1", None)
        .paragraph(Para::empty().style("TOC1").raw_run(&begin).raw_run(entry))
        .paragraph(Para::empty().raw_run(r#"<w:r><w:fldChar w:fldCharType="end"/></w:r>"#))
        .paragraph(Para::new("Setup").style("Heading1").bookmark("_Toc1"));
    assert!(txt004(&builder).is_empty(), "{:#?}", txt004(&builder));
}

#[test]
fn txt_004_reports_each_table_of_contents() {
    let builder = DocxBuilder::new()
        .toc(&[("Old", 1)], false)
        .paragraph(Para::new("New").style("Heading1").bookmark("_Toc1"))
        .toc(&[("Older", 1)], false);
    assert_eq!(txt004(&builder).len(), 2);
}
