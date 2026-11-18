//! What Word writes for the cells of a table, and what veraPDF says about it (test
//! plan row S5-14). It was measured, not read from a specification: Word 16.0.20527
//! and veraPDF 1.28.2 on 2026-11-18.
//!
//! The test exports small documents with the tool's own hidden Word (one session
//! for all of them), reads the structure tree of each PDF with `lopdf`, and runs
//! the bundled veraPDF on it, as Word wrote it and after stage 4 (which adds the
//! `/Scope` of the header cells). It needs both cargo features:
//!
//! ```text
//! cargo test --features windows_word,verapdf --test export_tables -- --nocapture
//! ```
//!
//! The `WINWORD.EXE` processes are listed with `tasklist` before and after, and
//! the lists must be equal. The files are removed at the end.
#![cfg(all(windows, feature = "windows_word", feature = "verapdf"))]

mod export_support;
mod word_support;

use std::fs;
use std::time::Duration;

use adapdf::fixtures::{Table, clean};
use adapdf::pdfmeta::{self, SourceMetadata};
use adapdf::verapdf::{Tool, validate};
use adapdf::word::ExportParameters;
use export_support::{pdf_tables, table_summary};
use word_support::{TempDir, exclusive, wait_for_word_pids, within, word_pids};

const BORDERS: &str = r#"<w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders>"#;

/// Word's "Table Grid": borders, and no conditional format at all.
const TABLE_GRID: &str = r#"<w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:left w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:right w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="auto"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="auto"/></w:tblBorders></w:tblPr>"#;

/// Word's "Grid Table 4 - Accent 1" as Word writes it (the theme colours as their
/// plain values): borders, a shaded first row, and a bold first and last column.
const GRID_TABLE_4: &str = r#"<w:uiPriority w:val="49"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr><w:tblPr><w:tblStyleRowBandSize w:val="1"/><w:tblStyleColBandSize w:val="1"/><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="8EAADB"/><w:left w:val="single" w:sz="4" w:space="0" w:color="8EAADB"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="8EAADB"/><w:right w:val="single" w:sz="4" w:space="0" w:color="8EAADB"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="8EAADB"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="8EAADB"/></w:tblBorders></w:tblPr><w:tblStylePr w:type="firstRow"><w:rPr><w:b/><w:bCs/><w:color w:val="FFFFFF"/></w:rPr><w:tblPr/><w:tcPr><w:tcBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="4472C4"/><w:left w:val="single" w:sz="4" w:space="0" w:color="4472C4"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="4472C4"/><w:right w:val="single" w:sz="4" w:space="0" w:color="4472C4"/><w:insideH w:val="nil"/><w:insideV w:val="nil"/></w:tcBorders><w:shd w:val="clear" w:color="auto" w:fill="4472C4"/></w:tcPr></w:tblStylePr><w:tblStylePr w:type="lastRow"><w:rPr><w:b/><w:bCs/></w:rPr><w:tblPr/><w:tcPr><w:tcBorders><w:top w:val="double" w:sz="4" w:space="0" w:color="4472C4"/></w:tcBorders></w:tcPr></w:tblStylePr><w:tblStylePr w:type="firstCol"><w:rPr><w:b/><w:bCs/></w:rPr></w:tblStylePr><w:tblStylePr w:type="lastCol"><w:rPr><w:b/><w:bCs/></w:rPr></w:tblStylePr><w:tblStylePr w:type="band1Vert"><w:tblPr/><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="D9E2F3"/></w:tcPr></w:tblStylePr><w:tblStylePr w:type="band1Horz"><w:tblPr/><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="D9E2F3"/></w:tcPr></w:tblStylePr>"#;

/// Shading in the first row, and no borders.
const SHADED: &str = r#"<w:tblStylePr w:type="firstRow"><w:rPr><w:b/></w:rPr><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="D9E2F3"/></w:tcPr></w:tblStylePr>"#;

