//! Stage 4 of the pipeline (spec 02): PDF metadata fixes.
//!
//! Only document-level metadata is checked and fixed. The content, the tags and
//! the fonts are never read for a change and never written. A fix is an
//! incremental update: the original bytes stay as they are, and a new part
//! (changed objects, a cross-reference section and a trailer) is appended.
//! [`Outcome::appended`] holds only the appended part, so the original file is
//! a prefix of the output by construction.
//!
//! | Check | Fix if missing |
//! |---|---|
//! | XMP `pdfuaid:part` = 1 | Add it. |
//! | XMP `dc:title` and the Info `Title` | Copy from the .docx title. |
//! | `/ViewerPreferences /DisplayDocTitle true` | Set it. |
//! | Catalog `/Lang` | Copy from the .docx default language. |
//! | `/MarkInfo /Marked true` and a `/StructTreeRoot` | Not fixed. An error finding. |

mod update;
mod xmp;

use std::fs;
use std::io;
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use serde::Serialize;

use crate::findings::Severity;
use crate::model::Document as DocxDocument;

use update::Update;

/// Stage 4 could not run. These are tool failures (exit code 2).
#[derive(Debug, thiserror::Error)]
pub enum PdfMetaError {
    #[error("the PDF file cannot be read or written: {0}")]
    Io(#[from] io::Error),

    #[error(
        "the PDF is encrypted, and the tool does not change encrypted PDFs. \
         Export the PDF again without a password."
    )]
    Encrypted,

    #[error("the PDF is damaged and cannot be changed safely: {0}")]
    Unreadable(String),

    #[error("the metadata change could not be written: {0}")]
    Write(String),
}

pub type Result<T> = std::result::Result<T, PdfMetaError>;

/// The values that stage 4 copies from the .docx.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMetadata {
    /// The title in `docProps/core.xml`.
    pub title: Option<String>,
    /// The default language, for example `en-US`.
    pub language: Option<String>,
}

impl SourceMetadata {
    pub fn from_document(document: &DocxDocument) -> SourceMetadata {
        SourceMetadata {
            title: document.core.title.clone(),
            language: document.default_language.clone(),
        }
    }
}

/// The value that a check looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Field {
    /// XMP `pdfuaid:part`.
    #[serde(rename = "xmp-pdfuaid-part")]
    XmpPdfUaPart,
    /// XMP `dc:title`.
    #[serde(rename = "xmp-title")]
    XmpTitle,
    /// The `Title` entry of the Info dictionary.
    #[serde(rename = "info-title")]
    InfoTitle,
    /// `/ViewerPreferences /DisplayDocTitle`.
    #[serde(rename = "display-doc-title")]
    DisplayDocTitle,
    /// The catalog's `/Lang`.
    #[serde(rename = "lang")]
    Lang,
}

impl Field {
    /// What the value is, in plain words.
    pub fn describe(self) -> &'static str {
        match self {
            Field::XmpPdfUaPart => "PDF/UA part in the XMP metadata (pdfuaid:part)",
            Field::XmpTitle => "title in the XMP metadata (dc:title)",
            Field::InfoTitle => "title in the document information (Title)",
            Field::DisplayDocTitle => "show the title in the title bar (DisplayDocTitle)",
            Field::Lang => "document language (Lang)",
        }
    }
}

/// One change that stage 4 made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    pub field: Field,
    /// `describe()` of the field.
    pub what: String,
    /// The value before. `None` when there was none.
    pub old: Option<String>,
    pub new: String,
}

/// Why stage 4 reports an error finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueKind {
    /// `/MarkInfo /Marked true` is missing.
    NotMarked,
    /// The catalog has no `/StructTreeRoot`.
    NoStructTreeRoot,
    /// `pdfuaid:part` has a value other than 1.
    WrongPdfUaPart,
    /// The PDF has no title and the .docx has none to copy.
    NoTitleToCopy,
    /// The PDF has no language and the .docx has none to copy.
    NoLanguageToCopy,
}

/// A problem that stage 4 does not fix. It is an error: the exit code is 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub kind: IssueKind,
    pub severity: Severity,
    /// What is wrong, in plain language.
    pub message: String,
    /// What the user does.
    pub how_to_fix: String,
}

impl Issue {
    fn error(kind: IssueKind, message: &str, how_to_fix: &str) -> Issue {
        Issue {
            kind,
            severity: Severity::Error,
            message: message.to_string(),
            how_to_fix: how_to_fix.to_string(),
        }
    }
}

