//! Tables: rows, cells and the properties the table rules read.
//!
//! Every `w:tbl` in the body is in [`Document::tables`](super::Document::tables),
//! outer tables before the tables inside them. Tables in text boxes are not
//! read, like the paragraphs in them. Table styles are not applied.

use std::ops::Range;

use roxmltree::Node;

use super::Paragraph;
use super::body::is_alternative_content;
use super::props::toggle;
use crate::docx::WordNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The table is inside a cell of another table.
    pub nested: bool,
    /// Index of the first body paragraph in the table.
    pub first_paragraph: Option<usize>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    /// `w:trPr/w:tblHeader`: "Repeat as header row at the top of each page".
    pub header: bool,
    /// `w:trPr/w:cantSplit`: "Allow row to break across pages" is off.
    pub cant_split: bool,
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMerge {
    /// The top cell of a vertical merge.
    Restart,
    /// A cell that continues the merge from the row above.
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCell {
    /// Text of all paragraphs in the cell (including a nested table), joined
    /// with a space and trimmed.
    pub text: String,
    /// Nothing in the cell: no text, drawing, equation, break or field.
    pub empty: bool,
    /// Columns the cell spans (`w:gridSpan`), at least 1.
    pub grid_span: u32,
    pub v_merge: Option<VMerge>,
}

impl TableRow {
    /// The width of the row in grid columns.
    pub fn width(&self) -> u32 {
        self.cells.iter().map(|c| c.grid_span).sum()
    }
}

/// The paragraphs whose `w:p` lies inside `range`. `paragraphs` is in
/// document order, so this is a slice.
fn paragraphs_in<'a>(paragraphs: &'a [Paragraph], range: &Range<usize>) -> &'a [Paragraph] {
    let start = paragraphs.partition_point(|p| p.range.start < range.start);
    let len = paragraphs[start..].partition_point(|p| p.range.start < range.end);
    &paragraphs[start..start + len]
}

pub(super) fn parse(body: Node, paragraphs: &[Paragraph]) -> Vec<Table> {
    body.descendants()
        .filter(|n| n.is_w("tbl") && !is_alternative_content(*n))
        .map(|tbl| Table {
            nested: tbl.ancestors().skip(1).any(|a| a.is_w("tbl")),
            first_paragraph: paragraphs_in(paragraphs, &tbl.range())
                .first()
                .map(|p| p.index),
            rows: tbl
                .children()
                .filter(|n| n.is_w("tr"))
                .map(|tr| parse_row(tr, paragraphs))
                .collect(),
        })
        .collect()
}

fn parse_row(tr: Node, paragraphs: &[Paragraph]) -> TableRow {
    let flag = |name: &str| {
        tr.w_child("trPr")
            .and_then(|pr| pr.w_child(name))
            .is_some_and(|n| toggle(&n))
    };
    TableRow {
        header: flag("tblHeader"),
        cant_split: flag("cantSplit"),
        cells: tr
            .children()
            .filter(|n| n.is_w("tc"))
            .map(|tc| parse_cell(tc, paragraphs))
            .collect(),
    }
}

