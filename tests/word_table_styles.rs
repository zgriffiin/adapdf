//! The model's table style colours against Word (level `windows_word`).
//!
//! Word is asked for the fill and the text colour of every cell of many test
//! tables, and the model must give the same. The tables use every kind of
//! conditional format, with a different colour in each, so the order in which
//! the formats apply is tested too. See `src/model/table_styles.rs` for what
//! the model does and why.
#![cfg(all(windows, feature = "windows_word"))]

use std::fs;

use adapdf::docx::Package;
use adapdf::fixtures::{DocxBuilder, Table, zip_stored};
use adapdf::model::Document;
use adapdf::word::WordSession;

fn shd(fill: &str) -> String {
    format!(r#"<w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="{fill}"/></w:tcPr>"#)
}

const SIZES: &str = r#"<w:tblStyleRowBandSize w:val="1"/><w:tblStyleColBandSize w:val="1"/>"#;

/// A table style where each conditional format has its own text colour and its
/// own fill. `sizes` is the XML of the band sizes in the style.
fn every_format(sizes: &str) -> String {
    let formats = [
        ("band1Horz", "00B050", "E2EFDA"),
        ("band2Horz", "BF8F00", "FFF2CC"),
        ("band1Vert", "C00000", "F8CBAD"),
        ("band2Vert", "0000CD", "BDD7EE"),
        ("lastCol", "00B0F0", "DDEBF7"),
        ("firstCol", "7030A0", "E4DFEC"),
        ("lastRow", "808000", "FFE699"),
        ("firstRow", "FFFFFF", "1F3864"),
        ("seCell", "FEDCBA", "123456"),
        ("swCell", "ABCDEF", "654321"),
        ("neCell", "FEDCBA", "ABCDEF"),
        ("nwCell", "123456", "FEDCBA"),
    ];
    let mut xml = format!(
        r#"<w:rPr><w:color w:val="111111"/></w:rPr><w:tblPr>{sizes}</w:tblPr>{}"#,
        shd("EEEEEE")
    );
    for (name, colour, fill) in formats {
        xml += &format!(
            r#"<w:tblStylePr w:type="{name}"><w:rPr><w:color w:val="{colour}"/></w:rPr>{}</w:tblStylePr>"#,
            shd(fill)
        );
    }
    xml
}

/// A style with only the first row and the row bands, as fills.
fn rows_only() -> String {
    format!(
        r#"<w:tblPr>{SIZES}</w:tblPr><w:tblStylePr w:type="firstRow">{}</w:tblStylePr><w:tblStylePr w:type="band1Horz">{}</w:tblStylePr><w:tblStylePr w:type="band2Horz">{}</w:tblStylePr>"#,
        shd("FF0000"),
        shd("00FF00"),
        shd("0000FF")
    )
}

fn look(first_row: u8, last_row: u8, first_col: u8, last_col: u8) -> String {
    format!(
        r#"<w:tblLook w:firstRow="{first_row}" w:lastRow="{last_row}" w:firstColumn="{first_col}" w:lastColumn="{last_col}" w:noHBand="0" w:noVBand="0"/>"#
    )
}

fn grid(rows: usize, cols: usize) -> Table {
    let text: Vec<Vec<String>> = (0..rows)
        .map(|r| (0..cols).map(|c| format!("r{r}c{c}")).collect())
        .collect();
    let refs: Vec<Vec<&str>> = text
        .iter()
        .map(|row| row.iter().map(String::as_str).collect())
        .collect();
    let rows: Vec<&[&str]> = refs.iter().map(Vec::as_slice).collect();
    Table::new(&rows)
}

/// A test table: its name, and the number of cells in each row.
struct Case {
    name: &'static str,
    widths: Vec<usize>,
}

/// One document with a table for every case. The first row of a table is the
/// header row only when the case says so.
fn the_tables() -> (Vec<u8>, Vec<Case>) {
    let mut builder = DocxBuilder::new()
        .table_style("Every", "Every", None, &every_format(SIZES))
        .table_style("Every2", "Every2", None, &every_format(&SIZES.replace('1', "2")))
        .table_style("NoSizes", "NoSizes", None, &every_format(""))
        .table_style("Rows", "Rows", None, &rows_only())
        .table_style(
            "Derived",
            "Derived",
            Some("Every"),
            &format!(
                r#"<w:tblStylePr w:type="firstRow">{}</w:tblStylePr><w:tblStylePr w:type="band1Horz">{}</w:tblStylePr>"#,
                shd("FFFF00"),
                shd("00FFFF")
            ),
        );
    let mut cases = Vec::new();
    let mut add = |name: &'static str, table: Table| {
        let widths = table_widths(&table);
        builder = std::mem::take(&mut builder).para("-").table(table);
        cases.push(Case { name, widths });
    };

    let all = look(1, 1, 1, 1);
    for (name, rows, cols) in [
        ("every format, 5x5", 5, 5),
        ("every format, 1x1", 1, 1),
        ("every format, 1x3", 1, 3),
        ("every format, 3x1", 3, 1),
        ("every format, 2x2", 2, 2),
    ] {
        add(name, grid(rows, cols).style("Every").look(&all));
    }
    for (name, l) in [
        ("rows only", look(1, 1, 0, 0)),
        ("columns only", look(0, 0, 1, 1)),
        ("first row and first column", look(1, 0, 1, 0)),
        ("last row and last column", look(0, 1, 0, 1)),
        ("first row and last column", look(1, 0, 0, 1)),
        ("last row and first column", look(0, 1, 1, 0)),
        ("nothing", look(0, 0, 0, 0)),
        ("legacy 06E0", r#"<w:tblLook w:val="06E0"/>"#.to_string()),
        ("legacy 0000", r#"<w:tblLook w:val="0000"/>"#.to_string()),
        (
            "one attribute, the rest missing",
            r#"<w:tblLook w:val="0000" w:firstRow="1"/>"#.to_string(),
        ),
    ] {
        add(name, grid(4, 4).style("Every").look(&l));
    }
    add("no tblLook", grid(4, 4).style("Every"));
    add("band size 2", grid(7, 6).style("Every2").look(&all));
    add("no band sizes", grid(5, 5).style("NoSizes").look(&all));
    add(
        "band sizes in the table",
        grid(5, 5).style("NoSizes").look(&all).band_sizes(1, 1),
    );
    add(
        "band sizes in the table beat the style's",
        grid(7, 6).style("Every").look(&all).band_sizes(2, 2),
    );
    for (name, headers) in [
        ("header rows 0", vec![0]),
        ("header rows 0 and 1", vec![0, 1]),
        ("header row 1 only", vec![1]),
        ("header rows 0, 1 and 3", vec![0, 1, 3]),
        ("header rows 0 and 2", vec![0, 2]),
    ] {
        let mut table = grid(6, 2)
            .style("Rows")
            .look(&look(1, 0, 0, 0))
            .raw_row_properties(0, "");
        for row in headers {
            table = table.raw_row_properties(row, "<w:tblHeader/>");
        }
        add(name, table);
    }
    add(
        "cells that span columns",
        Table::new(&[
            &["a", "b", "c", "d"],
            &["e", "f", "g"],
            &["h", "i", "j"],
            &["k", "l", "m", "n"],
        ])
        .style("Every")
        .look(&all)
        .span(1, 0, 2)
        .span(2, 2, 2),
    );
    add(
        "cell fill set in the cell",
        grid(4, 3)
            .style("Every")
            .look(&look(1, 0, 1, 0))
            .cell_fill(0, 1, "00FF00")
            .cell_fill(2, 0, "FFFF00"),
    );
    add(
        "a style based on another",
        grid(5, 3).style("Derived").look(&look(1, 0, 1, 0)),
    );
    (builder.build(), cases)
}

/// The number of cells in each row of a table, from its XML.
fn table_widths(table: &Table) -> Vec<usize> {
    let xml = table.xml();
    xml.split("<w:tr>")
        .skip(1)
        .map(|row| row.matches("<w:tc>").count())
        .collect()
}

/// A document where paragraph styles set a text colour, in a table whose
/// style sets another. Word lets the paragraph style win.
fn paragraph_styles() -> (Vec<u8>, Vec<Case>) {
    let cell = |style: &str, text: &str| {
        format!(
            r#"<w:p><w:pPr><w:pStyle w:val="{style}"/></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
        )
    };
    let bytes = DocxBuilder::new()
        .custom_style("Coloured", "Coloured", None)
        .table_style("Every", "Every", None, &every_format(SIZES))
        .table(
            grid(3, 2)
                .style("Every")
                .look(&look(1, 0, 1, 0))
                .raw_cell(0, 0, &cell("Coloured", "r0c0"))
                .raw_cell(1, 1, &cell("Coloured", "r1c1"))
                .raw_cell(2, 0, &cell("Coloured", "r2c0")),
        )
        .build();
    let package = Package::from_bytes(&bytes).unwrap();
    let parts: Vec<(String, Vec<u8>)> = package
        .parts()
        .iter()
        .map(|part| {
            let mut data = part.bytes.clone();
            if part.name == "word/styles.xml" {
                let mut text = String::from_utf8(data).unwrap();
                for (from, to) in [
                    (
                        r#"<w:name w:val="Normal"/></w:style>"#,
                        r#"<w:name w:val="Normal"/><w:rPr><w:color w:val="AA0000"/></w:rPr></w:style>"#,
                    ),
                    (
                        r#"<w:name w:val="Coloured"/><w:basedOn w:val="Normal"/></w:style>"#,
                        r#"<w:name w:val="Coloured"/><w:basedOn w:val="Normal"/><w:rPr><w:color w:val="00AA00"/></w:rPr></w:style>"#,
                    ),
                ] {
                    assert!(text.contains(from), "{from}");
                    text = text.replacen(from, to, 1);
                }
                data = text.into_bytes();
            }
            (part.name.clone(), data)
        })
        .collect();
    let entries: Vec<(&str, &[u8])> = parts
        .iter()
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();
    (
        zip_stored(&entries),
        vec![Case {
            name: "paragraph styles over the table style",
            widths: vec![2, 2, 2],
        }],
    )
}

fn mismatches(
    session: &WordSession,
    bytes: &[u8],
    cases: &[Case],
    dir: &std::path::Path,
) -> Vec<String> {
    let path = dir.join("tables.docx");
    fs::write(&path, bytes).unwrap();
    let model = Document::from_bytes(bytes).unwrap();
    let in_tables: Vec<_> = model.paragraphs.iter().filter(|p| p.in_table).collect();
    let word = session.open_read_only(&path).expect("Word opens the file");

    let mut found = Vec::new();
    let mut next = 0;
    let mut cells = 0;
    for (table, case) in cases.iter().enumerate() {
        for (row, width) in case.widths.iter().enumerate() {
            for column in 0..*width {
                let paragraph = in_tables[next];
                next += 1;
                cells += 1;
                let expected = (
                    paragraph.container_fill,
                    paragraph.runs[0].props.color.rgb(),
                );
                let shown = word.cell_colours(table, row, column).unwrap();
                if (shown.fill, shown.text) != expected {
                    found.push(format!(
                        "{}: row {row}, cell {column}: Word shows fill {:?} and text {:?}; the model has fill {:?} and text {:?}",
                        case.name, shown.fill, shown.text, expected.0, expected.1
                    ));
                }
            }
        }
    }
    assert_eq!(
        next,
        in_tables.len(),
        "every paragraph in a table was compared"
    );
    println!("compared {cells} cells in {} tables", cases.len());
    word.close().unwrap();
    found
}

#[test]
fn table_style_colours_match_word() {
    let dir = std::env::temp_dir().join(format!("adapdf-word-tables-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let session = WordSession::start().expect("start Word");
    println!("Word build {}", session.build().unwrap());
    let mut found = Vec::new();
    for (bytes, cases) in [the_tables(), paragraph_styles()] {
        found.extend(mismatches(&session, &bytes, &cases, &dir));
    }
    drop(session);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        found.is_empty(),
        "{} cells differ:\n{}",
        found.len(),
        found.join("\n")
    );
}
