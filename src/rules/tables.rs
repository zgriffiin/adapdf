//! Table rules TBL-001 to TBL-006.
//!
//! Each rule reports once for each table. The location of a finding is the
//! first paragraph in the table, the nearest heading above it, and the text
//! of the first cell as the snippet.

use crate::docx::DOCUMENT_PART;
use crate::findings::{Finding, Location, RuleId, snippet};
use crate::model::Document;
use crate::model::tables::{Table, VMerge};

/// TBL-004: a table is a probable layout table when more than this share of
/// its cells (in percent) have no text.
pub const LAYOUT_EMPTY_CELLS_PERCENT: usize = 50;

fn location(doc: &Document, table: &Table) -> Location {
    let first_cell = table
        .rows
        .first()
        .and_then(|r| r.cells.first())
        .map_or("", |c| c.text.as_str());
    Location {
        heading: table
            .first_paragraph
            .and_then(|i| doc.heading_above(i))
            .map(|h| snippet(&h.text)),
        snippet: Some(snippet(first_cell)),
        paragraph_index: table.first_paragraph,
        part: DOCUMENT_PART.to_string(),
    }
}

/// One finding for each table that `problem` gives a message for.
fn each_table(
    doc: &Document,
    rule: RuleId,
    how_to_fix: &str,
    problem: impl Fn(&Table) -> Option<String>,
) -> Vec<Finding> {
    doc.tables
        .iter()
        .filter_map(|t| Some((t, problem(t)?)))
        .map(|(t, message)| Finding::new(rule, message, how_to_fix, location(doc, t)))
        .collect()
}

/// TBL-001: the first row is not marked as a header row.
pub fn tbl_001(doc: &Document) -> Vec<Finding> {
    each_table(
        doc,
        RuleId::Tbl001,
        "Click in the first row, open Table Properties > Row, and select Repeat as header row at the top of each page. Make sure the first row holds the column headings.",
        |t| {
            let first = t.rows.first()?;
            (!first.header).then(|| {
                "No header row is marked in this table. Screen readers cannot tell which cells are column headings.".to_string()
            })
        },
    )
}

/// TBL-002: a table is inside another table.
pub fn tbl_002(doc: &Document) -> Vec<Finding> {
    each_table(
        doc,
        RuleId::Tbl002,
        "Move the inner table out of the cell, or replace it with rows and columns in the outer table. Use simple tables with one header row.",
        |t| {
            t.nested.then(|| {
                "This table is inside another table. Screen readers handle nested tables badly."
                    .to_string()
            })
        },
    )
}

/// TBL-003: a cell spans more than one column (`w:gridSpan`), or is merged
/// with the cell above or below (`w:vMerge`).
pub fn tbl_003(doc: &Document) -> Vec<Finding> {
    each_table(
        doc,
        RuleId::Tbl003,
        "Split the merged cells (Table Layout > Split Cells) so that every row has the same number of cells. If you keep them, check the headers in PAC.",
        |t| {
            let merged = t
                .rows
                .iter()
                .flat_map(|r| &r.cells)
                .any(|c| c.grid_span > 1 || c.v_merge.is_some());
            merged.then(|| {
                "This table has merged or split cells. Screen readers may not link the cells to the right headings."
                    .to_string()
            })
        },
    )
}

/// TBL-004: probable layout table: one row, or one column, or no text in
/// more than [`LAYOUT_EMPTY_CELLS_PERCENT`] percent of the cells. Cells that
/// continue a vertical merge are not counted as cells.
pub fn tbl_004(doc: &Document) -> Vec<Finding> {
    each_table(
        doc,
        RuleId::Tbl004,
        "Use a table only for data with column headings. For layout, use paragraphs, styles, or columns with a section break. If this is a data table, you can ignore this warning.",
        |t| {
            let cells: Vec<_> = t
                .rows
                .iter()
                .flat_map(|r| &r.cells)
                .filter(|c| c.v_merge != Some(VMerge::Continue))
                .collect();
            let empty = cells.iter().filter(|c| c.text.is_empty()).count();
            let reason = if t.rows.len() == 1 {
                "has only one row"
            } else if t.rows.iter().map(|r| r.width()).max() == Some(1) {
                "has only one column"
            } else if empty * 100 > cells.len() * LAYOUT_EMPTY_CELLS_PERCENT {
                "has no text in most cells"
            } else {
                return None;
            };
            Some(format!(
                "This table {reason}. It is probably used for layout, not for data."
            ))
        },
    )
}

/// TBL-005: a row where every cell is empty. A row with a cell that continues
/// a vertical merge is not reported, because deleting it would change the
/// merged cell. One finding for each table, with the row numbers.
pub fn tbl_005(doc: &Document) -> Vec<Finding> {
    each_table(
        doc,
        RuleId::Tbl005,
        "Delete the empty row (right-click it, then Delete Cells > Delete entire row).",
        |t| {
            let rows: Vec<String> = t
                .rows
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    !r.cells.is_empty()
                        && r.cells
                            .iter()
                            .all(|c| c.empty && c.v_merge != Some(VMerge::Continue))
                })
                .map(|(i, _)| (i + 1).to_string())
                .collect();
            (!rows.is_empty()).then(|| {
                let (what, label) = if rows.len() == 1 {
                    ("an empty row", "row")
                } else {
                    ("empty rows", "rows")
                };
                format!(
                    "This table has {what} ({label} {}). Screen readers announce an empty row.",
                    rows.join(", ")
                )
            })
        },
    )
}

/// TBL-006: a row can break across pages. One finding for each table, with
/// the number of rows that can.
pub fn tbl_006(doc: &Document) -> Vec<Finding> {
    each_table(
        doc,
        RuleId::Tbl006,
        "Select the whole table, open Table Properties > Row, and clear Allow row to break across pages.",
        |t| {
            let rows = t.rows.iter().filter(|r| !r.cant_split).count();
            (rows > 0).then(|| {
                format!(
                    "{rows} of {} rows in this table can break across pages. A row split over two pages is hard to read.",
                    t.rows.len()
                )
            })
        },
    )
}
