//! Stage 4: the `/Scope` of the header cells of a table.
//!
//! Word writes a header cell (`TH`) for the first row and the first column of a
//! table, from `w:tblLook`, and it never writes a `/Scope` for it. veraPDF fails
//! rule 7.5-1 for every table that mixes `TH` and `TD` cells without a scope
//! (test plan S5-14). This module works out the scope that each such `TH` needs
//! and the new version of its dictionary. It changes no tag, no text and no
//! order: the only new thing is a table attribute on the `TH`.
//!
//! The rule for a `TH` that has no `/Scope`:
//!
//! - in a `THead`, or in the first row of the table when that row has no `TD`
//!   (a row of headings): `/Column`;
//! - otherwise, the first cell of its row: `/Row`. This includes the first cell
//!   of a first row that has a `TD` as well, which is a label and not a column
//!   heading (the table has only a first column of headings);
//! - otherwise nothing is guessed. The cell is listed in the report.
//!
//! A `TH` that has a `/Scope` is left as it is.

use std::collections::{BTreeMap, HashMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};

/// How deep the walk of the structure tree goes, so that a damaged PDF cannot
/// use all the stack.
const MAX_DEPTH: usize = 200;

/// How many `/RoleMap` steps are followed from a custom tag to a standard one.
const MAX_ROLE_STEPS: usize = 16;

/// What stage 4 did for one table: the header cells that got a scope. A table
/// that needed nothing is not listed. The tables are numbered from 1 in document
/// order, the same way as the places of a veraPDF failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableScopeChange {
    pub table: usize,
    /// Header cells that were given the scope `Column`.
    pub column: usize,
    /// Header cells that were given the scope `Row`.
    pub row: usize,
}

impl TableScopeChange {
    /// For example `2 header cells given Column, 3 given Row`.
    pub fn summary(&self) -> String {
        let cells = |n: usize| {
            if n == 1 {
                "1 header cell".to_string()
            } else {
                format!("{n} header cells")
            }
        };
        match (self.column, self.row) {
            (0, row) => format!("{} given Row", cells(row)),
            (column, 0) => format!("{} given Column", cells(column)),
            (column, row) => format!("{} given Column, {row} given Row", cells(column)),
        }
    }

    /// For example `Table 1: 2 header cells given Column, 3 given Row`.
    pub fn describe(&self) -> String {
        format!("Table {}: {}", self.table, self.summary())
    }
}

/// A header cell that stage 4 could not give a scope, because the table is not
/// regular (a heading in the middle of a table) or the cell cannot be changed.
/// The numbers count from 1: the row counts the header rows first, then the body
/// rows and the footer rows; the cell counts the cells of the row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnscopedHeader {
    pub table: usize,
    pub row: usize,
    pub cell: usize,
}

impl UnscopedHeader {
    pub fn describe(&self) -> String {
        format!(
            "header cell not given a scope (table {}, row {}, cell {})",
            self.table, self.row, self.cell
        )
    }
}

/// The new version of each `TH` that needs a scope, and what to tell the user.
#[derive(Debug, Default)]
pub struct Plan {
    /// The object number and the new dictionary of each changed `TH`.
    pub edits: Vec<(ObjectId, Dictionary)>,
    pub tables: Vec<TableScopeChange>,
    pub unscoped: Vec<UnscopedHeader>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Table,
    Head,
    Body,
    Foot,
    Row,
    HeaderCell,
    DataCell,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Column,
    Row,
}

impl Scope {
    fn name(self) -> &'static str {
        match self {
            Scope::Column => "Column",
            Scope::Row => "Row",
        }
    }
}

/// A structure element: its object number (a direct dictionary has none) and its
/// dictionary.
#[derive(Clone, Copy)]
struct Elem<'a> {
    id: Option<ObjectId>,
    dict: &'a Dictionary,
}

struct Walker<'a> {
    document: &'a Document,
    /// `/RoleMap` of the structure tree: a custom tag and the tag it stands for.
    roles: HashMap<Vec<u8>, Vec<u8>>,
    seen: HashSet<ObjectId>,
    tables: usize,
    /// For each table number: the cells given `Column` and the cells given `Row`.
    counts: BTreeMap<usize, (usize, usize)>,
    plan: Plan,
}

