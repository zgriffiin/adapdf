//! Table styles: the text properties and the fill that a table style gives to
//! a cell.
//!
//! # Precedence
//!
//! The layers are the ones in ECMA-376 Part 1, 17.7.2 ("Style Hierarchy"):
//! document defaults, then table styles, then paragraph styles, then character
//! styles, then direct formatting. Each later layer wins, property by
//! property. [`super::styles::Styles`] builds that order; this module gives
//! the table style layer. Word agrees: a text colour in the paragraph style
//! (Normal or another) wins over the table style's.
//!
//! Inside one table style, the whole-table format comes first. Then the
//! conditional formats (`w:tblStylePr`, ECMA-376 17.7.6) that apply to the
//! cell, each over the ones before it, from lowest to highest:
//!
//! 1. row bands `band1Horz`, `band2Horz`
//! 2. column bands `band1Vert`, `band2Vert`
//! 3. `lastCol`, then `firstCol`
//! 4. `lastRow`, then `firstRow`
//! 5. the corner cells `seCell`, `swCell`, `neCell`, `nwCell`
//!
//! This order is what Word does. It was not read from the specification:
//! it was found by asking Word (build 16.0.20527) for the effective colours of
//! test tables, with a different colour in every conditional format (the test
//! `table_style_colours_match_word` repeats this). It has been checked for
//! these: the row formats beat the column formats, the column bands beat the
//! row bands, the first row and the first column beat the last row and the
//! last column, and a corner cell beats both of its edges. The order among the
//! corners other than `nwCell` is a guess; they only meet in a table of one
//! cell.
//!
//! What applies, from `w:tblLook` (ECMA-376 17.4.56) and the table:
//!
//! - `firstRow`, `lastRow`, `firstColumn` and `lastColumn` switch the four
//!   edge formats on. A corner format needs both of its edges switched on.
//! - `noHBand` and `noVBand` switch the two kinds of banding off. Banding
//!   also needs a band size (`w:tblStyleRowBandSize`, `w:tblStyleColBandSize`)
//!   in the table or in the table style (or in a style it is based on). With
//!   none, Word draws no bands.
//! - Both forms of `w:tblLook` are read: the attributes, and the older
//!   hexadecimal `w:val` (0x0020 first row, 0x0040 last row, 0x0080 first
//!   column, 0x0100 last column, 0x0200 no row bands, 0x0400 no column
//!   bands). With any attribute, Word reads only the attributes, and an
//!   attribute that is missing counts as on. With no `w:tblLook`, Word uses
//!   `04A0`: the first row, the first column and no column bands.
//! - The first row is the row at the top and every row after it that is
//!   marked "repeat as header row" (`w:tblHeader`) with no gap. Banding starts
//!   after these rows, and stops before the last row when that is switched on.
//!   The same holds for columns, with the first and the last column.
//!
//! A style that is based on another (`w:basedOn`) inherits from it first, per
//! part: its whole-table format over the base's whole-table format, its
//! `firstRow` over the base's `firstRow`, and so on. The merged parts are then
//! applied in the order above.
//!
//! A fill that the cell itself sets (`w:tcPr/w:shd`) wins over the table
//! style. See `container_fill` in `body.rs`.
//!
//! Not applied: the table style's borders, margins and widths, which do not
//! matter for colour.

use std::collections::HashMap;

use roxmltree::Node;

use super::props::{Color, ParFmt, RunFmt, parse_shading};
use super::styles::Styles;
use super::theme::Theme;
use crate::docx::WordNode;

/// A conditional format of a table style, in the order Word applies them
/// (the later one wins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Condition {
    Band1Horz,
    Band2Horz,
    Band1Vert,
    Band2Vert,
    LastCol,
    FirstCol,
    LastRow,
    FirstRow,
    SeCell,
    SwCell,
    NeCell,
    NwCell,
}

impl Condition {
    /// All of them, from the one applied first to the one applied last.
    const IN_ORDER: [Condition; 12] = [
        Condition::Band1Horz,
        Condition::Band2Horz,
        Condition::Band1Vert,
        Condition::Band2Vert,
        Condition::LastCol,
        Condition::FirstCol,
        Condition::LastRow,
        Condition::FirstRow,
        Condition::SeCell,
        Condition::SwCell,
        Condition::NeCell,
        Condition::NwCell,
    ];

