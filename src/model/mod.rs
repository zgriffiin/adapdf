//! A typed, read-only view of a .docx for the rules.
//!
//! [`Document::load`] reads the parts of a [`Package`] once and builds owned
//! data, so rules never touch XML. Build only what a rule needs, and add new
//! fields here when a later rule needs them.
//!
//! Paragraph indexes count the body paragraphs in reading order, including
//! paragraphs in table cells. Paragraphs inside text boxes are not in the
//! list; they are marked on the paragraph that holds the text box (see
//! [`Drawing::text_box`]).

use std::ops::Range;
use std::path::Path;

use crate::docx::{
    COMMENTS_PART, CORE_PART, DC_NS, DOCUMENT_PART, ENDNOTES_PART, FOOTNOTES_PART, NUMBERING_PART,
    Package, SETTINGS_PART, STYLES_PART, W_NS, parse_xml,
};
use crate::error::{Error, Result};

mod body;
pub mod colour;
pub mod fields;
pub mod numbering;
pub mod props;
pub mod styles;
pub mod theme;

use numbering::Numbering;
use props::{ParagraphProps, RunProps};
use styles::Styles;
use theme::{Rgb, Theme};

const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

#[derive(Debug, Clone)]
pub struct Document {
    /// Body paragraphs in reading order.
    pub paragraphs: Vec<Paragraph>,
    /// Page layout sections; the last one is the document's final section.
    pub sections: Vec<Section>,
    pub styles: Styles,
    pub numbering: Numbering,
    pub core: CoreProperties,
    pub settings: Settings,
    pub markup: Markup,
    /// Names of the header and footer parts, for example `word/header1.xml`.
    pub header_footer_parts: Vec<String>,
    /// Properties of text in the default paragraph style (usually Normal).
    pub body_run: RunProps,
    /// The document's default language tag, for example `en-US`. It comes
    /// from the default paragraph style, then the document defaults, then the
    /// theme font language in `settings.xml`.
    pub default_language: Option<String>,
    /// Links in the body (`w:hyperlink` elements and `HYPERLINK` fields).
    pub links: Vec<fields::Link>,
    /// The name of every bookmark in `word/document.xml`.
    pub bookmarks: Vec<String>,
    pub tocs: Vec<fields::Toc>,
    /// The page colour (`w:background`), whether or not Word shows it. See
    /// [`Settings::display_background_shape`].
    pub page_background: Option<Rgb>,
    /// The text of each header and footer part.
    pub header_footers: Vec<fields::HeaderFooter>,
}

#[derive(Debug, Clone)]
pub struct Paragraph {
    /// Position in [`Document::paragraphs`], counting from 0.
    pub index: usize,
    /// The style that applies: the one the paragraph names, or the default.
    pub style_id: Option<String>,
    pub style_name: Option<String>,
    /// 1 to 9 when the style is a built-in Heading style.
    pub heading_level: Option<u8>,
    pub text: String,
    pub runs: Vec<Run>,
    /// Effective properties after style inheritance and direct formatting.
    pub props: ParagraphProps,
    pub in_table: bool,
    /// The fill behind the paragraph that comes from its table cell or table.
    pub container_fill: Option<Rgb>,
    pub drawings: Vec<Drawing>,
    /// Index into [`Document::sections`].
    pub section: usize,
    /// Byte range of the `w:p` element in `word/document.xml` (the text from
    /// [`Package::text`]). A later milestone splices fixes at this range.
    pub range: Range<usize>,
}

impl Paragraph {
    /// No text (spaces do not count) and no drawing.
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty() && self.drawings.is_empty()
    }

    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }
}

#[derive(Debug, Clone)]
pub struct Run {
    pub text: String,
    /// Effective properties after style inheritance and direct formatting.
    pub props: RunProps,
    /// The id of the character style the run names, if any.
    pub char_style: Option<String>,
    /// True when the run is inside a `w:hyperlink` element.
    pub in_hyperlink: bool,
}

