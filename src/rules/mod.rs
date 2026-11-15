//! The pre-check rules from spec 03, one function per rule, one file per
//! group. A rule reads the [`Document`] and returns its findings.

use crate::findings::{Finding, RuleId};
use crate::fonts::FontLookup;
use crate::model::Document;

pub mod colour;
pub mod document;
pub mod headings;
pub mod links;
pub mod lists;
pub mod text;

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
    (RuleId::Hdg005, headings::hdg_005),
    (RuleId::Hdg006, headings::hdg_006),
    (RuleId::Lst001, lists::lst_001),
    (RuleId::Lst002, lists::lst_002),
    (RuleId::Lnk001, links::lnk_001),
    (RuleId::Lnk002, links::lnk_002),
    (RuleId::Lnk003, links::lnk_003),
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