/// The result of stage 4 for one PDF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The changes that were made, in a fixed order.
    pub changes: Vec<Change>,
    /// Problems that were not fixed.
    pub issues: Vec<Issue>,
}

impl Report {
    /// `true` when stage 4 found an error that blocks a pass.
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }
}

/// What to do with a PDF, as bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The bytes to put after the original bytes. Empty when nothing changed.
    pub appended: Vec<u8>,
    /// The numbers of the objects that the update writes.
    pub written_objects: Vec<ObjectId>,
    pub report: Report,
}

impl Outcome {
    /// The new file: the original bytes and then the appended part.
    pub fn apply(&self, original: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(original.len() + self.appended.len());
        output.extend_from_slice(original);
        output.extend_from_slice(&self.appended);
        output
    }
}

/// Checks the PDF and works out the fix, without writing anything.
pub fn check_bytes(pdf: &[u8], source: &SourceMetadata) -> Result<Outcome> {
    let document = load(pdf)?;
    let last = update::last_xref(pdf)
        .ok_or_else(|| PdfMetaError::Unreadable("it has no cross-reference section".to_string()))?;
    if document.xref_start != last.offset {
        return Err(PdfMetaError::Unreadable(
            "its cross-reference section is not where the file says it is".to_string(),
        ));
    }

    let root_id = document
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .map_err(|_| PdfMetaError::Unreadable("it has no document catalog".to_string()))?;
    let catalog = document
        .get_dictionary(root_id)
        .map_err(|_| PdfMetaError::Unreadable("its document catalog is missing".to_string()))?
        .clone();

    let mut report = Report {
        changes: Vec::new(),
        issues: Vec::new(),
    };

    // The export failed if the PDF has no tags. Nothing is fixed then.
    if !is_marked(&document, &catalog) {
        report.issues.push(Issue::error(
            IssueKind::NotMarked,
            "The PDF is not marked as a tagged PDF (/MarkInfo /Marked true is missing). \
             The Word export did not write tags.",
            "Export the PDF again from Word with \"Document structure tags for accessibility\" on.",
        ));
    }
    let has_tree = catalog
        .get_deref(b"StructTreeRoot", &document)
        .is_ok_and(|o| o.as_dict().is_ok());
    if !has_tree {
        report.issues.push(Issue::error(
            IssueKind::NoStructTreeRoot,
            "The PDF has no structure tree (/StructTreeRoot is missing). The Word export did not write tags.",
            "Export the PDF again from Word with \"Document structure tags for accessibility\" on.",
        ));
    }
    if !report.issues.is_empty() {
        return Ok(Outcome {
            appended: Vec::new(),
            written_objects: Vec::new(),
            report,
        });
    }

    let next_id = {
        let from_trailer = document
            .trailer
            .get(b"Size")
            .and_then(Object::as_i64)
            .ok()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0);
        from_trailer.max(document.max_id.saturating_add(1))
    };
    let mut update = Update::new(pdf, last, &document.trailer, next_id);
    let mut new_catalog = catalog.clone();
    let mut catalog_changed = false;

    // The XMP metadata: `pdfuaid:part` and `dc:title`.
    let metadata_id = catalog.get(b"Metadata").and_then(Object::as_reference).ok();
    let xmp_text = match metadata_id {
        Some(id) => Some(read_xmp(&document, id)?),
        None => None,
    };
    let state = match &xmp_text {
        Some(text) => xmp::read(text).map_err(|e| {
            PdfMetaError::Unreadable(format!("its XMP metadata is not valid XML ({})", e.0))
        })?,
        None => xmp::XmpState {
            part: None,
            title: None,
        },
    };
    let source_title = source
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());

    let add_part = match state.part.as_deref() {
        Some("1") => false,
        Some(other) => {
            report.issues.push(Issue::error(
                IssueKind::WrongPdfUaPart,
                &format!(
                    "The XMP metadata says the PDF is PDF/UA part {other}, but this tool checks for part 1. The tool did not change it."
                ),
                "Check how the PDF was made. The value pdfuaid:part must be 1 for PDF/UA-1.",
            ));
            false
        }
        None => true,
    };
    if add_part {
        report.changes.push(Change {
            field: Field::XmpPdfUaPart,
            what: Field::XmpPdfUaPart.describe().to_string(),
            old: None,
            new: "1".to_string(),
        });
    }
    let add_xmp_title = if state.title.is_none() {
        source_title
    } else {
        None
    };
    if let Some(title) = add_xmp_title {
        report.changes.push(Change {
            field: Field::XmpTitle,
            what: Field::XmpTitle.describe().to_string(),
            old: None,
            new: title.to_string(),
        });
    }

    // The Info dictionary: `Title`.
    // The trailer should point to the Info dictionary. If it holds the
    // dictionary itself, its keys are kept and the new dictionary is an object.
    let info_entry = document.trailer.get(b"Info").ok();
    let info_ref = info_entry.and_then(|o| o.as_reference().ok());
    let mut info = info_entry
        .and_then(|o| document.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
        .cloned()
        .unwrap_or_default();
    let info_title = info
        .get(b"Title")
        .ok()
        .and_then(|t| t.as_str().ok())
        .map(text_string)
        .filter(|t| !t.trim().is_empty());
    let mut info_changed = false;
    if info_title.is_none()
        && let Some(title) = source_title
    {
        info.set("Title", encode_text_string(title));
        info_changed = true;
        report.changes.push(Change {
            field: Field::InfoTitle,
            what: Field::InfoTitle.describe().to_string(),
            old: None,
            new: title.to_string(),
        });
    }
    if source_title.is_none() && (state.title.is_none() || info_title.is_none()) {
        report.issues.push(Issue::error(
            IssueKind::NoTitleToCopy,
            "The PDF has no title in its metadata, and the Word document has no title to copy.",
            "Set the document title in Word (File, Info, Title), then export the PDF again.",
        ));
    }

    // `/ViewerPreferences /DisplayDocTitle true`.
    let prefs_ref = catalog
        .get(b"ViewerPreferences")
        .and_then(Object::as_reference)
        .ok();
    let prefs = catalog
        .get_deref(b"ViewerPreferences", &document)
        .ok()
        .and_then(|o| o.as_dict().ok());
    let display = prefs
        .and_then(|p| p.get(b"DisplayDocTitle").ok())
        .and_then(|o| o.as_bool().ok());
    if display != Some(true) {
        let mut new_prefs = prefs.cloned().unwrap_or_default();
        new_prefs.set("DisplayDocTitle", true);
        match prefs_ref {
            Some(id) => update.replace(id, Object::Dictionary(new_prefs)),
            None => {
                new_catalog.set("ViewerPreferences", Object::Dictionary(new_prefs));
                catalog_changed = true;
            }
        }
        report.changes.push(Change {
            field: Field::DisplayDocTitle,
            what: Field::DisplayDocTitle.describe().to_string(),
            old: display.map(|v| v.to_string()),
            new: "true".to_string(),
        });
    }

    // The catalog's `/Lang`.
    let lang = catalog
        .get_deref(b"Lang", &document)
        .ok()
        .and_then(|l| l.as_str().ok())
        .map(text_string)
        .filter(|l| !l.trim().is_empty());
    if lang.is_none() {
        match source
            .language
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
        {
            Some(language) => {
                new_catalog.set("Lang", Object::string_literal(language.as_bytes()));
                catalog_changed = true;
                report.changes.push(Change {
                    field: Field::Lang,
                    what: Field::Lang.describe().to_string(),
                    old: None,
                    new: language.to_string(),
                });
            }
            None => report.issues.push(Issue::error(
                IssueKind::NoLanguageToCopy,
                "The PDF has no document language, and the Word document has no default language to copy.",
                "Set the language in Word (Review, Language, Set Proofing Language), then export the PDF again.",
            )),
        }
    }

    // The metadata stream and the Info dictionary are written last, because
    // they may need new object numbers.
    if add_part || add_xmp_title.is_some() {
        let text = match &xmp_text {
            Some(text) => xmp::edit(text, add_part, add_xmp_title).map_err(|e| {
                PdfMetaError::Unreadable(format!("its XMP metadata cannot be changed ({})", e.0))
            })?,
            None => xmp::new_packet(add_part, add_xmp_title),
        };
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name(b"Metadata".to_vec()));
        dict.set("Subtype", Object::Name(b"XML".to_vec()));
        let stream = Object::Stream(Stream::new(dict, text.into_bytes()));
        match metadata_id {
            Some(id) => update.replace(id, stream),
            None => {
                let id = update.add(stream);
                new_catalog.set("Metadata", Object::Reference(id));
                catalog_changed = true;
            }
        }
    }
    if info_changed {
        match info_ref {
            Some(id) => update.replace(id, Object::Dictionary(info)),
            None => {
                let id = update.add(Object::Dictionary(info));
                update.set_info(id);
            }
        }
    }
    if catalog_changed {
        update.replace(root_id, Object::Dictionary(new_catalog));
    }

    if update.is_empty() {
        return Ok(Outcome {
            appended: Vec::new(),
            written_objects: Vec::new(),
            report,
        });
    }
    let written_objects = update.object_ids();
    let appended = update.finish().map_err(PdfMetaError::Write)?;
    let outcome = Outcome {
        appended,
        written_objects,
        report,
    };

    // The output must open, and its cross-reference chain must be sound.
    let output = outcome.apply(pdf);
    let reread = Document::load_mem(&output)
        .map_err(|e| PdfMetaError::Write(format!("the new file does not open again ({e})")))?;
    if reread.xref_recovery.is_some()
        || reread.xref_section_count != document.xref_section_count + 1
    {
        return Err(PdfMetaError::Write(
            "the new cross-reference section is not read as an update of the old one".to_string(),
        ));
    }
    Ok(outcome)
}

