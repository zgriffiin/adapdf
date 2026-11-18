//! The fix log: every fix that Adapdf applies is recorded in the accessible copy
//! itself, as the custom document property `Adapdf fix log` (`docProps/custom.xml`).
//!
//! The value is a compact JSON list of [`FixLogEntry`], oldest first. A fix
//! appends to it, in the same write that writes the fix, so the log and the
//! document never disagree. The report reads it, so that it can list every fix
//! of the document and not only the fixes of one run. Word keeps a custom
//! property when the person edits and saves the copy, so the log stays (test
//! S2-37, with the real Word).
//!
//! The part `docProps/custom.xml`, its content type and the package relationship
//! are added when the document has none. Nothing else in the package changes.

use std::collections::BTreeMap;

use roxmltree::Node;
use serde::{Deserialize, Serialize};

use crate::docx::{Package, parse_xml};
use crate::findings::{FixId, RuleId};

use super::{FixError, package, xml};

/// The name of the custom property.
pub const PROPERTY_NAME: &str = "Adapdf fix log";

const CUSTOM_PART: &str = "docProps/custom.xml";
const CONTENT_TYPES_PART: &str = "[Content_Types].xml";
const ROOT_RELS_PART: &str = "_rels/.rels";
const CUSTOM_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.custom-properties+xml";
const CUSTOM_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties";
const CUSTOM_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties";
const VT_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes";
/// The format id that Word writes for every custom property.
const FMTID: &str = "{D5CDD505-2E9C-101B-9397-08002B2CF9AE}";

/// One fix that was applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixLogEntry {
    /// The local time of the fix, `yyyy-mm-dd hh:mm:ss`.
    pub time: String,
    pub rule: RuleId,
    pub fix: FixId,
    /// Where the problem was, in words (the `place` of the finding).
    pub place: String,
    /// What the tool did, in a sentence.
    pub summary: String,
    /// The text the person typed or chose, for a fix that asks: the title, the
    /// alt text, or `decorative`.
    #[serde(default)]
    pub text: Option<String>,
}

/// The log of a document, oldest first. `Err` has the reason when the property
/// is there and is not a log that this tool wrote.
pub fn read(package: &Package) -> Result<Vec<FixLogEntry>, String> {
    let Some(text) = package.text(CUSTOM_PART).map_err(|e| e.to_string())? else {
        return Ok(Vec::new());
    };
    let doc = parse_xml(CUSTOM_PART, text).map_err(|e| e.to_string())?;
    let Some(property) = property_node(&doc) else {
        return Ok(Vec::new());
    };
    let value = property
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "lpwstr")
        .and_then(|n| n.text())
        .ok_or_else(|| format!("the property \"{PROPERTY_NAME}\" has no text"))?;
    serde_json::from_str(value)
        .map_err(|e| format!("the property \"{PROPERTY_NAME}\" is not a fix log ({e})"))
}

fn property_node<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> Option<Node<'a, 'i>> {
    doc.root_element().children().find(|n| {
        n.is_element()
            && n.tag_name().name() == "property"
            && n.attribute("name") == Some(PROPERTY_NAME)
    })
}

fn invalid(reason: impl std::fmt::Display) -> FixError {
    FixError::Invalid(format!("the fix log cannot be written: {reason}"))
}

fn property_xml(pid: u32, value: &str) -> String {
    format!(
        "<property fmtid=\"{FMTID}\" pid=\"{pid}\" name=\"{PROPERTY_NAME}\"><vt:lpwstr xmlns:vt=\"{VT_NS}\">{}</vt:lpwstr></property>",
        xml::escape_text(value)
    )
}

