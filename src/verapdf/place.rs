//! Where a veraPDF failure is, in words.
//!
//! veraPDF names the first place that failed a rule as a path through the PDF's
//! structure tree, for example
//! `root/document[0]/StructTreeRoot[0](34 0 obj PDStructTreeRoot)/K[0](38 0 obj SEDocument Document)/K[7](68 0 obj SETable Table)/K[1](75 0 obj SETBody TBody)/K[0](76 0 obj SETR TR)/K[1](79 0 obj SETD TD)`.
//! A person who works in Word cannot use that. [`describe`] turns it into
//! `Table 1, body row 1, cell 2`, `Figure 3` or `Link 5`.
//!
//! The path says the row and the cell (the `K[i]` index of each element), but
//! not which table or which figure it is. For that the numbers come from the
//! structure tree of the PDF ([`StructIndex`]), counted in document order. The
//! numbers are counted in the PDF, which has the same order as the Word
//! document. When the PDF cannot be read, the words stay without a number. The
//! raw path is always kept next to the words (`VeraFinding::context`).

use std::collections::HashSet;
use std::path::Path;

use lopdf::{Document, Object, ObjectId};

use super::VeraFinding;

/// The structure elements of a PDF in document order: the object number and the
/// tag (`Table`, `Figure`, `H2`, ...) of each.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructIndex {
    elements: Vec<(u32, String)>,
}

/// How deep the walk of the structure tree goes, so that a damaged PDF cannot
/// use all the stack.
const MAX_DEPTH: usize = 200;

impl StructIndex {
    pub fn new(elements: Vec<(u32, String)>) -> StructIndex {
        StructIndex { elements }
    }

    /// Reads the structure tree of the PDF. `None` when the PDF cannot be read or
    /// has no structure tree.
    pub fn from_pdf(path: &Path) -> Option<StructIndex> {
        let doc = Document::load(path).ok()?;
        let root = doc.catalog().ok()?.get(b"StructTreeRoot").ok()?;
        let mut elements = Vec::new();
        let mut seen = HashSet::new();
        walk(&doc, root, &mut elements, &mut seen, 0);
        Some(StructIndex { elements })
    }

    /// The position of the element among those of its kind (1 for the first),
    /// when the PDF has it.
    fn ordinal(&self, object: u32, class: &str) -> Option<usize> {
        let mut count = 0;
        for (id, tag) in &self.elements {
            if class_of(tag) == class {
                count += 1;
                if *id == object {
                    return Some(count);
                }
            }
        }
        None
    }
}

fn walk(
    doc: &Document,
    node: &Object,
    out: &mut Vec<(u32, String)>,
    seen: &mut HashSet<ObjectId>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let (id, object) = match node {
        Object::Reference(id) => {
            if !seen.insert(*id) {
                return;
            }
            match doc.get_object(*id) {
                Ok(object) => (Some(*id), object),
                Err(_) => return,
            }
        }
        other => (None, other),
    };
    match object {
        Object::Array(items) => {
            for item in items {
                walk(doc, item, out, seen, depth + 1);
            }
        }
        Object::Dictionary(dict) => {
            if let (Some(id), Ok(tag)) = (id, dict.get(b"S").and_then(|o| o.as_name())) {
                out.push((id.0, String::from_utf8_lossy(tag).into_owned()));
            }
            if let Ok(kids) = dict.get(b"K") {
                walk(doc, kids, out, seen, depth + 1);
            }
        }
        _ => {}
    }
}

/// Tags that are counted together: all heading levels are headings.
fn class_of(tag: &str) -> &str {
    match tag {
        "H" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6" => "H",
        other => other,
    }
}

/// One element of a veraPDF path that is a structure element.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Segment {
    /// The `K[i]` index: the place among the children of the parent.
    index: usize,
    object: u32,
    tag: String,
}

/// The structure elements of a veraPDF path, root first. The parts of the path
/// that are not structure elements (`root`, `document[0]`, `StructTreeRoot`)
/// are left out.
fn segments(context: &str) -> Vec<Segment> {
    context
        .split('/')
        .filter_map(|part| {
            let (head, rest) = part.split_once('(')?;
            let inner = rest.strip_suffix(')')?;
            let index = head
                .strip_prefix("K[")?
                .strip_suffix(']')?
                .parse::<usize>()
                .ok()?;
            // `68 0 obj SETable Table`: number, generation, `obj`, veraPDF's type
            // and the tag.
            let words: Vec<&str> = inner.split_whitespace().collect();
            if words.len() < 5 || words[2] != "obj" || !words[3].starts_with("SE") {
                return None;
            }
            Some(Segment {
                index,
                object: words[0].parse().ok()?,
                tag: words[words.len() - 1].to_string(),
            })
        })
        .collect()
}

