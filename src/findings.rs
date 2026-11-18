//! What a check reports: [`Finding`] and the types it is made of.
//!
//! Messages are for staff who are not accessibility specialists. They say
//! what is wrong in plain words and how to fix it in Word.

use serde::{Deserialize, Serialize};

use crate::docx::{CORE_PART, DOCUMENT_PART, SETTINGS_PART, STYLES_PART};
use crate::model::{Document, Paragraph};

/// Number of characters of paragraph text kept in a [`Location`] snippet.
pub const SNIPPET_CHARS: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RuleId {
    #[serde(rename = "DOC-001")]
    Doc001,
    #[serde(rename = "DOC-002")]
    Doc002,
    #[serde(rename = "DOC-003")]
    Doc003,
    #[serde(rename = "DOC-004")]
    Doc004,
    #[serde(rename = "DOC-005")]
    Doc005,
    #[serde(rename = "DOC-006")]
    Doc006,
    #[serde(rename = "HDG-001")]
    Hdg001,
    #[serde(rename = "HDG-002")]
    Hdg002,
    #[serde(rename = "HDG-003")]
    Hdg003,
    #[serde(rename = "HDG-004")]
    Hdg004,
    #[serde(rename = "HDG-006")]
    Hdg006,
    #[serde(rename = "IMG-001")]
    Img001,
    #[serde(rename = "IMG-002")]
    Img002,
    #[serde(rename = "IMG-003")]
    Img003,
    #[serde(rename = "IMG-004")]
    Img004,
    #[serde(rename = "IMG-005")]
    Img005,
    #[serde(rename = "IMG-006")]
    Img006,
    #[serde(rename = "IMG-007")]
    Img007,
    #[serde(rename = "TXB-001")]
    Txb001,
    #[serde(rename = "TXB-002")]
    Txb002,
    #[serde(rename = "TXB-003")]
    Txb003,
    #[serde(rename = "TXB-004")]
    Txb004,
    #[serde(rename = "LST-001")]
    Lst001,
    #[serde(rename = "LST-002")]
    Lst002,
    #[serde(rename = "TBL-001")]
    Tbl001,
    #[serde(rename = "TBL-002")]
    Tbl002,
    #[serde(rename = "TBL-003")]
    Tbl003,
    #[serde(rename = "TBL-004")]
    Tbl004,
    #[serde(rename = "TBL-005")]
    Tbl005,
    #[serde(rename = "TBL-006")]
    Tbl006,
    #[serde(rename = "LNK-001")]
    Lnk001,
    #[serde(rename = "LNK-002")]
    Lnk002,
    #[serde(rename = "LNK-003")]
    Lnk003,
    #[serde(rename = "LNK-004")]
    Lnk004,
    #[serde(rename = "LNK-005")]
    Lnk005,
    #[serde(rename = "CLR-001")]
    Clr001,
    #[serde(rename = "CLR-002")]
    Clr002,
    #[serde(rename = "TXT-001")]
    Txt001,
    #[serde(rename = "TXT-002")]
    Txt002,
    #[serde(rename = "TXT-003")]
    Txt003,
    #[serde(rename = "TXT-004")]
    Txt004,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Blocks export.
    Error,
    /// Reported, does not block export.
    Warning,
}

/// A fix that a later milestone can apply. The rule says which fix exists; the
/// fix code itself is not part of the pre-check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixId {
    SetTitle,
    SetDefaultLanguage,
    ApplyHeadingStyle,
    DeleteEmptyHeading,
    ApplyListStyle,
    SetAltText,
    MakeImageInline,
    MarkAnnotationsDecorative,
    DeleteEmptyParagraphs,
    MarkHeaderRow,
    DeleteEmptyRow,
    DisallowRowBreak,
    AlignLeft,
    UpdateFields,
    AddScreenTip,
    RemoveTocLinks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "kebab-case")]
pub enum FixKind {
    /// The tool can fix it after the user confirms.
    Auto(FixId),
    /// The tool can fix it after the user types the needed text.
    Ask(FixId),
    /// The user fixes it in Word.
    Manual,
}

