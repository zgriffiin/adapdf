//! TBL-001 to TBL-006.

mod common;

use adapdf::findings::{FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{DocxBuilder, Table, clean};
use adapdf::rules::tables::LAYOUT_EMPTY_CELLS_PERCENT;
use common::{assert_location, fixture_findings, of, rule_findings, the_failing_finding};

fn count(builder: &DocxBuilder, rule: RuleId) -> usize {
    rule_findings(builder, rule).len()
}

fn good() -> Table {
    Table::new(&[&["Setting", "Value"], &["Paper", "A4"], &["Colour", "Mono"]])
}

fn doc(table: Table) -> DocxBuilder {
    clean().table(table)
}

// ---- TBL-001 -------------------------------------------------------------

#[test]
fn tbl_001_fails_without_a_header_row() {
    let f = the_failing_finding(RuleId::Tbl001);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Auto(FixId::MarkHeaderRow));
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Setting"),
        Some("Test guide"),
    );
}

#[test]
fn tbl_001_passes_with_a_header_row() {
    assert!(of(&fixture_findings(RuleId::Tbl001, false), RuleId::Tbl001).is_empty());
}

#[test]
fn tbl_001_only_the_first_row_counts() {
    let second = Table::new(&[&["A", "B"], &["C", "D"]])
        .no_header_row()
        .raw_row_properties(1, "<w:cantSplit/><w:tblHeader/>");
    assert_eq!(count(&doc(second), RuleId::Tbl001), 1);
}

#[test]
fn tbl_001_a_header_flag_that_is_off_does_not_count() {
    let off = Table::new(&[&["A", "B"], &["C", "D"]])
        .raw_row_properties(0, r#"<w:cantSplit/><w:tblHeader w:val="0"/>"#);
    assert_eq!(count(&doc(off), RuleId::Tbl001), 1);
    let on = Table::new(&[&["A", "B"], &["C", "D"]])
        .raw_row_properties(0, r#"<w:cantSplit/><w:tblHeader w:val="true"/>"#);
    assert_eq!(count(&doc(on), RuleId::Tbl001), 0);
}

#[test]
fn tbl_001_one_finding_for_each_table() {
    let builder = clean()
        .table(good().no_header_row())
        .heading(2, "Second")
        .table(good().no_header_row());
    let found = rule_findings(&builder, RuleId::Tbl001);
    assert_eq!(found.len(), 2, "two failing tables, two findings");
    assert_eq!(found[0].location.heading.as_deref(), Some("Test guide"));
    assert_eq!(found[1].location.heading.as_deref(), Some("Second"));
    // A passing table in between is not reported.
    let builder = clean()
        .table(good().no_header_row())
        .table(good())
        .table(good().no_header_row());
    assert_eq!(count(&builder, RuleId::Tbl001), 2);
}

#[test]
fn tbl_001_a_table_before_any_heading_has_no_heading_in_its_location() {
    let builder = DocxBuilder::new().table(good().no_header_row());
    let found = rule_findings(&builder, RuleId::Tbl001);
    assert_eq!(found[0].location.heading, None);
    assert_eq!(found[0].location.paragraph_index, Some(0));
}

// ---- TBL-002 -------------------------------------------------------------

#[test]
fn tbl_002_fails_on_a_table_inside_a_table() {
    let f = the_failing_finding(RuleId::Tbl002);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    // The inner table: 3 paragraphs of the outer table come before it.
    assert_location(
        &f,
        "word/document.xml",
        Some(2 + 3),
        Some("Setting"),
        Some("Test guide"),
    );
}

#[test]
fn tbl_002_passes_for_a_plain_table() {
    assert!(of(&fixture_findings(RuleId::Tbl002, false), RuleId::Tbl002).is_empty());
}

#[test]
fn tbl_002_reports_each_nested_table_and_not_the_outer_one() {
    let inner = Table::new(&[&["x", "y"], &["z", "w"]]);
    let builder = doc(good().nested(1, 0, &inner).nested(2, 0, &inner));
    assert_eq!(count(&builder, RuleId::Tbl002), 2);
}

// ---- TBL-003 -------------------------------------------------------------

#[test]
fn tbl_003_fails_on_merged_cells() {
    let f = the_failing_finding(RuleId::Tbl003);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Settings"),
        Some("Test guide"),
    );
}

#[test]
fn tbl_003_passes_without_merged_cells() {
    assert!(of(&fixture_findings(RuleId::Tbl003, false), RuleId::Tbl003).is_empty());
}

#[test]
fn tbl_003_vertical_merges_count_and_a_span_of_1_does_not() {
    let down = good().merge_down(1, 0, true).merge_down(2, 0, false);
    assert_eq!(count(&doc(down), RuleId::Tbl003), 1);
    let span_1 = good().span(1, 0, 1);
    assert_eq!(count(&doc(span_1), RuleId::Tbl003), 0);
}

#[test]
fn tbl_003_is_one_finding_for_each_table() {
    let builder = doc(good()
        .span(0, 0, 2)
        .merge_down(1, 0, true)
        .merge_down(2, 0, false));
    assert_eq!(count(&builder, RuleId::Tbl003), 1);
}

// ---- TBL-004 -------------------------------------------------------------

#[test]
fn tbl_004_fails_on_a_table_with_one_row() {
    let f = the_failing_finding(RuleId::Tbl004);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Setting"),
        Some("Test guide"),
    );
    assert!(f.message.contains("one row"), "{}", f.message);
}

