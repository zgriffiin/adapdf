//! Builder additions and fixtures for the LNK, CLR and TXT rules: links,
//! fields, bookmarks, shading, tables of contents, headers and footers.

use super::{CustomStyle, DocxBuilder, Para, Run, clean, clean_settings, esc};
use crate::findings::RuleId;

/// A `w:shd` element that fills with `hex` (`RRGGBB`).
pub(super) fn shd_xml(hex: &str) -> String {
    format!(
        r#"<w:shd w:val="clear" w:color="auto" w:fill="{}"/>"#,
        esc(hex)
    )
}

/// The runs of a complex field: begin, instruction, separate, result, end.
pub fn complex_field_xml(instruction: &str, result: &str) -> String {
    format!(
        r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> {} </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t xml:space="preserve">{}</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r>"#,
        esc(instruction),
        esc(result)
    )
}

/// A simple field (`w:fldSimple`).
pub fn simple_field_xml(instruction: &str, result: &str) -> String {
    format!(
        r#"<w:fldSimple w:instr=" {} "><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:fldSimple>"#,
        esc(instruction),
        esc(result)
    )
}

/// A paragraph for a footer: "Page 1 of 3" with PAGE and NUMPAGES fields.
pub fn page_x_of_y_xml() -> String {
    format!(
        r#"<w:p><w:r><w:t xml:space="preserve">Page </w:t></w:r>{}<w:r><w:t xml:space="preserve"> of </w:t></w:r>{}</w:p>"#,
        complex_field_xml("PAGE", "1"),
        complex_field_xml("NUMPAGES", "3")
    )
}

/// A text paragraph for a header or footer.
pub fn text_paragraph_xml(text: &str) -> String {
    format!(
        r#"<w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p>"#,
        esc(text)
    )
}

/// A short id for the relationship of an external link, the same for the
/// same address.
fn link_id(url: &str) -> String {
    let hash = url.bytes().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    });
    format!("rIdLink{hash:08x}")
}

impl Run {
    pub fn italic(mut self) -> Run {
        self.italic = true;
        self
    }

    pub fn underline(mut self) -> Run {
        self.underline = true;
        self
    }

    /// A highlight colour by its Word name, for example `yellow` or `darkBlue`.
    pub fn highlight(mut self, name: &str) -> Run {
        self.highlight = Some(name.to_string());
        self
    }

    /// A fill behind the run (`RRGGBB`).
    pub fn shading(mut self, hex: &str) -> Run {
        self.shading = Some(hex.to_string());
        self
    }

    /// A character style by id. Define it with [`DocxBuilder::char_style`].
    pub fn char_style(mut self, style_id: &str) -> Run {
        self.char_style = Some(style_id.to_string());
        self
    }
}

impl Para {
    /// A fill behind the paragraph (`RRGGBB`).
    pub fn shading(mut self, hex: &str) -> Para {
        self.shading = Some(hex.to_string());
        self
    }

    /// Puts the paragraph in a one-cell table whose cell has this fill.
    pub fn cell_shading(mut self, hex: &str) -> Para {
        self.in_table = true;
        self.cell_shading = Some(hex.to_string());
        self
    }

    /// A bookmark around the whole paragraph.
    pub fn bookmark(mut self, name: &str) -> Para {
        self.bookmark = Some(name.to_string());
        self
    }

    /// Adds a link to a web address, written as `w:hyperlink` with a
    /// relationship.
    pub fn hyperlink(mut self, text: &str, url: &str) -> Para {
        let id = link_id(url);
        self.link_targets.push((id.clone(), url.to_string()));
        self.raw_runs.push(format!(
            r#"<w:hyperlink r:id="{id}"><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:hyperlink>"#,
            esc(text)
        ));
        self
    }

    /// Adds a link to a bookmark (`w:hyperlink w:anchor`).
    pub fn internal_link(mut self, text: &str, bookmark: &str) -> Para {
        self.raw_runs.push(format!(
            r#"<w:hyperlink w:anchor="{}"><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:hyperlink>"#,
            esc(bookmark),
            esc(text)
        ));
        self
    }

    /// Adds a complex field with its result text.
    pub fn field(self, instruction: &str, result: &str) -> Para {
        self.raw_run(&complex_field_xml(instruction, result))
    }

    /// Adds a simple field (`w:fldSimple`) with its result text.
    pub fn simple_field(self, instruction: &str, result: &str) -> Para {
        self.raw_run(&simple_field_xml(instruction, result))
    }

    /// XML written before the paragraph, outside any table.
    pub fn raw_before(mut self, xml: &str) -> Para {
        self.raw_before = xml.to_string();
        self
    }

    /// XML written after the paragraph, outside any table.
    pub fn raw_after(mut self, xml: &str) -> Para {
        self.raw_after = xml.to_string();
        self
    }
}