/// The text of `docProps/custom.xml` with the property set to `value`.
fn custom_part(package: &Package, value: &str) -> Result<Vec<u8>, FixError> {
    let existing = package.text(CUSTOM_PART)?;
    let Some(text) = existing else {
        let fresh = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Properties xmlns=\"{CUSTOM_NS}\" xmlns:vt=\"{VT_NS}\">{}</Properties>",
            property_xml(2, value)
        );
        return Ok(fresh.into_bytes());
    };
    let doc = parse_xml(CUSTOM_PART, text)?;
    let root = doc.root_element();
    if root.tag_name().name() != "Properties" || root.tag_name().namespace() != Some(CUSTOM_NS) {
        return Err(invalid("docProps/custom.xml is not a list of properties"));
    }
    if root.lookup_prefix(CUSTOM_NS).is_some() {
        return Err(invalid(
            "docProps/custom.xml writes its elements with a prefix",
        ));
    }
    let mut splices = xml::Splices::default();
    match property_node(&doc) {
        Some(property) => {
            let pid = property
                .attribute("pid")
                .and_then(|p| p.parse().ok())
                .unwrap_or(2);
            splices.replace(property.range(), property_xml(pid, value));
        }
        None => {
            let pid = root
                .children()
                .filter_map(|n| n.attribute("pid")?.parse::<u32>().ok())
                .max()
                .map_or(2, |max| max + 1);
            let range = root.range();
            let source = &text[range.clone()];
            if let Some(head) = source.strip_suffix("/>") {
                // `<Properties .../>` with no property: open it and close it.
                splices.replace(
                    range,
                    format!("{head}>{}</Properties>", property_xml(pid, value)),
                );
            } else {
                let close = source
                    .rfind("</")
                    .ok_or_else(|| invalid("docProps/custom.xml has no end tag"))?;
                let at = range.start + close;
                splices.insert(at, property_xml(pid, value));
            }
        }
    }
    let resolved = splices
        .resolve()
        .map_err(|_| invalid("the property overlaps another edit"))?;
    let new_text = xml::apply_resolved(text, &resolved);
    parse_xml(CUSTOM_PART, &new_text).map_err(|e| FixError::Invalid(e.to_string()))?;
    let mut bytes = Vec::new();
    if package
        .part(CUSTOM_PART)
        .is_some_and(|b| b.starts_with(b"\xEF\xBB\xBF"))
    {
        bytes.extend_from_slice(b"\xEF\xBB\xBF");
    }
    bytes.extend_from_slice(new_text.as_bytes());
    Ok(bytes)
}

/// `text` with `item` put before the last `close` tag.
fn insert_before_close(
    package: &Package,
    part: &str,
    close: &str,
    item: &str,
) -> Result<Vec<u8>, FixError> {
    let text = package
        .text(part)?
        .ok_or_else(|| invalid(format!("the document has no {part}")))?;
    let at = text
        .rfind(close)
        .ok_or_else(|| invalid(format!("{part} has no {close}")))?;
    let mut new_text = String::with_capacity(text.len() + item.len());
    new_text.push_str(&text[..at]);
    new_text.push_str(item);
    new_text.push_str(&text[at..]);
    parse_xml(part, &new_text).map_err(|e| FixError::Invalid(e.to_string()))?;
    let mut bytes = Vec::new();
    if package
        .part(part)
        .is_some_and(|b| b.starts_with(b"\xEF\xBB\xBF"))
    {
        bytes.extend_from_slice(b"\xEF\xBB\xBF");
    }
    bytes.extend_from_slice(new_text.as_bytes());
    Ok(bytes)
}