/// A first-column format only: no borders, no shading.
const PLAIN: &str = r#"<w:tblStylePr w:type="firstCol"><w:rPr><w:b/></w:rPr></w:tblStylePr>"#;

/// `w:tblLook` as Word writes it: the six attributes and the matching `w:val`.
fn look_xml(first_row: bool, first_column: bool) -> String {
    let val = 0x0400 + if first_row { 0x20 } else { 0 } + if first_column { 0x80 } else { 0 };
    format!(
        r#"<w:tblLook w:val="{val:04X}" w:firstRow="{}" w:lastRow="0" w:firstColumn="{}" w:lastColumn="0" w:noHBand="0" w:noVBand="1"/>"#,
        u8::from(first_row),
        u8::from(first_column)
    )
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    /// 3 rows of a label and a value.
    LabelValue,
    /// 4 rows of 3 columns: a column heading row and 3 body rows.
    ThreeColumns,
    /// A row of column headings and nothing else.
    HeadingsOnly,
}

impl Shape {
    fn table(self) -> Table {
        match self {
            Shape::LabelValue => Table::new(&[
                &["Training Topic:", "Parking"],
                &["Use Case:", "Reserve a space"],
                &["Audience:", "All staff"],
            ]),
            Shape::ThreeColumns => Table::new(&[
                &["Step", "What to do", "Who"],
                &["1", "Open the app", "Staff"],
                &["2", "Pick a space", "Staff"],
                &["3", "Confirm", "Staff"],
            ]),
            Shape::HeadingsOnly => Table::new(&[&["Step", "What to do"]]),
        }
    }

    /// Rows and columns.
    fn size(self) -> (usize, usize) {
        match self {
            Shape::LabelValue => (3, 2),
            Shape::ThreeColumns => (4, 3),
            Shape::HeadingsOnly => (1, 2),
        }
    }
}

/// What is in `w:tblLook`, or nothing.
#[derive(Clone, Copy, Debug)]
enum Look {
    Absent,
    Flags { first_row: bool, first_column: bool },
}

struct Case {
    name: String,
    shape: Shape,
    style: Option<&'static str>,
    /// Borders written in the table itself (not in a style).
    direct_borders: bool,
    look: Look,
    /// "Repeat as header row at the top of each page" on the first row.
    repeat_header: bool,
    /// Word writes a `Table` (borders or shading are there) and not paragraphs.
    tagged: bool,
}

