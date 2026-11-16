//! Fixes for tables: the header row (TBL-001), empty rows (TBL-005) and rows
//! that break across pages (TBL-006).

use roxmltree::Node;

use crate::docx::{DOCUMENT_PART, WordNode};
use crate::model::tables::{Table, VMerge};

use super::xml::{
    Splices, insert_children_in_order, prepend_child, source_name, toggle_on, w_name,
};
use super::{Engine, Reason};

/// The children of `w:trPr` in schema order (ECMA-376, `CT_TrPr`). The first
/// ones can be in any order; the last three must come last.
const TRPR_ORDER: &[&str] = &[
    "cnfStyle",
    "divId",
    "gridBefore",
    "gridAfter",
    "wBefore",
    "wAfter",
    "cantSplit",
    "trHeight",
    "tblHeader",
    "tblCellSpacing",
    "jc",
    "hidden",
    "ins",
    "del",
    "trPrChange",
];

/// Changes to the properties of one table row.
#[derive(Debug, Default)]
pub(super) struct RowEdit {
    pub header: bool,
    pub cant_split: bool,
}

/// Writes the collected changes to the properties of row `tr`.
pub(super) fn render_row(sp: &mut Splices, text: &str, tr: Node, edit: &RowEdit) {
    let wanted: Vec<&str> = [(edit.cant_split, "cantSplit"), (edit.header, "tblHeader")]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| name)
        .collect();
    let element = |local: &str| format!("<{}/>", w_name(tr, local));
    let Some(properties) = tr.w_child("trPr") else {
        let children: String = wanted.iter().map(|n| element(n)).collect();
        let name = w_name(tr, "trPr");
        let xml = format!("<{name}>{children}</{name}>");
        match tr.w_child("tblPrEx") {
            Some(exceptions) => sp.insert(exceptions.range().end, xml),
            None => prepend_child(sp, text, tr, &xml),
        }
        return;
    };
    let mut new = Vec::new();
    for local in wanted {
        match properties.w_child(local) {
            Some(existing) if toggle_on(existing) => {}
            Some(existing) => sp.replace(
                existing.range(),
                format!("<{}/>", source_name(text, existing)),
            ),
            None => new.push((local, element(local))),
        }
    }
    insert_children_in_order(sp, text, properties, TRPR_ORDER, &new);
}

/// The rows that TBL-005 reports: every cell is empty, and none continues a
/// vertical merge.
fn empty_rows(table: &Table) -> Vec<usize> {
    table
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            !r.cells.is_empty()
                && r.cells
                    .iter()
                    .all(|c| c.empty && c.v_merge != Some(VMerge::Continue))
        })
        .map(|(i, _)| i)
        .collect()
}

fn lacks_header(table: &Table) -> bool {
    table.rows.first().is_some_and(|r| !r.header)
}

fn has_breaking_row(table: &Table) -> bool {
    table.rows.iter().any(|r| !r.cant_split)
}

impl<'a> Engine<'a> {
    pub(super) fn row_node(&self, table: usize, row: usize) -> Option<Node<'a, 'a>> {
        self.table_nodes
            .get(table)?
            .children()
            .filter(|n| n.is_w("tr"))
            .nth(row)
    }

    /// The table of the `position`-th finding of a table rule: every table
    /// that `problem` is true for gives one finding, in table order.
    fn table_of_finding(
        &self,
        position: usize,
        problem: impl Fn(&Table) -> bool,
    ) -> Result<usize, Reason> {
        self.doc
            .tables
            .iter()
            .enumerate()
            .filter(|(_, t)| problem(t))
            .nth(position)
            .map(|(i, _)| i)
            .filter(|&i| i < self.table_nodes.len())
            .ok_or_else(|| "The table could not be found in the file.".to_string())
    }

    pub(super) fn mark_header_row(&mut self, position: usize) -> Result<String, Reason> {
        let table = self.table_of_finding(position, lacks_header)?;
        self.row_node(table, 0)
            .ok_or("The first row could not be found in the file.")?;
        self.row_edits.entry((table, 0)).or_default().header = true;
        Ok(format!(
            "Marked the first row of {} as the header row.",
            self.describe_table(table)
        ))
    }

    pub(super) fn disallow_row_break(&mut self, position: usize) -> Result<String, Reason> {
        let table = self.table_of_finding(position, has_breaking_row)?;
        let mut rows = 0;
        for (row, r) in self.doc.tables[table].rows.iter().enumerate() {
            if !r.cant_split {
                self.row_node(table, row)
                    .ok_or("A row could not be found in the file.")?;
                self.row_edits.entry((table, row)).or_default().cant_split = true;
                rows += 1;
            }
        }
        Ok(format!(
            "Stopped {rows} rows of {} from breaking across pages.",
            self.describe_table(table)
        ))
    }

    pub(super) fn delete_empty_rows(&mut self, position: usize) -> Result<String, Reason> {
        let table = self.table_of_finding(position, |t| !empty_rows(t).is_empty())?;
        let model = &self.doc.tables[table];
        let empty = empty_rows(model);
        // A row that starts a vertical merge cannot go: the rows below would
        // lose the cell they continue.
        let (blocked, deletable): (Vec<usize>, Vec<usize>) = empty
            .iter()
            .partition(|&&r| model.rows[r].cells.iter().any(|c| c.v_merge.is_some()));
        if deletable.is_empty() {
            return Err(
                "The empty rows have merged cells, so they were not deleted. Delete them in Word."
                    .to_string(),
            );
        }
        if deletable.len() == model.rows.len() {
            return Err("Every row of the table is empty, so the rows were not deleted. Delete the table in Word.".to_string());
        }
        for &row in &deletable {
            let node = self
                .row_node(table, row)
                .ok_or("A row could not be found in the file.")?;
            self.splices_for(DOCUMENT_PART).delete(node.range());
        }
        let numbers: Vec<String> = deletable.iter().map(|r| (r + 1).to_string()).collect();
        let mut summary = format!(
            "Deleted empty row {} of {}.",
            numbers.join(", "),
            self.describe_table(table)
        );
        if !blocked.is_empty() {
            summary.push_str(" Rows with merged cells were kept.");
        }
        Ok(summary)
    }

    fn describe_table(&self, table: usize) -> String {
        match self.doc.tables[table].first_paragraph {
            Some(p) => format!("the table at paragraph {}", p + 1),
            None => "the table".to_string(),
        }
    }
}
