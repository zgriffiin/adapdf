//! Small PDFs for the stage 4 and stage 5 tests, made with `lopdf`.
//!
//! They are made up for the tests. No real document is used.
#![allow(dead_code)] // each test file uses a different part of this module

use lopdf::{Dictionary, Document, IncrementalDocument, Object, ObjectId, Stream, dictionary};

pub const TITLE: &str = "Adapdf test guide";

/// What the PDF has. The default is a tagged PDF with none of the metadata that
/// stage 4 checks.
#[derive(Clone)]
pub struct Pdf {
    /// `/MarkInfo /Marked true` in the catalog.
    pub marked: bool,
    /// A `/StructTreeRoot` with a `Document` element.
    pub struct_tree: bool,
    /// The text of the XMP packet. `None`: no `/Metadata`.
    pub xmp: Option<String>,
    /// The Info dictionary, with this `Title` (`Some("")` is an empty title).
    pub info_title: Option<String>,
    /// An Info dictionary that has a `Producer` and no `Title`.
    pub info_without_title: bool,
    /// The catalog's `/Lang`.
    pub lang: Option<String>,
    /// `/ViewerPreferences` with this `/DisplayDocTitle`. `None`: no `/ViewerPreferences`.
    pub display_doc_title: Option<bool>,
    /// `/ViewerPreferences` is an indirect object.
    pub prefs_indirect: bool,
    /// `/ViewerPreferences` has another key as well (`/HideToolbar`).
    pub prefs_other_key: bool,
    /// Object streams and a cross-reference stream (PDF 1.5), as Word writes.
    pub modern: bool,
    /// Text on the page, in a font that is not embedded. veraPDF fails this.
    pub unembedded_text: bool,
    /// A tagged `Figure` on the page. `Some(true)` has alternative text,
    /// `Some(false)` has none (veraPDF PDF/UA-1 rule 7.3-1 fails).
    pub figure: Option<bool>,
    /// Tables in the structure tree, after the figure, in this order.
    pub tables: Vec<TableSpec>,
    /// `/RoleMap` of the structure tree: a custom tag and the tag it stands for.
    pub role_map: Vec<(String, String)>,
}

/// A table of the structure tree: `Table`, and in it `THead`, `TBody` and
/// `TFoot` with `TR` rows, and `TR` rows that are directly in the table. The rows
/// are lists of cells, written as text (see [`TableSpec::cells`]).
#[derive(Clone)]
pub struct TableSpec {
    pub table_tag: String,
    pub row_tag: String,
    pub head: Vec<Vec<Cell>>,
    pub body: Vec<Vec<Cell>>,
    pub foot: Vec<Vec<Cell>>,
    /// Rows that are in no group.
    pub rows: Vec<Vec<Cell>>,
}

/// How the `/A` of a cell is written.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Attributes {
    /// No `/A`.
    None,
    /// `/A << /O /Table /RowSpan 1 >>`, a dictionary with no `/Scope`.
    Dict,
    /// `/A [ << /O /Table /RowSpan 1 >> ]`.
    Array,
    /// `/A 9 0 R`, an indirect dictionary with no `/Scope`.
    Indirect,
}

#[derive(Clone)]
pub struct Cell {
    pub tag: String,
    pub attributes: Attributes,
    /// A `/Scope` in the attributes (`Row`, `Column` or `Both`).
    pub scope: Option<String>,
}

impl TableSpec {
    pub fn new() -> TableSpec {
        TableSpec {
            table_tag: "Table".to_string(),
            row_tag: "TR".to_string(),
            head: Vec::new(),
            body: Vec::new(),
            foot: Vec::new(),
            rows: Vec::new(),
        }
    }

