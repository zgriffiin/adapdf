//! Pictures, charts, shapes, text boxes, embedded objects and equations.
//!
//! A [`Drawing`] is one `w:drawing` (DrawingML), `w:pict` (VML) or `w:object`
//! in a run. Only the copy that Word reads is kept: for
//! `mc:AlternateContent` the model reads `mc:Choice` and ignores
//! `mc:Fallback` (see `scan_run` in `body.rs`).
//!
//! Text boxes: paragraphs inside `w:txbxContent` are not body paragraphs (see
//! `is_alternative_content` in `body.rs`). Their text is kept only here, in
//! [`Drawing::text`]. Pictures and other drawings inside a text box are not
//! read.
//!
//! Groups and canvases: a group of shapes (DrawingML `wpg:wgp`, VML
//! `v:group`) and a drawing canvas (`wpc:wpc`) are ONE drawing. The alt text,
//! the decorative mark and the position come from the group's or canvas's own
//! `wp:docPr` (or the VML group's attributes). The pictures and shapes inside
//! are not drawings of their own. Text in their shapes is in
//! [`Drawing::text`], as for a text box.

use roxmltree::Node;

use super::props::toggle;
use crate::docx::WordNode;

const WP_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
/// The extension Word writes when "Mark as decorative" is on.
const ADEC_NS: &str = "http://schemas.microsoft.com/office/drawing/2017/decorative";
const V_NS: &str = "urn:schemas-microsoft-com:vml";
const O_NS: &str = "urn:schemas-microsoft-com:office:office";
const M_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/math";

/// VML elements that are a shape in their own right.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingKind {
    /// A DrawingML object "in line with text".
    Inline,
    /// A DrawingML object that floats (`wp:anchor`).
    Anchored,
    /// An older VML picture or an embedded object.
    Legacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// A picture: DrawingML `pic:pic`, or a VML shape with `v:imagedata`.
    /// A drawing with no graphic data is also a picture.
    Picture,
    Chart,
    /// A shape or other drawing that is not a picture, chart, SmartArt, group
    /// or canvas. A text box is a shape with [`Drawing::text_box`] set.
    Shape,
    /// A group of shapes and pictures (DrawingML `wpg:wgp`, VML `v:group`).
    Group,
    /// A drawing canvas (`wpc:wpc`). A flowchart made in Word is often one.
    Canvas,
    SmartArt,
    /// An embedded object (OLE): `w:object`, or VML with `o:OLEObject`.
    Ole,
}

/// A picture, shape, chart, text box or embedded object in a paragraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawing {
    pub kind: DrawingKind,
    pub object: ObjectKind,
    /// Not "In line with text". DrawingML: `wp:anchor`. VML: the style says
    /// `position:absolute`.
    pub floating: bool,
    /// The drawing holds a text box (`w:txbxContent`).
    pub text_box: bool,
    /// The text in the text box, or in all the shapes of a group or canvas,
    /// paragraphs joined with a space, trimmed.
    pub text: String,
    /// The description: `wp:docPr/@descr`, or VML `@alt`.
    pub description: Option<String>,
    /// `wp:docPr/@title`, or VML `o:title`.
    pub title: Option<String>,
    /// Marked decorative (`adec:decorative` with `val="1"`). VML has no such
    /// mark.
    pub decorative: bool,
}

impl Drawing {
    /// The alt text: the description, or the title when the description is
    /// empty. Trimmed. `None` when both are empty or the drawing is marked
    /// decorative (a decorative drawing is not exported with alt text).
    pub fn alt_text(&self) -> Option<&str> {
        if self.decorative {
            return None;
        }
        [&self.description, &self.title]
            .into_iter()
            .flatten()
            .map(|t| t.trim())
            .find(|t| !t.is_empty())
    }

    /// True for a group or a canvas.
    pub fn is_container(&self) -> bool {
        matches!(self.object, ObjectKind::Group | ObjectKind::Canvas)
    }

    /// True when the drawing holds text in a text box or in the shapes of a
    /// group or canvas. TXB-001 reports it.
    pub fn has_box_text(&self) -> bool {
        self.text_box && !self.text.is_empty()
    }

