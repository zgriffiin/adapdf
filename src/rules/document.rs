//! Document rules DOC-001 to DOC-006.

use crate::docx::{CORE_PART, DOCUMENT_PART, SETTINGS_PART, STYLES_PART};
use crate::findings::{Finding, Location, RuleId};
use crate::fonts::{FontLookup, FontStatus};
use crate::model::{Document, Paragraph};

/// The file name DOC-004 looks for in the document's attached template.
pub const TEMPLATE_FILE_NAME: &str = "Adapdf Training Guide.dotx";

/// DOC-003 ignores a run with fewer letters than this in another script, so
/// that a symbol such as the micro sign in "µm" is not read as a language.
pub const MIN_FOREIGN_SCRIPT_LETTERS: usize = 3;

/// DOC-001: the Title property is empty.
pub fn doc_001(doc: &Document) -> Vec<Finding> {
    let has_title = doc
        .core
        .title
        .as_deref()
        .is_some_and(|t| !t.trim().is_empty());
    if has_title {
        return Vec::new();
    }
    let finding = Finding::new(
        RuleId::Doc001,
        "The document has no title in its properties. PDF viewers show this title in the window bar, and screen readers read it out.",
        "In Word, choose File > Info and type a title in the Title box.",
        Location::document(CORE_PART),
    );
    // The first Title or Heading 1 paragraph is only a suggestion.
    let suggestion = doc
        .paragraphs
        .iter()
        .filter(|p| !p.is_blank())
        .find(|p| is_title_style(p) || p.heading_level == Some(1))
        .map(|p| p.text.trim());
    vec![match suggestion {
        Some(text) => finding.with_suggestion(text),
        None => finding,
    }]
}

fn is_title_style(p: &Paragraph) -> bool {
    p.style_name
        .as_deref()
        .is_some_and(|n| n.eq_ignore_ascii_case("title"))
}

/// DOC-002: the default language is not set.
pub fn doc_002(doc: &Document) -> Vec<Finding> {
    if doc.default_language.is_some() {
        return Vec::new();
    }
    vec![Finding::new(
        RuleId::Doc002,
        "The document language is not set. Screen readers need it to pick the right voice and pronunciation.",
        "In Word, press Ctrl+A, choose Review > Language > Set Proofing Language, and pick the language, for example English (United States).",
        Location::document(STYLES_PART),
    )]
}

/// The non-Latin scripts DOC-003 can recognise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Script {
    /// Greek, Cyrillic, Armenian, Devanagari and Thai. Marked with `w:lang w:val`.
    Other,
    /// Chinese, Japanese and Korean. Marked with `w:lang w:eastAsia`.
    EastAsian,
    /// Hebrew, Arabic and Syriac. Marked with `w:lang w:bidi`.
    RightToLeft,
}

impl Script {
    fn description(self) -> &'static str {
        match self {
            Script::Other => "a non-Latin script (such as Greek, Cyrillic or Thai)",
            Script::EastAsian => "Chinese, Japanese or Korean",
            Script::RightToLeft => "a right-to-left script (such as Arabic or Hebrew)",
        }
    }
}

/// The script of a letter, or `None` for Latin letters and for characters
/// that are not letters.
pub(crate) fn script_of(c: char) -> Option<Script> {
    if !c.is_alphabetic() {
        return None;
    }
    match u32::from(c) {
        0x0370..=0x058F | 0x0900..=0x097F | 0x0E00..=0x0E7F => Some(Script::Other),
        0x0590..=0x077F => Some(Script::RightToLeft),
        0x1100..=0x11FF | 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF => {
            Some(Script::EastAsian)
        }
        _ => None,
    }
}

