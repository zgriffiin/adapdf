//! Fixes for links: a ScreenTip for a link (LNK-004) and a table of contents
//! made without links (LNK-005).
//!
//! The ScreenTip is the text of the link itself. The tool does not make up a
//! description. A link with no text is refused.
//!
//! A `w:hyperlink` gets a `w:tooltip` attribute. A `HYPERLINK` field gets a
//! `\o "text"` switch inserted after the last word of its instruction (a simple
//! field has the instruction in `w:instr`; a complex field has it in one or more
//! `w:instrText` elements, and the switch goes in the last one). The text is
//! escaped for the field syntax (`\` and `"`) and then for XML. The `\h` switch
//! of a `TOC` field is deleted from the `w:instrText` that holds it, with the
//! spaces before it. Both are splices of the raw bytes: the rest of the
//! instruction, including how it writes quote marks, stays as it is.

use std::ops::Range;

use roxmltree::Node;

use crate::docx::{DOCUMENT_PART, W_NS, WordNode};
use crate::model::fields::{LinkSource, has_open_quote, has_switch, quoted, switch_span};
use crate::rules::links::{links_without_screen_tip, tocs_made_of_links};

use super::xml::{escape_attr, escape_text, set_attr, set_w_attr, tag};
use super::{Engine, Reason};

const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// The text of a link as a ScreenTip: runs of spaces, tabs and line breaks
/// become one space, and the ends are trimmed.
pub(super) fn screen_tip_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

const IN_WORD: &str = "In Word, right-click the link, choose Edit Hyperlink, choose ScreenTip, and type a short description of where the link goes.";

/// Why a switch cannot be added to this field instruction, or `Ok`.
fn check_can_add_screen_tip(instruction: &str) -> Result<(), Reason> {
    if has_switch(instruction, "o") {
        return Err(format!(
            "The field code of the link has an empty ScreenTip, and the tool does not change it. {IN_WORD}"
        ));
    }
    if has_open_quote(instruction) {
        return Err(format!(
            "The field code of the link is not complete (a quote mark is not closed), so the ScreenTip was not added. {IN_WORD}"
        ));
    }
    Ok(())
}

/// The switch to insert, with the space before it, as instruction text.
fn screen_tip_switch(tip: &str) -> String {
    format!(" \\o {}", quoted(tip))
}

/// The text of the raw XML `raw` with its character references and the five
/// named entities decoded, and the byte position in `raw` of each decoded
/// character (and `raw.len()` after the last one). `None` for a reference that
/// it does not know.
fn decode(raw: &str) -> Option<(String, Vec<usize>)> {
    let mut text = String::new();
    let mut starts = Vec::new();
    let mut at = 0;
    while let Some(c) = raw[at..].chars().next() {
        starts.push(at);
        if c == '&' {
            let end = at + raw[at..].find(';')?;
            let name = &raw[at + 1..end];
            text.push(match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                _ => {
                    let number = name.strip_prefix('#')?;
                    let code = match number.strip_prefix('x') {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => number.parse().ok()?,
                    };
                    char::from_u32(code)?
                }
            });
            at = end + 1;
        } else {
            text.push(c);
            at += c.len_utf8();
        }
    }
    starts.push(at);
    Some((text, starts))
}

impl<'a> Engine<'a> {
    /// The element of `word/document.xml` that has this byte range.
    fn element_at(&self, range: &Range<usize>) -> Result<Node<'a, 'a>, Reason> {
        self.document
            .xml
            .descendants()
            .find(|n| n.is_element() && n.range() == *range)
            .ok_or_else(|| "The link could not be found in the file.".to_string())
    }

    /// The `w:instrText` elements of a field, in order.
    fn instruction_nodes(&self, ranges: &[Range<usize>]) -> Result<Vec<Node<'a, 'a>>, Reason> {
        ranges
            .iter()
            .map(|range| {
                self.element_at(range)
                    .ok()
                    .filter(|n| n.is_w("instrText"))
                    .ok_or_else(|| "The field could not be found in the file.".to_string())
            })
            .collect()
    }

    /// The raw text between the tags of a `w:instrText` element, and where it
    /// starts in the part.
    fn raw_content(&self, node: Node) -> (usize, &'a str) {
        let t = tag(self.document.text, node);
        match t.close_start {
            Some(close) => (t.open_end, &self.document.text[t.open_end..close]),
            None => (t.open_end, ""),
        }
    }

    /// Gives a `w:instrText` element `xml:space="preserve"` when it has none,
    /// so that Word keeps the spaces of the instruction.
    fn preserve_space(&mut self, node: Node) {
        if node.attribute((XML_NS, "space")).is_none() {
            let text = self.document.text;
            set_attr(
                self.splices_for(DOCUMENT_PART),
                text,
                node,
                Some(XML_NS),
                "space",
                "xml:space",
                "preserve",
            );
        }
    }

