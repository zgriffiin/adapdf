//! Styles (`word/styles.xml`): definitions and inheritance.

use std::collections::{HashMap, HashSet};

use super::props::{ParFmt, RunFmt};
use super::table_styles::{TableStyle, TableStyles};
use super::theme::Theme;
use crate::docx::{WordNode, parse_xml};
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKind {
    Paragraph,
    Character,
    Table,
    Numbering,
}

#[derive(Debug, Clone)]
pub struct Style {
    pub id: String,
    /// The name Word shows (for built-in styles, lower case such as `heading 1`).
    pub name: String,
    pub kind: StyleKind,
    pub based_on: Option<String>,
    /// The style used when a paragraph names none (usually Normal).
    pub is_default: bool,
    /// True for styles made by a user (`w:customStyle`).
    pub is_custom: bool,
    pub par: ParFmt,
    pub run: RunFmt,
}

impl Style {
    /// 1 to 9 for the built-in styles Heading 1 to Heading 9.
    pub fn heading_level(&self) -> Option<u8> {
        let lower = self.name.to_ascii_lowercase();
        let digit = lower.strip_prefix("heading ")?;
        match digit.parse::<u8>() {
            Ok(level @ 1..=9) if digit.len() == 1 => Some(level),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Styles {
    doc_run: RunFmt,
    doc_par: ParFmt,
    styles: HashMap<String, Style>,
    default_paragraph: Option<String>,
    table_styles: TableStyles,
    default_table: Option<String>,
}

impl Styles {
    pub fn parse(part: &str, text: &str, theme: &Theme) -> Result<Styles> {
        let doc = parse_xml(part, text)?;
        let mut styles = Styles::default();
        let root = doc.root_element();
        if let Some(defaults) = root.w_child("docDefaults") {
            let rpr = defaults
                .w_child("rPrDefault")
                .and_then(|n| n.w_child("rPr"));
            let ppr = defaults
                .w_child("pPrDefault")
                .and_then(|n| n.w_child("pPr"));
            styles.doc_run = RunFmt::parse(rpr, theme);
            styles.doc_par = ParFmt::parse(ppr, theme);
        }
        for node in root.children().filter(|n| n.is_w("style")) {
            let Some(id) = node.w_attr("styleId") else {
                continue;
            };
            let kind = match node.w_attr("type") {
                Some("character") => StyleKind::Character,
                Some("table") => StyleKind::Table,
                Some("numbering") => StyleKind::Numbering,
                _ => StyleKind::Paragraph,
            };
            let flag = |name: &str| matches!(node.w_attr(name), Some("1" | "true" | "on"));
            let style = Style {
                id: id.to_string(),
                name: node
                    .w_child("name")
                    .and_then(|n| n.w_attr("val"))
                    .unwrap_or(id)
                    .to_string(),
                kind,
                based_on: node
                    .w_child("basedOn")
                    .and_then(|n| n.w_attr("val"))
                    .map(str::to_string),
                is_default: flag("default"),
                is_custom: flag("customStyle"),
                par: ParFmt::parse(node.w_child("pPr"), theme),
                run: RunFmt::parse(node.w_child("rPr"), theme),
            };
            if style.is_default && kind == StyleKind::Paragraph {
                styles
                    .default_paragraph
                    .get_or_insert_with(|| id.to_string());
            }
            if kind == StyleKind::Table {
                styles
                    .table_styles
                    .insert(id.to_string(), TableStyle::parse(node, theme));
                if style.is_default {
                    styles.default_table.get_or_insert_with(|| id.to_string());
                }
            }
            styles.styles.insert(id.to_string(), style);
        }
        Ok(styles)
    }

    pub fn get(&self, id: &str) -> Option<&Style> {
        self.styles.get(id)
    }

    /// The id of the paragraph style used when a paragraph names none.
    pub fn default_paragraph_id(&self) -> Option<&str> {
        self.default_paragraph.as_deref()
    }

    pub(super) fn table_style(&self, id: &str) -> Option<&TableStyle> {
        self.table_styles.get(id)
    }

    /// The id of the table style used when a table names none.
    pub(super) fn default_table_id(&self) -> Option<&str> {
        self.default_table.as_deref()
    }

    /// The style and its base styles, the style itself first. Stops at a
    /// missing style or a loop.
    pub fn chain<'a>(&'a self, id: &str) -> Vec<&'a Style> {
        let mut seen = HashSet::new();
        let mut chain = Vec::new();
        let mut next = self.styles.get(id);
        while let Some(style) = next {
            if !seen.insert(style.id.as_str()) {
                break;
            }
            chain.push(style);
            next = style.based_on.as_deref().and_then(|b| self.styles.get(b));
        }
        chain
    }

    /// Run properties from the style chain only (base style first).
    pub fn style_run_fmt(&self, id: &str) -> RunFmt {
        let mut fmt = RunFmt::default();
        for style in self.chain(id).into_iter().rev() {
            fmt.overlay(&style.run);
        }
        fmt
    }

    /// Paragraph properties from the style chain only (base style first).
    pub fn style_par_fmt(&self, id: &str) -> ParFmt {
        let mut fmt = ParFmt::default();
        for style in self.chain(id).into_iter().rev() {
            fmt.overlay(&style.par);
        }
        fmt
    }

    /// Run properties of text in a paragraph style: document defaults, then
    /// the style chain.
    pub fn paragraph_run_fmt(&self, paragraph_style: Option<&str>) -> RunFmt {
        self.paragraph_run_fmt_under(&RunFmt::default(), paragraph_style)
    }

    /// Like [`Styles::paragraph_run_fmt`] for text in a table cell: document
    /// defaults, then the table style (`table`), then the paragraph style chain.
    pub fn paragraph_run_fmt_under(&self, table: &RunFmt, paragraph_style: Option<&str>) -> RunFmt {
        let mut fmt = self.doc_run.clone();
        fmt.overlay(table);
        if let Some(id) = paragraph_style {
            fmt.overlay(&self.style_run_fmt(id));
        }
        fmt
    }

    /// Paragraph properties of a paragraph style: document defaults, then the
    /// style chain.
    pub fn paragraph_par_fmt(&self, paragraph_style: Option<&str>) -> ParFmt {
        self.paragraph_par_fmt_under(&ParFmt::default(), paragraph_style)
    }

    /// Like [`Styles::paragraph_par_fmt`] for a paragraph in a table cell:
    /// document defaults, then the table style (`table`), then the paragraph
    /// style chain.
    pub fn paragraph_par_fmt_under(&self, table: &ParFmt, paragraph_style: Option<&str>) -> ParFmt {
        let mut fmt = self.doc_par.clone();
        fmt.overlay(table);
        if let Some(id) = paragraph_style {
            fmt.overlay(&self.style_par_fmt(id));
        }
        fmt
    }

    /// The heading level (1 to 9) when the style is a built-in Heading style.
    pub fn heading_level(&self, id: &str) -> Option<u8> {
        self.styles.get(id)?.heading_level()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::props::{Alignment, Color};

    const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

    fn parse(inner: &str) -> Styles {
        let xml = format!("<w:styles {NS}>{inner}</w:styles>");
        Styles::parse("word/styles.xml", &xml, &Theme::default()).unwrap()
    }

    const SAMPLE: &str = r#"
      <w:docDefaults>
        <w:rPrDefault><w:rPr><w:sz w:val="22"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault>
        <w:pPrDefault><w:pPr><w:jc w:val="left"/></w:pPr></w:pPrDefault>
      </w:docDefaults>
      <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
      <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/>
        <w:pPr><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style>
      <w:style w:type="paragraph" w:customStyle="1" w:styleId="Fancy"><w:name w:val="Fancy"/><w:basedOn w:val="Heading1"/>
        <w:rPr><w:b w:val="0"/><w:color w:val="FF0000"/></w:rPr><w:pPr><w:jc w:val="both"/></w:pPr></w:style>
      <w:style w:type="character" w:styleId="Strong"><w:name w:val="Strong"/><w:rPr><w:b/></w:rPr></w:style>
      <w:style w:type="paragraph" w:styleId="LoopA"><w:name w:val="a"/><w:basedOn w:val="LoopB"/></w:style>
      <w:style w:type="paragraph" w:styleId="LoopB"><w:name w:val="b"/><w:basedOn w:val="LoopA"/></w:style>"#;

    #[test]
    fn finds_the_default_paragraph_style() {
        let s = parse(SAMPLE);
        assert_eq!(s.default_paragraph_id(), Some("Normal"));
    }

    #[test]
    fn inherits_through_based_on_chains() {
        let s = parse(SAMPLE);
        let run = s.paragraph_run_fmt(Some("Fancy")).resolve();
        assert!(!run.bold, "Fancy turns bold off");
        assert_eq!(run.size_half_points, 32, "size comes from Heading1");
        assert!(matches!(run.color, Color::Rgb(_)));
        assert_eq!(run.lang.val.as_deref(), Some("en-US"), "from docDefaults");
        let par = s.paragraph_par_fmt(Some("Fancy")).resolve();
        assert_eq!(par.outline_level, Some(1), "inherited from Heading1");
        assert_eq!(par.alignment, Alignment::Justify);
    }

    #[test]
    fn heading_levels_come_from_the_built_in_names() {
        let s = parse(SAMPLE);
        assert_eq!(s.heading_level("Heading1"), Some(1));
        assert_eq!(s.heading_level("Fancy"), None);
        assert_eq!(s.heading_level("Normal"), None);
        assert_eq!(s.heading_level("Missing"), None);
    }

    #[test]
    fn heading_names_must_be_exact() {
        let mk = |name: &str| Style {
            id: "x".into(),
            name: name.into(),
            kind: StyleKind::Paragraph,
            based_on: None,
            is_default: false,
            is_custom: false,
            par: ParFmt::default(),
            run: RunFmt::default(),
        };
        assert_eq!(mk("Heading 3").heading_level(), Some(3));
        assert_eq!(mk("heading 9").heading_level(), Some(9));
        assert_eq!(mk("heading 10").heading_level(), None);
        assert_eq!(mk("heading 0").heading_level(), None);
        assert_eq!(mk("heading").heading_level(), None);
        assert_eq!(mk("Heading Style").heading_level(), None);
    }

    #[test]
    fn a_style_loop_does_not_hang() {
        let s = parse(SAMPLE);
        assert_eq!(s.chain("LoopA").len(), 2);
    }

    #[test]
    fn flags_custom_styles() {
        let s = parse(SAMPLE);
        assert!(s.get("Fancy").unwrap().is_custom);
        assert!(!s.get("Heading1").unwrap().is_custom);
        assert_eq!(s.get("Strong").unwrap().kind, StyleKind::Character);
    }
}
