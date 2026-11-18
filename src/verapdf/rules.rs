//! The table that links veraPDF PDF/UA-1 rules to the likely cause in Word, the
//! fix, and the pre-check rule of stage 1 that would have caught it.
//!
//! A rule is named by its clause in ISO 14289-1 and its test number, for
//! example `7.3` and `1` (a `Figure` tag with no alternative text). A rule that
//! is not in the table is still reported, with veraPDF's own description.

use crate::findings::RuleId;

pub struct Entry {
    /// The clause of ISO 14289-1.
    pub clause: &'static str,
    pub test_number: u32,
    /// What is wrong, in plain language.
    pub message: &'static str,
    /// The likely cause in Word, and what to do.
    pub how_to_fix: &'static str,
    /// The pre-check rule that finds the same problem in the .docx.
    pub pre_check: Option<RuleId>,
}

const EXPORT_AGAIN: &str =
    "Export the PDF again from Word with \"Document structure tags for accessibility\" turned on.";

const TABLE_FIX: &str = "Simplify the table: no merged or split cells, and headings only in the first row and/or the first column (the pre-check rule TBL-003 finds merged and split cells). Adapdf adds the scope of the header cells itself after the export, but only for a table like that. Then export again.";

const FONT_FIX: &str = "Use a font that Word can embed (a TrueType or OpenType font that allows embedding), \
     or change the font of that text. Then export again.";

const LINK_FIX: &str = "Give every link a ScreenTip: in Word, right-click the link, choose Edit Hyperlink, choose ScreenTip, and type a short description of where the link goes (the pre-check rule LNK-004 finds these links, and `adapdf fix` can copy the link text into the ScreenTip). Word writes the ScreenTip into the PDF as the description of the link. Word makes the links of a table of contents again each time it updates the table, so a ScreenTip typed there is lost: in References, Table of Contents, Custom Table of Contents, clear \"Use hyperlinks instead of page numbers\" (the pre-check rule LNK-005 finds this, and `adapdf fix` can do it). Then export again.";

