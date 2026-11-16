//! Fixes for pictures, charts, shapes, groups and canvases: alt text and the
//! decorative mark (IMG-001, IMG-002), inline layout (IMG-004) and arrows on
//! screenshots (IMG-007).
//!
//! The model gives each drawing a place: the paragraph and the number of the
//! drawing in it. [`drawing_nodes`] lists the XML elements in the same order.
//!
//! A DrawingML shape, group or canvas that Word wrote has a VML copy in
//! `mc:Fallback`. The model reads only the `mc:Choice` copy. Alt text goes in
//! both copies, so that they agree (decided point 2). The decorative mark exists
//! only in DrawingML.

use roxmltree::Node;

use crate::docx::{DOCUMENT_PART, MC_NS, WordNode};
use crate::findings::{Finding, RuleId};
use crate::model::{Drawing, DrawingKind, ObjectKind};
use crate::rules::images::{img_001_hits, img_002_hits, img_004_hits, screenshots};

use super::xml::{
    Splices, append_child, in_alternative_content, prefix_of, set_attr, source_name, tag,
};
use super::{Engine, Reason};

const WP_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const ADEC_NS: &str = "http://schemas.microsoft.com/office/drawing/2017/decorative";
const V_NS: &str = "urn:schemas-microsoft-com:vml";
/// The `a:ext` URI of the decorative mark.
const DECORATIVE_URI: &str = "{C183D7F6-B498-43B3-948B-1728B52AA6E4}";

/// VML elements that are a shape (the same list as the model).
const VML_SHAPES: &[&str] = &[
    "shape",
    "group",
    "rect",
    "roundrect",
    "oval",
    "line",
    "polyline",
    "curve",
    "arc",
    "image",
];

/// A drawing element in the XML.
#[derive(Clone, Copy)]
struct DrawingNode<'a> {
    /// `w:drawing`, `w:pict` or `w:object`.
    node: Node<'a, 'a>,
    /// The `mc:AlternateContent` that holds it in its `mc:Choice`.
    alternate: Option<Node<'a, 'a>>,
}

fn is(node: Node, ns: &str, local: &str) -> bool {
    node.is_element() && node.tag_name().namespace() == Some(ns) && node.tag_name().name() == local
}

/// The drawings of a paragraph, in the order the model lists them.
fn drawing_nodes<'a>(p: Node<'a, 'a>) -> Vec<DrawingNode<'a>> {
    fn scan<'a>(
        container: Node<'a, 'a>,
        alternate: Option<Node<'a, 'a>>,
        out: &mut Vec<DrawingNode<'a>>,
    ) {
        for child in container.children().filter(Node::is_element) {
            if child.is_w("drawing") || child.is_w("pict") || child.is_w("object") {
                out.push(DrawingNode {
                    node: child,
                    alternate,
                });
            } else if is(child, MC_NS, "AlternateContent") {
                for choice in child.children().filter(|c| is(*c, MC_NS, "Choice")) {
                    scan(choice, Some(child), out);
                }
            }
        }
    }
    let mut out = Vec::new();
    for run in p.descendants().filter(|n| n.is_w("r")) {
        if in_alternative_content(run) || run.ancestors().find(|a| a.is_w("p")) != Some(p) {
            continue;
        }
        scan(run, None, &mut out);
    }
    out
}

/// The first VML shape in a `w:pict`.
fn vml_shape<'a>(pict: Node<'a, 'a>) -> Option<Node<'a, 'a>> {
    pict.descendants().find(|n| {
        n.is_element()
            && n.tag_name().namespace() == Some(V_NS)
            && VML_SHAPES.contains(&n.tag_name().name())
    })
}

/// The `wp:docPr` of a `w:drawing`.
fn doc_pr<'a>(drawing: Node<'a, 'a>) -> Option<Node<'a, 'a>> {
    drawing
        .children()
        .find(|n| is(*n, WP_NS, "inline") || is(*n, WP_NS, "anchor"))?
        .children()
        .find(|n| is(*n, WP_NS, "docPr"))
}

fn label(object: ObjectKind) -> &'static str {
    match object {
        ObjectKind::Picture => "picture",
        ObjectKind::Chart => "chart",
        ObjectKind::Shape => "shape",
        ObjectKind::Group => "group of shapes",
        ObjectKind::Canvas => "drawing canvas",
        ObjectKind::SmartArt => "SmartArt graphic",
        ObjectKind::Ole => "embedded object",
    }
}