/// The .docx with `entries` added to its fix log, written in the same package.
/// Every other part keeps its bytes, except `[Content_Types].xml` and
/// `_rels/.rels` when the document had no custom properties yet.
pub fn append(bytes: &[u8], entries: &[FixLogEntry]) -> Result<Vec<u8>, FixError> {
    if entries.is_empty() {
        return Ok(bytes.to_vec());
    }
    let package = Package::from_bytes(bytes)?;
    let mut log = read(&package).map_err(invalid)?;
    log.extend(entries.iter().cloned());
    let value = serde_json::to_string(&log).map_err(invalid)?;

    let mut replacements: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    replacements.insert(CUSTOM_PART.to_string(), custom_part(&package, &value)?);

    if package.part(CUSTOM_PART).is_none() {
        let types = package
            .text(CONTENT_TYPES_PART)?
            .ok_or_else(|| invalid("the document has no [Content_Types].xml"))?;
        if !types.contains("PartName=\"/docProps/custom.xml\"") {
            replacements.insert(
                CONTENT_TYPES_PART.to_string(),
                insert_before_close(
                    &package,
                    CONTENT_TYPES_PART,
                    "</Types>",
                    &format!(
                        "<Override PartName=\"/docProps/custom.xml\" ContentType=\"{CUSTOM_CONTENT_TYPE}\"/>"
                    ),
                )?,
            );
        }
        let rels = package
            .text(ROOT_RELS_PART)?
            .ok_or_else(|| invalid("the document has no _rels/.rels"))?;
        if !rels.contains(CUSTOM_REL_TYPE) {
            let mut id = "rIdAdapdfLog".to_string();
            let mut n = 1;
            while rels.contains(&format!("Id=\"{id}\"")) {
                n += 1;
                id = format!("rIdAdapdfLog{n}");
            }
            replacements.insert(
                ROOT_RELS_PART.to_string(),
                insert_before_close(
                    &package,
                    ROOT_RELS_PART,
                    "</Relationships>",
                    &format!(
                        "<Relationship Id=\"{id}\" Type=\"{CUSTOM_REL_TYPE}\" Target=\"docProps/custom.xml\"/>"
                    ),
                )?,
            );
        }
    }
    Ok(package::rewrite(bytes, &replacements)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::DocxBuilder;
    use crate::model::Document;

    fn entry(n: u32, text: Option<&str>) -> FixLogEntry {
        FixLogEntry {
            time: format!("2026-03-05 10:15:{n:02}"),
            rule: RuleId::Doc001,
            fix: FixId::SetTitle,
            place: "The document's properties (File > Info)".to_string(),
            summary: format!("Set the title {n}."),
            text: text.map(str::to_string),
        }
    }

    fn log_of(bytes: &[u8]) -> Vec<FixLogEntry> {
        read(&Package::from_bytes(bytes).unwrap()).unwrap()
    }

    #[test]
    fn a_document_with_no_custom_properties_has_an_empty_log() {
        let bytes = DocxBuilder::new().para("Hi").build();
        assert!(log_of(&bytes).is_empty());
    }

    #[test]
    fn the_first_fix_creates_the_part_the_content_type_and_the_relationship() {
        let bytes = DocxBuilder::new().para("Hi").build();
        let new = append(&bytes, &[entry(1, Some("A & B <C> \"D\" é 日本"))]).unwrap();
        let package = Package::from_bytes(&new).unwrap();
        let custom = package.text(CUSTOM_PART).unwrap().unwrap();
        assert!(custom.contains("name=\"Adapdf fix log\""), "{custom}");
        assert!(
            package
                .text(CONTENT_TYPES_PART)
                .unwrap()
                .unwrap()
                .contains("/docProps/custom.xml")
        );
        assert!(
            package
                .text(ROOT_RELS_PART)
                .unwrap()
                .unwrap()
                .contains("custom-properties")
        );
        assert_eq!(log_of(&new), [entry(1, Some("A & B <C> \"D\" é 日本"))]);
        // The rest of the package is as it was, and the tool still reads it.
        let old = Package::from_bytes(&bytes).unwrap();
        for part in old.parts() {
            if part.name != CONTENT_TYPES_PART && part.name != ROOT_RELS_PART {
                assert_eq!(
                    package.part(&part.name),
                    Some(part.bytes.as_slice()),
                    "{}",
                    part.name
                );
            }
        }
        Document::from_bytes(&new).unwrap();
    }

    #[test]
    fn later_fixes_are_appended_and_the_old_ones_stay() {
        let bytes = DocxBuilder::new().para("Hi").build();
        let one = append(&bytes, &[entry(1, None)]).unwrap();
        let two = append(&one, &[entry(2, Some("x")), entry(3, None)]).unwrap();
        assert_eq!(
            log_of(&two),
            [entry(1, None), entry(2, Some("x")), entry(3, None)]
        );
        // The part, the content type and the relationship are not added twice.
        let package = Package::from_bytes(&two).unwrap();
        assert_eq!(
            package
                .text(CONTENT_TYPES_PART)
                .unwrap()
                .unwrap()
                .matches("custom.xml")
                .count(),
            1
        );
        assert_eq!(
            package
                .text(ROOT_RELS_PART)
                .unwrap()
                .unwrap()
                .matches("custom-properties")
                .count(),
            1
        );
        // An empty list of entries changes nothing.
        assert_eq!(append(&two, &[]).unwrap(), two);
    }

    #[test]
    fn the_log_is_added_to_other_custom_properties() {
        let custom = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Properties xmlns=\"{CUSTOM_NS}\" xmlns:vt=\"{VT_NS}\"><property fmtid=\"{FMTID}\" pid=\"2\" name=\"Department\"><vt:lpwstr>Parks</vt:lpwstr></property><property fmtid=\"{FMTID}\" pid=\"5\" name=\"Count\"><vt:i4>3</vt:i4></property></Properties>"
        );
        let base = DocxBuilder::new().para("Hi").build();
        let with_custom = crate::fixtures::zip_stored(
            &Package::from_bytes(&base)
                .unwrap()
                .parts()
                .iter()
                .map(|p| (p.name.as_str(), p.bytes.as_slice()))
                .chain([(CUSTOM_PART, custom.as_bytes())])
                .collect::<Vec<_>>(),
        );
        let new = append(&with_custom, &[entry(1, None)]).unwrap();
        let package = Package::from_bytes(&new).unwrap();
        let text = package.text(CUSTOM_PART).unwrap().unwrap();
        assert!(
            text.contains("name=\"Department\"") && text.contains("<vt:i4>3</vt:i4>"),
            "{text}"
        );
        assert!(text.contains("pid=\"6\" name=\"Adapdf fix log\""), "{text}");
        assert_eq!(log_of(&new), [entry(1, None)]);
        // And a second fix replaces the value of the same property.
        let again = append(&new, &[entry(2, None)]).unwrap();
        let text = Package::from_bytes(&again)
            .unwrap()
            .text(CUSTOM_PART)
            .unwrap()
            .unwrap()
            .to_string();
        assert_eq!(text.matches("Adapdf fix log").count(), 1, "{text}");
        assert!(text.contains("pid=\"6\""), "{text}");
        assert_eq!(log_of(&again), [entry(1, None), entry(2, None)]);
    }

    #[test]
    fn a_property_that_is_not_a_log_is_not_overwritten() {
        let custom = format!(
            "<Properties xmlns=\"{CUSTOM_NS}\" xmlns:vt=\"{VT_NS}\"><property fmtid=\"{FMTID}\" pid=\"2\" name=\"Adapdf fix log\"><vt:lpwstr>not json</vt:lpwstr></property></Properties>"
        );
        let base = DocxBuilder::new().para("Hi").build();
        let broken = crate::fixtures::zip_stored(
            &Package::from_bytes(&base)
                .unwrap()
                .parts()
                .iter()
                .map(|p| (p.name.as_str(), p.bytes.as_slice()))
                .chain([(CUSTOM_PART, custom.as_bytes())])
                .collect::<Vec<_>>(),
        );
        let error = append(&broken, &[entry(1, None)]).unwrap_err();
        assert!(error.to_string().contains("not a fix log"), "{error}");
    }

    #[test]
    fn a_long_log_is_one_property() {
        let bytes = DocxBuilder::new().para("Hi").build();
        let long = "x".repeat(5000);
        let new = append(&bytes, &[entry(1, Some(&long))]).unwrap();
        assert_eq!(log_of(&new)[0].text.as_deref(), Some(long.as_str()));
    }
}
