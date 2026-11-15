//! Heading rules HDG-001 to HDG-004 and HDG-006. HDG-005 was removed: WCAG 2.1
//! and PDF/UA-1 do not require a single Heading 1.
//!
//! A heading is a paragraph in a built-in Heading 1 to 9 style that has some
//! text ([`Document::headings`]). Title is not a heading here.

use std::collections::HashSet;

use crate::docx::{DOCUMENT_PART, STYLES_PART};
use crate::findings::{Finding, Location, RuleId};
use crate::model::{Document, Paragraph};

/// HDG-001: a document with more words than this is longer than one page.
pub const PAGE_WORD_LIMIT: usize = 400;

/// HDG-003: a paragraph with at most this many words can be a fake heading.
pub const FAKE_HEADING_MAX_WORDS: usize = 12;

/// HDG-003: the deepest heading level the tool suggests. The template does not
/// use levels below 4.
pub const MAX_SUGGESTED_HEADING_LEVEL: u8 = 4;

/// HDG-001: no headings in a document longer than one page.
pub fn hdg_001(doc: &Document) -> Vec<Finding> {
    let words = doc.word_count();
    if doc.headings().next().is_some() || words <= PAGE_WORD_LIMIT {
        return Vec::new();
    }
    vec![Finding::new(
        RuleId::Hdg001,
        format!(
            "The document has about {words} words (more than one page) and no headings. Screen reader users move through a long document by its headings."
        ),
        "Divide the text into sections. In Word, select the title of each section and choose a heading style on the Home tab, for example Heading 1 for the main sections and Heading 2 for the parts inside them.",
        Location::document(DOCUMENT_PART),
    )]
}

/// HDG-002: a heading level is skipped. The first heading counts as a skip
/// when it is not Heading 1.
pub fn hdg_002(doc: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut previous = 0;
    for h in doc.headings() {
        let level = h.heading_level.unwrap_or(0);
        if level > previous + 1 {
            let (message, missing) = if previous == 0 {
                (
                    format!(
                        "The first heading is Heading {level}. The first heading must be Heading 1."
                    ),
                    1,
                )
            } else {
                (
                    format!(
                        "Heading {level} comes right after Heading {previous}. Heading {} is missing.",
                        previous + 1
                    ),
                    previous + 1,
                )
            };
            findings.push(Finding::new(
                RuleId::Hdg002,
                message,
                format!("Change this heading to Heading {missing}, or add a Heading {missing} before it. Use the heading levels in order, and do not choose a level for its size."),
                Location::paragraph(doc, h),
            ));
        }
        previous = level;
    }
    findings
}

/// Built-in styles that are short and large or bold on purpose.
pub(crate) fn is_exempt_style(p: &Paragraph) -> bool {
    p.style_name.as_deref().is_some_and(|name| {
        let name = name.to_ascii_lowercase();
        name == "title" || name == "subtitle" || name == "caption" || name.starts_with("toc")
    })
}

/// HDG-003: a fake heading.
///
/// A paragraph is a fake heading when it has text, has at most
/// [`FAKE_HEADING_MAX_WORDS`] words, is not in a Heading style, and all its
/// text is bold or larger than the body text. To limit false alarms it skips
/// table cells, list items, and the Title, Subtitle, Caption and TOC styles.
pub fn hdg_003(doc: &Document) -> Vec<Finding> {
    let body_size = doc.body_run.size_half_points;
    let mut findings = Vec::new();
    for p in &doc.paragraphs {
        let words = p.word_count();
        if p.heading_level.is_some()
            || p.in_table
            || p.props.num.is_some()
            || is_exempt_style(p)
            || words == 0
            || words > FAKE_HEADING_MAX_WORDS
        {
            continue;
        }
        let mut text_runs = p
            .runs
            .iter()
            .filter(|r| !r.text.trim().is_empty())
            .peekable();
        if text_runs.peek().is_none() {
            continue;
        }
        let (all_bold, all_larger) = text_runs.fold((true, true), |(bold, larger), r| {
            (
                bold && r.props.bold,
                larger && r.props.size_half_points > body_size,
            )
        });
        if !(all_bold || all_larger) {
            continue;
        }
        let level = doc
            .heading_above(p.index)
            .and_then(|h| h.heading_level)
            .map_or(1, |l| (l + 1).min(MAX_SUGGESTED_HEADING_LEVEL));
        findings.push(
            Finding::new(
                RuleId::Hdg003,
                "This short line looks like a heading, but it does not use a heading style. Screen readers cannot find it.",
                "Apply a heading style to it (Home tab > Styles > Heading).",
                Location::paragraph(doc, p),
            )
            .with_suggestion(format!("Heading {level}")),
        );
    }
    findings
}

/// HDG-004: a heading paragraph is empty.
pub fn hdg_004(doc: &Document) -> Vec<Finding> {
    doc.paragraphs
        .iter()
        .filter(|p| p.is_blank())
        .filter_map(|p| {
            let level = p.heading_level?;
            Some(Finding::new(
                RuleId::Hdg004,
                format!("A Heading {level} paragraph is empty. Screen readers announce an empty heading."),
                "Delete the empty paragraph, or type the heading text.",
                Location::paragraph(doc, p),
            ))
        })
        .collect()
}

/// HDG-006: a style that is not a built-in Heading style has an outline
/// level. One finding per style, at the first paragraph that uses it. Styles
/// that no paragraph uses are not reported.
pub fn hdg_006(doc: &Document) -> Vec<Finding> {
    let mut seen = HashSet::new();
    let mut findings = Vec::new();
    for p in &doc.paragraphs {
        let Some(style_id) = p.style_id.as_deref() else {
            continue;
        };
        if p.heading_level.is_some() || !seen.insert(style_id) {
            continue;
        }
        let level = doc
            .styles
            .paragraph_par_fmt(Some(style_id))
            .resolve()
            .outline_level;
        if let Some(level) = level {
            let name = p.style_name.as_deref().unwrap_or(style_id);
            findings.push(Finding::new(
                RuleId::Hdg006,
                format!("The style \"{name}\" has outline level {level} but is not a built-in Heading style. Word may not export its paragraphs as headings."),
                "Use the built-in Heading 1 to Heading 4 styles for headings. Remove the outline level from the style (Modify Style > Format > Paragraph > Outline level: Body Text), or change these paragraphs to a Heading style.",
                Location::paragraph_in(doc, p, STYLES_PART),
            ));
        }
    }
    findings
}