/// Marks a drawing decorative: `adec:decorative` with `val="1"` in the
/// `a:extLst` of its `wp:docPr`, the way Word writes it.
fn mark_decorative(sp: &mut Splices, text: &str, doc_pr: Node) {
    let ext = |a: &str| {
        format!(
            r#"<{a}ext uri="{DECORATIVE_URI}"><adec:decorative xmlns:adec="{ADEC_NS}" val="1"/></{a}ext>"#
        )
    };
    match doc_pr.children().find(|n| is(*n, A_NS, "extLst")) {
        Some(list) => {
            let decorative = list.descendants().find(|n| is(*n, ADEC_NS, "decorative"));
            match decorative {
                Some(mark) => set_attr(sp, text, mark, None, "val", "val", "1"),
                None => {
                    let a = prefix_of(list, A_NS).map_or(String::new(), |p| format!("{p}:"));
                    append_child(sp, text, list, &ext(&a));
                }
            }
        }
        None => {
            let (declaration, a) = match prefix_of(doc_pr, A_NS) {
                Some(p) => (String::new(), p.to_string()),
                None => (format!(r#" xmlns:a="{A_NS}""#), "a".to_string()),
            };
            let xml = format!(
                "<{a}:extLst{declaration}>{}</{a}:extLst>",
                ext(&format!("{a}:"))
            );
            append_child(sp, text, doc_pr, &xml);
        }
    }
}

impl<'a> Engine<'a> {
    /// The XML element of drawing number `slot` in paragraph `paragraph`.
    fn drawing_node(
        &self,
        paragraph: usize,
        slot: usize,
        model: &Drawing,
    ) -> Result<DrawingNode<'a>, Reason> {
        let (_, p) = self.paragraph(paragraph)?;
        drawing_nodes(p)
            .get(slot)
            .copied()
            .filter(|d| d.node.is_w("drawing") == (model.kind != DrawingKind::Legacy))
            .ok_or_else(|| "The drawing could not be found in the file.".to_string())
    }

    /// `SetAltText` (IMG-001, IMG-002). `text` is the typed alt text; `None`
    /// means "mark as decorative".
    pub(super) fn set_alt_text(
        &mut self,
        finding: &Finding,
        position: usize,
        text: Option<&str>,
    ) -> Result<String, Reason> {
        let hits = match finding.rule_id {
            RuleId::Img001 => img_001_hits(self.doc),
            RuleId::Img002 => img_002_hits(self.doc),
            other => return Err(format!("{other} does not have an alt text fix.")),
        };
        let (paragraph, slot, model) = *hits
            .get(position)
            .ok_or("The drawing is not in the document.")?;
        let drawing = self.drawing_node(paragraph.index, slot.1, model)?;
        let source = self.document.text;
        let place = format!("paragraph {}", paragraph.index + 1);
        let sp = self.splices_for(DOCUMENT_PART);
        match text {
            Some(alt) => {
                match model.kind {
                    DrawingKind::Legacy => {
                        let shape = vml_shape(drawing.node)
                            .ok_or("The picture has no shape that can hold alt text.")?;
                        set_attr(sp, source, shape, None, "alt", "alt", alt);
                    }
                    _ => {
                        let pr = doc_pr(drawing.node)
                            .ok_or("The drawing has no properties element for alt text.")?;
                        set_attr(sp, source, pr, None, "descr", "descr", alt);
                    }
                }
                // The older copy of a shape, group or canvas.
                let fallback = drawing
                    .alternate
                    .and_then(|a| a.children().find(|c| is(*c, MC_NS, "Fallback")))
                    .and_then(|f| f.descendants().find(|n| n.is_w("pict")))
                    .and_then(vml_shape);
                if let Some(shape) = fallback {
                    set_attr(sp, source, shape, None, "alt", "alt", alt);
                }
                Ok(format!(
                    "Wrote the alt text of the {} in {place}.",
                    label(model.object)
                ))
            }
            None => {
                if model.kind == DrawingKind::Legacy {
                    return Err(format!(
                        "The {} in {place} is in the older VML format, which has no decorative mark. Type alt text for it, or mark it decorative in Word.",
                        label(model.object)
                    ));
                }
                let pr = doc_pr(drawing.node)
                    .ok_or("The drawing has no properties element for the decorative mark.")?;
                mark_decorative(sp, source, pr);
                if pr.attribute("descr").is_some_and(|d| !d.is_empty()) {
                    set_attr(sp, source, pr, None, "descr", "descr", "");
                }
                Ok(format!(
                    "Marked the {} in {place} as decorative.",
                    label(model.object)
                ))
            }
        }
    }

    /// `MakeImageInline` (IMG-004), for pictures, charts and SmartArt.
    pub(super) fn make_inline(&mut self, position: usize) -> Result<String, Reason> {
        let hits = img_004_hits(self.doc);
        let (paragraph, slot, model) = *hits
            .get(position)
            .ok_or("The drawing is not in the document.")?;
        let what = label(model.object);
        let in_word = format!(
            "In Word: select the {what}, choose Layout Options, and select In Line with Text."
        );
        if !matches!(
            model.object,
            ObjectKind::Picture | ObjectKind::Chart | ObjectKind::SmartArt
        ) {
            return Err(format!(
                "The tool does not make a {what} in line with the text. Word may move the shapes inside it, and a single shape loses its place. {in_word}"
            ));
        }
        if model.kind == DrawingKind::Legacy {
            return Err(format!(
                "This {what} is in the older VML format. The tool does not change it. {in_word}"
            ));
        }
        let drawing = self.drawing_node(paragraph.index, slot.1, model)?;
        if drawing.alternate.is_some() {
            return Err(format!(
                "This {what} also has a copy in an older format. The tool does not change it. {in_word}"
            ));
        }
        let holder = drawing
            .node
            .children()
            .find(|n| is(*n, WP_NS, "anchor"))
            .ok_or("The drawing is not floating in the file.")?;
        let source = self.document.text;
        let t = tag(source, holder);
        let Some(close) = t.close_start else {
            return Err("The drawing has no content to keep.".to_string());
        };
        let name = source_name(source, holder);
        let prefix = name
            .rsplit_once(':')
            .map_or(String::new(), |(p, _)| format!("{p}:"));
        let sp = self.splices_for(DOCUMENT_PART);
        sp.replace(
            t.start..t.open_end,
            format!(r#"<{prefix}inline distT="0" distB="0" distL="0" distR="0">"#),
        );
        sp.replace(close..t.end, format!("</{prefix}inline>"));
        // An inline object has only these children. The position, the wrapping
        // and the other settings of a floating object are removed.
        for child in holder.children().filter(Node::is_element) {
            let kept = (child.tag_name().namespace() == Some(WP_NS)
                && matches!(
                    child.tag_name().name(),
                    "extent" | "effectExtent" | "docPr" | "cNvGraphicFramePr"
                ))
                || is(child, A_NS, "graphic");
            if !kept {
                sp.delete(child.range());
            }
        }
        Ok(format!(
            "Made the {what} in paragraph {} in line with the text.",
            paragraph.index + 1
        ))
    }

    /// `MarkAnnotationsDecorative` (IMG-007): the user confirmed that the alt
    /// text of the screenshot describes the arrows and boxes.
    pub(super) fn mark_annotations_decorative(
        &mut self,
        position: usize,
    ) -> Result<String, Reason> {
        let all = screenshots(self.doc);
        let screenshot = all
            .iter()
            .filter(|s| s.unresolved().next().is_some())
            .nth(position)
            .ok_or("The screenshot is not in the document.")?;
        let picture_without_alt =
            screenshot.paragraph.drawings.iter().any(|d| {
                d.object == ObjectKind::Picture && !d.decorative && d.alt_text().is_none()
            });
        if picture_without_alt {
            return Err("The screenshot has no alt text yet. Give the screenshot alt text first (IMG-001), because the arrows and boxes are marked decorative only when its alt text describes them. Then run the fix again.".to_string());
        }
        let slots: Vec<(usize, usize)> = screenshot.unresolved().collect();
        let paragraph_number = screenshot.paragraph.index + 1;
        let mut targets = Vec::new();
        for (paragraph, slot) in &slots {
            let model = &self.doc.paragraphs[*paragraph].drawings[*slot];
            if model.kind == DrawingKind::Legacy {
                return Err(format!(
                    "The arrows and boxes on the screenshot in paragraph {paragraph_number} are in the older VML format, and VML shapes cannot be marked decorative. Either give each shape alt text in Word (right-click it, then Edit Alt Text), or convert the file to the current format (open it in Word, then File > Info > Convert), then run the fix again."
                ));
            }
            let drawing = self.drawing_node(*paragraph, *slot, model)?;
            let pr = doc_pr(drawing.node)
                .ok_or("A shape has no properties element for the decorative mark.")?;
            targets.push(pr);
        }
        let source = self.document.text;
        let sp = self.splices_for(DOCUMENT_PART);
        for pr in &targets {
            mark_decorative(sp, source, *pr);
        }
        Ok(format!(
            "Marked {} arrow(s) or box(es) on the screenshot in paragraph {paragraph_number} as decorative.",
            targets.len()
        ))
    }
}
