//! Stage 4: the `/Scope` of table header cells (rows S4-13 to S4-20 of the test
//! plan). The PDFs are made by test code with `lopdf`: a structure tree with
//! `Table`, `THead`, `TBody`, `TR`, `TH` and `TD`, as Word writes it, and no
//! `/Scope`.

mod pdf_support;

use std::collections::BTreeSet;

use adapdf::pdfmeta::{self, Outcome, SourceMetadata, TableScopeChange, UnscopedHeader};
use lopdf::{Document, Object};
use pdf_support::{Pdf, TITLE, TableSpec, elements, scopes};

fn source() -> SourceMetadata {
    SourceMetadata {
        title: Some(TITLE.to_string()),
        language: Some("en-US".to_string()),
    }
}

fn run(pdf: &[u8]) -> Outcome {
    pdfmeta::check_bytes(pdf, &source()).expect("stage 4 runs")
}

/// A PDF that has all the metadata, so that stage 4 writes only the scopes.
fn with_tables(tables: Vec<TableSpec>, modern: bool) -> Vec<u8> {
    Pdf {
        modern,
        tables,
        ..Pdf::complete()
    }
    .build()
}

fn scopes_after(pdf: &[u8], outcome: &Outcome, tag: &str) -> Vec<Option<String>> {
    scopes(&outcome.apply(pdf), tag)
}

fn scope_names(names: &[Option<&str>]) -> Vec<Option<String>> {
    names.iter().map(|n| n.map(str::to_string)).collect()
}

const COLUMN: Option<&str> = Some("Column");
const ROW: Option<&str> = Some("Row");

fn change(table: usize, column: usize, row: usize) -> TableScopeChange {
    TableScopeChange { table, column, row }
}

fn unscoped(table: usize, row: usize, cell: usize) -> UnscopedHeader {
    UnscopedHeader { table, row, cell }
}

// S4-13
#[test]
fn a_header_row_gets_the_scope_column() {
    for modern in [false, true] {
        let table = TableSpec::new()
            .head(&["TH TH TH"])
            .body(&["TD TD TD", "TD TD TD"]);
        let original = with_tables(vec![table], modern);
        // In the modern layout the structure elements are in object streams, so
        // the bytes of the file do not show them.
        let plain = original.windows(3).any(|w| w == b"/TH");
        assert_eq!(plain, !modern, "modern {modern}");
        let outcome = run(&original);
        assert_eq!(
            scopes_after(&original, &outcome, "TH"),
            scope_names(&[COLUMN, COLUMN, COLUMN]),
            "modern {modern}"
        );
        assert_eq!(outcome.report.table_scopes, vec![change(1, 3, 0)]);
        assert!(outcome.report.unscoped_headers.is_empty());
        assert!(outcome.report.changes.is_empty(), "no metadata change");
        // The cells that are not header cells have no attributes at all.
        assert!(
            scopes_after(&original, &outcome, "TD")
                .iter()
                .all(Option::is_none)
        );
        // The attribute is `<< /O /Table /Scope /Column >>` and nothing else.
        let after = outcome.apply(&original);
        for (_, th) in elements(&after, "TH") {
            let a = th.get(b"A").unwrap().as_dict().unwrap();
            assert_eq!(a.len(), 2);
            assert_eq!(a.get(b"O").unwrap().as_name().unwrap(), b"Table");
            assert_eq!(a.get(b"Scope").unwrap().as_name().unwrap(), b"Column");
        }
    }
}

#[test]
fn a_first_row_in_no_header_group_is_a_header_row_when_it_has_only_headings() {
    // Word writes no `THead` when "Repeat as header row" is off, and still writes
    // `TH` for the first row from `w:tblLook`.
    let table = TableSpec::new().body(&["TH TH", "TD TD", "TD TD"]);
    let original = with_tables(vec![table], false);
    let outcome = run(&original);
    assert_eq!(
        scopes_after(&original, &outcome, "TH"),
        scope_names(&[COLUMN, COLUMN])
    );
    assert_eq!(outcome.report.table_scopes, vec![change(1, 2, 0)]);
}

