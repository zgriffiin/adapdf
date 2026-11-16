//! Fixes for paragraphs: heading style (HDG-003), empty headings (HDG-004),
//! typed list markers (LST-001), empty paragraphs (TXB-004) and justified text
//! (TXT-002).

use std::collections::HashSet;

use roxmltree::Node;

use crate::docx::{DOCUMENT_PART, WordNode};
use crate::findings::Finding;
use crate::rules::lists::{MarkerKind, typed_marker};

use super::xml::{
    Splices, escape_attr, escape_text, in_alternative_content, insert_children_in_order,
    prepend_child, set_w_attr, tag, w_name,
};
use super::{Engine, Reason};

/// The children of `w:pPr` in schema order (ECMA-376, `CT_PPr`).
const PPR_ORDER: &[&str] = &[
    "pStyle",
    "keepNext",
    "keepLines",
    "pageBreakBefore",
    "framePr",
    "widowControl",
    "numPr",
    "suppressLineNumbers",
    "pBdr",
    "shd",
    "tabs",
    "suppressAutoHyphens",
    "kinsoku",
    "wordWrap",
    "overflowPunct",
    "topLinePunct",
    "autoSpaceDE",
    "autoSpaceDN",
    "bidi",
    "adjustRightInd",
    "snapToGrid",
    "spacing",
    "ind",
    "contextualSpacing",
    "mirrorIndents",
    "suppressOverlap",
    "jc",
    "textDirection",
    "textAlignment",
    "textboxTightWrap",
    "outlineLvl",
    "divId",
    "cnfStyle",
    "rPr",
    "sectPr",
    "pPrChange",
];

/// Changes to the properties of one paragraph. Several fixes can change the
/// same `w:pPr`, so they are collected and written once.
#[derive(Debug, Default)]
pub(super) struct ParagraphEdit {
    /// The id of the paragraph style to set.
    pub style: Option<String>,
    /// Set `w:jc` to `left` in the paragraph itself.
    pub left: bool,
}

/// Writes the collected changes to the properties of paragraph `p`.
pub(super) fn render_paragraph_properties(
    sp: &mut Splices,
    text: &str,
    p: Node,
    edit: &ParagraphEdit,
) {
    let style_xml = |id: &str| {
        format!(
            r#"<{} {}="{}"/>"#,
            w_name(p, "pStyle"),
            w_name(p, "val"),
            escape_attr(id)
        )
    };
    let left_xml = format!(r#"<{} {}="left"/>"#, w_name(p, "jc"), w_name(p, "val"));
    let Some(ppr) = p.w_child("pPr") else {
        let children = format!(
            "{}{}",
            edit.style.as_deref().map(style_xml).unwrap_or_default(),
            if edit.left { left_xml.as_str() } else { "" }
        );
        let ppr_name = w_name(p, "pPr");
        prepend_child(sp, text, p, &format!("<{ppr_name}>{children}</{ppr_name}>"));
        return;
    };
    let mut new: Vec<(&str, String)> = Vec::new();
    if let Some(id) = &edit.style {
        match ppr.w_child("pStyle") {
            Some(existing) => set_w_attr(sp, text, existing, "val", id),
            None => new.push(("pStyle", style_xml(id))),
        }
    }
    if edit.left {
        match ppr.w_child("jc") {
            Some(existing) => set_w_attr(sp, text, existing, "val", "left"),
            None => new.push(("jc", left_xml)),
        }
    }
    insert_children_in_order(sp, text, ppr, PPR_ORDER, &new);
}

/// Why a paragraph must not be deleted, or `None` when it holds only empty
/// space (spaces, tabs and line breaks).
fn not_empty_space(p: Node) -> Option<String> {
    for child in p.children().filter(Node::is_element) {
        if child.is_w("pPr") {
            if child.w_child("sectPr").is_some() {
                return Some("it holds a section break".to_string());
            }
        } else if child.is_w("proofErr") {
        } else if child.is_w("r") {
            for part in child.children().filter(Node::is_element) {
                let plain = part.is_w("rPr")
                    || part.is_w("tab")
                    || part.is_w("cr")
                    || part.is_w("lastRenderedPageBreak")
                    || (part.is_w("t") && part.text().is_none_or(|t| t.trim().is_empty()))
                    || (part.is_w("br")
                        && matches!(part.w_attr("type"), None | Some("textWrapping")));
                if !plain {
                    return Some(format!(
                        "it holds a {}",
                        describe_element(part.tag_name().name())
                    ));
                }
            }
        } else {
            return Some(format!(
                "it holds {}",
                match child.tag_name().name() {
                    "bookmarkStart" | "bookmarkEnd" => "a bookmark".to_string(),
                    "commentRangeStart" | "commentRangeEnd" => "a comment".to_string(),
                    other => format!("a {}", describe_element(other)),
                }
            ));
        }
    }
    None
}

fn describe_element(local: &str) -> String {
    match local {
        "br" => "page or column break".to_string(),
        "drawing" | "pict" | "object" => "picture or shape".to_string(),
        "fldChar" | "instrText" | "fldSimple" => "field".to_string(),
        other => format!("`{other}` element"),
    }
}

/// Why a paragraph cannot be removed from where it is, or `None`.
fn needed_where_it_is(p: Node) -> Option<&'static str> {
    let parent = p.parent_element()?;
    if parent.is_w("tc") && !p.next_siblings().skip(1).any(|s| s.is_w("p")) {
        return Some("it is the last paragraph in a table cell, and a cell must end with one");
    }
    if parent.is_w("body") {
        let next = p.next_siblings().skip(1).find(Node::is_element);
        let previous = p.prev_siblings().skip(1).find(Node::is_element);
        let last = next.is_none_or(|n| n.is_w("sectPr"));
        if last && previous.is_some_and(|n| n.is_w("tbl")) {
            return Some(
                "it comes after a table at the end of the document, and a document cannot end with a table",
            );
        }
    }
    None
}