fn numbered(word: &str, number: Option<usize>) -> String {
    match number {
        Some(n) => format!("{word} {n}"),
        None => format!("A {}", word.to_lowercase()),
    }
}

/// The place in words, or `None` when the path does not name a kind of element
/// that this function knows.
pub fn describe(context: &str, index: Option<&StructIndex>) -> Option<String> {
    let segs = segments(context);
    let last = segs.last()?;
    let ordinal = |seg: &Segment| index.and_then(|i| i.ordinal(seg.object, class_of(&seg.tag)));
    match last.tag.as_str() {
        "Table" => Some(numbered("Table", ordinal(last))),
        "TR" | "TD" | "TH" | "THead" | "TBody" | "TFoot" => {
            let table_at = segs.iter().rposition(|s| s.tag == "Table")?;
            let mut text = numbered("Table", ordinal(&segs[table_at]));
            let below = &segs[table_at + 1..];
            let group = below
                .iter()
                .find(|s| matches!(s.tag.as_str(), "THead" | "TBody" | "TFoot"));
            let row = below.iter().find(|s| s.tag == "TR");
            let cell = below.iter().find(|s| matches!(s.tag.as_str(), "TD" | "TH"));
            if let Some(row) = row {
                let kind = match group.map(|g| g.tag.as_str()) {
                    Some("THead") => "header row",
                    Some("TFoot") => "footer row",
                    Some(_) => "body row",
                    None => "row",
                };
                text.push_str(&format!(", {kind} {}", row.index + 1));
            } else if let Some(group) = group {
                text.push_str(match group.tag.as_str() {
                    "THead" => ", the header rows",
                    "TFoot" => ", the footer rows",
                    _ => ", the body rows",
                });
            }
            if let Some(cell) = cell {
                text.push_str(&format!(", cell {}", cell.index + 1));
            }
            Some(text)
        }
        "Figure" => Some(numbered("Figure", ordinal(last))),
        "Link" => Some(numbered("Link", ordinal(last))),
        "H" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6" => Some(format!(
            "{} ({})",
            numbered("Heading", ordinal(last)),
            last.tag
        )),
        "P" => Some(numbered("Paragraph", ordinal(last))),
        "L" => Some(numbered("List", ordinal(last))),
        "LI" | "LBody" | "Lbl" => {
            let list_at = segs.iter().rposition(|s| s.tag == "L")?;
            let item = segs[list_at + 1..].iter().find(|s| s.tag == "LI")?;
            Some(format!(
                "{}, item {}",
                numbered("List", ordinal(&segs[list_at])),
                item.index + 1
            ))
        }
        _ => None,
    }
}