/// Checks the PDF in the file and, if something is missing, writes the fix into
/// the same file. The new file is written to a temporary file next to it and
/// moved into place, so a failure leaves the original as it was. When nothing
/// changes, the file is not touched.
pub fn fix_file(path: &Path, source: &SourceMetadata) -> Result<Report> {
    let original = fs::read(path)?;
    let outcome = check_bytes(&original, source)?;
    if outcome.appended.is_empty() {
        return Ok(outcome.report);
    }
    let mut temp_name = path.as_os_str().to_owned();
    temp_name.push(".adapdf-metadata.tmp");
    let temp = std::path::PathBuf::from(temp_name);
    let written = fs::write(&temp, outcome.apply(&original)).and_then(|()| fs::rename(&temp, path));
    if let Err(error) = written {
        let _ = fs::remove_file(&temp);
        return Err(PdfMetaError::Io(error));
    }
    Ok(outcome.report)
}

fn load(pdf: &[u8]) -> Result<Document> {
    let document = Document::load_mem(pdf).map_err(|error| {
        let text = error.to_string();
        if text.to_lowercase().contains("password") || text.to_lowercase().contains("encrypt") {
            PdfMetaError::Encrypted
        } else {
            PdfMetaError::Unreadable(text)
        }
    })?;
    if document.trailer.has(b"Encrypt") || document.is_encrypted() || document.was_encrypted() {
        return Err(PdfMetaError::Encrypted);
    }
    if document.xref_recovery.is_some() {
        return Err(PdfMetaError::Unreadable(
            "its cross-reference table is damaged".to_string(),
        ));
    }
    Ok(document)
}

