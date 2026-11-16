//! Reads and edits the XMP packet of a PDF, as text.
//!
//! The edit adds a new `rdf:Description` before the end of `rdf:RDF`, or
//! replaces an empty `dc:title`. Everything else in the packet stays as it is.

use roxmltree::{Document, Node};

pub const NS_RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const NS_DC: &str = "http://purl.org/dc/elements/1.1/";
pub const NS_PDFUAID: &str = "http://www.aiim.org/pdfua/ns/id/";

/// What the packet says now about the three values stage 4 checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmpState {
    /// `pdfuaid:part`, as an element or as an attribute. `None` if it is missing.
    pub part: Option<String>,
    /// The first non-empty `dc:title` text. `None` if it is missing or empty.
    pub title: Option<String>,
}

/// The XMP text could not be used.
#[derive(Debug, PartialEq, Eq)]
pub struct XmpError(pub String);

fn is_element(node: &Node, ns: &str, name: &str) -> bool {
    node.is_element() && node.tag_name().namespace() == Some(ns) && node.tag_name().name() == name
}

fn parse(xmp: &str) -> Result<Document<'_>, XmpError> {
    Document::parse(xmp).map_err(|e| XmpError(e.to_string()))
}

/// The element `dc:title`, if the packet has one.
fn title_element<'a, 'input>(doc: &'a Document<'input>) -> Option<Node<'a, 'input>> {
    doc.descendants().find(|n| is_element(n, NS_DC, "title"))
}

/// The first non-blank text inside a `dc:title` element.
fn title_text(title: &Node) -> Option<String> {
    title
        .descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .map(str::trim)
        .find(|t| !t.is_empty())
        .map(str::to_string)
}

pub fn read(xmp: &str) -> Result<XmpState, XmpError> {
    let doc = parse(xmp)?;
    let part = doc.descendants().find_map(|n| {
        if is_element(&n, NS_PDFUAID, "part") {
            return Some(n.text().unwrap_or("").trim().to_string());
        }
        if is_element(&n, NS_RDF, "Description") {
            return n
                .attribute((NS_PDFUAID, "part"))
                .map(|v| v.trim().to_string());
        }
        None
    });
    let title = title_element(&doc).and_then(|t| title_text(&t));
    Ok(XmpState { part, title })
}

/// Escapes text for an XML element.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn title_element_text(title: &str) -> String {
    format!(
        "<dc:title xmlns:dc=\"{NS_DC}\" xmlns:rdf=\"{NS_RDF}\"><rdf:Alt>\
         <rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>",
        escape(title)
    )
}

/// A `rdf:Description` with the values to add. `about` is the `rdf:about` text
/// that the packet already uses.
fn description(about: &str, add_part: bool, title: Option<&str>) -> String {
    let mut text = format!("<rdf:Description xmlns:rdf=\"{NS_RDF}\" rdf:about=\"{about}\"");
    if add_part {
        text.push_str(&format!(" xmlns:pdfuaid=\"{NS_PDFUAID}\""));
    }
    text.push('>');
    if add_part {
        text.push_str("<pdfuaid:part>1</pdfuaid:part>");
    }
    if let Some(title) = title {
        text.push_str(&title_element_text(title));
    }
    text.push_str("</rdf:Description>");
    text
}

/// A new packet that has only the values to add.
pub fn new_packet(add_part: bool, title: Option<&str>) -> String {
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
         <rdf:RDF xmlns:rdf=\"{NS_RDF}\">\n{}\n</rdf:RDF>\n</x:xmpmeta>\n\
         <?xpacket end=\"w\"?>",
        description("", add_part, title)
    )
}