    fn from_type(name: &str) -> Option<Condition> {
        Some(match name {
            "band1Horz" => Condition::Band1Horz,
            "band2Horz" => Condition::Band2Horz,
            "band1Vert" => Condition::Band1Vert,
            "band2Vert" => Condition::Band2Vert,
            "lastCol" => Condition::LastCol,
            "firstCol" => Condition::FirstCol,
            "lastRow" => Condition::LastRow,
            "firstRow" => Condition::FirstRow,
            "seCell" => Condition::SeCell,
            "swCell" => Condition::SwCell,
            "neCell" => Condition::NeCell,
            "nwCell" => Condition::NwCell,
            _ => return None,
        })
    }
}

/// What one part of a table style gives to the text and the fill of a cell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellFmt {
    pub par: ParFmt,
    pub run: RunFmt,
    /// The cell fill (`w:tcPr/w:shd`). [`Color::Auto`] means no fill.
    pub fill: Option<Color>,
}

impl CellFmt {
    /// Reads a `w:style` or a `w:tblStylePr` element.
    fn parse(node: Node, theme: &Theme) -> CellFmt {
        CellFmt {
            par: ParFmt::parse(node.w_child("pPr"), theme),
            run: RunFmt::parse(node.w_child("rPr"), theme),
            fill: node
                .w_child("tcPr")
                .and_then(|pr| pr.w_child("shd"))
                .and_then(|shd| parse_shading(&shd, theme)),
        }
    }

    fn overlay(&mut self, over: &CellFmt) {
        self.par.overlay(&over.par);
        self.run.overlay(&over.run);
        if over.fill.is_some() {
            self.fill = over.fill;
        }
    }
}

/// One table style as written in `word/styles.xml`.
#[derive(Debug, Clone, Default)]
pub struct TableStyle {
    whole: CellFmt,
    conditional: Vec<(Condition, CellFmt)>,
    row_band_size: Option<u32>,
    col_band_size: Option<u32>,
}

impl TableStyle {
    pub(super) fn parse(style: Node, theme: &Theme) -> TableStyle {
        let band_size = |name: &str| {
            style
                .w_child("tblPr")
                .and_then(|pr| pr.w_child(name))
                .and_then(|n| n.w_attr("val"))
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|size| *size >= 1)
        };
        TableStyle {
            whole: CellFmt::parse(style, theme),
            conditional: style
                .children()
                .filter(|n| n.is_w("tblStylePr"))
                .filter_map(|n| {
                    let condition = Condition::from_type(n.w_attr("type")?)?;
                    Some((condition, CellFmt::parse(n, theme)))
                })
                .collect(),
            row_band_size: band_size("tblStyleRowBandSize"),
            col_band_size: band_size("tblStyleColBandSize"),
        }
    }
}

/// The table styles of a document.
pub(super) type TableStyles = HashMap<String, TableStyle>;

/// What `w:tblLook` switches on and off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look {
    pub first_row: bool,
    pub last_row: bool,
    pub first_column: bool,
    pub last_column: bool,
    pub no_h_band: bool,
    pub no_v_band: bool,
}

impl Default for Look {
    /// What Word uses when a table has no `w:tblLook`: `04A0`.
    fn default() -> Look {
        Look {
            first_row: true,
            last_row: false,
            first_column: true,
            last_column: false,
            no_h_band: false,
            no_v_band: true,
        }
    }
}

impl Look {
    const FIRST_ROW: u32 = 0x0020;
    const LAST_ROW: u32 = 0x0040;
    const FIRST_COLUMN: u32 = 0x0080;
    const LAST_COLUMN: u32 = 0x0100;
    const NO_H_BAND: u32 = 0x0200;
    const NO_V_BAND: u32 = 0x0400;

