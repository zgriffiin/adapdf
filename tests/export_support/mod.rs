//! Helpers for the export tests that need Word (`tests/export_word.rs` and
//! `tests/export_fixtures.rs`).
#![allow(dead_code)] // each test file uses a different part of this module

use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// How often `needle` is in `bytes`.
pub fn count(bytes: &[u8], needle: &[u8]) -> usize {
    bytes.windows(needle.len()).filter(|w| *w == needle).count()
}

pub fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    count(bytes, needle) > 0
}

/// Word wrote a tagged PDF: `/MarkInfo` with `/Marked true` and a
/// `/StructTreeRoot` (the check of spec 02, stage 4).
pub fn assert_tagged(name: &str, pdf: &[u8]) {
    assert!(pdf.starts_with(b"%PDF-"), "{name}: not a PDF");
    assert!(contains(pdf, b"/MarkInfo"), "{name}: no /MarkInfo");
    assert!(contains(pdf, b"/Marked true"), "{name}: not /Marked true");
    assert!(
        contains(pdf, b"/StructTreeRoot"),
        "{name}: no /StructTreeRoot"
    );
}

/// The bytes and the modified time of a file, to compare before and after.
pub fn fingerprint(path: &Path) -> (Vec<u8>, SystemTime) {
    (
        fs::read(path).unwrap(),
        fs::metadata(path).unwrap().modified().unwrap(),
    )
}

/// The names in a folder, sorted.
pub fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The same .docx with an empty `[Content_Types].xml`. The tool reads the parts
/// by name, so stage 1 accepts it; Word does not.
pub fn without_content_types(docx: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(docx)).unwrap();
    let parts: Vec<(String, Vec<u8>)> = (0..archive.len())
        .map(|i| {
            let mut entry = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            let name = entry.name().unwrap().into_owned();
            if name == "[Content_Types].xml" {
                data = br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#.to_vec();
            }
            (name, data)
        })
        .collect();
    let entries: Vec<(&str, &[u8])> = parts
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    adapdf::fixtures::zip_stored(&entries)
}

/// How many structure elements of this type (`/S`, for example `TOCI`) the tags
/// of a PDF have. The tags are in compressed object streams, so the bytes cannot
/// be searched: the PDF is read with `lopdf`.
pub fn structure_elements(pdf: &[u8], kind: &str) -> usize {
    let document = lopdf::Document::load_mem(pdf).expect("a PDF that lopdf can read");
    document
        .objects
        .values()
        .filter_map(|object| object.as_dict().ok())
        .filter(|dict| {
            dict.get(b"S")
                .and_then(lopdf::Object::as_name)
                .is_ok_and(|name| name == kind.as_bytes())
        })
        .count()
}

/// The text of all pages of a PDF, read with `lopdf` (it decodes the font maps
/// that Word writes). Words that are on a line of their own come out one per line.
pub fn page_text(pdf: &[u8]) -> String {
    let document = lopdf::Document::load_mem(pdf).expect("a PDF that lopdf can read");
    let pages: Vec<u32> = document.get_pages().keys().copied().collect();
    document
        .extract_text(&pages)
        .expect("the text of the pages")
}

/// A cell of a `Table` structure element in a PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfCell {
    /// `TH` or `TD`.
    pub tag: String,
    /// The `/Scope` of its table attributes (`Row`, `Column` or `Both`).
    pub scope: Option<String>,
    /// The cell has a `/Headers` array.
    pub headers: bool,
}

impl PdfCell {
    /// `TD`, `TH` or `TH(Row)`.
    pub fn short(&self) -> String {
        match &self.scope {
            Some(scope) => format!("{}({scope})", self.tag),
            None if self.headers => format!("{}(headers)", self.tag),
            None => self.tag.clone(),
        }
    }
}

fn resolve<'a>(
    document: &'a lopdf::Document,
    object: &'a lopdf::Object,
) -> Option<&'a lopdf::Object> {
    match object {
        lopdf::Object::Reference(id) => document.get_object(*id).ok(),
        other => Some(other),
    }
}

/// The structure elements below `object`, in document order, with the elements
/// of `stop` not entered.
fn walk_elements<'a>(
    document: &'a lopdf::Document,
    object: &'a lopdf::Object,
    out: &mut Vec<&'a lopdf::Dictionary>,
    depth: usize,
) {
    if depth > 100 {
        return;
    }
    match resolve(document, object) {
        Some(lopdf::Object::Array(items)) => {
            for item in items {
                walk_elements(document, item, out, depth + 1);
            }
        }
        Some(lopdf::Object::Dictionary(dict)) => {
            if dict.get(b"S").is_ok() {
                out.push(dict);
            }
            if let Ok(kids) = dict.get(b"K") {
                walk_elements(document, kids, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn tag_of(dict: &lopdf::Dictionary) -> String {
    dict.get(b"S")
        .and_then(lopdf::Object::as_name)
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .unwrap_or_default()
}

/// The `/Scope` in the table attributes (`/A`) of a structure element.
fn scope_of(document: &lopdf::Document, dict: &lopdf::Dictionary) -> Option<String> {
    let attributes = dict.get(b"A").ok()?;
    let mut found = Vec::new();
    let mut stack = vec![attributes];
    while let Some(object) = stack.pop() {
        match resolve(document, object) {
            Some(lopdf::Object::Array(items)) => stack.extend(items),
            Some(lopdf::Object::Dictionary(d)) => found.push(d),
            _ => {}
        }
    }
    found.into_iter().find_map(|d| {
        d.get(b"Scope")
            .and_then(lopdf::Object::as_name)
            .ok()
            .map(|n| String::from_utf8_lossy(n).into_owned())
    })
}

/// The tables of a PDF (in document order), as rows of cells: what Word wrote for
/// each `TH` and `TD`, read with `lopdf`.
pub fn pdf_tables(pdf: &[u8]) -> Vec<Vec<Vec<PdfCell>>> {
    let document = lopdf::Document::load_mem(pdf).expect("a PDF that lopdf can read");
    let root = document
        .catalog()
        .and_then(|c| c.get(b"StructTreeRoot"))
        .expect("a structure tree");
    let mut elements = Vec::new();
    walk_elements(&document, root, &mut elements, 0);
    let mut tables = Vec::new();
    let mut i = 0;
    while i < elements.len() {
        if tag_of(elements[i]) == "Table" {
            // The elements of this table are the ones that follow, up to the next
            // `Table` (a nested table is not looked for here).
            let mut rows: Vec<Vec<PdfCell>> = Vec::new();
            let mut j = i + 1;
            while j < elements.len() && tag_of(elements[j]) != "Table" {
                match tag_of(elements[j]).as_str() {
                    "TR" => rows.push(Vec::new()),
                    tag @ ("TH" | "TD") => {
                        if let Some(row) = rows.last_mut() {
                            row.push(PdfCell {
                                tag: tag.to_string(),
                                scope: scope_of(&document, elements[j]),
                                headers: elements[j].get(b"Headers").is_ok()
                                    || elements[j]
                                        .get(b"A")
                                        .is_ok_and(|a| format!("{a:?}").contains("Headers")),
                            });
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            tables.push(rows);
        }
        i += 1;
    }
    tables
}

/// A table as a short text: cells of a row are separated by a space, rows by
/// ` / `.
pub fn table_summary(table: &[Vec<PdfCell>]) -> String {
    table
        .iter()
        .map(|row| row.iter().map(PdfCell::short).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join(" / ")
}
