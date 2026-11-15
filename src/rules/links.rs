//! Link rules LNK-001 to LNK-003.
//!
//! A link is a `w:hyperlink` element or a `HYPERLINK` field in the body
//! ([`Document::links`]). Its text is the text that shows. Links in headers,
//! footers, footnotes and endnotes are not read.

use std::collections::HashSet;

use crate::findings::{Finding, Location, RuleId};
use crate::model::Document;
use crate::model::fields::{Link, LinkTarget};

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