impl Case {
    /// `TH` or `TD` for each cell: a cell is a header cell when `w:tblLook` turns
    /// on the first row or the first column for it, and for no other reason.
    fn expected_cells(&self) -> Vec<Vec<&'static str>> {
        let (rows, columns) = self.shape.size();
        let (first_row, first_column) = match self.look {
            // Word's default for a table with no `w:tblLook` is 04A0.
            Look::Absent => (true, true),
            Look::Flags {
                first_row,
                first_column,
            } => (first_row, first_column),
        };
        (0..rows)
            .map(|r| {
                (0..columns)
                    .map(|c| {
                        if (r == 0 && first_row) || (c == 0 && first_column) {
                            "TH"
                        } else {
                            "TD"
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

impl Case {
    /// The scope that stage 4 must give each cell: `Column`, `Row`, or none for a
    /// `TD`. A `TH` in the first row is a column heading when the row has no `TD`;
    /// otherwise a `TH` in the first column is a row heading (the table has no
    /// heading that is neither).
    fn expected_scopes(&self) -> Vec<Vec<Option<&'static str>>> {
        self.expected_cells()
            .iter()
            .enumerate()
            .map(|(r, row)| {
                let heading_row = r == 0 && row.iter().all(|tag| *tag == "TH");
                row.iter()
                    .enumerate()
                    .map(|(c, tag)| match (*tag, heading_row, c) {
                        ("TD", _, _) => None,
                        (_, true, _) => Some("Column"),
                        (_, false, 0) => Some("Row"),
                        _ => panic!("{}: a heading that is in neither", self.name),
                    })
                    .collect()
            })
            .collect()
    }
}

fn cases() -> Vec<Case> {
    let flags = |first_row, first_column| Look::Flags {
        first_row,
        first_column,
    };
    let looks = [
        ("04A0", flags(true, true)),
        ("0420", flags(true, false)),
        ("0480", flags(false, true)),
        ("0400", flags(false, false)),
        ("absent", Look::Absent),
    ];
    let mut cases = Vec::new();
    // The two styles with borders, every look, with and without the repeated
    // header row.
    for style in ["TableGrid", "GridTable4"] {
        for (look_name, look) in looks {
            for repeat_header in [true, false] {
                let header = if repeat_header {
                    "header-row"
                } else {
                    "no-header-row"
                };
                cases.push(Case {
                    name: format!("{style} {look_name} {header}"),
                    shape: Shape::LabelValue,
                    style: Some(style),
                    direct_borders: false,
                    look,
                    repeat_header,
                    tagged: true,
                });
            }
        }
    }
    // What makes Word write a `Table` at all.
    for (name, style, direct_borders, tagged) in [
        ("shading only (no borders)", Some("Shaded"), false, true),
        (
            "first-column format only (no borders, no shading)",
            Some("Plain"),
            false,
            false,
        ),
        ("no style, borders in the table", None, true, true),
        ("no style, no borders", None, false, false),
    ] {
        cases.push(Case {
            name: name.to_string(),
            shape: Shape::LabelValue,
            style,
            direct_borders,
            look: flags(true, true),
            repeat_header: true,
            tagged,
        });
    }
    // The other shapes of the tables that a guide has.
    for (name, shape) in [
        ("TableGrid 0420 three columns", Shape::ThreeColumns),
        ("TableGrid 0420 headings only", Shape::HeadingsOnly),
    ] {
        cases.push(Case {
            name: name.to_string(),
            shape,
            style: Some("TableGrid"),
            direct_borders: false,
            look: flags(true, false),
            repeat_header: true,
            tagged: true,
        });
    }
    cases
}

fn document(case: &Case) -> Vec<u8> {
    let mut table = case.shape.table();
    if !case.repeat_header {
        table = table.no_header_row();
    }
    if let Some(style) = case.style {
        table = table.style(style);
    }
    let borders = if case.direct_borders { BORDERS } else { "" };
    table = match case.look {
        Look::Absent if case.direct_borders => table.look(borders),
        Look::Absent => table,
        Look::Flags {
            first_row,
            first_column,
        } => table.look(&format!("{borders}{}", look_xml(first_row, first_column))),
    };
    clean()
        .table_style("TableGrid", "Table Grid", None, TABLE_GRID)
        .table_style("GridTable4", "Grid Table 4 Accent 1", None, GRID_TABLE_4)
        .table_style("Shaded", "Shaded", None, SHADED)
        .table_style("Plain", "Plain", None, PLAIN)
        .table(table)
        .build()
}

// S5-14
#[test]
fn word_writes_th_cells_without_a_scope_and_stage_4_makes_verapdf_pass_the_table() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s5-14");
    let cases = cases();
    let files: Vec<_> = cases
        .iter()
        .enumerate()
        .map(|(i, case)| dir.write(&format!("case-{i}.docx"), &document(case)))
        .collect();
    let to_export = files.clone();
    within(Duration::from_secs(600), move |session| {
        for docx in &to_export {
            let document = session.open_read_only(docx).unwrap();
            document.update_fields().unwrap();
            document
                .export_as_fixed_format(&docx.with_extension("pdf"), &ExportParameters::SPEC)
                .unwrap();
            document.close().unwrap();
        }
    });

    let tool = Tool::locate();
    for (case, docx) in cases.iter().zip(&files) {
        let pdf = docx.with_extension("pdf");
        let tables = pdf_tables(&fs::read(&pdf).unwrap());
        // The stage 4 fix of the XMP part is not run here: leave rule 5-1 out.
        let failed: Vec<String> = validate(&tool, &pdf)
            .unwrap()
            .ua1
            .findings
            .iter()
            .map(|f| format!("{}-{}", f.clause, f.test_number))
            .filter(|rule| rule != "5-1")
            .collect();
        let summary: Vec<String> = tables.iter().map(|t| table_summary(t)).collect();
        println!("S5-14 {}: {summary:?} veraPDF {failed:?}", case.name);

        if !case.tagged {
            assert!(
                tables.is_empty(),
                "{}: Word wrote a Table for a table with no borders and no shading",
                case.name
            );
            assert!(failed.is_empty(), "{}: {failed:?}", case.name);
            continue;
        }
        assert_eq!(tables.len(), 1, "{}: {summary:?}", case.name);
        let expected = case.expected_cells();
        let got: Vec<Vec<&str>> = tables[0]
            .iter()
            .map(|row| row.iter().map(|c| c.tag.as_str()).collect())
            .collect();
        assert_eq!(got, expected, "{}: TH and TD cells", case.name);
        // Word never writes a /Scope or a /Headers.
        assert!(
            tables[0]
                .iter()
                .flatten()
                .all(|c| c.scope.is_none() && !c.headers),
            "{}: {summary:?}",
            case.name
        );
        // 7.5-1 fails when the table has a TH and a TD: the TH has no Scope.
        let has_th = expected.iter().flatten().any(|t| *t == "TH");
        let has_td = expected.iter().flatten().any(|t| *t == "TD");
        let rules: Vec<&str> = if has_th && has_td {
            vec!["7.5-1"]
        } else {
            vec![]
        };
        assert_eq!(failed, rules, "{}", case.name);

        // Stage 4 adds the scope of the `TH` cells, and veraPDF accepts the table.
        let original = fs::read(&pdf).unwrap();
        let source = SourceMetadata {
            title: Some("Adapdf test guide".to_string()),
            language: Some("en-US".to_string()),
        };
        let outcome = pdfmeta::check_bytes(&original, &source).unwrap();
        let fixed = outcome.apply(&original);
        assert!(fixed.starts_with(&original), "{}: prefix", case.name);
        assert!(outcome.report.issues.is_empty(), "{}", case.name);
        assert!(outcome.report.unscoped_headers.is_empty(), "{}", case.name);
        let headings = expected.iter().flatten().filter(|t| **t == "TH").count();
        let given: usize = outcome
            .report
            .table_scopes
            .iter()
            .map(|t| t.column + t.row)
            .sum();
        assert_eq!(given, headings, "{}: {:?}", case.name, outcome.report);
        let fixed_pdf = docx.with_extension("fixed.pdf");
        fs::write(&fixed_pdf, &fixed).unwrap();
        let fixed_tables = pdf_tables(&fixed);
        let scopes: Vec<Vec<Option<&str>>> = fixed_tables[0]
            .iter()
            .map(|row| row.iter().map(|c| c.scope.as_deref()).collect())
            .collect();
        assert_eq!(scopes, case.expected_scopes(), "{}", case.name);
        let after: Vec<String> = validate(&tool, &fixed_pdf)
            .unwrap()
            .ua1
            .findings
            .iter()
            .map(|f| format!("{}-{}", f.clause, f.test_number))
            .collect();
        let fixed_summary: Vec<String> = fixed_tables.iter().map(|t| table_summary(t)).collect();
        println!(
            "S5-14 {}: after stage 4 {fixed_summary:?} veraPDF {after:?}",
            case.name
        );
        assert!(after.is_empty(), "{}: {after:?}", case.name);
    }
    assert_eq!(wait_for_word_pids(&before), before);
}
