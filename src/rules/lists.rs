//! List rules LST-001 and LST-002.

use crate::findings::{Finding, Location, RuleId};
use crate::model::{Document, Paragraph};

/// LST-001: the most digits a typed number can have (`12.` yes, `2024.` no).
pub const MAX_TYPED_NUMBER_DIGITS: usize = 3;

/// Characters that are typed as a bullet. `-` and `*` are in the list from
/// spec 03; the others are the bullets people type or paste most. U+F0B7 is
/// the Symbol font bullet.
const BULLET_CHARS: &[char] = &[
    '•', '·', '▪', '●', '○', '◦', '■', '□', '‣', '⁃', '-', '*', '\u{F0B7}',
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerKind {
    Number,
    Bullet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypedMarker {
    /// The marker as typed, for example `1.` or `•`.
    pub text: String,
    pub kind: MarkerKind,
}

/// Finds a typed list marker at the start of a paragraph: a number of up to
/// [`MAX_TYPED_NUMBER_DIGITS`] digits or one letter followed by `.` or `)`,
/// or a bullet character. The marker must be followed by a space or tab and
/// then some text, so `1.5 million` and `-5 degrees` are not markers.
pub(crate) fn typed_marker(text: &str) -> Option<TypedMarker> {
    let text = text.trim_start();
    let first = text.chars().next()?;
    let (marker_len, kind) = if BULLET_CHARS.contains(&first) {
        (first.len_utf8(), MarkerKind::Bullet)
    } else if first.is_ascii_digit() {
        let digits = text.bytes().take_while(u8::is_ascii_digit).count();
        if digits > MAX_TYPED_NUMBER_DIGITS
            || !matches!(text.as_bytes().get(digits), Some(b'.' | b')'))
        {
            return None;
        }
        (digits + 1, MarkerKind::Number)
    } else if first.is_ascii_alphabetic() && matches!(text.as_bytes().get(1), Some(b'.' | b')')) {
        (2, MarkerKind::Number)
    } else {
        return None;
    };
    let (marker, rest) = text.split_at(marker_len);
    let spaced = rest.starts_with(char::is_whitespace);
    (spaced && !rest.trim().is_empty()).then(|| TypedMarker {
        text: marker.to_string(),
        kind,
    })
}

/// The typed marker of a paragraph that is not in a real list. Headings are
/// skipped: typed section numbers such as "1. Overview" are not lists.
fn typed_marker_of(p: &Paragraph) -> Option<TypedMarker> {
    if p.props.num.is_some() || p.heading_level.is_some() {
        return None;
    }
    typed_marker(&p.text)
}

/// LST-001: a typed number or bullet instead of a real list.
pub fn lst_001(doc: &Document) -> Vec<Finding> {
    doc.paragraphs
        .iter()
        .filter_map(|p| {
            let marker = typed_marker_of(p)?;
            let (what, list, style) = match marker.kind {
                MarkerKind::Number => ("number", "numbered", "List Number"),
                MarkerKind::Bullet => ("bullet", "bulleted", "List Bullet"),
            };
            Some(Finding::new(
                RuleId::Lst001,
                format!(
                    "This paragraph starts with a typed {what} (\"{}\") instead of a real {list} list. Screen readers do not announce it as a list.",
                    marker.text
                ),
                format!(
                    "Select the list paragraphs, apply the {style} style or use the Home tab list button, then delete the typed {what}."
                ),
                Location::paragraph(doc, p),
            ))
        })
        .collect()
}

/// What a paragraph is in a numbered list, for LST-002.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Typed,
    Automatic,
}

fn numbered_item(doc: &Document, p: &Paragraph) -> Option<Item> {
    match p.props.num {
        Some(num) => doc.numbering.is_numbered(num).then_some(Item::Automatic),
        None => typed_marker_of(p)
            .filter(|m| m.kind == MarkerKind::Number)
            .map(|_| Item::Typed),
    }
}

/// LST-002: one list mixes typed numbers and automatic numbers.
///
/// A list is a run of paragraphs, one right after the other, that are each a
/// typed number or an automatically numbered item. Any other paragraph ends
/// the list. One finding per list, at its first typed number.
pub fn lst_002(doc: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut list: Vec<(&Paragraph, Item)> = Vec::new();
    // A sentinel `None` after the last paragraph ends the final list.
    let items = doc.paragraphs.iter().map(Some).chain(std::iter::once(None));
    for p in items {
        match p.and_then(|p| Some((p, numbered_item(doc, p)?))) {
            Some(entry) => list.push(entry),
            None => {
                let first_typed = list.iter().find(|(_, item)| *item == Item::Typed);
                let has_automatic = list.iter().any(|(_, item)| *item == Item::Automatic);
                if let (Some((typed, _)), true) = (first_typed, has_automatic) {
                    findings.push(Finding::new(
                        RuleId::Lst002,
                        "This list mixes typed numbers with automatic numbering. The automatic numbers will not count the typed items, so the numbers can be wrong.",
                        "Delete the typed numbers and apply automatic numbering to the whole list (Home tab > Numbering).",
                        Location::paragraph(doc, typed),
                    ));
                }
                list.clear();
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(text: &str) -> Option<(String, MarkerKind)> {
        typed_marker(text).map(|m| (m.text, m.kind))
    }

    #[test]
    fn finds_typed_numbers() {
        use MarkerKind::Number;
        assert_eq!(marker("1. Open the file"), Some(("1.".into(), Number)));
        assert_eq!(marker("12) Save it"), Some(("12)".into(), Number)));
        assert_eq!(marker("a. First choice"), Some(("a.".into(), Number)));
        assert_eq!(marker("B) Second choice"), Some(("B)".into(), Number)));
        assert_eq!(marker("  3.\tTabbed step"), Some(("3.".into(), Number)));
    }

    #[test]
    fn finds_typed_bullets() {
        use MarkerKind::Bullet;
        for bullet in ["•", "-", "*", "·", "▪", "\u{F0B7}"] {
            assert_eq!(
                marker(&format!("{bullet} Item")),
                Some((bullet.to_string(), Bullet)),
                "{bullet}"
            );
        }
        assert_eq!(marker("•\tTabbed"), Some(("•".into(), Bullet)));
    }

    #[test]
    fn digit_limit() {
        assert!(marker("999. Three digits").is_some());
        assert!(marker("2024. Four digits").is_none());
    }

    #[test]
    fn ordinary_text_is_not_a_marker() {
        for text in [
            "",
            "Open the file",
            "1.5 million people",
            "3.14 is pi",
            "-5 degrees",
            "*bold* markup",
            "e.g. an example",
            "U.S. Army",
            "1.",
            "•",
            "- ",
            "1) ",
            "2023 was a year",
            "A.B. Smith",
        ] {
            assert_eq!(marker(text), None, "{text:?}");
        }
    }
}