fn is_marked(document: &Document, catalog: &Dictionary) -> bool {
    catalog
        .get_deref(b"MarkInfo", document)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get_deref(b"Marked", document).ok())
        .and_then(|o| o.as_bool().ok())
        == Some(true)
}

fn read_xmp(document: &Document, id: ObjectId) -> Result<String> {
    let stream = document
        .get_object(id)
        .ok()
        .and_then(|o| o.as_stream().ok())
        .ok_or_else(|| PdfMetaError::Unreadable("its XMP metadata is not a stream".to_string()))?;
    let bytes = stream
        .decompressed_content()
        .map_err(|e| PdfMetaError::Unreadable(format!("its XMP metadata cannot be read ({e})")))?;
    String::from_utf8(bytes)
        .map_err(|_| PdfMetaError::Unreadable("its XMP metadata is not UTF-8 text".to_string()))
}

/// A PDF text string as text: UTF-16BE with a byte order mark, UTF-8 with a
/// byte order mark, or PDFDocEncoding (read here as Latin-1, which matches it
/// for the letters and digits that matter for a comparison or a report).
fn text_string(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// A text string for the Info dictionary: plain bytes for ASCII text, and
/// UTF-16BE with a byte order mark for anything else.
fn encode_text_string(text: &str) -> Object {
    if text.bytes().all(|b| (0x20..0x7F).contains(&b)) {
        return Object::string_literal(text.as_bytes());
    }
    let mut bytes = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Object::string_literal(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_strings_round_trip() {
        assert_eq!(text_string(b"Guide"), "Guide");
        let encoded = encode_text_string("Guide é – 1");
        let bytes = encoded.as_str().unwrap();
        assert_eq!(&bytes[..2], &[0xFE, 0xFF]);
        assert_eq!(text_string(bytes), "Guide é – 1");
        assert_eq!(encode_text_string("Plain").as_str().unwrap(), b"Plain");
    }
}