    /// True for a single text box that has text (not a group or canvas). The
    /// text is the content: TXB-001 reports it, so IMG-001 does not ask for
    /// alt text and IMG-004 does not report it.
    pub fn is_text_box_with_text(&self) -> bool {
        self.has_box_text() && !self.is_container()
    }

    /// True when IMG-001 expects alt text or a decorative mark: a picture,
    /// chart, shape, group, canvas or SmartArt that is not a text box with
    /// text. A group or canvas always needs it, also when its shapes have
    /// text. Embedded objects are reported by IMG-006.
    pub fn needs_alt_text(&self) -> bool {
        self.object != ObjectKind::Ole && !self.is_text_box_with_text()
    }
}

fn is(node: Node, ns: &str, local: &str) -> bool {
    node.is_element() && node.tag_name().namespace() == Some(ns) && node.tag_name().name() == local
}

/// The text of all text boxes under `node`, paragraphs joined with a space.
fn text_box_text(node: Node) -> String {
    let paragraphs = node
        .descendants()
        .filter(|n| n.is_w("txbxContent"))
        .flat_map(|content| content.descendants().filter(|n| n.is_w("p")))
        .map(|p| {
            p.descendants()
                .filter(|n| n.is_w("t"))
                .filter_map(|t| t.text())
                .collect::<String>()
        });
    paragraphs
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value.filter(|v| !v.is_empty()).map(str::to_string)
}

/// A `w:drawing` element.
pub(super) fn drawing_ml(node: Node) -> Drawing {
    let holder = node
        .children()
        .find(|n| is(*n, WP_NS, "inline") || is(*n, WP_NS, "anchor"));
    let floating = holder.is_some_and(|h| h.tag_name().name() == "anchor");
    let doc_pr = holder.and_then(|h| h.children().find(|n| is(*n, WP_NS, "docPr")));
    let uri = holder
        .and_then(|h| h.descendants().find(|n| is(*n, A_NS, "graphicData")))
        .and_then(|g| g.attribute("uri"));
    let object = match uri.and_then(|u| u.rsplit('/').next()) {
        None | Some("picture") => ObjectKind::Picture,
        Some("chart" | "chartex") => ObjectKind::Chart,
        Some("diagram") => ObjectKind::SmartArt,
        Some("wordprocessingGroup") => ObjectKind::Group,
        Some("wordprocessingCanvas") => ObjectKind::Canvas,
        Some(_) => ObjectKind::Shape,
    };
    let decorative = doc_pr.is_some_and(|d| {
        d.descendants().any(|n| {
            is(n, ADEC_NS, "decorative") && matches!(n.attribute("val"), Some("1" | "true"))
        })
    });
    let text = text_box_text(node);
    Drawing {
        kind: if floating {
            DrawingKind::Anchored
        } else {
            DrawingKind::Inline
        },
        object,
        floating,
        text_box: node.descendants().any(|n| n.is_w("txbxContent")),
        text,
        description: doc_pr.and_then(|d| non_empty(d.attribute("descr"))),
        title: doc_pr.and_then(|d| non_empty(d.attribute("title"))),
        decorative,
    }
}

/// A `w:pict` (VML) or `w:object` element.
pub(super) fn legacy(node: Node) -> Drawing {
    let shape = node.descendants().find(|n| {
        n.is_element()
            && n.tag_name().namespace() == Some(V_NS)
            && VML_SHAPES.contains(&n.tag_name().name())
    });
    let group = shape.is_some_and(|s| s.tag_name().name() == "group");
    // The pictures inside a group are not the group's own.
    let image_data = node
        .descendants()
        .find(|n| is(*n, V_NS, "imagedata"))
        .filter(|_| !group);
    let ole = node.is_w("object") || node.descendants().any(|n| is(n, O_NS, "OLEObject"));
    let object = if ole {
        ObjectKind::Ole
    } else if group {
        ObjectKind::Group
    } else if image_data.is_some() {
        ObjectKind::Picture
    } else {
        ObjectKind::Shape
    };
    let floating = !ole
        && shape
            .and_then(|s| s.attribute("style"))
            .is_some_and(|style| {
                style
                    .replace(' ', "")
                    .to_ascii_lowercase()
                    .contains("position:absolute")
            });
    let title = [shape, image_data]
        .into_iter()
        .flatten()
        .find_map(|n| non_empty(n.attribute((O_NS, "title"))));
    Drawing {
        kind: DrawingKind::Legacy,
        object,
        floating,
        text_box: node.descendants().any(|n| n.is_w("txbxContent")),
        text: text_box_text(node),
        description: shape.and_then(|s| non_empty(s.attribute("alt"))),
        title,
        decorative: false,
    }
}