    /// `AddScreenTip` (LNK-004): copies the text of the link into its ScreenTip.
    /// `position` is the place of the finding among the findings of LNK-004.
    pub(super) fn add_screen_tip(&mut self, position: usize) -> Result<String, Reason> {
        let link = links_without_screen_tip(self.doc)
            .nth(position)
            .ok_or("The link is not in the document.")?;
        let tip = screen_tip_text(&link.text);
        if tip.is_empty() {
            return Err(format!(
                "The link has no text to copy into a ScreenTip, and the tool does not make one up. {IN_WORD}"
            ));
        }
        let text = self.document.text;
        match &link.source {
            LinkSource::Element(range) => {
                let node = self.element_at(range)?;
                if node.is_w("hyperlink") {
                    set_w_attr(self.splices_for(DOCUMENT_PART), text, node, "tooltip", &tip);
                } else if node.is_w("fldSimple") {
                    let attr = node
                        .attributes()
                        .find(|a| a.namespace() == Some(W_NS) && a.name() == "instr")
                        .ok_or("The field of the link has no instruction.")?;
                    check_can_add_screen_tip(attr.value())?;
                    let range = attr.range_value();
                    let at = range.start + text[range].trim_end().len();
                    self.splices_for(DOCUMENT_PART)
                        .insert(at, escape_attr(&screen_tip_switch(&tip)));
                } else {
                    return Err("The link could not be found in the file.".to_string());
                }
            }
            LinkSource::Field(ranges) => {
                let nodes = self.instruction_nodes(ranges)?;
                let whole: String = nodes.iter().map(|n| n.text().unwrap_or("")).collect();
                check_can_add_screen_tip(&whole)?;
                let last = *nodes
                    .iter()
                    .rev()
                    .find(|n| n.text().is_some_and(|t| !t.trim().is_empty()))
                    .ok_or("The field of the link has no instruction.")?;
                let (start, raw) = self.raw_content(last);
                self.splices_for(DOCUMENT_PART).insert(
                    start + raw.trim_end().len(),
                    escape_text(&screen_tip_switch(&tip)),
                );
                self.preserve_space(last);
            }
        }
        Ok(format!(
            "Added the ScreenTip \"{tip}\" to the link in paragraph {}.",
            link.paragraph + 1
        ))
    }

    /// `RemoveTocLinks` (LNK-005): removes the `\h` switch from the `TOC` field.
    /// `position` is the place of the finding among the findings of LNK-005.
    pub(super) fn remove_toc_links(&mut self, position: usize) -> Result<String, Reason> {
        let toc = tocs_made_of_links(self.doc)
            .nth(position)
            .ok_or("The table of contents is not in the document.")?;
        for node in self.instruction_nodes(&toc.instruction_ranges)? {
            let (start, raw) = self.raw_content(node);
            let Some((decoded, starts)) = decode(raw) else {
                continue;
            };
            if decoded != node.text().unwrap_or("") {
                continue;
            }
            let Some(span) = switch_span(&decoded, "h") else {
                continue;
            };
            let char_at = |byte: usize| decoded[..byte].chars().count();
            let delete = start + starts[char_at(span.start)]..start + starts[char_at(span.end)];
            self.splices_for(DOCUMENT_PART).delete(delete);
            return Ok(format!(
                "Removed the \\h switch from the table of contents in paragraph {}. Word makes the table without links when the PDF is exported.",
                toc.first_paragraph + 1
            ));
        }
        Err("The \\h switch of the table of contents is split over several parts of the field, so the tool did not remove it. In Word, choose References > Table of Contents > Custom Table of Contents and clear \"Use hyperlinks instead of page numbers\".".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_tip_text_folds_whitespace() {
        assert_eq!(
            screen_tip_text("  Parking\t policy\r\n page  "),
            "Parking policy page"
        );
        assert_eq!(screen_tip_text(" \t\n"), "");
    }

    #[test]
    fn decode_reads_entities_and_keeps_positions() {
        let raw = " TOC \\o &quot;1-3&quot; \\h é&#38;&#x41;";
        let (text, starts) = decode(raw).unwrap();
        assert_eq!(text, " TOC \\o \"1-3\" \\h é&A");
        assert_eq!(starts.len(), text.chars().count() + 1);
        assert_eq!(*starts.last().unwrap(), raw.len());
        // The quote mark is written as 6 bytes of raw text.
        let quote = text.chars().position(|c| c == '"').unwrap();
        assert_eq!(starts[quote + 1] - starts[quote], "&quot;".len());
        assert!(decode("a &unknown; b").is_none());
        assert!(decode("a & b").is_none());
    }
}
