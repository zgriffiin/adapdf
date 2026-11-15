//! Text box and layout rules TXB-001 to TXB-004.
//!
//! Text in a text box is not a body paragraph (see the module notes in
//! `model::drawings`), so the other rules never see it. TXB-001 reports it.

use crate::findings::{Finding, Location, RuleId, snippet};
use crate::model::{Document, Paragraph};

/// TXB-003: more spaces than this in a row is spacing made with spaces.
pub const MAX_SPACES_IN_A_ROW: usize = 2;

/// TXB-003: more tabs than this in a row is spacing made with tabs.
pub const MAX_TABS_IN_A_ROW: usize = 1;

/// TXB-004: more empty paragraphs than this in a row is spacing made with
/// empty paragraphs.
pub const MAX_EMPTY_PARAGRAPHS_IN_A_ROW: usize = 1;

/// TXB-002: a section with more columns than this is a column section.
pub const MAX_COLUMNS: u32 = 1;

/// TXB-001: a text box or shape contains text. One finding for each text box.
/// The snippet in the location is the text in the box.
pub fn txb_001(doc: &Document) -> Vec<Finding> {
    doc.paragraphs
        .iter()
        .flat_map(|p| p.drawings.iter().map(move |d| (p, d)))
        .filter(|(_, d)| d.has_box_text())
        .map(|(p, d)| {
            Finding::new(
                RuleId::Txb001,
                format!(
                    "A text box contains text (\"{}\"). Text boxes are often skipped or read in the wrong order by screen readers.",
                    snippet(&d.text)
                ),
                "Move the text into the body of the document and delete the text box. To keep the look, use a paragraph style (for example a callout style) with a border or shading.",
                Location {
                    snippet: Some(snippet(&d.text)),
                    ..Location::paragraph(doc, p)
                },
            )
        })
        .collect()
}

/// TXB-002: a section has more than one column. One finding for the
/// document, at the first paragraph of the first column section.
pub fn txb_002(doc: &Document) -> Vec<Finding> {
    let column_sections: Vec<usize> = doc
        .sections
        .iter()
        .enumerate()
        .filter(|(_, s)| s.columns > MAX_COLUMNS)
        .map(|(i, _)| i)
        .collect();
    let Some(&first) = column_sections.first() else {
        return Vec::new();
    };
    let location = match doc.paragraphs.iter().find(|p| p.section == first) {
        Some(p) => Location::paragraph(doc, p),
        None => Location::document(crate::docx::DOCUMENT_PART),
    };
    let count = column_sections.len();
    vec![Finding::new(
        RuleId::Txb002,
        format!(
            "The document has {count} section{} with more than one column. The reading order of columns may be wrong in the PDF.",
            if count == 1 { "" } else { "s" }
        ),
        "Check the reading order in PAC. If it is wrong, use one column (Layout > Columns > One) or a table.",
        location,
    )]
}

/// True when the text has more than [`MAX_SPACES_IN_A_ROW`] spaces or more
/// than [`MAX_TABS_IN_A_ROW`] tabs in a row. The text of a paragraph joins
/// its runs, so a row of spaces that crosses runs counts.
fn has_spacing_row(text: &str) -> bool {
    let (mut spaces, mut tabs) = (0, 0);
    for c in text.chars() {
        match c {
            ' ' => (spaces, tabs) = (spaces + 1, 0),
            '\t' => (spaces, tabs) = (0, tabs + 1),
            _ => (spaces, tabs) = (0, 0),
        }
        if spaces > MAX_SPACES_IN_A_ROW || tabs > MAX_TABS_IN_A_ROW {
            return true;
        }
    }
    false
}

/// TXB-003: spacing made with spaces or tabs. A paragraph with no text but
/// spaces is an empty paragraph (TXB-004), not reported here.
pub fn txb_003(doc: &Document) -> Vec<Finding> {
    doc.paragraphs
        .iter()
        .filter(|p| !p.text.trim().is_empty() && has_spacing_row(&p.text))
        .map(|p| {
            Finding::new(
                RuleId::Txb003,
                format!(
                    "This paragraph uses {} or more spaces in a row, or {} or more tabs in a row, to make space. Screen readers may read the gap wrongly, and the layout breaks when the text changes.",
                    MAX_SPACES_IN_A_ROW + 1,
                    MAX_TABS_IN_A_ROW + 1
                ),
                "Delete the extra spaces or tabs. Use paragraph indents, spacing before or after, or a table with real columns.",
                Location::paragraph(doc, p),
            )
        })
        .collect()
}

/// An empty paragraph for TXB-004: not in a table, and just space (see
/// [`Paragraph::is_spacing_only`]). A paragraph with only a picture, an
/// equation, a page or section break or a field is not empty. A paragraph with
/// only spaces or line breaks is empty. Paragraphs in a table are skipped:
/// every empty cell has one, and they are not in a row in the document.
fn is_empty_spacer(p: &Paragraph) -> bool {
    !p.in_table && p.is_spacing_only()
}

/// TXB-004: more than [`MAX_EMPTY_PARAGRAPHS_IN_A_ROW`] empty paragraphs in a
/// row. One finding for each row of them, at the first.
pub fn txb_004(doc: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut rest = doc.paragraphs.as_slice();
    while let Some(start) = rest.iter().position(is_empty_spacer) {
        let run = rest[start..]
            .iter()
            .take_while(|p| is_empty_spacer(p))
            .count();
        if run > MAX_EMPTY_PARAGRAPHS_IN_A_ROW {
            findings.push(Finding::new(
                RuleId::Txb004,
                format!(
                    "{run} empty paragraphs in a row are used to make space. Screen readers may announce each one."
                ),
                "Delete the extra empty paragraphs. Use spacing before or after the paragraph (Paragraph settings), or a page break before, to make space.",
                Location::paragraph(doc, &rest[start]),
            ));
        }
        rest = &rest[start + run..];
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_rows() {
        assert!(!has_spacing_row("a  b"));
        assert!(has_spacing_row("a   b"));
        assert!(!has_spacing_row("a\tb"));
        assert!(has_spacing_row("a\t\tb"));
        assert!(!has_spacing_row("a \t \t b  c"));
        assert!(!has_spacing_row("a\t b\t c"));
    }
}