/// A text-producing child of a run, in the order the model reads them.
struct Atom<'a> {
    node: Node<'a, 'a>,
    run: Node<'a, 'a>,
    kind: AtomKind,
    /// The number of characters it adds to the paragraph text.
    chars: usize,
}

#[derive(PartialEq, Eq)]
enum AtomKind {
    Text,
    Single,
    Break,
}

fn text_atoms<'a>(p: Node<'a, 'a>) -> Vec<Atom<'a>> {
    let mut atoms = Vec::new();
    for run in p.descendants().filter(|n| n.is_w("r")) {
        if in_alternative_content(run) || run.ancestors().find(|a| a.is_w("p")) != Some(p) {
            continue;
        }
        for node in run.children().filter(Node::is_element) {
            // A symbol adds a character only when its code is valid.
            let symbol = node.is_w("sym")
                && node
                    .w_attr("char")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .and_then(char::from_u32)
                    .is_some();
            let (kind, chars) = if node.is_w("t") {
                (AtomKind::Text, node.text().unwrap_or("").chars().count())
            } else if node.is_w("tab") || node.is_w("ptab") || node.is_w("noBreakHyphen") || symbol
            {
                (AtomKind::Single, 1)
            } else if node.is_w("br") || node.is_w("cr") {
                (AtomKind::Break, 1)
            } else {
                continue;
            };
            atoms.push(Atom {
                node,
                run,
                kind,
                chars,
            });
        }
    }
    atoms
}

