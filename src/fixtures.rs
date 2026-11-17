//! Test documents made by code.
//!
//! [`DocxBuilder`] writes a small, valid .docx. The zip output is the same on
//! every run (stored, not compressed, with a fixed time stamp), so the files in
//! `tests/fixtures/` can be compared byte for byte with the generator output.
//! [`catalogue`] lists the passing and failing fixture for each rule.
//!
//! This module is for tests and for `examples/gen-fixtures.rs`. It is not used
//! by the checker.

use std::io::{Cursor, Write};

use crate::findings::RuleId;
use crate::fonts::{FontLookup, FontStatus};
use crate::rules::document::TEMPLATE_FILE_NAME;

mod links_colour_text;
mod media;
mod objects;
use links_colour_text::shd_xml;
pub use links_colour_text::{
    TEST_THEME, TOC_INSTRUCTION, TOC_INSTRUCTION_NO_LINKS, complex_field_xml, page_x_of_y_xml,
    raw_docx, simple_field_xml, text_paragraph_xml,
};
pub use objects::{
    Image, Table, alt_of_length, equation, ole_object, split_runs, text_box, vml_text_box,
};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Gives every `wp:docPr` a different `id`, counting from 1 in document order.
/// Word needs the ids of the drawings in a document to be unique, so the
/// builders write a placeholder and this numbers them.
fn number_drawing_ids(xml: &str) -> String {
    const OPEN: &str = r#"<wp:docPr id=""#;
    let mut numbered = String::with_capacity(xml.len());
    let mut rest = xml;
    let mut next_id = 1;
    while let Some(at) = rest.find(OPEN) {
        let value_start = at + OPEN.len();
        let value_len = rest[value_start..]
            .find('"')
            .expect("a wp:docPr id has a closing quote");
        numbered.push_str(&rest[..value_start]);
        numbered.push_str(&next_id.to_string());
        rest = &rest[value_start + value_len..];
        next_id += 1;
    }
    numbered.push_str(rest);
    numbered
}

/// Escapes text for XML content and attribute values.
fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Text of `count` words, for documents that must be a given length.
pub fn words(count: usize) -> String {
    vec!["word"; count].join(" ")
}

/// A zip file with these entries, stored without compression and with a
/// fixed time stamp.
pub fn zip_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer.start_file(*name, options).expect("zip entry name");
        writer.write_all(data).expect("write to memory");
    }
    writer.finish().expect("finish zip in memory").into_inner()
}

/// One piece of text in a paragraph.
#[derive(Debug, Clone, Default)]
pub struct Run {
    text: String,
    bold: bool,
    size: Option<u32>,
    font: Option<String>,
    color: Option<String>,
    lang: Option<String>,
    lang_east_asia: Option<String>,
    lang_bidi: Option<String>,
    inserted: bool,
    italic: bool,
    underline: bool,
    highlight: Option<String>,
    shading: Option<String>,
    char_style: Option<String>,
}

impl Run {
    pub fn new(text: &str) -> Run {
        Run {
            text: text.to_string(),
            ..Run::default()
        }
    }

    pub fn bold(mut self) -> Run {
        self.bold = true;
        self
    }

    /// Size in half points (24 = 12 pt).
    pub fn size(mut self, half_points: u32) -> Run {
        self.size = Some(half_points);
        self
    }

    pub fn font(mut self, name: &str) -> Run {
        self.font = Some(name.to_string());
        self
    }

    /// Text colour as `RRGGBB`.
    pub fn color(mut self, hex: &str) -> Run {
        self.color = Some(hex.to_string());
        self
    }

    pub fn lang(mut self, tag: &str) -> Run {
        self.lang = Some(tag.to_string());
        self
    }

    pub fn lang_east_asia(mut self, tag: &str) -> Run {
        self.lang_east_asia = Some(tag.to_string());
        self
    }

    pub fn lang_bidi(mut self, tag: &str) -> Run {
        self.lang_bidi = Some(tag.to_string());
        self
    }

    /// A tracked insertion.
    pub fn inserted(mut self) -> Run {
        self.inserted = true;
        self
    }