/// Works out the scope fix for the structure tree of the document. The caller
/// has checked that the catalog has a `/StructTreeRoot`.
pub fn plan(document: &Document) -> Plan {
    let Ok(root) = document
        .catalog()
        .and_then(|c| c.get_deref(b"StructTreeRoot", document))
        .and_then(Object::as_dict)
    else {
        return Plan::default();
    };
    let mut roles = HashMap::new();
    if let Ok(map) = root
        .get_deref(b"RoleMap", document)
        .and_then(Object::as_dict)
    {
        for (custom, standard) in map.iter() {
            if let Ok(Ok(standard)) = document
                .dereference(standard)
                .map(|(_, object)| object.as_name())
            {
                roles.insert(custom.clone(), standard.to_vec());
            }
        }
    }
    let mut walker = Walker {
        document,
        roles,
        seen: HashSet::new(),
        tables: 0,
        counts: BTreeMap::new(),
        plan: Plan::default(),
    };
    for kid in walker.kids(root) {
        walker.walk(kid, 0);
    }
    let mut plan = walker.plan;
    plan.tables = walker
        .counts
        .into_iter()
        .filter(|(_, (column, row))| column + row > 0)
        .map(|(table, (column, row))| TableScopeChange { table, column, row })
        .collect();
    plan.unscoped.sort_by_key(|u| (u.table, u.row, u.cell));
    plan
}

impl<'a> Walker<'a> {
    /// The structure elements that are the children of `dict` (`/K` is one
    /// element or an array; marked-content references and numbers are skipped).
    fn kids(&self, dict: &'a Dictionary) -> Vec<Elem<'a>> {
        let Ok(kids) = dict.get(b"K") else {
            return Vec::new();
        };
        let items: Vec<&'a Object> = match self.document.dereference(kids) {
            Ok((_, Object::Array(items))) => items.iter().collect(),
            _ => vec![kids],
        };
        items
            .into_iter()
            .filter_map(|item| match self.document.dereference(item) {
                Ok((id, Object::Dictionary(dict))) if dict.has(b"S") => Some(Elem { id, dict }),
                _ => None,
            })
            .collect()
    }

    fn role(&self, dict: &Dictionary) -> Role {
        let Ok(mut name) = dict.get(b"S").and_then(Object::as_name) else {
            return Role::Other;
        };
        let mut steps = 0;
        while let Some(next) = self.roles.get(name).filter(|n| n.as_slice() != name) {
            // A loop in the map does not reach a standard tag.
            steps += 1;
            if steps > MAX_ROLE_STEPS {
                return Role::Other;
            }
            name = next;
        }
        match name {
            b"Table" => Role::Table,
            b"THead" => Role::Head,
            b"TBody" => Role::Body,
            b"TFoot" => Role::Foot,
            b"TR" => Role::Row,
            b"TH" => Role::HeaderCell,
            b"TD" => Role::DataCell,
            _ => Role::Other,
        }
    }

    /// `false` when the element was visited before (a loop in a damaged tree).
    fn first_visit(&mut self, elem: Elem) -> bool {
        elem.id.is_none_or(|id| self.seen.insert(id))
    }

    fn walk(&mut self, elem: Elem<'a>, depth: usize) {
        if depth > MAX_DEPTH || !self.first_visit(elem) {
            return;
        }
        if self.role(elem.dict) == Role::Table {
            self.table(elem, depth);
        } else {
            for kid in self.kids(elem.dict) {
                self.walk(kid, depth + 1);
            }
        }
    }

