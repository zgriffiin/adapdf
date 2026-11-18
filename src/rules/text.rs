//! Text rules TXT-001 to TXT-004.
//!
//! A body paragraph, for TXT-001 and TXT-002, is a paragraph in the document
//! body (including table cells) that is not a heading, has no outline level,
//! and is not in a Title, Subtitle, Caption or TOC style. Those styles are
//! small or large on purpose. Text in headers, footers, footnotes and endnotes
//! is not body text.

use crate::findings::{Finding, Location, RuleId, snippet};
use crate::model::Paragraph;
use crate::model::fields::{HeaderFooter, Toc, has_switch, switch_arg};
use crate::model::props::Alignment;
use crate::model::{Document, HeaderFooterKind, HeaderFooterPlace};
use crate::rules::headings::is_exempt_style;

/// TXT-001: body text smaller than this is reported, in half points (10 pt).
/// Exactly this size passes.
pub const MIN_BODY_SIZE_HALF_POINTS: u32 = 20;

/// TXT-003: fields whose result may be in a header or footer: page numbers,
/// dates and times, and the document title.
pub const HEADER_FOOTER_FIELDS: &[&str] = &[
    "PAGE",
    "NUMPAGES",
    "SECTIONPAGES",
    "DATE",
    "TIME",
    "CREATEDATE",
    "SAVEDATE",
    "PRINTDATE",
    "TITLE",
];

/// TXT-003: words that may be typed in a header or footer, next to a page
/// number ("Page 3 of 10"). Numbers may also be typed.
pub const HEADER_FOOTER_WORDS: &[&str] = &["page", "of"];

fn is_body_paragraph(p: &Paragraph) -> bool {
    p.heading_level.is_none() && p.props.outline_level.is_none() && !is_exempt_style(p)
}

/// TXT-001: body text is smaller than 10 pt. One finding for each paragraph,
/// for its smallest run.
pub fn txt_001(doc: &Document) -> Vec<Finding> {
    doc.paragraphs
        .iter()
        .filter(|p| is_body_paragraph(p))
        .filter_map(|p| {
            let smallest = p
                .runs
                .iter()
                .filter(|r| r.has_visible_text())
                .map(|r| r.props.size_half_points)
                .min()?;
            (smallest < MIN_BODY_SIZE_HALF_POINTS).then(|| {
                Finding::new(
                    RuleId::Txt001,
                    format!(
                        "Body text is {} pt. Text smaller than {} pt is hard to read.",
                        f64::from(smallest) / 2.0,
                        MIN_BODY_SIZE_HALF_POINTS / 2
                    ),
                    format!("Select the text and choose a size of at least {} in the Font Size box (Home tab). Change the style (Home > Styles > right-click the style > Modify) if all the text in it is small.", MIN_BODY_SIZE_HALF_POINTS / 2),
                    Location::paragraph(doc, p),
                )
            })
        })
        .collect()
}

/// TXT-002: a body paragraph is justified (`both`, `distribute` or another
/// justified value), after styles. Paragraphs with no visible text are skipped.
pub fn txt_002(doc: &Document) -> Vec<Finding> {
    doc.paragraphs
        .iter()
        .filter(|p| is_body_paragraph(p) && p.props.alignment == Alignment::Justify)
        .filter(|p| p.runs.iter().any(|r| r.has_visible_text()))
        .map(|p| {
            Finding::new(
                RuleId::Txt002,
                "This paragraph is justified. Justified text has uneven gaps between words, which is hard to read for many people, for example with dyslexia.",
                "Select the paragraph and press Ctrl+L to align it to the left. Change the style if it makes paragraphs justified.",
                Location::paragraph(doc, p),
            )
        })
        .collect()
}

/// Words (letters and digits) of a text, lower case.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The title that a header may repeat: the Title property, and the text of
/// the first Title or Heading 1 paragraph (the one DOC-001 suggests).
fn titles(doc: &Document) -> Vec<Vec<String>> {
    let property = doc.core.title.as_deref();
    let first_title = doc
        .paragraphs
        .iter()
        .filter(|p| !p.is_blank())
        .find(|p| {
            p.heading_level == Some(1)
                || p.style_name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case("title"))
        })
        .map(|p| p.text.as_str());
    [property, first_title]
        .into_iter()
        .flatten()
        .map(words)
        .filter(|w| !w.is_empty())
        .collect()
}

