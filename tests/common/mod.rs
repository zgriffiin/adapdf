//! Helpers shared by the rule tests.
#![allow(dead_code)] // each test file uses a different part of this module

use adapdf::findings::{Finding, RuleId};
use adapdf::fixtures::{DocxBuilder, FixtureFonts, catalogue};

/// All findings for a document, with the fixture font lookup.
pub fn check(builder: &DocxBuilder) -> Vec<Finding> {
    adapdf::check_bytes(&builder.build(), &FixtureFonts).expect("the fixture is a valid .docx")
}

/// The findings of one rule.
pub fn of(findings: &[Finding], rule: RuleId) -> Vec<&Finding> {
    findings.iter().filter(|f| f.rule_id == rule).collect()
}

/// The findings of one rule for a document.
pub fn rule_findings(builder: &DocxBuilder, rule: RuleId) -> Vec<Finding> {
    check(builder)
        .into_iter()
        .filter(|f| f.rule_id == rule)
        .collect()
}

/// The passing (`fails = false`) or failing fixture of a rule, checked.
pub fn fixture_findings(rule: RuleId, fails: bool) -> Vec<Finding> {
    let fixture = catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .expect("the catalogue has this fixture");
    adapdf::check_bytes(&fixture.bytes, &FixtureFonts).expect("the fixture is a valid .docx")
}

/// The one finding of a failing fixture for its own rule.
pub fn the_failing_finding(rule: RuleId) -> Finding {
    let mut found: Vec<Finding> = fixture_findings(rule, true)
        .into_iter()
        .filter(|f| f.rule_id == rule)
        .collect();
    assert_eq!(found.len(), 1, "{rule}: {found:#?}");
    found.remove(0)
}
