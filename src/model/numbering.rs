//! Automatic lists (`word/numbering.xml`).
//!
//! Only what the rules need: whether a list level is numbered or a bullet,
//! and which level a style is linked to. `w:lvlOverride` is not applied.

use std::collections::HashMap;

use super::props::NumPr;
use crate::docx::{WordNode, parse_xml};
use crate::error::Result;

#[derive(Debug, Clone)]
struct Level {
    index: u8,
    /// `w:numFmt`, for example `decimal`, `lowerLetter` or `bullet`.
    format: String,
    /// The style linked to this level (`w:pStyle`).
    style: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Numbering {
    /// `w:abstractNumId` to its levels.
    abstract_nums: HashMap<i64, Vec<Level>>,
    /// `w:numId` to `w:abstractNumId`.
    nums: HashMap<i64, i64>,
}

impl Numbering {
    pub fn parse(part: &str, text: &str) -> Result<Numbering> {
        let doc = parse_xml(part, text)?;
        let mut numbering = Numbering::default();
        for node in doc.root_element().children() {
            if node.is_w("abstractNum") {
                let Some(id) = node.w_attr("abstractNumId").and_then(|v| v.parse().ok()) else {
                    continue;
                };
                let levels = node
                    .children()
                    .filter(|n| n.is_w("lvl"))
                    .filter_map(|lvl| {
                        Some(Level {
                            index: lvl.w_attr("ilvl")?.parse().ok()?,
                            format: lvl.w_child("numFmt")?.w_attr("val")?.to_string(),
                            style: lvl
                                .w_child("pStyle")
                                .and_then(|n| n.w_attr("val"))
                                .map(str::to_string),
                        })
                    })
                    .collect();
                numbering.abstract_nums.insert(id, levels);
            } else if node.is_w("num") {
                let num_id = node.w_attr("numId").and_then(|v| v.parse().ok());
                let abstract_id = node
                    .w_child("abstractNumId")
                    .and_then(|n| n.w_attr("val"))
                    .and_then(|v| v.parse().ok());
                if let (Some(num_id), Some(abstract_id)) = (num_id, abstract_id) {
                    numbering.nums.insert(num_id, abstract_id);
                }
            }
        }
        Ok(numbering)
    }

    fn level(&self, num_id: i64, level: u8) -> Option<&Level> {
        self.abstract_nums
            .get(self.nums.get(&num_id)?)?
            .iter()
            .find(|l| l.index == level)
    }

    /// True when the list level shows numbers or letters, false for bullets,
    /// "none" and unknown lists.
    pub fn is_numbered(&self, num: NumPr) -> bool {
        self.level(num.num_id, num.level)
            .is_some_and(|l| l.format != "bullet" && l.format != "none")
    }

    /// The level that one of these styles is linked to in the list. Used
    /// when a style gives a list but not a level.
    pub fn level_for_styles(&self, num_id: i64, style_ids: &[&str]) -> Option<u8> {
        let levels = self.abstract_nums.get(self.nums.get(&num_id)?)?;
        style_ids.iter().find_map(|style| {
            levels
                .iter()
                .find(|l| l.style.as_deref() == Some(style))
                .map(|l| l.index)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
      <w:abstractNum w:abstractNumId="0">
        <w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/><w:pStyle w:val="ListNumber"/></w:lvl>
        <w:lvl w:ilvl="1"><w:numFmt w:val="lowerLetter"/><w:pStyle w:val="ListNumber2"/></w:lvl>
      </w:abstractNum>
      <w:abstractNum w:abstractNumId="1">
        <w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl>
        <w:lvl w:ilvl="1"><w:numFmt w:val="none"/></w:lvl>
      </w:abstractNum>
      <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
      <w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>
    </w:numbering>"#;

    fn num(num_id: i64, level: u8) -> NumPr {
        NumPr { num_id, level }
    }

    #[test]
    fn tells_numbers_from_bullets() {
        let n = Numbering::parse("word/numbering.xml", XML).unwrap();
        assert!(n.is_numbered(num(1, 0)));
        assert!(n.is_numbered(num(1, 1)));
        assert!(!n.is_numbered(num(2, 0)), "bullet");
        assert!(!n.is_numbered(num(2, 1)), "none");
        assert!(!n.is_numbered(num(9, 0)), "unknown list");
        assert!(!n.is_numbered(num(1, 5)), "unknown level");
    }

    #[test]
    fn finds_the_level_linked_to_a_style() {
        let n = Numbering::parse("word/numbering.xml", XML).unwrap();
        assert_eq!(n.level_for_styles(1, &["ListNumber2"]), Some(1));
        assert_eq!(n.level_for_styles(1, &["Other", "ListNumber"]), Some(0));
        assert_eq!(n.level_for_styles(1, &["Other"]), None);
        assert_eq!(n.level_for_styles(2, &["ListNumber"]), None);
    }
}