    /// Reads a `w:tblLook`. With any of its six attributes, only the
    /// attributes are read, and each one that is missing counts as on (this is
    /// how Word reads it, also when `w:val` is there; Word itself writes all
    /// six). With none, `w:val` is read, and with no `w:val` the default.
    fn parse(tbl_look: Option<Node>) -> Look {
        const ATTRIBUTES: [&str; 6] = [
            "firstRow",
            "lastRow",
            "firstColumn",
            "lastColumn",
            "noHBand",
            "noVBand",
        ];
        let Some(node) = tbl_look else {
            return Look::default();
        };
        if ATTRIBUTES.iter().any(|name| node.w_attr(name).is_some()) {
            let flag = |name: &str| {
                node.w_attr(name)
                    .is_none_or(|value| matches!(value, "1" | "true" | "on"))
            };
            return Look {
                first_row: flag("firstRow"),
                last_row: flag("lastRow"),
                first_column: flag("firstColumn"),
                last_column: flag("lastColumn"),
                no_h_band: flag("noHBand"),
                no_v_band: flag("noVBand"),
            };
        }
        match node
            .w_attr("val")
            .and_then(|v| u32::from_str_radix(v, 16).ok())
        {
            Some(bits) => Look {
                first_row: bits & Look::FIRST_ROW != 0,
                last_row: bits & Look::LAST_ROW != 0,
                first_column: bits & Look::FIRST_COLUMN != 0,
                last_column: bits & Look::LAST_COLUMN != 0,
                no_h_band: bits & Look::NO_H_BAND != 0,
                no_v_band: bits & Look::NO_V_BAND != 0,
            },
            None => Look::default(),
        }
    }
}

/// The table properties that choose and shape the table style: the style
/// (`w:tblStyle`), `w:tblLook`, and the band sizes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableProps {
    pub style: Option<String>,
    pub look: Look,
    pub row_band_size: Option<u32>,
    pub col_band_size: Option<u32>,
}

impl TableProps {
    /// Reads the `w:tblPr` of a `w:tbl` element.
    pub fn parse(tbl: Node) -> TableProps {
        let Some(pr) = tbl.w_child("tblPr") else {
            return TableProps::default();
        };
        let band_size = |name: &str| {
            pr.w_child(name)
                .and_then(|n| n.w_attr("val"))
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|size| *size >= 1)
        };
        TableProps {
            style: pr
                .w_child("tblStyle")
                .and_then(|n| n.w_attr("val"))
                .map(str::to_string),
            look: Look::parse(pr.w_child("tblLook")),
            row_band_size: band_size("tblStyleRowBandSize"),
            col_band_size: band_size("tblStyleColBandSize"),
        }
    }
}

/// Where a cell is in its table, counted from 0. Columns are counted in cells
/// of the row, not in grid columns: a cell that spans two columns is one
/// column, as in Word (checked with a first column that spans two columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPos {
    pub row: usize,
    pub rows: usize,
    /// How many rows at the top get the first row format: the row at the top
    /// and the rows after it that are marked as header rows, with no gap.
    pub header_rows: usize,
    pub col: usize,
    /// The number of cells in this row.
    pub cols: usize,
}

impl CellPos {
    /// The table around a `w:tc` element, and the cell's place in it. `None`
    /// when the cell is not in a row of a table.
    pub fn of<'a, 'input>(tc: Node<'a, 'input>) -> Option<(Node<'a, 'input>, CellPos)> {
        let tr = tc.parent().filter(|n| n.is_w("tr"))?;
        let tbl = tr.parent().filter(|n| n.is_w("tbl"))?;
        let rows: Vec<Node> = tbl.children().filter(|n| n.is_w("tr")).collect();
        let row = rows.iter().position(|r| *r == tr)?;
        let cells: Vec<Node> = tr.children().filter(|n| n.is_w("tc")).collect();
        let col = cells.iter().position(|c| *c == tc)?;
        let is_header = |row: &Node| {
            row.w_child("trPr")
                .and_then(|pr| pr.w_child("tblHeader"))
                .is_some_and(|n| !matches!(n.w_attr("val"), Some("0" | "false" | "off")))
        };
        let header_rows = 1 + rows[1..].iter().take_while(|r| is_header(r)).count();
        Some((
            tbl,
            CellPos {
                row,
                rows: rows.len(),
                header_rows,
                col,
                cols: cells.len(),
            },
        ))
    }
}

/// The band (0 for the first band, 1 for the next, then 0 again) of the item
/// at `index` in `first..end`. `None` when it is outside the range or when
/// there is no band size: Word draws no bands then.
fn band(index: usize, first: usize, end: usize, size: Option<u32>) -> Option<usize> {
    let size = size? as usize;
    (first..end)
        .contains(&index)
        .then(|| ((index - first) / size) % 2)
}

