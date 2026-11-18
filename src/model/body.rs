//! Reading `word/document.xml` into paragraphs and sections.

use roxmltree::Node;

use super::numbering::Numbering;
use super::props::{Color, ParFmt, RunFmt, parse_shading};
use super::styles::Styles;
use super::table_styles::{CellFmt, CellPos, TableProps};
use super::tables::{self, Table};
use super::theme::{Rgb, Theme};
use super::{Drawing, HeaderFooterKind, HeaderFooterRef, Paragraph, Run, Section, drawings};
use crate::docx::{DOCUMENT_PART, MC_NS, WordNode, parse_xml};
use crate::error::{Error, Result};

/// What paragraph parsing needs to resolve formatting.
pub(super) struct Context<'a> {
    pub theme: &'a Theme,
    pub styles: &'a Styles,
    pub numbering: &'a Numbering,
    /// The relationship id and part name of each header and footer.
    pub header_footer_rels: &'a [(String, String)],
}

pub(super) struct Body {
    pub paragraphs: Vec<Paragraph>,
    pub sections: Vec<Section>,
    pub tables: Vec<Table>,
    pub tracked_changes: usize,
}

/// Elements that Word writes for tracked changes.
const TRACKED_CHANGE_ELEMENTS: &[&str] = &[
    "ins",
    "del",
    "moveFrom",
    "moveTo",
    "rPrChange",
    "pPrChange",
    "sectPrChange",
    "tblPrChange",
    "tblGridChange",
    "trPrChange",
    "tcPrChange",
    "tblPrExChange",
    "numberingChange",
    "cellIns",
    "cellDel",
    "cellMerge",
];

/// Counts tracked-change elements in a parsed XML part.
pub(super) fn count_tracked_changes(doc: &roxmltree::Document) -> usize {
    doc.descendants()
        .filter(|n| TRACKED_CHANGE_ELEMENTS.iter().any(|name| n.is_w(name)))
        .count()
}

pub(super) fn parse(text: &str, ctx: &Context) -> Result<Body> {
    let doc = parse_xml(DOCUMENT_PART, text)?;
    let root = doc.root_element();
    let body = root
        .is_w("document")
        .then(|| root.w_child("body"))
        .flatten()
        .ok_or_else(|| Error::NotDocx(format!("{DOCUMENT_PART} is not a Word document body")))?;

    let mut paragraphs = Vec::new();
    let mut sections = Vec::new();
    for node in body.descendants().filter(|n| n.is_w("p")) {
        if is_alternative_content(node) {
            continue;
        }
        let ppr = node.w_child("pPr");
        let section = sections.len();
        paragraphs.push(parse_paragraph(node, ppr, paragraphs.len(), section, ctx));
        if let Some(sect) = ppr.and_then(|p| p.w_child("sectPr")) {
            sections.push(parse_section(sect, ctx));
        }
    }
    sections.push(
        body.w_child("sectPr")
            .map_or(Section::default(), |sect| parse_section(sect, ctx)),
    );

    let tables = tables::parse(body, &paragraphs);
    Ok(Body {
        paragraphs,
        tables,
        sections,
        tracked_changes: count_tracked_changes(&doc),
    })
}

/// True for paragraphs inside a text box or in the fallback copy of
/// `mc:AlternateContent`. They are not part of the body flow.
pub(super) fn is_alternative_content(p: Node) -> bool {
    p.ancestors().any(|a| {
        a.is_w("txbxContent")
            || (a.is_element()
                && a.tag_name().namespace() == Some(MC_NS)
                && a.tag_name().name() == "Fallback")
    })
}