impl RuleId {
    /// Every rule, in spec order.
    pub const ALL: [RuleId; 41] = [
        RuleId::Doc001,
        RuleId::Doc002,
        RuleId::Doc003,
        RuleId::Doc004,
        RuleId::Doc005,
        RuleId::Doc006,
        RuleId::Hdg001,
        RuleId::Hdg002,
        RuleId::Hdg003,
        RuleId::Hdg004,
        RuleId::Hdg006,
        RuleId::Img001,
        RuleId::Img002,
        RuleId::Img003,
        RuleId::Img004,
        RuleId::Img005,
        RuleId::Img006,
        RuleId::Img007,
        RuleId::Txb001,
        RuleId::Txb002,
        RuleId::Txb003,
        RuleId::Txb004,
        RuleId::Lst001,
        RuleId::Lst002,
        RuleId::Tbl001,
        RuleId::Tbl002,
        RuleId::Tbl003,
        RuleId::Tbl004,
        RuleId::Tbl005,
        RuleId::Tbl006,
        RuleId::Lnk001,
        RuleId::Lnk002,
        RuleId::Lnk003,
        RuleId::Lnk004,
        RuleId::Lnk005,
        RuleId::Clr001,
        RuleId::Clr002,
        RuleId::Txt001,
        RuleId::Txt002,
        RuleId::Txt003,
        RuleId::Txt004,
    ];

    /// The ID as written in the specs, for example `HDG-002`.
    pub fn as_str(self) -> &'static str {
        match self {
            RuleId::Doc001 => "DOC-001",
            RuleId::Doc002 => "DOC-002",
            RuleId::Doc003 => "DOC-003",
            RuleId::Doc004 => "DOC-004",
            RuleId::Doc005 => "DOC-005",
            RuleId::Doc006 => "DOC-006",
            RuleId::Hdg001 => "HDG-001",
            RuleId::Hdg002 => "HDG-002",
            RuleId::Hdg003 => "HDG-003",
            RuleId::Hdg004 => "HDG-004",
            RuleId::Hdg006 => "HDG-006",
            RuleId::Img001 => "IMG-001",
            RuleId::Img002 => "IMG-002",
            RuleId::Img003 => "IMG-003",
            RuleId::Img004 => "IMG-004",
            RuleId::Img005 => "IMG-005",
            RuleId::Img006 => "IMG-006",
            RuleId::Img007 => "IMG-007",
            RuleId::Txb001 => "TXB-001",
            RuleId::Txb002 => "TXB-002",
            RuleId::Txb003 => "TXB-003",
            RuleId::Txb004 => "TXB-004",
            RuleId::Lst001 => "LST-001",
            RuleId::Lst002 => "LST-002",
            RuleId::Tbl001 => "TBL-001",
            RuleId::Tbl002 => "TBL-002",
            RuleId::Tbl003 => "TBL-003",
            RuleId::Tbl004 => "TBL-004",
            RuleId::Tbl005 => "TBL-005",
            RuleId::Tbl006 => "TBL-006",
            RuleId::Lnk001 => "LNK-001",
            RuleId::Lnk002 => "LNK-002",
            RuleId::Lnk003 => "LNK-003",
            RuleId::Lnk004 => "LNK-004",
            RuleId::Lnk005 => "LNK-005",
            RuleId::Clr001 => "CLR-001",
            RuleId::Clr002 => "CLR-002",
            RuleId::Txt001 => "TXT-001",
            RuleId::Txt002 => "TXT-002",
            RuleId::Txt003 => "TXT-003",
            RuleId::Txt004 => "TXT-004",
        }
    }

    /// Severity from spec 03.
    pub fn severity(self) -> Severity {
        use RuleId::*;
        match self {
            Doc001 | Doc002 | Doc005 | Doc006 | Hdg001 | Hdg002 | Hdg004 | Lst001 | Lnk003
            | Lnk004 | Lnk005 | Clr001 => Severity::Error,
            Doc003 | Doc004 | Hdg003 | Hdg006 | Lst002 | Lnk001 | Lnk002 | Clr002 | Txt001
            | Txt002 | Txt003 | Txt004 => Severity::Warning,
            Img001 | Img007 | Txb001 | Tbl001 | Tbl002 => Severity::Error,
            Img002 | Img003 | Img004 | Img005 | Img006 | Txb002 | Txb003 | Txb004 | Tbl003
            | Tbl004 | Tbl005 | Tbl006 => Severity::Warning,
        }
    }

    /// Fix kind from spec 03.
    pub fn fix(self) -> FixKind {
        use RuleId::*;
        match self {
            Doc001 => FixKind::Ask(FixId::SetTitle),
            Doc002 => FixKind::Auto(FixId::SetDefaultLanguage),
            Hdg003 => FixKind::Auto(FixId::ApplyHeadingStyle),
            Hdg004 => FixKind::Auto(FixId::DeleteEmptyHeading),
            Lst001 => FixKind::Auto(FixId::ApplyListStyle),
            Img001 | Img002 => FixKind::Ask(FixId::SetAltText),
            Img004 => FixKind::Auto(FixId::MakeImageInline),
            Img007 => FixKind::Ask(FixId::MarkAnnotationsDecorative),
            Txb004 => FixKind::Auto(FixId::DeleteEmptyParagraphs),
            Tbl001 => FixKind::Auto(FixId::MarkHeaderRow),
            Tbl005 => FixKind::Auto(FixId::DeleteEmptyRow),
            Tbl006 => FixKind::Auto(FixId::DisallowRowBreak),
            Txt002 => FixKind::Auto(FixId::AlignLeft),
            Txt004 => FixKind::Auto(FixId::UpdateFields),
            Lnk004 => FixKind::Auto(FixId::AddScreenTip),
            Lnk005 => FixKind::Auto(FixId::RemoveTocLinks),
            Img003 | Img005 | Img006 | Txb001 | Txb002 | Txb003 | Tbl002 | Tbl003 | Tbl004 => {
                FixKind::Manual
            }
            Doc003 | Doc004 | Doc005 | Doc006 | Hdg001 | Hdg002 | Hdg006 | Lst002 | Lnk001
            | Lnk002 | Lnk003 | Clr001 | Clr002 | Txt001 | Txt003 => FixKind::Manual,
        }
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a problem is. Page numbers are not known without Word, so a
/// location is the nearest heading, the start of the paragraph and its index.
/// [`Location::place`] says the same in Word's own terms, for people. The
/// other fields are for tooling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// Where the problem is, in words that a Word user knows, for example
    /// `paragraph 5 "3. Close the file.", after the heading "Setup"`, or `In
    /// the page header of section 2 (all pages)`. The apps and the reports
    /// show this text.
    pub place: String,
    /// Text of the nearest heading above the paragraph (first 60 characters).
    pub heading: Option<String>,
    /// First 60 characters of the paragraph text.
    pub snippet: Option<String>,
    /// Index into the body paragraphs, counting from 0. The text report shows
    /// it counting from 1.
    pub paragraph_index: Option<usize>,
    /// The part of the .docx that holds the problem, for example
    /// `word/document.xml`. For tooling; people read [`Location::place`].
    pub part: String,
}

