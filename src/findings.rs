//! What a check reports: [`Finding`] and the types it is made of.
//!
//! Messages are for staff who are not accessibility specialists. They say
//! what is wrong in plain words and how to fix it in Word.

use serde::Serialize;

use crate::docx::DOCUMENT_PART;
use crate::model::{Document, Paragraph};

/// Number of characters of paragraph text kept in a [`Location`] snippet.
pub const SNIPPET_CHARS: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
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
    #[serde(rename = "HDG-005")]
    Hdg005,
    #[serde(rename = "HDG-006")]
    Hdg006,
    #[serde(rename = "LST-001")]
    Lst001,
    #[serde(rename = "LST-002")]
    Lst002,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Blocks export.
    Error,
    /// Reported, does not block export.
    Warning,
}

/// A fix that a later milestone can apply. The rule says which fix exists; the
/// fix code itself is not part of the pre-check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixId {
    SetTitle,
    SetDefaultLanguage,
    ApplyHeadingStyle,
    DeleteEmptyHeading,
    ApplyListStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
    pub const ALL: [RuleId; 14] = [
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
        RuleId::Hdg005,
        RuleId::Hdg006,
        RuleId::Lst001,
        RuleId::Lst002,
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
            RuleId::Hdg005 => "HDG-005",
            RuleId::Hdg006 => "HDG-006",
            RuleId::Lst001 => "LST-001",
            RuleId::Lst002 => "LST-002",
        }
    }

    /// Severity from spec 03.
    pub fn severity(self) -> Severity {
        use RuleId::*;
        match self {
            Doc001 | Doc002 | Doc005 | Doc006 | Hdg001 | Hdg002 | Hdg004 | Lst001 => {
                Severity::Error
            }
            Doc003 | Doc004 | Hdg003 | Hdg005 | Hdg006 | Lst002 => Severity::Warning,
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
            Doc003 | Doc004 | Doc005 | Doc006 | Hdg001 | Hdg002 | Hdg005 | Hdg006 | Lst002 => {
                FixKind::Manual
            }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    /// Text of the nearest heading above the paragraph (first 60 characters).
    pub heading: Option<String>,
    /// First 60 characters of the paragraph text.
    pub snippet: Option<String>,
    /// Index into the body paragraphs, counting from 0. The text report shows
    /// it counting from 1.
    pub paragraph_index: Option<usize>,
    /// The part of the .docx that holds the problem, for example
    /// `word/document.xml`.
    pub part: String,
}

impl Location {
    /// A problem with the document as a whole, found in `part`.
    pub fn document(part: &str) -> Location {
        Location {
            heading: None,
            snippet: None,
            paragraph_index: None,
            part: part.to_string(),
        }
    }

    /// A problem in one body paragraph.
    pub fn paragraph(doc: &Document, paragraph: &Paragraph) -> Location {
        Location {
            heading: doc.heading_above(paragraph.index).map(|h| snippet(&h.text)),
            snippet: Some(snippet(&paragraph.text)),
            paragraph_index: Some(paragraph.index),
            part: DOCUMENT_PART.to_string(),
        }
    }

    /// The same, but the problem is defined in another part (a style, say).
    pub fn paragraph_in(doc: &Document, paragraph: &Paragraph, part: &str) -> Location {
        Location {
            part: part.to_string(),
            ..Location::paragraph(doc, paragraph)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

    /// The table in spec 03 for the M1 rules.
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
            (Hdg005, Severity::Warning, FixKind::Manual),
            (Hdg006, Severity::Warning, FixKind::Manual),
            (Lst001, Severity::Error, FixKind::Auto(ApplyListStyle)),
            (Lst002, Severity::Warning, FixKind::Manual),
        ];
        for (rule, severity, fix) in expected {
            assert_eq!(rule.severity(), severity, "{rule}");
            assert_eq!(rule.fix(), fix, "{rule}");
        }
    }
}
