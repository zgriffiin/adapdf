//! Reading a .docx file.
//!
//! A .docx is a zip file of "parts". [`Package`] keeps the raw bytes of every
//! part, in the original order, so that a later milestone can splice fixes
//! into the original XML text and write the other parts back unchanged.
//!
//! XML parts are parsed with `roxmltree`. It is read-only, and every node
//! knows its byte range in the source text (`Node::range`). Only the
//! transitional OOXML namespace that Word writes is supported.

use std::io::{Cursor, Read};
use std::path::Path;

use roxmltree::Node;

use crate::error::{Error, Result};

pub const DOCUMENT_PART: &str = "word/document.xml";
pub const STYLES_PART: &str = "word/styles.xml";
pub const NUMBERING_PART: &str = "word/numbering.xml";
pub const SETTINGS_PART: &str = "word/settings.xml";
pub const COMMENTS_PART: &str = "word/comments.xml";
pub const FOOTNOTES_PART: &str = "word/footnotes.xml";
pub const ENDNOTES_PART: &str = "word/endnotes.xml";
pub const CORE_PART: &str = "docProps/core.xml";

/// WordprocessingML main namespace.
pub const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
/// Markup Compatibility namespace (`mc:AlternateContent`).
pub const MC_NS: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
/// Dublin Core namespace (`dc:title` in the core properties).
pub const DC_NS: &str = "http://purl.org/dc/elements/1.1/";

const REL_HEADER: &str = "/header";
const REL_FOOTER: &str = "/footer";
const REL_THEME: &str = "/theme";

/// A part bigger than this is refused, so a damaged or hostile zip cannot use
/// all the memory.
const MAX_PART_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// One entry in a `.rels` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    pub id: String,
    /// The full relationship type URI.
    pub kind: String,
    /// The target as written (not resolved).
    pub target: String,
    /// True when the target is outside the package (`TargetMode="External"`).
    pub external: bool,
}

impl Relationship {
    fn has_kind(&self, suffix: &str) -> bool {
        self.kind.ends_with(suffix)
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    parts: Vec<Part>,
}

impl Package {
    pub fn open(path: &Path) -> Result<Self> {
        Self::from_bytes(&std::fs::read(path)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let not_docx = |e: zip::result::ZipError| Error::NotDocx(e.to_string());
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(not_docx)?;
        let mut parts = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(not_docx)?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().map_err(not_docx)?.into_owned();
            let mut data = Vec::new();
            entry.take(MAX_PART_BYTES + 1).read_to_end(&mut data)?;
            if data.len() as u64 > MAX_PART_BYTES {
                return Err(Error::PartTooLarge {
                    part: name,
                    limit_mib: MAX_PART_BYTES / (1024 * 1024),
                });
            }
            parts.push(Part { name, bytes: data });
        }
        let package = Package { parts };
        if package.part(DOCUMENT_PART).is_none() {
            return Err(Error::NotDocx(format!(
                "it is a zip file but has no {DOCUMENT_PART}"
            )));
        }
        Ok(package)
    }

    /// All parts, in the order they have in the zip file.
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    pub fn part(&self, name: &str) -> Option<&[u8]> {
        self.parts
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.bytes.as_slice())
    }

    /// The text of an XML part, without a UTF-8 byte order mark. Byte ranges
    /// of parsed nodes refer to this text. `Ok(None)` when the part does not
    /// exist.
    pub fn text(&self, name: &str) -> Result<Option<&str>> {
        let Some(bytes) = self.part(name) else {
            return Ok(None);
        };
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
        std::str::from_utf8(bytes)
            .map(Some)
            .map_err(|_| Error::Xml {
                part: name.to_string(),
                reason: "it is not UTF-8 text".to_string(),
            })
    }

    /// The relationships of a part (read from `<dir>/_rels/<file>.rels`).
    /// Empty when the part has no `.rels` file.
    pub fn relationships(&self, source_part: &str) -> Result<Vec<Relationship>> {
        let (dir, file) = source_part.rsplit_once('/').unwrap_or(("", source_part));
        let rels_name = if dir.is_empty() {
            format!("_rels/{file}.rels")
        } else {
            format!("{dir}/_rels/{file}.rels")
        };
        let Some(text) = self.text(&rels_name)? else {
            return Ok(Vec::new());
        };
        let doc = parse_xml(&rels_name, text)?;
        Ok(doc
            .root_element()
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "Relationship")
            .filter_map(|n| {
                Some(Relationship {
                    id: n.attribute("Id")?.to_string(),
                    kind: n.attribute("Type")?.to_string(),
                    target: n.attribute("Target")?.to_string(),
                    external: n.attribute("TargetMode") == Some("External"),
                })
            })
            .collect())
    }

    /// Names of the header and footer parts that `word/document.xml` points to.
    pub fn header_footer_parts(&self) -> Result<Vec<String>> {
        Ok(self
            .header_footer_relationships()?
            .into_iter()
            .map(|(_, part)| part)
            .collect())
    }

    /// The relationship id and the part name of each header and footer that
    /// `word/document.xml` points to. A section names them by id.
    pub fn header_footer_relationships(&self) -> Result<Vec<(String, String)>> {
        Ok(self
            .relationships(DOCUMENT_PART)?
            .iter()
            .filter(|r| !r.external && (r.has_kind(REL_HEADER) || r.has_kind(REL_FOOTER)))
            .map(|r| (r.id.clone(), resolve_target(DOCUMENT_PART, &r.target)))
            .collect())
    }

    /// Name of the theme part that `word/document.xml` points to.
    pub fn theme_part(&self) -> Result<Option<String>> {
        Ok(self
            .relationships(DOCUMENT_PART)?
            .iter()
            .find(|r| !r.external && r.has_kind(REL_THEME))
            .map(|r| resolve_target(DOCUMENT_PART, &r.target)))
    }
}