fn parse_section(sect: Node, ctx: &Context) -> Section {
    let header_footers = sect
        .children()
        .filter(|n| n.is_w("headerReference") || n.is_w("footerReference"))
        .filter_map(|n| {
            let id = n.attribute((super::R_NS, "id"))?;
            let part = ctx
                .header_footer_rels
                .iter()
                .find(|(rel_id, _)| rel_id == id)?
                .1
                .clone();
            let kind = match n.w_attr("type") {
                Some("first") => HeaderFooterKind::First,
                Some("even") => HeaderFooterKind::Even,
                _ => HeaderFooterKind::Default,
            };
            Some(HeaderFooterRef {
                part,
                footer: n.is_w("footerReference"),
                kind,
            })
        })
        .collect();
    Section {
        columns: sect
            .w_child("cols")
            .and_then(|c| c.w_attr("num"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        header_footers,
    }
}

fn parse_paragraph(
    p: Node,
    ppr: Option<Node>,
    index: usize,
    section: usize,
    ctx: &Context,
) -> Paragraph {
    let styles = ctx.styles;
    let style_id = ppr
        .and_then(|n| n.w_child("pStyle"))
        .and_then(|n| n.w_attr("val"))
        .filter(|id| styles.get(id).is_some())
        .or_else(|| styles.default_paragraph_id());

    let table = table_layer(p, styles);
    let direct = ParFmt::parse(ppr, ctx.theme);
    let mut par = styles.paragraph_par_fmt_under(&table.par, style_id);
    par.overlay(&direct);
    // A style that gives a list but no level is linked to a level in the list.
    if direct.num_id.is_none()
        && par.num_level.is_none()
        && let (Some(num_id), Some(style_id)) = (par.num_id, style_id)
    {
        let chain: Vec<&str> = styles
            .chain(style_id)
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        par.num_level = ctx.numbering.level_for_styles(num_id, &chain);
    }

    let base_run = styles.paragraph_run_fmt_under(&table.run, style_id);
    let mut runs = Vec::new();
    let mut drawings = Vec::new();
    let mut text = String::new();
    for run in p.descendants().filter(|n| n.is_w("r")) {
        if is_alternative_content(run) || !belongs_to(run, p) {
            continue;
        }
        let mut run_text = String::new();
        scan_run(run, &mut run_text, &mut drawings);

        let mut fmt = base_run.clone();
        if let Some(char_style) = run
            .w_child("rPr")
            .and_then(|r| r.w_child("rStyle"))
            .and_then(|n| n.w_attr("val"))
        {
            fmt.overlay(&styles.style_run_fmt(char_style));
        }
        fmt.overlay(&RunFmt::parse(run.w_child("rPr"), ctx.theme));

        text.push_str(&run_text);
        runs.push(Run {
            text: run_text,
            props: fmt.resolve(),
            char_style: run
                .w_child("rPr")
                .and_then(|r| r.w_child("rStyle"))
                .and_then(|n| n.w_attr("val"))
                .map(str::to_string),
            in_hyperlink: run.ancestors().any(|a| a.is_w("hyperlink")),
        });
    }

    let extras = drawings::extras(p, ppr);
    Paragraph {
        index,
        style_id: style_id.map(str::to_string),
        style_name: style_id
            .and_then(|id| styles.get(id))
            .map(|s| s.name.clone()),
        heading_level: style_id.and_then(|id| styles.heading_level(id)),
        text,
        runs,
        props: par.resolve(),
        style_run: base_run.resolve(),
        in_table: p.ancestors().any(|a| a.is_w("tbl")),
        container_fill: container_fill(p, ctx),
        drawings,
        equations: extras.equations,
        has_break_or_field: extras.has_break_or_field,
        section,
        range: p.range(),
    }
}

/// What the table style of the table around the paragraph gives to its text.
/// Only the nearest table counts: a nested table does not take the style of
/// the table it is in.
fn table_layer(p: Node, styles: &Styles) -> CellFmt {
    let Some(tc) = p.ancestors().find(|a| a.is_w("tc")) else {
        return CellFmt::default();
    };
    let Some((tbl, pos)) = CellPos::of(tc) else {
        return CellFmt::default();
    };
    styles.table_cell_fmt(&TableProps::parse(tbl), pos)
}

/// The fill behind a paragraph that comes from the table around it. Working
/// outwards through nested tables, for each cell in turn: the shading of the
/// cell itself (`w:tcPr/w:shd`); else the shading of its table
/// (`w:tblPr/w:shd`); else the fill that the table style gives to the cell
/// (see [`super::table_styles`]). A cell shading of "no fill" (`fill="auto"`)
/// also overrides the table style, and the fill of the cell around it shows.
fn container_fill(p: Node, ctx: &Context) -> Option<Rgb> {
    let mut cell: Option<Node> = None;
    let mut no_fill = false;
    for ancestor in p.ancestors() {
        let direct = |pr: Option<Node>| {
            pr.and_then(|pr| pr.w_child("shd"))
                .and_then(|shd| parse_shading(&shd, ctx.theme))
        };
        if ancestor.is_w("tc") {
            cell = Some(ancestor);
            no_fill = false;
            match direct(ancestor.w_child("tcPr")) {
                Some(Color::Rgb(rgb)) => return Some(rgb),
                Some(Color::Auto) => no_fill = true,
                None => {}
            }
        } else if ancestor.is_w("tbl") {
            match direct(ancestor.w_child("tblPr")) {
                Some(Color::Rgb(rgb)) => return Some(rgb),
                Some(Color::Auto) => continue,
                None => {}
            }
            if no_fill {
                continue;
            }
            if let Some((_, pos)) = cell.and_then(CellPos::of) {
                let fmt = ctx.styles.table_cell_fmt(&TableProps::parse(ancestor), pos);
                if let Some(Color::Rgb(rgb)) = fmt.fill {
                    return Some(rgb);
                }
            }
        }
    }
    None
}

/// True when the nearest `w:p` above the run is `p` (so the run is not inside
/// a paragraph nested in a text box).
pub(super) fn belongs_to(run: Node, p: Node) -> bool {
    run.ancestors().find(|a| a.is_w("p")) == Some(p)
}

/// Collects the text and drawings of one run.
pub(super) fn scan_run(run: Node, text: &mut String, drawings: &mut Vec<Drawing>) {
    for child in run.children().filter(Node::is_element) {
        if child.is_w("t") {
            text.push_str(child.text().unwrap_or(""));
        } else if child.is_w("tab") || child.is_w("ptab") {
            text.push('\t');
        } else if child.is_w("br") || child.is_w("cr") {
            text.push('\n');
        } else if child.is_w("noBreakHyphen") {
            text.push('-');
        } else if child.is_w("sym") {
            // A symbol such as a typed bullet in the Symbol font.
            if let Some(c) = child
                .w_attr("char")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .and_then(char::from_u32)
            {
                text.push(c);
            }
        } else if child.is_w("drawing") {
            drawings.push(drawings::drawing_ml(child));
        } else if child.is_w("pict") || child.is_w("object") {
            drawings.push(drawings::legacy(child));
        } else if child.is_element()
            && child.tag_name().namespace() == Some(MC_NS)
            && child.tag_name().name() == "AlternateContent"
        {
            for choice in child
                .children()
                .filter(|c| c.is_element() && c.tag_name().name() == "Choice")
            {
                scan_run(choice, text, drawings);
            }
        }
    }
}
