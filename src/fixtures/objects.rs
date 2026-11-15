//! Fixture builders for pictures, shapes, text boxes, equations and tables,
//! and the passing and failing fixtures for the IMG, TXB and TBL rules.
//!
//! The pictures have no image data and the charts, SmartArt and embedded
//! objects have no parts behind them, so these documents are for the checker
//! only. The fixture files on disk use only pictures, shapes, a text box, an
//! equation and tables.

use super::{DocxBuilder, Para, Run, clean, esc};
use crate::findings::RuleId;

const PICTURE_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";
const CHART_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const DIAGRAM_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";
const SHAPE_URI: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingShape";
const GROUP_URI: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingGroup";
const CANVAS_URI: &str = "http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas";
const OFFICE_NS: &str = "urn:schemas-microsoft-com:office:office";
const PICTURE_XML: &str = r#"<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="0" name="Picture 1"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic>"#;
const EXTENT: &str = r#"<wp:extent cx="914400" cy="914400"/>"#;

#[derive(Debug, Clone, Copy)]
enum ImageKind {
    Picture,
    Chart,
    SmartArt,
    Shape,
    Group,
    Canvas,
    VmlPicture,
    VmlShape,
    VmlGroup,
}

/// A drawing in a run, for [`Para::raw_run`]. Use [`Image::xml`].
#[derive(Debug, Clone)]
pub struct Image {
    kind: ImageKind,
    alt: Option<String>,
    title: Option<String>,
    decorative: bool,
    floating: bool,
    /// Text in a shape (or in a shape inside a group or canvas).
    text: Option<String>,
}

impl Image {
    fn new(kind: ImageKind) -> Image {
        Image {
            kind,
            alt: None,
            title: None,
            decorative: false,
            floating: false,
            text: None,
        }
    }

    /// A DrawingML picture.
    pub fn picture() -> Image {
        Image::new(ImageKind::Picture)
    }

    pub fn chart() -> Image {
        Image::new(ImageKind::Chart)
    }

    pub fn smart_art() -> Image {
        Image::new(ImageKind::SmartArt)
    }

    /// A DrawingML shape with no text.
    pub fn shape() -> Image {
        Image::new(ImageKind::Shape)
    }

    /// A group (`wpg:wgp`) of a picture and a shape.
    pub fn group() -> Image {
        Image::new(ImageKind::Group)
    }

    /// A drawing canvas (`wpc:wpc`) with two shapes, like a small flowchart.
    pub fn canvas() -> Image {
        Image::new(ImageKind::Canvas)
    }

    /// An older VML group (`v:group`) of a picture and a shape.
    pub fn vml_group() -> Image {
        Image::new(ImageKind::VmlGroup)
    }

    /// An older VML picture (`v:shape` with `v:imagedata`).
    pub fn vml_picture() -> Image {
        Image::new(ImageKind::VmlPicture)
    }

    /// A VML shape with no picture (`v:rect`).
    pub fn vml_shape() -> Image {
        Image::new(ImageKind::VmlShape)
    }

    /// The alt text: `descr` in DrawingML, `alt` in VML.
    pub fn alt(mut self, text: &str) -> Image {
        self.alt = Some(text.to_string());
        self
    }

    /// The title: `title` in DrawingML, `o:title` in VML.
    pub fn title(mut self, text: &str) -> Image {
        self.title = Some(text.to_string());
        self
    }

    /// Text in the shape, or in one shape of a group or canvas. A shape with
    /// text is a text box. Pictures, charts and SmartArt ignore it.
    pub fn text(mut self, text: &str) -> Image {
        self.text = Some(text.to_string());
        self
    }

    /// Marks the drawing decorative (DrawingML only).
    pub fn decorative(mut self) -> Image {
        self.decorative = true;
        self
    }

    /// Not "In line with text": `wp:anchor` in DrawingML, `position:absolute`
    /// in VML.
    pub fn floating(mut self) -> Image {
        self.floating = true;
        self
    }

    /// The run XML.
    pub fn xml(&self) -> String {
        match self.kind {
            ImageKind::VmlPicture | ImageKind::VmlShape | ImageKind::VmlGroup => self.vml(),
            _ => self.drawing_ml(),
        }
    }