/// Puts the words of the place in each finding that has a path.
pub fn add_places(findings: &mut [VeraFinding], index: Option<&StructIndex>) {
    for finding in findings {
        finding.place = finding
            .context
            .as_deref()
            .and_then(|context| describe(context, index));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path of the first failure in a real report (a cell of a table).
    const CELL: &str = "root/document[0]/StructTreeRoot[0](34 0 obj PDStructTreeRoot)/K[0](38 0 obj SEDocument Document)/K[7](68 0 obj SETable Table)/K[1](75 0 obj SETBody TBody)/K[0](76 0 obj SETR TR)/K[1](79 0 obj SETD TD)";
    const FIGURE: &str = "root/document[0]/StructTreeRoot[0](4 0 obj PDStructTreeRoot)/K[0](5 0 obj SEDocument Document)/K[0](6 0 obj SEFigure Figure)";

    fn tree() -> StructIndex {
        StructIndex::new(
            [
                (38, "Document"),
                (50, "H1"),
                (52, "Figure"),
                (60, "Table"),
                (68, "Table"),
                (75, "TBody"),
                (76, "TR"),
                (79, "TD"),
                (90, "H2"),
                (91, "Link"),
                (92, "Link"),
                (95, "L"),
                (96, "LI"),
                (110, "H3"),
                (111, "P"),
            ]
            .into_iter()
            .map(|(id, tag)| (id, tag.to_string()))
            .collect(),
        )
    }

    #[test]
    fn a_table_cell_has_its_table_row_and_cell() {
        assert_eq!(
            describe(CELL, Some(&tree())).as_deref(),
            Some("Table 2, body row 1, cell 2")
        );
        // Without the PDF the table has no number.
        assert_eq!(
            describe(CELL, None).as_deref(),
            Some("A table, body row 1, cell 2")
        );
    }

    #[test]
    fn a_header_row_and_a_row_with_no_group() {
        let header = "root/K[0](38 0 obj SEDocument Document)/K[0](68 0 obj SETable Table)/K[0](70 0 obj SETHead THead)/K[2](71 0 obj SETR TR)/K[0](72 0 obj SETH TH)";
        assert_eq!(
            describe(header, Some(&tree())).as_deref(),
            Some("Table 2, header row 3, cell 1")
        );
        let plain = "root/K[0](38 0 obj SEDocument Document)/K[0](68 0 obj SETable Table)/K[3](71 0 obj SETR TR)";
        assert_eq!(
            describe(plain, Some(&tree())).as_deref(),
            Some("Table 2, row 4")
        );
        let group_only = "root/K[0](38 0 obj SEDocument Document)/K[0](68 0 obj SETable Table)/K[1](75 0 obj SETBody TBody)";
        assert_eq!(
            describe(group_only, Some(&tree())).as_deref(),
            Some("Table 2, the body rows")
        );
    }

    #[test]
    fn a_table_itself() {
        let table = "root/K[0](38 0 obj SEDocument Document)/K[7](60 0 obj SETable Table)";
        assert_eq!(describe(table, Some(&tree())).as_deref(), Some("Table 1"));
    }

    #[test]
    fn figures_links_headings_paragraphs_and_lists_are_counted_by_kind() {
        let tree = tree();
        assert_eq!(describe(FIGURE, None).as_deref(), Some("A figure"));
        let at = |object: u32, tag: &str, type_name: &str| {
            format!(
                "root/K[0](38 0 obj SEDocument Document)/K[4]({object} 0 obj SE{type_name} {tag})"
            )
        };
        assert_eq!(
            describe(&at(52, "Figure", "Figure"), Some(&tree)).as_deref(),
            Some("Figure 1")
        );
        assert_eq!(
            describe(&at(92, "Link", "Link"), Some(&tree)).as_deref(),
            Some("Link 2")
        );
        // All heading levels are counted together: H1, H2, H3.
        assert_eq!(
            describe(&at(110, "H3", "H3"), Some(&tree)).as_deref(),
            Some("Heading 3 (H3)")
        );
        assert_eq!(
            describe(&at(111, "P", "P"), Some(&tree)).as_deref(),
            Some("Paragraph 1")
        );
        let item =
            "root/K[0](38 0 obj SEDocument Document)/K[9](95 0 obj SEL L)/K[2](96 0 obj SELI LI)";
        assert_eq!(
            describe(item, Some(&tree)).as_deref(),
            Some("List 1, item 3")
        );
    }

    #[test]
    fn an_element_that_the_pdf_does_not_have_has_no_number() {
        let missing = "root/K[0](38 0 obj SEDocument Document)/K[1](999 0 obj SEFigure Figure)";
        assert_eq!(
            describe(missing, Some(&tree())).as_deref(),
            Some("A figure")
        );
    }

    #[test]
    fn paths_that_are_not_structure_elements_have_no_words() {
        for context in [
            "root",
            "root/document[0]/metadata[0](6 0 obj PDMetadata)/XMPPackage[0]",
            "root/document[0]/StructTreeRoot[0](4 0 obj PDStructTreeRoot)",
            // A kind of element that has no words yet.
            "root/K[0](5 0 obj SEDocument Document)/K[1](9 0 obj SESect Sect)",
            "",
        ] {
            assert_eq!(describe(context, Some(&tree())), None, "{context}");
        }
    }

    #[test]
    fn the_places_are_put_in_the_findings() {
        let report = crate::verapdf::parse(
            include_str!("../../tests/data/verapdf/ua1-figure-without-alt.json"),
            crate::verapdf::Profile::Ua1,
        )
        .unwrap();
        let mut findings = report.findings;
        add_places(&mut findings, None);
        assert_eq!(findings[0].place.as_deref(), Some("A figure"));
        assert!(findings[0].context.as_deref().unwrap().starts_with("root/"));
    }
}