fn parse_cell(tc: Node, paragraphs: &[Paragraph]) -> TableCell {
    let inside = paragraphs_in(paragraphs, &tc.range());
    let tc_pr = tc.w_child("tcPr");
    let grid_span = tc_pr
        .and_then(|pr| pr.w_child("gridSpan"))
        .and_then(|n| n.w_attr("val"))
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|&span| span >= 1)
        .unwrap_or(1);
    let v_merge = tc_pr.and_then(|pr| pr.w_child("vMerge")).map(|n| {
        if n.w_attr("val") == Some("restart") {
            VMerge::Restart
        } else {
            VMerge::Continue
        }
    });
    TableCell {
        text: inside
            .iter()
            .map(|p| p.text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        empty: inside.iter().all(Paragraph::is_spacing_only),
        grid_span,
        v_merge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{DocxBuilder, Para, Table as FixtureTable};
    use crate::model::Document;

    fn tables(builder: &DocxBuilder) -> Vec<Table> {
        Document::from_bytes(&builder.build()).unwrap().tables
    }

    #[test]
    fn rows_cells_and_row_properties() {
        let t = tables(
            &DocxBuilder::new()
                .para("Before")
                .table(FixtureTable::new(&[&["A", "B"], &["C", ""]]).rows_can_split()),
        );
        assert_eq!(t.len(), 1);
        assert!(!t[0].nested);
        assert_eq!(t[0].first_paragraph, Some(1));
        assert_eq!(t[0].rows.len(), 2);
        assert!(t[0].rows[0].header);
        assert!(!t[0].rows[1].header);
        assert!(!t[0].rows[0].cant_split);
        assert_eq!(t[0].rows[0].cells[1].text, "B");
        assert!(t[0].rows[1].cells[1].empty);
        assert!(!t[0].rows[1].cells[0].empty);
        assert_eq!(t[0].rows[0].width(), 2);
    }

    #[test]
    fn the_toggle_value_is_read() {
        let t =
            tables(&DocxBuilder::new().table(
                FixtureTable::new(&[&["A"], &["B"]]).raw_row_properties(
                    0,
                    r#"<w:tblHeader w:val="0"/><w:cantSplit w:val="false"/>"#,
                ),
            ));
        assert!(!t[0].rows[0].header);
        assert!(!t[0].rows[0].cant_split);
    }

    #[test]
    fn spans_and_merges() {
        let t = tables(
            &DocxBuilder::new().table(
                FixtureTable::new(&[&["A", "B", "C"], &["D", "E", "F"], &["G", "H", "I"]])
                    .span(0, 0, 2)
                    .merge_down(1, 2, true)
                    .merge_down(2, 2, false),
            ),
        );
        let cells = |row: usize| &t[0].rows[row].cells;
        assert_eq!(cells(0)[0].grid_span, 2);
        assert_eq!(cells(1)[2].v_merge, Some(VMerge::Restart));
        assert_eq!(cells(2)[2].v_merge, Some(VMerge::Continue));
        assert_eq!(cells(0)[1].v_merge, None);
    }

    #[test]
    fn nested_tables_come_after_their_outer_table() {
        let inner = FixtureTable::new(&[&["x", "y"], &["z", "w"]]);
        let t = tables(
            &DocxBuilder::new()
                .table(FixtureTable::new(&[&["A", ""], &["B", "C"]]).nested(0, 1, &inner)),
        );
        assert_eq!(t.len(), 2);
        assert!(!t[0].nested);
        assert!(t[1].nested);
        assert_eq!(t[0].rows[0].cells[1].text, "x y z w");
        assert!(!t[0].rows[0].cells[1].empty);
        assert_eq!(t[1].first_paragraph, Some(1));
    }

    #[test]
    fn a_cell_with_only_a_picture_is_not_empty_but_has_no_text() {
        let picture = r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="P"/></wp:inline></w:drawing></w:r>"#;
        let t = tables(
            &DocxBuilder::new().table(FixtureTable::new(&[&["A", ""]]).raw_cell(
                0,
                1,
                &format!("<w:p>{picture}</w:p>"),
            )),
        );
        assert_eq!(t[0].rows[0].cells[1].text, "");
        assert!(!t[0].rows[0].cells[1].empty);
    }

    #[test]
    fn tables_in_text_boxes_are_not_read() {
        let run = r#"<w:r><w:drawing><wp:anchor><wp:docPr id="1" name="B"/><w:txbxContent><w:tbl><w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl><w:p/></w:txbxContent></wp:anchor></w:drawing></w:r>"#;
        let t = tables(&DocxBuilder::new().paragraph(Para::empty().raw_run(run)));
        assert!(t.is_empty());
    }
}