impl Run {
    /// Some text that shows (spaces and tabs do not count).
    pub fn has_visible_text(&self) -> bool {
        !self.text.trim().is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingKind {
    /// A DrawingML object "in line with text".
    Inline,
    /// A DrawingML object that floats (`wp:anchor`).
    Anchored,
    /// An older VML picture or an embedded object.
    Legacy,
}

/// A picture, shape, chart or text box in a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drawing {
    pub kind: DrawingKind,
    pub text_box: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub columns: u32,
}

impl Default for Section {
    fn default() -> Self {
        Section { columns: 1 }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CoreProperties {
    /// `dc:title` exactly as stored. `None` when the element is missing.
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// The template the document is attached to: the target of
    /// `w:attachedTemplate`, percent-decoded (a path or a `file:` URL).
    pub attached_template: Option<String>,
    /// `w:themeFontLang`, the language of the theme fonts.
    pub theme_font_lang: Option<String>,
    /// `w:displayBackgroundShape` is on. Word shows (and exports) the page
    /// colour only then.
    pub display_background_shape: bool,
}

/// Tracked changes and comments.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Markup {
    /// Revision elements in the body, headers, footers, footnotes and endnotes.
    pub tracked_changes: usize,
    /// Comments in `word/comments.xml`.
    pub comments: usize,
}

impl Document {
    pub fn open(path: &Path) -> Result<Document> {
        Document::load(&Package::open(path)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Document> {
        Document::load(&Package::from_bytes(bytes)?)
    }

    pub fn load(package: &Package) -> Result<Document> {
        let header_footer_parts = package.header_footer_parts()?;
        let theme = match package.theme_part()? {
            Some(name) => match package.text(&name)? {
                Some(text) => Theme::parse(&name, text)?,
                None => Theme::default(),
            },
            None => Theme::default(),
        };
        let styles = match package.text(STYLES_PART)? {
            Some(text) => Styles::parse(STYLES_PART, text, &theme)?,
            None => Styles::default(),
        };
        let numbering = match package.text(NUMBERING_PART)? {
            Some(text) => Numbering::parse(NUMBERING_PART, text)?,
            None => Numbering::default(),
        };
        let settings = load_settings(package)?;

        let ctx = body::Context {
            theme: &theme,
            styles: &styles,
            numbering: &numbering,
        };
        let document_text = package
            .text(DOCUMENT_PART)?
            .ok_or_else(|| Error::NotDocx(format!("it has no {DOCUMENT_PART}")))?;
        let parsed = body::parse(document_text, &ctx)?;
        let body_fields = fields::scan_body(
            document_text,
            &parsed.paragraphs,
            &theme,
            &package.relationships(DOCUMENT_PART)?,
        )?;
        let mut header_footers = Vec::new();
        for name in &header_footer_parts {
            if let Some(text) = package.text(name)? {
                header_footers.push(fields::scan_header_footer(name, text)?);
            }
        }

        let mut markup = Markup {
            tracked_changes: parsed.tracked_changes,
            comments: 0,
        };
        let other_parts = header_footer_parts
            .iter()
            .map(String::as_str)
            .chain([FOOTNOTES_PART, ENDNOTES_PART]);
        for name in other_parts {
            if let Some(text) = package.text(name)? {
                markup.tracked_changes += body::count_tracked_changes(&parse_xml(name, text)?);
            }
        }
        if let Some(text) = package.text(COMMENTS_PART)? {
            markup.comments = parse_xml(COMMENTS_PART, text)?
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "comment")
                .count();
        }

        let body_run = styles
            .paragraph_run_fmt(styles.default_paragraph_id())
            .resolve();
        let default_language = body_run
            .lang
            .val
            .clone()
            .or_else(|| settings.theme_font_lang.clone());

        Ok(Document {
            paragraphs: parsed.paragraphs,
            sections: parsed.sections,
            styles,
            numbering,
            core: load_core(package)?,
            settings,
            markup,
            header_footer_parts,
            body_run,
            default_language,
            links: body_fields.links,
            bookmarks: body_fields.bookmarks,
            tocs: body_fields.tocs,
            page_background: body_fields.page_background,
            header_footers,
        })
    }

    /// Paragraphs with a built-in Heading style and some text, in order.
    /// (An empty heading is not a real heading; rule HDG-004 reports it.)
    pub fn headings(&self) -> impl Iterator<Item = &Paragraph> {
        self.paragraphs
            .iter()
            .filter(|p| p.heading_level.is_some() && !p.is_blank())
    }

    /// The nearest heading before the paragraph.
    pub fn heading_above(&self, index: usize) -> Option<&Paragraph> {
        self.paragraphs[..index.min(self.paragraphs.len())]
            .iter()
            .rev()
            .find(|p| p.heading_level.is_some() && !p.is_blank())
    }

    pub fn word_count(&self) -> usize {
        self.paragraphs.iter().map(Paragraph::word_count).sum()
    }
}

fn load_core(package: &Package) -> Result<CoreProperties> {
    let Some(text) = package.text(CORE_PART)? else {
        return Ok(CoreProperties::default());
    };
    let doc = parse_xml(CORE_PART, text)?;
    let title = doc
        .descendants()
        .find(|n| {
            n.is_element()
                && n.tag_name().namespace() == Some(DC_NS)
                && n.tag_name().name() == "title"
        })
        .map(|n| n.text().unwrap_or("").to_string());
    Ok(CoreProperties { title })
}

fn load_settings(package: &Package) -> Result<Settings> {
    let Some(text) = package.text(SETTINGS_PART)? else {
        return Ok(Settings::default());
    };
    let doc = parse_xml(SETTINGS_PART, text)?;
    let root = doc.root_element();
    let theme_font_lang = root
        .children()
        .find(|n| n.tag_name().name() == "themeFontLang")
        .and_then(|n| n.attribute((W_NS, "val")))
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let attached_template = root
        .children()
        .find(|n| n.tag_name().name() == "attachedTemplate")
        .and_then(|n| n.attribute((R_NS, "id")))
        .and_then(|id| {
            package
                .relationships(SETTINGS_PART)
                .ok()?
                .into_iter()
                .find(|r| r.id == id)
        })
        .map(|r| percent_decode(&r.target));
    let display_background_shape = root
        .children()
        .find(|n| n.tag_name().name() == "displayBackgroundShape")
        .is_some_and(|n| !matches!(n.attribute((W_NS, "val")), Some("0" | "false" | "off")));
    Ok(Settings {
        attached_template,
        theme_font_lang,
        display_background_shape,
    })
}

/// Decodes `%20` style escapes. Invalid escapes are kept as written.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests;