    /// A row of cells. Each is its tag, then `+` (a `/A` dictionary without a
    /// scope), `++` (the same in an array) or `&` (the same, indirect), then `:`
    /// and a scope. For example `TH`, `TD`, `TH+`, `TH:Row`, `Heading`.
    pub fn cells(row: &str) -> Vec<Cell> {
        row.split_whitespace()
            .map(|word| {
                let (word, scope) = match word.split_once(':') {
                    Some((word, scope)) => (word, Some(scope.to_string())),
                    None => (word, None),
                };
                let (tag, attributes) = if let Some(tag) = word.strip_suffix("++") {
                    (tag, Attributes::Array)
                } else if let Some(tag) = word.strip_suffix('+') {
                    (tag, Attributes::Dict)
                } else if let Some(tag) = word.strip_suffix('&') {
                    (tag, Attributes::Indirect)
                } else {
                    (word, Attributes::None)
                };
                Cell {
                    tag: tag.to_string(),
                    attributes,
                    scope,
                }
            })
            .collect()
    }

    pub fn head(mut self, rows: &[&str]) -> TableSpec {
        self.head = rows.iter().map(|r| TableSpec::cells(r)).collect();
        self
    }

    pub fn body(mut self, rows: &[&str]) -> TableSpec {
        self.body = rows.iter().map(|r| TableSpec::cells(r)).collect();
        self
    }

    pub fn foot(mut self, rows: &[&str]) -> TableSpec {
        self.foot = rows.iter().map(|r| TableSpec::cells(r)).collect();
        self
    }

    pub fn rows(mut self, rows: &[&str]) -> TableSpec {
        self.rows = rows.iter().map(|r| TableSpec::cells(r)).collect();
        self
    }
}

impl Default for TableSpec {
    fn default() -> TableSpec {
        TableSpec::new()
    }
}

impl Default for Pdf {
    fn default() -> Pdf {
        Pdf {
            marked: true,
            struct_tree: true,
            xmp: None,
            info_title: None,
            info_without_title: false,
            lang: None,
            display_doc_title: None,
            prefs_indirect: false,
            prefs_other_key: false,
            modern: false,
            unembedded_text: false,
            figure: None,
            tables: Vec::new(),
            role_map: Vec::new(),
        }
    }
}

/// An XMP packet like the one a PDF producer writes, with other content.
pub fn xmp_packet(extra: &str) -> String {
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
<rdf:Description rdf:about=\"uuid:7a3e0c52-0000-4000-8000-000000000001\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\
<xmp:CreatorTool>Adapdf test maker</xmp:CreatorTool></rdf:Description>\n{extra}\
</rdf:RDF></x:xmpmeta>\n<?xpacket end=\"w\"?>"
    )
}

pub const PART_1: &str = "<rdf:Description rdf:about=\"uuid:7a3e0c52-0000-4000-8000-000000000001\" \
xmlns:pdfuaid=\"http://www.aiim.org/pdfua/ns/id/\"><pdfuaid:part>1</pdfuaid:part></rdf:Description>\n";
pub const PART_2: &str = "<rdf:Description rdf:about=\"uuid:7a3e0c52-0000-4000-8000-000000000001\" \
xmlns:pdfuaid=\"http://www.aiim.org/pdfua/ns/id/\"><pdfuaid:part>2</pdfuaid:part></rdf:Description>\n";

pub fn xmp_title(title: &str) -> String {
    format!(
        "<rdf:Description rdf:about=\"uuid:7a3e0c52-0000-4000-8000-000000000001\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title><rdf:Alt>\
<rdf:li xml:lang=\"x-default\">{title}</rdf:li></rdf:Alt></dc:title></rdf:Description>\n"
    )
}

impl Pdf {
    /// A PDF that has every value stage 4 checks.
    pub fn complete() -> Pdf {
        Pdf {
            xmp: Some(xmp_packet(&format!("{PART_1}{}", xmp_title(TITLE)))),
            info_title: Some(TITLE.to_string()),
            lang: Some("en-US".to_string()),
            display_doc_title: Some(true),
            ..Pdf::default()
        }
    }

    /// A PDF that is missing every value, and has other XMP content and an Info
    /// dictionary (so a fix has existing objects to rewrite).
    pub fn all_missing() -> Pdf {
        Pdf {
            xmp: Some(xmp_packet("")),
            info_without_title: true,
            ..Pdf::default()
        }
    }