/// The mapped rules, in clause order. Every entry has a test
/// (`mapped_rules_become_findings_with_their_message` in `tests/verapdf_parse.rs`).
pub const TABLE: &[Entry] = &[
    Entry {
        clause: "5",
        test_number: 1,
        message: "The PDF does not say that it follows PDF/UA: the XMP metadata has no PDF/UA identification (pdfuaid:part).",
        how_to_fix: "The tool adds this value in the metadata step after the export. If it is reported here, that step did not run or did not succeed.",
        pre_check: None,
    },
    Entry {
        clause: "5",
        test_number: 2,
        message: "The PDF says that it follows a PDF/UA part other than part 1 (the value pdfuaid:part is wrong).",
        how_to_fix: "Check how the PDF was made. For PDF/UA-1 the value must be 1. The tool does not change a value that is already there.",
        pre_check: None,
    },
    Entry {
        clause: "6.2",
        test_number: 1,
        message: "The PDF is not marked as a tagged PDF (/MarkInfo /Marked true is missing or false).",
        how_to_fix: EXPORT_AGAIN,
        pre_check: None,
    },
    Entry {
        clause: "7.1",
        test_number: 3,
        message: "Some content in the PDF is not tagged and is not marked as decoration. A screen reader may skip it or read it in the wrong place.",
        how_to_fix: "The likely cause is text in a text box or shape, or a floating picture or shape. Move the text into the body text, make pictures \"In line with text\", and export again.",
        pre_check: Some(RuleId::Txb001),
    },
    Entry {
        clause: "7.1",
        test_number: 4,
        message: "The PDF says that its tags may be wrong (the Suspects entry is true).",
        how_to_fix: EXPORT_AGAIN,
        pre_check: None,
    },
    Entry {
        clause: "7.1",
        test_number: 8,
        message: "The PDF has no metadata stream (XMP).",
        how_to_fix: "The tool adds the metadata stream in the metadata step after the export. If it is reported here, that step did not run or did not succeed.",
        pre_check: None,
    },
    Entry {
        clause: "7.1",
        test_number: 9,
        message: "The PDF metadata has no document title (dc:title).",
        how_to_fix: "Set the title in Word (File, Info, Title). The tool copies it into the PDF after the export.",
        pre_check: Some(RuleId::Doc001),
    },
    Entry {
        clause: "7.1",
        test_number: 10,
        message: "The PDF does not ask viewers to show the document title (DisplayDocTitle is not true).",
        how_to_fix: "The tool sets this value in the metadata step after the export. If it is reported here, that step did not run or did not succeed.",
        pre_check: None,
    },
    Entry {
        clause: "7.1",
        test_number: 11,
        message: "The PDF has no structure tree, so it has no tags.",
        how_to_fix: EXPORT_AGAIN,
        pre_check: None,
    },
    Entry {
        clause: "7.2",
        test_number: 17,
        message: "A list item (LI) is not inside a list (L). The list is not built as a real list.",
        how_to_fix: "Use Word's list buttons or the List Bullet and List Number styles, not typed numbers or bullets. Then export again.",
        pre_check: Some(RuleId::Lst001),
    },
    Entry {
        clause: "7.2",
        test_number: 18,
        message: "A list body (LBody) is not inside a list item (LI). The list is not built as a real list.",
        how_to_fix: "Use Word's list buttons or the List Bullet and List Number styles, not typed numbers or bullets. Then export again.",
        pre_check: Some(RuleId::Lst001),
    },
    Entry {
        clause: "7.2",
        test_number: 19,
        message: "A list (L) contains something other than list items. The list is not built as a real list.",
        how_to_fix: "Use Word's list buttons or the List Bullet and List Number styles, not typed numbers or bullets. Check that one list does not mix typed and automatic numbers. Then export again.",
        pre_check: Some(RuleId::Lst002),
    },
    Entry {
        clause: "7.2",
        test_number: 20,
        message: "A list item (LI) contains something other than a label and a body. The list is not built as a real list.",
        how_to_fix: "Use Word's list buttons or the List Bullet and List Number styles, not typed numbers or bullets. Then export again.",
        pre_check: Some(RuleId::Lst001),
    },
    Entry {
        clause: "7.2",
        test_number: 22,
        message: "The language of some alternative text cannot be determined.",
        how_to_fix: "Set the default language of the document in Word (Review, Language, Set Proofing Language). Then export again.",
        pre_check: Some(RuleId::Doc002),
    },
    Entry {
        clause: "7.2",
        test_number: 29,
        message: "The document language in the PDF is not a valid language code.",
        how_to_fix: "Set the default language of the document in Word (Review, Language, Set Proofing Language) to a normal language such as English (United States). Then export again.",
        pre_check: Some(RuleId::Doc002),
    },
    Entry {
        clause: "7.2",
        test_number: 34,
        message: "The language of the text in the PDF cannot be determined, because the PDF has no document language.",
        how_to_fix: "Set the default language of the document in Word (Review, Language, Set Proofing Language). The tool also copies it into the PDF after the export.",
        pre_check: Some(RuleId::Doc002),
    },
    Entry {
        clause: "7.2",
        test_number: 41,
        message: "The columns of a table do not have the same number of rows. The table has merged or split cells that make its structure unclear.",
        how_to_fix: "Simplify the table: avoid merged and split cells. Then export again.",
        pre_check: Some(RuleId::Tbl003),
    },
    Entry {
        clause: "7.2",
        test_number: 42,
        message: "The rows of a table do not have the same number of columns. The table has merged or split cells that make its structure unclear.",
        how_to_fix: "Simplify the table: avoid merged and split cells. Then export again.",
        pre_check: Some(RuleId::Tbl003),
    },
    Entry {
        clause: "7.2",
        test_number: 43,
        message: "The rows of a table do not have the same number of columns. The table has merged or split cells that make its structure unclear.",
        how_to_fix: "Simplify the table: avoid merged and split cells. Then export again.",
        pre_check: Some(RuleId::Tbl003),
    },
    Entry {
        clause: "7.3",
        test_number: 1,
        message: "A figure (picture, chart, shape or drawing) has no alternative text.",
        how_to_fix: "In Word, right-click the picture, choose Edit Alt Text, and describe it, or mark it as decorative. Then export again.",
        pre_check: Some(RuleId::Img001),
    },
    Entry {
        clause: "7.4.2",
        test_number: 1,
        message: "The headings do not follow a clear order: a heading level is skipped, or the first heading is not level 1.",
        how_to_fix: "Use heading styles in order (Heading 1, then Heading 2, and so on) without skipping a level. Then export again.",
        pre_check: Some(RuleId::Hdg002),
    },
    // Word writes a `TH` for the first row and the first column of a table, from
    // `w:tblLook`, and never writes a `/Scope` for it (S5-14). The tool adds the
    // scope after the export (stage 4). This rule is still reported for a table
    // where it could not: a table that is not regular, with a merged or split
    // cell or a heading in the middle.
    Entry {
        clause: "7.5",
        test_number: 1,
        message: "A table cell has no header that a screen reader can find. The table is not regular (merged or split cells, or a heading in the middle of the table), so Adapdf could not give its header cells a scope.",
        how_to_fix: TABLE_FIX,
        pre_check: Some(RuleId::Tbl003),
    },
    Entry {
        clause: "7.5",
        test_number: 2,
        message: "A table cell names header cells (Headers) that are not in the PDF, so a screen reader cannot tell which headings belong to it.",
        how_to_fix: TABLE_FIX,
        pre_check: Some(RuleId::Tbl003),
    },
    // Word writes the ScreenTip of a link (`w:tooltip`) as the `/Contents` of the
    // link annotation, and writes no `/Contents` for a link without one. A
    // Word-made table of contents has links with no ScreenTip. Seen with Word
    // 16.0.20527 on every pass fixture that has a link (S5-10). LNK-004 finds a
    // link without a ScreenTip and LNK-005 finds a table of contents made of links;
    // both fail these two rules, and the entry names LNK-004 (a table can name
    // only one rule).
    Entry {
        clause: "7.18.1",
        test_number: 2,
        message: "A link has no description (Word's ScreenTip), so a screen reader has nothing to read for it except the address.",
        how_to_fix: LINK_FIX,
        pre_check: Some(RuleId::Lnk004),
    },
    Entry {
        clause: "7.18.5",
        test_number: 2,
        message: "A link has no description for assistive technology (Word's ScreenTip).",
        how_to_fix: LINK_FIX,
        pre_check: Some(RuleId::Lnk004),
    },
    Entry {
        clause: "7.21.4.1",
        test_number: 1,
        message: "A font is not embedded in the PDF. The PDF may look wrong on another computer.",
        how_to_fix: FONT_FIX,
        pre_check: Some(RuleId::Doc006),
    },
    Entry {
        clause: "7.21.4.1",
        test_number: 2,
        message: "An embedded font is missing some of the characters that the PDF uses.",
        how_to_fix: FONT_FIX,
        pre_check: Some(RuleId::Doc006),
    },
    Entry {
        clause: "7.21.7",
        test_number: 1,
        message: "A font has no map from its characters to Unicode text, so a screen reader may read wrong letters.",
        how_to_fix: FONT_FIX,
        pre_check: Some(RuleId::Doc006),
    },
    Entry {
        clause: "7.21.8",
        test_number: 1,
        message: "The PDF uses a character that its font does not have (the .notdef glyph).",
        how_to_fix: "Find the character that shows as a box or a missing glyph, and use a font that has it. Then export again.",
        pre_check: Some(RuleId::Doc006),
    },
];

/// The table entry for a rule, if there is one.
pub fn lookup(clause: &str, test_number: u32) -> Option<&'static Entry> {
    TABLE
        .iter()
        .find(|e| e.clause == clause && e.test_number == test_number)
}