// S4-14
#[test]
fn a_first_column_gets_the_scope_row() {
    for modern in [false, true] {
        // First Column only: the label and value table. The first row has a `TD`,
        // so its `TH` is a label (Row), not a column heading.
        let table = TableSpec::new().body(&["TH TD", "TH TD", "TH TD"]);
        let original = with_tables(vec![table], modern);
        let outcome = run(&original);
        assert_eq!(
            scopes_after(&original, &outcome, "TH"),
            scope_names(&[ROW, ROW, ROW]),
            "modern {modern}"
        );
        assert_eq!(outcome.report.table_scopes, vec![change(1, 0, 3)]);
        assert!(outcome.report.unscoped_headers.is_empty());
    }
}

// S4-15
#[test]
fn a_header_row_and_a_first_column_get_column_and_row() {
    for modern in [false, true] {
        // First Row and First Column (Word's default, `04A0`).
        let table = TableSpec::new()
            .head(&["TH TH TH"])
            .body(&["TH TD TD", "TH TD TD"]);
        let original = with_tables(vec![table], modern);
        let outcome = run(&original);
        assert_eq!(
            scopes_after(&original, &outcome, "TH"),
            scope_names(&[COLUMN, COLUMN, COLUMN, ROW, ROW]),
            "modern {modern}"
        );
        assert_eq!(outcome.report.table_scopes, vec![change(1, 3, 2)]);
        assert_eq!(
            outcome.report.table_scopes[0].describe(),
            "Table 1: 3 header cells given Column, 2 given Row"
        );
        assert!(outcome.report.unscoped_headers.is_empty());
    }
}

#[test]
fn every_header_row_of_a_header_group_gets_the_scope_column() {
    let table = TableSpec::new()
        .head(&["TH TH TH", "TH TH TH"])
        .body(&["TH TD TD"]);
    let original = with_tables(vec![table], false);
    let outcome = run(&original);
    assert_eq!(
        scopes_after(&original, &outcome, "TH"),
        scope_names(&[COLUMN, COLUMN, COLUMN, COLUMN, COLUMN, COLUMN, ROW])
    );
    assert_eq!(outcome.report.table_scopes, vec![change(1, 6, 1)]);
}

#[test]
fn rows_are_counted_header_rows_then_body_rows_then_footer_rows() {
    // The rows that are in no group are body rows: they come after the rows of
    // the body and before the footer.
    let table = TableSpec::new()
        .head(&["TH TH"])
        .body(&["TH TD"])
        .foot(&["TH TD"])
        .rows(&["TD TH"]);
    let original = with_tables(vec![table], false);
    let outcome = run(&original);
    // Row 1 is the header row, row 2 the body, row 3 the loose row, row 4 the
    // footer. The second cell of the loose row is a heading in the middle.
    assert_eq!(outcome.report.unscoped_headers, vec![unscoped(1, 3, 2)]);
    assert_eq!(outcome.report.table_scopes, vec![change(1, 2, 2)]);
}

// S4-16
#[test]
fn a_header_cell_in_the_middle_is_not_guessed_and_is_reported() {
    let table = TableSpec::new()
        .head(&["TH TH TH"])
        .body(&["TH TD TD", "TD TH TD", "TD TD TH"]);
    let original = with_tables(vec![table], false);
    let outcome = run(&original);
    assert_eq!(
        scopes_after(&original, &outcome, "TH"),
        scope_names(&[COLUMN, COLUMN, COLUMN, ROW, None, None])
    );
    assert_eq!(outcome.report.table_scopes, vec![change(1, 3, 1)]);
    // Row 3 and row 4 of the table (the header row is row 1), cells 2 and 3.
    assert_eq!(
        outcome.report.unscoped_headers,
        vec![unscoped(1, 3, 2), unscoped(1, 4, 3)]
    );
    assert_eq!(
        outcome.report.unscoped_headers[0].describe(),
        "header cell not given a scope (table 1, row 3, cell 2)"
    );
    // Only the three cells that were scoped are in the update.
    assert_eq!(outcome.written_objects.len(), 4);
}