    pub fn build(&self) -> Vec<u8> {
        let mut doc = Document::with_version(if self.modern { "1.7" } else { "1.4" });
        let pages_id = doc.new_object_id();
        let content = if self.unembedded_text {
            b"BT /F1 12 Tf 20 100 Td (Hello) Tj ET".to_vec()
        } else if self.figure.is_some() {
            b"/Figure <</MCID 0>> BDC 0 g 20 20 50 50 re f EMC".to_vec()
        } else {
            b"q Q".to_vec()
        };
        let content_id = doc.add_object(Stream::new(Dictionary::new(), content));
        let mut page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
            "Contents" => content_id,
            "Resources" => Dictionary::new(),
        };
        if self.figure.is_some() {
            page.set("StructParents", 0);
        }
        if self.unembedded_text {
            let font_id = doc.add_object(dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Helvetica",
            });
            page.set(
                "Resources",
                dictionary! { "Font" => dictionary! { "F1" => font_id } },
            );
        }
        let page_id = doc.add_object(page);
        doc.set_object(
            pages_id,
            dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            },
        );

        let mut catalog = dictionary! { "Type" => "Catalog", "Pages" => pages_id };
        if self.marked {
            catalog.set("MarkInfo", dictionary! { "Marked" => true });
        }
        if self.struct_tree {
            let tree_id = doc.new_object_id();
            let element_id = doc.new_object_id();
            let mut tree = dictionary! { "Type" => "StructTreeRoot", "K" => element_id };
            let mut kids = Vec::<Object>::new();
            if let Some(has_alt) = self.figure {
                let mut figure = dictionary! {
                    "Type" => "StructElem",
                    "S" => "Figure",
                    "P" => element_id,
                    "Pg" => page_id,
                    "K" => dictionary! { "Type" => "MCR", "Pg" => page_id, "MCID" => 0 },
                };
                if has_alt {
                    figure.set("Alt", Object::string_literal("A blue square"));
                }
                let figure_id = doc.add_object(figure);
                kids.push(figure_id.into());
                tree.set("ParentTreeNextKey", 1);
                tree.set(
                    "ParentTree",
                    dictionary! {
                        "Nums" => vec![0.into(), Object::Array(vec![figure_id.into()])],
                    },
                );
            }
            for table in &self.tables {
                kids.push(add_table(&mut doc, table, element_id).into());
            }
            doc.set_object(
                element_id,
                dictionary! {
                    "Type" => "StructElem",
                    "S" => "Document",
                    "P" => tree_id,
                    "K" => kids,
                },
            );
            if !self.role_map.is_empty() {
                let mut map = Dictionary::new();
                for (custom, standard) in &self.role_map {
                    map.set(custom.as_str(), Object::Name(standard.as_bytes().to_vec()));
                }
                tree.set("RoleMap", map);
            }
            doc.set_object(tree_id, tree);
            catalog.set("StructTreeRoot", tree_id);
        }
        if let Some(xmp) = &self.xmp {
            let stream = Stream::new(
                dictionary! { "Type" => "Metadata", "Subtype" => "XML" },
                xmp.clone().into_bytes(),
            );
            let id = doc.add_object(stream);
            catalog.set("Metadata", id);
        }
        if let Some(lang) = &self.lang {
            catalog.set("Lang", Object::string_literal(lang.as_bytes()));
        }
        if let Some(display) = self.display_doc_title {
            let mut prefs = dictionary! { "DisplayDocTitle" => display };
            if self.prefs_other_key {
                prefs.set("HideToolbar", true);
            }
            if self.prefs_indirect {
                let id = doc.add_object(prefs);
                catalog.set("ViewerPreferences", id);
            } else {
                catalog.set("ViewerPreferences", prefs);
            }
        }
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", catalog_id);

        if self.info_title.is_some() || self.info_without_title {
            let mut info =
                dictionary! { "Producer" => Object::string_literal("Adapdf test maker") };
            if let Some(title) = &self.info_title {
                info.set("Title", Object::string_literal(title.as_bytes()));
            }
            let info_id = doc.add_object(info);
            doc.trailer.set("Info", info_id);
        }
        doc.trailer.set(
            "ID",
            Object::Array(vec![
                Object::string_literal(b"adapdf-test-id-0001".to_vec()),
                Object::string_literal(b"adapdf-test-id-0001".to_vec()),
            ]),
        );

        let mut bytes = Vec::new();
        if self.modern {
            doc.save_modern(&mut bytes).expect("the PDF can be written");
        } else {
            doc.save_to(&mut bytes).expect("the PDF can be written");
        }
        bytes
    }
}