/// A theme with the Office colours that the tests use: accent 1 is `4472C4`.
pub const TEST_THEME: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements><a:clrScheme name="t"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1></a:clrScheme></a:themeElements></a:theme>"#;

/// A .docx written from raw XML, for tests that need XML the builder does not
/// write (style colours, theme colours, table fills). `body` goes inside
/// `w:body`; `styles` goes after the Normal style in `word/styles.xml`; the
/// theme is [`TEST_THEME`] when `theme` is true.
pub fn raw_docx(body: &str, styles: &str, theme: bool) -> Vec<u8> {
    const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{}" xmlns:r="{REL}"><w:body>{body}</w:body></w:document>"#,
        super::W
    );
    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="{}"><w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val="22"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>{styles}</w:styles>"#,
        super::W
    );
    let theme_rel = if theme {
        format!(r#"<Relationship Id="rIdTheme" Type="{REL}/theme" Target="theme/theme1.xml"/>"#)
    } else {
        String::new()
    };
    let rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyles" Type="{REL}/styles" Target="styles.xml"/>{theme_rel}</Relationships>"#
    );
    let mut entries: Vec<(&str, &[u8])> = vec![
        ("_rels/.rels", super::ROOT_RELS.as_bytes()),
        ("word/document.xml", document.as_bytes()),
        ("word/_rels/document.xml.rels", rels.as_bytes()),
        ("word/styles.xml", styles.as_bytes()),
    ];
    if theme {
        entries.push(("word/theme/theme1.xml", TEST_THEME.as_bytes()));
    }
    super::zip_stored(&entries)
}

/// The instruction that Word writes for a table of contents of Heading 1 to 3.
pub const TOC_INSTRUCTION: &str = r#"TOC \o "1-3" \h \z \u"#;

const TOC_SDT_START: &str = r#"<w:sdt><w:sdtPr><w:docPartObj><w:docPartGallery w:val="Table of Contents"/><w:docPartUnique/></w:docPartObj></w:sdtPr><w:sdtContent>"#;
const TOC_SDT_END: &str = "</w:sdtContent></w:sdt>";