/// Turns a relationship target into a part name. Targets are relative to the
/// folder of the part that owns the `.rels` file, unless they start with `/`.
pub fn resolve_target(source_part: &str, target: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    let relative = if let Some(absolute) = target.strip_prefix('/') {
        absolute
    } else {
        if let Some((dir, _)) = source_part.rsplit_once('/') {
            segments.extend(dir.split('/'));
        }
        target
    };
    for segment in relative.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}

pub fn parse_xml<'a>(part: &str, text: &'a str) -> Result<roxmltree::Document<'a>> {
    roxmltree::Document::parse(text).map_err(|e| Error::Xml {
        part: part.to_string(),
        reason: e.to_string(),
    })
}

/// Short helpers for reading WordprocessingML nodes.
pub trait WordNode<'a, 'input> {
    /// True for an element in the `w:` namespace with this local name.
    fn is_w(&self, local: &str) -> bool;
    /// The first child element `w:<local>`.
    fn w_child(&self, local: &str) -> Option<Node<'a, 'input>>;
    /// The attribute `w:<local>`.
    fn w_attr(&self, local: &str) -> Option<&'a str>;
}

impl<'a, 'input> WordNode<'a, 'input> for Node<'a, 'input> {
    fn is_w(&self, local: &str) -> bool {
        self.is_element()
            && self.tag_name().namespace() == Some(W_NS)
            && self.tag_name().name() == local
    }

    fn w_child(&self, local: &str) -> Option<Node<'a, 'input>> {
        self.children().find(|n| n.is_w(local))
    }

    fn w_attr(&self, local: &str) -> Option<&'a str> {
        self.attribute((W_NS, local))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::DocxBuilder;

    #[test]
    fn resolves_relative_and_absolute_targets() {
        assert_eq!(
            resolve_target("word/document.xml", "header1.xml"),
            "word/header1.xml"
        );
        assert_eq!(
            resolve_target("word/document.xml", "../customXml/item1.xml"),
            "customXml/item1.xml"
        );
        assert_eq!(
            resolve_target("word/document.xml", "/word/theme/theme1.xml"),
            "word/theme/theme1.xml"
        );
        assert_eq!(
            resolve_target("word/document.xml", "./media/a.png"),
            "word/media/a.png"
        );
    }

    #[test]
    fn keeps_every_part_in_zip_order() {
        let package = Package::from_bytes(&DocxBuilder::new().para("Hello").build()).unwrap();
        let names: Vec<&str> = package.parts().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names[0], "[Content_Types].xml");
        assert!(names.contains(&DOCUMENT_PART));
        assert!(names.contains(&STYLES_PART));
        assert!(package.part("word/nothing.xml").is_none());
    }

    #[test]
    fn rejects_input_that_is_not_a_zip() {
        let err = Package::from_bytes(b"this is plain text").unwrap_err();
        assert!(matches!(err, Error::NotDocx(_)), "{err:?}");
    }

    #[test]
    fn rejects_a_zip_without_a_main_document() {
        let bytes = crate::fixtures::zip_stored(&[("hello.txt", b"hi".as_slice())]);
        let err = Package::from_bytes(&bytes).unwrap_err();
        assert!(err.to_string().contains("word/document.xml"), "{err}");
    }

    #[test]
    fn reads_relationships_and_header_parts() {
        let package = Package::from_bytes(&DocxBuilder::new().header_text("Top").build()).unwrap();
        let rels = package.relationships(DOCUMENT_PART).unwrap();
        assert!(rels.iter().any(|r| r.kind.ends_with("/styles")));
        assert_eq!(
            package.header_footer_parts().unwrap(),
            vec!["word/header1.xml".to_string()]
        );
    }

    #[test]
    fn reports_bad_xml_with_the_part_name() {
        let err = parse_xml("word/styles.xml", "<a><b></a>").unwrap_err();
        assert!(err.to_string().contains("word/styles.xml"), "{err}");
    }
}