/// One structure element, as an object with a number.
fn add_element(doc: &mut Document, tag: &str, parent: ObjectId) -> ObjectId {
    doc.add_object(dictionary! {
        "Type" => "StructElem",
        "S" => Object::Name(tag.as_bytes().to_vec()),
        "P" => parent,
    })
}

fn set_kids(doc: &mut Document, id: ObjectId, kids: Vec<Object>) {
    if let Ok(Object::Dictionary(element)) = doc.get_object_mut(id) {
        element.set("K", kids);
    }
}

fn add_cell(doc: &mut Document, cell: &Cell, parent: ObjectId) -> ObjectId {
    let id = add_element(doc, &cell.tag, parent);
    if cell.attributes == Attributes::None && cell.scope.is_none() {
        return id;
    }
    let mut attributes = dictionary! { "O" => "Table" };
    if cell.attributes != Attributes::None {
        attributes.set("RowSpan", 1);
    }
    if let Some(scope) = &cell.scope {
        attributes.set("Scope", Object::Name(scope.as_bytes().to_vec()));
    }
    let value = match cell.attributes {
        Attributes::Array => Object::Array(vec![attributes.into()]),
        Attributes::Indirect => Object::Reference(doc.add_object(attributes)),
        Attributes::None | Attributes::Dict => Object::Dictionary(attributes),
    };
    if let Ok(Object::Dictionary(element)) = doc.get_object_mut(id) {
        element.set("A", value);
    }
    id
}

fn add_rows(
    doc: &mut Document,
    rows: &[Vec<Cell>],
    row_tag: &str,
    parent: ObjectId,
) -> Vec<Object> {
    rows.iter()
        .map(|cells| {
            let row = add_element(doc, row_tag, parent);
            let kids: Vec<Object> = cells
                .iter()
                .map(|cell| add_cell(doc, cell, row).into())
                .collect();
            set_kids(doc, row, kids);
            row.into()
        })
        .collect()
}

/// A table of the structure tree. The elements get their numbers in document
/// order.
fn add_table(doc: &mut Document, spec: &TableSpec, parent: ObjectId) -> ObjectId {
    let table = add_element(doc, &spec.table_tag, parent);
    let mut kids: Vec<Object> = Vec::new();
    for (tag, rows) in [
        ("THead", &spec.head),
        ("TBody", &spec.body),
        ("TFoot", &spec.foot),
    ] {
        if rows.is_empty() {
            continue;
        }
        let group = add_element(doc, tag, table);
        let group_rows = add_rows(doc, rows, &spec.row_tag, group);
        set_kids(doc, group, group_rows);
        kids.push(group.into());
    }
    kids.extend(add_rows(doc, &spec.rows, &spec.row_tag, table));
    set_kids(doc, table, kids);
    table
}

/// The structure elements with this tag, in the order of their object numbers
/// (the order that the builder made them in, which is the document order).
pub fn elements(pdf: &[u8], tag: &str) -> Vec<(ObjectId, Dictionary)> {
    let doc = Document::load_mem(pdf).expect("the PDF opens");
    doc.objects
        .iter()
        .filter_map(|(id, object)| Some((*id, object.as_dict().ok()?)))
        .filter(|(_, dict)| {
            dict.get(b"S")
                .and_then(Object::as_name)
                .is_ok_and(|name| name == tag.as_bytes())
        })
        .map(|(id, dict)| (id, dict.clone()))
        .collect()
}