    fn drawing_ml(&self) -> String {
        let (uri, graphic): (&str, String) = match self.kind {
            ImageKind::Picture => (PICTURE_URI, PICTURE_XML.to_string()),
            ImageKind::Chart => (
                CHART_URI,
                r#"<c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"/>"#.to_string(),
            ),
            ImageKind::SmartArt => (
                DIAGRAM_URI,
                r#"<dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#.to_string(),
            ),
            ImageKind::Group => (
                GROUP_URI,
                format!(
                    r#"<wpg:wgp xmlns:wpg="{GROUP_URI}"><wpg:cNvGrpSpPr/><wpg:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/><a:chOff x="0" y="0"/><a:chExt cx="914400" cy="914400"/></a:xfrm></wpg:grpSpPr>{}{}</wpg:wgp>"#,
                    PICTURE_XML,
                    wps_shape(self.text.as_deref())
                ),
            ),
            ImageKind::Canvas => (
                CANVAS_URI,
                format!(
                    r#"<wpc:wpc xmlns:wpc="{CANVAS_URI}"><wpc:bg/><wpc:whole/>{}{}</wpc:wpc>"#,
                    wps_shape(self.text.as_deref()),
                    wps_shape(None)
                ),
            ),
            _ => (SHAPE_URI, wps_shape(self.text.as_deref())),
        };
        let mut doc_pr = r#"<wp:docPr id="1" name="Object 1""#.to_string();
        if let Some(alt) = &self.alt {
            doc_pr += &format!(r#" descr="{}""#, esc(alt));
        }
        if let Some(title) = &self.title {
            doc_pr += &format!(r#" title="{}""#, esc(title));
        }
        doc_pr += ">";
        if self.decorative {
            doc_pr += r#"<a:extLst><a:ext uri="{C183D7F6-B498-43B3-948B-1728B52AA6E4}"><adec:decorative xmlns:adec="http://schemas.microsoft.com/office/drawing/2017/decorative" val="1"/></a:ext></a:extLst>"#;
        }
        doc_pr += "</wp:docPr>";
        format!(
            "<w:r>{}</w:r>",
            drawing(self.floating, &doc_pr, uri, &graphic)
        )
    }

    fn vml(&self) -> String {
        let position = if self.floating {
            ";position:absolute"
        } else {
            ""
        };
        let mut attrs = format!(r#"id="_x0000_i1025" style="width:72pt;height:72pt{position}""#);
        if let Some(alt) = &self.alt {
            attrs += &format!(r#" alt="{}""#, esc(alt));
        }
        let title = self
            .title
            .as_ref()
            .map(|t| format!(r#" o:title="{}""#, esc(t)))
            .unwrap_or_default();
        let text_box = self
            .text
            .as_ref()
            .map(|t| {
                format!(
                    r#"<v:textbox><w:txbxContent><w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:txbxContent></v:textbox>"#,
                    esc(t)
                )
            })
            .unwrap_or_default();
        let shape = match self.kind {
            ImageKind::VmlPicture => {
                format!(r#"<v:shape {attrs}><v:imagedata{title}/></v:shape>"#)
            }
            ImageKind::VmlGroup => format!(
                r#"<v:group {attrs}{title}><v:shape id="_x0000_s1030" style="position:absolute;width:36pt;height:36pt"><v:imagedata/></v:shape><v:rect id="_x0000_s1031" style="position:absolute;width:18pt;height:18pt">{text_box}</v:rect></v:group>"#
            ),
            _ if self.text.is_some() => format!(r#"<v:rect {attrs}{title}>{text_box}</v:rect>"#),
            _ => format!(r#"<v:rect {attrs}{title}/>"#),
        };
        format!(r#"<w:r><w:pict xmlns:o="{OFFICE_NS}">{shape}</w:pict></w:r>"#)
    }
}

/// A DrawingML shape (`wps:wsp`), with a text box when it has text.
fn wps_shape(text: Option<&str>) -> String {
    let text_box = text
        .map(|t| {
            format!(
                r#"<wps:txbx><w:txbxContent><w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:txbxContent></wps:txbx>"#,
                esc(t)
            )
        })
        .unwrap_or_default();
    format!(
        r#"<wps:wsp><wps:cNvSpPr/><wps:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></wps:spPr>{text_box}<wps:bodyPr/></wps:wsp>"#
    )
}

/// `w:drawing` with a DrawingML object. `floating` writes `wp:anchor`, else
/// `wp:inline`.
fn drawing(floating: bool, doc_pr: &str, uri: &str, graphic: &str) -> String {
    let graphic =
        format!(r#"<a:graphic><a:graphicData uri="{uri}">{graphic}</a:graphicData></a:graphic>"#);
    let holder = if floating {
        format!(
            r#"<wp:anchor distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="column"><wp:posOffset>0</wp:posOffset></wp:positionH><wp:positionV relativeFrom="paragraph"><wp:posOffset>0</wp:posOffset></wp:positionV>{EXTENT}<wp:wrapSquare wrapText="bothSides"/>{doc_pr}{graphic}</wp:anchor>"#
        )
    } else {
        format!(
            r#"<wp:inline distT="0" distB="0" distL="0" distR="0">{EXTENT}{doc_pr}{graphic}</wp:inline>"#
        )
    };
    format!("<w:drawing>{holder}</w:drawing>")
}

/// A DrawingML text box (a `wps` shape) with one paragraph of text, with the
/// VML copy that Word writes in `mc:Fallback`.
pub fn text_box(text: &str) -> String {
    let text = esc(text);
    let content = format!(
        r#"<w:txbxContent><w:p><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p></w:txbxContent>"#
    );
    let shape = format!(
        r#"<wps:wsp><wps:cNvSpPr txBox="1"/><wps:spPr><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></wps:spPr><wps:txbx>{content}</wps:txbx><wps:bodyPr/></wps:wsp>"#
    );
    let drawing = drawing(
        true,
        r#"<wp:docPr id="2" name="Text Box 2"/>"#,
        SHAPE_URI,
        &shape,
    );
    format!(
        r#"<w:r><mc:AlternateContent><mc:Choice Requires="wps">{drawing}</mc:Choice><mc:Fallback><w:pict><v:rect id="_x0000_s1026" style="position:absolute;width:72pt;height:72pt"><v:textbox>{content}</v:textbox></v:rect></w:pict></mc:Fallback></mc:AlternateContent></w:r>"#
    )
}

/// A VML text box only (no DrawingML copy).
pub fn vml_text_box(text: &str) -> String {
    format!(
        r#"<w:r><w:pict><v:rect id="_x0000_s1027" style="position:absolute;width:72pt;height:72pt"><v:textbox><w:txbxContent><w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:txbxContent></v:textbox></v:rect></w:pict></w:r>"#,
        esc(text)
    )
}

/// An embedded object (OLE) in a run.
pub fn ole_object() -> String {
    format!(
        r#"<w:r><w:object w:dxaOrig="1440" w:dyaOrig="1440"><v:shape id="_x0000_i1026" style="width:72pt;height:72pt"><v:imagedata/></v:shape><o:OLEObject xmlns:o="{OFFICE_NS}" Type="Embed" ProgID="Package" ShapeID="_x0000_i1026" DrawAspect="Content" ObjectID="_1"/></w:object></w:r>"#
    )
}

/// A display equation (`m:oMathPara`). It goes in a paragraph, not in a run.
pub fn equation(text: &str) -> String {
    format!(
        r#"<m:oMathPara xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:oMath><m:r><m:t>{}</m:t></m:r></m:oMath></m:oMathPara>"#,
        esc(text)
    )
}

#[derive(Debug, Clone)]
struct TableCell {
    text: String,
    span: u32,
    /// `Some(true)` starts a vertical merge, `Some(false)` continues it.
    v_merge: Option<bool>,
    /// Paragraph XML that replaces the text.
    raw: Option<String>,
}

#[derive(Debug, Clone)]
struct TableRow {
    header: bool,
    cant_split: bool,
    /// Replaces the row properties.
    raw_properties: Option<String>,
    cells: Vec<TableCell>,
}

/// A table. [`Table::new`] gives a good table: the first row is the header
/// row and no row can break across pages. Other methods break that.
#[derive(Debug, Clone)]
pub struct Table {
    rows: Vec<TableRow>,
}

impl Table {
    /// A table with these rows of cell text. An empty string is an empty cell.
    pub fn new(rows: &[&[&str]]) -> Table {
        Table {
            rows: rows
                .iter()
                .enumerate()
                .map(|(i, cells)| TableRow {
                    header: i == 0,
                    cant_split: true,
                    raw_properties: None,
                    cells: cells
                        .iter()
                        .map(|text| TableCell {
                            text: text.to_string(),
                            span: 1,
                            v_merge: None,
                            raw: None,
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// The first row is not marked as a header row.
    pub fn no_header_row(mut self) -> Table {
        self.rows[0].header = false;
        self
    }

    /// No row has "do not allow row to break across pages".
    pub fn rows_can_split(mut self) -> Table {
        self.rows.iter_mut().for_each(|r| r.cant_split = false);
        self
    }

    /// Only this row can break across pages.
    pub fn row_can_split(mut self, row: usize) -> Table {
        self.rows[row].cant_split = false;
        self
    }

    /// Replaces the XML inside `w:trPr` of a row.
    pub fn raw_row_properties(mut self, row: usize, xml: &str) -> Table {
        self.rows[row].raw_properties = Some(xml.to_string());
        self
    }

    /// The cell spans `columns` columns (`w:gridSpan`).
    pub fn span(mut self, row: usize, cell: usize, columns: u32) -> Table {
        self.rows[row].cells[cell].span = columns;
        self
    }

    /// The cell takes part in a vertical merge (`w:vMerge`): `restart` for
    /// the top cell, false for the cells below it.
    pub fn merge_down(mut self, row: usize, cell: usize, restart: bool) -> Table {
        self.rows[row].cells[cell].v_merge = Some(restart);
        self
    }

    /// Replaces the content of a cell with paragraph XML.
    pub fn raw_cell(mut self, row: usize, cell: usize, paragraphs: &str) -> Table {
        self.rows[row].cells[cell].raw = Some(paragraphs.to_string());
        self
    }

    /// Puts a table inside a cell.
    pub fn nested(self, row: usize, cell: usize, inner: &Table) -> Table {
        let content = format!("{}<w:p/>", inner.xml());
        self.raw_cell(row, cell, &content)
    }

    /// The `w:tbl` XML.
    pub fn xml(&self) -> String {
        let columns = self
            .rows
            .iter()
            .map(|r| r.cells.iter().map(|c| c.span).sum::<u32>())
            .max()
            .unwrap_or(1)
            .max(1);
        let width = 9360 / columns;
        let grid = format!(r#"<w:gridCol w:w="{width}"/>"#).repeat(columns as usize);
        let rows: String = self.rows.iter().map(|r| r.xml(width)).collect();
        format!(
            r#"<w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr><w:tblGrid>{grid}</w:tblGrid>{rows}</w:tbl>"#
        )
    }
}

impl TableRow {
    fn xml(&self, column_width: u32) -> String {
        let properties = match &self.raw_properties {
            Some(xml) => xml.clone(),
            None => {
                let mut xml = String::new();
                if self.cant_split {
                    xml += "<w:cantSplit/>";
                }
                if self.header {
                    xml += "<w:tblHeader/>";
                }
                xml
            }
        };
        let properties = if properties.is_empty() {
            String::new()
        } else {
            format!("<w:trPr>{properties}</w:trPr>")
        };
        let cells: String = self.cells.iter().map(|c| c.xml(column_width)).collect();
        format!("<w:tr>{properties}{cells}</w:tr>")
    }
}

impl TableCell {
    fn xml(&self, column_width: u32) -> String {
        let mut properties = format!(
            r#"<w:tcW w:w="{}" w:type="dxa"/>"#,
            column_width * self.span
        );
        if self.span > 1 {
            properties += &format!(r#"<w:gridSpan w:val="{}"/>"#, self.span);
        }
        match self.v_merge {
            Some(true) => properties += r#"<w:vMerge w:val="restart"/>"#,
            Some(false) => properties += "<w:vMerge/>",
            None => {}
        }
        let content = match (&self.raw, self.text.is_empty()) {
            (Some(raw), _) => raw.clone(),
            (None, true) => "<w:p/>".to_string(),
            (None, false) => Para::new(&self.text).xml(&mut 0, &mut Vec::new()),
        };
        format!("<w:tc><w:tcPr>{properties}</w:tcPr>{content}</w:tc>")
    }
}

impl DocxBuilder {
    /// Adds a table to the body.
    pub fn table(self, table: Table) -> DocxBuilder {
        self.paragraph(Para {
            table_xml: Some(table.xml()),
            ..Para::default()
        })
    }
}

/// A table that no table rule reports.
fn good_table() -> Table {
    Table::new(&[&["Setting", "Value"], &["Paper", "A4"], &["Colour", "Mono"]])
}

/// An alt text of exactly `chars` characters that no rule objects to except
/// IMG-003 (when it is too long).
pub fn alt_of_length(chars: usize) -> String {
    "x".repeat(chars)
}

const GOOD_ALT: &str = "Front panel of the printer with the power button circled in red";

fn with_image(image: &Image) -> DocxBuilder {
    clean().paragraph(Para::empty().raw_run(&image.xml()))
}

/// A screenshot with three floating shapes (arrows or boxes) drawn on it, in
/// the same paragraph.
fn annotated(picture: &Image, shape: &Image) -> DocxBuilder {
    clean().paragraph(
        Para::empty()
            .raw_run(&picture.xml())
            .raw_run(&shape.xml())
            .raw_run(&shape.xml())
            .raw_run(&shape.xml()),
    )
}

/// The passing and failing fixture of the IMG, TXB and TBL rules, as
/// `(rule, passing, failing)`.
pub(super) fn pairs() -> Vec<(RuleId, DocxBuilder, DocxBuilder)> {
    let good_picture = Image::picture().alt(GOOD_ALT);
    let caption = |text: &str| Para::new(text).style("Caption");
    let with_caption = |alt: &str, caption_text: &str| {
        clean()
            .custom_style("Caption", "Caption", None)
            .paragraph(Para::empty().raw_run(&Image::picture().alt(alt).xml()))
            .paragraph(caption(caption_text))
    };
    vec![
        (
            RuleId::Img001,
            with_image(&good_picture),
            with_image(&Image::picture()),
        ),
        (
            RuleId::Img002,
            with_image(&good_picture),
            with_image(&Image::picture().alt("IMG_1234.jpg")),
        ),
        (
            RuleId::Img003,
            with_image(&Image::picture().alt(&alt_of_length(250))),
            with_image(&Image::picture().alt(&alt_of_length(251))),
        ),
        (
            RuleId::Img004,
            with_image(&good_picture),
            with_image(&good_picture.clone().floating()),
        ),
        (
            RuleId::Img005,
            with_caption(GOOD_ALT, "Figure 1: the printer"),
            with_caption("Figure 1: the printer", "Figure 1: the printer"),
        ),
        (
            RuleId::Img006,
            clean().para("Print the guide on A4 paper."),
            clean().paragraph(Para::new("The area is ").raw_run(&equation("A=wh"))),
        ),
        (
            RuleId::Img007,
            annotated(&good_picture, &Image::shape().floating().decorative()),
            annotated(&good_picture, &Image::shape().floating()),
        ),
        (
            RuleId::Txb001,
            clean().para("Note: save your work before you print."),
            clean().paragraph(
                Para::new("Before you print.")
                    .raw_run(&text_box("Note: save your work before you print.")),
            ),
        ),
        (
            RuleId::Txb002,
            clean().para("One column of text."),
            clean().paragraph(Para::new("Two columns of text.").ends_section(2)),
        ),
        (
            RuleId::Txb003,
            clean().para("Name: Ana Silva"),
            clean().para("Name:     Ana Silva"),
        ),
        (
            RuleId::Txb004,
            clean()
                .para("First paragraph.")
                .paragraph(Para::empty())
                .para("Second paragraph."),
            clean()
                .para("First paragraph.")
                .paragraph(Para::empty())
                .paragraph(Para::empty())
                .para("Second paragraph."),
        ),
        (
            RuleId::Tbl001,
            clean().table(good_table()),
            clean().table(good_table().no_header_row()),
        ),
        (
            RuleId::Tbl002,
            clean().table(good_table()),
            clean().table(good_table().nested(1, 1, &good_table())),
        ),
        (
            RuleId::Tbl003,
            clean().table(good_table()),
            clean().table(
                Table::new(&[&["Settings"], &["Paper", "A4"], &["Colour", "Mono"]]).span(0, 0, 2),
            ),
        ),
        (
            RuleId::Tbl004,
            clean().table(good_table()),
            clean().table(Table::new(&[&["Setting", "Value"]])),
        ),
        (
            RuleId::Tbl005,
            clean().table(good_table()),
            clean().table(Table::new(&[
                &["Setting", "Value"],
                &["Paper", "A4"],
                &["", ""],
                &["Colour", "Mono"],
            ])),
        ),
        (
            RuleId::Tbl006,
            clean().table(good_table()),
            clean().table(good_table().rows_can_split()),
        ),
    ]
}

/// A paragraph with one run for each part, to split text across runs.
pub fn split_runs(parts: &[&str]) -> Para {
    parts
        .iter()
        .fold(Para::empty(), |para, part| para.then(Run::new(part)))
}
