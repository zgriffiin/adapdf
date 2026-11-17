//! The pre-check rules from spec 03, one function per rule, one file per
//! group. A rule reads the [`Document`] and returns its findings.

use crate::findings::{Finding, RuleId};
use crate::fonts::FontLookup;
use crate::model::Document;

pub mod colour;
pub mod document;
pub mod headings;
pub mod images;
pub mod links;
pub mod lists;
pub mod tables;
pub mod text;
pub mod textboxes;

pub type Rule = fn(&Document) -> Vec<Finding>;

/// Every rule that needs only the document, in spec order. DOC-006 also
/// needs a [`FontLookup`], so [`check`] runs it separately.
pub const RULES: &[(RuleId, Rule)] = &[
    (RuleId::Doc001, document::doc_001),
    (RuleId::Doc002, document::doc_002),
    (RuleId::Doc003, document::doc_003),
    (RuleId::Doc004, document::doc_004),
    (RuleId::Doc005, document::doc_005),
    (RuleId::Hdg001, headings::hdg_001),
    (RuleId::Hdg002, headings::hdg_002),
    (RuleId::Hdg003, headings::hdg_003),
    (RuleId::Hdg004, headings::hdg_004),
    (RuleId::Hdg006, headings::hdg_006),
    (RuleId::Img001, images::img_001),
    (RuleId::Img002, images::img_002),
    (RuleId::Img003, images::img_003),
    (RuleId::Img004, images::img_004),
    (RuleId::Img005, images::img_005),
    (RuleId::Img006, images::img_006),
    (RuleId::Img007, images::img_007),
    (RuleId::Txb001, textboxes::txb_001),
    (RuleId::Txb002, textboxes::txb_002),
    (RuleId::Txb003, textboxes::txb_003),
    (RuleId::Txb004, textboxes::txb_004),
    (RuleId::Lst001, lists::lst_001),
    (RuleId::Lst002, lists::lst_002),
    (RuleId::Tbl001, tables::tbl_001),
    (RuleId::Tbl002, tables::tbl_002),
    (RuleId::Tbl003, tables::tbl_003),
    (RuleId::Tbl004, tables::tbl_004),
    (RuleId::Tbl005, tables::tbl_005),
    (RuleId::Tbl006, tables::tbl_006),
    (RuleId::Lnk001, links::lnk_001),
    (RuleId::Lnk002, links::lnk_002),
    (RuleId::Lnk003, links::lnk_003),
    (RuleId::Lnk004, links::lnk_004),
    (RuleId::Lnk005, links::lnk_005),
    (RuleId::Clr001, colour::clr_001),
    (RuleId::Clr002, colour::clr_002),
    (RuleId::Txt001, text::txt_001),
    (RuleId::Txt002, text::txt_002),
    (RuleId::Txt003, text::txt_003),
    (RuleId::Txt004, text::txt_004),
];

/// Runs every rule. Findings are in rule order, and in document order within
/// a rule.
pub fn check(doc: &Document, fonts: &dyn FontLookup) -> Vec<Finding> {
    let mut findings: Vec<Finding> = RULES.iter().flat_map(|(_, rule)| rule(doc)).collect();
    findings.extend(document::doc_006(doc, fonts));
    findings.sort_by_key(|f| f.rule_id); // stable: keeps document order
    findings
}
