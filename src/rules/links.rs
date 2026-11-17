//! Link rules LNK-001 to LNK-005.
//!
//! A link is a `w:hyperlink` element or a `HYPERLINK` field in the body
//! ([`Document::links`]). Its text is the text that shows. Links in headers,
//! footers, footnotes and endnotes are not read.

use std::collections::HashSet;

use crate::findings::{Finding, Location, RuleId, snippet};
use crate::model::Document;
use crate::model::fields::{Link, LinkTarget, Toc, has_switch};

/// LNK-001: link texts that do not say where the link goes. The whole text
/// must match, ignoring case, spaces and punctuation at either end.
pub const VAGUE_LINK_TEXTS: &[&str] = &["click here", "here", "link", "more", "read more"];

/// LNK-002: a link text that is a web address longer than this many
/// characters is reported. Exactly this many passes.
pub const MAX_URL_LINK_CHARS: usize = 40;

/// LNK-003: bookmarks that Word has without a `w:bookmarkStart` in the file.
pub const BUILT_IN_BOOKMARKS: &[&str] = &["_top"];

fn at(doc: &Document, link: &Link) -> Location {
    Location::paragraph(doc, &doc.paragraphs[link.paragraph])
}

/// Lower case, with spaces folded and punctuation removed from both ends.
fn normalise(text: &str) -> String {
    text.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// LNK-001: the link text is vague.
pub fn lnk_001(doc: &Document) -> Vec<Finding> {
    doc.links
        .iter()
        .filter(|l| VAGUE_LINK_TEXTS.contains(&normalise(&l.text).as_str()))
        .map(|l| {
            let text = l.text.trim();
            Finding::new(
                RuleId::Lnk001,
                format!("The link text \"{text}\" does not say where the link goes. Screen reader users often list all the links in a document, and \"{text}\" means nothing out of context."),
                "Change the link text to name the place the link goes to, for example the title of the page or document.",
                at(doc, l),
            )
        })
        .collect()
}

/// True for text that starts with `http://`, `https://` or `www.` and has no
/// spaces: a bare web address.
fn is_bare_url(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["http://", "https://", "www."]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
        && !text.chars().any(char::is_whitespace)
}

/// LNK-002: the link text is a bare web address longer than
/// [`MAX_URL_LINK_CHARS`] characters.
pub fn lnk_002(doc: &Document) -> Vec<Finding> {
    doc.links
        .iter()
        .filter(|l| {
            let text = l.text.trim();
            is_bare_url(text) && text.chars().count() > MAX_URL_LINK_CHARS
        })
        .map(|l| {
            let text = l.text.trim();
            Finding::new(
                RuleId::Lnk002,
                format!("The link text is a long web address ({} characters). A screen reader reads it out letter by letter.", text.chars().count()),
                "Replace the address with words that describe the page, for example the page title. Select the link text and type the new words; the link stays.",
                at(doc, l),
            )
        })
        .collect()
}

/// LNK-003: an internal link points to a bookmark that does not exist.
/// Bookmark names are compared without regard to case, as Word does.
pub fn lnk_003(doc: &Document) -> Vec<Finding> {
    let known: HashSet<String> = doc
        .bookmarks
        .iter()
        .map(String::as_str)
        .chain(BUILT_IN_BOOKMARKS.iter().copied())
        .map(str::to_lowercase)
        .collect();
    doc.links
        .iter()
        .filter_map(|l| match &l.target {
            LinkTarget::Internal(name) if !known.contains(&name.to_lowercase()) => Some((l, name)),
            _ => None,
        })
        .map(|(l, name)| {
            Finding::new(
                RuleId::Lnk003,
                format!("This link goes to the bookmark \"{name}\", but the document has no bookmark with that name. The link does not work."),
                "Select the link, choose Insert > Link, and pick the place in the document again. Or remove the link.",
                at(doc, l),
            )
        })
        .collect()
}

/// The links that LNK-004 reports, in document order: links that have no
/// ScreenTip and are not entries of a table of contents (LNK-005 covers those).
/// The fix uses the same list, so the position of a finding is the position
/// here.
pub fn links_without_screen_tip(doc: &Document) -> impl Iterator<Item = &Link> {
    doc.links
        .iter()
        .filter(|l| !l.in_toc && !l.has_screen_tip())
}

/// LNK-004: a link has no ScreenTip. Word writes the ScreenTip into the PDF as
/// the description of the link, and writes no description for a link without
/// one (test plan, "S5-10 result").
pub fn lnk_004(doc: &Document) -> Vec<Finding> {
    links_without_screen_tip(doc)
        .map(|l| {
            let text = snippet(&l.text);
            let which = if text.is_empty() {
                "This link".to_string()
            } else {
                format!("The link \"{text}\"")
            };
            Finding::new(
                RuleId::Lnk004,
                format!("{which} has no ScreenTip. Screen readers need a description of each link in the PDF, and Word adds it to the PDF only from the ScreenTip."),
                "Right-click the link, choose Edit Hyperlink, choose ScreenTip, and type a short description of where the link goes. The tool can copy the link text into the ScreenTip for you.",
                at(doc, l),
            )
        })
        .collect()
}

/// The tables of contents that LNK-005 reports, in document order: a `TOC`
/// field with the `\h` switch. The fix uses the same list.
pub fn tocs_made_of_links(doc: &Document) -> impl Iterator<Item = &Toc> {
    doc.tocs.iter().filter(|t| has_switch(&t.instruction, "h"))
}

/// LNK-005: the table of contents is made of links (the `\h` switch). Word
/// makes the links again at each update, without ScreenTips.
pub fn lnk_005(doc: &Document) -> Vec<Finding> {
    tocs_made_of_links(doc)
        .map(|t| {
            Finding::new(
                RuleId::Lnk005,
                "The table of contents is made of links (its field has the \\h switch). Screen readers need a description of each link in the PDF, and Word adds it only from a ScreenTip. Word cannot keep a ScreenTip on these links, because it makes them again each time the table updates.",
                "Choose References > Table of Contents > Custom Table of Contents, clear \"Use hyperlinks instead of page numbers\", and choose OK. The tool can do the same for you. Readers still move through the PDF with its bookmarks and headings.",
                Location::paragraph(doc, &doc.paragraphs[t.first_paragraph]),
            )
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_trims_punctuation_and_folds_case() {
        assert_eq!(normalise("  Click   HERE! "), "click here");
        assert_eq!(normalise("(here)"), "here");
        assert_eq!(normalise("here to read"), "here to read");
        assert_eq!(normalise("..."), "");
    }

    #[test]
    fn bare_urls() {
        assert!(is_bare_url("https://example.org/a"));
        assert!(is_bare_url("HTTP://EXAMPLE.ORG"));
        assert!(is_bare_url("www.example.org"));
        assert!(!is_bare_url("https://example.org is our site"));
        assert!(!is_bare_url("example.org"));
        assert!(!is_bare_url("The www.example.org site"));
    }
}