/// What a paragraph holds besides text and drawings.
pub(super) struct Extras {
    /// Equations (`m:oMath`) that are not inside another equation.
    pub equations: usize,
    /// A page or column break, a field, a section break or "page break
    /// before". Such a paragraph is not just empty space.
    pub has_break_or_field: bool,
}

/// Scans a paragraph. Nested paragraphs (in text boxes) are not part of it.
pub(super) fn extras(p: Node, ppr: Option<Node>) -> Extras {
    let mut extras = Extras {
        equations: 0,
        has_break_or_field: ppr.is_some_and(|ppr| {
            ppr.w_child("sectPr").is_some()
                || ppr.w_child("pageBreakBefore").is_some_and(|n| toggle(&n))
        }),
    };
    for node in p.descendants().filter(Node::is_element) {
        if node.ancestors().find(|a| a.is_w("p")) != Some(p) {
            continue;
        }
        if is(node, M_NS, "oMath") && !node.ancestors().skip(1).any(|a| is(a, M_NS, "oMath")) {
            extras.equations += 1;
        } else if node.is_w("fldChar")
            || node.is_w("instrText")
            || node.is_w("fldSimple")
            || (node.is_w("br") && matches!(node.w_attr("type"), Some("page" | "column")))
        {
            extras.has_break_or_field = true;
        }
    }
    extras
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{DocxBuilder, Para};
    use crate::model::Document;

    fn drawings(run_xml: &str) -> Vec<Drawing> {
        let bytes = DocxBuilder::new()
            .paragraph(Para::empty().raw_run(run_xml))
            .build();
        Document::from_bytes(&bytes).unwrap().paragraphs[0]
            .drawings
            .clone()
    }

    #[test]
    fn alt_text_is_the_description_then_the_title() {
        let mut d = drawings(
            r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="P" descr=" Front view " title="Printer"/></wp:inline></w:drawing></w:r>"#,
        )
        .remove(0);
        assert_eq!(d.alt_text(), Some("Front view"));
        d.description = None;
        assert_eq!(d.alt_text(), Some("Printer"));
        d.title = Some("  ".into());
        assert_eq!(d.alt_text(), None);
        d.description = Some("x".into());
        d.decorative = true;
        assert_eq!(d.alt_text(), None, "decorative has no alt text");
    }

    #[test]
    fn decorative_mark_needs_the_extension_with_val_1() {
        let marked = |val: &str| {
            drawings(&format!(
                r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="P"><a:extLst><a:ext uri="{{C183D7F6-B498-43B3-948B-1728B52AA6E4}}"><adec:decorative xmlns:adec="http://schemas.microsoft.com/office/drawing/2017/decorative" val="{val}"/></a:ext></a:extLst></wp:docPr></wp:inline></w:drawing></w:r>"#
            ))[0]
                .decorative
        };
        assert!(marked("1"));
        assert!(marked("true"));
        assert!(!marked("0"));
    }

    #[test]
    fn object_kind_comes_from_the_graphic_uri() {
        let kind = |uri: &str| {
            drawings(&format!(
                r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="P"/><a:graphic><a:graphicData uri="{uri}"/></a:graphic></wp:inline></w:drawing></w:r>"#
            ))[0]
                .object
        };
        let dml = "http://schemas.openxmlformats.org/drawingml/2006";
        assert_eq!(kind(&format!("{dml}/picture")), ObjectKind::Picture);
        assert_eq!(kind(&format!("{dml}/chart")), ObjectKind::Chart);
        assert_eq!(
            kind("http://schemas.microsoft.com/office/drawing/2014/chartex"),
            ObjectKind::Chart
        );
        assert_eq!(kind(&format!("{dml}/diagram")), ObjectKind::SmartArt);
        assert_eq!(
            kind("http://schemas.microsoft.com/office/word/2010/wordprocessingShape"),
            ObjectKind::Shape
        );
    }

    #[test]
    fn a_group_or_canvas_is_one_drawing_with_its_own_alt_text() {
        let one = |kind: &str, inner: &str| {
            let run = format!(
                r#"<w:r><w:drawing><wp:anchor><wp:docPr id="1" name="G" descr="Sign-in flow"/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/{kind}">{inner}</a:graphicData></a:graphic></wp:anchor></w:drawing></w:r>"#
            );
            let found = drawings(&run);
            assert_eq!(found.len(), 1, "{kind}");
            found.into_iter().next().unwrap()
        };
        let pic = r#"<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="0" name="Inner" descr="Inner alt"/></pic:nvPicPr></pic:pic>"#;
        let group = one(
            "wordprocessingGroup",
            &format!("<wpg:wgp xmlns:wpg=\"urn:t\">{pic}</wpg:wgp>"),
        );
        assert_eq!(group.object, ObjectKind::Group);
        assert!(group.is_container() && group.floating);
        assert_eq!(group.alt_text(), Some("Sign-in flow"));
        let canvas = one("wordprocessingCanvas", "<wpc:wpc xmlns:wpc=\"urn:t\"/>");
        assert_eq!(canvas.object, ObjectKind::Canvas);
        assert!(canvas.is_container());
    }

    #[test]
    fn text_in_the_shapes_of_a_group_is_kept_and_the_group_still_needs_alt_text() {
        let d = drawings(
            r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="G"/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"><wpg:wgp xmlns:wpg="urn:t"><wps:wsp><wps:txbx><w:txbxContent><w:p><w:r><w:t>Start</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp><wps:wsp><wps:txbx><w:txbxContent><w:p><w:r><w:t>Stop</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></wpg:wgp></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#,
        )
        .remove(0);
        assert_eq!(d.object, ObjectKind::Group);
        assert_eq!(d.text, "Start Stop");
        assert!(d.has_box_text());
        assert!(!d.is_text_box_with_text());
        assert!(d.needs_alt_text());
    }

    #[test]
    fn a_vml_group_is_one_drawing_and_its_pictures_are_not_its_own() {
        let d = drawings(
            r#"<w:r><w:pict xmlns:o="urn:schemas-microsoft-com:office:office"><v:shapetype id="t"/><v:group id="g" alt="Sign-in flow" o:title="Flow" style="position:absolute;width:10pt"><v:shape id="a" alt="Inner" style="position:absolute"><v:imagedata o:title="Inner title"/></v:shape><v:rect id="b" style="position:absolute"/></v:group></w:pict></w:r>"#,
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].object, ObjectKind::Group);
        assert!(d[0].floating);
        assert_eq!(d[0].description.as_deref(), Some("Sign-in flow"));
        assert_eq!(d[0].title.as_deref(), Some("Flow"));
        // With no attributes of its own, the group has no alt text.
        let bare = drawings(
            r#"<w:r><w:pict><v:group id="g"><v:shape id="a" alt="Inner"><v:imagedata/></v:shape></v:group></w:pict></w:r>"#,
        );
        assert_eq!(bare[0].object, ObjectKind::Group);
        assert_eq!(bare[0].alt_text(), None);
        assert!(!bare[0].floating);
    }

    #[test]
    fn a_shape_group_or_canvas_in_alternate_content_is_one_drawing() {
        use crate::fixtures::Image;
        for image in [Image::shape(), Image::group(), Image::canvas()] {
            let image = image.floating().alt("A box");
            let xml = image.xml();
            assert!(xml.contains("<mc:Fallback>"), "{xml}");
            let found = drawings(&xml);
            assert_eq!(found.len(), 1, "the fallback copy is not a second drawing");
            assert!(found[0].floating);
            assert_eq!(found[0].alt_text(), Some("A box"));
        }
    }

    #[test]
    fn inline_and_anchored_drawings() {
        let inline = drawings(
            r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="P"/></wp:inline></w:drawing></w:r>"#,
        );
        let anchored = drawings(
            r#"<w:r><w:drawing><wp:anchor><wp:docPr id="1" name="P"/></wp:anchor></w:drawing></w:r>"#,
        );
        assert!(!inline[0].floating);
        assert_eq!(inline[0].kind, DrawingKind::Inline);
        assert!(anchored[0].floating);
        assert_eq!(anchored[0].kind, DrawingKind::Anchored);
    }

    #[test]
    fn vml_picture_alt_title_and_position() {
        let d = drawings(
            r#"<w:r><w:pict><v:shapetype id="t"/><v:shape id="s" alt="A cat" style="position:absolute;width:10pt" xmlns:o="urn:schemas-microsoft-com:office:office"><v:imagedata o:title="Cat"/></v:shape></w:pict></w:r>"#,
        )
        .remove(0);
        assert_eq!(d.kind, DrawingKind::Legacy);
        assert_eq!(d.object, ObjectKind::Picture);
        assert_eq!(d.description.as_deref(), Some("A cat"));
        assert_eq!(d.title.as_deref(), Some("Cat"));
        assert!(d.floating);
    }

    #[test]
    fn vml_without_imagedata_is_a_shape_and_ole_is_an_object() {
        let shape = drawings(r#"<w:r><w:pict><v:rect style="width:1pt"/></w:pict></w:r>"#);
        assert_eq!(shape[0].object, ObjectKind::Shape);
        assert!(!shape[0].floating);
        let ole = drawings(
            r#"<w:r><w:object><v:shape><v:imagedata/></v:shape><o:OLEObject xmlns:o="urn:schemas-microsoft-com:office:office" Type="Embed"/></w:object></w:r>"#,
        );
        assert_eq!(ole[0].object, ObjectKind::Ole);
        assert!(!ole[0].needs_alt_text());
    }

    #[test]
    fn text_box_text_is_kept_and_joined() {
        let d = drawings(
            r#"<w:r><w:drawing><wp:anchor><wp:docPr id="1" name="Box"/><w:txbxContent><w:p><w:r><w:t>One</w:t></w:r></w:p><w:p><w:r><w:t xml:space="preserve"> Two </w:t></w:r></w:p><w:p/></w:txbxContent></wp:anchor></w:drawing></w:r>"#,
        )
        .remove(0);
        assert!(d.text_box);
        assert_eq!(d.text, "One Two");
        assert!(d.has_box_text());
        assert!(!d.needs_alt_text());
    }

    #[test]
    fn equations_breaks_and_fields_are_noted_on_the_paragraph() {
        let doc = |content: &str| {
            let bytes = DocxBuilder::new()
                .paragraph(Para::empty().raw_run(content))
                .build();
            Document::from_bytes(&bytes).unwrap().paragraphs.remove(0)
        };
        let math = doc(
            r#"<m:oMathPara xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:oMath><m:oMath/></m:oMath><m:oMath/></m:oMathPara>"#,
        );
        assert_eq!(math.equations, 2, "an equation inside an equation is one");
        assert!(!math.has_break_or_field);
        assert!(doc(r#"<w:r><w:br w:type="page"/></w:r>"#).has_break_or_field);
        assert!(doc(r#"<w:r><w:br w:type="column"/></w:r>"#).has_break_or_field);
        assert!(!doc(r#"<w:r><w:br/></w:r>"#).has_break_or_field);
        assert!(doc(r#"<w:fldSimple w:instr="PAGE"/>"#).has_break_or_field);
        assert!(doc(r#"<w:r><w:fldChar w:fldCharType="begin"/></w:r>"#).has_break_or_field);
    }
}