/// Adds `pdfuaid:part` = 1 (when `add_part`) and the title (when given) to an
/// existing packet. An empty `dc:title` is replaced; a missing one is added.
pub fn edit(xmp: &str, add_part: bool, title: Option<&str>) -> Result<String, XmpError> {
    let doc = parse(xmp)?;
    let rdf = doc
        .descendants()
        .find(|n| is_element(n, NS_RDF, "RDF"))
        .ok_or_else(|| XmpError("the XMP has no rdf:RDF element".to_string()))?;

    // An empty dc:title is replaced where it is. Only then is a second
    // dc:title not wanted.
    let mut replace_title: Option<std::ops::Range<usize>> = None;
    let mut add_title = title;
    if let (Some(existing), Some(_)) = (title_element(&doc), title) {
        replace_title = Some(existing.range());
        add_title = None;
    }

    let about = doc
        .descendants()
        .find(|n| is_element(n, NS_RDF, "Description"))
        .and_then(|n| n.attribute((NS_RDF, "about")))
        .unwrap_or("");
    let about = about.replace('&', "&amp;").replace('"', "&quot;");

    let mut result = xmp.to_string();
    // Edit the later position first, so the earlier range stays valid.
    let insertion =
        (add_part || add_title.is_some()).then(|| description(&about, add_part, add_title));
    let rdf_range = rdf.range();
    let rdf_text = &xmp[rdf_range.clone()];
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    if let Some(range) = replace_title {
        edits.push((range, title_element_text(title.unwrap_or(""))));
    }
    if let Some(insertion) = insertion {
        if let Some(open) = rdf_text.strip_suffix("/>") {
            // `<rdf:RDF .../>`: open it, add the description, and close it.
            let open = open.trim_end();
            let tag = open[1..].split_whitespace().next().unwrap_or("rdf:RDF");
            edits.push((rdf_range, format!("{open}>{insertion}</{tag}>")));
        } else {
            let close = rdf_range.start
                + rdf_text
                    .rfind("</")
                    .ok_or_else(|| XmpError("the XMP has a damaged rdf:RDF element".to_string()))?;
            edits.push((close..close, insertion));
        }
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    for (range, text) in edits {
        result.replace_range(range, &text);
    }
    // The result must still be XML.
    parse(&result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORD_LIKE: &str = "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"uuid:1\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\
<xmp:CreatorTool>Word</xmp:CreatorTool></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";

    #[test]
    fn reads_nothing_from_a_packet_without_the_values() {
        let state = read(WORD_LIKE).unwrap();
        assert_eq!(
            state,
            XmpState {
                part: None,
                title: None
            }
        );
    }

    #[test]
    fn adds_part_and_title_and_keeps_the_rest() {
        let edited = edit(WORD_LIKE, true, Some("A & B <1>")).unwrap();
        assert!(edited.contains("<xmp:CreatorTool>Word</xmp:CreatorTool>"));
        assert!(edited.starts_with("<?xpacket begin="));
        assert!(edited.ends_with("<?xpacket end=\"w\"?>"));
        // The new description uses the about value that is already there.
        assert!(edited.contains("rdf:about=\"uuid:1\" xmlns:pdfuaid"));
        let state = read(&edited).unwrap();
        assert_eq!(state.part.as_deref(), Some("1"));
        assert_eq!(state.title.as_deref(), Some("A & B <1>"));
    }

    #[test]
    fn reads_part_written_as_an_attribute() {
        let xmp = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"\" xmlns:pdfuaid=\"http://www.aiim.org/pdfua/ns/id/\" pdfuaid:part=\"2\"/></rdf:RDF></x:xmpmeta>";
        assert_eq!(read(xmp).unwrap().part.as_deref(), Some("2"));
    }

    #[test]
    fn an_empty_title_is_replaced_not_duplicated() {
        let xmp = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\"> </rdf:li></rdf:Alt></dc:title></rdf:Description></rdf:RDF></x:xmpmeta>";
        assert_eq!(read(xmp).unwrap().title, None);
        let edited = edit(xmp, false, Some("Guide")).unwrap();
        assert_eq!(edited.matches("<dc:title").count(), 1);
        assert_eq!(read(&edited).unwrap().title.as_deref(), Some("Guide"));
    }

    #[test]
    fn a_self_closing_rdf_element_is_opened() {
        let xmp = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"/></x:xmpmeta>";
        let edited = edit(xmp, true, None).unwrap();
        assert_eq!(read(&edited).unwrap().part.as_deref(), Some("1"));
    }

    #[test]
    fn a_new_packet_reads_back() {
        let packet = new_packet(true, Some("Guide"));
        let state = read(&packet).unwrap();
        assert_eq!(state.part.as_deref(), Some("1"));
        assert_eq!(state.title.as_deref(), Some("Guide"));
    }

    #[test]
    fn text_that_is_not_xml_is_an_error() {
        assert!(read("not xml").is_err());
        assert!(edit("<a><b></a>", true, None).is_err());
    }
}