/// `paragraph 5 "text", after the heading "Heading"`.
fn paragraph_place(index: usize, snippet: &str, heading: Option<&str>) -> String {
    let mut place = if snippet.is_empty() {
        format!("paragraph {} (no text)", index + 1)
    } else {
        format!("paragraph {} \"{snippet}\"", index + 1)
    };
    if let Some(heading) = heading {
        place.push_str(&format!(", after the heading \"{heading}\""));
    }
    place
}

/// "section 2", "sections 2 and 3" or "sections 1, 2 and 3", for sections
/// counted from 0.
fn sections_text(sections: &[usize]) -> String {
    let numbers: Vec<String> = sections.iter().map(|s| (s + 1).to_string()).collect();
    match numbers.as_slice() {
        [one] => format!("section {one}"),
        [first @ .., last] => format!("sections {} and {last}", first.join(", ")),
        [] => "the document".to_string(),
    }
}

/// `In the page header of section 2 (all pages)`. A document with one section
/// does not say "of section 1". A part that no section uses has no place.
pub fn header_footer_place(doc: &Document, part: &str) -> String {
    let places = doc.header_footer_places(part);
    if places.is_empty() {
        let what = if part.contains("footer") {
            "footer"
        } else {
            "header"
        };
        return format!("In a page {what} that no section of the document uses");
    }
    let single = doc.sections.len() <= 1;
    let phrases: Vec<String> = places
        .iter()
        .map(|place| {
            let what = if place.footer { "footer" } else { "header" };
            let of = if single {
                String::new()
            } else {
                format!(" of {}", sections_text(&place.sections))
            };
            format!("the page {what}{of} ({})", place.kind.pages())
        })
        .collect();
    format!("In {}", phrases.join(" and "))
}