impl Engine<'_> {
    pub(super) fn apply_heading_style(&mut self, finding: &Finding) -> Result<String, Reason> {
        let index = self.finding_paragraph(finding)?;
        let level: u8 = finding
            .suggestion
            .as_deref()
            .and_then(|s| s.strip_prefix("Heading "))
            .and_then(|n| n.parse().ok())
            .ok_or("The finding does not say which heading level to use.")?;
        self.paragraph(index)?;
        let id = self.heading_style(level)?;
        self.paragraph_edits.entry(index).or_default().style = Some(id);
        Ok(format!(
            "Applied the style Heading {level} to paragraph {}.",
            index + 1
        ))
    }

    pub(super) fn align_left(&mut self, finding: &Finding) -> Result<String, Reason> {
        let index = self.finding_paragraph(finding)?;
        self.paragraph(index)?;
        self.paragraph_edits.entry(index).or_default().left = true;
        Ok(format!("Aligned paragraph {} to the left.", index + 1))
    }

    /// Deletes paragraph `index` if it holds only empty space and the document
    /// needs no paragraph there.
    fn delete_paragraph(&mut self, index: usize, check_place: bool) -> Result<(), Reason> {
        let (_, node) = self.paragraph(index)?;
        let refusal = not_empty_space(node).or_else(|| {
            check_place
                .then(|| needed_where_it_is(node))
                .flatten()
                .map(str::to_string)
        });
        if let Some(reason) = refusal {
            return Err(format!(
                "Paragraph {} was not deleted because {reason}. Delete it in Word if it is not needed.",
                index + 1
            ));
        }
        self.splices_for(DOCUMENT_PART).delete(node.range());
        Ok(())
    }

    pub(super) fn delete_empty_heading(&mut self, finding: &Finding) -> Result<String, Reason> {
        let index = self.finding_paragraph(finding)?;
        self.delete_paragraph(index, true)?;
        Ok(format!(
            "Deleted the empty heading (paragraph {}).",
            index + 1
        ))
    }

    /// Deletes all but the first of a row of empty paragraphs (decided point 3).
    pub(super) fn delete_empty_paragraphs(&mut self, finding: &Finding) -> Result<String, Reason> {
        let first = self.finding_paragraph(finding)?;
        let row = self.doc.paragraphs[first..]
            .iter()
            .take_while(|p| !p.in_table && p.is_spacing_only())
            .count();
        let mut deleted = 0;
        let mut kept_reason = None;
        for index in first + 1..first + row {
            match self.delete_paragraph(index, false) {
                Ok(()) => deleted += 1,
                Err(reason) => kept_reason = Some(reason),
            }
        }
        match (deleted, kept_reason) {
            (0, Some(reason)) => Err(reason),
            (deleted, kept) => {
                let mut summary = format!(
                    "Deleted {deleted} of {} empty paragraphs from paragraph {}. One stays.",
                    row - 1,
                    first + 1
                );
                if let Some(reason) = kept {
                    summary.push_str(&format!(" Some were kept: {reason}"));
                }
                Ok(summary)
            }
        }
    }

    /// The id of the paragraph style with this name (case ignored).
    fn paragraph_style_named(&self, name: &str) -> Option<String> {
        let styles = self.styles?;
        styles
            .xml
            .root_element()
            .children()
            .filter(|n| n.is_w("style") && n.w_attr("type").is_none_or(|t| t == "paragraph"))
            .find(|s| {
                s.w_child("name")
                    .and_then(|n| n.w_attr("val"))
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
            })
            .and_then(|s| s.w_attr("styleId"))
            .map(str::to_string)
    }

    /// `ApplyListStyle` (LST-001): removes the typed marker and the space after
    /// it, and sets the List Number or List Bullet style.
    pub(super) fn apply_list_style(&mut self, finding: &Finding) -> Result<String, Reason> {
        let index = self.finding_paragraph(finding)?;
        let (paragraph, node) = self.paragraph(index)?;
        let marker = typed_marker(&paragraph.text)
            .ok_or("The paragraph no longer starts with a typed number or bullet.")?;
        let style_name = match marker.kind {
            MarkerKind::Number => "List Number",
            MarkerKind::Bullet => "List Bullet",
        };
        let style_id = self.paragraph_style_named(style_name).ok_or_else(|| {
            format!(
                "The document does not have the style \"{style_name}\". Apply a list in Word (Home tab > Numbering or Bullets), then delete the typed {}.",
                marker.text
            )
        })?;

        let text = &paragraph.text;
        let leading = text.chars().take_while(|c| c.is_whitespace()).count();
        let after_marker = &text.trim_start()[marker.text.len()..];
        let separator = after_marker
            .chars()
            .take_while(|c| c.is_whitespace())
            .count();
        let strip = leading + marker.text.chars().count() + separator;
        self.strip_leading_chars(node, strip)?;
        self.paragraph_edits.entry(index).or_default().style = Some(style_id);
        Ok(format!(
            "Removed the typed \"{}\" from paragraph {} and applied the style {style_name}.",
            marker.text,
            index + 1
        ))
    }

    /// Removes the first `count` characters of the text of paragraph `p`.
    fn strip_leading_chars(&mut self, p: Node, count: usize) -> Result<(), Reason> {
        let atoms = text_atoms(p);
        let mut remaining = count;
        let mut removed = HashSet::new();
        let mut partial = None;
        for (i, atom) in atoms.iter().enumerate() {
            if remaining == 0 {
                break;
            }
            match atom.kind {
                AtomKind::Break => {
                    return Err(
                        "A line break comes before the typed marker, so the marker was not removed."
                            .to_string(),
                    );
                }
                AtomKind::Single => {
                    removed.insert(i);
                    remaining -= 1;
                }
                AtomKind::Text if atom.chars <= remaining => {
                    removed.insert(i);
                    remaining -= atom.chars;
                }
                AtomKind::Text => {
                    let kept: String = atom
                        .node
                        .text()
                        .unwrap_or("")
                        .chars()
                        .skip(remaining)
                        .collect();
                    partial = Some((i, kept));
                    remaining = 0;
                }
            }
        }
        if remaining > 0 {
            return Err(
                "The typed marker could not be found in the text of the paragraph.".to_string(),
            );
        }
        let text = self.document.text;
        let removed_starts: HashSet<usize> = removed
            .iter()
            .map(|&i| atoms[i].node.range().start)
            .collect();
        let mut whole_runs = HashSet::new();
        for &i in &removed {
            let run = atoms[i].run;
            let all_gone = run
                .children()
                .filter(|c| c.is_element() && !c.is_w("rPr"))
                .all(|c| removed_starts.contains(&c.range().start));
            if all_gone && whole_runs.insert(run.range().start) {
                self.splices_for(DOCUMENT_PART).delete(run.range());
            }
        }
        for &i in &removed {
            if !whole_runs.contains(&atoms[i].run.range().start) {
                self.splices_for(DOCUMENT_PART)
                    .delete(atoms[i].node.range());
            }
        }
        if let Some((i, kept)) = partial {
            let t = tag(text, atoms[i].node);
            if let Some(close) = t.close_start {
                self.splices_for(DOCUMENT_PART)
                    .replace(t.open_end..close, escape_text(&kept));
            }
        }
        Ok(())
    }
}