/// The words of a text, each with its lower-case form (for comparing) and its
/// byte range in the text (for quoting it as it was written).
fn tokens(text: &str) -> Vec<(String, std::ops::Range<usize>)> {
    let mut found = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                found.push((text[s..i].to_lowercase(), s..i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        found.push((text[s..].to_lowercase(), s..text.len()));
    }
    found
}

/// The typed text of a header or footer that is not allowed, as it was written
/// (with its own capitals and spacing). The text that is the result of an
/// allowed field, the allowed words, numbers and titles are taken out. Words
/// that stay next to each other stay together with what is between them;
/// separate groups are separate items.
fn extra_text(hf: &HeaderFooter, titles: &[Vec<String>]) -> Vec<String> {
    let mut extra = Vec::new();
    for piece in &hf.text {
        if piece
            .field
            .as_deref()
            .is_some_and(|f| HEADER_FOOTER_FIELDS.contains(&f))
        {
            continue;
        }
        let tokens = tokens(&piece.text);
        // The words that are left, as indexes into `tokens`.
        let mut left: Vec<usize> = (0..tokens.len()).collect();
        for title in titles {
            let mut i = 0;
            while i + title.len() <= left.len() {
                if left[i..i + title.len()]
                    .iter()
                    .zip(title)
                    .all(|(t, word)| tokens[*t].0 == *word)
                {
                    left.drain(i..i + title.len());
                } else {
                    i += 1;
                }
            }
        }
        let keep: Vec<usize> = left
            .into_iter()
            .filter(|t| {
                let word = tokens[*t].0.as_str();
                !HEADER_FOOTER_WORDS.contains(&word) && !word.chars().all(|c| c.is_ascii_digit())
            })
            .collect();
        let mut group: Option<(usize, usize)> = None; // first and last token
        for t in keep {
            group = match group {
                Some((first, last)) if last + 1 == t => Some((first, t)),
                Some((first, last)) => {
                    extra.push(piece.text[tokens[first].1.start..tokens[last].1.end].to_string());
                    Some((t, t))
                }
                None => Some((t, t)),
            };
        }
        if let Some((first, last)) = group {
            extra.push(piece.text[tokens[first].1.start..tokens[last].1.end].to_string());
        }
    }
    extra
}

/// TXT-003: a header or footer has text other than the title, page numbers
/// and dates. The text may be typed or the result of a field, other than
/// [`HEADER_FOOTER_FIELDS`]. Typed dates are not recognised: a date has to be
/// a field. One finding for each part. The message quotes the text as it was
/// written.
pub fn txt_003(doc: &Document) -> Vec<Finding> {
    let titles = titles(doc);
    doc.header_footers
        .iter()
        .filter_map(|hf| {
            let extra = extra_text(hf, &titles);
            if extra.is_empty() {
                return None;
            }
            let places = doc.header_footer_places(&hf.part);
            let footer = places
                .first()
                .map_or_else(|| hf.part.contains("footer"), |p| p.footer);
            let kind = if footer { "footer" } else { "header" };
            let shown = snippet(&extra.join(" "));
            let how = how_to_open(doc, &places, footer);
            Some(Finding::new(
                RuleId::Txt003,
                format!("The {kind} has text other than the title, page numbers and dates (\"{shown}\"). Screen readers skip headers and footers in the PDF, so people who use them do not get this text."),
                format!("{how} Delete \"{shown}\", or move it into the body of the document. Keep only the title, page numbers and dates in the {kind}."),
                Location::header_footer(doc, &hf.part, &shown),
            ))
        })
        .collect()
}

/// "Double-click the top of a page in section 2 to open the header."
fn how_to_open(doc: &Document, places: &[HeaderFooterPlace], footer: bool) -> String {
    let (edge, kind) = if footer {
        ("bottom", "footer")
    } else {
        ("top", "header")
    };
    let Some(place) = places.first() else {
        return format!("Double-click the {edge} of a page to open the {kind}.");
    };
    let page = match place.kind {
        HeaderFooterKind::Default => "a page",
        HeaderFooterKind::First => "the first page",
        HeaderFooterKind::Even => "an even-numbered page",
    };
    let section = match (doc.sections.len() > 1, place.sections.first()) {
        (true, Some(s)) => format!(" in section {}", s + 1),
        _ => String::new(),
    };
    format!("Double-click the {edge} of {page}{section} to open the {kind}.")
}

/// The levels of headings a TOC lists: `\o "1-3"` gives 1 to 3. Without `\o`
/// all levels (1 to 9) count.
fn toc_levels(instruction: &str) -> (u8, u8) {
    let parsed = switch_arg(instruction, "o").and_then(|range| {
        let (low, high) = range.split_once('-')?;
        Some((low.trim().parse().ok()?, high.trim().parse().ok()?))
    });
    parsed.unwrap_or((1, 9))
}

/// Folds all white space (tabs and line breaks too) to single spaces.
fn fold_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The heading text in a TOC entry: the text without the page number (the
/// part after the last tab) and without the heading number (a part before the
/// heading text). `\n` switches page numbers off.
fn entry_text(entry: &str, page_numbers: bool) -> String {
    let mut parts: Vec<&str> = entry.split('\t').collect();
    if page_numbers && parts.len() > 1 {
        parts.pop();
    }
    fold_space(parts.last().copied().unwrap_or(""))
}

/// The headings that the TOC should list, in order.
fn expected_entries(doc: &Document, toc: &Toc) -> Vec<String> {
    let (low, high) = toc_levels(&toc.instruction);
    let by_outline = has_switch(&toc.instruction, "u");
    doc.paragraphs
        .iter()
        .filter(|p| !p.is_blank())
        .filter(|p| {
            let level = if by_outline {
                p.props.outline_level
            } else {
                p.heading_level
            };
            level.is_some_and(|l| (low..=high).contains(&l))
        })
        .map(|p| fold_space(&p.text))
        .collect()
}

/// What is different between the headings and the entries, for the message.
/// `None` when they match. With `\t` or `\b` the TOC can list other
/// paragraphs, or only part of the document, so only a missing heading counts.
fn first_difference(expected: &[String], actual: &[String], lenient: bool) -> Option<String> {
    if lenient {
        let missing = expected.iter().find(|h| !actual.contains(h))?;
        return Some(format!("it has no entry for the heading \"{missing}\""));
    }
    for i in 0..expected.len().max(actual.len()) {
        match (expected.get(i), actual.get(i)) {
            (Some(h), Some(e)) if h != e => {
                return Some(format!(
                    "entry {} is \"{e}\" but the heading is \"{h}\"",
                    i + 1
                ));
            }
            (Some(h), None) => {
                return Some(format!("it has no entry for the heading \"{h}\""));
            }
            (None, Some(e)) => {
                return Some(format!("the entry \"{e}\" is not a heading"));
            }
            _ => {}
        }
    }
    None
}

/// TXT-004: the table of contents is out of date. It is when its field is
/// marked dirty, or its entries are not the current headings. The headings are
/// the built-in Heading styles in the levels of its `\o` switch (all levels if
/// there is none), or the paragraphs with an outline level in those levels when
/// it has `\u`. Entry text is compared without the page number, the heading
/// number, and differences in white space.
pub fn txt_004(doc: &Document) -> Vec<Finding> {
    doc.tocs
        .iter()
        .filter_map(|toc| {
            let why = if toc.dirty {
                "Word has marked it as needing an update".to_string()
            } else {
                let page_numbers = !has_switch(&toc.instruction, "n");
                let actual: Vec<String> = toc
                    .entries
                    .iter()
                    .map(|e| entry_text(e, page_numbers))
                    .collect();
                let lenient = has_switch(&toc.instruction, "t") || has_switch(&toc.instruction, "b");
                first_difference(&expected_entries(doc, toc), &actual, lenient)?
            };
            Some(Finding::new(
                RuleId::Txt004,
                format!("The table of contents is out of date: {why}. A reader who follows it can land in the wrong place."),
                "In Word, click in the table of contents and choose Update Table, then Update entire table (or press F9). Adapdf can also update the fields when it exports the PDF.",
                Location::paragraph(doc, &doc.paragraphs[toc.first_paragraph]),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toc_levels_come_from_the_o_switch() {
        assert_eq!(toc_levels(r#"TOC \o "1-3" \h"#), (1, 3));
        assert_eq!(toc_levels(r#"TOC \o "2-4""#), (2, 4));
        assert_eq!(toc_levels(r"TOC \h"), (1, 9));
        assert_eq!(toc_levels(r#"TOC \o "Headings""#), (1, 9));
    }

    #[test]
    fn entry_text_drops_the_page_number_and_the_heading_number() {
        assert_eq!(entry_text("Setup\t3", true), "Setup");
        assert_eq!(entry_text("1.2\tSetup\t3", true), "Setup");
        assert_eq!(entry_text("Setup", true), "Setup");
        assert_eq!(entry_text("1.2\tSetup", false), "Setup");
        assert_eq!(entry_text("Set  up \t 3", true), "Set up");
    }

    #[test]
    fn words_split_on_anything_that_is_not_a_letter_or_digit() {
        assert_eq!(
            words("Page 3 of 10 - Draft!"),
            ["page", "3", "of", "10", "draft"]
        );
    }

    #[test]
    fn differences_are_described() {
        let h = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(first_difference(&h(&["a"]), &h(&["a"]), false), None);
        assert!(
            first_difference(&h(&["a", "b"]), &h(&["a"]), false)
                .unwrap()
                .contains("\"b\"")
        );
        assert!(
            first_difference(&h(&["a"]), &h(&["a", "x"]), false)
                .unwrap()
                .contains("not a heading")
        );
        assert!(
            first_difference(&h(&["a"]), &h(&["z"]), false)
                .unwrap()
                .contains("entry 1")
        );
        assert_eq!(first_difference(&h(&["a"]), &h(&["a", "x"]), true), None);
    }
}