/// The `/Scope` of each element with this tag, found in its `/A` (a dictionary,
/// an array of dictionaries, or a reference to either).
pub fn scopes(pdf: &[u8], tag: &str) -> Vec<Option<String>> {
    let doc = Document::load_mem(pdf).expect("the PDF opens");
    elements(pdf, tag)
        .iter()
        .map(|(_, dict)| {
            let mut stack = vec![dict.get(b"A").ok()?];
            while let Some(object) = stack.pop() {
                match doc.dereference(object).ok()?.1 {
                    Object::Array(items) => stack.extend(items),
                    Object::Dictionary(attributes) => {
                        if let Ok(scope) = attributes.get(b"Scope").and_then(Object::as_name) {
                            return Some(String::from_utf8_lossy(scope).into_owned());
                        }
                    }
                    _ => {}
                }
            }
            None
        })
        .collect()
}

/// The same PDF after another program changed its Info dictionary in an
/// incremental update. The result has two cross-reference sections.
pub fn with_foreign_update(pdf: &[u8]) -> Vec<u8> {
    let doc = Document::load_mem(pdf).expect("the PDF opens");
    let mut incremental = IncrementalDocument::create_from(pdf.to_vec(), doc);
    let info_id = incremental
        .get_prev_documents()
        .trailer
        .get(b"Info")
        .and_then(Object::as_reference)
        .expect("the test PDF has an Info dictionary");
    incremental
        .opt_clone_object_to_new_document(info_id)
        .expect("the Info dictionary can be copied");
    if let Ok(Object::Dictionary(info)) = incremental.new_document.get_object_mut(info_id) {
        info.set("Creator", Object::string_literal("A foreign tool"));
    }
    let mut out = Vec::new();
    incremental
        .save_to(&mut out)
        .expect("the update can be written");
    out
}

/// Every variant that has a different file layout.
pub fn layouts() -> Vec<(&'static str, Vec<u8>)> {
    let classic = Pdf::all_missing().build();
    let modern = Pdf {
        modern: true,
        ..Pdf::all_missing()
    }
    .build();
    vec![
        ("classic", classic.clone()),
        ("modern (xref stream)", modern.clone()),
        (
            "classic with an earlier update",
            with_foreign_update(&classic),
        ),
        (
            "modern with an earlier update",
            with_foreign_update(&modern),
        ),
    ]
}

/// The text of the XMP stream and the Info `Title` that a PDF has now.
pub struct Seen {
    pub xmp: Option<String>,
    pub info_title: Option<String>,
    pub lang: Option<String>,
    pub display_doc_title: Option<bool>,
}

pub fn seen(pdf: &[u8]) -> Seen {
    let doc = Document::load_mem(pdf).expect("the PDF opens");
    let catalog = doc.catalog().expect("catalog");
    let xmp = catalog
        .get_deref(b"Metadata", &doc)
        .ok()
        .and_then(|o| o.as_stream().ok())
        .map(|s| String::from_utf8(s.decompressed_content().unwrap()).unwrap());
    let info_title = doc
        .trailer
        .get(b"Info")
        .and_then(Object::as_reference)
        .ok()
        .and_then(|id| doc.get_dictionary(id).ok())
        .and_then(|d| d.get(b"Title").ok())
        .and_then(|t| t.as_str().ok())
        .map(|b| String::from_utf8_lossy(b).into_owned());
    let lang = catalog
        .get(b"Lang")
        .ok()
        .and_then(|t| t.as_str().ok())
        .map(|b| String::from_utf8_lossy(b).into_owned());
    let display_doc_title = catalog
        .get_deref(b"ViewerPreferences", &doc)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"DisplayDocTitle").ok())
        .and_then(|o| o.as_bool().ok());
    Seen {
        xmp,
        info_title,
        lang,
        display_doc_title,
    }
}

/// A folder for one test, under `target/tmp/pdf/`. It is removed when the value
/// is dropped.
pub struct TempDir(std::path::PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        let call = CALLS.fetch_add(1, Ordering::Relaxed);
        let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("pdf")
            .join(format!("{name}-{}-{call}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.0
    }

    /// Writes a file in the folder.
    pub fn write(&self, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    /// The names of the files in the folder, sorted.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