/// `en-US` gives `en`.
fn primary_subtag(tag: &str) -> String {
    tag.split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// DOC-003: text in another script whose language is not marked.
///
/// This tool has no language detector. It recognises text in a non-Latin
/// script (see [`Script`]) whose language tag is missing or has the same main
/// language as the document default. It runs only when the default language
/// is English, which is the product scope (spec 01). Text in another
/// Latin-script language, such as Spanish, is not found. One finding per
/// paragraph.
pub fn doc_003(doc: &Document) -> Vec<Finding> {
    let is_english = |tag: &str| primary_subtag(tag) == "en";
    if !doc.default_language.as_deref().is_some_and(is_english) {
        return Vec::new(); // No default (DOC-002 reports it), or not English.
    }
    let mut findings = Vec::new();
    for p in &doc.paragraphs {
        let unmarked = p.runs.iter().find_map(|run| {
            [Script::Other, Script::EastAsian, Script::RightToLeft]
                .into_iter()
                .find(|script| {
                    let letters = run.text.chars().filter(|c| script_of(*c) == Some(*script));
                    if letters.count() < MIN_FOREIGN_SCRIPT_LETTERS {
                        return false;
                    }
                    let tag = match script {
                        Script::Other => &run.props.lang.val,
                        Script::EastAsian => &run.props.lang.east_asia,
                        Script::RightToLeft => &run.props.lang.bidi,
                    };
                    tag.as_deref().is_none_or(is_english)
                })
        });
        if let Some(script) = unmarked {
            findings.push(Finding::new(
                RuleId::Doc003,
                format!(
                    "This text looks like it is written in {}, but its language is not marked. A screen reader would read it in the document language.",
                    script.description()
                ),
                "Select the text, choose Review > Language > Set Proofing Language, and pick the language of that text.",
                Location::paragraph(doc, p),
            ));
        }
    }
    findings
}

/// DOC-004: the document is not based on the Adapdf template.
///
/// A document made from a template names it in `word/settings.xml`
/// (`w:attachedTemplate`). The document passes when the file name of that
/// template is [`TEMPLATE_FILE_NAME`] (compared without regard to case).
pub fn doc_004(doc: &Document) -> Vec<Finding> {
    let attached = doc.settings.attached_template.as_deref();
    let file_name = attached.map(|t| t.rsplit(['/', '\\']).next().unwrap_or(t));
    if file_name.is_some_and(|n| n.eq_ignore_ascii_case(TEMPLATE_FILE_NAME)) {
        return Vec::new();
    }
    let message = match file_name {
        Some(name) => format!(
            "The document is based on the template \"{name}\", not on the Adapdf Training Guide template."
        ),
        None => "The document is not based on the Adapdf Training Guide template.".to_string(),
    };
    vec![Finding::new(
        RuleId::Doc004,
        message,
        "Start new guides from the template: File > New > Personal > Adapdf Training Guide. To move an existing guide, choose Developer > Document Template > Attach, then select \"Automatically update document styles\".",
        Location::document(SETTINGS_PART),
    )]
}

/// DOC-005: tracked changes or comments.
pub fn doc_005(doc: &Document) -> Vec<Finding> {
    let what = match (doc.markup.tracked_changes > 0, doc.markup.comments > 0) {
        (false, false) => return Vec::new(),
        (true, false) => "tracked changes",
        (false, true) => "comments",
        (true, true) => "tracked changes and comments",
    };
    vec![Finding::new(
        RuleId::Doc005,
        format!(
            "The document has {what}. The PDF must not contain draft markup, and unresolved edits can change what the PDF says."
        ),
        "In Word, open the Review tab. Choose Accept or Reject for every tracked change (Accept > Accept All Changes). Then choose Delete > Delete All Comments in Document. Save the document.",
        Location::document(DOCUMENT_PART),
    )]
}

/// DOC-006: a font in use cannot be embedded.
///
/// "In use" means the font of a run that has text. For Latin text that is the
/// `ascii` font for ASCII characters and the `hAnsi` font for other
/// characters. East Asian and right-to-left fonts are not checked. A font that
/// is not installed gets its own message, because the tool cannot tell
/// whether Word could embed it.
pub fn doc_006(doc: &Document, fonts: &dyn FontLookup) -> Vec<Finding> {
    // Fonts in order of first use, with the first paragraph that uses each.
    let mut used: Vec<(String, &Paragraph)> = Vec::new();
    for p in &doc.paragraphs {
        for run in p.runs.iter().filter(|r| !r.text.trim().is_empty()) {
            let ascii = run.text.chars().any(|c| c.is_ascii() && !c.is_whitespace());
            let other = run.text.chars().any(|c| {
                !c.is_ascii()
                    && !c.is_whitespace()
                    && matches!(script_of(c), None | Some(Script::Other))
            });
            let candidates = [
                ascii.then_some(&run.props.font_ascii),
                other.then_some(&run.props.font_h_ansi),
            ];
            for font in candidates.into_iter().flatten().flatten() {
                if !used.iter().any(|(f, _)| f.eq_ignore_ascii_case(font)) {
                    used.push((font.clone(), p));
                }
            }
        }
    }

    used.into_iter()
        .filter_map(|(font, p)| {
            let (message, how) = match fonts.status(&font) {
                FontStatus::Embeddable => return None,
                FontStatus::Restricted => (
                    format!("The font \"{font}\" does not allow embedding, so Word cannot put it in the PDF. The PDF would show a different font or fail the check."),
                    "Change the text to a font that allows embedding, for example Arial.".to_string(),
                ),
                FontStatus::NotInstalled => (
                    format!("The font \"{font}\" is not installed on this computer, so Adapdf cannot tell whether it can be embedded in the PDF."),
                    "Install the font, or change the text to a font that is installed and allows embedding, for example Arial.".to_string(),
                ),
            };
            Some(Finding::new(RuleId::Doc006, message, how, Location::paragraph(doc, p)))
        })
        .collect()
}