#[test]
fn a_table_that_has_nothing_to_scope_is_not_listed() {
    let table = TableSpec::new().body(&["TD TD", "TD TD"]);
    let original = with_tables(vec![table], false);
    let outcome = run(&original);
    assert!(outcome.appended.is_empty());
    assert!(outcome.report.table_scopes.is_empty());
    assert!(outcome.report.unscoped_headers.is_empty());
}

// S4-17
#[test]
fn a_header_cell_that_has_a_scope_is_left_as_it_is() {
    for modern in [false, true] {
        let table = TableSpec::new()
            .head(&["TH:Both TH"])
            .body(&["TH:Column TD", "TH TD"]);
        let original = with_tables(vec![table], modern);
        let before = elements(&original, "TH");
        let outcome = run(&original);
        let after = outcome.apply(&original);
        assert_eq!(
            scopes(&after, "TH"),
            scope_names(&[Some("Both"), COLUMN, Some("Column"), ROW]),
            "modern {modern}"
        );
        // The two cells that had a scope are byte for byte the objects they were,
        // and they are not in the update.
        let after_cells = elements(&after, "TH");
        assert_eq!(before[0], after_cells[0]);
        assert_eq!(before[2], after_cells[2]);
        let written: BTreeSet<_> = outcome.written_objects.iter().copied().collect();
        let expected: BTreeSet<_> = [before[1].0, before[3].0].into_iter().collect();
        assert_eq!(written, expected);
        assert_eq!(outcome.report.table_scopes, vec![change(1, 1, 1)]);
    }
}

// S4-18
#[test]
fn an_existing_attribute_dictionary_is_kept_and_a_table_attribute_is_added() {
    for modern in [false, true] {
        // A dictionary (a column span, no scope), an array, and an indirect
        // dictionary.
        let table = TableSpec::new().head(&["TH+ TH++ TH&"]).body(&["TD TD TD"]);
        let original = with_tables(vec![table], modern);
        let before = elements(&original, "TH");
        let outcome = run(&original);
        let after = elements(&outcome.apply(&original), "TH");
        assert_eq!(after.len(), 3);
        for (i, ((_, old), (_, new))) in before.iter().zip(&after).enumerate() {
            // Only `/A` is different.
            for (key, value) in old.iter() {
                if key != b"A" {
                    assert_eq!(new.get(key).unwrap(), value, "cell {i}");
                }
            }
            assert_eq!(new.len(), old.len(), "cell {i}");
        }
        let document = Document::load_mem(&outcome.apply(&original)).unwrap();
        for (i, (_, new)) in after.iter().enumerate() {
            let a = new.get(b"A").unwrap().as_array().unwrap();
            assert_eq!(a.len(), 2, "cell {i}: the old attributes and the new ones");
            let old = document.dereference(&a[0]).unwrap().1.as_dict().unwrap();
            assert_eq!(
                old.get(b"RowSpan").unwrap(),
                &Object::Integer(1),
                "cell {i}"
            );
            assert!(!old.has(b"Scope"), "cell {i}");
            let added = a[1].as_dict().unwrap();
            assert_eq!(added.get(b"O").unwrap().as_name().unwrap(), b"Table");
            assert_eq!(added.get(b"Scope").unwrap().as_name().unwrap(), b"Column");
        }
        // The indirect dictionary is still an object of its own, not rewritten.
        let indirect = &after[2].1.get(b"A").unwrap().as_array().unwrap()[0];
        assert!(indirect.as_reference().is_ok());
        assert_eq!(outcome.written_objects.len(), 3);
    }
}