#[test]
fn tbl_004_passes_for_a_data_table() {
    assert!(of(&fixture_findings(RuleId::Tbl004, false), RuleId::Tbl004).is_empty());
}

#[test]
fn tbl_004_one_column() {
    let one = Table::new(&[&["Steps"], &["Plug it in"], &["Switch it on"]]);
    let found = rule_findings(&doc(one), RuleId::Tbl004);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("one column"),
        "{}",
        found[0].message
    );
}

#[test]
fn tbl_004_threshold_is_more_than_half_of_the_cells_without_text() {
    assert_eq!(LAYOUT_EMPTY_CELLS_PERCENT, 50);
    // 6 cells: 3 empty is exactly half, 4 empty is most.
    let half = Table::new(&[&["A", "", ""], &["B", "C", ""]]);
    assert_eq!(count(&doc(half), RuleId::Tbl004), 0);
    let most = Table::new(&[&["A", "", ""], &["B", "", ""]]);
    let found = rule_findings(&doc(most), RuleId::Tbl004);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("most cells"),
        "{}",
        found[0].message
    );
}

#[test]
fn tbl_004_spaces_are_no_text() {
    let spaces = Table::new(&[&["A", "  ", "  "], &["B", "  ", " "]]);
    assert_eq!(count(&doc(spaces), RuleId::Tbl004), 1);
}

#[test]
fn tbl_004_cells_that_continue_a_merge_are_not_cells() {
    // Real cells: "A", "", "", "x". Two of four is half. With the two
    // continuation cells it would be four of six.
    let table = Table::new(&[&["A", ""], &["", ""], &["", "x"]])
        .merge_down(0, 0, true)
        .merge_down(1, 0, false)
        .merge_down(2, 0, false);
    assert_eq!(count(&doc(table), RuleId::Tbl004), 0);
}

// ---- TBL-005 -------------------------------------------------------------

#[test]
fn tbl_005_fails_on_an_empty_row() {
    let f = the_failing_finding(RuleId::Tbl005);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Auto(FixId::DeleteEmptyRow));
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Setting"),
        Some("Test guide"),
    );
    assert!(f.message.contains("row 3"), "{}", f.message);
}

#[test]
fn tbl_005_passes_when_every_row_has_text() {
    assert!(of(&fixture_findings(RuleId::Tbl005, false), RuleId::Tbl005).is_empty());
}

#[test]
fn tbl_005_is_one_finding_for_each_table_with_the_row_numbers() {
    let table = Table::new(&[&["A", "B"], &["", ""], &["C", "D"], &["  ", ""]]);
    let found = rule_findings(&doc(table), RuleId::Tbl005);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("rows 2, 4"),
        "{}",
        found[0].message
    );
}

#[test]
fn tbl_005_a_row_with_any_text_or_a_picture_is_not_empty() {
    let partly = Table::new(&[&["A", "B"], &["", "x"], &["C", "D"]]);
    assert_eq!(count(&doc(partly), RuleId::Tbl005), 0);
    let picture = r#"<w:p><w:r><w:drawing><wp:inline><wp:docPr id="1" name="P"/></wp:inline></w:drawing></w:r></w:p>"#;
    let with_picture = Table::new(&[&["A", "B"], &["", ""], &["C", "D"]]).raw_cell(1, 0, picture);
    assert_eq!(count(&doc(with_picture), RuleId::Tbl005), 0);
}

#[test]
fn tbl_005_a_row_in_a_vertical_merge_is_not_reported() {
    let table = Table::new(&[&["A", "B"], &["", ""], &["C", "D"]])
        .merge_down(0, 0, true)
        .merge_down(1, 0, false);
    assert_eq!(count(&doc(table), RuleId::Tbl005), 0);
}

// ---- TBL-006 -------------------------------------------------------------

#[test]
fn tbl_006_fails_when_rows_can_break_across_pages() {
    let f = the_failing_finding(RuleId::Tbl006);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Auto(FixId::DisallowRowBreak));
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Setting"),
        Some("Test guide"),
    );
    assert!(f.message.contains("3 of 3 rows"), "{}", f.message);
}

#[test]
fn tbl_006_passes_when_no_row_can_break() {
    assert!(of(&fixture_findings(RuleId::Tbl006, false), RuleId::Tbl006).is_empty());
}

#[test]
fn tbl_006_is_one_finding_for_each_table_even_if_one_row_can_break() {
    let found = rule_findings(&doc(good().row_can_split(1)), RuleId::Tbl006);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("1 of 3 rows"),
        "{}",
        found[0].message
    );
    assert_eq!(count(&doc(good().rows_can_split()), RuleId::Tbl006), 1);
}

#[test]
fn tbl_006_a_cant_split_flag_that_is_off_does_not_count() {
    let off = Table::new(&[&["A", "B"], &["C", "D"]])
        .raw_row_properties(0, r#"<w:cantSplit w:val="0"/><w:tblHeader/>"#);
    assert_eq!(count(&doc(off), RuleId::Tbl006), 1);
}

// ---- All table rules -----------------------------------------------------

#[test]
fn a_good_table_has_no_findings_from_any_rule() {
    let findings = common::check(&doc(good()));
    assert!(findings.is_empty(), "{findings:#?}");
}