impl Styles {
    /// What the table style gives to a cell: the whole-table format, then each
    /// conditional format that applies to the cell, the later over the earlier.
    /// An empty result when the table has no style that is known.
    pub fn table_cell_fmt(&self, table: &TableProps, pos: CellPos) -> CellFmt {
        let id = table
            .style
            .as_deref()
            .filter(|id| self.table_style(id).is_some())
            .or(self.default_table_id());
        let Some(id) = id else {
            return CellFmt::default();
        };
        // Base styles first.
        let layers: Vec<&TableStyle> = self
            .chain(id)
            .into_iter()
            .rev()
            .filter_map(|style| self.table_style(&style.id))
            .collect();

        let size = |own: Option<u32>, pick: fn(&TableStyle) -> Option<u32>| {
            own.or_else(|| layers.iter().rev().find_map(|l| pick(l)))
        };
        let row_size = size(table.row_band_size, |l| l.row_band_size);
        let col_size = size(table.col_band_size, |l| l.col_band_size);

        let look = table.look;
        let first_data_row = if look.first_row { pos.header_rows } else { 0 };
        let end_data_row = pos.rows - usize::from(look.last_row).min(pos.rows);
        let first_data_col = usize::from(look.first_column);
        let end_data_col = pos.cols - usize::from(look.last_column).min(pos.cols);
        let top = look.first_row && pos.row == 0;
        let bottom = look.last_row && pos.row + 1 == pos.rows;
        let left = look.first_column && pos.col == 0;
        let right = look.last_column && pos.col + 1 == pos.cols;
        let applies = |condition: Condition| match condition {
            Condition::Band1Horz | Condition::Band2Horz => {
                let wanted = usize::from(condition == Condition::Band2Horz);
                !look.no_h_band
                    && band(pos.row, first_data_row, end_data_row, row_size) == Some(wanted)
            }
            Condition::Band1Vert | Condition::Band2Vert => {
                let wanted = usize::from(condition == Condition::Band2Vert);
                !look.no_v_band
                    && band(pos.col, first_data_col, end_data_col, col_size) == Some(wanted)
            }
            Condition::LastCol => right,
            Condition::FirstCol => left,
            Condition::LastRow => bottom,
            Condition::FirstRow => look.first_row && pos.row < pos.header_rows,
            Condition::SeCell => bottom && right,
            Condition::SwCell => bottom && left,
            Condition::NeCell => top && right,
            Condition::NwCell => top && left,
        };

        let mut fmt = CellFmt::default();
        for layer in &layers {
            fmt.overlay(&layer.whole);
        }
        for condition in Condition::IN_ORDER {
            if !applies(condition) {
                continue;
            }
            for layer in &layers {
                for (_, over) in layer.conditional.iter().filter(|(c, _)| *c == condition) {
                    fmt.overlay(over);
                }
            }
        }
        fmt
    }
}

#[cfg(test)]
mod tests {
    use crate::fixtures::{DocxBuilder, Table as FixtureTable, raw_docx};
    use crate::model::Document;
    use crate::model::props::Alignment;
    use crate::model::theme::Rgb;

    fn hex(h: &str) -> Rgb {
        Rgb::from_hex(h).unwrap()
    }

