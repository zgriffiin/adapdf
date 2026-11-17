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