    fn xml(&self, ids: &mut u32) -> String {
        let mut rpr = String::new();
        if let Some(style) = &self.char_style {
            rpr += &format!(r#"<w:rStyle w:val="{}"/>"#, esc(style));
        }
        if let Some(font) = &self.font {
            let font = esc(font);
            rpr += &format!(r#"<w:rFonts w:ascii="{font}" w:hAnsi="{font}"/>"#);
        }
        if self.bold {
            rpr += "<w:b/>";
        }
        if self.italic {
            rpr += "<w:i/>";
        }
        if let Some(color) = &self.color {
            rpr += &format!(r#"<w:color w:val="{}"/>"#, esc(color));
        }
        if let Some(size) = self.size {
            rpr += &format!(r#"<w:sz w:val="{size}"/>"#);
        }
        if let Some(name) = &self.highlight {
            rpr += &format!(r#"<w:highlight w:val="{}"/>"#, esc(name));
        }
        if self.underline {
            rpr += r#"<w:u w:val="single"/>"#;
        }
        if let Some(fill) = &self.shading {
            rpr += &format!(
                r#"<w:shd w:val="clear" w:color="auto" w:fill="{}"/>"#,
                esc(fill)
            );
        }
        if self.lang.is_some() || self.lang_east_asia.is_some() || self.lang_bidi.is_some() {
            rpr += "<w:lang";
            for (attr, value) in [
                ("val", &self.lang),
                ("eastAsia", &self.lang_east_asia),
                ("bidi", &self.lang_bidi),
            ] {
                if let Some(v) = value {
                    rpr += &format!(r#" w:{attr}="{}""#, esc(v));
                }
            }
            rpr += "/>";
        }
        let rpr = if rpr.is_empty() {
            String::new()
        } else {
            format!("<w:rPr>{rpr}</w:rPr>")
        };
        let run = format!(
            r#"<w:r>{rpr}<w:t xml:space="preserve">{}</w:t></w:r>"#,
            esc(&self.text)
        );
        if self.inserted {
            *ids += 1;
            format!(
                r#"<w:ins w:id="{ids}" w:author="Test" w:date="2020-01-01T00:00:00Z">{run}</w:ins>"#
            )
        } else {
            run
        }
    }
}

/// One body paragraph.
#[derive(Debug, Clone, Default)]
pub struct Para {
    style: Option<String>,
    num: Option<(u32, u32)>,
    justify: bool,
    in_table: bool,
    section_columns: Option<u32>,
    runs: Vec<Run>,
    raw_runs: Vec<String>,
    comments: Vec<String>,
    /// A `w:tbl` written instead of the paragraph. See [`DocxBuilder::table`].
    table_xml: Option<String>,
    shading: Option<String>,
    cell_shading: Option<String>,
    bookmark: Option<String>,
    /// Relationship ids and targets of the external links in this paragraph.
    link_targets: Vec<(String, String)>,
    raw_before: String,
    raw_after: String,
}

impl Para {
    /// A paragraph with one run of text.
    pub fn new(text: &str) -> Para {
        Para::default().then(Run::new(text))
    }

    /// A paragraph with no text.
    pub fn empty() -> Para {
        Para::default()
    }

    /// Adds a run.
    pub fn then(mut self, run: Run) -> Para {
        self.runs.push(run);
        self
    }

    /// Adds a run written as XML. For drawings and other objects.
    pub fn raw_run(mut self, xml: &str) -> Para {
        self.raw_runs.push(xml.to_string());
        self
    }

    /// The paragraph style by id, for example `Heading2` or `ListNumber`.
    pub fn style(mut self, style_id: &str) -> Para {
        self.style = Some(style_id.to_string());
        self
    }

    /// Direct list formatting: list `num_id` (1 = numbers, 2 = bullets), level.
    pub fn num(mut self, num_id: u32, level: u32) -> Para {
        self.num = Some((num_id, level));
        self
    }

    pub fn justify(mut self) -> Para {
        self.justify = true;
        self
    }

    /// Puts the paragraph in a one-cell table.
    pub fn in_table(mut self) -> Para {
        self.in_table = true;
        self
    }

    /// Ends a section after this paragraph. The section has this many columns.
    pub fn ends_section(mut self, columns: u32) -> Para {
        self.section_columns = Some(columns);
        self
    }

    /// Adds a comment on this paragraph's text.
    pub fn comment(mut self, text: &str) -> Para {
        self.comments.push(text.to_string());
        self
    }

    /// Makes every run bold.
    pub fn bold(mut self) -> Para {
        self.runs.iter_mut().for_each(|r| r.bold = true);
        self
    }

    /// Sets the size of every run, in half points.
    pub fn size(mut self, half_points: u32) -> Para {
        self.runs
            .iter_mut()
            .for_each(|r| r.size = Some(half_points));
        self
    }

    /// Sets the font of every run.
    pub fn font(mut self, name: &str) -> Para {
        self.runs
            .iter_mut()
            .for_each(|r| r.font = Some(name.to_string()));
        self
    }

    /// Sets the language of every run.
    pub fn lang(mut self, tag: &str) -> Para {
        self.runs
            .iter_mut()
            .for_each(|r| r.lang = Some(tag.to_string()));
        self
    }

    fn xml(&self, ids: &mut u32, comments: &mut Vec<String>) -> String {
        if let Some(table) = &self.table_xml {
            return table.clone();
        }
        let mut ppr = String::new();
        if let Some(style) = &self.style {
            ppr += &format!(r#"<w:pStyle w:val="{}"/>"#, esc(style));
        }
        if let Some((num_id, level)) = self.num {
            ppr += &format!(
                r#"<w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{num_id}"/></w:numPr>"#
            );
        }
        if let Some(fill) = &self.shading {
            ppr += &shd_xml(fill);
        }
        if self.justify {
            ppr += r#"<w:jc w:val="both"/>"#;
        }
        if let Some(columns) = self.section_columns {
            ppr += &format!(
                r#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:cols w:num="{columns}"/></w:sectPr>"#
            );
        }
        let ppr = if ppr.is_empty() {
            String::new()
        } else {
            format!("<w:pPr>{ppr}</w:pPr>")
        };
        let mut content = String::new();
        let mut comment_ids = Vec::new();
        for text in &self.comments {
            comment_ids.push(comments.len());
            comments.push(text.clone());
        }
        for id in &comment_ids {
            content += &format!(r#"<w:commentRangeStart w:id="{id}"/>"#);
        }
        let bookmark_id = self.bookmark.as_ref().map(|name| {
            *ids += 1;
            content += &format!(r#"<w:bookmarkStart w:id="{ids}" w:name="{}"/>"#, esc(name));
            *ids
        });
        for run in &self.runs {
            content += &run.xml(ids);
        }
        for raw in &self.raw_runs {
            content += raw;
        }
        if let Some(id) = bookmark_id {
            content += &format!(r#"<w:bookmarkEnd w:id="{id}"/>"#);
        }
        for id in &comment_ids {
            content += &format!(
                r#"<w:commentRangeEnd w:id="{id}"/><w:r><w:commentReference w:id="{id}"/></w:r>"#
            );
        }
        let p = format!("<w:p>{ppr}{content}</w:p>");
        let block = if self.in_table {
            let cell_shd = self
                .cell_shading
                .as_deref()
                .map(shd_xml)
                .unwrap_or_default();
            format!(
                r#"<w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr><w:tblGrid><w:gridCol w:w="9360"/></w:tblGrid><w:tr><w:tc><w:tcPr><w:tcW w:w="9360" w:type="dxa"/>{cell_shd}</w:tcPr>{p}</w:tc></w:tr></w:tbl>"#
            )
        } else {
            p
        };
        format!("{}{block}{}", self.raw_before, self.raw_after)
    }
}

#[derive(Debug, Clone)]
struct CustomStyle {
    id: String,
    name: String,
    outline_level: Option<u32>,
}

/// Builds a .docx. `new()` gives a bare document: no title, no language and
/// no template, so the DOC rules find problems in it. Add what a test needs.
#[derive(Debug, Clone)]
pub struct DocxBuilder {
    title: Option<String>,
    language: Option<String>,
    theme_font_lang: Option<String>,
    template: Option<String>,
    font: Option<String>,
    body_size: u32,
    custom_styles: Vec<CustomStyle>,
    header_text: Option<String>,
    /// Paragraph XML for the header, instead of the plain text in
    /// `header_text` (which then only says that a header exists).
    header_raw: Option<String>,
    /// Paragraph XML for a footer.
    footer_raw: Option<String>,
    /// The page colour `RRGGBB`, and whether the settings show it.
    page_background: Option<(String, bool)>,
    /// Character styles: id, name and the XML inside `w:rPr`.
    char_styles: Vec<(String, String, String)>,
    /// Table styles as `w:style` XML. See [`DocxBuilder::table_style`].
    table_styles: Vec<String>,
    body: Vec<Para>,
}

impl Default for DocxBuilder {
    fn default() -> Self {
        DocxBuilder::new()
    }
}

impl DocxBuilder {
    pub fn new() -> DocxBuilder {
        DocxBuilder {
            title: None,
            language: None,
            theme_font_lang: None,
            template: None,
            font: None,
            body_size: 22,
            custom_styles: Vec::new(),
            header_text: None,
            header_raw: None,
            footer_raw: None,
            page_background: None,
            char_styles: Vec::new(),
            table_styles: Vec::new(),
            body: Vec::new(),
        }
    }

    /// The document Title property. An empty string writes an empty title.
    pub fn title(mut self, title: &str) -> DocxBuilder {
        self.title = Some(title.to_string());
        self
    }

    /// The default language in the document defaults, for example `en-US`.
    pub fn language(mut self, tag: &str) -> DocxBuilder {
        self.language = Some(tag.to_string());
        self
    }

    pub fn no_language(mut self) -> DocxBuilder {
        self.language = None;
        self
    }

    /// The theme font language in `settings.xml`.
    pub fn theme_font_lang(mut self, tag: &str) -> DocxBuilder {
        self.theme_font_lang = Some(tag.to_string());
        self
    }

    /// Attaches a template by file name. The target is written the way Word
    /// writes it: a `file:` URL in the user's templates folder.
    pub fn template(mut self, file_name: &str) -> DocxBuilder {
        self.template = Some(file_name.to_string());
        self
    }

    pub fn no_template(mut self) -> DocxBuilder {
        self.template = None;
        self
    }

    /// The font in the document defaults.
    pub fn font(mut self, name: &str) -> DocxBuilder {
        self.font = Some(name.to_string());
        self
    }

    /// The body text size in half points. The default is 22 (11 pt).
    pub fn body_size(mut self, half_points: u32) -> DocxBuilder {
        self.body_size = half_points;
        self
    }

    /// Adds a paragraph style based on Normal. `outline_level` is the raw XML
    /// value (0 = level 1).
    pub fn custom_style(mut self, id: &str, name: &str, outline_level: Option<u32>) -> DocxBuilder {
        self.custom_styles.push(CustomStyle {
            id: id.to_string(),
            name: name.to_string(),
            outline_level,
        });
        self
    }

    /// Adds a table style. `inner` is the XML of the style after `w:name` and
    /// `w:basedOn`, in schema order (`w:pPr`, `w:rPr`, `w:tblPr`, `w:tcPr`, then
    /// the `w:tblStylePr` elements). The style is based on `based_on`, or on
    /// Word's default table style `TableNormal` when that is `None`.
    pub fn table_style(
        mut self,
        id: &str,
        name: &str,
        based_on: Option<&str>,
        inner: &str,
    ) -> DocxBuilder {
        self.table_styles.push(format!(
            r#"<w:style w:type="table" w:styleId="{}"><w:name w:val="{}"/><w:basedOn w:val="{}"/>{inner}</w:style>
"#,
            esc(id),
            esc(name),
            esc(based_on.unwrap_or("TableNormal"))
        ));
        self
    }

    /// Adds a header with one paragraph of text.
    pub fn header_text(mut self, text: &str) -> DocxBuilder {
        self.header_text = Some(text.to_string());
        self
    }

    pub fn heading(self, level: u8, text: &str) -> DocxBuilder {
        self.paragraph(Para::new(text).style(&format!("Heading{level}")))
    }

    pub fn para(self, text: &str) -> DocxBuilder {
        self.paragraph(Para::new(text))
    }

    pub fn paragraph(mut self, para: Para) -> DocxBuilder {
        self.body.push(para);
        self
    }

    /// The .docx file as bytes.
    pub fn build(&self) -> Vec<u8> {
        let mut ids = 0;
        let mut comments = Vec::new();
        let mut body = String::new();
        for para in &self.body {
            body += &para.xml(&mut ids, &mut comments);
        }
        if self
            .body
            .last()
            .is_some_and(|p| p.in_table || p.table_xml.is_some())
        {
            body += "<w:p/>"; // a table cannot be the last thing in a body
        }
        let body = number_drawing_ids(&body);
        let has_image = media::uses_image(&body);
        let header_ref = if self.header_text.is_some() {
            r#"<w:headerReference w:type="default" r:id="rIdHeader1"/>"#
        } else {
            ""
        };
        let footer_ref = if self.footer_raw.is_some() {
            r#"<w:footerReference w:type="default" r:id="rIdFooter1"/>"#
        } else {
            ""
        };
        let background = self
            .page_background
            .as_ref()
            .map(|(hex, _)| format!(r#"<w:background w:color="{}"/>"#, esc(hex)))
            .unwrap_or_default();
        let document = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{W}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" xmlns:v="urn:schemas-microsoft-com:vml">{background}<w:body>{body}<w:sectPr>{header_ref}{footer_ref}<w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>"#
        );

        let mut parts: Vec<(String, Vec<u8>)> = vec![
            (
                "[Content_Types].xml".into(),
                self.content_types(&comments, has_image).into_bytes(),
            ),
            ("_rels/.rels".into(), ROOT_RELS.as_bytes().to_vec()),
            ("docProps/core.xml".into(), self.core().into_bytes()),
            ("word/document.xml".into(), document.into_bytes()),
            (
                "word/_rels/document.xml.rels".into(),
                self.document_rels(&comments, has_image).into_bytes(),
            ),
            ("word/styles.xml".into(), self.styles().into_bytes()),
            ("word/numbering.xml".into(), NUMBERING.as_bytes().to_vec()),
            ("word/settings.xml".into(), self.settings().into_bytes()),
        ];
        if let Some(template) = &self.template {
            parts.push((
                "word/_rels/settings.xml.rels".into(),
                settings_rels(template).into_bytes(),
            ));
        }
        if let Some(text) = &self.header_text {
            let paragraphs = self
                .header_raw
                .clone()
                .unwrap_or_else(|| format!("<w:p><w:r><w:t>{}</w:t></w:r></w:p>", esc(text)));
            let header = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="{W}">{paragraphs}</w:hdr>"#
            );
            parts.push(("word/header1.xml".into(), header.into_bytes()));
        }
        if let Some(paragraphs) = &self.footer_raw {
            let footer = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="{W}">{paragraphs}</w:ftr>"#
            );
            parts.push(("word/footer1.xml".into(), footer.into_bytes()));
        }
        if !comments.is_empty() {
            let items: String = comments
                .iter()
                .enumerate()
                .map(|(id, text)| {
                    format!(
                        r#"<w:comment w:id="{id}" w:author="Test" w:date="2020-01-01T00:00:00Z"><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:comment>"#,
                        esc(text)
                    )
                })
                .collect();
            let xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:comments xmlns:w="{W}">{items}</w:comments>"#
            );
            parts.push(("word/comments.xml".into(), xml.into_bytes()));
        }
        if has_image {
            parts.push((media::IMAGE_PART.into(), media::png()));
        }
        let entries: Vec<(&str, &[u8])> = parts
            .iter()
            .map(|(name, data)| (name.as_str(), data.as_slice()))
            .collect();
        zip_stored(&entries)
    }

    fn content_types(&self, comments: &[String], has_image: bool) -> String {
        let mut overrides = String::new();
        let mut add = |part: &str, kind: &str| {
            overrides += &format!(
                r#"<Override PartName="/{part}" ContentType="application/vnd.openxmlformats-{kind}+xml"/>"#
            );
        };
        add(
            "word/document.xml",
            "officedocument.wordprocessingml.document.main",
        );
        add("word/styles.xml", "officedocument.wordprocessingml.styles");
        add(
            "word/numbering.xml",
            "officedocument.wordprocessingml.numbering",
        );
        add(
            "word/settings.xml",
            "officedocument.wordprocessingml.settings",
        );
        add("docProps/core.xml", "package.core-properties");
        if self.header_text.is_some() {
            add("word/header1.xml", "officedocument.wordprocessingml.header");
        }
        if self.footer_raw.is_some() {
            add("word/footer1.xml", "officedocument.wordprocessingml.footer");
        }
        if !comments.is_empty() {
            add(
                "word/comments.xml",
                "officedocument.wordprocessingml.comments",
            );
        }
        let png = if has_image {
            r#"<Default Extension="png" ContentType="image/png"/>"#
        } else {
            ""
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>{png}{overrides}</Types>"#
        )
    }

    fn core(&self) -> String {
        let title = self
            .title
            .as_ref()
            .map(|t| format!("<dc:title>{}</dc:title>", esc(t)))
            .unwrap_or_default();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/">{title}</cp:coreProperties>"#
        )
    }

    fn document_rels(&self, comments: &[String], has_image: bool) -> String {
        const BASE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
        let mut rels = format!(
            r#"<Relationship Id="rIdStyles" Type="{BASE}/styles" Target="styles.xml"/><Relationship Id="rIdNumbering" Type="{BASE}/numbering" Target="numbering.xml"/><Relationship Id="rIdSettings" Type="{BASE}/settings" Target="settings.xml"/>"#
        );
        if self.header_text.is_some() {
            rels += &format!(
                r#"<Relationship Id="rIdHeader1" Type="{BASE}/header" Target="header1.xml"/>"#
            );
        }
        if self.footer_raw.is_some() {
            rels += &format!(
                r#"<Relationship Id="rIdFooter1" Type="{BASE}/footer" Target="footer1.xml"/>"#
            );
        }
        let mut link_ids = Vec::new();
        for (id, url) in self.body.iter().flat_map(|p| &p.link_targets) {
            if !link_ids.contains(&id) {
                link_ids.push(id);
                rels += &format!(
                    r#"<Relationship Id="{id}" Type="{BASE}/hyperlink" Target="{}" TargetMode="External"/>"#,
                    esc(url)
                );
            }
        }
        if !comments.is_empty() {
            rels += &format!(
                r#"<Relationship Id="rIdComments" Type="{BASE}/comments" Target="comments.xml"/>"#
            );
        }
        if has_image {
            rels += &format!(
                r#"<Relationship Id="{}" Type="{BASE}/image" Target="media/image1.png"/>"#,
                media::IMAGE_REL_ID
            );
        }
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>"#
        )
    }

    fn settings(&self) -> String {
        let mut settings = String::new();
        if self
            .page_background
            .as_ref()
            .is_some_and(|(_, shown)| *shown)
        {
            settings += "<w:displayBackgroundShape/>";
        }
        if self.template.is_some() {
            settings += r#"<w:attachedTemplate r:id="rIdTemplate"/>"#;
        }
        if let Some(lang) = &self.theme_font_lang {
            settings += &format!(r#"<w:themeFontLang w:val="{}"/>"#, esc(lang));
        }
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="{W}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{settings}</w:settings>"#
        )
    }

    fn styles(&self) -> String {
        let mut defaults = String::new();
        if let Some(font) = &self.font {
            let font = esc(font);
            defaults += &format!(r#"<w:rFonts w:ascii="{font}" w:hAnsi="{font}"/>"#);
        }
        defaults += &format!(r#"<w:sz w:val="{}"/>"#, self.body_size);
        if let Some(lang) = &self.language {
            defaults += &format!(r#"<w:lang w:val="{}"/>"#, esc(lang));
        }
        let mut styles = String::from(
            r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:rPr><w:sz w:val="52"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="ListNumber"><w:name w:val="List Number"/><w:basedOn w:val="Normal"/><w:pPr><w:numPr><w:numId w:val="1"/></w:numPr></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="ListBullet"><w:name w:val="List Bullet"/><w:basedOn w:val="Normal"/><w:pPr><w:numPr><w:numId w:val="2"/></w:numPr></w:pPr></w:style>
"#,
        );
        for (level, size) in [(1, 32), (2, 28), (3, 26), (4, 24)] {
            styles += &format!(
                r#"<w:style w:type="paragraph" w:styleId="Heading{level}"><w:name w:val="heading {level}"/><w:basedOn w:val="Normal"/><w:pPr><w:outlineLvl w:val="{}"/></w:pPr><w:rPr><w:b/><w:sz w:val="{size}"/></w:rPr></w:style>
"#,
                level - 1
            );
        }
        for style in &self.custom_styles {
            let outline = style
                .outline_level
                .map(|l| format!(r#"<w:pPr><w:outlineLvl w:val="{l}"/></w:pPr>"#))
                .unwrap_or_default();
            styles += &format!(
                r#"<w:style w:type="paragraph" w:customStyle="1" w:styleId="{}"><w:name w:val="{}"/><w:basedOn w:val="Normal"/>{outline}</w:style>
"#,
                esc(&style.id),
                esc(&style.name)
            );
        }
        if !self.table_styles.is_empty() {
            // Word's own definition of the default table style, which a table
            // style is based on.
            styles += r#"<w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/><w:uiPriority w:val="99"/><w:semiHidden/><w:unhideWhenUsed/><w:tblPr><w:tblInd w:w="0" w:type="dxa"/><w:tblCellMar><w:top w:w="0" w:type="dxa"/><w:left w:w="108" w:type="dxa"/><w:bottom w:w="0" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style>
"#;
            styles += &self.table_styles.concat();
        }
        for (id, name, rpr) in &self.char_styles {
            styles += &format!(
                r#"<w:style w:type="character" w:styleId="{}"><w:name w:val="{}"/><w:rPr>{rpr}</w:rPr></w:style>
"#,
                esc(id),
                esc(name)
            );
        }
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr>{defaults}</w:rPr></w:rPrDefault></w:docDefaults>
{styles}</w:styles>"#
        )
    }
}

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/></Relationships>"#;

const NUMBERING: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="singleLevel"/><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:pStyle w:val="ListNumber"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId="1"><w:multiLevelType w:val="singleLevel"/><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:pStyle w:val="ListBullet"/><w:lvlText w:val="&#8226;"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num></w:numbering>"#;

fn settings_rels(template: &str) -> String {
    let encoded = template.replace(' ', "%20");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdTemplate" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/attachedTemplate" Target="file:///C:\Users\test\AppData\Roaming\Microsoft\Templates\{}" TargetMode="External"/></Relationships>"#,
        esc(&encoded)
    )
}

/// A fixture file for one rule.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub rule: RuleId,
    /// True for the failing fixture, false for the passing one.
    pub fails: bool,
    pub bytes: Vec<u8>,
}

impl Fixture {
    /// For example `HDG-002-fail.docx`.
    pub fn file_name(&self) -> String {
        let kind = if self.fails { "fail" } else { "pass" };
        format!("{}-{kind}.docx", self.rule)
    }
}

/// A clean document: it has a title, a language, the template, one Heading 1
/// and one paragraph. It has no findings.
pub fn clean() -> DocxBuilder {
    DocxBuilder::new()
        .title("Adapdf test guide")
        .language("en-US")
        .template(TEMPLATE_FILE_NAME)
        .heading(1, "Test guide")
        .para("This is a short paragraph of body text.")
}

/// A document with the same settings as [`clean`] but no content.
fn clean_settings() -> DocxBuilder {
    DocxBuilder::new()
        .title("Adapdf test guide")
        .language("en-US")
        .template(TEMPLATE_FILE_NAME)
}

/// Fonts for the DOC-006 fixtures: Arial can be embedded, "Restricted Test
/// Font" cannot, and every other font is not installed.
pub struct FixtureFonts;

impl FontLookup for FixtureFonts {
    fn status(&self, family: &str) -> FontStatus {
        match family {
            "Arial" => FontStatus::Embeddable,
            "Restricted Test Font" => FontStatus::Restricted,
            _ => FontStatus::NotInstalled,
        }
    }
}

/// The passing and failing fixture for every rule, in rule order. The
/// failing fixture of a rule makes that rule fire. The passing fixture has no
/// findings at all.
pub fn catalogue() -> Vec<Fixture> {
    let long_text = words(401);
    let mut pairs: Vec<(RuleId, DocxBuilder, DocxBuilder)> = vec![
        (RuleId::Doc001, clean(), clean().title("")),
        (RuleId::Doc002, clean(), clean().no_language()),
        (
            RuleId::Doc003,
            clean().paragraph(Para::new("Привет мир").lang("ru-RU")),
            clean().paragraph(Para::new("Привет мир")),
        ),
        (RuleId::Doc004, clean(), clean().no_template()),
        (
            RuleId::Doc005,
            clean(),
            clean().paragraph(Para::empty().then(Run::new("Added text").inserted())),
        ),
        (
            RuleId::Doc006,
            clean().paragraph(Para::new("Text in Arial.").font("Arial")),
            clean().paragraph(Para::new("Text in a locked font.").font("Restricted Test Font")),
        ),
        (
            RuleId::Hdg001,
            clean_settings().heading(1, "Long guide").para(&long_text),
            clean_settings().para(&long_text),
        ),
        (
            RuleId::Hdg002,
            clean_settings()
                .heading(1, "One")
                .heading(2, "Two")
                .heading(3, "Three"),
            clean_settings().heading(1, "One").heading(3, "Three"),
        ),
        (
            RuleId::Hdg003,
            clean()
                .heading(2, "Setting up the printer")
                .para("Plug it in."),
            clean()
                .paragraph(Para::new("Setting up the printer").bold())
                .para("Plug it in."),
        ),
        (
            RuleId::Hdg004,
            clean().heading(2, "Setup").para("Plug it in."),
            clean().heading(2, "").para("Plug it in."),
        ),
        (
            RuleId::Hdg006,
            clean()
                .custom_style("Callout", "Callout", None)
                .paragraph(Para::new("Look here.").style("Callout")),
            clean()
                .custom_style("Callout", "Callout", Some(1))
                .paragraph(Para::new("Look here.").style("Callout")),
        ),
        (
            RuleId::Lst001,
            clean()
                .paragraph(Para::new("Open the file.").style("ListNumber"))
                .paragraph(Para::new("Save the file.").style("ListNumber")),
            clean()
                .paragraph(Para::new("1. Open the file."))
                .paragraph(Para::new("2. Save the file.")),
        ),
        (
            RuleId::Lst002,
            clean()
                .paragraph(Para::new("Open the file.").style("ListNumber"))
                .paragraph(Para::new("Save the file.").style("ListNumber"))
                .paragraph(Para::new("Close the file.").style("ListNumber")),
            clean()
                .paragraph(Para::new("Open the file.").style("ListNumber"))
                .paragraph(Para::new("Save the file.").style("ListNumber"))
                .paragraph(Para::new("3. Close the file.")),
        ),
    ];
    pairs.extend(objects::pairs());
    pairs.extend(links_colour_text::pairs());
    pairs
        .into_iter()
        .flat_map(|(rule, pass, fail)| {
            [
                Fixture {
                    rule,
                    fails: false,
                    bytes: pass.build(),
                },
                Fixture {
                    rule,
                    fails: true,
                    bytes: fail.build(),
                },
            ]
        })
        .collect()
}