    fn shd(fill: &str) -> String {
        format!(r#"<w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="{fill}"/></w:tcPr>"#)
    }

    fn colour(hex: &str) -> String {
        format!(r#"<w:rPr><w:color w:val="{hex}"/></w:rPr>"#)
    }

    /// A format for one conditional type: a text colour and a fill.
    fn format(kind: &str, text: &str, fill: &str) -> String {
        format!(
            r#"<w:tblStylePr w:type="{kind}">{}{}</w:tblStylePr>"#,
            colour(text),
            shd(fill)
        )
    }

    const SIZES: &str = r#"<w:tblStyleRowBandSize w:val="1"/><w:tblStyleColBandSize w:val="1"/>"#;
    const LOOK_ALL: &str = r#"<w:tblLook w:firstRow="1" w:lastRow="1" w:firstColumn="1" w:lastColumn="1" w:noHBand="0" w:noVBand="0"/>"#;
    /// The first row only, with row bands.
    const LOOK_ROWS: &str = r#"<w:tblLook w:firstRow="1" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="0" w:noVBand="1"/>"#;

    /// `(fill, text colour)` of the first run of each paragraph with text in a
    /// table, in order.
    fn cells(bytes: &[u8]) -> Vec<(Option<Rgb>, Option<Rgb>)> {
        Document::from_bytes(bytes)
            .unwrap()
            .paragraphs
            .iter()
            .filter(|p| p.in_table && !p.runs.is_empty())
            .map(|p| (p.container_fill, p.runs[0].props.color.rgb()))
            .collect()
    }

    fn fills(cells: &[(Option<Rgb>, Option<Rgb>)]) -> Vec<String> {
        cells
            .iter()
            .map(|c| c.0.map_or("none".to_string(), |f| f.to_string()))
            .collect()
    }

    fn grid(rows: usize, columns: usize) -> FixtureTable {
        let text: Vec<Vec<String>> = (0..rows)
            .map(|r| (0..columns).map(|c| format!("r{r}c{c}")).collect())
            .collect();
        let refs: Vec<Vec<&str>> = text
            .iter()
            .map(|row| row.iter().map(String::as_str).collect())
            .collect();
        let rows: Vec<&[&str]> = refs.iter().map(Vec::as_slice).collect();
        FixtureTable::new(&rows)
    }

    /// A document with a table style `T` and this table, which uses it.
    fn document(style_inner: &str, table: FixtureTable) -> Vec<u8> {
        DocxBuilder::new()
            .table_style("T", "T", None, style_inner)
            .table(table.style("T"))
            .build()
    }

    #[test]
    fn the_header_row_gets_its_text_colour_and_fill_from_the_table_style() {
        let style = format("firstRow", "FFFFFF", "1F3864");
        let found = cells(&document(&style, grid(3, 2).look(LOOK_ALL)));
        assert_eq!(found[0], (Some(hex("1F3864")), Some(hex("FFFFFF"))));
        assert_eq!(found[1], (Some(hex("1F3864")), Some(hex("FFFFFF"))));
        assert_eq!(found[2], (None, None), "the next row has neither");
    }

    #[test]
    fn a_table_without_a_style_gets_nothing() {
        let bytes = DocxBuilder::new()
            .table_style("T", "T", None, &format("firstRow", "FFFFFF", "1F3864"))
            .table(grid(2, 2))
            .build();
        assert_eq!(cells(&bytes)[0], (None, None));
    }

    #[test]
    fn the_whole_table_format_applies_to_every_cell() {
        let style = format!("{}{}", colour("336699"), shd("EEEEEE"));
        let found = cells(&document(&style, grid(2, 2).look(LOOK_ALL)));
        assert!(
            found
                .iter()
                .all(|c| *c == (Some(hex("EEEEEE")), Some(hex("336699"))))
        );
    }

    #[test]
    fn the_default_table_style_applies_when_a_table_names_none() {
        let styles = r#"<w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/><w:rPr><w:color w:val="AA0000"/></w:rPr></w:style>"#;
        let body = r#"<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w="9000"/></w:tblGrid><w:tr><w:tc><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p/>"#;
        assert_eq!(
            cells(&raw_docx(body, styles, false))[0].1,
            Some(hex("AA0000"))
        );
    }

    #[test]
    fn row_bands_alternate_after_the_header_row_and_stop_before_the_last_row() {
        let style = format!(
            "<w:tblPr>{SIZES}</w:tblPr>{}{}{}",
            format("firstRow", "FFFFFF", "000000"),
            format("band1Horz", "111111", "D9E2F3"),
            format("band2Horz", "222222", "FCE4D6"),
        );
        let found = cells(&document(&style, grid(5, 1).look(LOOK_ALL)));
        assert_eq!(
            fills(&found),
            ["#000000", "#D9E2F3", "#FCE4D6", "#D9E2F3", "none"],
            "the last row is switched on, so it is not banded"
        );
    }

    #[test]
    fn a_band_size_of_two_makes_bands_of_two_rows() {
        let style = format!(
            r#"<w:tblPr><w:tblStyleRowBandSize w:val="2"/></w:tblPr>{}{}"#,
            format("band1Horz", "111111", "D9E2F3"),
            format("band2Horz", "222222", "FCE4D6"),
        );
        let look = r#"<w:tblLook w:firstRow="0" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="0" w:noVBand="1"/>"#;
        let found = cells(&document(&style, grid(5, 1).look(look)));
        assert_eq!(
            fills(&found),
            ["#D9E2F3", "#D9E2F3", "#FCE4D6", "#FCE4D6", "#D9E2F3"]
        );
    }

    #[test]
    fn without_a_band_size_there_are_no_bands() {
        let style = format("band1Horz", "111111", "D9E2F3");
        let look = r#"<w:tblLook w:firstRow="0" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="0" w:noVBand="1"/>"#;
        let found = cells(&document(&style, grid(3, 1).look(look)));
        assert!(found.iter().all(|c| *c == (None, None)), "{found:?}");
        // A band size in the table is enough.
        let found = cells(&document(&style, grid(3, 1).look(look).band_sizes(1, 1)));
        assert_eq!(found[0].0, Some(hex("D9E2F3")));
    }

    #[test]
    fn tbl_look_can_switch_a_band_off_in_both_forms() {
        let style = format!(
            "<w:tblPr>{SIZES}</w:tblPr>{}",
            format("band1Horz", "111111", "D9E2F3")
        );
        let band_fill = |look: &str| cells(&document(&style, grid(2, 1).look(look)))[1].0;
        // 0x0020 is the first row, so row 1 is the first band.
        assert_eq!(
            band_fill(r#"<w:tblLook w:val="0020"/>"#),
            Some(hex("D9E2F3"))
        );
        // 0x0200 is "no row bands".
        assert_eq!(band_fill(r#"<w:tblLook w:val="0220"/>"#), None);
        let attributes = |no_h_band: u8| {
            format!(
                r#"<w:tblLook w:firstRow="1" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="{no_h_band}" w:noVBand="1"/>"#
            )
        };
        assert_eq!(band_fill(&attributes(0)), Some(hex("D9E2F3")));
        assert_eq!(band_fill(&attributes(1)), None);
    }

    #[test]
    fn a_tbl_look_attribute_that_is_missing_counts_as_on_and_hides_the_value() {
        let style = format!(
            "<w:tblPr>{SIZES}</w:tblPr>{}",
            format("band1Horz", "111111", "D9E2F3")
        );
        // noHBand is missing, so it is on: no row bands. w:val is not read.
        let found = cells(&document(
            &style,
            grid(2, 1).look(r#"<w:tblLook w:val="0020" w:firstRow="1"/>"#),
        ));
        assert_eq!(found[1].0, None);
    }

    #[test]
    fn without_tbl_look_word_uses_04a0() {
        // 04A0 is: first row, first column, no column bands.
        let style = format!(
            "<w:tblPr>{SIZES}</w:tblPr>{}{}{}{}",
            format("firstRow", "FFFFFF", "000000"),
            format("firstCol", "00FF00", "0000FF"),
            format("band1Horz", "111111", "D9E2F3"),
            format("band1Vert", "222222", "FCE4D6"),
        );
        let found = cells(&document(&style, grid(3, 3)));
        assert_eq!(found[0].0, Some(hex("000000")), "first row");
        assert_eq!(found[3].0, Some(hex("0000FF")), "first column");
        assert_eq!(
            found[4],
            (Some(hex("D9E2F3")), Some(hex("111111"))),
            "the row band applies (its colour 111111), the column band does not (222222)"
        );
    }

    #[test]
    fn the_formats_apply_in_the_order_that_word_uses() {
        let style = format!(
            "<w:tblPr>{SIZES}</w:tblPr>{}{}{}{}{}{}{}{}",
            format("band1Horz", "000001", "000001"),
            format("band1Vert", "000002", "000002"),
            format("lastCol", "000003", "000003"),
            format("firstCol", "000004", "000004"),
            format("lastRow", "000005", "000005"),
            format("firstRow", "000006", "000006"),
            format("seCell", "000007", "000007"),
            format("nwCell", "000008", "000008"),
        );
        let found = cells(&document(&style, grid(4, 4).look(LOOK_ALL)));
        let fill = |row: usize, column: usize| found[row * 4 + column].0.unwrap().to_string();
        assert_eq!(fill(1, 1), "#000002", "a column band beats a row band");
        assert_eq!(fill(1, 0), "#000004", "the first column beats a band");
        assert_eq!(fill(1, 3), "#000003", "the last column beats a band");
        assert_eq!(fill(0, 1), "#000006", "the first row");
        assert_eq!(fill(0, 3), "#000006", "the first row beats the last column");
        assert_eq!(fill(3, 0), "#000005", "the last row beats the first column");
        assert_eq!(fill(3, 3), "#000007", "a corner beats the last row");
        assert_eq!(fill(0, 0), "#000008", "a corner beats the first row");
        assert_eq!(found[0].1, Some(hex("000008")), "the text colour follows");
    }

    #[test]
    fn a_corner_needs_both_of_its_edges_switched_on() {
        let style = format!(
            "{}{}",
            format("firstRow", "FFFFFF", "000006"),
            format("nwCell", "123456", "000008")
        );
        let corner = |look: &str| cells(&document(&style, grid(2, 2).look(look)))[0].0;
        assert_eq!(corner(LOOK_ALL), Some(hex("000008")));
        assert_eq!(corner(LOOK_ROWS), Some(hex("000006")));
    }

    #[test]
    fn rows_marked_as_headers_directly_below_the_first_row_are_first_rows_too() {
        let style = format!(
            "<w:tblPr>{SIZES}</w:tblPr>{}{}",
            format("firstRow", "FFFFFF", "000000"),
            format("band1Horz", "111111", "D9E2F3"),
        );
        let with_headers = |headers: &[usize]| {
            let mut table = grid(4, 1).look(LOOK_ROWS).raw_row_properties(0, "");
            for row in headers {
                table = table.raw_row_properties(*row, "<w:tblHeader/>");
            }
            fills(&cells(&document(&style, table)))
        };
        assert_eq!(
            with_headers(&[0]),
            ["#000000", "#D9E2F3", "none", "#D9E2F3"]
        );
        assert_eq!(
            with_headers(&[0, 1]),
            ["#000000", "#000000", "#D9E2F3", "none"]
        );
        assert_eq!(
            with_headers(&[0, 2]),
            ["#000000", "#D9E2F3", "none", "#D9E2F3"],
            "a gap ends the header rows"
        );
    }

    #[test]
    fn a_cell_that_spans_columns_is_one_column() {
        let style = format!(
            "{}{}",
            format("firstCol", "FFFFFF", "000001"),
            format("lastCol", "FFFFFF", "000002")
        );
        let look = r#"<w:tblLook w:firstRow="0" w:lastRow="0" w:firstColumn="1" w:lastColumn="1" w:noHBand="1" w:noVBand="1"/>"#;
        let table = FixtureTable::new(&[&["a", "b", "c"], &["d", "e"]])
            .look(look)
            .span(1, 0, 2);
        let found = fills(&cells(&document(&style, table)));
        assert_eq!(
            found,
            ["#000001", "none", "#000002", "#000001", "#000002"],
            "the wide cell is the first column, and the cell after it the last"
        );
    }

    #[test]
    fn a_fill_set_in_the_cell_beats_the_table_style() {
        let style = format!("{}{}", colour("336699"), shd("EEEEEE"));
        let table = grid(2, 2)
            .look(LOOK_ALL)
            .cell_fill(0, 0, "00FF00")
            .cell_fill(0, 1, "auto");
        let found = cells(&document(&style, table));
        assert_eq!(found[0].0, Some(hex("00FF00")), "the cell's own fill");
        assert_eq!(found[1].0, None, "a fill of 'none' also beats the style");
        assert_eq!(found[2].0, Some(hex("EEEEEE")));
        assert_eq!(found[0].1, Some(hex("336699")), "the text colour stays");
    }

    #[test]
    fn a_style_based_on_another_inherits_and_overrides_per_format() {
        let base = format!(
            "{}{}",
            colour("336699"),
            format("firstRow", "FFFFFF", "000000")
        );
        let derived = r#"<w:tblStylePr w:type="firstRow"><w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="222222"/></w:tcPr></w:tblStylePr>"#;
        let bytes = DocxBuilder::new()
            .table_style("T", "T", None, &base)
            .table_style("U", "U", Some("T"), derived)
            .table(grid(2, 1).style("U").look(LOOK_ROWS))
            .build();
        let found = cells(&bytes);
        assert_eq!(
            found[0],
            (Some(hex("222222")), Some(hex("FFFFFF"))),
            "the fill is the derived style's, the text colour the base style's first row"
        );
        assert_eq!(
            found[1],
            (None, Some(hex("336699"))),
            "the base whole format"
        );
    }

    #[test]
    fn a_paragraph_style_beats_the_table_style_and_direct_formatting_beats_both() {
        let styles = r#"<w:style w:type="paragraph" w:styleId="Grey"><w:name w:val="Grey"/><w:rPr><w:color w:val="777777"/></w:rPr></w:style>
<w:style w:type="table" w:styleId="TS"><w:name w:val="TS"/><w:rPr><w:color w:val="0000FF"/></w:rPr></w:style>"#;
        let cell = |p: &str| format!("<w:tc>{p}</w:tc>");
        let grey =
            r#"<w:p><w:pPr><w:pStyle w:val="Grey"/></w:pPr><w:r><w:t>grey</w:t></w:r></w:p>"#;
        let plain = r#"<w:p><w:r><w:t>plain</w:t></w:r></w:p>"#;
        let direct = r#"<w:p><w:pPr><w:pStyle w:val="Grey"/></w:pPr><w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t>direct</w:t></w:r></w:p>"#;
        let body = format!(
            r#"<w:tbl><w:tblPr><w:tblStyle w:val="TS"/></w:tblPr><w:tblGrid><w:gridCol w:w="3000"/><w:gridCol w:w="3000"/><w:gridCol w:w="3000"/></w:tblGrid><w:tr>{}{}{}</w:tr></w:tbl><w:p/>"#,
            cell(grey),
            cell(plain),
            cell(direct)
        );
        let found = cells(&raw_docx(&body, styles, false));
        assert_eq!(found[0].1, Some(hex("777777")), "paragraph style");
        assert_eq!(found[1].1, Some(hex("0000FF")), "table style");
        assert_eq!(found[2].1, Some(hex("FF0000")), "direct formatting");
    }

    #[test]
    fn the_table_style_is_in_the_style_run_of_the_paragraph() {
        let style = colour("336699");
        let model = Document::from_bytes(&document(&style, grid(1, 1))).unwrap();
        let p = model.paragraphs.iter().find(|p| p.in_table).unwrap();
        assert_eq!(p.style_run.color.rgb(), Some(hex("336699")));
    }

    #[test]
    fn paragraph_properties_of_a_table_style_apply_to_its_paragraphs() {
        let styles = r#"<w:style w:type="table" w:styleId="TS"><w:name w:val="TS"/><w:pPr><w:jc w:val="both"/></w:pPr></w:style>"#;
        let body = r#"<w:tbl><w:tblPr><w:tblStyle w:val="TS"/></w:tblPr><w:tblGrid><w:gridCol w:w="9000"/></w:tblGrid><w:tr><w:tc><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:t>y</w:t></w:r></w:p>"#;
        let model = Document::from_bytes(&raw_docx(body, styles, false)).unwrap();
        assert_eq!(model.paragraphs[0].props.alignment, Alignment::Justify);
        assert_eq!(model.paragraphs[1].props.alignment, Alignment::Left);
    }

    #[test]
    fn a_nested_table_does_not_take_the_style_of_the_table_around_it() {
        let style = format!("{}{}", colour("336699"), shd("EEEEEE"));
        let inner = grid(1, 1);
        let table = grid(1, 2).look(LOOK_ALL).nested(0, 1, &inner);
        let found = cells(&document(&style, table));
        assert_eq!(found[0], (Some(hex("EEEEEE")), Some(hex("336699"))));
        // The inner cell has no fill of its own, so the cell around it shows
        // through. Its text has no colour from the outer table's style.
        assert_eq!(found[1], (Some(hex("EEEEEE")), None));
    }

    #[test]
    fn an_unknown_table_style_gives_nothing() {
        let bytes = DocxBuilder::new()
            .table_style("T", "T", None, &colour("336699"))
            .table(grid(1, 1).style("Missing"))
            .build();
        assert_eq!(cells(&bytes)[0], (None, None));
    }
}