    fn table(&mut self, table: Elem<'a>, depth: usize) {
        self.tables += 1;
        let number = self.tables;

        // The rows in table order: the header rows, then the body rows (and the
        // rows that are directly in the table), then the footer rows.
        let mut head = Vec::new();
        let mut body = Vec::new();
        let mut foot = Vec::new();
        let mut other = Vec::new();
        for kid in self.kids(table.dict) {
            let group = match self.role(kid.dict) {
                Role::Head => &mut head,
                Role::Body => &mut body,
                Role::Foot => &mut foot,
                Role::Row => {
                    if self.first_visit(kid) {
                        body.push(kid);
                    }
                    continue;
                }
                _ => {
                    other.push(kid);
                    continue;
                }
            };
            if !self.first_visit(kid) {
                continue;
            }
            for row in self.kids(kid.dict) {
                if self.role(row.dict) == Role::Row {
                    if self.first_visit(row) {
                        group.push(row);
                    }
                } else {
                    other.push(row);
                }
            }
        }
        let head_rows = head.len();
        let rows: Vec<Elem<'a>> = head.into_iter().chain(body).chain(foot).collect();

        for (row_index, row) in rows.iter().enumerate() {
            let mut cell_index = 0;
            let cells = self.kids(row.dict);
            let has_data_cell = cells
                .iter()
                .any(|kid| self.role(kid.dict) == Role::DataCell);
            for kid in cells {
                let role = self.role(kid.dict);
                if !matches!(role, Role::HeaderCell | Role::DataCell) {
                    other.push(kid);
                    continue;
                }
                if role == Role::HeaderCell && self.first_visit(kid) {
                    let heading_row = row_index < head_rows || (row_index == 0 && !has_data_cell);
                    let scope = if heading_row {
                        Some(Scope::Column)
                    } else if cell_index == 0 {
                        Some(Scope::Row)
                    } else {
                        None
                    };
                    self.scope_header(number, row_index, cell_index, kid, scope);
                }
                // A table can be inside a cell.
                for nested in self.kids(kid.dict) {
                    self.walk(nested, depth + 1);
                }
                cell_index += 1;
            }
        }
        for kid in other {
            self.walk(kid, depth + 1);
        }
    }

    fn scope_header(
        &mut self,
        table: usize,
        row: usize,
        cell: usize,
        th: Elem<'a>,
        scope: Option<Scope>,
    ) {
        if self.has_scope(th.dict) {
            return;
        }
        let edit = match (scope, th.id) {
            (Some(scope), Some(id)) => self.with_scope(th.dict, scope).map(|d| (id, scope, d)),
            _ => None,
        };
        let Some((id, scope, dict)) = edit else {
            self.plan.unscoped.push(UnscopedHeader {
                table,
                row: row + 1,
                cell: cell + 1,
            });
            return;
        };
        self.plan.edits.push((id, dict));
        let counts = self.counts.entry(table).or_default();
        match scope {
            Scope::Column => counts.0 += 1,
            Scope::Row => counts.1 += 1,
        }
    }

    /// The attribute dictionaries of a structure element: `/A` is one dictionary
    /// or an array of them (and revision numbers), and each may be indirect.
    fn attributes(&self, dict: &'a Dictionary) -> Vec<&'a Dictionary> {
        let Ok(attributes) = dict.get(b"A") else {
            return Vec::new();
        };
        let items: Vec<&'a Object> = match self.document.dereference(attributes) {
            Ok((_, Object::Array(items))) => items.iter().collect(),
            _ => vec![attributes],
        };
        items
            .into_iter()
            .filter_map(|item| match self.document.dereference(item) {
                Ok((_, Object::Dictionary(dict))) => Some(dict),
                _ => None,
            })
            .collect()
    }

    fn has_scope(&self, dict: &'a Dictionary) -> bool {
        self.attributes(dict).iter().any(|a| a.has(b"Scope"))
    }

    /// The `TH` with a new table attribute dictionary `<< /O /Table /Scope ... >>`.
    /// The attributes it has are kept. `None` when `/A` is not a dictionary or an
    /// array, so that the entry is not guessed at.
    fn with_scope(&self, dict: &'a Dictionary, scope: Scope) -> Option<Dictionary> {
        let mut new = Dictionary::new();
        new.set("O", Object::Name(b"Table".to_vec()));
        new.set("Scope", Object::Name(scope.name().as_bytes().to_vec()));
        let new = Object::Dictionary(new);
        let mut changed = dict.clone();
        match dict.get(b"A") {
            Err(_) | Ok(Object::Null) => changed.set("A", new),
            Ok(attributes) => match self.document.dereference(attributes) {
                Ok((_, Object::Dictionary(_))) => {
                    changed.set("A", Object::Array(vec![attributes.clone(), new]));
                }
                Ok((_, Object::Array(items))) => {
                    let mut items = items.clone();
                    items.push(new);
                    changed.set("A", Object::Array(items));
                }
                _ => return None,
            },
        }
        Some(changed)
    }
}