/// The paragraphs of a table of contents, as Word writes one: the field
/// starts in the first entry and ends in a paragraph of its own. Entry `n`
/// (counting from 1) links to the bookmark `_Toc{n}`.
fn toc_paragraphs(
    instruction: &str,
    entries: &[(&str, u8)],
    dirty: bool,
    content_control: bool,
) -> Vec<Para> {
    let dirty = if dirty { r#" w:dirty="true""# } else { "" };
    let begin = format!(
        r#"<w:r><w:fldChar w:fldCharType="begin"{dirty}/></w:r><w:r><w:instrText xml:space="preserve"> {} </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r>"#,
        esc(instruction)
    );
    let end = r#"<w:r><w:fldChar w:fldCharType="end"/></w:r>"#;
    let mut paragraphs = Vec::new();
    if entries.is_empty() {
        let placeholder = "<w:r><w:t>No table of contents entries found.</w:t></w:r>";
        paragraphs.push(Para::empty().raw_run(&format!("{begin}{placeholder}{end}")));
    } else {
        for (i, (text, level)) in entries.iter().enumerate() {
            let mut p = Para::empty().style(&format!("TOC{level}"));
            if i == 0 {
                p = p.raw_run(&begin);
            }
            paragraphs.push(p.raw_run(&format!(
                r#"<w:hyperlink w:anchor="_Toc{}" w:history="1"><w:r><w:t>{}</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>{}</w:t></w:r></w:hyperlink>"#,
                i + 1,
                esc(text),
                i + 1
            )));
        }
        paragraphs.push(Para::empty().raw_run(end));
    }
    if content_control {
        let first = paragraphs.remove(0).raw_before(TOC_SDT_START);
        paragraphs.insert(0, first);
        let last = paragraphs
            .pop()
            .expect("a table of contents has paragraphs");
        paragraphs.push(last.raw_after(TOC_SDT_END));
    }
    paragraphs
}

impl DocxBuilder {
    /// Paragraph XML for the header (`w:p` elements).
    pub fn header_xml(mut self, paragraphs: &str) -> DocxBuilder {
        self.header_text = Some(String::new());
        self.header_raw = Some(paragraphs.to_string());
        self
    }

    /// Paragraph XML for the footer (`w:p` elements).
    pub fn footer_xml(mut self, paragraphs: &str) -> DocxBuilder {
        self.footer_raw = Some(paragraphs.to_string());
        self
    }

    /// The page colour (`RRGGBB`). It is shown (`displayBackgroundShape` is
    /// on in the settings) only when `shown` is true.
    pub fn page_background(mut self, hex: &str, shown: bool) -> DocxBuilder {
        self.page_background = Some((hex.to_string(), shown));
        self
    }

    /// A character style. `rpr` is the XML inside its `w:rPr`.
    pub fn char_style(mut self, id: &str, name: &str, rpr: &str) -> DocxBuilder {
        self.char_styles
            .push((id.to_string(), name.to_string(), rpr.to_string()));
        self
    }

    /// Adds paragraphs.
    pub fn paragraphs(mut self, paragraphs: Vec<Para>) -> DocxBuilder {
        self.body.extend(paragraphs);
        self
    }

    /// A table of contents field with these entries (text and heading level).
    /// `dirty` marks the field as needing an update.
    pub fn toc(self, entries: &[(&str, u8)], dirty: bool) -> DocxBuilder {
        self.toc_with(TOC_INSTRUCTION, entries, dirty)
    }

    /// A table of contents with a field instruction of your choice.
    pub fn toc_with(self, instruction: &str, entries: &[(&str, u8)], dirty: bool) -> DocxBuilder {
        self.with_toc_styles()
            .paragraphs(toc_paragraphs(instruction, entries, dirty, false))
    }

    /// A table of contents inside a "Table of Contents" content control.
    pub fn toc_content_control(self, entries: &[(&str, u8)], dirty: bool) -> DocxBuilder {
        self.with_toc_styles()
            .paragraphs(toc_paragraphs(TOC_INSTRUCTION, entries, dirty, true))
    }

    /// Defines the styles TOC 1 to TOC 3 once.
    fn with_toc_styles(mut self) -> DocxBuilder {
        for level in 1..=3 {
            let id = format!("TOC{level}");
            if !self.custom_styles.iter().any(|s| s.id == id) {
                self.custom_styles.push(CustomStyle {
                    id,
                    name: format!("toc {level}"),
                    outline_level: None,
                });
            }
        }
        self
    }
}

/// The long address that LNK-002 reports, and a short one that it does not.
const LONG_URL: &str = "https://example.org/departments/parking/policy-2026-final.pdf";
const SHORT_URL: &str = "https://example.org/parking-policy";

/// The pass and fail document for each of the LNK, CLR and TXT rules.
pub(super) fn pairs() -> Vec<(RuleId, DocxBuilder, DocxBuilder)> {
    let toc_headings = |second: &str| {
        clean_settings()
            .toc(&[("Test guide", 1), ("Setup", 2)], false)
            .paragraph(Para::new("Test guide").style("Heading1").bookmark("_Toc1"))
            .paragraph(Para::new(second).style("Heading2").bookmark("_Toc2"))
            .para("Plug it in.")
    };
    vec![
        (
            RuleId::Lnk001,
            clean().paragraph(Para::new("Read the ").hyperlink("parking policy", SHORT_URL)),
            clean().paragraph(
                Para::new("To read the parking policy, ").hyperlink("click here", SHORT_URL),
            ),
        ),
        (
            RuleId::Lnk002,
            clean().paragraph(Para::new("Visit ").hyperlink(SHORT_URL, SHORT_URL)),
            clean().paragraph(Para::new("Visit ").hyperlink(LONG_URL, LONG_URL)),
        ),
        (
            RuleId::Lnk003,
            clean()
                .paragraph(Para::new("Setup").style("Heading2").bookmark("Setup"))
                .paragraph(Para::empty().internal_link("See the setup section", "Setup")),
            clean()
                .paragraph(Para::new("Setup").style("Heading2").bookmark("Setup"))
                .paragraph(Para::empty().internal_link("See the setup section", "Nowhere")),
        ),
        (
            RuleId::Clr001,
            clean().paragraph(
                Para::empty().then(Run::new("Dark grey text.").color("595959").italic()),
            ),
            clean().paragraph(
                Para::empty().then(Run::new("Light grey text.").color("777777").italic()),
            ),
        ),
        (
            RuleId::Clr002,
            clean().paragraph(
                Para::empty()
                    .then(Run::new("Warning:").color("C00000").bold())
                    .then(Run::new(" save your work first.")),
            ),
            clean().paragraph(
                Para::empty()
                    .then(Run::new("Warning:").color("C00000"))
                    .then(Run::new(" save your work first.")),
            ),
        ),
        (
            RuleId::Txt001,
            clean().paragraph(Para::new("Small print at 10 pt.").size(20)),
            clean().paragraph(Para::new("Small print at 9.5 pt.").size(19)),
        ),
        (
            RuleId::Txt002,
            clean().para("Text aligned to the left."),
            clean().paragraph(Para::new("Text that is justified.").justify()),
        ),
        (
            RuleId::Txt003,
            clean()
                .header_text("Adapdf test guide")
                .footer_xml(&page_x_of_y_xml()),
            clean().header_text("Parks and Recreation Department"),
        ),
        (
            RuleId::Txt004,
            toc_headings("Setup"),
            toc_headings("Installation"),
        ),
    ]
}