// S4-19
#[test]
fn custom_tags_that_the_role_map_sends_to_table_cells_are_read() {
    let table = TableSpec {
        table_tag: "Grid".to_string(),
        row_tag: "Line".to_string(),
        ..TableSpec::new()
    }
    .head(&["Heading Heading"])
    .body(&["Heading Cell", "Heading Cell"]);
    let original = Pdf {
        tables: vec![table],
        role_map: ["Grid:Table", "Line:TR", "Heading:TH", "Cell:TD"]
            .iter()
            .map(|pair| {
                let (custom, standard) = pair.split_once(':').unwrap();
                (custom.to_string(), standard.to_string())
            })
            .collect(),
        ..Pdf::complete()
    }
    .build();
    let outcome = run(&original);
    assert_eq!(
        scopes_after(&original, &outcome, "Heading"),
        scope_names(&[COLUMN, COLUMN, ROW, ROW])
    );
    assert_eq!(outcome.report.table_scopes, vec![change(1, 2, 2)]);

    // Without the role map the tags mean nothing, and nothing is changed.
    let table = TableSpec {
        table_tag: "Grid".to_string(),
        row_tag: "Line".to_string(),
        ..TableSpec::new()
    }
    .head(&["Heading Heading"]);
    let no_map = with_tables(vec![table], false);
    assert!(run(&no_map).appended.is_empty());
}

#[test]
fn a_role_map_that_loops_does_not_hang() {
    let table = TableSpec::new().head(&["TH TH"]);
    let original = Pdf {
        tables: vec![table],
        role_map: vec![
            ("TH".to_string(), "A".to_string()),
            ("A".to_string(), "TH".to_string()),
        ],
        ..Pdf::complete()
    }
    .build();
    // The tag does not reach a standard tag: nothing is changed.
    assert!(run(&original).appended.is_empty());
}

#[test]
fn several_tables_are_numbered_in_document_order() {
    let first = TableSpec::new().body(&["TD TD"]);
    let second = TableSpec::new().head(&["TH TH"]).body(&["TD TD"]);
    let third = TableSpec::new().body(&["TH TD"]);
    let original = with_tables(vec![first, second, third], false);
    let outcome = run(&original);
    assert_eq!(
        outcome.report.table_scopes,
        vec![change(2, 2, 0), change(3, 0, 1)]
    );
}

// S4-20
#[test]
fn only_the_header_cells_are_written_and_the_original_is_a_prefix() {
    for modern in [false, true] {
        let table = TableSpec::new().head(&["TH TH"]).body(&["TH TD", "TH TD"]);
        let original = with_tables(vec![table], modern);
        let before = Document::load_mem(&original).unwrap();
        let outcome = run(&original);
        let output = outcome.apply(&original);
        assert!(output.starts_with(&original), "modern {modern}");
        assert_eq!(&output[original.len()..], &outcome.appended[..]);

        // The update has one more cross-reference section and is read without a
        // repair.
        let after = Document::load_mem(&output).unwrap();
        assert_eq!(after.xref_section_count, before.xref_section_count + 1);
        assert!(after.xref_recovery.is_none());

        // The objects in the update are the four `TH` and nothing else.
        let th: BTreeSet<_> = elements(&original, "TH")
            .iter()
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(th.len(), 4);
        let written: BTreeSet<_> = outcome.written_objects.iter().copied().collect();
        assert_eq!(written, th, "modern {modern}");

        // Every other object is the same as before: the tags are not changed.
        for (id, object) in &before.objects {
            if !th.contains(id) {
                assert_eq!(after.objects.get(id), Some(object), "{id:?}");
            }
        }
        // The `TH` are the same but for `/A`: the tag, the parent and the kids.
        for ((_, old), (_, new)) in elements(&original, "TH")
            .iter()
            .zip(elements(&output, "TH").iter())
        {
            for key in [&b"S"[..], b"P", b"K", b"Type"] {
                assert_eq!(old.get(key).ok(), new.get(key).ok());
            }
        }
        // The appended text has no content stream, page, font or metadata.
        let tail = String::from_utf8_lossy(&outcome.appended);
        for word in ["/Kids", "/MediaBox", "/Font", "/Contents", "/Metadata"] {
            assert!(
                !tail.contains(word),
                "modern {modern}: the update has {word}"
            );
        }
        assert!(tail.contains("/Scope"));
    }
}