impl Location {
    /// A problem with the document as a whole, found in `part`.
    pub fn document(part: &str) -> Location {
        let place = match part {
            CORE_PART => "The document's properties (File > Info)",
            SETTINGS_PART => "Word's template setting (Developer > Document Template)",
            STYLES_PART => "The whole document (its default language)",
            DOCUMENT_PART => "The whole document",
            _ => "The whole document",
        };
        Location {
            place: place.to_string(),
            heading: None,
            snippet: None,
            paragraph_index: None,
            part: part.to_string(),
        }
    }

    /// A problem in one body paragraph.
    pub fn paragraph(doc: &Document, paragraph: &Paragraph) -> Location {
        let heading = doc.heading_above(paragraph.index).map(|h| snippet(&h.text));
        let snippet = snippet(&paragraph.text);
        Location {
            place: paragraph_place(paragraph.index, &snippet, heading.as_deref()),
            heading,
            snippet: Some(snippet),
            paragraph_index: Some(paragraph.index),
            part: DOCUMENT_PART.to_string(),
        }
    }

    /// The same paragraph, but with other text as the snippet (the text in a
    /// text box that the paragraph holds, say).
    pub fn with_snippet(mut self, text: &str) -> Location {
        let shown = snippet(text);
        if let Some(index) = self.paragraph_index {
            self.place = paragraph_place(index, &shown, self.heading.as_deref());
        }
        self.snippet = Some(shown);
        self
    }

    /// A problem in a style, found through the first paragraph that uses it.
    pub fn style(doc: &Document, paragraph: &Paragraph) -> Location {
        let name = paragraph
            .style_name
            .as_deref()
            .or(paragraph.style_id.as_deref())
            .unwrap_or("Normal");
        let at = Location::paragraph(doc, paragraph);
        Location {
            place: format!(
                "The style \"{name}\" (Home > Styles), first used in {}",
                at.place
            ),
            part: STYLES_PART.to_string(),
            ..at
        }
    }

    /// A problem in a header or footer part. `text` is the text that is wrong.
    pub fn header_footer(doc: &Document, part: &str, text: &str) -> Location {
        Location {
            place: header_footer_place(doc, part),
            snippet: Some(snippet(text)),
            ..Location::document(part)
        }
    }

    /// A problem in one table (counting from 1 in the order of the document).
    pub fn table(
        doc: &Document,
        number: usize,
        first_cell: &str,
        paragraph: Option<usize>,
    ) -> Location {
        let heading = paragraph
            .and_then(|i| doc.heading_above(i))
            .map(|h| snippet(&h.text));
        let first = snippet(first_cell);
        let under = heading
            .as_deref()
            .map(|h| format!(" under the heading \"{h}\""))
            .unwrap_or_default();
        let cell = if first.is_empty() {
            String::new()
        } else {
            format!(", first cell \"{first}\"")
        };
        Location {
            place: format!("The table{under} (table {number}{cell})"),
            heading,
            snippet: Some(first),
            paragraph_index: paragraph,
            part: DOCUMENT_PART.to_string(),
        }
    }
}

/// The first [`SNIPPET_CHARS`] characters of the text, with surrounding spaces
/// removed and tabs and line breaks shown as spaces.
pub fn snippet(text: &str) -> String {
    text.trim()
        .chars()
        .take(SNIPPET_CHARS)
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: RuleId,
    pub severity: Severity,
    /// What is wrong, in plain language.
    pub message: String,
    /// What the user does in Word to fix it.
    pub how_to_fix: String,
    pub location: Location,
    pub fix: FixKind,
    /// Text the tool proposes, for example a title taken from the first
    /// heading. It is only a suggestion. The tool never applies it by itself.
    pub suggestion: Option<String>,
}