#[test]
fn the_scopes_and_the_metadata_are_written_in_one_update() {
    for modern in [false, true] {
        let table = TableSpec::new().head(&["TH TH"]).body(&["TD TD"]);
        let original = Pdf {
            modern,
            tables: vec![table],
            ..Pdf::all_missing()
        }
        .build();
        let before = Document::load_mem(&original).unwrap();
        let outcome = run(&original);
        assert_eq!(outcome.report.changes.len(), 5, "the five metadata changes");
        assert_eq!(outcome.report.table_scopes, vec![change(1, 2, 0)]);
        let after = Document::load_mem(&outcome.apply(&original)).unwrap();
        assert_eq!(
            after.xref_section_count,
            before.xref_section_count + 1,
            "one appended section, modern {modern}"
        );
        assert_eq!(
            outcome
                .appended
                .windows(9)
                .filter(|w| *w == b"startxref")
                .count(),
            1
        );
        // The two header cells, the catalog, the XMP stream and the Info.
        assert_eq!(
            outcome.written_objects.len(),
            5,
            "{:?}",
            outcome.written_objects
        );
    }
}

#[test]
fn running_stage_4_again_changes_nothing() {
    for modern in [false, true] {
        let table = TableSpec::new().head(&["TH TH"]).body(&["TH TD", "TD TH+"]);
        let original = with_tables(vec![table], modern);
        let first = run(&original);
        assert!(!first.appended.is_empty());
        let output = first.apply(&original);
        let second = run(&output);
        assert!(second.appended.is_empty(), "modern {modern}");
        assert!(second.written_objects.is_empty());
        assert!(second.report.table_scopes.is_empty());
        // The header cell that was not scoped is still reported, and is not
        // reported as a change.
        assert_eq!(
            second.report.unscoped_headers,
            first.report.unscoped_headers
        );
        assert_eq!(second.apply(&output), output);
    }
}

#[test]
fn a_header_cell_that_is_not_an_object_of_its_own_is_reported() {
    // A `TH` that is a dictionary inside its row cannot get a new version of its
    // own, and the row is not rewritten: nothing is guessed.
    let pdf = with_tables(vec![TableSpec::new().head(&["TH"])], false);
    let (row_id, _) = elements(&pdf, "TR")[0].clone();
    let (_, th) = elements(&pdf, "TH")[0].clone();
    let loaded = Document::load_mem(&pdf).unwrap();
    let mut incremental = lopdf::IncrementalDocument::create_from(pdf, loaded);
    incremental
        .opt_clone_object_to_new_document(row_id)
        .unwrap();
    if let Ok(Object::Dictionary(row)) = incremental.new_document.get_object_mut(row_id) {
        row.set("K", Object::Array(vec![Object::Dictionary(th)]));
    }
    let mut original = Vec::new();
    incremental.save_to(&mut original).unwrap();

    let outcome = run(&original);
    assert_eq!(outcome.report.unscoped_headers, vec![unscoped(1, 1, 1)]);
    assert!(outcome.report.table_scopes.is_empty());
    assert!(outcome.appended.is_empty());
}

#[test]
fn an_untagged_pdf_is_unchanged_and_is_an_error() {
    let original = Pdf {
        struct_tree: false,
        ..Pdf::all_missing()
    }
    .build();
    let outcome = run(&original);
    assert!(outcome.appended.is_empty());
    assert!(outcome.report.has_errors());
    assert!(outcome.report.table_scopes.is_empty());
}

#[test]
fn the_report_names_of_the_scopes_are_stable() {
    let table = TableSpec::new().head(&["TH"]).body(&["TD TH"]);
    let original = with_tables(vec![table], false);
    let json = serde_json::to_value(&run(&original).report).unwrap();
    assert_eq!(json["table_scopes"][0]["table"], 1);
    assert_eq!(json["table_scopes"][0]["column"], 1);
    assert_eq!(json["table_scopes"][0]["row"], 0);
    assert_eq!(json["unscoped_headers"][0]["table"], 1);
    assert_eq!(json["unscoped_headers"][0]["row"], 2);
    assert_eq!(json["unscoped_headers"][0]["cell"], 2);
}