impl Finding {
    /// Severity and fix kind come from the rule ID.
    pub fn new(
        rule: RuleId,
        message: impl Into<String>,
        how_to_fix: impl Into<String>,
        location: Location,
    ) -> Finding {
        Finding {
            rule_id: rule,
            severity: rule.severity(),
            message: message.into(),
            how_to_fix: how_to_fix.into(),
            location,
            fix: rule.fix(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Finding {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_keeps_exactly_60_characters() {
        let exact = "a".repeat(60);
        assert_eq!(snippet(&exact), exact);
        assert_eq!(snippet(&"a".repeat(61)), exact);
        assert_eq!(snippet(&"a".repeat(59)).len(), 59);
    }

    #[test]
    fn snippet_counts_characters_not_bytes() {
        let text = "é".repeat(70);
        assert_eq!(snippet(&text).chars().count(), 60);
    }

    #[test]
    fn snippet_trims_and_flattens_whitespace() {
        assert_eq!(snippet("  Step one\tdo it\nnow  "), "Step one do it now");
        assert_eq!(snippet(""), "");
    }

    #[test]
    fn json_shape_of_fix_kinds() {
        let auto = serde_json::to_value(FixKind::Auto(FixId::ApplyListStyle)).unwrap();
        assert_eq!(
            auto,
            serde_json::json!({"kind": "auto", "id": "apply-list-style"})
        );
        let ask = serde_json::to_value(FixKind::Ask(FixId::SetTitle)).unwrap();
        assert_eq!(ask, serde_json::json!({"kind": "ask", "id": "set-title"}));
        let manual = serde_json::to_value(FixKind::Manual).unwrap();
        assert_eq!(manual, serde_json::json!({"kind": "manual"}));
    }

    #[test]
    fn rule_ids_serialise_as_in_the_specs() {
        assert_eq!(serde_json::to_value(RuleId::Hdg003).unwrap(), "HDG-003");
        assert_eq!(RuleId::Lst002.to_string(), "LST-002");
        assert_eq!(serde_json::to_value(Severity::Warning).unwrap(), "warning");
    }

    /// The tables in spec 03 for the DOC, HDG, IMG, TXB, LST and TBL rules. The
    /// rest are in `tests/rules_links.rs`.
    #[test]
    fn severities_and_fixes_match_spec_03() {
        use FixId::*;
        use RuleId::*;
        let expected = [
            (Doc001, Severity::Error, FixKind::Ask(SetTitle)),
            (Doc002, Severity::Error, FixKind::Auto(SetDefaultLanguage)),
            (Doc003, Severity::Warning, FixKind::Manual),
            (Doc004, Severity::Warning, FixKind::Manual),
            (Doc005, Severity::Error, FixKind::Manual),
            (Doc006, Severity::Error, FixKind::Manual),
            (Hdg001, Severity::Error, FixKind::Manual),
            (Hdg002, Severity::Error, FixKind::Manual),
            (Hdg003, Severity::Warning, FixKind::Auto(ApplyHeadingStyle)),
            (Hdg004, Severity::Error, FixKind::Auto(DeleteEmptyHeading)),
            (Hdg006, Severity::Warning, FixKind::Manual),
            (Lst001, Severity::Error, FixKind::Auto(ApplyListStyle)),
            (Lst002, Severity::Warning, FixKind::Manual),
            (Img001, Severity::Error, FixKind::Ask(SetAltText)),
            (Img002, Severity::Warning, FixKind::Ask(SetAltText)),
            (Img003, Severity::Warning, FixKind::Manual),
            (Img004, Severity::Warning, FixKind::Auto(MakeImageInline)),
            (Img005, Severity::Warning, FixKind::Manual),
            (Img006, Severity::Warning, FixKind::Manual),
            (
                Img007,
                Severity::Error,
                FixKind::Ask(MarkAnnotationsDecorative),
            ),
            (Txb001, Severity::Error, FixKind::Manual),
            (Txb002, Severity::Warning, FixKind::Manual),
            (Txb003, Severity::Warning, FixKind::Manual),
            (
                Txb004,
                Severity::Warning,
                FixKind::Auto(DeleteEmptyParagraphs),
            ),
            (Tbl001, Severity::Error, FixKind::Auto(MarkHeaderRow)),
            (Tbl002, Severity::Error, FixKind::Manual),
            (Tbl003, Severity::Warning, FixKind::Manual),
            (Tbl004, Severity::Warning, FixKind::Manual),
            (Tbl005, Severity::Warning, FixKind::Auto(DeleteEmptyRow)),
            (Tbl006, Severity::Warning, FixKind::Auto(DisallowRowBreak)),
        ];
        for (rule, severity, fix) in expected {
            assert_eq!(rule.severity(), severity, "{rule}");
            assert_eq!(rule.fix(), fix, "{rule}");
        }
    }
}
